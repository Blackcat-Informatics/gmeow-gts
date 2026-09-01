// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reifier coherence for the fold (§7.3, §7.8).
//!
//! `rdf:reifies` is not functional, so the `reifies` frame is a multi-valued
//! statement layer: several triples MAY bind to one reifier id and no reader
//! chooses among them. Two incoherent shapes remain, and both are decided here
//! rather than inside the frame loop:
//!
//! * a `reifies` row that makes a quoted-triple term resolve THROUGH ITSELF is
//!   structurally damaged — RDF 1.2 triple terms are well founded — and is
//!   rejected at fold time so no downstream projection can chase the cycle;
//! * a legacy `"tt"`-less term over a reifier that binds more than one triple is
//!   one term asking for two meanings, which is `ConflictingReifier`.

use std::collections::HashSet;

use crate::model::{Diagnostic, Graph, TermKind, Triple3};

/// Can `term_id`'s resolution reach `anchor`, with `pending` not yet recorded?
fn term_depends_on_anchor(
    graph: &Graph,
    term_id: usize,
    anchor: usize,
    pending: (usize, Triple3),
    seen: &mut HashSet<usize>,
) -> bool {
    if term_id == anchor {
        return true;
    }
    if !seen.insert(term_id) {
        return false;
    }
    let Some(term) = graph.terms.get(term_id) else {
        return false;
    };
    if term.kind != TermKind::Triple {
        return false;
    }
    // A "tt" term is self-describing and its components are id-ordered, so it
    // cannot participate in a reifier cycle; only the legacy indirect form can.
    let binding = if let Some(spo) = term.triple {
        Some(spo)
    } else {
        let Some(reifier) = term.reifier else {
            return false;
        };
        if reifier == pending.0 {
            Some(pending.1)
        } else {
            graph.reifier(reifier)
        }
    };
    let Some((s, p, o)) = binding else {
        return false;
    };
    [s, p, o]
        .into_iter()
        .any(|component| term_depends_on_anchor(graph, component, anchor, pending, seen))
}

/// Would recording `(rid, triple)` make some term resolve through itself?
///
/// Anchors are the LEGACY `"tt"`-less terms bound to `rid`: those are the only
/// terms whose meaning this row can change (§7.3 step 2).
pub(crate) fn reifier_binding_is_recursive(graph: &Graph, rid: usize, triple: Triple3) -> bool {
    graph
        .terms
        .iter()
        .enumerate()
        .filter(|(_, term)| {
            term.kind == TermKind::Triple && term.triple.is_none() && term.reifier == Some(rid)
        })
        .any(|(anchor, _)| {
            [triple.0, triple.1, triple.2].into_iter().any(|component| {
                let mut seen = HashSet::new();
                term_depends_on_anchor(graph, component, anchor, (rid, triple), &mut seen)
            })
        })
}

/// Diagnose the one incoherent shape that survives §7.3, in place.
///
/// `rdf:reifies` is not functional, so a reifier id bound to several triples is
/// ordinary RDF 1.2 and is NOT a conflict. The single remaining incoherence is
/// a `"tt"`-less (legacy indirect) quoted-triple TERM whose reifier id binds
/// MORE THAN ONE distinct triple: the file is asking for one term with two
/// meanings. `ConflictingReifier` is reported once per offending term; NO
/// reifier row is dropped, and the term keeps resolving to the first binding in
/// file order so the rendering of legacy files never changes.
///
/// Runs on a fold that is about to be handed to a consumer (a whole read, or
/// one segment of a per-segment read), never twice over the same terms.
pub(crate) fn check_conflicting_reifiers(graph: &mut Graph) {
    let offenders: Vec<(usize, usize, usize)> = graph
        .terms
        .iter()
        .enumerate()
        .filter(|(_, term)| term.kind == TermKind::Triple && term.triple.is_none())
        .filter_map(|(tid, term)| term.reifier.map(|rid| (tid, rid)))
        .filter_map(|(tid, rid)| {
            let count = graph.reifier_triples(rid).len();
            (count > 1).then_some((tid, rid, count))
        })
        .collect();
    for (tid, rid, count) in offenders {
        graph.diagnostics.push(Diagnostic {
            code: "ConflictingReifier".to_string(),
            detail: format!(
                "legacy triple term {tid} resolves through reifier {rid}, which binds \
                 {count} distinct triples; state the triple with 'tt' (§7.3)"
            ),
            frame_index: None,
        });
    }
}
