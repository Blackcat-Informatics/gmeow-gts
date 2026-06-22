// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XML Schema datatype lexical validation for parser-facing RDF import paths.
//!
//! GTS transport equality deliberately keeps literal lexical forms verbatim.
//! This module provides the syntax-side companion: callers can ask whether a
//! recognized XSD datatype lexical is valid, obtain a canonical lexical form,
//! and flag ill-typed RDF literals without rewriting the stored GTS term.

use std::fmt;
use std::str::FromStr;

use ciborium::value::Value;
use oxsdatatypes::{
    Boolean, Date, DateTime, DayTimeDuration, Double, Duration, Float, GDay, GMonth, GMonthDay,
    GYear, GYearMonth, Time, YearMonthDuration,
};

use crate::model::{
    Diagnostic, Graph, Term, TermKind, RDF_DIR_LANG_STRING, RDF_LANG_STRING, XSD_STRING,
};

/// XML Schema namespace used by RDF typed literals.
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
/// Diagnostic code emitted for recognized XSD literals with invalid lexical forms.
pub const ILL_TYPED_LITERAL_CODE: &str = "IllTypedLiteral";
/// GTS metadata key carrying round-trippable ill-typed literal sidecar rows.
pub const ILL_TYPED_LITERAL_META_KEY: &str = "gts:illTypedLiterals";

/// Lexical validation result for one literal/datatype pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XsdLexicalStatus {
    /// The datatype is recognized and the lexical form is valid.
    Valid {
        /// Canonical lexical form for the datatype value.
        canonical: String,
    },
    /// The datatype is recognized, but the lexical form is invalid.
    Invalid {
        /// Human-readable parse/facet failure.
        reason: String,
    },
    /// The datatype is not covered by this syntax-side layer.
    Unsupported,
}

impl XsdLexicalStatus {
    /// Return `true` only when the datatype is recognized and valid.
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }

    /// Return `true` only when the datatype is recognized and invalid.
    pub fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid { .. })
    }

    /// Borrow the canonical lexical form for valid recognized literals.
    pub fn canonical(&self) -> Option<&str> {
        match self {
            Self::Valid { canonical } => Some(canonical),
            Self::Invalid { .. } | Self::Unsupported => None,
        }
    }

    /// Borrow the invalid reason for ill-typed recognized literals.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Invalid { reason } => Some(reason),
            Self::Valid { .. } | Self::Unsupported => None,
        }
    }
}

/// One ill-typed literal observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IllTypedLiteral {
    /// GTS term id of the literal.
    pub term_id: usize,
    /// Effective datatype IRI after GTS literal defaulting.
    pub datatype_iri: String,
    /// Original literal lexical form preserved by GTS.
    pub lexical: String,
    /// Syntax/facet failure detail.
    pub reason: String,
}

impl IllTypedLiteral {
    /// Build a stable graph diagnostic for this observation.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            code: ILL_TYPED_LITERAL_CODE.to_string(),
            detail: format!(
                "term {} literal {:?} is ill-typed for {}: {}",
                self.term_id, self.lexical, self.datatype_iri, self.reason
            ),
            frame_index: None,
        }
    }
}

