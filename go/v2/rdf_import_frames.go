// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"errors"
	"fmt"
	"slices"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

const (
	termsFrameType      = 1
	quadsFrameType      = 2
	emptyGraphFrameType = 5
	arrivalOrder        = 2
	unorderedRows       = 2
	termsPerFrame       = 4096
	rowsPerFrame        = 16_384
	restartGroup        = 64
	rdfFormedCapacity   = 1 << 37
)

type rdfSegmentWriter struct {
	segment            *SegmentWriter
	capacity           ReadCapacity
	items              uint64
	decoded            uint64
	formed             uint64
	valueTextRemaining uint64
	lexicalValues      map[string]Digest
	valueUnits         map[Digest]struct{}
	wroteOptionalValue bool
}

type rdfImportCapacityError string

func (e rdfImportCapacityError) Error() string { return string(e) }

func writeRDF(terms []rdfTerm, quads []rdfQuad, empty []int) ([]byte, error) {
	return writeRDFSegment(terms, quads, empty, DefaultReadCapacity(), rdfValueTextLimit, true)
}

func writeRDFSegment(
	terms []rdfTerm, quads []rdfQuad, empty []int, capacity ReadCapacity, valueTextLimit uint64, allowOptional bool,
) ([]byte, error) {
	segment, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		return nil, err
	}
	writer := rdfSegmentWriter{
		segment: segment, capacity: capacity, valueTextRemaining: valueTextLimit,
		items: 2, // Include the opener and the closing commit.
	}
	data, err := writer.dataset(terms, quads, empty, allowOptional)
	var limit rdfImportCapacityError
	if writer.wroteOptionalValue && errors.As(err, &limit) {
		// A fitting inline import may need the space occupied by optional
		// attachments. Retry privately with the mandatory references only.
		return writeRDFSegment(terms, quads, empty, capacity, valueTextLimit, false)
	}
	return data, err
}

func (w *rdfSegmentWriter) dataset(terms []rdfTerm, quads []rdfQuad, empty []int, allowOptional bool) ([]byte, error) {
	if err := w.writeLiteralValues(terms, allowOptional); err != nil {
		return nil, err
	}
	base := 0
	for batch := range slices.Chunk(terms, termsPerFrame) {
		if err := w.terms(base, batch); err != nil {
			return nil, err
		}
		base += len(batch)
	}
	for batch := range slices.Chunk(quads, rowsPerFrame) {
		if err := w.quads(batch); err != nil {
			return nil, err
		}
	}
	for batch := range slices.Chunk(empty, rowsPerFrame) {
		payload := [][]int{deltaColumn(len(batch), func(i int) int { return batch[i] })}
		if err := w.appendRecord(emptyGraphFrameType, []int{len(batch)}, payload); err != nil {
			return nil, err
		}
	}
	if _, err := w.segment.Commit(); err != nil {
		return nil, err
	}
	return w.segment.Bytes(), nil
}

func (w *rdfSegmentWriter) payloadLimit() uint64 {
	return min(w.capacity.StoredBytesPerFrame, w.capacity.DecodedBytesPerFrame)
}

func (w *rdfSegmentWriter) append(frameType uint64, content, payload []byte, formed uint64) error {
	for _, limit := range []struct {
		used, capacity uint64
		resource       string
	}{
		{uint64(len(content)), w.capacity.StoredBytesPerFrame, "content bytes per frame"},
		{uint64(len(payload)), w.payloadLimit(), "payload bytes per frame"},
		{w.decoded + uint64(len(payload)), w.capacity.DecodedBytesPerRead, "decoded bytes per read"},
		{w.formed + formed, rdfFormedCapacity, "formed prefix bytes per read"},
		{w.items + 1, w.capacity.ItemsPerRead, "items per read"},
	} {
		if limit.used > limit.capacity {
			return rdfImportCapacityError(fmt.Sprintf("RDF import needs %d %s, limit %d", limit.used, limit.resource, limit.capacity))
		}
	}
	if _, err := w.segment.Append(frameType, content, payload); err != nil {
		return err
	}
	w.items++
	w.decoded += uint64(len(payload))
	w.formed += formed
	return nil
}

func (w *rdfSegmentWriter) appendRecord(frameType uint64, record, rows any) error {
	content, err := wire.Encode(record)
	if err != nil {
		return err
	}
	payload, err := wire.Encode(rows)
	if err != nil {
		return err
	}
	return w.append(frameType, content, payload, 0)
}

func (w *rdfSegmentWriter) terms(base int, terms []rdfTerm) error {
	if w.termDataFits(terms) {
		content, payload, formed, err := encodeRDFTerms(base, terms, w.lexicalValues)
		if err != nil {
			return err
		}
		if len(terms) == 1 || uint64(len(payload)) <= w.payloadLimit() && uint64(len(content)) <= w.capacity.StoredBytesPerFrame {
			return w.append(termsFrameType, content, payload, formed)
		}
	}
	if len(terms) == 1 {
		return rdfImportCapacityError(fmt.Sprintf("RDF import term %d exceeds the limit of %d payload bytes per frame", base, w.payloadLimit()))
	}

	// Split a dictionary batch without renumbering its terms. Each half
	// has independent front-coding runs and a fresh language table.
	middle := len(terms) / 2
	if err := w.terms(base, terms[:middle]); err != nil {
		return err
	}
	return w.terms(base+middle, terms[middle:])
}

