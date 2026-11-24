// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{collections::BTreeSet, sync::Arc};

use super::{terms::resolve, Projector, Term, RDF_REIFIES};
use crate::v2::{Decoder, HeaderError, MalformedReason};

const QUADS: u64 = 2;
const REIFIER_BINDINGS: u64 = 3;
const ANNOTATIONS: u64 = 4;
const EMPTY_GRAPHS: u64 = 5;

fn run_values<'a, T: Copy + Eq>(
    cursor: &mut Decoder<'a>,
    count: u64,
    value: impl FnMut(&mut Decoder<'a>) -> Result<T, HeaderError>,
) -> Result<impl Iterator<Item = T>, HeaderError> {
    let mut runs = Vec::new();
    cursor.runs(count, value, |_, value, _, rows| {
        runs.push((value, rows));
        Ok(())
    })?;
    Ok(runs
        .into_iter()
        .flat_map(|(value, rows)| (0..rows).map(move |_| value)))
}

fn delta_column(cursor: &mut Decoder<'_>, count: u64) -> Result<Vec<u64>, HeaderError> {
    let mut previous = 0;
    cursor.column(count, |cursor| {
        previous = cursor.delta(previous)?;
        Ok(previous)
    })
}

impl Projector {
    pub(super) fn read_rows(
        &mut self,
        dictionary: &[usize],
        kind: u64,
        content: &[u8],
        bytes: &[u8],
    ) -> Result<(), HeaderError> {
        let mut fields = Decoder::new(content, 0);
        let mut cursor = Decoder::new(bytes, 0);
        if kind == EMPTY_GRAPHS {
            fields.record(1)?;
            let count = fields.positive()?;
            fields.end()?;
            cursor.record(1)?;
            let graphs = delta_column(&mut cursor, count)?;
            cursor.end()?;
            for graph in graphs {
                let graph = self.node(dictionary, graph, &cursor)?;
                self.projection.empty_graphs.insert(graph);
            }
            return Ok(());
        }

        fields.record(2)?;
        fields.code(2)?;
        let count = fields.positive()?;
        fields.end()?;
        let arity = match kind {
            QUADS => 4,
            ANNOTATIONS => 5,
            REIFIER_BINDINGS => 3,
            _ => return Err(fields.error(MalformedReason::UnregisteredValue)),
        };
        cursor.record(arity)?;
        let graphs = run_values(&mut cursor, count, |cursor| cursor.option(Decoder::uint))?;
        match kind {
            QUADS => {
                let predicates = run_values(&mut cursor, count, Decoder::uint)?;
                let objects = delta_column(&mut cursor, count)?;
                let subjects = delta_column(&mut cursor, count)?;
                cursor.end()?;
                let rows = graphs.zip(predicates).zip(objects).zip(subjects);
                for (((graph, predicate), object), subject) in rows {
                    let graph = self.resolve_graph(dictionary, graph, &cursor)?;
                    let subject = self.node(dictionary, subject, &cursor)?;
                    let predicate = self.iri(dictionary, predicate, &cursor)?;
                    let object = resolve(dictionary, object, &cursor)?;
                    self.projection
                        .quads
                        .insert((subject, predicate, object, graph));
                }
            }
            REIFIER_BINDINGS => {
                let reifiers = delta_column(&mut cursor, count)?;
                let triples = delta_column(&mut cursor, count)?;
                cursor.end()?;
                let predicate = self.intern(Term::Iri(Arc::from(RDF_REIFIES)));
                for ((graph, reifier), triple) in graphs.zip(reifiers).zip(triples) {
                    let graph = self.resolve_graph(dictionary, graph, &cursor)?;
                    let subject = self.node(dictionary, reifier, &cursor)?;
                    let object = self.position(dictionary, triple, &cursor, |term| {
                        matches!(term, Term::Triple(_))
                    })?;
                    self.projection
                        .quads
                        .insert((subject, predicate, object, graph));
                }
            }
            _ => {
                let reifiers = delta_column(&mut cursor, count)?;
                let predicates = delta_column(&mut cursor, count)?;
                let objects = delta_column(&mut cursor, count)?;
                let counts = delta_column(&mut cursor, count)?;
                cursor.end()?;
                if counts.contains(&0) {
                    return Err(cursor.error(MalformedReason::OutOfRange));
                }
                let mut annotations = BTreeSet::new();
                for (((graph, reifier), predicate), object) in
                    graphs.zip(reifiers).zip(predicates).zip(objects)
                {
                    let graph = self.resolve_graph(dictionary, graph, &cursor)?;
                    let subject = self.node(dictionary, reifier, &cursor)?;
                    let predicate = self.iri(dictionary, predicate, &cursor)?;
                    let object = resolve(dictionary, object, &cursor)?;
                    if !annotations.insert((subject, predicate, object, graph)) {
                        return Err(cursor.error(MalformedReason::UnorderedOrRepeated));
                    }
                }
                self.projection.quads.extend(annotations);
            }
        }
        Ok(())
    }

    fn resolve_graph(
        &self,
        dictionary: &[usize],
        graph: Option<u64>,
        cursor: &Decoder<'_>,
    ) -> Result<Option<usize>, HeaderError> {
        graph
            .map(|id| self.node(dictionary, id, cursor))
            .transpose()
    }
}