/// Validate a lexical form for a datatype IRI and return a canonical lexical form.
///
/// Unsupported datatypes return [`XsdLexicalStatus::Unsupported`] instead of
/// guessing. Bad lexical forms for recognized datatypes return `Invalid`; they
/// should be flagged, not rejected, by RDF importers.
pub fn validate_lexical(datatype_iri: &str, lexical: &str) -> XsdLexicalStatus {
    let Some(local) = datatype_iri.strip_prefix(XSD_NS) else {
        return XsdLexicalStatus::Unsupported;
    };

    match local {
        "string" | "anyURI" => valid(lexical.to_string()),
        "normalizedString" => valid(replace_xml_whitespace(lexical)),
        "token" => valid(collapse_xml_whitespace(lexical)),
        "boolean" => parse_display::<Boolean>(lexical, "boolean"),
        "decimal" => canonical_decimal(lexical)
            .map(valid)
            .unwrap_or_else(invalid),
        "integer" => canonical_integer(lexical)
            .map(|canonical| valid(canonical.lexical))
            .unwrap_or_else(invalid),
        "nonPositiveInteger" | "negativeInteger" | "long" | "int" | "short" | "byte"
        | "nonNegativeInteger" | "unsignedLong" | "unsignedInt" | "unsignedShort"
        | "unsignedByte" | "positiveInteger" => validate_integer_family(local, lexical),
        "float" => validate_float::<Float>(lexical, "float"),
        "double" => validate_float::<Double>(lexical, "double"),
        "dateTime" => parse_display::<DateTime>(lexical, "dateTime"),
        "date" => parse_display::<Date>(lexical, "date"),
        "time" => parse_display::<Time>(lexical, "time"),
        "gYearMonth" => parse_display::<GYearMonth>(lexical, "gYearMonth"),
        "gYear" => parse_display::<GYear>(lexical, "gYear"),
        "gMonthDay" => parse_display::<GMonthDay>(lexical, "gMonthDay"),
        "gMonth" => parse_display::<GMonth>(lexical, "gMonth"),
        "gDay" => parse_display::<GDay>(lexical, "gDay"),
        "duration" => parse_display::<Duration>(lexical, "duration"),
        "yearMonthDuration" => parse_display::<YearMonthDuration>(lexical, "yearMonthDuration"),
        "dayTimeDuration" => parse_display::<DayTimeDuration>(lexical, "dayTimeDuration"),
        "hexBinary" => validate_hex_binary(lexical),
        _ => XsdLexicalStatus::Unsupported,
    }
}

/// Return ill-typed literal observations for recognized XSD terms in a graph.
pub fn ill_typed_literals(graph: &Graph) -> Vec<IllTypedLiteral> {
    ill_typed_literals_in_terms(&graph.terms)
}

/// Append `IllTypedLiteral` diagnostics and metadata to a graph if needed.
pub fn annotate_ill_typed_literals(graph: &mut Graph) {
    let items = ill_typed_literals(graph);
    if items.is_empty() {
        return;
    }
    graph
        .diagnostics
        .extend(items.iter().map(IllTypedLiteral::diagnostic));
    graph.set_meta(
        ILL_TYPED_LITERAL_META_KEY.to_string(),
        ill_typed_literals_metadata(&items),
    );
}

/// CBOR metadata payload for ill-typed literal sidecar rows.
pub fn ill_typed_literals_metadata(items: &[IllTypedLiteral]) -> Value {
    Value::Map(vec![
        ("version".into(), 1.into()),
        (
            "items".into(),
            Value::Array(items.iter().map(ill_typed_literal_value).collect()),
        ),
    ])
}

pub(crate) fn ill_typed_literals_in_terms(terms: &[Term]) -> Vec<IllTypedLiteral> {
    let mut out = Vec::new();
    for (term_id, term) in terms.iter().enumerate() {
        if term.kind != TermKind::Literal {
            continue;
        }
        let Some(lexical) = term.value.as_deref() else {
            continue;
        };
        let datatype_iri = effective_datatype_iri(terms, term);
        if let XsdLexicalStatus::Invalid { reason } = validate_lexical(&datatype_iri, lexical) {
            out.push(IllTypedLiteral {
                term_id,
                datatype_iri,
                lexical: lexical.to_string(),
                reason,
            });
        }
    }
    out
}

fn valid(canonical: String) -> XsdLexicalStatus {
    XsdLexicalStatus::Valid { canonical }
}

fn invalid(reason: String) -> XsdLexicalStatus {
    XsdLexicalStatus::Invalid { reason }
}

fn parse_display<T>(lexical: &str, name: &str) -> XsdLexicalStatus
where
    T: FromStr + fmt::Display,
    T::Err: fmt::Display,
{
    T::from_str(lexical)
        .map(|value| valid(value.to_string()))
        .unwrap_or_else(|err| invalid(format!("invalid xsd:{name} lexical form: {err}")))
}

