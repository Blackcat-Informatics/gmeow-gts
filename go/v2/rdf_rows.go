// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import "go.blackcatinformatics.ca/gts/model"

type rdfGraphID struct {
	id    uint64
	named bool
}

func (c *rdfCursor) graph() rdfGraphID {
	length := c.takeHead(4, false).value
	c.check(length > 0, WrongLength)
	code := c.code(2)
	c.check(length == code, WrongLength)
	if code == 2 {
		return rdfGraphID{id: c.uint(), named: true}
	}
	return rdfGraphID{}
}

type rdfRun[T comparable] struct {
	value T
	count uint64
}

type rdfRuns[T comparable] struct {
	runs []rdfRun[T]
	used uint64
}

func readRDFRuns[T comparable](c *rdfCursor, read func() T) rdfRuns[T] {
	var previous T
	seen := false
	runs := rdfColumn(c, func() rdfRun[T] {
		c.record(2)
		value := read()
		length := c.positive()
		c.check(!seen || previous != value, NotDeterministic)
		previous, seen = value, true
		return rdfRun[T]{value: value, count: length}
	})
	return rdfRuns[T]{runs: runs}
}

func (r *rdfRuns[T]) matchesCount(count uint64) bool {
	for _, run := range r.runs {
		if run.count > count {
			return false
		}
		count -= run.count
	}
	return count == 0
}

// next is used only after the runs and the row count have been checked.
func (r *rdfRuns[T]) next() T {
	run := r.runs[0]
	r.used++
	if r.used == run.count {
		r.runs = r.runs[1:]
		r.used = 0
	}
	return run.value
}

func readRDFDeltas(c *rdfCursor, minimum uint64) []uint64 {
	var previous uint64
	return rdfColumn(c, func() uint64 {
		previous = c.delta(previous)
		c.check(previous >= minimum, OutOfRange)
		return previous
	})
}

type rdfRowColumns struct {
	graphs                    rdfRuns[rdfGraphID]
	predicateRuns             rdfRuns[uint64]
	subjects, predicates      []uint64
	objects, annotationCounts []uint64
}

func readRDFRows(c *rdfCursor, frameType uint64) rdfRowColumns {
	var cols rdfRowColumns
	switch frameType {
	case quadsFrameType:
		c.record(4)
		cols.graphs = readRDFRuns(c, c.graph)
		cols.predicateRuns = readRDFRuns(c, c.uint)
		cols.objects = readRDFDeltas(c, 0)
		cols.subjects = readRDFDeltas(c, 0)
	case reifierFrameType:
		c.record(3)
		cols.graphs = readRDFRuns(c, c.graph)
		cols.subjects = readRDFDeltas(c, 0)
		cols.objects = readRDFDeltas(c, 0)
	case annotationFrameType:
		c.record(5)
		cols.graphs = readRDFRuns(c, c.graph)
		cols.subjects = readRDFDeltas(c, 0)
		cols.predicates = readRDFDeltas(c, 0)
		cols.objects = readRDFDeltas(c, 0)
		cols.annotationCounts = readRDFDeltas(c, 1)
	}
	return cols
}

func (c *rdfRowColumns) matchesCount(kind, count uint64) bool {
	if !c.graphs.matchesCount(count) || uint64(len(c.subjects)) != count || uint64(len(c.objects)) != count {
		return false
	}
	switch kind {
	case quadsFrameType:
		return c.predicateRuns.matchesCount(count)
	case annotationFrameType:
		return uint64(len(c.predicates)) == count && uint64(len(c.annotationCounts)) == count
	default:
		return true
	}
}

func (p *rdfProjector) readRows(dictionary []int, frameType uint64, content, payload []byte) Reason {
	fields := rdfCursor{data: content}
	if frameType == emptyGraphFrameType {
		fields.record(1)
	} else {
		fields.record(2)
		fields.code(2)
	}
	count := fields.positive()
	if reason := fields.end(); reason != "" {
		return reason
	}
	c := rdfCursor{data: payload}
	if frameType == emptyGraphFrameType {
		c.record(1)
		graphs := readRDFDeltas(&c, 0)
		if reason := c.end(); reason != "" {
			return reason
		}
		if uint64(len(graphs)) != count {
			return inconsistentCounts
		}
		for _, graph := range graphs {
			id := p.node(&c, dictionary, graph)
			if c.reason != "" {
				return c.reason
			}
			p.result.emptyGraphs = append(p.result.emptyGraphs, id)
		}
		return ""
	}
	columns := readRDFRows(&c, frameType)
	if reason := c.end(); reason != "" {
		return reason
	}
	if !columns.matchesCount(frameType, count) {
		return inconsistentCounts
	}
	if frameType == annotationFrameType && columns.hasDuplicateAnnotation(dictionary) {
		return UnorderedOrRepeated
	}
	return p.projectRows(&c, dictionary, frameType, &columns)
}

// Annotation rows are checked for written-alike terms before testing whether
// their IDs and term kinds are valid in each position.
func (c *rdfRowColumns) hasDuplicateAnnotation(dictionary []int) bool {
	type termKey struct {
		id      uint64
		defined bool
	}
	type annotationKey struct {
		subject, predicate, object, graph termKey
		named                             bool
	}
	key := func(id uint64) termKey {
		if id < uint64(len(dictionary)) {
			return termKey{id: uint64(dictionary[id]), defined: true}
		}
		return termKey{id: id}
	}
	seen := make(map[annotationKey]struct{})
	graphs := c.graphs
	for i, subject := range c.subjects {
		row := annotationKey{subject: key(subject), predicate: key(c.predicates[i]), object: key(c.objects[i])}
		if graph := graphs.next(); graph.named {
			row.graph, row.named = key(graph.id), true
		}
		if _, exists := seen[row]; exists {
			return true
		}
		seen[row] = struct{}{}
	}
	return false
}

func (p *rdfProjector) projectRows(c *rdfCursor, dictionary []int, kind uint64, cols *rdfRowColumns) Reason {
	var reifies int
	if kind == reifierFrameType {
		reifies = p.intern(rdfTerm{kind: model.Iri, text: rdfReifies})
	}
	for index, subject := range cols.subjects {
		row := rdfQuad{graph: -1}
		if graph := cols.graphs.next(); graph.named {
			row.graph = p.node(c, dictionary, graph.id)
		}
		row.subject = p.node(c, dictionary, subject)
		switch kind {
		case quadsFrameType:
			row.predicate = p.resolve(c, dictionary, cols.predicateRuns.next(), model.Iri)
			row.object = p.resolve(c, dictionary, cols.objects[index])
		case reifierFrameType:
			row.predicate = reifies
			row.object = p.resolve(c, dictionary, cols.objects[index], model.Triple)
		case annotationFrameType:
			row.predicate = p.resolve(c, dictionary, cols.predicates[index], model.Iri)
			row.object = p.resolve(c, dictionary, cols.objects[index])
		}
		if c.reason != "" {
			return c.reason
		}
		p.result.quads = append(p.result.quads, row)
	}
	return ""
}
