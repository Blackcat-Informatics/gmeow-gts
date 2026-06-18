# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The ``gts`` command-line tool: inspect, fold, verify, and compose GTS files.

``cat`` and ``verify`` implement the §14.1 composition-tooling contract: raw
byte concatenation is always valid GTS (§3.1), but a publish-class tool
refuses pathological states instead of trusting them to be intentional. The
Rust engine ships a binary with the IDENTICAL command surface; this entry
point keeps the contract while the native wheel lands.

Exit codes: 0 clean; 1 diagnostics found or input refused; 2 usage/IO error.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import TYPE_CHECKING

from gts.mmr import parse_hex_32
from gts.model import Graph
from gts.nquads import to_nquads
from gts.policy import TrustPolicy, evaluate_profile_policy
from gts.reader import read, read_segments
from gts.replication import (
    heads_json,
    inventory,
    missing,
    missing_json,
    resume_after,
    segments_json,
)

if TYPE_CHECKING:
    from gts.crypto import KeyProvider


def _load(path: str) -> bytes:
    try:
        return Path(path).read_bytes()
    except OSError as exc:
        print(f"gts: cannot read {path}: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc


def _print_ledger(path: str, segments: list[Graph], torn: int | None) -> None:
    """Print the per-segment composition ledger (§14.1 "SHOULD report")."""
    suffix = f", TORN at byte {torn}" if torn is not None else ""
    print(f"{path}: {len(segments)} segment(s){suffix}")
    for idx, seg in enumerate(segments):
        head = seg.segment_heads[0].hex() if seg.segment_heads else "<none>"
        profile = seg.segment_profiles[0] if seg.segment_profiles else "<none>"
        signers = sum(1 for s in seg.signatures if s.status != "invalid")
        print(
            f"  segment {idx}: head {head} profile {profile} "
            f"terms {len(seg.terms)} quads {len(seg.quads)} "
            f"reifies {len(seg.reifiers)} annot {len(seg.annotations)} "
            f"blobs {len(seg.blobs)} suppress {len(seg.suppressions)} "
            f"opaque {len(seg.opaque)} sigs {signers}"
        )
        layout = seg.segment_streamable[0] if seg.segment_streamable else None
        if layout is not None and layout.claimed:
            head_hex = layout.head.hex() if layout.head is not None else "<none>"
            tail = f", accretive tail {layout.tail} frame(s)" if layout.tail else ""
            print(
                f"    layout: streamable through frame {layout.covered} "
                f"(head {head_hex}){tail}"
            )
        for o in seg.opaque:
            print(f"    opaque: {o.frame_type} ({o.reason})")
        for d in seg.diagnostics:
            where = f" [item {d.frame_index}]" if d.frame_index is not None else ""
            print(f"    diagnostic {d.code}: {d.detail}{where}")


def _has_problems(
    segments: list[Graph], torn: int | None, fatal: object | None
) -> bool:
    return (
        fatal is not None
        or torn is not None
        or any(seg.diagnostics for seg in segments)
    )


def _cmd_info(paths: list[str]) -> int:
    for path in paths:
        segments, torn, fatal = read_segments(_load(path))
        if fatal is not None:
            print(f"{path}: 0 segment(s)")
            print(f"  FATAL {fatal.code}: {fatal.detail}")
            continue
        _print_ledger(path, segments, torn)
    return 0


def _cmd_fold(path: str) -> int:
    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    sys.stdout.write(to_nquads(g))
    # The (possibly partial) fold is still emitted, but any diagnostic —
    # or never reaching segmentation at all — is a nonzero exit, so
    # `gts fold … && publish` pipelines fail on damage.
    return 1 if g.diagnostics or not g.segment_heads else 0


def _build_verifier(key_specs: list[str] | None) -> KeyProvider | None:
    """Build an in-memory key provider from ``kid:hexpubkey`` specs, or None."""
    if not key_specs:
        return None
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

    from gts.crypto import InMemoryKeys

    verifiers = {}
    for spec in key_specs:
        kid, _, hexpub = spec.partition(":")
        if not kid or not hexpub:
            msg = f"gts verify: bad --key {spec!r} (want kid:hexpubkey)"
            print(msg, file=sys.stderr)
            raise SystemExit(2)
        verifiers[kid] = Ed25519PublicKey.from_public_bytes(bytes.fromhex(hexpub))
    return InMemoryKeys(verifiers=verifiers)


def _finding_label(code: str, severity: str) -> str:
    if code.startswith("ProfileVocabulary"):
        return f"profile {severity}"
    if code == "StreamVocabularyWithoutLayout":
        return "layout warning"
    return severity


def _cmd_verify(
    paths: list[str],
    key_specs: list[str] | None = None,
    trusted_signers: list[str] | None = None,
) -> int:
    problems = False
    keys = _build_verifier(key_specs)
    policy = TrustPolicy(
        trusted_signers=frozenset(trusted_signers or ()),
        require_trusted_signer=bool(trusted_signers),
    )
    for path in paths:
        data = _load(path)
        segments, torn, fatal = read_segments(data, keys=keys)
        if fatal is not None:
            print(f"{path}: 0 segment(s)")
            print(f"  FATAL {fatal.code}: {fatal.detail}")
            problems = True
            continue
        _print_ledger(path, segments, torn)
        problems = problems or _has_problems(segments, torn, fatal)
        # §14.1: declared-vs-computed profile requirements + layout warnings.
        for idx, seg in enumerate(segments):
            for finding in evaluate_profile_policy(seg, policy, segment_index=idx):
                label = _finding_label(finding.code, finding.severity)
                print(
                    f"  segment {idx}: {label}: {finding.code}: {finding.detail}",
                    file=sys.stderr,
                )
                if finding.severity == "error":
                    problems = True
        # §9.2: COSE signature verification against the provided keys.
        if keys is not None:
            graph = read(data, keys=keys)
            for sig in graph.signatures:
                print(f"  signature {sig.kid or '?'}: {sig.status}")
                if sig.status == "invalid":
                    problems = True
    return 1 if problems else 0


def _cmd_verify_proof(path: str) -> int:
    from gts.mmr import proof_from_json, verify_proof

    try:
        text = Path(path).read_text(encoding="utf-8")
    except OSError as exc:
        print(f"gts verify-proof: cannot read {path}: {exc}", file=sys.stderr)
        return 2
    except UnicodeDecodeError as exc:
        print(f"gts verify-proof: invalid proof JSON: {exc}", file=sys.stderr)
        return 1
    try:
        proof = proof_from_json(text)
    except ValueError as exc:
        print(f"gts verify-proof: invalid proof JSON: {exc}", file=sys.stderr)
        return 1
    try:
        verify_proof(proof)
    except ValueError as exc:
        print(f"gts verify-proof: invalid proof: {exc}", file=sys.stderr)
        return 1
    print(f"proof ok: root {proof.root.hex()} frame {proof.frame_id.hex()}")
    return 0


def _cmd_heads(path: str) -> int:
    inv = inventory(_load(path))
    print(heads_json(inv), end="")
    return 1 if inv.has_problems() else 0


def _cmd_segments(path: str) -> int:
    inv = inventory(_load(path))
    print(segments_json(inv), end="")
    return 1 if inv.has_problems() else 0


def _cmd_missing(from_head: str, path: str) -> int:
    try:
        peer_head = parse_hex_32(from_head)
    except ValueError as exc:
        print(f"gts missing: invalid peer head: {exc}", file=sys.stderr)
        return 2
    result = missing(inventory(_load(path)), peer_head)
    print(missing_json(result), end="")
    return 1 if result.status == "error" else 0


def _cmd_resume(after: str, path: str) -> int:
    try:
        frame_id = parse_hex_32(after)
    except ValueError as exc:
        print(f"gts resume: invalid frame id: {exc}", file=sys.stderr)
        return 2
    data = _load(path)
    try:
        tail = resume_after(data, frame_id)
    except ValueError as exc:
        print(f"gts resume: {exc}", file=sys.stderr)
        return 1
    return _write_out(None, tail)


def _cmd_extract_key(path: str) -> int:
    """Print the embedded transport (verification) key for a signed GTS (§9.2).

    Emits the ``kid``, the OpenPGP fingerprint, an emojihash for eyeball
    verification, and the armored public key. Exit 1 if no key is embedded.
    """
    from gts.verify import extract_transport_key, format_fingerprint

    key = extract_transport_key(read(_load(path)))
    if key is None:
        print(f"{path}: no embedded transport key", file=sys.stderr)
        return 1

    armored = key["gpg"]
    print(f"kid:         {key['kid']}")
    try:
        from cryptography.hazmat.primitives import serialization

        from gts.emojihash import emojihash
        from gts.openpgp import load_public_key, public_key_fingerprint

        raw = load_public_key(armored).public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw
        )
        print(f"fingerprint: {format_fingerprint(public_key_fingerprint(armored))}")
        print(f"emojihash:   {emojihash(raw)}")
    except Exception:  # noqa: BLE001 - malformed embedded key still prints below
        print(f"fingerprint: {format_fingerprint(key['kid'])}")
    print(armored)
    return 0


