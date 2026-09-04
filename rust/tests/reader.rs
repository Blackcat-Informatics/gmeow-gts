// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

fn literal(value: &str, datatype: Option<usize>) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_string()),
        datatype,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

fn triple(reifier: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(reifier),
        triple: None,
    }
}

fn read_without_panic(data: &[u8]) -> Graph {
    std::panic::catch_unwind(|| read(data, true, None)).expect("public reader must not panic")
}

fn diagnostic_codes(graph: &Graph) -> Vec<&str> {
    graph
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn multi_segment_union_preserves_literal_datatype_mapping() {
    let datatype_iri = "http://www.w3.org/2001/XMLSchema#integer";
    let mut first = Writer::new("dist");
    first.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        iri(datatype_iri),
        literal("7", Some(2)),
    ]);
    first.add_quads(&[(0, 1, 3, None)]);

    let second = Writer::new("dist");
    let mut data = first.to_bytes();
    data.extend(second.to_bytes());

    let graph = read(&data, true, None);

    assert!(graph.diagnostics.is_empty());
    assert_eq!(graph.quads.len(), 1);
    let object = graph.quads[0].2;
    assert_eq!(graph.terms[object].kind, TermKind::Literal);
    assert_eq!(graph.terms[object].value.as_deref(), Some("7"));
    let datatype = graph.terms[object].datatype.expect("literal datatype");
    assert_eq!(graph.terms[datatype].value.as_deref(), Some(datatype_iri));
}

#[test]
fn public_reader_reports_malformed_input_diagnostics_without_panicking() {
    assert_eq!(
        diagnostic_codes(&read_without_panic(&[])),
        vec!["EmptyFile"]
    );
    assert_eq!(
        diagnostic_codes(&read_without_panic(&[0x01])),
        vec!["DamagedFrame"]
    );

    let writer = Writer::new("generic");
    let mut torn = writer.to_bytes();
    torn.push(0xa3);

    assert_eq!(
        diagnostic_codes(&read_without_panic(&torn)),
        vec!["TornAppendError"]
    );
}

// §7.3 resolution MUST terminate, and the normative strategy is degradation:
// a term that reaches itself states NO triple for that walk. The row is NOT
// dropped — every `reifies` row projects — so these assert retention plus a
// terminating projection. The reifier is an IRI on purpose so the separate
// reifier-position rule cannot mask the termination guard.
#[test]
fn self_reaching_quoted_triple_reifier_degrades_and_keeps_its_row() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        iri("https://example.org/reifier"),
        iri("https://example.org/predicate"),
        iri("https://example.org/object"),
        triple(0), // 3: legacy k:3 resolving through reifier 0
    ]);
    // The row names term 3, which is the very term resolving through reifier 0.
    writer.add_reifies(&[(0, (3, 1, 2), None)]);

    let graph = read(&writer.to_bytes(), true, None);

    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    assert_eq!(
        graph.reifiers,
        vec![(0, (3, 1, 2), None)],
        "every reifies row projects; none is dropped"
    );
    assert_eq!(graph.triple_of(3), None, "term 3 reaches itself");
    // Terminates, and the unresolvable term renders as a fresh blank node.
    assert!(to_nquads(&graph).contains("_:unbound_triple_3"));
}

#[test]
fn indirect_self_reaching_quoted_triple_reifier_degrades() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        iri("https://example.org/reifier-a"),
        iri("https://example.org/reifier-b"),
        iri("https://example.org/predicate"),
        iri("https://example.org/object"),
        triple(0), // 4: resolves through reifier 0
        triple(1), // 5: resolves through reifier 1
    ]);
    // 4 -> (5, …) -> (4, …) is a two-step cycle through two IRI reifiers.
    writer.add_reifies(&[(0, (5, 2, 3), None), (1, (4, 2, 3), None)]);

    let graph = read(&writer.to_bytes(), true, None);

    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    assert_eq!(graph.reifiers.len(), 2, "both rows are retained");
    assert_eq!(graph.triple_of(4), None);
    assert_eq!(graph.triple_of(5), None);
    let _ = to_nquads(&graph); // must terminate
}