fn validate_float<T>(lexical: &str, name: &str) -> XsdLexicalStatus
where
    T: FromStr + fmt::Display,
    T::Err: fmt::Display,
{
    if !is_xsd_float_lexical(lexical) {
        return invalid(format!("invalid xsd:{name} lexical form"));
    }
    parse_display::<T>(lexical, name)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalInteger {
    lexical: String,
    sign: i8,
}

fn validate_integer_family(local: &str, lexical: &str) -> XsdLexicalStatus {
    let canonical = match canonical_integer(lexical) {
        Ok(canonical) => canonical,
        Err(reason) => return invalid(reason),
    };

    let valid_facet = match local {
        "nonPositiveInteger" => canonical.sign <= 0,
        "negativeInteger" => canonical.sign < 0,
        "nonNegativeInteger" => canonical.sign >= 0,
        "positiveInteger" => canonical.sign > 0,
        "long" => integer_in_range(&canonical.lexical, i64::MIN as i128, i64::MAX as i128),
        "int" => integer_in_range(&canonical.lexical, i32::MIN as i128, i32::MAX as i128),
        "short" => integer_in_range(&canonical.lexical, i16::MIN as i128, i16::MAX as i128),
        "byte" => integer_in_range(&canonical.lexical, i8::MIN as i128, i8::MAX as i128),
        "unsignedLong" => integer_in_range(&canonical.lexical, 0, u64::MAX as i128),
        "unsignedInt" => integer_in_range(&canonical.lexical, 0, u32::MAX as i128),
        "unsignedShort" => integer_in_range(&canonical.lexical, 0, u16::MAX as i128),
        "unsignedByte" => integer_in_range(&canonical.lexical, 0, u8::MAX as i128),
        _ => true,
    };

    if valid_facet {
        valid(canonical.lexical)
    } else {
        invalid(format!("xsd:{local} facet violation"))
    }
}

fn canonical_integer(lexical: &str) -> Result<CanonicalInteger, String> {
    if lexical.is_empty() {
        return Err("integer lexical form is empty".to_string());
    }

    let (negative, digits) = match lexical.as_bytes()[0] {
        b'+' => (false, &lexical[1..]),
        b'-' => (true, &lexical[1..]),
        _ => (false, lexical),
    };
    if digits.is_empty() {
        return Err("integer lexical form has no digits".to_string());
    }
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("integer lexical form contains a non-digit character".to_string());
    }

    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return Ok(CanonicalInteger {
            lexical: "0".to_string(),
            sign: 0,
        });
    }
    Ok(CanonicalInteger {
        lexical: if negative {
            format!("-{trimmed}")
        } else {
            trimmed.to_string()
        },
        sign: if negative { -1 } else { 1 },
    })
}

fn integer_in_range(lexical: &str, min: i128, max: i128) -> bool {
    lexical
        .parse::<i128>()
        .is_ok_and(|value| value >= min && value <= max)
}

fn canonical_decimal(lexical: &str) -> Result<String, String> {
    if lexical.is_empty() {
        return Err("decimal lexical form is empty".to_string());
    }

    let (negative, body) = match lexical.as_bytes()[0] {
        b'+' => (false, &lexical[1..]),
        b'-' => (true, &lexical[1..]),
        _ => (false, lexical),
    };
    if body.is_empty() {
        return Err("decimal lexical form has no digits".to_string());
    }
    if body.bytes().filter(|byte| *byte == b'.').count() > 1 {
        return Err("decimal lexical form has more than one decimal point".to_string());
    }
    let (whole, fractional) = body.split_once('.').unwrap_or((body, ""));
    if whole.is_empty() && fractional.is_empty() {
        return Err("decimal lexical form has no digits".to_string());
    }
    if !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("decimal lexical form contains an invalid character".to_string());
    }

    let whole = whole.trim_start_matches('0');
    let fractional = fractional.trim_end_matches('0');
    if whole.is_empty() && fractional.is_empty() {
        return Ok("0.0".to_string());
    }

    let mut canonical = String::new();
    if negative {
        canonical.push('-');
    }
    canonical.push_str(if whole.is_empty() { "0" } else { whole });
    canonical.push('.');
    canonical.push_str(if fractional.is_empty() {
        "0"
    } else {
        fractional
    });
    Ok(canonical)
}

fn is_xsd_float_lexical(lexical: &str) -> bool {
    matches!(lexical, "INF" | "-INF" | "NaN") || is_decimal_with_optional_exponent(lexical)
}