def _cmd_from_nq(path: str, out: str | None) -> int:
    """Build a GTS from N-Quads text — the inverse of ``fold`` (§14).

    Reads ``path`` (or stdin when ``path`` is ``-``); writes GTS to ``out`` (or
    stdout). Lets RDF producers delegate the GTS encoding to the binary.
    """
    from gts.from_nquads import NQuadsParseError, from_nquads

    try:
        text = sys.stdin.read() if path == "-" else _load(path).decode("utf-8")
    except OSError as exc:
        print(f"gts from-nq: cannot read {path}: {exc}", file=sys.stderr)
        return 2
    try:
        data = from_nquads(text)
    except NQuadsParseError as exc:
        print(f"gts from-nq: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


_DB_EXTRA_HINT = "requires the [db] extra: pip install 'gmeow-gts[db]'"


def _cmd_to_sqlite(path: str, out: str) -> int:
    """Export a folded graph to a SQLite database (stdlib, §14)."""
    from gts.db import to_sqlite

    to_sqlite(read(_load(path)), out)
    return 0


def _cmd_to_duckdb(path: str, out: str) -> int:
    """Export a folded graph to a DuckDB database (needs the [db] extra)."""
    from gts.db import to_duckdb

    try:
        to_duckdb(read(_load(path)), out)
    except ImportError:
        print(f"gts to-duckdb: {_DB_EXTRA_HINT}", file=sys.stderr)
        return 2
    return 0


def _cmd_to_parquet(path: str, out_dir: str) -> int:
    """Export a folded graph to Parquet, one file per table (needs [db])."""
    from gts.db import to_parquet

    try:
        for written in to_parquet(read(_load(path)), out_dir):
            print(written)
    except ImportError:
        print(f"gts to-parquet: {_DB_EXTRA_HINT}", file=sys.stderr)
        return 2
    return 0


def _all_quads_suppressed(g: Graph) -> bool:
    """True iff the fold has quads and EVERY one is hidden by a suppression.

    A quad is hidden by a direct quad target or a term target on any of its
    components (§11) — the union graph is value-interned, so id matching IS
    value matching.
    """
    if not g.quads or not g.suppressions:
        return False
    term_sup: set[int] = set()
    quad_sup: set[tuple[int, ...]] = set()
    for sup in g.suppressions:
        for target in sup.targets:
            kind = target.get("kind")
            tid = target.get("id")
            if kind in ("term", "reifier") and isinstance(tid, int):
                term_sup.add(tid)
            elif kind == "quad":
                q = target.get("q")
                if isinstance(q, list) and all(isinstance(x, int) for x in q):
                    quad_sup.add(tuple(q))
    return all(
        (s, p, o) in quad_sup
        or (gq is not None and (s, p, o, gq) in quad_sup)
        or s in term_sup
        or p in term_sup
        or o in term_sup
        or (gq is not None and gq in term_sup)
        for s, p, o, gq in g.quads
    )


def _write_out(out: str | None, data: bytes) -> int:
    """Write to a path or stdout; IO failure is exit 2, never a traceback."""
    try:
        if out is not None:
            Path(out).write_bytes(data)
        else:
            sys.stdout.buffer.write(data)
    except OSError as exc:  # includes BrokenPipeError
        print(f"gts: cannot write {out or 'stdout'}: {exc}", file=sys.stderr)
        return 2
    return 0


def _cmd_ls(path: str) -> int:
    """List inline blobs: digest, size, declared media type (tar's ``t``)."""
    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    for digest, data in g.blobs.items():
        mt = g.blob_meta.get(digest, {}).get("mt")
        mt_text = mt if isinstance(mt, str) else "-"
        print(f"{digest}  {len(data):>10}  {mt_text}")
    return 0


def _normalize_digest(digest: str) -> str:
    return digest if digest.startswith("blake3:") else f"blake3:{digest}"


def _suppressed_blob_digests(g: Graph) -> set[str]:
    """Digests hidden by ``{"kind": "blob", "digest": …}`` targets (§11)."""
    out: set[str] = set()
    for sup in g.suppressions:
        for target in sup.targets:
            if target.get("kind") != "blob":
                continue
            d = target.get("digest")
            if isinstance(d, bytes):
                out.add(f"blake3:{d.hex()}")
            elif isinstance(d, str):
                out.add(_normalize_digest(d))
    return out


def _cmd_extract(
    path: str,
    digest: str,
    out: str | None,
    mt: str | None,
    include_suppressed: bool,
) -> int:
    """Extract one blob by content digest (tar's ``x``), refuse-don't-trust.

    Verifies the bytes against the requested digest on the way out, honours
    blob suppression (§11) unless overridden, and treats ``--mt`` as an
    ASSERTION against the blob's declared media type — never a conversion.
    """
    g = read(_load(path))
    digest = _normalize_digest(digest)
    data = g.blobs.get(digest)
    if data is None:
        print(f"gts: no inline blob {digest} in {path}", file=sys.stderr)
        return 1
    if digest in _suppressed_blob_digests(g) and not include_suppressed:
        print(
            f"gts: refusing {digest}: suppressed (§11); "
            "pass --include-suppressed to extract anyway",
            file=sys.stderr,
        )
        return 1
    if mt is not None:
        declared = g.blob_meta.get(digest, {}).get("mt")
        if declared != mt:
            print(
                f"gts: refusing {digest}: declared media type "
                f"{declared!r} does not match asserted {mt!r}",
                file=sys.stderr,
            )
            return 1
    from gts.wire import digest_str

    if digest_str(data) != digest:
        print(
            f"gts: integrity failure: {digest} bytes re-hash differently",
            file=sys.stderr,
        )
        return 1
    return _write_out(out, data)


def _cmd_cat(paths: list[str], out: str | None) -> int:
    """The validating composer (§14.1): refuse-don't-trust, then ``cat``."""
    if len(paths) < 2:
        print("gts: cat needs at least two inputs", file=sys.stderr)
        return 2
    combined = bytearray()
    for path in paths:
        data = _load(path)
        segments, torn, fatal = read_segments(data)
        if _has_problems(segments, torn, fatal):
            print(f"gts: refusing {path}: not a clean GTS input", file=sys.stderr)
            return 1
        # §14.1: a segment that contributes NOTHING (no quads, blobs, reifier
        # bindings, annotations, or suppressions) is almost always a wiring
        # bug — never a real package. Refuse, don't trust.
        for idx, seg in enumerate(segments):
            contributes = bool(
                seg.quads
                or seg.blobs
                or seg.reifiers
                or seg.annotations
                or seg.suppressions
            )
            if not contributes:
                print(
                    f"gts: refusing {path}: segment {idx} folds to nothing "
                    "(no quads/blobs/reifies/annot/suppress) — wiring bug?",
                    file=sys.stderr,
                )
                return 1
        combined += data

    # §14.1: refuse an output in which suppressions would hide every quad.
    folded = read(bytes(combined))
    if _all_quads_suppressed(folded):
        print(
            "gts: refusing composition: suppressions hide every quad in the "
            "folded output",
            file=sys.stderr,
        )
        return 1

    return _write_out(out, bytes(combined))


def _cmd_compact(
    path: str,
    out: str,
    *,
    streamable: bool,
    seal_original: bool,
    timestamp: str | None,
) -> int:
    """Rewrite a GTS file into the streamable layout state (§10.1, §14.1)."""
    if not streamable:
        # The verb is reserved for layout rewrites; a future --snapshot mode
        # (§10) would land here. Without a mode the request is ambiguous.
        print("gts: compact requires --streamable", file=sys.stderr)
        return 2
    from datetime import UTC, datetime

    from gts.compact import CompactRefusedError, compact_streamable

    ts = timestamp or datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    try:
        data = compact_streamable(
            _load(path), timestamp=ts, seal_original=seal_original
        )
    except CompactRefusedError as exc:
        print(f"gts: refusing compact: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


def _cmd_pack(sources: list[str], out: str) -> int:
    """Pack files/directories into a files-profile GTS archive (tar's ``c``)."""
    from gts.files import pack

    try:
        data = pack([Path(s) for s in sources])
    except (OSError, ValueError) as exc:
        print(f"gts: refusing pack: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


def _cmd_unpack(path: str, dest: str | None, include_suppressed: bool) -> int:
    """Unpack a files-profile GTS archive (tar's ``x``), verifying digests."""
    from gts.files import unpack

    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    if g.diagnostics or not g.segment_heads:
        print("gts: refusing unpack: archive did not read cleanly", file=sys.stderr)
        return 1
    try:
        unpack(g, Path(dest or "."), include_suppressed=include_suppressed)
    except (OSError, ValueError) as exc:
        print(f"gts: refusing unpack: {exc}", file=sys.stderr)
        return 1
    return 0


def _cmd_diff(path: str, directory: str) -> int:
    """Compare an archive to a directory by content digest (tar's ``d``)."""
    from gts.files import diff

    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    if g.diagnostics or not g.segment_heads:
        print("gts: refusing diff: archive did not read cleanly", file=sys.stderr)
        return 1
    try:
        lines = diff(g, Path(directory))
    except (OSError, ValueError) as exc:
        print(f"gts: refusing diff: {exc}", file=sys.stderr)
        return 1
    for line in lines:
        print(line)
    return 1 if lines else 0


def main(argv: list[str] | None = None) -> int:
    """Entry point for the ``gts`` console script."""
    parser = argparse.ArgumentParser(
        prog="gts",
        description="Inspect, fold, verify, and compose GTS files.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_info = sub.add_parser("info", help="per-segment composition ledger (§14.1)")
    p_info.add_argument("files", nargs="+")

    p_fold = sub.add_parser("fold", help="fold to N-Quads on stdout")
    p_fold.add_argument("file")

    p_verify = sub.add_parser(
        "verify", help="verify chains; ledger + diagnostics; exit 1 on any"
    )
    p_verify.add_argument("files", nargs="+")
    p_verify.add_argument(
        "--key",
        action="append",
        metavar="KID:HEXPUB",
        help="verify COSE signatures against a raw Ed25519 public key (repeatable)",
    )
    p_verify.add_argument(
        "--trusted-signer",
        action="append",
        metavar="KID",
        help="profile-policy trust anchor for an already verified signer kid",
    )

    p_verify_proof = sub.add_parser(
        "verify-proof",
        help="verify detached MMR proof JSON without the GTS file",
    )
    p_verify_proof.add_argument("proof")

    p_heads = sub.add_parser(
        "heads", help="JSON segment heads and aggregate comparison digest"
    )
    p_heads.add_argument("file")

    p_segments = sub.add_parser(
        "segments", help="JSON segment byte ranges and layout inventory"
    )
    p_segments.add_argument("file")

    p_missing = sub.add_parser(
        "missing", help="JSON byte ranges needed after a peer head"
    )
    p_missing.add_argument("--from-head", required=True)
    p_missing.add_argument("file")

    p_resume = sub.add_parser(
        "resume", help="emit bytes after a verified frame boundary"
    )
    p_resume.add_argument("--after", required=True)
    p_resume.add_argument("file")

    p_ls = sub.add_parser(
        "ls", help="list inline blobs: digest, size, declared media type"
    )
    p_ls.add_argument("file")

    p_extract_key = sub.add_parser(
        "extract-key",
        help="print the embedded transport/verification key: kid, fingerprint, "
        "emojihash, armored public key (§9.2)",
    )
    p_extract_key.add_argument("file")

    p_from_nq = sub.add_parser(
        "from-nq",
        help="build a GTS from N-Quads — the inverse of fold; '-' reads stdin",
    )
    p_from_nq.add_argument("file")
    p_from_nq.add_argument("-o", "--out", default=None)

    p_to_sqlite = sub.add_parser(
        "to-sqlite", help="export a folded graph to a SQLite database (§14)"
    )
    p_to_sqlite.add_argument("file")
    p_to_sqlite.add_argument("out")

    p_to_duckdb = sub.add_parser(
        "to-duckdb",
        help="export a folded graph to a DuckDB database (needs the [db] extra)",
    )
    p_to_duckdb.add_argument("file")
    p_to_duckdb.add_argument("out")

    p_to_parquet = sub.add_parser(
        "to-parquet",
        help="export a folded graph to Parquet, one file per table "
        "(needs the [db] extra)",
    )
    p_to_parquet.add_argument("file")
    p_to_parquet.add_argument("out_dir")

    p_extract = sub.add_parser(
        "extract",
        help="extract one blob by content digest; --mt asserts the declared "
        "media type (never converts)",
    )
    p_extract.add_argument("file")
    p_extract.add_argument("digest")
    p_extract.add_argument("-o", "--out", default=None)
    p_extract.add_argument("--mt", default=None)
    p_extract.add_argument("--include-suppressed", action="store_true")

    p_cat = sub.add_parser(
        "cat",
        help="validating composer: refuse degenerate inputs, then "
        "byte-concatenate (§3.1, §14.1)",
    )
    p_cat.add_argument("files", nargs="+")
    p_cat.add_argument("-o", "--out", default=None)

    p_compact = sub.add_parser(
        "compact",
        help="rewrite into the streamable layout state: leading streaming "
        "index, blobs most-significant-first, trailing index footer (§10.1)",
    )
    p_compact.add_argument("file")
    p_compact.add_argument("-o", "--out", required=True)
    p_compact.add_argument(
        "--streamable",
        action="store_true",
        help="produce the delivery-ordered streamable layout (§3.3)",
    )
    p_compact.add_argument(
        "--seal-original",
        action="store_true",
        help="carry the verbatim source as a nested GTS blob, role 'source' "
        "(REQUIRED for evidence input)",
    )
    p_compact.add_argument(
        "--timestamp",
        default=None,
        help="rewrite time recorded as stream:timestamp (ISO 8601 UTC); "
        "defaults to now — pass a fixed value for reproducible output",
    )

    p_pack = sub.add_parser(
        "pack", help="pack files/directories into a files-profile GTS archive"
    )
    p_pack.add_argument("sources", nargs="+")
    p_pack.add_argument("-o", "--out", required=True)

    p_unpack = sub.add_parser("unpack", help="unpack a files-profile GTS archive")
    p_unpack.add_argument("file")
    p_unpack.add_argument("-C", dest="dest", default=None)
    p_unpack.add_argument(
        "--include-suppressed",
        action="store_true",
        help="extract digest-suppressed entries anyway",
    )

    p_diff = sub.add_parser(
        "diff",
        help="compare a files-profile GTS archive to a directory by digest",
    )
    p_diff.add_argument("archive")
    p_diff.add_argument("directory")

    args = parser.parse_args(argv)
    if args.command == "info":
        return _cmd_info(args.files)
    if args.command == "fold":
        return _cmd_fold(args.file)
    if args.command == "verify":
        return _cmd_verify(args.files, args.key, args.trusted_signer)
    if args.command == "verify-proof":
        return _cmd_verify_proof(args.proof)
    if args.command == "heads":
        return _cmd_heads(args.file)
    if args.command == "segments":
        return _cmd_segments(args.file)
    if args.command == "missing":
        return _cmd_missing(args.from_head, args.file)
    if args.command == "resume":
        return _cmd_resume(args.after, args.file)
    if args.command == "ls":
        return _cmd_ls(args.file)
    if args.command == "extract-key":
        return _cmd_extract_key(args.file)
    if args.command == "from-nq":
        return _cmd_from_nq(args.file, args.out)
    if args.command == "to-sqlite":
        return _cmd_to_sqlite(args.file, args.out)
    if args.command == "to-duckdb":
        return _cmd_to_duckdb(args.file, args.out)
    if args.command == "to-parquet":
        return _cmd_to_parquet(args.file, args.out_dir)
    if args.command == "extract":
        return _cmd_extract(
            args.file, args.digest, args.out, args.mt, args.include_suppressed
        )
    if args.command == "compact":
        return _cmd_compact(
            args.file,
            args.out,
            streamable=args.streamable,
            seal_original=args.seal_original,
            timestamp=args.timestamp,
        )
    if args.command == "pack":
        return _cmd_pack(args.sources, args.out)
    if args.command == "unpack":
        return _cmd_unpack(args.file, args.dest, args.include_suppressed)
    if args.command == "diff":
        return _cmd_diff(args.archive, args.directory)
    return _cmd_cat(args.files, args.out)


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
