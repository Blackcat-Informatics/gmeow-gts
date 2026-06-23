# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Verification-family commands for the internal Python CLI."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import TYPE_CHECKING

from gts.cli_common import _has_problems, _load, _print_ledger
from gts.policy import TrustPolicy, evaluate_profile_policy
from gts.reader import read, read_segments

if TYPE_CHECKING:
    from gts.crypto import KeyProvider


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
            for seg in segments:
                for sig in seg.signatures:
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
