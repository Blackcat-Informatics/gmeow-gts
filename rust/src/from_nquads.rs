// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `nquads -> gts` transform: the inverse of the §14 fold projection.
//!
//! This parser accepts the N-Quads(-star) text emitted by [`crate::nquads`]
//! and builds a canonical GTS segment with the shared writer semantics. Blobs,
//! suppressions, and opaque frames are not expressible in N-Quads and are
//! intentionally out of scope, matching the Python reference implementation.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::model::{
    AnnotationRow, Quad, ReifierRow, Term, TermKind, Triple3, RDF_DIR_LANG_STRING, RDF_LANG_STRING,
};
use crate::writer::Writer;

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Raised when N-Quads(-star) input is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NQuadsParseError {
    detail: String,
}

impl NQuadsParseError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for NQuadsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for NQuadsParseError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Atom {
    kind: TermKind,
    value: String,
    lang: Option<String>,
    direction: Option<String>,
    datatype: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TripleNode {
    s: Box<Node>,
    p: Box<Node>,
    o: Box<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Atom(Atom),
    Triple(TripleNode),
}

struct Tokenizer<'a> {
    text: &'a str,
    pos: usize,
}

fn is_bnode_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')
}

fn is_lang_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

fn has_iri_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    for ch in chars {
        if ch == ':' {
            return true;
        }
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

fn is_forbidden_iri_char(ch: char) -> bool {
    ch.is_control()
        || ch.is_whitespace()
        || matches!(ch, '<' | '>' | '"' | '{' | '}' | '|' | '\\' | '^' | '`')
}

fn validate_iri(value: &str, line: &str) -> Result<(), NQuadsParseError> {
    if value.is_empty() || value.starts_with("//") || !has_iri_scheme(value) {
        return Err(NQuadsParseError::new(format!(
            "IRI must be absolute: {line:?}"
        )));
    }
    if value.chars().any(is_forbidden_iri_char) {
        return Err(NQuadsParseError::new(format!(
            "invalid character in IRI: {line:?}"
        )));
    }
    Ok(())
}

fn validate_language_tag(tag: &str, line: &str) -> Result<(), NQuadsParseError> {
    let mut parts = tag.split('-');
    let Some(primary) = parts.next() else {
        return Err(NQuadsParseError::new(format!(
            "empty language tag in {line:?}"
        )));
    };
    if primary.is_empty()
        || primary.len() > 8
        || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(NQuadsParseError::new(format!(
            "invalid language tag {tag:?} in {line:?}"
        )));
    }
    // BCP-47 private-use sequences are introduced by the singleton `x`. GMEOW relies on
    // long private-use subtags (e.g. `x-gmeow-norwegiannynorsk`) that exceed the 8-char
    // per-subtag limit, so once `x` appears the length cap is dropped for the remainder
    // (subtags must still be non-empty and alphanumeric). This is the native equivalent
    // of the oxttl `.lenient()` mode the 909 branch depended on.
    let mut private_use = primary.eq_ignore_ascii_case("x");
    for subtag in parts {
        let alnum = !subtag.is_empty() && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric());
        let acceptable = if private_use {
            alnum
        } else {
            alnum && subtag.len() <= 8
        };
        if !acceptable {
            return Err(NQuadsParseError::new(format!(
                "invalid language tag {tag:?} in {line:?}"
            )));
        }
        if subtag.eq_ignore_ascii_case("x") {
            private_use = true;
        }
    }
    Ok(())
}

