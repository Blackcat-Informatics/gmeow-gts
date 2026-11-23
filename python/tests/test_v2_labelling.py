# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise L1 line encoding and refinement with the normative worked examples."""

import pytest

from gts.v2.dictionary import Triple
from gts.v2.labelling import (
    Blank,
    Iri,
    LabellingInput,
    LabellingRefused,
    LabellingTerms,
    Literal,
    RecordLine,
)
from gts.v2.refinement import individualize, inherit_components, refine_top_components
from gts.v2.rows import AnnotationRow, EmptyGraphRow, QuadRow, ReifierBindingRow


def test_empty_sequence_has_the_l1_digest() -> None:
    source = LabellingInput(LabellingTerms(), ())
    assert not refine_top_components(source)
    assert source.positioned_sequence({}).digest.hex() == (
        "bf4b40a613c1cc97ababeccefc1c1c810dde52818d8928603f2180376f06fb5e"
    )


def test_path_uses_the_specified_colour_order_and_work() -> None:
    terms = LabellingTerms()
    p = terms.add(Iri(b"p"))
    a, b, c, d = (terms.add(Blank(node)) for node in (b"a", b"b", b"c", b"d"))
    source = LabellingInput(
        terms, [QuadRow(a, p, b), QuadRow(b, p, c), QuadRow(c, p, d)]
    )
    assert source.mask(0, {}).hex() == "4004010000000000000001700420"
    (path,) = refine_top_components(source)
    assert path.node_cells == ({b"a"}, {b"b"}, {b"c"}, {b"d"})
    assert path.line_cells == ({1}, {2}, {0})
    assert (path.start_units, path.refinement_units) == (9, 20)
    positions = {next(iter(cell)): i for i, cell in enumerate(path.node_cells)}
    assert source.positioned_sequence(positions).digest.hex() == (
        "15da0b8d7fc3f8829c516a8dc98fdf387a0854eae0c72b9041ff443a18fb27f8"
    )


def test_record_changes_the_cycle_partition_and_line_digest() -> None:
    terms = LabellingTerms()
    p = terms.add(Iri(b"p"))
    a, b, c, d = (terms.add(Blank(node)) for node in (b"a", b"b", b"c", b"d"))
    cycle = (QuadRow(a, p, b), QuadRow(b, p, c), QuadRow(c, p, d), QuadRow(d, p, a))
    cycle_input = LabellingInput(terms, cycle)
    (unmarked,) = refine_top_components(cycle_input)
    assert unmarked.node_cells == ({b"a", b"b", b"c", b"d"},)
    child = individualize(cycle_input, unmarked, {}, b"a")
    assert child.node_cells == ({b"a"}, {b"d"}, {b"c"}, {b"b"})

    source = LabellingInput(terms, (*cycle, RecordLine(b"m", (a,))))
    assert source.mask(4, {}).hex() == "4400000000000000016d000000000000000104"
    (marked,) = refine_top_components(source)
    assert marked.node_cells == ({b"d"}, {b"c"}, {b"b"}, {b"a"})
    positions = {next(iter(cell)): i for i, cell in enumerate(marked.node_cells)}
    assert source.positioned_sequence(positions).digest.hex() == (
        "66b0c9dbc78970596f2640853924262cd5e59f638956280bd8df10dfe79f977a"
    )


