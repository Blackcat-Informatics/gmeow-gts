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
use crate::ulid::deterministic_label;

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

// XSD datatypes for Turtle's bare numeric and boolean literals. The lexical form is
// preserved verbatim (no canonicalisation) so `0.70`, `1.0E0`, `+00:00`-style values
// survive the codec unchanged.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

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

fn is_forbidden_iri_char(ch: char) -> bool {
    ch.is_control()
        || ch.is_whitespace()
        || matches!(ch, '<' | '>' | '"' | '{' | '}' | '|' | '\\' | '^' | '`')
}

fn remove_dot_segments(path: &str) -> String {
    let absolute = path.starts_with('/');
    let keep_trailing_slash = path.ends_with('/')
        || path.ends_with("/.")
        || path.ends_with("/..")
        || path == "."
        || path == "..";
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }

    let mut normalized = String::new();
    if absolute {
        normalized.push('/');
    }
    normalized.push_str(&segments.join("/"));
    if keep_trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    if normalized.is_empty() && absolute {
        normalized.push('/');
    }
    normalized
}

fn split_raw_path_suffix(raw: &str) -> (&str, &str) {
    let split = raw.find(['?', '#']).unwrap_or(raw.len());
    (&raw[..split], &raw[split..])
}

fn split_base_for_path(base: &str) -> (String, &str) {
    let Some(scheme_end) = base.find(':') else {
        return (String::new(), base);
    };
    let scheme_prefix = &base[..=scheme_end];
    let rest = &base[scheme_end + 1..];
    if let Some(after_slashes) = rest.strip_prefix("//") {
        let authority_end = after_slashes.find('/').unwrap_or(after_slashes.len());
        let authority = &after_slashes[..authority_end];
        let path = &after_slashes[authority_end..];
        (format!("{scheme_prefix}//{authority}"), path)
    } else {
        (scheme_prefix.to_string(), rest)
    }
}

