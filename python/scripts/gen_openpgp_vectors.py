# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regenerate the cross-engine OpenPGP key-parsing vectors (for `extract-key`).

Each engine must parse a GPG-armored Ed25519 public key the same way: extract the
raw 32-byte key, compute the OpenPGP v4 fingerprint (SHA-1), and derive the
emojihash. Reproducible:

    uv run python scripts/gen_openpgp_vectors.py
    git diff --exit-code ../vectors/openpgp
"""

from __future__ import annotations

import contextlib
import io
import json
from pathlib import Path

from cryptography.hazmat.primitives import serialization

from gts import Term, TermKind, Writer
from gts.cli import main as cli_main
from gts.crypto import Signer
from gts.emojihash import emojihash
from gts.openpgp import load_public_key, public_key_fingerprint
from gts.verify import format_fingerprint

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "openpgp"
FIXTURES = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
CAT = "https://example.org/Cat"


def _key_vector(armored: str) -> dict[str, str]:
    raw = load_public_key(armored).public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    fp = public_key_fingerprint(armored)
    return {
        "armored": armored,
        "raw_pub": raw.hex(),
        "fingerprint": fp,
        "fingerprint_grouped": format_fingerprint(fp),
        "emojihash": emojihash(raw),
    }


def _extract_key_vector(pub: str, sec: str, tmp: Path) -> dict[str, str]:
    """A signed GTS with an embedded transport key + the exact `extract-key` stdout.

    Ed25519 is deterministic and the Writer is deterministic, so the signed
    bytes (and thus the CLI output) are reproducible across every engine.
    """
    signer = Signer.from_gpg_secret_key(sec)
    w = Writer(profile="dist", signer=signer)
    w.add_meta({"gts:transportKey": {"kid": signer.kid, "gpg": pub}})
    w.add_terms([Term(TermKind.IRI, CAT)])
    data = w.to_bytes()

    path = tmp / "signed.gts"
    path.write_bytes(data)
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = cli_main(["extract-key", str(path)])
    assert rc == 0, rc
    return {"gts": data.hex(), "kid": signer.kid, "stdout": buf.getvalue()}


def main() -> None:
    import tempfile

    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    pub = (FIXTURES / "test_key.pub.asc").read_text(encoding="utf-8")
    sec = (FIXTURES / "test_key.sec.asc").read_text(encoding="utf-8")

    (VECTORS_DIR / "test-key.json").write_text(
        json.dumps(_key_vector(pub), indent=2, sort_keys=True, ensure_ascii=False)
        + "\n",
        encoding="utf-8",
    )
    with tempfile.TemporaryDirectory() as td:
        ek = _extract_key_vector(pub, sec, Path(td))
    (VECTORS_DIR / "extract-key.json").write_text(
        json.dumps(ek, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"wrote OpenPGP vectors to {VECTORS_DIR}")


if __name__ == "__main__":
    main()