def test_lines_keep_counts_triples_and_empty_graphs_in_the_sequence() -> None:
    terms = LabellingTerms()
    g, r, s, o = (terms.add(Blank(node)) for node in (b"g", b"r", b"s", b"o"))
    p, q, h = (terms.add(Iri(node)) for node in (b"p", b"q", b"h"))
    v = terms.add(Literal(b"v", b"t"))
    t = terms.add(Triple(s, p, o))
    source = LabellingInput(
        terms,
        (
            QuadRow(s, p, o),
            ReifierBindingRow(r, t),
            AnnotationRow(2, r, q, v),
            EmptyGraphRow(g),
            QuadRow(g, q, r),
            EmptyGraphRow(h),
        ),
    )
    assert source.incidences(1, {b"s": 2}) == {b"r": (0,), b"o": (1,)}
    (component,) = refine_top_components(source)
    assert component.node_cells == ({b"g"}, {b"r"}, {b"s"}, {b"o"})
    assert component.line_cells == ({0}, {4}, {1}, {2}, {3})
    assert (component.start_units, component.refinement_units) == (18, 25)
    result = source.positioned_sequence({b"g": 0, b"r": 1, b"s": 2, b"o": 3})
    assert result.digest.hex() == (
        "efb4715babe1737213c03fdd7daaacf29d2400f6b132aa2455b7be06d14ecc3e"
    )


def test_input_rejects_repeated_annotation_ignoring_the_count() -> None:
    terms = LabellingTerms()
    iri = terms.add(Iri(b"p"))
    with pytest.raises(LabellingRefused) as error:
        LabellingInput(
            terms, [AnnotationRow(1, iri, iri, iri), AnnotationRow(2, iri, iri, iri)]
        )
    assert (error.value.kind, error.value.reason) == ("input", "repeated")


def test_fixing_a_hub_splits_components_with_inherited_cells() -> None:
    terms = LabellingTerms()
    a, b, hub = (terms.add(Blank(node)) for node in (b"a", b"b", b"hub"))
    p = terms.add(Iri(b"p"))
    source = LabellingInput(terms, [QuadRow(hub, p, a), QuadRow(hub, p, b)])
    (parent,) = refine_top_components(source)
    assert parent.node_cells == ({b"hub"}, {b"a", b"b"})
    left, right = inherit_components(source, parent, {b"hub": 0})
    assert left.node_cells == ({b"a"},) and left.line_cells == ({0},)
    assert right.node_cells == ({b"b"},) and right.line_cells == ({1},)
    assert left.refinement_units == right.refinement_units == 0


def test_input_rejects_a_carried_empty_graph_name() -> None:
    terms = LabellingTerms()
    name = terms.add(Blank(b""))
    iri = terms.add(Iri(b"p"))
    with pytest.raises(LabellingRefused) as error:
        LabellingInput(terms, [EmptyGraphRow(name), QuadRow(iri, iri, iri, name)])
    assert (error.value.kind, error.value.reason, error.value.blank) == (
        "graph",
        "blank",
        b"",
    )


def test_shared_deep_terms_exceed_width_before_expansion() -> None:
    terms = LabellingTerms()
    ref = terms.add(Blank(b"b"))
    p = terms.add(Iri(b"p"))
    for _ in range(100):
        ref = terms.add(Triple(ref, p, ref))
    with pytest.raises(LabellingRefused) as error:
        LabellingInput(terms, [RecordLine(b"", (ref,))])
    assert (error.value.kind, error.value.reason) == ("width", "occurrences")


def test_input_width_sums_free_encodings_and_bounds_each_fixed_line() -> None:
    terms = LabellingTerms()
    ref = terms.add(Blank(b"b"))
    p = terms.add(Iri(b""))
    for _ in range(59):
        ref = terms.add(Triple(ref, p, ref))
    source = LabellingInput(terms, [RecordLine(b"x", (ref,)), RecordLine(b"y", (ref,))])
    assert len(source) == 2


def test_lazy_line_input_can_intern_terms_before_yielding_their_ids() -> None:
    terms = LabellingTerms()

    def lines():
        yield RecordLine(b"", (terms.add(Blank(b"x")),))

    source = LabellingInput(terms, lines())
    assert source.incidences(0, {}) == {b"x": (0,)}


def test_discovery_skips_a_deep_shared_ground_term() -> None:
    terms = LabellingTerms()
    p = terms.add(Iri(b""))
    ground = p
    for _ in range(59):
        ground = terms.add(Triple(ground, p, ground))
    source = LabellingInput(
        terms, [RecordLine(b"x", (ground,)), RecordLine(b"y", (ground,))]
    )
    assert not refine_top_components(source)
