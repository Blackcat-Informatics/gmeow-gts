// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A minimal GTS writer: build frames, maintain the id/prev chain, emit a CBOR
//! Sequence.
//!
//! This is the encoder counterpart to [`crate::reader`]. It currently supports
//! the frame types needed for the `files` profile (§13.2) and the conformance
//! vectors added with it. Deterministic CBOR and BLAKE3 self-hashes are handled
//! by [`crate::wire`].

use std::collections::HashMap;

use ciborium::value::Value;

use crate::codec::{Codec, CodecError};
use crate::model::{Graph, Quad, Suppression, Term, TermKind, Triple3};
use crate::wire::{canonical, content_id, digest_str, header_id, SELF_DESCRIBE_TAG};

fn iv(n: i64) -> Value {
    Value::Integer(ciborium::value::Integer::from(n))
}

/// Serialise a [`Term`] to its wire map (dropping absent fields).
pub fn term_to_wire(t: &Term) -> Value {
    let mut entries: Vec<(Value, Value)> = vec![("k".into(), iv(t.kind as i64))];
    if let Some(v) = &t.value {
        entries.push(("v".into(), v.clone().into()));
    }
    if let Some(dt) = t.datatype {
        entries.push(("dt".into(), iv(dt as i64)));
    }
    if let Some(l) = &t.lang {
        entries.push(("l".into(), l.clone().into()));
    }
    if let Some(rf) = t.reifier {
        entries.push(("rf".into(), iv(rf as i64)));
    }
    Value::Map(entries)
}

/// Accumulate a GTS log as a CBOR Sequence.
pub struct Writer {
    name_to_id: HashMap<String, i64>,
    prev: Vec<u8>,
    buf: Vec<u8>,
    // Per-frame byte offsets and types, in append order — the raw material
    // of an `index` footer (§6.2): offsets enable random access/parallel
    // verify, types the "ti" locator map.
    offsets: Vec<usize>,
    types: Vec<String>,
    frame_ids: Vec<Vec<u8>>,
    // When set, every appended frame is COSE_Sign1-signed over its id (§9.2).
    signer: Option<(ed25519_dalek::SigningKey, String)>,
}

impl Writer {
    /// Create a writer and emit the Header (the chain genesis).
    pub fn new(profile: &str) -> Self {
        Self::with_layout(profile, None)
    }

