# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check L1 certificates, positions and copy indices against the specification."""

from itertools import combinations, product

from gts.v2._certificates import (
    Call,
    canonical_map,
    compare_certificates,
    component_certificates,
)
from gts.v2._labelling_result import label_unlimited
from gts.v2._twins import TwinIndex
from gts.v2.labelling import Blank, Iri, LabellingInput, LabellingTerms, RecordLine
from gts.v2.rows import QuadRow


def _graph(arcs: list[tuple[bytes, bytes]]) -> LabellingInput:
    terms = LabellingTerms()
    predicate = terms.add(Iri(b"p"))
    return LabellingInput(
        terms,
        (QuadRow(terms.add(Blank(a)), predicate, terms.add(Blank(b))) for a, b in arcs),
    )


def test_cycle_copies_use_source_order_for_ties_but_share_an_identity() -> None:
    # Section 12.2 gives two copies of the directed triangle.
    source = _graph(
        [
            (b"z", b"x"),
            (b"y", b"z"),
            (b"x", b"y"),
            (b"b", b"c"),
            (b"c", b"a"),
            (b"a", b"b"),
        ]
    )
    components = component_certificates(source)
    identity = bytes.fromhex(
        "0642fe795c2b0f0ac68c93348399d176cbd6666baf179e35f5da142aaa8dc857"
    )
    for component in components:
        assert component.identity == identity
        assert component.call.target_size == component.call.attaining == 3
        assert component.refinement_units + component.call.refinement_units == 20
        assert component.call.fix_units == 12
    labels = canonical_map(components)
    assert list(labels) == [b"a", b"c", b"b", b"x", b"z", b"y"]
    for i, label in enumerate(labels.values()):
        assert (
            label.position,
            label.component,
            label.local_position,
            label.copy_index,
        ) == (
            i,
            identity,
            i % 3,
            i // 3,
        )
    result = source.positioned_sequence(
        {node: label.position for node, label in labels.items()}
    )
    assert result.digest.hex() == (
        "aad8fba9f3f9b55f69d2d2d7bdbc48f43f5164eaec2b947ce5870077c50501fd"
    )


def test_fixed_node_counts_distinguish_otherwise_identical_certificates() -> None:
    # Section 12.6 keeps the entries equal and changes the count at every level.
    first = list(product((b"a1", b"a2"), (b"b1", b"b2")))
    second = [(b"u", b"c1"), (b"v1", b"c1"), (b"u", b"c2"), (b"v2", b"c2")]
    source = _graph(first + second)
    left, right = components = component_certificates(source)
    assert (left.identity.hex(), right.identity.hex()) == (
        "317cfcc5349aad5e59854af7992d2355d81d3661a4ff5fd8baf3157b092c9cd1",
        "7868b0cc1be5dc2938479ebcf7b247340ec1f2336684c8e4e421ac4769ddb7a4",
    )
    a, b = (b"".join(component.call.certificate_chunks()) for component in components)
    assert [i for i, (x, y) in enumerate(zip(a, b, strict=True)) if x != y] == [
        7,
        31,
        131,
    ]
    assert compare_certificates(left.call, right.call) > 0
    assert sum(call.fix_units for call in left.call.preorder()) == 19
    assert sum(call.fix_units for call in right.call.preorder()) == 21
    labels = canonical_map(components)
    assert list(labels) == [
        b"a1",
        b"a2",
        b"b1",
        b"b2",
        b"u",
        b"v1",
        b"c1",
        b"v2",
        b"c2",
    ]
    result = source.positioned_sequence(
        {node: label.position for node, label in labels.items()}
    )
    assert result.digest.hex() == (
        "05ba28bf4bce28c3e4113f8de9adf8cbf549e6de2550db47de69ddbd9bc652f6"
    )


def test_tied_node_cell_can_have_nonattaining_children() -> None:
    # Section 6.8 gives a cubic graph with two distinct orbits in one cell.
    edges = [(b"a2", b"b2"), (b"a3", b"b3")]
    for prefix in (b"a", b"b"):
        nodes = [prefix + digit for digit in (b"0", b"1", b"2", b"3")]
        edges.extend(list(combinations(nodes, 2))[:-1])
    source = _graph(edges + [(b, a) for a, b in edges])
    (component,) = component_certificates(source)
    assert (component.call.target_size, component.call.attaining) == (8, 4)
    assert component.call.fixed[0] == b"a0"
    names = {b"a0": b"a2", b"a2": b"a0"}
    renamed_edges = [(names.get(a, a), names.get(b, b)) for a, b in edges]
    renamed = _graph(renamed_edges + [(b, a) for a, b in renamed_edges])
    (renamed_component,) = component_certificates(renamed)
    assert renamed_component.call.fixed[0] == b"a1"
    assert renamed_component.call.attaining == 4
    assert renamed_component.identity == component.identity


def test_certificate_operations_accept_nesting_beyond_the_recursion_limit() -> None:
    a = Call((b"a",), ((0, b"a"),), (), b"a", 0, 0)
    b = Call((b"b",), ((0, b"b"),), (), b"b", 0, 0)
    for _ in range(1200):
        a = Call((b"a",), (), (a,), b"a", 0, 0)
        b = Call((b"b",), (), (b,), b"b", 0, 0)
    assert compare_certificates(a, b) < 0
    assert b"".join(a.certificate_chunks()).endswith(b"a" + bytes(8))