fn is_decimal_with_optional_exponent(lexical: &str) -> bool {
    let exponent_markers = lexical
        .bytes()
        .filter(|byte| matches!(byte, b'e' | b'E'))
        .count();
    if exponent_markers > 1 {
        return false;
    }
    let (mantissa, exponent) = match lexical.find(['e', 'E']) {
        Some(index) => (&lexical[..index], Some(&lexical[index + 1..])),
        None => (lexical, None),
    };
    if !is_decimal_mantissa(mantissa) {
        return false;
    }
    exponent.is_none_or(is_signed_digits)
}

fn is_decimal_mantissa(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let body = match value.as_bytes()[0] {
        b'+' | b'-' => &value[1..],
        _ => value,
    };
    if body.is_empty() || body.bytes().filter(|byte| *byte == b'.').count() > 1 {
        return false;
    }
    let (whole, fractional) = body.split_once('.').unwrap_or((body, ""));
    (!whole.is_empty() || !fractional.is_empty())
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fractional.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_signed_digits(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let digits = match value.as_bytes()[0] {
        b'+' | b'-' => &value[1..],
        _ => value,
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_hex_binary(lexical: &str) -> XsdLexicalStatus {
    if !lexical.as_bytes().chunks_exact(2).remainder().is_empty() {
        return invalid("xsd:hexBinary lexical form has an odd number of digits".to_string());
    }
    if !lexical.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return invalid("xsd:hexBinary lexical form contains a non-hex digit".to_string());
    }
    valid(lexical.to_ascii_uppercase())
}

fn replace_xml_whitespace(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if matches!(ch, '\t' | '\n' | '\r') {
                ' '
            } else {
                ch
            }
        })
        .collect()
}

fn collapse_xml_whitespace(value: &str) -> String {
    replace_xml_whitespace(value)
        .split(' ')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn effective_datatype_iri(terms: &[Term], term: &Term) -> String {
    if let Some(datatype) = term.datatype {
        return terms
            .get(datatype)
            .and_then(|term| term.value.clone())
            .unwrap_or_else(|| XSD_STRING.to_string());
    }
    if term.lang.is_some() && matches!(term.direction.as_deref(), Some("ltr" | "rtl")) {
        RDF_DIR_LANG_STRING.to_string()
    } else if term.lang.is_some() {
        RDF_LANG_STRING.to_string()
    } else {
        XSD_STRING.to_string()
    }
}

fn ill_typed_literal_value(item: &IllTypedLiteral) -> Value {
    Value::Map(vec![
        (
            "term".into(),
            Value::Integer(u64::try_from(item.term_id).expect("usize fits u64").into()),
        ),
        ("datatype".into(), item.datatype_iri.clone().into()),
        ("lexical".into(), item.lexical.clone().into()),
        ("reason".into(), item.reason.clone().into()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xsd(local: &str) -> String {
        format!("{XSD_NS}{local}")
    }

    #[test]
    fn validates_and_canonicalizes_common_xsd_lexicals() {
        assert_eq!(
            validate_lexical(&xsd("boolean"), "1").canonical(),
            Some("true")
        );
        assert_eq!(
            validate_lexical(&xsd("integer"), "+00042").canonical(),
            Some("42")
        );
        assert_eq!(
            validate_lexical(&xsd("decimal"), "-001.2300").canonical(),
            Some("-1.23")
        );
        assert_eq!(
            validate_lexical(&xsd("dateTime"), "2026-06-10T20:00:00Z").canonical(),
            Some("2026-06-10T20:00:00Z")
        );
        assert_eq!(
            validate_lexical(&xsd("hexBinary"), "0a1B").canonical(),
            Some("0A1B")
        );
    }

    #[test]
    fn detects_invalid_recognized_xsd_lexicals_without_rejecting_terms() {
        assert!(validate_lexical(&xsd("boolean"), "maybe").is_invalid());
        assert!(validate_lexical(&xsd("integer"), "12.0").is_invalid());
        assert!(validate_lexical(&xsd("unsignedByte"), "256").is_invalid());
        assert!(validate_lexical(&xsd("date"), "2026-02-31").is_invalid());
    }

    #[test]
    fn leaves_unrecognized_datatypes_unsupported() {
        assert_eq!(
            validate_lexical("https://example.org/customDatatype", "not our syntax"),
            XsdLexicalStatus::Unsupported
        );
    }
}
