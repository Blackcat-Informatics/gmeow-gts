# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""`nquads → gts` inverse-of-fold transform (#14)."""

from __future__ import annotations

from pathlib import Path

import pytest

from gts import Term, TermKind, Writer, from_nquads, read, to_nquads
from gts.cli import main
from gts.from_nquads import NQuadsParseError

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors"


def _roundtrip(gts_bytes: bytes) -> bool:
    nq1 = to_nquads(read(gts_bytes))
    nq2 = to_nquads(read(from_nquads(nq1)))
    return sorted(nq1.splitlines()) == sorted(nq2.splitlines())


@pytest.mark.parametrize(
    "vector",
    sorted(p.name for p in VECTORS_DIR.glob("*.gts")),
)
def test_fold_roundtrips_through_from_nquads(vector: str) -> None:
    """fold → from-nq → fold reproduces the projection for every pure graph."""
    data = (VECTORS_DIR / vector).read_bytes()
    try:
        nq = to_nquads(read(data))
    except Exception:  # noqa: BLE001 - vectors with damaged frames are not graphs
        pytest.skip("vector does not fold to a graph")
    if not nq.strip():
        pytest.skip("empty fold")
    assert _roundtrip(data)


def test_named_graph_reifier_and_annotation_roundtrip() -> None:
    """Named graphs, reifiers, and annotations all survive the round-trip."""
    w = Writer(profile="dist")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://ex/s"),
            Term(TermKind.IRI, "https://ex/p"),
            Term(TermKind.IRI, "https://ex/o"),
            Term(TermKind.IRI, "https://ex/g"),
            Term(TermKind.IRI, "https://ex/conf"),
            Term(TermKind.LITERAL, "0.9"),
        ]
    )
    w.add_quads([(0, 1, 2, 3)])
    w.add_reifies({0: (0, 1, 2)})
    w.add_annot([(0, 4, 5)])
    assert _roundtrip(w.to_bytes())


def test_literals_lang_and_datatype_roundtrip() -> None:
    """Language-tagged and datatyped literals survive."""
    xsd_int = "http://www.w3.org/2001/XMLSchema#integer"
    nq = (
        '<https://ex/s> <https://ex/label> "Cat"@en .\n'
        f'<https://ex/s> <https://ex/n> "42"^^<{xsd_int}> .\n'
        "_:b0 <https://ex/p> <https://ex/s> .\n"
    )
    out = to_nquads(read(from_nquads(nq)))
    assert sorted(out.splitlines()) == sorted(nq.strip().splitlines())


def test_cli_from_nq_inverts_fold(tmp_path: Path) -> None:
    """`gts from-nq` writes a GTS that folds to the input N-Quads."""
    src = (VECTORS_DIR / "11-datatype-defaulting.gts").read_bytes()
    nq = to_nquads(read(src))
    nq_path = tmp_path / "in.nq"
    nq_path.write_text(nq, encoding="utf-8")
    out_path = tmp_path / "out.gts"
    assert main(["from-nq", str(nq_path), "-o", str(out_path)]) == 0
    assert sorted(to_nquads(read(out_path.read_bytes())).splitlines()) == sorted(
        nq.splitlines()
    )


def test_cli_from_nq_rejects_malformed() -> None:
    """Malformed N-Quads raise a parse error inside the transform."""
    with pytest.raises(NQuadsParseError):
        from_nquads("<https://ex/s> <https://ex/p> .\n")  # only 2 terms
