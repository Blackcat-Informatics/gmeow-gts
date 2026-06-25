# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""End-to-end tests of the ``gts`` console script (§14.1 tooling contract).

The command surface here MUST stay identical to the Rust binary's
(``rust/src/bin/gts.rs`` + ``rust/tests/cli.rs``): same verbs,
same refusals, same exit codes.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from gts import Term, TermKind, Writer
from gts.cli import main
from gts.wire import digest_str

# The frozen conformance corpus lives at <repo root>/vectors; this file is at
# <repo root>/python/tests/.
VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors"

CAT = "https://example.org/Cat"
LABEL = "http://www.w3.org/2000/01/rdf-schema#label"

BLOB = b"not really webp bytes"


@pytest.mark.parametrize(
    ("locale", "usage_marker", "error_marker"),
    [
        ("nonsense", "usage: gts", "unknown command"),
        ("fr_CA", "utilisation: gts", "commande inconnue"),
        ("zh_CN", "用法: gts", "未知命令"),
    ],
)
def test_cli_localized_help_and_unknown_command(
    locale: str,
    usage_marker: str,
    error_marker: str,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setenv("GTS_LANG", locale)
    monkeypatch.delenv("LC_ALL", raising=False)
    monkeypatch.delenv("LC_MESSAGES", raising=False)
    monkeypatch.delenv("LANG", raising=False)

    assert main(["help"]) == 0
    out = capsys.readouterr().out
    assert usage_marker in out
    assert "from-nq" in out

    assert main(["not-a-gts-command"]) == 2
    err = capsys.readouterr().err
    assert error_marker in err
    assert "not-a-gts-command" in err
    assert "from-nq" in err


def _replication_file(tmp_path: Path) -> tuple[Path, bytes, bytes, bytes]:
    first = Writer()
    first_head = first.add_blob(b"a", mt="text/plain")
    first_bytes = first.to_bytes()
    second = Writer()
    second_head = second.add_blob(b"b", mt="text/plain")
    second_bytes = second.to_bytes()
    path = tmp_path / "replicated.gts"
    path.write_bytes(first_bytes + second_bytes)
    return path, first_head, second_head, second_bytes


def test_replication_verbs_emit_json_shapes_and_resume_boundary(
    tmp_path: Path, capsysbinary: pytest.CaptureFixture[bytes]
) -> None:
    path, first_head, second_head, second_bytes = _replication_file(tmp_path)
    total_size = path.stat().st_size
    first_size = total_size - len(second_bytes)

    assert main(["heads", str(path)]) == 0
    heads = json.loads(capsysbinary.readouterr().out.decode())
    assert heads["schema"] == "gts-replication-heads-v1"
    assert heads["clean"] is True
    assert heads["segment_heads"] == [first_head.hex(), second_head.hex()]
    assert heads["aggregate"]["schema"] == "gts-segment-heads-v1"
    assert heads["aggregate"]["count"] == 2
    assert heads["aggregate"]["file_head"] == second_head.hex()
    assert heads["fatal"] is None

    assert main(["segments", str(path)]) == 0
    segments = json.loads(capsysbinary.readouterr().out.decode())
    assert segments["schema"] == "gts-replication-segments-v1"
    assert segments["clean"] is True
    assert segments["item_count"] == 4
    assert segments["segments"][0]["byte_range"] == {
        "start": 0,
        "end": first_size,
        "length": first_size,
    }
    assert segments["segments"][1]["byte_range"] == {
        "start": first_size,
        "end": total_size,
        "length": len(second_bytes),
    }
    assert segments["segments"][0]["frame_count"] == 1

    assert main(["missing", "--from-head", first_head.hex(), str(path)]) == 0
    missing = json.loads(capsysbinary.readouterr().out.decode())
    assert missing == {
        "schema": "gts-replication-missing-v1",
        "status": "ranges",
        "from_head": first_head.hex(),
        "ranges": [
            {"start": first_size, "end": total_size, "length": len(second_bytes)}
        ],
        "scan_required": False,
        "detail": None,
    }

    assert main(["resume", "--after", first_head.hex(), str(path)]) == 0
    assert capsysbinary.readouterr().out == second_bytes


def _blob_file(tmp_path: Path) -> tuple[Path, str]:
    w = Writer()
    w.add_terms(
        [
            Term(TermKind.IRI, CAT),
            Term(TermKind.IRI, LABEL),
            Term(TermKind.LITERAL, "Cat", lang="en"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    w.add_blob(BLOB, mt="image/webp")
    path = tmp_path / "blob.gts"
    path.write_bytes(w.to_bytes())
    return path, digest_str(BLOB)


def test_ls_lists_digest_size_and_media_type(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    path, digest = _blob_file(tmp_path)
    assert main(["ls", str(path)]) == 0
    out = capsys.readouterr().out
    assert digest in out
    assert str(len(BLOB)) in out
    assert "image/webp" in out


def test_extract_writes_verified_bytes(tmp_path: Path) -> None:
    path, digest = _blob_file(tmp_path)
    out = tmp_path / "yak.webp"
    assert main(["extract", str(path), digest, "-o", str(out)]) == 0
    assert out.read_bytes() == BLOB


def test_extract_accepts_bare_hex_digest(tmp_path: Path) -> None:
    path, digest = _blob_file(tmp_path)
    out = tmp_path / "yak.webp"
    bare = digest.removeprefix("blake3:")
    assert main(["extract", str(path), bare, "-o", str(out)]) == 0
    assert out.read_bytes() == BLOB


def test_extract_mt_is_an_assertion_not_a_conversion(tmp_path: Path) -> None:
    path, digest = _blob_file(tmp_path)
    out = tmp_path / "yak.png"
    # asserted type mismatches the declared image/webp — refuse, never convert
    assert (
        main(["extract", str(path), digest, "-o", str(out), "--mt", "image/png"]) == 1
    )
    assert not out.exists()
    assert (
        main(["extract", str(path), digest, "-o", str(out), "--mt", "image/webp"]) == 0
    )


def test_extract_unknown_digest_fails(tmp_path: Path) -> None:
    path, _ = _blob_file(tmp_path)
    assert main(["extract", str(path), "blake3:" + "0" * 64, "-o", "/dev/null"]) == 1


def test_extract_refuses_suppressed_blob_by_default(tmp_path: Path) -> None:
    w = Writer()
    w.add_blob(BLOB, mt="image/webp")
    digest = digest_str(BLOB)
    w.add_suppress([{"kind": "blob", "digest": digest}], reason="retracted")
    path = tmp_path / "suppressed.gts"
    path.write_bytes(w.to_bytes())

    out = tmp_path / "yak.webp"
    assert main(["extract", str(path), digest, "-o", str(out)]) == 1
    assert not out.exists()
    # suppression is a display overlay (§11) — history stays extractable
    assert (
        main(["extract", str(path), digest, "-o", str(out), "--include-suppressed"])
        == 0
    )
    assert out.read_bytes() == BLOB


def test_fold_exits_nonzero_on_diagnostics() -> None:
    # damaged corpus vector: the partial fold is emitted, the exit is 1 —
    # `gts fold … && publish` pipelines must fail on damage
    damaged = VECTORS_DIR / "04-damaged-frame.gts"
    assert main(["fold", str(damaged)]) == 1


def _make_tree(tmp_path: Path) -> Path:
    src = tmp_path / "src"
    src.mkdir()
    (src / "a.txt").write_text("hello")
    (src / "subdir").mkdir()
    (src / "subdir" / "b.txt").write_text("world")
    fixed_mtime = 1_700_000_000.0
    for p in [src / "a.txt", src / "subdir" / "b.txt"]:
        p.chmod(0o644)
        os.utime(p, (fixed_mtime, fixed_mtime))
    return src


def _files_archive_with_path(tmp_path: Path, archive_path: str) -> Path:
    w = Writer(profile="files")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://w3id.org/gts/files#FileEntry"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#path"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#digest"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#size"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#mode"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#modified"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#mediaType"),
            Term(TermKind.IRI, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            Term(TermKind.IRI, "http://www.w3.org/2001/XMLSchema#integer"),
            Term(TermKind.IRI, "http://www.w3.org/2001/XMLSchema#dateTime"),
            Term(TermKind.BNODE, "e0"),
            Term(TermKind.LITERAL, archive_path),
            Term(TermKind.LITERAL, "blake3:" + "0" * 64),
        ]
    )
    w.add_quads(
        [
            (10, 7, 0, None),  # e0 a FileEntry
            (10, 1, 11, None),  # e0 files:path
            (10, 2, 12, None),  # e0 files:digest
        ]
    )
    archive = tmp_path / "unsafe-path.gts"
    archive.write_bytes(w.to_bytes())
    return archive


def test_pack_round_trips_bit_for_bit(tmp_path: Path) -> None:
    src = _make_tree(tmp_path)
    archive = tmp_path / "out.gts"
    assert main(["pack", str(src), "-o", str(archive)]) == 0

    dst = tmp_path / "dst"
    assert main(["unpack", str(archive), "-C", str(dst)]) == 0
    assert (dst / "a.txt").read_text() == "hello"
    assert (dst / "subdir" / "b.txt").read_text() == "world"

    # Identical tree produces identical archive bytes.
    archive2 = tmp_path / "out2.gts"
    assert main(["pack", str(dst), "-o", str(archive2)]) == 0
    assert archive.read_bytes() == archive2.read_bytes()


def test_pack_deduplicates_identical_content(tmp_path: Path) -> None:
    from gts.reader import read

    src = tmp_path / "src"
    src.mkdir()
    (src / "a.txt").write_text("shared")
    (src / "b.txt").write_text("shared")
    archive = tmp_path / "out.gts"
    assert main(["pack", str(src), "-o", str(archive)]) == 0
    g = read(archive.read_bytes())
    # Two FileEntries but one inline blob.
    assert len(g.blobs) == 1


def test_unpack_refuses_traversal(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    # Unpack must refuse the traversal path before it complains about the
    # missing blob.
    archive = _files_archive_with_path(tmp_path, "../escape.txt")

    assert main(["unpack", str(archive), "-C", str(tmp_path / "dst")]) == 1
    stderr = capsys.readouterr().err
    assert "traversal" in stderr or "escapes" in stderr, stderr


def test_unpack_refuses_windows_style_paths(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    archive = _files_archive_with_path(tmp_path, "..\\..\\etc\\passwd")
    assert main(["unpack", str(archive), "-C", str(tmp_path / "dst")]) == 1
    assert "traversal" in capsys.readouterr().err

    archive = _files_archive_with_path(tmp_path, "C:\\secret.txt")
    assert main(["unpack", str(archive), "-C", str(tmp_path / "dst2")]) == 1
    assert "drive-relative" in capsys.readouterr().err


def test_unpack_refuses_destination_symlink_escape(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    dest = tmp_path / "dst"
    outside = tmp_path / "outside"
    dest.mkdir()
    outside.mkdir()
    try:
        (dest / "link").symlink_to(outside, target_is_directory=True)
    except (NotImplementedError, OSError) as exc:
        pytest.skip(f"symlink creation unavailable: {exc}")

    archive = _files_archive_with_path(tmp_path, "link/escape.txt")
    assert main(["unpack", str(archive), "-C", str(dest)]) == 1
    assert "escapes" in capsys.readouterr().err
    assert not (outside / "escape.txt").exists()


def test_unpack_refuses_leaf_symlink_redirect(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    dest = tmp_path / "dst"
    outside = tmp_path / "outside"
    dest.mkdir()
    outside.mkdir()
    try:
        (dest / "target.txt").symlink_to(outside / "escape.txt")
    except (NotImplementedError, OSError) as exc:
        pytest.skip(f"symlink creation unavailable: {exc}")

    archive = _files_archive_with_path(tmp_path, "target.txt")
    assert main(["unpack", str(archive), "-C", str(dest)]) == 1
    stderr = capsys.readouterr().err
    assert "symlink" in stderr or "escapes" in stderr, stderr
    assert not (outside / "escape.txt").exists()


def test_pack_refuses_symlink_entry(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    src = _make_tree(tmp_path)
    link = src / "linked.txt"
    try:
        link.symlink_to(src / "a.txt")
    except (NotImplementedError, OSError) as exc:
        pytest.skip(f"symlink creation unavailable: {exc}")

    archive = tmp_path / "out.gts"
    assert main(["pack", str(src), "-o", str(archive)]) == 1
    assert "symlink" in capsys.readouterr().err


def test_diff_refuses_symlink_entry(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    src = _make_tree(tmp_path)
    archive = tmp_path / "out.gts"
    assert main(["pack", str(src), "-o", str(archive)]) == 0

    link = src / "linked.txt"
    try:
        link.symlink_to(src / "a.txt")
    except (NotImplementedError, OSError) as exc:
        pytest.skip(f"symlink creation unavailable: {exc}")

    assert main(["diff", str(archive), str(src)]) == 1
    assert "symlink" in capsys.readouterr().err


def test_diff_reports_changes(tmp_path: Path) -> None:
    src = _make_tree(tmp_path)
    archive = tmp_path / "out.gts"
    assert main(["pack", str(src), "-o", str(archive)]) == 0

    # Identical directory -> no differences.
    assert main(["diff", str(archive), str(src)]) == 0

    # Add a file.
    (src / "new.txt").write_text("new")
    assert main(["diff", str(archive), str(src)]) == 1
    # (specific reports tested via direct diff() unit tests)


def test_unpack_skips_suppressed_blob_by_default(tmp_path: Path) -> None:
    src = tmp_path / "src"
    src.mkdir()
    (src / "secret.txt").write_text("secret")
    archive = tmp_path / "out.gts"
    assert main(["pack", str(src), "-o", str(archive)]) == 0

    # Read the archive and append a suppression frame for the secret blob.
    data = bytearray(archive.read_bytes())
    from gts.wire import canonical, content_id, iter_items

    # Find the actual last frame id by walking items.
    items, _torn = iter_items(bytes(data))
    _off, last_item = items[-1]
    assert isinstance(last_item, dict)
    last_id = last_item["id"]
    frame = {
        "t": "suppress",
        "prev": last_id,
        "d": {"targets": [{"kind": "blob", "digest": digest_str(b"secret")}]},
    }
    frame["id"] = content_id(frame)
    data += canonical(frame)
    archive.write_bytes(data)

    dst = tmp_path / "dst"
    assert main(["unpack", str(archive), "-C", str(dst)]) == 0
    assert not (dst / "secret.txt").exists()
    assert main(["unpack", str(archive), "-C", str(dst), "--include-suppressed"]) == 0
    assert (dst / "secret.txt").read_text() == "secret"


def test_cat_refuses_suppress_everything_composition(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    # Vector 21: a structurally valid file where the second segment suppresses
    # every prior quad. Raw byte concat is valid GTS, but gts cat refuses it.
    v21 = VECTORS_DIR / "21-degenerate-composition.gts"
    out = tmp_path / "out.gts"
    assert main(["cat", str(v21), str(v21), "-o", str(out)]) == 1
    err = capsys.readouterr().err
    assert "hide every quad" in err


def test_verify_checks_declared_vs_computed_profiles(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    # Uses files# vocabulary without declaring the files profile -> error.
    w = Writer(profile="generic")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://w3id.org/gts/files#FileEntry"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#path"),
            Term(TermKind.LITERAL, "x.txt"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "undeclared-files.gts"
    path.write_bytes(w.to_bytes())
    assert main(["verify", str(path)]) == 1
    err = capsys.readouterr().err
    assert "profile error" in err


def test_verify_warns_on_declared_but_unused_profile(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    # Declares files profile but uses no files# vocabulary -> warning.
    w = Writer(profile="files")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/Cat"),
            Term(TermKind.IRI, "https://example.org/label"),
            Term(TermKind.LITERAL, "Cat", lang="en"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "unused-files.gts"
    path.write_bytes(w.to_bytes())
    # Warnings do not make verify exit nonzero.
    assert main(["verify", str(path)]) == 0
    err = capsys.readouterr().err
    assert "profile warning" in err


def test_verify_flags_undeclared_files_profile_object_only(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    # Regression: profile vocabulary in ordinary object position must be
    # detected, not only rdf:type objects (§14.1).
    w = Writer(profile="generic")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/Thing"),
            Term(TermKind.IRI, "https://example.org/relatedTo"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#FileEntry"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "undeclared-files-object-only.gts"
    path.write_bytes(w.to_bytes())
    assert main(["verify", str(path)]) == 1
    err = capsys.readouterr().err
    assert "profile error" in err


def test_verify_declared_files_profile_object_only_is_not_unused(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    # A declared profile whose term appears only as an object IRI must not
    # trigger the "declared but unused" warning.
    w = Writer(profile="files")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/Thing"),
            Term(TermKind.IRI, "https://example.org/relatedTo"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#FileEntry"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "declared-files-object-only.gts"
    path.write_bytes(w.to_bytes())
    assert main(["verify", str(path)]) == 0
    err = capsys.readouterr().err
    assert "profile warning" not in err


# --------------------------------------------------------------------------- #
# gts compact --streamable (§10.1, §14.1) + layout reporting (§3.3)
# --------------------------------------------------------------------------- #


def _accretive_file(tmp_path: Path) -> Path:
    w = Writer()
    w.add_blob(b"Z" * 64, mt="application/octet-stream")  # blob before catalog
    w.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/Cat"),
            Term(TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
            Term(TermKind.LITERAL, "Cat", lang="en"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "accretive.gts"
    path.write_bytes(w.to_bytes())
    return path


def test_compact_requires_streamable_flag(tmp_path: Path) -> None:
    path = _accretive_file(tmp_path)
    assert main(["compact", str(path), "-o", str(tmp_path / "x.gts")]) == 2


def test_compact_verify_info_round_trip(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    path = _accretive_file(tmp_path)
    out = tmp_path / "streamable.gts"
    assert (
        main(
            [
                "compact",
                str(path),
                "-o",
                str(out),
                "--streamable",
                "--timestamp",
                "2026-01-01T00:00:00Z",
            ]
        )
        == 0
    )
    capsys.readouterr()  # isolate the verify assertions from compact output
    assert main(["verify", str(out)]) == 0
    captured = capsys.readouterr()
    assert "layout: streamable through frame" in captured.out
    assert "accretive tail" not in captured.out
    assert "warning" not in captured.err


def test_compact_is_reproducible_with_fixed_timestamp(tmp_path: Path) -> None:
    path = _accretive_file(tmp_path)
    a, b = tmp_path / "a.gts", tmp_path / "b.gts"
    args = ["compact", str(path), "--streamable", "--timestamp", "2026-01-01T00:00:00Z"]
    assert main([*args, "-o", str(a)]) == 0
    assert main([*args, "-o", str(b)]) == 0
    assert a.read_bytes() == b.read_bytes()


def test_verify_refuses_streamable_lie(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    from gts.vectors import _streamable_lie

    path = tmp_path / "lie.gts"
    path.write_bytes(_streamable_lie())
    assert main(["verify", str(path)]) == 1
    assert "StreamableLayoutError" in capsys.readouterr().out


def test_info_reports_accretive_tail(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    from gts.vectors import _streamable_tail

    path = tmp_path / "tailed.gts"
    path.write_bytes(_streamable_tail())
    assert main(["info", str(path)]) == 0
    out = capsys.readouterr().out
    assert "layout: streamable through frame" in out
    assert "accretive tail 2 frame(s)" in out


def test_verify_warns_on_stream_vocab_without_claim(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """§13.3: stream# provenance in an unclaimed segment is a warning, never
    an error — it legitimately survives nq → gts round trips."""
    from gts.stream import COMPACT_AGENT, COMPACTION

    w = Writer()
    w.add_terms(
        [
            Term(TermKind.BNODE, "c"),
            Term(TermKind.IRI, COMPACTION),
            Term(TermKind.LITERAL, COMPACT_AGENT),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "unclaimed-stream.gts"
    path.write_bytes(w.to_bytes())
    assert main(["verify", str(path)]) == 0  # warning, exit stays 0
    err = capsys.readouterr().err
    assert "layout warning" in err


def test_compact_refusal_exits_one(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    w = Writer(profile="evidence")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/Cat"),
            Term(TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
            Term(TermKind.LITERAL, "Cat", lang="en"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    path = tmp_path / "evidence.gts"
    path.write_bytes(w.to_bytes())
    out = tmp_path / "out.gts"
    assert main(["compact", str(path), "-o", str(out), "--streamable"]) == 1
    assert "seal-original" in capsys.readouterr().err
    assert (
        main(["compact", str(path), "-o", str(out), "--streamable", "--seal-original"])
        == 0
    )
    assert main(["verify", str(out)]) == 0


def test_compact_seal_original_extracts_verbatim(tmp_path: Path) -> None:
    """§10.1: the sealed original extracts by digest and re-reads byte-intact."""
    path = _accretive_file(tmp_path)
    src = path.read_bytes()
    out = tmp_path / "sealed.gts"
    assert (
        main(["compact", str(path), "-o", str(out), "--streamable", "--seal-original"])
        == 0
    )
    extracted = tmp_path / "original.gts"
    assert (
        main(
            [
                "extract",
                str(out),
                digest_str(src),
                "-o",
                str(extracted),
                "--mt",
                "application/vnd.blackcat.gts+cbor-seq",
            ]
        )
        == 0
    )
    assert extracted.read_bytes() == src


def test_verify_scans_the_graph_slot_for_vocab(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """§14.1: a vocabulary IRI used only as a quad's GRAPH NAME still rots a
    declaration — the scans cover all four term positions."""
    w = Writer()
    w.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/Cat"),
            Term(TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
            Term(TermKind.LITERAL, "Cat", lang="en"),
            Term(TermKind.IRI, "https://w3id.org/gts/files#graph"),
        ]
    )
    w.add_quads([(0, 1, 2, 3)])  # files# vocabulary in the graph slot ONLY
    path = tmp_path / "graph-slot.gts"
    path.write_bytes(w.to_bytes())
    assert main(["verify", str(path)]) == 1
    assert "profile error" in capsys.readouterr().err


def test_writer_rejects_unsupported_layout_claim() -> None:
    """§5: 'streamable' is the only layout this revision defines; a typo'd
    claim must fail at construction, not persist into the header."""
    with pytest.raises(ValueError, match="unsupported layout claim"):
        Writer(layout="streamabel")


def test_extract_key_prints_embedded_transport_key(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """§9.2: `gts extract-key` prints kid, fingerprint, emojihash, and the key."""
    from gts.crypto import Signer

    fx = Path(__file__).parent / "fixtures"
    pub = (fx / "test_key.pub.asc").read_text(encoding="utf-8")
    sec = (fx / "test_key.sec.asc").read_text(encoding="utf-8")
    signer = Signer.from_gpg_secret_key(sec)
    w = Writer(profile="dist", signer=signer)
    w.add_meta({"gts:transportKey": {"kid": signer.kid, "gpg": pub}})
    w.add_terms([Term(TermKind.IRI, CAT)])
    path = tmp_path / "signed.gts"
    path.write_bytes(w.to_bytes())

    assert main(["extract-key", str(path)]) == 0
    out = capsys.readouterr().out
    assert "BEGIN PGP PUBLIC KEY BLOCK" in out
    assert "kid:" in out
    assert "fingerprint:" in out
    assert "emojihash:" in out


def test_extract_key_missing_returns_1(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """An unsigned file has no embedded transport key — exit 1."""
    w = Writer()
    w.add_terms([Term(TermKind.IRI, CAT)])
    path = tmp_path / "plain.gts"
    path.write_bytes(w.to_bytes())

    assert main(["extract-key", str(path)]) == 1
    assert "no embedded transport key" in capsys.readouterr().err
