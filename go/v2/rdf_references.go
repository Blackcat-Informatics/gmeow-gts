// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/model"
)

func (p *rdfProjector) addReferenceBlock(dictionary *rdfReadDictionary, block rdfBlock, languages []string) Reason {
	octets := block.kind == 7
	for i, digest := range block.digests {
		key := rebuildRef{digest: digest, length: block.lengths[i]}
		value := p.rebuild.values[key]
		reason := referenceFailure(value)
		if !octets && reason == "" && !value.validateText() {
			return illFormedTerm
		}
		var term rdfTerm
		if octets {
			term = rdfTerm{kind: model.Literal, form: 1}
		} else {
			var c rdfCursor
			term = p.literalFromBlock(&c, dictionary.entries, &block, languages, i, "")
			if c.reason != "" {
				return c.reason
			}
		}
		if reason != "" {
			return reason
		}
		term.text, reason = value.lexical(key.length, octets, &p.valueTextRemaining)
		if reason != "" {
			return reason
		}
		if octets {
			term.datatype = p.intern(rdfTerm{kind: model.Iri, text: rdfValueDatatype + "hexBinary"})
		}
		dictionary.append(rdfWrittenKey{kind: block.kind, rdf: p.intern(term), value: value})
	}
	return ""
}

func referenceFailure(value *rebuiltValue) Reason {
	if value == nil {
		return "missing_value"
	}
	return value.reason
}

func (v *rebuiltValue) validateText() bool {
	if v.validText {
		return true
	}
	var tail [utf8.UTFMax]byte
	held := 0
	for _, part := range v.parts {
		if held != 0 {
			read := copy(tail[held:], part)
			prefix := tail[:held+read]
			if !utf8.FullRune(prefix) {
				held += read
				continue
			}
			character, size := utf8.DecodeRune(prefix)
			if character == utf8.RuneError && size == 1 {
				return false
			}
			part = part[size-held:]
			held = 0
		}
		// Only the last three bytes can begin an incomplete rune. Verify
		// the complete portion and carry any incomplete tail to the next unit.
		end := len(part)
		for i := end - 1; i >= 0 && end-i < utf8.UTFMax; i-- {
			if utf8.RuneStart(part[i]) {
				if !utf8.FullRune(part[i:]) {
					held = copy(tail[:], part[i:])
					end = i
				}
				break
			}
		}
		if !utf8.Valid(part[:end]) {
			return false
		}
	}
	v.validText = held == 0
	return v.validText
}
