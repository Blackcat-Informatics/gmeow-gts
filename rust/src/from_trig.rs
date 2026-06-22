// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Turtle/TriG import for the GTS RDF text-codec surface.
//!
//! The parser is deliberately dependency-free and implements the tested GTS
//! surface: prefixes, base IRIs, graph blocks, `a`, predicate/object lists,
//! blank-node property lists, RDF collections, and RDF 1.2 quoted triples. It
//! lowers to N-Quads first so term validation and writer semantics stay shared
//! with [`crate::from_nquads`].

use std::collections::HashMap;
use std::fmt;

use crate::from_nquads::from_nquads;
use crate::nquads::escape_literal;

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// Raised when Turtle/TriG input is malformed or outside the supported surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriGParseError {
    detail: String,
}

impl TriGParseError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for TriGParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for TriGParseError {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Iri(String),
    Bnode(String),
    Literal {
        value: String,
        lang: Option<String>,
        datatype: Option<String>,
    },
    Triple(Box<Node>, Box<Node>, Box<Node>),
}

impl Node {
    fn token(&self) -> String {
        match self {
            Node::Iri(iri) => format!("<{iri}>"),
            Node::Bnode(label) => format!("_:{label}"),
            Node::Literal {
                value,
                lang,
                datatype,
            } => {
                let lit = format!("\"{}\"", escape_literal(value));
                if let Some(lang) = lang {
                    format!("{lit}@{lang}")
                } else if let Some(datatype) = datatype {
                    format!("{lit}^^<{datatype}>")
                } else {
                    lit
                }
            }
            Node::Triple(s, p, o) => {
                format!("<<( {} {} {} )>>", s.token(), p.token(), o.token())
            }
        }
    }

    fn is_graph_name(&self) -> bool {
        matches!(self, Node::Iri(_) | Node::Bnode(_))
    }
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    prefixes: HashMap<String, String>,
    nquads: Vec<String>,
    base_iri: Option<String>,
    bnode_counter: usize,
    allow_named_graphs: bool,
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

impl<'a> Parser<'a> {
    fn new(text: &'a str, allow_named_graphs: bool) -> Self {
        Self {
            text,
            pos: 0,
            prefixes: HashMap::from([("rdf".to_string(), RDF_NS.to_string())]),
            nquads: Vec::new(),
            base_iri: None,
            bnode_counter: 0,
            allow_named_graphs,
        }
    }

    fn parse(mut self) -> Result<String, TriGParseError> {
        while !self.eof() {
            self.skip_ws_and_comments();
            if self.eof() {
                break;
            }
            if self.consume("@prefix") {
                self.prefix_directive(true)?;
                continue;
            }
            if self.consume("@base") {
                self.base_directive(true)?;
                continue;
            }
            if self.consume_keyword("PREFIX") {
                self.prefix_directive(false)?;
                continue;
            }
            if self.consume_keyword("BASE") {
                self.base_directive(false)?;
                continue;
            }
            if self.consume_keyword("GRAPH") {
                if !self.allow_named_graphs {
                    return Err(TriGParseError::new(
                        "Turtle input cannot contain GRAPH blocks",
                    ));
                }
                let graph = self.term(None)?;
                self.graph_block(graph)?;
                continue;
            }

            let first = self.term(None)?;
            self.skip_ws_and_comments();
            if self.consume_char('{') {
                if !self.allow_named_graphs {
                    return Err(TriGParseError::new(
                        "Turtle input cannot contain graph blocks",
                    ));
                }
                self.graph_block_after_open(first)?;
            } else {
                self.statement_after_subject(first, None)?;
            }
        }
        Ok(if self.nquads.is_empty() {
            String::new()
        } else {
            format!("{}\n", self.nquads.join("\n"))
        })
    }