type termsPayload struct {
	languages   []string
	languageIDs map[string]int
	values      map[string]Digest
	formed      uint64
}

func encodeRDFTerms(base int, terms []rdfTerm, values map[string]Digest) ([]byte, []byte, uint64, error) {
	body := termsPayload{languages: []string{}, languageIDs: make(map[string]int), values: values}
	var runs [][2]int
	var blocks []any
	for len(terms) > 0 {
		end := 1
		kind := body.storageKind(terms[0])
		for end < len(terms) && body.storageKind(terms[end]) == kind {
			end++
		}
		runs = append(runs, [2]int{kind, end})
		blocks = append(blocks, body.block(kind, terms[:end]))
		terms = terms[end:]
	}
	content, err := wire.Encode([]any{base, runs, arrivalOrder, restartGroup})
	if err != nil {
		return nil, nil, 0, err
	}
	payload, err := wire.Encode([]any{blocks, body.languages})
	return content, payload, body.formed, err
}

func (b *termsPayload) storageKind(term rdfTerm) int {
	if term.kind == model.Literal {
		if _, exists := b.values[term.text]; exists {
			return literalRefBlock
		}
	}
	return int(term.kind) + 1
}

func (b *termsPayload) block(code int, terms []rdfTerm) []any {
	if code == literalRefBlock {
		digests := make([][]byte, 0, len(terms))
		lengths := make([]int, 0, len(terms))
		for _, term := range terms {
			digest := b.values[term.text]
			digests = append(digests, digest[:])
			lengths = append(lengths, len(term.text))
		}
		return b.literalFields([]any{code, digests, lengths}, terms)
	}
	switch terms[0].kind {
	case model.Bnode:
		labels := make([][]byte, 0, len(terms))
		for _, term := range terms {
			labels = append(labels, []byte(term.text))
		}
		return []any{code, labels}
	case model.Triple:
		subjects := make([]int, 0, len(terms))
		predicates := make([]int, 0, len(terms))
		objects := make([]int, 0, len(terms))
		for _, term := range terms {
			subjects = append(subjects, term.triple.S)
			predicates = append(predicates, term.triple.P)
			objects = append(objects, term.triple.O)
		}
		return []any{code, subjects, predicates, objects}
	default:
		return b.frontCodedBlock(code, terms)
	}
}

func (b *termsPayload) frontCodedBlock(code int, terms []rdfTerm) []any {
	prefixes := make([]int, 0, len(terms))
	lengths := make([]int, 0, len(terms))
	suffixes := []byte{}
	var previous string
	for index, term := range terms {
		prefix := frontCodedPrefix(previous, term.text, index)
		prefixes = append(prefixes, prefix)
		lengths = append(lengths, len(term.text)-prefix)
		suffixes = append(suffixes, term.text[prefix:]...)
		b.formed = min(rdfFormedCapacity+1, b.formed+uint64(prefix))
		previous = term.text
	}
	block := []any{code, prefixes, lengths, suffixes}
	if terms[0].kind != model.Literal {
		return block
	}
	return b.literalFields(block, terms)
}

func frontCodedPrefix(previous, text string, index int) int {
	prefix := 0
	if index%restartGroup != 0 {
		for prefix < min(len(previous), len(text)) && previous[prefix] == text[prefix] {
			prefix++
		}
	}
	return prefix
}

func (b *termsPayload) literalFields(block []any, terms []rdfTerm) []any {
	forms := make([]int, 0, len(terms))
	datatypes := []int{}
	languages := []int{}
	for _, term := range terms {
		forms = append(forms, term.form)
		if term.form == 1 {
			datatypes = append(datatypes, term.datatype)
		} else {
			index, ok := b.languageIDs[term.lang]
			if !ok {
				index = len(b.languages)
				b.languageIDs[term.lang] = index
				b.languages = append(b.languages, term.lang)
			}
			languages = append(languages, index)
		}
	}
	return append(block, forms, datatypes, languages)
}

func (w *rdfSegmentWriter) quads(rows []rdfQuad) error {
	graphs := quadRuns(rows, func(row rdfQuad) int { return row.graph }, func(id int) any {
		if id < 0 {
			return []int{1}
		}
		return []int{2, id}
	})
	predicates := quadRuns(rows, func(row rdfQuad) int { return row.predicate }, func(id int) any { return id })
	objects := deltaColumn(len(rows), func(i int) int { return rows[i].object })
	subjects := deltaColumn(len(rows), func(i int) int { return rows[i].subject })
	return w.appendRecord(quadsFrameType, []int{unorderedRows, len(rows)}, []any{graphs, predicates, objects, subjects})
}

func quadRuns(rows []rdfQuad, key func(rdfQuad) int, value func(int) any) [][]any {
	var runs [][]any
	for len(rows) > 0 {
		id := key(rows[0])
		end := 1
		for end < len(rows) && key(rows[end]) == id {
			end++
		}
		runs = append(runs, []any{value(id), end})
		rows = rows[end:]
	}
	return runs
}

func deltaColumn(count int, value func(int) int) []int {
	column := make([]int, 0, count)
	previous := 0
	for i := range count {
		id := value(i)
		column = append(column, id-previous)
		previous = id
	}
	return column
}