/// §7.3: the reifier id lands in subject position, where RDF 1.2 admits only an
/// IRI or blank node. A `k:3` reifier folded fine and then each projection
/// improvised — the RDF adapter errored, N-Quads and TriG silently dropped the
/// row, and Go emitted a triple term as subject. Reject it once, at fold time.
#[test]
fn reader_rejects_quoted_triple_in_reifier_position() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        iri("https://example.org/subject"),
        iri("https://example.org/predicate"),
        iri("https://example.org/object"),
        triple(0), // 3: a k:3 term used as the REIFIER below
    ]);
    writer.add_reifies(&[(3, (0, 1, 2), None)]);

    let graph = read(&writer.to_bytes(), true, None);

    assert!(
        graph
            .diagnostics
            .iter()
            .any(|d| d.code == "PositionConstraint"
                && d.detail.contains("must be an IRI or blank node")),
        "diagnostics = {:?}",
        graph.diagnostics
    );
    assert!(graph.reifiers.is_empty(), "the row must not fold");
    // Whatever survives must project without improvising.
    let _ = to_nquads(&graph);
}

// -- §7.3: self-describing triple terms and the multi-valued reifies layer ---

fn triple_term(spo: (usize, usize, usize), reifier: Option<usize>) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier,
        triple: Some(spo),
    }
}

/// A writer with `r`, `s`, `p`, `o`, `o2` and two `rdf:reifies` bindings on `r`.
fn two_bindings_on_one_reifier() -> Writer {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        Term {
            kind: TermKind::Bnode,
            value: Some("r".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        },
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        iri("https://example.org/o"),
        iri("https://example.org/o2"),
    ]);
    w.add_reifies(&[(0, (1, 2, 3), None)]);
    w.add_reifies(&[(0, (1, 2, 4), None)]);
    w
}

#[test]
fn reifies_frame_is_multi_valued() {
    let graph = read(&two_bindings_on_one_reifier().to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    assert_eq!(graph.reifier_triples(0), vec![(1, 2, 3), (1, 2, 4)]);
}

#[test]
fn identical_reifier_rows_still_collapse() {
    let mut w = two_bindings_on_one_reifier();
    w.add_reifies(&[(0, (1, 2, 3), None)]);
    let graph = read(&w.to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    assert_eq!(graph.reifiers.len(), 2);
}

#[test]
fn conflicting_reifier_is_only_the_legacy_indirect_term() {
    let mut w = two_bindings_on_one_reifier();
    w.add_terms(&[triple(0)]); // 5: legacy "tt"-less indirect term
    let graph = read(&w.to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), vec!["ConflictingReifier"]);
    // NO row is dropped, and the term still resolves to the first binding.
    assert_eq!(graph.reifiers.len(), 2);
    assert_eq!(graph.triple_of(5), Some((1, 2, 3)));
}

#[test]
fn self_describing_triple_terms_sharing_a_reifier_stay_distinct() {
    let mut w = two_bindings_on_one_reifier();
    w.add_terms(&[
        triple_term((1, 2, 3), Some(0)), // 5
        triple_term((1, 2, 4), Some(0)), // 6
    ]);
    let graph = read(&w.to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    assert_eq!(graph.triple_of(5), Some((1, 2, 3)));
    assert_eq!(graph.triple_of(6), Some((1, 2, 4)));
}

#[test]
fn unreified_triple_term_survives_a_round_trip() {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        iri("https://example.org/o"),
        triple_term((0, 1, 2), None), // 3: no reifier at all
        iri("https://example.org/says"),
    ]);
    w.add_quads(&[(0, 4, 3, None)]);
    let graph = read(&w.to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    assert!(graph.reifiers.is_empty());
    assert_eq!(
        to_nquads(&graph).trim(),
        "<https://example.org/s> <https://example.org/says> \
         <<( <https://example.org/s> <https://example.org/p> \
         <https://example.org/o> )>> ."
    );
}

#[test]
fn forward_referencing_tt_is_diagnosed_and_dropped() {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        // "tt" names term 3, which does not exist yet.
        triple_term((0, 1, 3), None),
    ]);
    let graph = read(&w.to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), vec!["ForwardReference"]);
    assert_eq!(graph.terms[2].triple, None);
}