def test_twin_charge_covers_the_union_of_affected_lines() -> None:
    # Section 8.1(f) prints both charges for the directed four-cycle.
    source = _graph([(b"a", b"b"), (b"b", b"c"), (b"c", b"d"), (b"d", b"a")])
    lines = range(len(source))
    twins = TwinIndex(
        source, lines, tuple(frozenset(source.incidences(line, {})) for line in lines)
    )
    assert twins.test(b"a", b"b") == (False, 9)
    assert twins.test(b"a", b"c") == (False, 12)


def test_nested_twin_class_counts_every_attaining_child() -> None:
    # The hub is fixed at the top; the complete subgraph first branches below it.
    nodes = [bytes([name]) for name in b"abcdef"]
    edges = [(b"hub", node) for node in nodes]
    edges.extend((a, b) for a, b in product(nodes, repeat=2) if a != b)
    source = _graph(edges)
    (component,) = component_certificates(source)
    assert component.call.fixed == (b"hub",)
    branch_sizes = [
        (call.target_size, call.attaining) for call in component.call.preorder()
    ]
    assert branch_sizes == [(0, 0), (6, 6), (5, 5), (4, 4), (3, 3), (2, 2)]
    assert list(canonical_map((component,))) == [b"hub", *nodes]


def test_structural_record_counts_top_copies_and_repeated_failures() -> None:
    # Section 12.2 has two isomorphic components that each fail the twin test.
    source = _graph(
        [
            (b"a", b"b"),
            (b"b", b"c"),
            (b"c", b"a"),
            (b"x", b"y"),
            (b"y", b"z"),
            (b"z", b"x"),
        ]
    )
    result = label_unlimited(source)
    stats = result.statistics
    assert (stats.calls, stats.chosen_children) == ((0, 0, 2), (2, 0))
    assert (stats.size_histogram, stats.top_multiplicities) == (((1, 2),), (2,))
    assert stats.group_order == ((0, 3, 2), (1, 2, 1))
    assert (stats.units, stats.allowance) == ((18, 40, 30, 30), 318)
    assert stats.broken_by == ((0, 3, 3, 3), (0, 3, 3, 3))
    assert result.diagnostic == (((0, 3, 3, 3), b"a", b"a"), ((0, 3, 3, 3), b"x", b"x"))
    assert not stats.guaranteed
    assert result.sequence.digest.hex() == (
        "aad8fba9f3f9b55f69d2d2d7bdbc48f43f5164eaec2b947ce5870077c50501fd"
    )


def test_structural_record_counts_nested_symmetry_and_work_parts() -> None:
    # Section 12.6 has a branching component and a forest, each with equal siblings.
    arcs = list(product((b"a1", b"a2"), (b"b1", b"b2")))
    arcs.extend([(b"u", b"c1"), (b"v1", b"c1"), (b"u", b"c2"), (b"v2", b"c2")])
    result = label_unlimited(_graph(arcs))
    first, second = result.components
    assert first.dimensions.largest_cell == 2
    assert not first.dimensions.forest
    assert second.dimensions.forest
    assert (second.dimensions.singled_out, second.dimensions.unsettled) == (1, 4)
    assert [
        (part.nestings, part.nesting_depth, part.branching_depth)
        for part in result.components
    ] == [
        (((2,),), 1, 1),
        (((2,),), 1, 0),
    ]
    assert (first.units, second.units) == ((12, 26, 19, 20), (12, 26, 21, 20))
    stats = result.statistics
    assert (stats.calls, stats.chosen_children) == ((4, 1, 1), (0, 1))
    assert stats.group_order == ((0, 2, 1), (1, 2, 2))
    assert (stats.units, stats.allowance) == ((24, 52, 48, 40), 476)
    assert (stats.blank_bearing_lines, stats.blank_bearing_bytes) == (8, 112)
    assert stats.guaranteed and not result.diagnostic


def test_search_free_failures_use_path_positions_in_each_sibling() -> None:
    cycles = [
        (b"b", b"c"),
        (b"c", b"d"),
        (b"d", b"b"),
        (b"e", b"f"),
        (b"f", b"g"),
        (b"g", b"e"),
    ]
    hub = [(b"a", node) for node, _ in cycles]
    result = label_unlimited(_graph(cycles + hub))
    failure = (1, 3, 7, 12)
    assert result.diagnostic == ((failure, b"a", b"b"), (failure, b"a", b"e"))
    assert result.statistics.broken_by == (failure, failure)


def test_structural_record_distinguishes_incidences_from_occurrences() -> None:
    terms = LabellingTerms()
    blank = terms.add(Blank(b"b"))
    source = LabellingInput(
        terms, [RecordLine(b"", (blank, blank)), RecordLine(b"", ())]
    )
    result = label_unlimited(source)
    (component,) = result.components
    assert component.dimensions.label_elements == 2
    assert component.dimensions.incidences == 1
    assert component.dimensions.line_bytes == 19
    assert component.dimensions.forest
    assert result.statistics.units == (4, 4, 3, 6)
    assert result.statistics.allowance == 39
    assert len(result.sequence.encodings) == 2