    /// Build a deterministic single-segment writer from folded graph state.
    ///
    /// This high-level authoring path remaps terms by semantic value, emits
    /// authorable graph frames in a fixed order, and relies on deterministic
    /// CBOR for every hashed frame. It does not replay reader observations such
    /// as diagnostics, signatures, opaque nodes, or segment ledgers.
    pub fn deterministic(graph: &Graph, profile: &str) -> Result<Self, CodecError> {
        let remap = deterministic_term_remap(graph);
        let mut writer = Self::new(profile);

        if !remap.old_by_new.is_empty() {
            let terms: Vec<Term> = remap
                .old_by_new
                .iter()
                .map(|&old| remap_term(&graph.terms[old], &remap.old_to_new))
                .collect();
            writer.add_terms(&terms);
        }

        let mut quads: Vec<Quad> = graph
            .quads
            .iter()
            .map(|&(s, p, o, g)| {
                (
                    remap_id(&remap.old_to_new, s),
                    remap_id(&remap.old_to_new, p),
                    remap_id(&remap.old_to_new, o),
                    g.map(|term| remap_id(&remap.old_to_new, term)),
                )
            })
            .collect();
        quads.sort_by_key(quad_key);
        if !quads.is_empty() {
            writer.add_quads(&quads);
        }

        let mut reifiers: Vec<(usize, Triple3)> = graph
            .reifiers
            .iter()
            .map(|&(rid, (s, p, o))| {
                (
                    remap_id(&remap.old_to_new, rid),
                    (
                        remap_id(&remap.old_to_new, s),
                        remap_id(&remap.old_to_new, p),
                        remap_id(&remap.old_to_new, o),
                    ),
                )
            })
            .collect();
        reifiers.sort();
        if !reifiers.is_empty() {
            writer.add_reifies(&reifiers);
        }

        let mut annotations: Vec<Triple3> = graph
            .annotations
            .iter()
            .map(|&(r, p, v)| {
                (
                    remap_id(&remap.old_to_new, r),
                    remap_id(&remap.old_to_new, p),
                    remap_id(&remap.old_to_new, v),
                )
            })
            .collect();
        annotations.sort();
        if !annotations.is_empty() {
            writer.add_annot(&annotations);
        }

        let mut blobs: Vec<(String, Vec<u8>)> = graph
            .blobs
            .iter()
            .map(|(digest, entry)| Ok((digest.clone(), entry.decoded_vec()?)))
            .collect::<Result<_, CodecError>>()?;
        blobs.sort_by(|a, b| a.0.cmp(&b.0));
        for (digest, data) in blobs {
            let meta = graph
                .blob_meta
                .iter()
                .find(|(candidate, _)| candidate == &digest)
                .map(|(_, meta)| meta);
            let mt = meta
                .and_then(|value| map_text(value, "mt"))
                .map(str::to_string);
            let rep = meta
                .and_then(|value| map_text(value, "rep"))
                .map(str::to_string);
            writer.add_blob(&data, mt.as_deref(), rep.as_deref());
        }

        if !graph.meta.is_empty() {
            let mut entries: Vec<(Value, Value)> = graph
                .meta
                .iter()
                .map(|(key, value)| (key.clone().into(), value.clone()))
                .collect();
            entries.sort_by_key(|(key, _)| canonical(key));
            writer.add_meta(Value::Map(entries));
        }

        let mut suppressions: Vec<Suppression> = graph
            .suppressions
            .iter()
            .map(|suppression| remap_suppression(suppression, &remap.old_to_new))
            .collect();
        suppressions.sort_by_key(suppression_key);
        for suppression in suppressions {
            writer.add_suppress(
                suppression.targets,
                suppression.reason.as_deref(),
                suppression.by,
            );
        }

        Ok(writer)
    }

    /// Build a writer from an Oxigraph store.
    ///
    /// This constructor is available only with `--features oxigraph-adapter`.
    #[cfg(feature = "oxigraph-adapter")]
    pub fn from_store(
        store: &::oxigraph::store::Store,
        profile: &str,
    ) -> Result<Self, crate::oxigraph::OxigraphAdapterError> {
        crate::oxigraph::store_to_writer(store, profile)
    }

