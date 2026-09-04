// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reifier coherence for the fold (§7.3, §7.8).
//!
//! `rdf:reifies` is not functional, so the `reifies` frame is a multi-valued
//! statement layer: several triples MAY bind to one reifier id and no reader
//! chooses among them. Two incoherent shapes remain, and both are decided here
//! rather than inside the frame loop:
//!
//! The one incoherent shape decided here is a legacy `"tt"`-less term over a
//! reifier that binds more than one triple: one term asking for two meanings,
//! which is `ConflictingReifier`.
//!
//! A `reifies` row that makes a term resolve THROUGH ITSELF is deliberately not
//! rejected here. Every row projects (§7.3), so dropping it would lose a
//! statement; termination is handled in `Graph::triple_of`, which reports a
//! self-reaching term as stating no triple for the walk.

use crate::model::{Diagnostic, Graph, TermKind};

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
