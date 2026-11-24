// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"fmt"

	"github.com/zeebo/blake3"
	"go.blackcatinformatics.ca/gts/model"
)

const (
	literalRefBlock       = 5
	rdfLongLiteralBytes   = 1 << 20
	rdfValueUnitBytes     = 256 << 10
	singleLiteralHeadroom = 64
)

// Required references take priority. Other long literals use the remaining
// value budget, while small or unselected literals retain their inline form.
func (w *rdfSegmentWriter) writeLiteralValues(terms []rdfTerm, allowOptional bool) error {
	for _, term := range terms {
		if term.kind != model.Literal || term.text == "" {
			continue
		}
		fits, err := w.singleLiteralFits(term)
		if err != nil {
			return err
		}
		if !fits {
			if err := w.writeLiteralValue(term.text, true); err != nil {
				return err
			}
		}
	}
	if !allowOptional {
		return nil
	}
	for _, term := range terms {
		if term.kind == model.Literal && len(term.text) >= rdfLongLiteralBytes {
			if err := w.writeLiteralValue(term.text, false); err != nil {
				return err
			}
		}
	}
	return nil
}

func (w *rdfSegmentWriter) singleLiteralFits(term rdfTerm) (bool, error) {
	raw := uint64(len(term.text)) + uint64(len(term.lang))
	limit := w.payloadLimit()
	if raw >= limit {
		return false, nil
	}
	// A singleton's array heads and integer fields fit in this margin.
	if limit-raw >= singleLiteralHeadroom {
		return true, nil
	}
	// Measure the real record near the boundary, using empty strings so
	// this check never allocates a frame-sized scratch buffer.
	empty := term
	empty.text, empty.lang = "", ""
	_, payload, _, err := encodeRDFTerms(0, []rdfTerm{empty}, nil)
	if err != nil {
		return false, err
	}
	lexicalHead := len(appendHead(nil, 0, uint64(len(term.text)))) - 1
	languageHead := len(appendHead(nil, 0, uint64(len(term.lang)))) - 1
	return raw+uint64(len(payload)+2*lexicalHead+languageHead) <= limit, nil
}

// Count the actual term bodies before allocating the payload. The encoded
// columns add overhead, so the caller still checks the encoded result.
func (w *rdfSegmentWriter) termDataFits(terms []rdfTerm) bool {
	remaining := w.payloadLimit()
	languages := make(map[string]struct{})
	body := termsPayload{values: w.lexicalValues}
	var previous string
	var previousKind, index int
	for _, term := range terms {
		kind := body.storageKind(term)
		if kind != previousKind {
			previous, previousKind, index = "", kind, 0
		}
		size := uint64(len(term.text))
		switch {
		case kind == literalRefBlock:
			size = 0
		case term.kind == model.Iri || term.kind == model.Literal:
			size -= uint64(frontCodedPrefix(previous, term.text, index))
		}
		if term.kind == model.Literal {
			if _, exists := languages[term.lang]; !exists && term.lang != "" {
				size += uint64(len(term.lang))
				languages[term.lang] = struct{}{}
			}
		}
		if size > remaining {
			return false
		}
		remaining -= size
		previous = term.text
		index++
	}
	return true
}

type rdfValueUnit struct {
	digest Digest
	text   string
}

func textUnitDigest(text string) (digest Digest) {
	h := blake3.NewDeriveKey(unitContext)
	_, _ = h.WriteString(text)
	h.Sum(digest[:0])
	return digest
}

func (w *rdfSegmentWriter) writeLiteralValue(text string, required bool) error {
	if _, exists := w.lexicalValues[text]; exists {
		return nil
	}
	length := uint64(len(text))
	unitSize := int(min(rdfValueUnitBytes, w.payloadLimit()))
	if length > w.valueTextRemaining || unitSize == 0 {
		if required {
			return fmt.Errorf("RDF import needs %d referenced literal bytes, %d remain within its projection limit", length, w.valueTextRemaining)
		}
		return nil
	}

	count := 1 + (len(text)-1)/unitSize
	lastSize := len(text) - (count-1)*unitSize
	var emptyDigest Digest
	entryHead := len(appendByteString(appendHead(nil, 4, 2), emptyDigest[:]))
	fullEntry := entryHead + len(appendHead(nil, 0, uint64(unitSize)))
	lastEntry := entryHead + len(appendHead(nil, 0, uint64(lastSize)))
	payloadSize := uint64(len(appendHead(nil, 4, uint64(count)))+lastEntry) + uint64(count-1)*uint64(fullEntry)
	if payloadSize > w.payloadLimit() {
		if required {
			return fmt.Errorf("RDF import value of %d bytes needs %d entry-list bytes per frame", length, payloadSize)
		}
		return nil
	}
	payload := appendHead(make([]byte, 0, int(payloadSize)), 4, uint64(count))
	units := make([]rdfValueUnit, 0, min(count, rdfDecodeBatch))
	newUnits := make(map[Digest]struct{})
	var unitBytes uint64
	for rest := text; len(rest) != 0; {
		size := min(len(rest), unitSize)
		chunk := rest[:size]
		rest = rest[size:]
		digest := textUnitDigest(chunk)
		payload = appendHead(payload, 4, 2)
		payload = appendByteString(payload, digest[:])
		payload = appendHead(payload, 0, uint64(size))
		if _, exists := w.valueUnits[digest]; exists {
			continue
		}
		if _, exists := newUnits[digest]; !exists {
			newUnits[digest] = struct{}{}
			units = append(units, rdfValueUnit{digest: digest, text: chunk})
			unitBytes += uint64(size)
		}
	}

	digest := textUnitDigest(text)
	content := appendByteString(appendHead(nil, 4, 3), digest[:])
	content = appendHead(content, 0, length)
	content = appendHead(content, 0, uint64(count))
	if uint64(len(content)) > w.capacity.StoredBytesPerFrame ||
		w.items+uint64(len(units))+1 > w.capacity.ItemsPerRead ||
		w.decoded+unitBytes+uint64(len(payload)) > w.capacity.DecodedBytesPerRead {
		if required {
			return fmt.Errorf("RDF import value of %d bytes exceeds the remaining frame capacities", length)
		}
		return nil
	}

	if w.valueUnits == nil {
		w.valueUnits = make(map[Digest]struct{})
	}
	for _, unit := range units {
		record := appendByteString(appendHead(nil, 4, 1), unit.digest[:])
		if err := w.append(chunkFrameType, record, []byte(unit.text), 0); err != nil {
			return err
		}
		w.valueUnits[unit.digest] = struct{}{}
	}
	if err := w.append(valueFrameType, content, payload, 0); err != nil {
		return err
	}
	if w.lexicalValues == nil {
		w.lexicalValues = make(map[string]Digest)
	}
	w.lexicalValues[text] = digest
	w.valueTextRemaining -= length
	w.wroteOptionalValue = w.wroteOptionalValue || !required
	return nil
}
