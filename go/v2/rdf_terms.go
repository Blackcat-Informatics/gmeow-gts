// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"math"
	"math/bits"
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/model"
)

type rdfTermRun struct {
	kind  uint64
	count uint64
}

type rdfBlock struct {
	kind              uint64
	prefixes, lengths []uint64
	suffixes          []byte
	forms, datatypes  []uint64
	languages         []uint64
	labels            [][]byte
	triples           [3][]uint64
	valueCount        uint64
	values            []byte
	texts             []string
}

func (p *rdfProjector) readTerms(dictionary *rdfReadDictionary, content, payload []byte) Reason {
	base := uint64(len(dictionary.entries))
	fields := rdfCursor{data: content}
	fields.record(4)
	fields.check(fields.uint() == base, inconsistentCounts)
	runs := readRDFTermRuns(&fields)
	fields.check(rdfTermIDsFit(base, runs), OutOfRange)
	fields.code(2)
	restart := fields.positive()
	if reason := fields.end(); reason != "" {
		return reason
	}

	c := rdfCursor{data: payload}
	c.record(2)
	blocks := rdfColumn(&c, func() rdfBlock { return readRDFBlock(&c) })
	languages := rdfColumn(&c, c.text)
	if reason := c.end(); reason != "" {
		return reason
	}
	if len(blocks) != len(runs) {
		return inconsistentCounts
	}
	for index, block := range blocks {
		if !block.matchesCount(runs[index].count) {
			return inconsistentCounts
		}
	}
	for index, block := range blocks {
		if block.kind != runs[index].kind {
			return ForbiddenCombination
		}
	}
	formed := p.formed
	for _, block := range blocks {
		for _, prefix := range block.prefixes {
			if prefix > rdfFormedCapacity-formed {
				return "total_formed"
			}
			formed += prefix
		}
	}
	p.formed = formed
	for index := range blocks {
		if reason := blocks[index].formTexts(restart); reason != "" {
			return reason
		}
	}
	if reason := checkRDFLanguages(blocks, languages); reason != "" {
		return reason
	}
	for _, block := range blocks {
		p.addBlock(&c, dictionary, block, languages)
		if c.reason != "" {
			return c.reason
		}
	}
	return ""
}

func readRDFTermRuns(c *rdfCursor) []rdfTermRun {
	var previous uint64
	runs := rdfColumn(c, func() rdfTermRun {
		c.record(2)
		kind := c.code(7)
		c.check(kind != previous, NotDeterministic)
		previous = kind
		return rdfTermRun{kind: kind, count: c.positive()}
	})
	c.check(len(runs) > 0, OutOfRange)
	return runs
}

func rdfTermIDsFit(base uint64, runs []rdfTermRun) bool {
	for index, run := range runs {
		next, overflow := bits.Add64(base, run.count, 0)
		if overflow != 0 {
			// A final exclusive bound of 2^64 still gives a valid last ID.
			return next == 0 && index == len(runs)-1
		}
		base = next
	}
	return true
}

func readRDFBlock(c *rdfCursor) rdfBlock {
	length := c.takeHead(4, false).value
	c.check(length > 0, WrongLength)
	block := rdfBlock{kind: c.code(7)}
	switch block.kind {
	case 1, 2:
		arity := uint64(4)
		if block.kind == 2 {
			arity = 7
		}
		c.check(length == arity, WrongLength)
		block.prefixes = rdfColumn(c, c.uint)
		block.lengths = rdfColumn(c, c.uint)
		block.suffixes = c.body(2)
		if block.kind == 2 {
			block.forms = rdfColumn(c, func() uint64 { return c.code(4) })
			block.datatypes = rdfColumn(c, c.uint)
			block.languages = rdfColumn(c, c.uint)
		}
	case 3:
		c.check(length == 2, WrongLength)
		block.labels = rdfColumn(c, func() []byte { return c.body(2) })
	case 4:
		c.check(length == 4, WrongLength)
		for i := range block.triples {
			block.triples[i] = rdfColumn(c, c.uint)
		}
	case 6:
		c.check(length == 2, WrongLength)
		block.valueCount = c.list()
		start := c.pos
		for index := uint64(0); index < block.valueCount && c.reason == ""; index++ {
			c.inlineValue()
		}
		// Keep the validated bytes for projection after admission. The
		// smallest values use three bytes each on the wire.
		block.values = c.data[start:c.pos]
	default:
		c.check(false, "unsupported_term_kind")
	}
	return block
}