impl<'a> Tokenizer<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while matches!(self.text.as_bytes().get(self.pos), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    fn at_end(&mut self) -> bool {
        self.skip_ws();
        self.pos >= self.text.len() || self.text.as_bytes()[self.pos] == b'.'
    }

    fn node(&mut self) -> Result<Node, NQuadsParseError> {
        self.skip_ws();
        if self.pos >= self.text.len() {
            return Err(NQuadsParseError::new(format!(
                "unexpected end of line: {:?}",
                self.text
            )));
        }
        if self.text[self.pos..].starts_with("<<(") {
            return self.quoted_triple().map(Node::Triple);
        }
        match self.peek_char() {
            Some('<') => Ok(Node::Atom(Atom {
                kind: TermKind::Iri,
                value: self.iri()?,
                lang: None,
                direction: None,
                datatype: None,
            })),
            Some('_') => Ok(Node::Atom(Atom {
                kind: TermKind::Bnode,
                value: self.bnode()?,
                lang: None,
                direction: None,
                datatype: None,
            })),
            Some('"') => self.literal().map(Node::Atom),
            _ => Err(NQuadsParseError::new(format!(
                "unexpected token at {} in {:?}",
                self.pos, self.text
            ))),
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn iri(&mut self) -> Result<String, NQuadsParseError> {
        if self.text.as_bytes().get(self.pos) != Some(&b'<') {
            return Err(NQuadsParseError::new(format!("bad IRI in {:?}", self.text)));
        }
        let start = self.pos + 1;
        let rel = self.text[start..]
            .find('>')
            .ok_or_else(|| NQuadsParseError::new(format!("unterminated IRI in {:?}", self.text)))?;
        let end = start + rel;
        self.pos = end + 1;
        let value = &self.text[start..end];
        validate_iri(value, self.text)?;
        Ok(value.to_string())
    }

    fn bnode(&mut self) -> Result<String, NQuadsParseError> {
        if !self.text[self.pos..].starts_with("_:") {
            return Err(NQuadsParseError::new(format!(
                "bad blank node in {:?}",
                self.text
            )));
        }
        self.pos += 2;
        let start = self.pos;
        while self.pos < self.text.len() && is_bnode_char(self.text.as_bytes()[self.pos]) {
            self.pos += 1;
        }
        if self.pos > start && self.text.as_bytes()[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        if self.pos == start {
            return Err(NQuadsParseError::new(format!(
                "empty blank node label in {:?}",
                self.text
            )));
        }
        Ok(self.text[start..self.pos].to_string())
    }

    fn literal(&mut self) -> Result<Atom, NQuadsParseError> {
        if self.bump_char() != Some('"') {
            return Err(NQuadsParseError::new(format!(
                "bad literal in {:?}",
                self.text
            )));
        }
        let mut value = String::new();
        loop {
            let Some(ch) = self.bump_char() else {
                return Err(NQuadsParseError::new(format!(
                    "unterminated literal in {:?}",
                    self.text
                )));
            };
            match ch {
                '\\' => value.push(self.escape()?),
                '"' => break,
                _ => value.push(ch),
            }
        }

        let mut lang = None;
        let mut direction = None;
        let mut datatype = None;
        if self.text.as_bytes().get(self.pos) == Some(&b'@') {
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.text.len() && is_lang_char(self.text.as_bytes()[self.pos]) {
                self.pos += 1;
            }
            if self.pos == start {
                return Err(NQuadsParseError::new(format!(
                    "empty language tag in {:?}",
                    self.text
                )));
            }
            let raw_lang = &self.text[start..self.pos];
            if let Some((base, dir)) = raw_lang.rsplit_once("--") {
                if matches!(dir, "ltr" | "rtl") && !base.is_empty() {
                    validate_language_tag(base, self.text)?;
                    lang = Some(base.to_string());
                    direction = Some(dir.to_string());
                } else {
                    return Err(NQuadsParseError::new(format!(
                        "invalid literal base direction in {:?}",
                        self.text
                    )));
                }
            } else {
                validate_language_tag(raw_lang, self.text)?;
                lang = Some(raw_lang.to_string());
            }
        } else if self.text[self.pos..].starts_with("^^") {
            self.pos += 2;
            self.skip_ws();
            let iri = self.iri()?;
            if matches!(iri.as_str(), RDF_LANG_STRING | RDF_DIR_LANG_STRING) {
                return Err(NQuadsParseError::new(format!(
                    "literal cannot explicitly use RDF language-string datatype in {:?}",
                    self.text
                )));
            }
            datatype = Some(iri);
        }

        Ok(Atom {
            kind: TermKind::Literal,
            value,
            lang,
            direction,
            datatype,
        })
    }

    fn escape(&mut self) -> Result<char, NQuadsParseError> {
        let Some(ch) = self.bump_char() else {
            return Err(NQuadsParseError::new(format!(
                "bad escape at end of {:?}",
                self.text
            )));
        };
        match ch {
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            'b' => Ok('\u{0008}'),
            'f' => Ok('\u{000c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' | 'U' => {
                let width = if ch == 'u' { 4 } else { 8 };
                let end = self.pos + width;
                if end > self.text.len() || !self.text.is_char_boundary(end) {
                    return Err(NQuadsParseError::new(format!(
                        "short or invalid unicode escape in {:?}",
                        self.text
                    )));
                }
                let raw = &self.text[self.pos..end];
                if !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(NQuadsParseError::new(format!(
                        "bad unicode escape \\{ch}{raw} in {:?}",
                        self.text
                    )));
                }
                self.pos += width;
                let code = u32::from_str_radix(raw, 16).map_err(|e| {
                    NQuadsParseError::new(format!("bad unicode escape \\{ch}{raw}: {e}"))
                })?;
                char::from_u32(code).ok_or_else(|| {
                    NQuadsParseError::new(format!("invalid unicode scalar \\{ch}{raw}"))
                })
            }
            other => Err(NQuadsParseError::new(format!(
                "unsupported escape \\{other} in {:?}",
                self.text
            ))),
        }
    }

    fn quoted_triple(&mut self) -> Result<TripleNode, NQuadsParseError> {
        self.pos += 3;
        let s = self.node()?;
        let p = self.node()?;
        let o = self.node()?;
        self.skip_ws();
        if !self.text[self.pos..].starts_with(")>>") {
            return Err(NQuadsParseError::new(format!(
                "unterminated quoted triple in {:?}",
                self.text
            )));
        }
        self.pos += 3;
        Ok(TripleNode {
            s: Box::new(s),
            p: Box::new(p),
            o: Box::new(o),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TermKey {
    Atom {
        kind: TermKind,
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<String>,
    },
    Triple(usize, usize, usize),
}

struct Interner {
    ids: HashMap<TermKey, usize>,
    terms: Vec<Term>,
}

impl Interner {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            terms: Vec::new(),
        }
    }

    fn atom(&mut self, atom: &Atom) -> usize {
        let key = TermKey::Atom {
            kind: atom.kind,
            value: atom.value.clone(),
            lang: atom.lang.clone(),
            direction: atom.direction.clone(),
            datatype: atom.datatype.clone(),
        };
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let datatype = if atom.kind == TermKind::Literal {
            atom.datatype.as_ref().map(|iri| {
                self.atom(&Atom {
                    kind: TermKind::Iri,
                    value: iri.clone(),
                    lang: None,
                    direction: None,
                    datatype: None,
                })
            })
        } else {
            None
        };
        let id = self.terms.len();
        self.terms.push(Term {
            kind: atom.kind,
            value: Some(atom.value.clone()),
            datatype,
            lang: atom.lang.clone(),
            direction: atom.direction.clone(),
            reifier: None,
        });
        self.ids.insert(key, id);
        id
    }

    fn node(&mut self, node: &Node, reifiers: &mut Vec<ReifierRow>) -> usize {
        match node {
            Node::Atom(atom) => self.atom(atom),
            Node::Triple(triple) => {
                let s = self.node(&triple.s, reifiers);
                let p = self.node(&triple.p, reifiers);
                let o = self.node(&triple.o, reifiers);
                let key = TermKey::Triple(s, p, o);
                if let Some(id) = self.ids.get(&key) {
                    return *id;
                }
                let id = self.terms.len();
                self.terms.push(Term {
                    kind: TermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: Some(id),
                });
                self.ids.insert(key, id);
                // A triple TERM stores its own components under its freshly-minted id
                // (the key dedup above guarantees this id is bound exactly once), so this
                // is always a first bind — push directly rather than going through the
                // conflict-checking `set_reifier` (which would force panic/error handling
                // on a branch that can never conflict).
                reifiers.push((id, (s, p, o), None));
                id
            }
        }
    }
}

fn set_reifier(
    reifiers: &mut Vec<ReifierRow>,
    rid: usize,
    spo: Triple3,
    graph_name: Option<usize>,
) -> Result<(), NQuadsParseError> {
    if let Some((_, existing, _)) = reifiers.iter().find(|(r, _, _)| *r == rid) {
        // A reifier bound to a DIFFERENT triple is a hard conflict (CONSTITUTION P7:
        // never silently last-write-win). An identical rebind is idempotent.
        if *existing != spo {
            return Err(NQuadsParseError::new(format!(
                "conflicting rdf:reifies binding for reifier term {rid}"
            )));
        }
        if reifiers
            .iter()
            .any(|&(r, existing, g)| r == rid && existing == spo && g == graph_name)
        {
            return Ok(());
        }
    }
    reifiers.push((rid, spo, graph_name));
    Ok(())
}

fn validate_subject(
    node: &Node,
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), NQuadsParseError> {
    if matches!(
        node,
        Node::Atom(Atom {
            kind: TermKind::Iri | TermKind::Bnode,
            ..
        })
    ) {
        return Ok(());
    }
    if allow_triple_subject {
        if let Node::Triple(triple) = node {
            return validate_triple_node(triple, line, allow_triple_subject);
        }
    }
    Err(NQuadsParseError::new(format!(
        "invalid subject term: {line:?}"
    )))
}

fn validate_predicate(node: &Node, line: &str) -> Result<(), NQuadsParseError> {
    if matches!(
        node,
        Node::Atom(Atom {
            kind: TermKind::Iri,
            ..
        })
    ) {
        Ok(())
    } else {
        Err(NQuadsParseError::new(format!(
            "predicate must be IRI: {line:?}"
        )))
    }
}

fn validate_object(
    node: &Node,
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), NQuadsParseError> {
    let is_iri = |node: &Node| {
        matches!(
            node,
            Node::Atom(Atom {
                kind: TermKind::Iri,
                ..
            })
        )
    };
    let is_bnode = |node: &Node| {
        matches!(
            node,
            Node::Atom(Atom {
                kind: TermKind::Bnode,
                ..
            })
        )
    };
    let is_literal = |node: &Node| {
        matches!(
            node,
            Node::Atom(Atom {
                kind: TermKind::Literal,
                ..
            })
        )
    };
    if is_iri(node) || is_bnode(node) || is_literal(node) {
        return Ok(());
    }
    if let Node::Triple(triple) = node {
        return validate_triple_node(triple, line, allow_triple_subject);
    }
    Err(NQuadsParseError::new(format!(
        "invalid object term: {line:?}"
    )))
}

fn validate_triple_node(
    triple: &TripleNode,
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), NQuadsParseError> {
    validate_subject(&triple.s, line, allow_triple_subject)?;
    validate_predicate(&triple.p, line)?;
    validate_object(&triple.o, line, allow_triple_subject)
}

fn validate_statement(
    nodes: &[Node],
    line: &str,
    allow_triple_subject: bool,
) -> Result<(), NQuadsParseError> {
    validate_subject(&nodes[0], line, allow_triple_subject)?;
    validate_predicate(&nodes[1], line)?;
    validate_object(&nodes[2], line, allow_triple_subject)?;
    if let Some(graph_name) = nodes.get(3) {
        if !matches!(
            graph_name,
            Node::Atom(Atom {
                kind: TermKind::Iri | TermKind::Bnode,
                ..
            })
        ) {
            return Err(NQuadsParseError::new(format!(
                "invalid graph name term: {line:?}"
            )));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ParseOptions {
    allow_graph: bool,
    allow_triple_subject: bool,
}

fn parse_text(text: &str, options: ParseOptions) -> Result<Vec<Vec<Node>>, NQuadsParseError> {
    let mut statements: Vec<Vec<Node>> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokenizer = Tokenizer::new(line);
        let mut nodes = Vec::new();
        while !tokenizer.at_end() {
            nodes.push(tokenizer.node()?);
        }
        let valid_len = if options.allow_graph {
            nodes.len() == 3 || nodes.len() == 4
        } else {
            nodes.len() == 3
        };
        if !valid_len {
            return Err(NQuadsParseError::new(format!(
                "expected {} terms, got {}: {:?}",
                if options.allow_graph { "3 or 4" } else { "3" },
                nodes.len(),
                line
            )));
        }
        validate_statement(&nodes, line, options.allow_triple_subject)?;
        statements.push(nodes);
    }
    Ok(statements)
}

fn build_gts(statements: &[Vec<Node>]) -> Result<Vec<u8>, NQuadsParseError> {
    let mut interner = Interner::new();
    let mut reifiers: Vec<ReifierRow> = Vec::new();
    let mut pending_quads: Vec<Quad> = Vec::new();

    for nodes in statements {
        let s = &nodes[0];
        let p = &nodes[1];
        let o = &nodes[2];
        let gname = nodes.get(3);

        if let (Node::Atom(subject), Node::Atom(predicate), Node::Triple(object)) = (s, p, o) {
            if predicate.value == RDF_REIFIES {
                let rid = interner.atom(subject);
                let ss = interner.node(&object.s, &mut reifiers);
                let pp = interner.node(&object.p, &mut reifiers);
                let oo = interner.node(&object.o, &mut reifiers);
                let gid = gname.map(|node| interner.node(node, &mut reifiers));
                set_reifier(&mut reifiers, rid, (ss, pp, oo), gid)?;
                continue;
            }
        }

        let sid = interner.node(s, &mut reifiers);
        let pid = interner.node(p, &mut reifiers);
        let oid = interner.node(o, &mut reifiers);
        let gid = gname.map(|node| interner.node(node, &mut reifiers));
        pending_quads.push((sid, pid, oid, gid));
    }

    let reifier_ids: HashSet<usize> = reifiers.iter().map(|(rid, _, _)| *rid).collect();
    let mut quads: Vec<Quad> = Vec::new();
    let mut annotations: Vec<AnnotationRow> = Vec::new();
    for (s, p, o, g) in pending_quads {
        if reifier_ids.contains(&s) {
            annotations.push((s, p, o, g));
        } else {
            quads.push((s, p, o, g));
        }
    }

    let mut writer = Writer::new("dist");
    if !interner.terms.is_empty() {
        writer.add_terms(&interner.terms);
    }
    if !quads.is_empty() {
        writer.add_quads(&quads);
    }
    if !reifiers.is_empty() {
        writer.add_reifies(&reifiers);
    }
    if !annotations.is_empty() {
        writer.add_annot(&annotations);
    }
    Ok(writer.to_bytes())
}

/// Parse N-Triples(-star) text into a canonical GTS file.
pub fn from_ntriples(text: &str) -> Result<Vec<u8>, NQuadsParseError> {
    let statements = parse_text(
        text,
        ParseOptions {
            allow_graph: false,
            allow_triple_subject: false,
        },
    )?;
    build_gts(&statements)
}

/// Parse N-Quads(-star) text into a canonical GTS file.
pub fn from_nquads(text: &str) -> Result<Vec<u8>, NQuadsParseError> {
    let statements = parse_text(
        text,
        ParseOptions {
            allow_graph: true,
            allow_triple_subject: true,
        },
    )?;
    build_gts(&statements)
}