    /// Create a writer with a header layout-state claim (§3.3;
    /// `"streamable"` is the only value this revision defines).
    pub fn with_layout(profile: &str, layout: Option<&str>) -> Self {
        // §5: "streamable" is the only layout this revision defines; a typo'd
        // claim would persist into the tamper-evident header.
        assert!(
            layout.is_none() || layout == Some("streamable"),
            "unsupported layout claim {layout:?} (§3.3)"
        );
        let catalog: HashMap<i64, Codec> = [
            (
                0i64,
                Codec {
                    name: "identity".to_string(),
                    cls: "encode".to_string(),
                },
            ),
            (
                1,
                Codec {
                    name: "gzip".to_string(),
                    cls: "compress".to_string(),
                },
            ),
            (
                2,
                Codec {
                    name: "zstd".to_string(),
                    cls: "compress".to_string(),
                },
            ),
            (
                3,
                Codec {
                    name: "zstd-rsyncable".to_string(),
                    cls: "compress".to_string(),
                },
            ),
            (
                7,
                Codec {
                    name: "cose-encrypt0".to_string(),
                    cls: "encrypt".to_string(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let name_to_id: HashMap<String, i64> = catalog
            .iter()
            .map(|(id, c)| (c.name.clone(), *id))
            .collect();

        let cat_entries: Vec<(Value, Value)> = catalog
            .iter()
            .map(|(id, c)| {
                let mut ce: Vec<(Value, Value)> = vec![
                    ("name".into(), c.name.clone().into()),
                    ("cls".into(), c.cls.clone().into()),
                ];
                ce.sort_by_key(|a| canonical(&a.0));
                (iv(*id), Value::Map(ce))
            })
            .collect();

        let mut header: Vec<(Value, Value)> = vec![
            ("gts".into(), "GTS1".into()),
            ("v".into(), iv(1)),
            ("prof".into(), profile.into()),
            ("cat".into(), Value::Map(cat_entries)),
        ];
        if let Some(layout) = layout {
            // The layout-state claim is part of the header content, so it is
            // covered by the genesis self-hash (§3.3, §5).
            header.push(("layout".into(), layout.into()));
        }
        header.sort_by_key(|a| canonical(&a.0));
        let id = header_id(&header);
        header.push(("id".into(), Value::Bytes(id.clone())));
        header.sort_by_key(|a| canonical(&a.0));

        let tagged = Value::Tag(SELF_DESCRIBE_TAG, Box::new(Value::Map(header)));
        let buf = canonical(&tagged);

        Self {
            name_to_id,
            prev: id,
            buf,
            offsets: Vec::new(),
            types: Vec::new(),
            frame_ids: Vec::new(),
            signer: None,
        }
    }

    /// Sign every subsequently appended frame's id with this Ed25519 key (§9.2).
    pub fn sign_with(&mut self, key: ed25519_dalek::SigningKey, kid: &str) {
        self.signer = Some((key, kid.to_string()));
    }

    /// Sign every subsequently appended frame with an unencrypted OpenPGP Ed25519 secret key.
    ///
    /// When `kid_override` is `None`, the COSE key id defaults to the key's
    /// OpenPGP v4 fingerprint.
    pub fn sign_with_openpgp_secret_key(
        &mut self,
        armored: &str,
        kid_override: Option<&str>,
    ) -> Result<(), crate::openpgp::OpenPgpError> {
        let signer = crate::openpgp::parse_secret_signing_key(armored, kid_override)?;
        let (key, kid) = signer.into_parts();
        self.sign_with(key, &kid);
        Ok(())
    }

    /// The id the next appended frame must reference as `"prev"`.
    pub fn head(&self) -> &[u8] {
        &self.prev
    }

    fn chain_ids(&self, chain: &[String]) -> Vec<i64> {
        chain.iter().map(|name| self.name_to_id[name]).collect()
    }

    /// Append one frame and return its `"id"`.
    pub fn add_frame(
        &mut self,
        frame_type: &str,
        payload: Option<Value>,
        raw: Option<Vec<u8>>,
        transform: Option<&[String]>,
        pub_meta: Option<Value>,
    ) -> Vec<u8> {
        assert!(
            payload.is_none() || raw.is_none(),
            "payload and raw are mutually exclusive"
        );
        let mut frame: Vec<(Value, Value)> = vec![("t".into(), frame_type.into())];

        let data: Option<Value> = match (transform, &payload, &raw) {
            (Some(chain), _, _) if !chain.is_empty() => {
                assert!(
                    raw.is_some() || payload.is_some(),
                    "transform requires a raw or payload source"
                );
                let source = match (raw.as_ref(), payload.as_ref()) {
                    (Some(r), _) => r.clone(),
                    (None, Some(p)) => canonical(p),
                    (None, None) => panic!("transform requires a raw or payload source"),
                };
                // For the files profile we only need identity; compression is
                // intentionally not implemented in this minimal writer.
                assert!(
                    chain.iter().all(|n| n == "identity"),
                    "non-identity transforms require the Python producer"
                );
                let x_ids: Vec<Value> = self.chain_ids(chain).into_iter().map(iv).collect();
                frame.push(("x".into(), Value::Array(x_ids)));
                Some(Value::Bytes(source))
            }
            (None, _, Some(r)) => Some(Value::Bytes(r.clone())),
            (None, Some(p), None) => Some(p.clone()),
            _ => None,
        };
        if let Some(data) = data {
            frame.push(("d".into(), data));
        }

        if let Some(meta) = pub_meta {
            frame.push(("pub".into(), meta));
        }
        frame.push(("prev".into(), Value::Bytes(self.prev.clone())));

        frame.sort_by_key(|a| canonical(&a.0));
        let id = content_id(&frame);
        frame.push(("id".into(), Value::Bytes(id.clone())));
        if let Some((key, kid)) = &self.signer {
            let sig = crate::cose::sign_id(&id, key, kid);
            frame.push(("sig".into(), Value::Bytes(sig)));
        }
        frame.sort_by_key(|a| canonical(&a.0));

        self.offsets.push(self.buf.len());
        self.types.push(frame_type.to_string());
        self.frame_ids.push(id.clone());
        self.buf.extend_from_slice(&canonical(&Value::Map(frame)));
        self.prev = id.clone();
        id
    }

    /// Append a `terms` frame.
    pub fn add_terms(&mut self, terms: &[Term]) -> Vec<u8> {
        let payload = Value::Array(terms.iter().map(term_to_wire).collect());
        self.add_frame("terms", Some(payload), None, None, None)
    }

    /// Append a `quads` frame (graph slot dropped when `None`).
    pub fn add_quads(&mut self, quads: &[Quad]) -> Vec<u8> {
        let rows: Vec<Value> = quads
            .iter()
            .map(|&(s, p, o, g)| {
                let mut row = vec![iv(s as i64), iv(p as i64), iv(o as i64)];
                if let Some(gv) = g {
                    row.push(iv(gv as i64));
                }
                Value::Array(row)
            })
            .collect();
        self.add_frame("quads", Some(Value::Array(rows)), None, None, None)
    }

    /// Append a `reifies` frame binding reifier-ids to triples.
    pub fn add_reifies(&mut self, bindings: &[(usize, Triple3)]) -> Vec<u8> {
        let mut map: Vec<(Value, Value)> = Vec::new();
        for (rid, (s, p, o)) in bindings {
            map.push((
                iv(*rid as i64),
                Value::Array(vec![iv(*s as i64), iv(*p as i64), iv(*o as i64)]),
            ));
        }
        self.add_frame("reifies", Some(Value::Map(map)), None, None, None)
    }

    /// Append an `annot` frame.
    pub fn add_annot(&mut self, rows: &[Triple3]) -> Vec<u8> {
        let rows: Vec<Value> = rows
            .iter()
            .map(|&(s, p, o)| Value::Array(vec![iv(s as i64), iv(p as i64), iv(o as i64)]))
            .collect();
        self.add_frame("annot", Some(Value::Array(rows)), None, None, None)
    }

    /// Append an inline `blob` frame; metadata goes in `pub` (§12).
    pub fn add_blob(&mut self, data: &[u8], mt: Option<&str>, rep: Option<&str>) -> Vec<u8> {
        let mut pub_entries: Vec<(Value, Value)> = vec![("digest".into(), digest_str(data).into())];
        if let Some(m) = mt {
            pub_entries.push(("mt".into(), m.into()));
        }
        if let Some(r) = rep {
            pub_entries.push(("rep".into(), r.into()));
        }
        let pub_meta = Some(Value::Map(pub_entries));
        self.add_frame("blob", None, Some(data.to_vec()), None, pub_meta)
    }

    /// Append a `meta` frame.
    pub fn add_meta(&mut self, meta: Value) -> Vec<u8> {
        self.add_frame("meta", Some(meta), None, None, None)
    }

    /// Append a `suppress` frame.
    pub fn add_suppress(
        &mut self,
        targets: Vec<Value>,
        reason: Option<&str>,
        by: Option<usize>,
    ) -> Vec<u8> {
        let mut payload: Vec<(Value, Value)> = vec![("targets".into(), Value::Array(targets))];
        if let Some(r) = reason {
            payload.push(("reason".into(), r.into()));
        }
        if let Some(b) = by {
            payload.push(("by".into(), Value::from(b as u64)));
        }
        payload.sort_by_key(|a| canonical(&a.0));
        self.add_frame("suppress", Some(Value::Map(payload)), None, None, None)
    }

    /// Append an `index` footer covering every frame appended so far (§6.2).
    ///
    /// `count`/`head` delimit the covered region (the streamable boundary,
    /// §3.3); `off` carries each covered frame's byte offset from the start
    /// of this writer's output; `ti` locates frames by type (0-based frame
    /// positions). A later `add_index` covers the earlier one too — the last
    /// index wins (§6.2).
    fn add_index_impl(&mut self, include_mmr: bool) -> Vec<u8> {
        let mut payload: Vec<(Value, Value)> = vec![
            ("count".into(), iv(self.types.len() as i64)),
            ("head".into(), Value::Bytes(self.prev.clone())),
        ];
        if include_mmr {
            payload.push((
                "mmr".into(),
                Value::Bytes(crate::mmr::root(&self.frame_ids)),
            ));
        }
        if !self.offsets.is_empty() {
            // "off"/"ti" are [+ uint]-shaped — omit when empty
            let off: Vec<Value> = self.offsets.iter().map(|&o| iv(o as i64)).collect();
            let mut ti: Vec<(Value, Value)> = Vec::new();
            for (pos, ftype) in self.types.iter().enumerate() {
                match ti
                    .iter_mut()
                    .find(|(k, _)| matches!(k, Value::Text(t) if t == ftype))
                {
                    Some((_, Value::Array(positions))) => positions.push(iv(pos as i64)),
                    _ => ti.push((ftype.clone().into(), Value::Array(vec![iv(pos as i64)]))),
                }
            }
            payload.push(("off".into(), Value::Array(off)));
            payload.push(("ti".into(), Value::Map(ti)));
        }
        self.add_frame("index", Some(Value::Map(payload)), None, None, None)
    }

    pub fn add_index(&mut self) -> Vec<u8> {
        self.add_index_impl(false)
    }

    /// Append an `index` footer with the optional `mmr` root over covered frame ids.
    ///
    /// This is opt-in so existing byte-oracle corpus vectors and cross-engine
    /// compact output remain stable until other engines claim the proof tier.
    pub fn add_index_with_mmr(&mut self) -> Vec<u8> {
        self.add_index_impl(true)
    }

    /// Return the complete GTS file bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.buf.clone()
    }
}

struct TermRemap {
    old_to_new: Vec<usize>,
    old_by_new: Vec<usize>,
}

fn deterministic_term_remap(graph: &Graph) -> TermRemap {
    let mut old_by_new: Vec<usize> = (0..graph.terms.len()).collect();
    let keys: Vec<Vec<u8>> = old_by_new
        .iter()
        .map(|&tid| canonical(&term_identity_value(graph, tid, &mut Vec::new())))
        .collect();
    old_by_new.sort_by(|a, b| keys[*a].cmp(&keys[*b]).then_with(|| a.cmp(b)));
    let mut old_to_new = vec![0; graph.terms.len()];
    for (new, old) in old_by_new.iter().enumerate() {
        old_to_new[*old] = new;
    }
    TermRemap {
        old_to_new,
        old_by_new,
    }
}

fn term_identity_value(graph: &Graph, tid: usize, stack: &mut Vec<usize>) -> Value {
    if stack.contains(&tid) {
        return Value::Array(vec!["cycle".into(), Value::from(tid as u64)]);
    }
    let Some(term) = graph.terms.get(tid) else {
        return Value::Array(vec!["missing".into(), Value::from(tid as u64)]);
    };
    stack.push(tid);
    let value = match term.kind {
        TermKind::Iri => Value::Array(vec!["iri".into(), text_or_null(term.value.as_deref())]),
        TermKind::Literal => Value::Array(vec![
            "literal".into(),
            text_or_null(term.value.as_deref()),
            graph.datatype_iri(term).into(),
            text_or_null(term.lang.as_deref()),
        ]),
        TermKind::Bnode => Value::Array(vec![
            "bnode".into(),
            match term.value.as_deref() {
                Some(value) if !value.is_empty() => value.into(),
                _ => Value::Array(vec!["anonymous".into(), Value::from(tid as u64)]),
            },
        ]),
        TermKind::Triple => match term.reifier.and_then(|rid| graph.reifier(rid)) {
            Some((s, p, o)) => Value::Array(vec![
                "triple".into(),
                term_identity_value(graph, s, stack),
                term_identity_value(graph, p, stack),
                term_identity_value(graph, o, stack),
            ]),
            None => Value::Array(vec![
                "triple".into(),
                Value::Null,
                term.reifier
                    .map(|rid| Value::from(rid as u64))
                    .unwrap_or(Value::Null),
            ]),
        },
    };
    stack.pop();
    value
}

fn text_or_null(value: Option<&str>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn remap_id(old_to_new: &[usize], tid: usize) -> usize {
    old_to_new.get(tid).copied().unwrap_or(tid)
}

fn remap_term(term: &Term, old_to_new: &[usize]) -> Term {
    Term {
        kind: term.kind,
        value: term.value.clone(),
        datatype: term.datatype.map(|tid| remap_id(old_to_new, tid)),
        lang: term.lang.clone(),
        reifier: term.reifier.map(|tid| remap_id(old_to_new, tid)),
    }
}

fn quad_key(quad: &Quad) -> Vec<u8> {
    let mut row = vec![iv(quad.0 as i64), iv(quad.1 as i64), iv(quad.2 as i64)];
    if let Some(graph_name) = quad.3 {
        row.push(iv(graph_name as i64));
    }
    canonical(&Value::Array(row))
}

fn remap_suppression(suppression: &Suppression, old_to_new: &[usize]) -> Suppression {
    let targets = suppression
        .targets
        .iter()
        .map(|target| remap_suppression_target(target, old_to_new))
        .collect();
    Suppression {
        targets,
        reason: suppression.reason.clone(),
        by: suppression.by.map(|tid| remap_id(old_to_new, tid)),
    }
}

fn remap_suppression_target(target: &Value, old_to_new: &[usize]) -> Value {
    let Value::Map(entries) = target else {
        return target.clone();
    };
    let kind = map_text(target, "kind").unwrap_or("");
    let mapped = entries
        .iter()
        .map(|(key, value)| {
            let key_text = match key {
                Value::Text(text) => text.as_str(),
                _ => "",
            };
            if (kind == "term" || kind == "reifier") && key_text == "id" {
                if let Some(tid) = value_idx(value) {
                    return (key.clone(), Value::from(remap_id(old_to_new, tid) as u64));
                }
            } else if kind == "quad" && key_text == "q" {
                if let Value::Array(ids) = value {
                    let remapped = ids
                        .iter()
                        .map(|id| {
                            value_idx(id)
                                .map(|tid| Value::from(remap_id(old_to_new, tid) as u64))
                                .unwrap_or_else(|| id.clone())
                        })
                        .collect();
                    return (key.clone(), Value::Array(remapped));
                }
            }
            (key.clone(), value.clone())
        })
        .collect();
    Value::Map(mapped)
}

fn suppression_key(suppression: &Suppression) -> Vec<u8> {
    let mut payload: Vec<(Value, Value)> =
        vec![("targets".into(), Value::Array(suppression.targets.clone()))];
    if let Some(reason) = &suppression.reason {
        payload.push(("reason".into(), reason.clone().into()));
    }
    if let Some(by) = suppression.by {
        payload.push(("by".into(), Value::from(by as u64)));
    }
    canonical(&Value::Map(payload))
}

fn map_text<'a>(value: &'a Value, wanted: &str) -> Option<&'a str> {
    let Value::Map(entries) = value else {
        return None;
    };
    entries.iter().find_map(|(key, value)| match (key, value) {
        (Value::Text(key), Value::Text(text)) if key == wanted => Some(text.as_str()),
        _ => None,
    })
}

fn value_idx(value: &Value) -> Option<usize> {
    if let Value::Integer(i) = value {
        usize::try_from(i128::from(*i)).ok()
    } else {
        None
    }
}

/// Pack bytes into a `blake3:<hex>` digest string.
pub fn digest_string(data: &[u8]) -> String {
    digest_str(data)
}