    fn eof(&mut self) -> bool {
        self.skip_ws_and_comments();
        self.pos >= self.text.len()
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while let Some(ch) = self.peek_char() {
                if ch.is_whitespace() {
                    self.bump_char();
                } else {
                    break;
                }
            }
            if self.peek_char() == Some('#') {
                while let Some(ch) = self.bump_char() {
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }
            break;
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

    fn consume(&mut self, text: &str) -> bool {
        self.skip_ws_and_comments();
        if self.text[self.pos..].starts_with(text) {
            self.pos += text.len();
            true
        } else {
            false
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        self.skip_ws_and_comments();
        let rest = &self.text[self.pos..];
        if rest.len() < keyword.len() || !rest[..keyword.len()].eq_ignore_ascii_case(keyword) {
            return false;
        }
        let boundary = rest[keyword.len()..]
            .chars()
            .next()
            .map(|ch| {
                ch.is_whitespace()
                    || matches!(
                        ch,
                        '{' | '}' | '[' | ']' | '(' | ')' | '<' | '_' | '"' | ';' | ',' | '.'
                    )
            })
            .unwrap_or(true);
        if boundary {
            self.pos += keyword.len();
            true
        } else {
            false
        }
    }

    fn consume_char(&mut self, ch: char) -> bool {
        self.skip_ws_and_comments();
        if self.peek_char() == Some(ch) {
            self.bump_char();
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, ch: char, context: &str) -> Result<(), TriGParseError> {
        if self.consume_char(ch) {
            Ok(())
        } else {
            Err(TriGParseError::new(format!(
                "expected {ch:?} {context} at byte {}",
                self.pos
            )))
        }
    }

    fn prefix_directive(&mut self, require_dot: bool) -> Result<(), TriGParseError> {
        let prefix = self.prefix_label()?;
        let iri = self.iri()?;
        self.prefixes.insert(prefix, iri);
        if require_dot {
            self.expect_char('.', "after @prefix directive")?;
        } else {
            self.consume_char('.');
        }
        Ok(())
    }

    fn base_directive(&mut self, require_dot: bool) -> Result<(), TriGParseError> {
        let iri = self.iri_raw()?;
        if !has_iri_scheme(&iri) {
            return Err(TriGParseError::new(format!(
                "base IRI must be absolute: {iri:?}"
            )));
        }
        self.base_iri = Some(iri);
        if require_dot {
            self.expect_char('.', "after @base directive")?;
        } else {
            self.consume_char('.');
        }
        Ok(())
    }

    fn prefix_label(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == ':' {
                let label = self.text[start..self.pos].to_string();
                self.bump_char();
                return Ok(label);
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                self.bump_char();
            } else {
                break;
            }
        }
        Err(TriGParseError::new(format!(
            "expected prefix label at byte {}",
            start
        )))
    }

    fn term(&mut self, graph: Option<&Node>) -> Result<Node, TriGParseError> {
        self.skip_ws_and_comments();
        if self.text[self.pos..].starts_with("<<(") {
            return self.parenthesized_quoted_triple(graph);
        }
        if self.text[self.pos..].starts_with("<<") {
            return self.legacy_quoted_triple(graph);
        }
        match self.peek_char() {
            Some('<') => self.iri().map(Node::Iri),
            Some('_') => self.bnode().map(Node::Bnode),
            Some('"') => self.literal(),
            Some('[') => self.blank_node_property_list(graph),
            Some('(') => self.collection(graph),
            Some(_) => self.prefixed_name().map(Node::Iri),
            None => Err(TriGParseError::new("unexpected end of Turtle/TriG input")),
        }
    }

    fn predicate(&mut self) -> Result<Node, TriGParseError> {
        self.skip_ws_and_comments();
        if self.consume_keyword("a") {
            Ok(Node::Iri(RDF_TYPE.to_string()))
        } else {
            self.term(None)
        }
    }

    fn iri_raw(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        if self.bump_char() != Some('<') {
            return Err(TriGParseError::new(format!(
                "expected IRI at byte {}",
                self.pos
            )));
        }
        let start = self.pos;
        while let Some(ch) = self.bump_char() {
            if ch == '>' {
                let end = self.pos - 1;
                let raw = &self.text[start..end];
                if raw
                    .chars()
                    .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | '"'))
                {
                    return Err(TriGParseError::new(format!(
                        "invalid character in IRI starting at byte {}",
                        start.saturating_sub(1)
                    )));
                }
                return Ok(raw.to_string());
            }
        }
        Err(TriGParseError::new(format!(
            "unterminated IRI starting at byte {}",
            start.saturating_sub(1)
        )))
    }

    fn iri(&mut self) -> Result<String, TriGParseError> {
        let raw = self.iri_raw()?;
        Ok(self.resolve_iri(&raw))
    }

    fn resolve_iri(&self, raw: &str) -> String {
        if has_iri_scheme(raw) || raw.starts_with("//") {
            raw.to_string()
        } else if let Some(base) = &self.base_iri {
            format!("{base}{raw}")
        } else {
            raw.to_string()
        }
    }

    fn bnode(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        if !self.text[self.pos..].starts_with("_:") {
            return Err(TriGParseError::new(format!(
                "expected blank node at byte {}",
                self.pos
            )));
        }
        self.pos += 2;
        let start = self.pos;
        while let Some(byte) = self.text.as_bytes().get(self.pos) {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos > start && self.text.as_bytes()[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        if self.pos == start {
            return Err(TriGParseError::new("empty blank node label"));
        }
        Ok(self.text[start..self.pos].to_string())
    }

    fn next_bnode(&mut self) -> Node {
        let id = self.bnode_counter;
        self.bnode_counter += 1;
        Node::Bnode(format!("gts_b{id}"))
    }

    fn literal(&mut self) -> Result<Node, TriGParseError> {
        self.skip_ws_and_comments();
        if self.bump_char() != Some('"') {
            return Err(TriGParseError::new("expected literal"));
        }
        let mut value = String::new();
        loop {
            let Some(ch) = self.bump_char() else {
                return Err(TriGParseError::new("unterminated literal"));
            };
            match ch {
                '\\' => value.push(self.escape()?),
                '"' => break,
                _ => value.push(ch),
            }
        }

        let mut lang = None;
        let mut datatype = None;
        if self.peek_char() == Some('@') {
            self.bump_char();
            let start = self.pos;
            while let Some(byte) = self.text.as_bytes().get(self.pos) {
                if byte.is_ascii_alphanumeric() || *byte == b'-' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.pos == start {
                return Err(TriGParseError::new("empty language tag"));
            }
            lang = Some(self.text[start..self.pos].to_string());
        } else if self.text[self.pos..].starts_with("^^") {
            self.pos += 2;
            datatype = Some(self.datatype_iri()?);
        }
        Ok(Node::Literal {
            value,
            lang,
            datatype,
        })
    }

    fn datatype_iri(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        if self.peek_char() == Some('<') {
            self.iri()
        } else {
            self.prefixed_name()
        }
    }

    fn escape(&mut self) -> Result<char, TriGParseError> {
        let Some(ch) = self.bump_char() else {
            return Err(TriGParseError::new("bad escape at end of literal"));
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
                    return Err(TriGParseError::new("short or invalid unicode escape"));
                }
                let raw = &self.text[self.pos..end];
                if !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(TriGParseError::new(format!(
                        "bad unicode escape \\{ch}{raw}"
                    )));
                }
                self.pos += width;
                let code = u32::from_str_radix(raw, 16)
                    .map_err(|e| TriGParseError::new(format!("bad unicode escape: {e}")))?;
                char::from_u32(code).ok_or_else(|| {
                    TriGParseError::new(format!("invalid unicode scalar \\{ch}{raw}"))
                })
            }
            other => Err(TriGParseError::new(format!("unsupported escape \\{other}"))),
        }
    }

    fn parenthesized_quoted_triple(
        &mut self,
        graph: Option<&Node>,
    ) -> Result<Node, TriGParseError> {
        self.pos += 3;
        let s = self.term(graph)?;
        let p = self.predicate()?;
        let o = self.term(graph)?;
        self.skip_ws_and_comments();
        if !self.text[self.pos..].starts_with(")>>") {
            return Err(TriGParseError::new("unterminated quoted triple"));
        }
        self.pos += 3;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    fn legacy_quoted_triple(&mut self, graph: Option<&Node>) -> Result<Node, TriGParseError> {
        self.pos += 2;
        let s = self.term(graph)?;
        let p = self.predicate()?;
        let o = self.term(graph)?;
        self.skip_ws_and_comments();
        if !self.text[self.pos..].starts_with(">>") {
            return Err(TriGParseError::new("unterminated quoted triple"));
        }
        self.pos += 2;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    fn prefixed_name(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace()
                || matches!(
                    ch,
                    '{' | '}' | '[' | ']' | '(' | ')' | '<' | '>' | '.' | ';' | ','
                )
            {
                break;
            }
            self.bump_char();
        }
        if self.pos == start {
            return Err(TriGParseError::new(format!(
                "expected term at byte {}",
                self.pos
            )));
        }
        let name = &self.text[start..self.pos];
        let Some((prefix, local)) = name.split_once(':') else {
            return Err(TriGParseError::new(format!(
                "unsupported bare token {name:?}; use an IRI or prefix"
            )));
        };
        let Some(base) = self.prefixes.get(prefix) else {
            return Err(TriGParseError::new(format!("unknown prefix {prefix:?}")));
        };
        Ok(format!("{base}{local}"))
    }

    fn blank_node_property_list(&mut self, graph: Option<&Node>) -> Result<Node, TriGParseError> {
        self.expect_char('[', "to open blank-node property list")?;
        let subject = self.next_bnode();
        if !self.consume_char(']') {
            self.predicate_object_list(&subject, graph)?;
            self.expect_char(']', "to close blank-node property list")?;
        }
        Ok(subject)
    }

    fn collection(&mut self, graph: Option<&Node>) -> Result<Node, TriGParseError> {
        self.expect_char('(', "to open RDF collection")?;
        let mut items = Vec::new();
        while !self.consume_char(')') {
            if self.eof() {
                return Err(TriGParseError::new("unterminated RDF collection"));
            }
            items.push(self.term(graph)?);
        }
        if items.is_empty() {
            return Ok(Node::Iri(RDF_NIL.to_string()));
        }

        let mut cells: Vec<Node> = (0..items.len()).map(|_| self.next_bnode()).collect();
        for (index, item) in items.into_iter().enumerate() {
            let current = cells[index].clone();
            let rest = if index + 1 == cells.len() {
                Node::Iri(RDF_NIL.to_string())
            } else {
                cells[index + 1].clone()
            };
            self.emit_statement(&current, &Node::Iri(RDF_FIRST.to_string()), &item, graph);
            self.emit_statement(&current, &Node::Iri(RDF_REST.to_string()), &rest, graph);
        }
        Ok(cells.remove(0))
    }

    fn graph_block(&mut self, graph: Node) -> Result<(), TriGParseError> {
        self.expect_char('{', "to open graph block")?;
        self.graph_block_after_open(graph)
    }

    fn graph_block_after_open(&mut self, graph: Node) -> Result<(), TriGParseError> {
        if !graph.is_graph_name() {
            return Err(TriGParseError::new(
                "graph block name must be an IRI or blank node",
            ));
        }
        while !self.consume_char('}') {
            if self.eof() {
                return Err(TriGParseError::new("unterminated graph block"));
            }
            let subject = self.term(Some(&graph))?;
            self.statement_after_subject(subject, Some(&graph))?;
        }
        Ok(())
    }

    fn statement_after_subject(
        &mut self,
        subject: Node,
        graph: Option<&Node>,
    ) -> Result<(), TriGParseError> {
        self.predicate_object_list(&subject, graph)?;
        self.expect_char('.', "to terminate statement")
    }

    fn predicate_object_list(
        &mut self,
        subject: &Node,
        graph: Option<&Node>,
    ) -> Result<(), TriGParseError> {
        loop {
            let predicate = self.predicate()?;
            loop {
                let object = self.term(graph)?;
                self.emit_statement(subject, &predicate, &object, graph);
                if self.consume_char(',') {
                    continue;
                }
                break;
            }
            if self.consume_char(';') {
                self.skip_ws_and_comments();
                if matches!(self.peek_char(), Some('.' | ']' | '}')) {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    fn emit_statement(
        &mut self,
        subject: &Node,
        predicate: &Node,
        object: &Node,
        graph: Option<&Node>,
    ) {
        let mut line = format!(
            "{} {} {}",
            subject.token(),
            predicate.token(),
            object.token()
        );
        if let Some(graph) = graph {
            line.push(' ');
            line.push_str(&graph.token());
        }
        line.push_str(" .");
        self.nquads.push(line);
    }
}

/// Parse Turtle text into a canonical GTS file.
pub fn from_turtle(text: &str) -> Result<Vec<u8>, TriGParseError> {
    let nquads = Parser::new(text, false).parse()?;
    from_nquads(&nquads).map_err(|err| TriGParseError::new(err.to_string()))
}

/// Parse TriG text into a canonical GTS file.
pub fn from_trig(text: &str) -> Result<Vec<u8>, TriGParseError> {
    let nquads = Parser::new(text, true).parse()?;
    from_nquads(&nquads).map_err(|err| TriGParseError::new(err.to_string()))
}
