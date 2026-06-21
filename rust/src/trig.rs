// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `gts -> trig` transform.
//!
//! TriG is the readable Turtle-family counterpart to the N-Quads projection.
//! This module preserves the same folded RDF 1.2 content as
//! [`crate::nquads::to_nquads`] while grouping named-graph quads into graph
//! blocks.

use crate::model::{Graph, TermKind};
use crate::nquads::{escape_literal, render_term};

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

fn render_trig_term(g: &Graph, tid: usize) -> String {
    let t = &g.terms[tid];
    match t.kind {
        TermKind::Iri if t.value.as_deref() == Some(RDF_REIFIES) => "rdf:reifies".to_string(),
        TermKind::Iri => format!("<{}>", t.value.as_deref().unwrap_or("")),
        TermKind::Bnode => match &t.value {
            Some(v) => format!("_:{v}"),
            None => format!("_:b{tid}"),
        },
        TermKind::Literal => {
            let lit = format!("\"{}\"", escape_literal(t.value.as_deref().unwrap_or("")));
            if let Some(lang) = &t.lang {
                match t.direction.as_deref() {
                    Some(direction) => format!("{lit}@{lang}--{direction}"),
                    None => format!("{lit}@{lang}"),
                }
            } else if let Some(dt) = t.datatype {
                format!("{lit}^^{}", render_trig_term(g, dt))
            } else {
                lit
            }
        }
        TermKind::Triple => match t.reifier.and_then(|rf| g.reifier(rf)) {
            Some((s, p, o)) => format!(
                "<<( {} {} {} )>>",
                render_trig_term(g, s),
                render_trig_term(g, p),
                render_trig_term(g, o)
            ),
            None => render_term(g, tid),
        },
    }
}

fn close_graph(out: &mut Vec<String>, open_graph: &mut Option<String>) {
    if open_graph.take().is_some() {
        out.push("}".to_string());
    }
}

/// Serialise a folded [`Graph`] to TriG text.
pub fn to_trig(g: &Graph) -> String {
    if g.quads.is_empty() && g.reifiers.is_empty() && g.annotations.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("@prefix rdf: <{RDF_NS}> ."), String::new()];
    let mut open_graph: Option<String> = None;

    for &(s, p, o, gname) in &g.quads {
        let triple = format!(
            "{} {} {} .",
            render_trig_term(g, s),
            render_trig_term(g, p),
            render_trig_term(g, o)
        );
        if let Some(gid) = gname {
            let graph = render_trig_term(g, gid);
            if open_graph.as_deref() != Some(graph.as_str()) {
                close_graph(&mut lines, &mut open_graph);
                lines.push(format!("{graph} {{"));
                open_graph = Some(graph);
            }
            lines.push(format!("  {triple}"));
        } else {
            close_graph(&mut lines, &mut open_graph);
            lines.push(triple);
        }
    }

    close_graph(&mut lines, &mut open_graph);

    for &(rid, (s, p, o)) in &g.reifiers {
        let quoted = format!(
            "<<( {} {} {} )>>",
            render_trig_term(g, s),
            render_trig_term(g, p),
            render_trig_term(g, o)
        );
        lines.push(format!(
            "{} rdf:reifies {quoted} .",
            render_trig_term(g, rid)
        ));
    }
    for &(r, p, v) in &g.annotations {
        lines.push(format!(
            "{} {} {} .",
            render_trig_term(g, r),
            render_trig_term(g, p),
            render_trig_term(g, v)
        ));
    }

    format!("{}\n", lines.join("\n"))
}