fn resolve_relative_iri(base: &str, raw: &str) -> String {
    if has_iri_scheme(raw) {
        return raw.to_string();
    }

    let base_without_fragment = base.split_once('#').map_or(base, |(before, _)| before);
    if raw.is_empty() {
        return base_without_fragment.to_string();
    }
    if raw.starts_with('#') {
        return format!("{base_without_fragment}{raw}");
    }

    let base_without_query = base_without_fragment
        .split_once('?')
        .map_or(base_without_fragment, |(before, _)| before);
    if raw.starts_with('?') {
        return format!("{base_without_query}{raw}");
    }

    if raw.starts_with("//") {
        if let Some(scheme_end) = base.find(':') {
            return format!("{}:{raw}", &base[..scheme_end]);
        }
        return raw.to_string();
    }

    let (prefix, base_path) = split_base_for_path(base_without_query);
    let (raw_path, suffix) = split_raw_path_suffix(raw);
    let merged_path = if raw_path.starts_with('/') {
        raw_path.to_string()
    } else {
        let base_dir = if base_path.is_empty() {
            "/"
        } else {
            base_path
                .rfind('/')
                .map(|index| &base_path[..=index])
                .unwrap_or("")
        };
        format!("{base_dir}{raw_path}")
    };
    format!("{prefix}{}{}", remove_dot_segments(&merged_path), suffix)
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
            // RDF 1.2 version directives. The declared version string is recorded
            // structurally elsewhere; the codec only needs to accept and skip it.
            // `@version "x" .` (dot-terminated) and `VERSION "x"` (keyword form).
            if self.consume("@version") {
                self.version_string()?;
                self.expect_char('.', "after @version directive")?;
                continue;
            }
            if self.consume_keyword("VERSION") {
                self.version_string()?;
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

            let allow_empty = self.subject_allows_empty_pol();
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
                self.statement_after_subject(first, None, allow_empty)?;
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
            return self.reifying_triple(graph);
        }
        match self.peek_char() {
            Some('<') => self.iri().map(Node::Iri),
            Some('_') => self.bnode().map(Node::Bnode),
            Some('"') | Some('\'') => self.literal(),
            Some('[') => self.blank_node_property_list(graph),
            Some('(') => self.collection(graph),
            Some(_) if self.looks_like_number() => self.numeric_literal(),
            Some(_) => match self.try_boolean_literal() {
                Some(node) => Ok(node),
                None => self.prefixed_name().map(Node::Iri),
            },
            None => Err(TriGParseError::new("unexpected end of Turtle/TriG input")),
        }
    }

    /// True when the cursor sits on a Turtle numeric literal: an optional sign then a
    /// digit, or a `.` immediately followed by a digit (`.5`). Prefixed names never
    /// start this way, so the lookahead unambiguously separates `42`/`-1.5e3` from
    /// `ex:foo`.
    fn looks_like_number(&self) -> bool {
        let bytes = self.text.as_bytes();
        let mut i = self.pos;
        if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        match bytes.get(i) {
            Some(b'0'..=b'9') => true,
            Some(b'.') => matches!(bytes.get(i + 1), Some(b'0'..=b'9')),
            _ => false,
        }
    }

    /// Parse a Turtle INTEGER / DECIMAL / DOUBLE, typing it by shape and keeping the
    /// lexical form verbatim. DOUBLE wins if an exponent is present, else DECIMAL if a
    /// fraction is present, else INTEGER.
    fn numeric_literal(&mut self) -> Result<Node, TriGParseError> {
        let start = self.pos;
        let bytes = self.text.as_bytes();
        if matches!(bytes.get(self.pos), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let mut has_digits = false;
        while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
            self.pos += 1;
            has_digits = true;
        }
        let mut is_decimal = false;
        if bytes.get(self.pos) == Some(&b'.')
            && matches!(bytes.get(self.pos + 1), Some(b'0'..=b'9'))
        {
            is_decimal = true;
            self.pos += 1;
            while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
                has_digits = true;
            }
        }
        let mut is_double = false;
        if matches!(bytes.get(self.pos), Some(b'e') | Some(b'E')) {
            is_double = true;
            self.pos += 1;
            if matches!(bytes.get(self.pos), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while matches!(bytes.get(self.pos), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == exp_start {
                return Err(TriGParseError::new(format!(
                    "malformed exponent in numeric literal at byte {start}"
                )));
            }
        }
        if !has_digits {
            return Err(TriGParseError::new(format!(
                "malformed numeric literal at byte {start}"
            )));
        }
        let datatype = if is_double {
            XSD_DOUBLE
        } else if is_decimal {
            XSD_DECIMAL
        } else {
            XSD_INTEGER
        };
        Ok(Node::Literal {
            value: self.text[start..self.pos].to_string(),
            lang: None,
            datatype: Some(datatype.to_string()),
        })
    }

    /// Consume a `true`/`false` boolean keyword (case-sensitive per Turtle) when it is
    /// followed by a name boundary, so `true`/`false` parse as `xsd:boolean` but
    /// `trueish`/`false:x` stay prefixed names.
    fn try_boolean_literal(&mut self) -> Option<Node> {
        for keyword in ["true", "false"] {
            let rest = &self.text[self.pos..];
            if let Some(after) = rest.strip_prefix(keyword) {
                let boundary = match after.chars().next() {
                    Some(ch) => {
                        ch.is_whitespace()
                            || matches!(ch, '.' | ';' | ',' | ')' | ']' | '}' | '>' | '#')
                    }
                    None => true,
                };
                if boundary {
                    self.pos += keyword.len();
                    return Some(Node::Literal {
                        value: keyword.to_string(),
                        lang: None,
                        datatype: Some(XSD_BOOLEAN.to_string()),
                    });
                }
            }
        }
        None
    }

    fn predicate(&mut self) -> Result<Node, TriGParseError> {
        self.skip_ws_and_comments();
        if self.consume_keyword("a") {
            Ok(Node::Iri(RDF_TYPE.to_string()))
        } else {
            self.term(None)
        }
    }

    /// A subject/object inside a quoted or reifying triple. Unlike a normal term it
    /// MUST NOT be a blank-node property list `[ … ]` or an RDF collection `( … )`:
    /// those expand to extra triples that cannot live inside a triple term, so the
    /// W3C suite treats them as syntax errors.
    fn quoted_component(&mut self, graph: Option<&Node>) -> Result<Node, TriGParseError> {
        self.skip_ws_and_comments();
        // An EMPTY `[]` (anonymous blank node) or `()` (rdf:nil) is a plain term and
        // is allowed; a NON-empty `[ pol ]` or `( … )` generates extra triples that
        // cannot live inside a triple term, so the W3C suite rejects those.
        let start_pos = self.pos;
        match self.peek_char() {
            Some('[') => {
                self.bump_char();
                self.skip_ws_and_comments();
                let is_empty = self.peek_char() == Some(']');
                self.pos = start_pos;
                if !is_empty {
                    return Err(TriGParseError::new(
                        "blank-node property list is not allowed inside a quoted triple",
                    ));
                }
            }
            Some('(') => {
                self.bump_char();
                self.skip_ws_and_comments();
                let is_empty = self.peek_char() == Some(')');
                self.pos = start_pos;
                if !is_empty {
                    return Err(TriGParseError::new(
                        "RDF collection is not allowed inside a quoted triple",
                    ));
                }
            }
            _ => {}
        }
        self.term(graph)
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
                if raw.chars().any(is_forbidden_iri_char) {
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
        if has_iri_scheme(raw) {
            raw.to_string()
        } else if let Some(base) = &self.base_iri {
            resolve_relative_iri(base, raw)
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
        Node::Bnode(deterministic_label("gts_", id as u128))
    }

    fn literal(&mut self) -> Result<Node, TriGParseError> {
        self.skip_ws_and_comments();
        let value = self.quoted_string()?;

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

    /// The string argument of a `VERSION` / `@version` directive: a SHORT string
    /// literal only (a triple-quoted string is a syntax error per the W3C suite).
    fn version_string(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        if self.text[self.pos..].starts_with("\"\"\"") || self.text[self.pos..].starts_with("'''") {
            return Err(TriGParseError::new(
                "version directive requires a simple (non-triple-quoted) string",
            ));
        }
        self.quoted_string()
    }

    fn datatype_iri(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        if self.peek_char() == Some('<') {
            self.iri()
        } else {
            self.prefixed_name()
        }
    }

    /// Read a Turtle string literal in any of its four quote styles: short `"…"` /
    /// `'…'` and long (triple-quoted) `"""…"""` / `'''…'''`. Long strings may span
    /// newlines and contain up to two consecutive quote characters; both forms honour
    /// the shared backslash escapes.
    fn quoted_string(&mut self) -> Result<String, TriGParseError> {
        let (quote, long) = match self.peek_char() {
            Some('"') => ('"', "\"\"\""),
            Some('\'') => ('\'', "'''"),
            _ => return Err(TriGParseError::new("expected literal")),
        };
        if self.text[self.pos..].starts_with(long) {
            self.pos += long.len();
            return self.long_string(long);
        }
        self.bump_char();
        let mut value = String::new();
        loop {
            let Some(ch) = self.bump_char() else {
                return Err(TriGParseError::new("unterminated literal"));
            };
            match ch {
                '\\' => value.push(self.escape()?),
                c if c == quote => break,
                _ => value.push(ch),
            }
        }
        Ok(value)
    }

    /// Read the body of a long (triple-quoted) string up to its closing triple quote.
    /// A lone or doubled quote inside the body is literal content; only the closing
    /// triple terminates.
    fn long_string(&mut self, closing: &str) -> Result<String, TriGParseError> {
        let mut value = String::new();
        loop {
            if self.text[self.pos..].starts_with(closing) {
                self.pos += closing.len();
                return Ok(value);
            }
            let Some(ch) = self.bump_char() else {
                return Err(TriGParseError::new("unterminated long string literal"));
            };
            match ch {
                '\\' => value.push(self.escape()?),
                _ => value.push(ch),
            }
        }
    }

    fn escape(&mut self) -> Result<char, TriGParseError> {
        let Some(ch) = self.bump_char() else {
            return Err(TriGParseError::new("bad escape at end of literal"));
        };
        match ch {
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
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
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        self.skip_ws_and_comments();
        if !self.text[self.pos..].starts_with(")>>") {
            return Err(TriGParseError::new("unterminated quoted triple"));
        }
        self.pos += 3;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// RDF 1.2 reifying triple `<< s p o [~ reifier] >>`. Unlike the triple-term
    /// form `<<( s p o )>>` (a value), this *asserts a reifier*: it evaluates to a
    /// reifier node — the explicit `~`-identifier when present, otherwise a fresh
    /// blank node — and emits `reifier rdf:reifies <<( s p o )>>`. The reifier node
    /// is what the enclosing statement uses as its subject/object.
    fn reifying_triple(&mut self, graph: Option<&Node>) -> Result<Node, TriGParseError> {
        self.pos += 2;
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        self.skip_ws_and_comments();
        let reifier = if self.consume_char('~') {
            self.skip_ws_and_comments();
            // `~` with no identifier before `>>` is an anonymous reifier.
            if self.text[self.pos..].starts_with(">>") {
                self.next_bnode()
            } else {
                self.term(graph)?
            }
        } else {
            self.next_bnode()
        };
        self.skip_ws_and_comments();
        if !self.text[self.pos..].starts_with(">>") {
            return Err(TriGParseError::new("unterminated reifying triple"));
        }
        self.pos += 2;
        let triple_term = Node::Triple(Box::new(s), Box::new(p), Box::new(o));
        self.emit_statement(
            &reifier,
            &Node::Iri(RDF_REIFIES.to_string()),
            &triple_term,
            graph,
        );
        Ok(reifier)
    }

    fn prefixed_name(&mut self) -> Result<String, TriGParseError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        // A `.` is NOT a delimiter here: Turtle's PN_LOCAL admits internal dots
        // (`repo:README.md`). A *trailing* dot is the statement terminator and is
        // stripped back below — PN_LOCAL may not end in `.`.
        //
        // A backslash starts a PN_LOCAL_ESC escape (`\(`, `\)`, `\,`, `\.`, …): the
        // backslash and the escaped character are part of the local name, so an
        // escaped delimiter does NOT terminate the scan (e.g.
        // `dbr:Semantic_analysis_\(linguistics\)`).
        while let Some(ch) = self.peek_char() {
            if ch == '\\' {
                self.bump_char();
                // Consume the escaped character verbatim (even if it is a delimiter).
                if self.peek_char().is_some() {
                    self.bump_char();
                }
                continue;
            }
            if ch.is_whitespace()
                || matches!(
                    ch,
                    '{' | '}' | '[' | ']' | '(' | ')' | '<' | '>' | ';' | ','
                )
            {
                break;
            }
            self.bump_char();
        }
        // Strip any trailing UNESCAPED dot(s): they terminate the statement, not the
        // name. An escaped `\.` is a literal dot in PN_LOCAL and is kept (an odd run
        // of backslashes immediately before the dot means it is escaped).
        while self.pos > start && self.text.as_bytes()[self.pos - 1] == b'.' {
            let mut backslashes = 0;
            while self.pos - 1 - backslashes > start
                && self.text.as_bytes()[self.pos - 2 - backslashes] == b'\\'
            {
                backslashes += 1;
            }
            if backslashes % 2 == 1 {
                break;
            }
            self.pos -= 1;
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
        // Resolve PN_LOCAL_ESC: `\X` denotes the literal `X` in the expanded IRI.
        Ok(format!("{base}{}", unescape_pn_local(local)))
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
            let allow_empty = self.subject_allows_empty_pol();
            let subject = self.term(Some(&graph))?;
            self.statement_after_subject(subject, Some(&graph), allow_empty)?;
        }
        Ok(())
    }

    fn statement_after_subject(
        &mut self,
        subject: Node,
        graph: Option<&Node>,
        allow_empty: bool,
    ) -> Result<(), TriGParseError> {
        // A reifying triple (`<< s p o >>`) or blank-node property list (`[ … ]`)
        // already asserts itself, so its predicate-object list is OPTIONAL — the
        // statement may end immediately at `.`. A plain subject still requires one.
        self.skip_ws_and_comments();
        if !(allow_empty && matches!(self.peek_char(), Some('.' | '}'))) {
            self.predicate_object_list(&subject, graph)?;
        }
        self.skip_ws_and_comments();
        // The trailing `.` is optional for the final statement inside a graph block
        // (`{ … }`): a `}` may follow directly.
        if self.consume_char('.') || (graph.is_some() && self.peek_char() == Some('}')) {
            Ok(())
        } else {
            Err(TriGParseError::new(format!(
                "expected '.' to terminate statement at byte {}",
                self.pos
            )))
        }
    }

    /// True when the upcoming subject is self-asserting (a reifying triple or a
    /// blank-node property list), so its predicate-object list may be omitted.
    fn subject_allows_empty_pol(&self) -> bool {
        let rest = self.text[self.pos..].trim_start();
        (rest.starts_with("<<") && !rest.starts_with("<<(")) || rest.starts_with('[')
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
                self.maybe_reify_and_annotate(subject, &predicate, &object, graph)?;
                if self.consume_char(',') {
                    continue;
                }
                break;
            }
            if self.consume_char(';') {
                self.skip_ws_and_comments();
                if matches!(self.peek_char(), Some('.' | ']' | '}'))
                    || self.text[self.pos..].starts_with("|}")
                {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    /// RDF 1.2 reifier/annotation suffix on a just-asserted triple `s p o`:
    /// an optional `~ reifier` identifier and zero or more `{| pol |}` annotation
    /// blocks. Each reifier (the explicit `~`-id for the first, a fresh blank for
    /// each subsequent block) gets `reifier rdf:reifies <<( s p o )>>`, and an
    /// annotation block applies its predicate-object list to that reifier.
    fn maybe_reify_and_annotate(
        &mut self,
        s: &Node,
        p: &Node,
        o: &Node,
        graph: Option<&Node>,
    ) -> Result<(), TriGParseError> {
        let triple_term = Node::Triple(
            Box::new(s.clone()),
            Box::new(p.clone()),
            Box::new(o.clone()),
        );
        let reifies = Node::Iri(RDF_REIFIES.to_string());

        // The reifier/annotation suffix is a SEQUENCE of items in any order: each
        // `~ id` declares a reifier (and becomes the pending target), and each
        // `{| pol |}` block applies its predicate-object list to the pending reifier
        // (consuming it) or to a fresh blank node when none is pending. Every reifier
        // — named, anonymous, or block-minted — gets `reifier rdf:reifies <<( s p o )>>`.
        let mut pending: Option<Node> = None;
        loop {
            self.skip_ws_and_comments();
            if self.consume_char('~') {
                self.skip_ws_and_comments();
                let id = if self.text[self.pos..].starts_with("{|")
                    || matches!(self.peek_char(), Some('.' | ',' | ';' | ']' | '}' | '~'))
                {
                    self.next_bnode()
                } else {
                    self.term(graph)?
                };
                self.emit_statement(&id, &reifies, &triple_term, graph);
                pending = Some(id);
            } else if self.text[self.pos..].starts_with("{|") {
                let reifier = match pending.take() {
                    Some(r) => r,
                    None => {
                        let r = self.next_bnode();
                        self.emit_statement(&r, &reifies, &triple_term, graph);
                        r
                    }
                };
                self.pos += 2; // consume "{|"
                self.predicate_object_list(&reifier, graph)?;
                self.skip_ws_and_comments();
                if !self.text[self.pos..].starts_with("|}") {
                    return Err(TriGParseError::new(
                        "unterminated annotation block (expected `|}`)",
                    ));
                }
                self.pos += 2; // consume "|}"
            } else {
                break;
            }
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

/// Resolve Turtle `PN_LOCAL_ESC` escapes in a prefixed name's local part: a
/// backslash denotes the literal next character (`\(` → `(`, `\.` → `.`, `\,` → `,`,
/// …), matching the Turtle grammar's
/// `PN_LOCAL_ESC ::= '\' ('_' | '~' | '.' | '-' | '!' | '$' | '&' | "'" | '(' | ')'
/// | '*' | '+' | ',' | ';' | '=' | '/' | '?' | '#' | '@' | '%')`.
fn unescape_pn_local(local: &str) -> String {
    if !local.contains('\\') {
        return local.to_string();
    }
    let mut out = String::with_capacity(local.len());
    let mut chars = local.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(escaped) = chars.next() {
                out.push(escaped);
            }
        } else {
            out.push(ch);
        }
    }
    out
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
