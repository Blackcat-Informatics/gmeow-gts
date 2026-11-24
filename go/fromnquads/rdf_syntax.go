// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package fromnquads

import (
	"fmt"
	"strings"
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/nquads"
)

func rdfIRI(raw string) (string, error) {
	text := raw
	if strings.ContainsRune(raw, '\\') {
		var value strings.Builder
		reader := tokenizer{text: raw, rdf12: true}
		for {
			relative := strings.IndexByte(raw[reader.pos:], '\\')
			if relative < 0 {
				value.WriteString(raw[reader.pos:])
				break
			}
			value.WriteString(raw[reader.pos : reader.pos+relative])
			reader.pos += relative + 1
			if reader.pos >= len(raw) || raw[reader.pos] != 'u' && raw[reader.pos] != 'U' {
				return "", ParseError{"an IRI escape must use \\u or \\U"}
			}
			ch, err := reader.escape()
			if err != nil {
				return "", err
			}
			value.WriteRune(ch)
		}
		text = value.String()
	}
	if !nquads.ValidIRI(text) {
		return "", ParseError{fmt.Sprintf("invalid RDF IRI %q", text)}
	}
	return text, nil
}

func (t *tokenizer) rdfBlankLabel() (string, error) {
	start := t.pos
	first, width := utf8.DecodeRuneInString(t.text[start:])
	if width == 0 || !blankStart(first) {
		return "", ParseError{fmt.Sprintf("invalid blank node label at byte %d", start)}
	}
	t.pos += width
	end := t.pos
	for t.pos < len(t.text) {
		ch, width := utf8.DecodeRuneInString(t.text[t.pos:])
		if ch == '.' {
			t.pos++
			continue
		}
		if !blankContinue(ch) {
			break
		}
		t.pos += width
		end = t.pos
	}
	t.pos = end
	return t.text[start:end], nil
}

// RDF 1.2 N-Quads permits PN_CHARS_U and an ASCII digit initially.
func blankStart(ch rune) bool {
	return ch >= 'A' && ch <= 'Z' || ch >= 'a' && ch <= 'z' || ch >= '0' && ch <= '9' || ch == '_' ||
		ch >= 0x00c0 && ch <= 0x00d6 || ch >= 0x00d8 && ch <= 0x00f6 || ch >= 0x00f8 && ch <= 0x02ff ||
		ch >= 0x0370 && ch <= 0x037d || ch >= 0x037f && ch <= 0x1fff || ch >= 0x200c && ch <= 0x200d ||
		ch >= 0x2070 && ch <= 0x218f || ch >= 0x2c00 && ch <= 0x2fef || ch >= 0x3001 && ch <= 0xd7ff ||
		ch >= 0xf900 && ch <= 0xfdcf || ch >= 0xfdf0 && ch <= 0xfffd || ch >= 0x10000 && ch <= 0xeffff
}

func blankContinue(ch rune) bool {
	return blankStart(ch) || ch == '-' || ch == 0x00b7 || ch >= 0x0300 && ch <= 0x036f || ch >= 0x203f && ch <= 0x2040
}