#[test]
fn tt_with_a_literal_predicate_is_a_position_constraint() {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        iri("https://example.org/s"),
        literal("not-a-predicate", None),
        iri("https://example.org/o"),
        triple_term((0, 1, 2), None),
    ]);
    let graph = read(&w.to_bytes(), true, None);
    assert_eq!(diagnostic_codes(&graph), vec!["PositionConstraint"]);
    assert_eq!(graph.terms[3].triple, None);
}

/// A segment stating one triple term over the shared reifier `r1`, used as the
/// object of a quad so the union materialises it.
fn segment_with_triple_term(object: &str) -> Writer {
    let mut w = Writer::new("generic");
    w.add_terms(&[
        iri("https://example.org/r1"),   // 0 reifier
        iri("https://example.org/s"),    // 1
        iri("https://example.org/p"),    // 2
        iri(object),                     // 3
        triple_term((1, 2, 3), Some(0)), // 4
        iri("https://example.org/says"), // 5
    ]);
    w.add_reifies(&[(0, (1, 2, 3), None)]);
    w.add_quads(&[(1, 5, 4, None)]);
    w
}

#[test]
fn union_keeps_distinct_triple_terms_that_share_a_reifier() {
    // Two segments each state a DIFFERENT triple term over the SAME reifier
    // IRI. Under the reifier-indirection model these collapsed into one term
    // and one binding; self-describing terms keep both.
    let mut data = segment_with_triple_term("https://example.org/o").to_bytes();
    data.extend(segment_with_triple_term("https://example.org/o2").to_bytes());
    let graph = read(&data, true, None);

    assert_eq!(diagnostic_codes(&graph), Vec::<&str>::new());
    let resolved: Vec<Option<(usize, usize, usize)>> = (0..graph.terms.len())
        .filter(|&tid| graph.terms[tid].kind == TermKind::Triple)
        .map(|tid| graph.triple_of(tid))
        .collect();
    assert_eq!(resolved.len(), 2, "two distinct triple terms survive");
    assert_ne!(resolved[0], resolved[1]);
    // Both `rdf:reifies` statements survive the union too.
    let rid = graph.reifiers[0].0;
    assert_eq!(graph.reifier_triples(rid).len(), 2);
}

/// A `reifies` row may name the very term that resolves through it. Resolution
/// MUST terminate (§7.3): this reader rejects the self-reaching row at fold time
/// with `DamagedFrame`, so the shape never enters the graph and neither the
/// multi-segment union nor a projection can recurse on it.
#[test]
fn wire_constructed_self_reaching_reifier_stays_total() {
    let mut w = Writer::new("generic");
    w.add_terms(&[iri("https://example.org/r"), iri("https://example.org/p")]);
    w.add_terms(&[triple(0)]); // 2: legacy k:3 rf=0
    w.add_reifies(&[(0, (2, 1, 1), None)]);
    let data = w.to_bytes();

    let single = read(&data, true, None);
    assert_eq!(diagnostic_codes(&single), Vec::<&str>::new());
    assert_eq!(
        single.reifiers,
        vec![(0, (2, 1, 1), None)],
        "the row is folded and projects; only the TERM degrades"
    );
    assert_eq!(single.triple_of(2), None, "term 2 reaches itself");
    let _ = to_nquads(&single);

    // The union interns a triple term on its RESOLVED (s, p, o) (§7.3), so a
    // self-reaching term would recurse if the row had been folded.
    let mut doubled = data.clone();
    doubled.extend(data);
    let union = read(&doubled, true, None);
    let _ = to_nquads(&union);
}