func (b *rdfBlock) matchesCount(count uint64) bool {
	switch b.kind {
	case 1:
		return uint64(len(b.prefixes)) == count && uint64(len(b.lengths)) == count
	case 2:
		if uint64(len(b.prefixes)) != count || uint64(len(b.lengths)) != count || uint64(len(b.forms)) != count {
			return false
		}
		typed := 0
		for _, form := range b.forms {
			if form == 1 {
				typed++
			}
		}
		return len(b.datatypes) == typed && len(b.languages) == len(b.forms)-typed
	case 3:
		return uint64(len(b.labels)) == count
	case 4:
		return uint64(len(b.triples[0])) == count && uint64(len(b.triples[1])) == count && uint64(len(b.triples[2])) == count
	case 6:
		return b.valueCount == count
	default:
		return false
	}
}

func checkRDFLanguages(blocks []rdfBlock, languages []string) Reason {
	for _, block := range blocks {
		for _, index := range block.languages {
			if index >= uint64(len(languages)) {
				return OutOfRange
			}
		}
	}
	used := uint64(0)
	seen := make(map[string]struct{})
	for _, block := range blocks {
		for _, index := range block.languages {
			if index > used {
				return NotDeterministic
			}
			if index == used {
				text := languages[index]
				if _, duplicate := seen[text]; duplicate {
					return NotDeterministic
				}
				seen[text] = struct{}{}
				used++
			}
		}
	}
	if used != uint64(len(languages)) {
		return NotDeterministic
	}
	return ""
}

func (p *rdfProjector) addBlock(c *rdfCursor, dictionary *rdfReadDictionary, block rdfBlock, languages []string) {
	switch block.kind {
	case 1, 2:
		p.addTextBlock(c, dictionary, block, languages)
	case 3:
		for _, label := range block.labels {
			id := p.intern(rdfTerm{kind: model.Bnode, text: string(label)})
			dictionary.append(rdfWrittenKey{kind: block.kind, rdf: id})
		}
	case 4:
		for i, subject := range block.triples[0] {
			predicate, object := block.triples[1][i], block.triples[2][i]
			entries := dictionary.entries
			triple := model.Triple3{
				S: p.node(c, entries, subject),
				P: p.resolve(c, entries, predicate, model.Iri),
				O: p.resolve(c, entries, object),
			}
			if c.reason != "" {
				return
			}
			id := p.intern(rdfTerm{kind: model.Triple, triple: triple})
			written := model.Triple3{S: entries[subject].written, P: entries[predicate].written, O: entries[object].written}
			dictionary.append(rdfWrittenKey{kind: block.kind, rdf: id, components: written})
		}
	case 6:
		c.reason = p.addInlineValues(dictionary, block)
	}
}

func (b *rdfBlock) formTexts(restart uint64) Reason {
	remaining := uint64(len(b.suffixes))
	for _, length := range b.lengths {
		if length > remaining {
			return inconsistentCounts
		}
		remaining -= length
	}
	if remaining != 0 {
		return inconsistentCounts
	}
	b.texts = make([]string, 0, min(len(b.prefixes), rdfDecodeBatch))
	suffixes := b.suffixes
	var previous string
	for index, prefix := range b.prefixes {
		length := b.lengths[index]
		atRestart := uint64(index)%restart == 0
		if index != 0 && prefix > uint64(len(previous)) {
			return OutOfRange
		}
		if atRestart && prefix != 0 {
			return ForbiddenCombination
		}
		if length > math.MaxInt-prefix {
			return OutOfRange
		}
		suffix := suffixes[:length]
		suffixes = suffixes[length:]
		if !atRestart && prefix < uint64(len(previous)) && len(suffix) != 0 && previous[prefix] == suffix[0] {
			return NotDeterministic
		}
		previous = previous[:prefix] + string(suffix)
		b.texts = append(b.texts, previous)
	}
	return ""
}

func (p *rdfProjector) addTextBlock(c *rdfCursor, dictionary *rdfReadDictionary, block rdfBlock, languages []string) {
	for index, text := range block.texts {
		c.check(utf8.ValidString(text), WrongType)
		term := rdfTerm{kind: model.Iri, text: text}
		if block.kind == 2 {
			term.kind = model.Literal
			term.form = int(block.forms[index])
			if term.form == 1 {
				term.datatype = p.resolve(c, dictionary.entries, block.datatypes[0], model.Iri)
				block.datatypes = block.datatypes[1:]
			} else {
				term.lang = languages[block.languages[0]]
				block.languages = block.languages[1:]
			}
		}
		if c.reason != "" {
			return
		}
		dictionary.append(rdfWrittenKey{kind: block.kind, rdf: p.intern(term)})
	}
}
