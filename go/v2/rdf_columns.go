// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"math"
	"unicode/utf8"
)

const inconsistentCounts Reason = "inconsistent_counts"
const rdfDecodeBatch = 256

// rdfCursor reads the fixed P3 records and columns. A failed step leaves the
// first reason in place so later steps cannot consume unchecked bytes.
type rdfCursor struct {
	data   []byte
	pos    int
	reason Reason
}

func (c *rdfCursor) takeHead(major byte, signed bool) head {
	if c.reason != "" {
		return head{}
	}
	h, reason := readHead(c.data, c.pos)
	switch {
	case reason != "" || h.major == 7 && h.indefinite:
		c.reason = WrongLength
	case h.major <= 4 && !h.shortest:
		c.reason = NotDeterministic
	case h.major != major && (!signed || h.major != 1):
		c.reason = WrongType
	default:
		c.pos = h.end
		return h
	}
	return head{}
}

func (c *rdfCursor) uint() uint64 { return c.takeHead(0, false).value }

func (c *rdfCursor) positive() uint64 {
	value := c.uint()
	c.check(value != 0, OutOfRange)
	return value
}

func (c *rdfCursor) code(maximum uint64) uint64 {
	value := c.uint()
	c.check(value > 0 && value <= maximum, UnregisteredValue)
	return value
}

func (c *rdfCursor) check(ok bool, reason Reason) {
	if !ok && c.reason == "" {
		c.reason = reason
	}
}

func (c *rdfCursor) record(arity uint64) {
	c.check(c.takeHead(4, false).value == arity, WrongLength)
}

func (c *rdfCursor) list() uint64 { return c.takeHead(4, false).value }

func (c *rdfCursor) body(major byte) []byte {
	length := c.takeHead(major, false).value
	c.check(length <= uint64(len(c.data)-c.pos), WrongLength)
	if c.reason != "" {
		return nil
	}
	start := c.pos
	c.pos += int(length)
	return c.data[start:c.pos]
}

func (c *rdfCursor) text() string {
	body := c.body(3)
	c.check(utf8.Valid(body), WrongType)
	return string(body)
}

func (c *rdfCursor) end() Reason {
	c.check(c.pos == len(c.data), WrongLength)
	return c.reason
}

// Read the list incrementally, then compare its length against other fields.
// A claimed count never causes a large reservation before any entries parse.
func rdfColumn[T any](c *rdfCursor, read func() T) []T {
	length := c.list()
	column := make([]T, 0, int(min(length, rdfDecodeBatch)))
	for index := uint64(0); index < length && c.reason == ""; index++ {
		column = append(column, read())
	}
	return column
}

func (c *rdfCursor) delta(previous uint64) uint64 {
	h := c.takeHead(0, true)
	if h.major == 1 {
		// A CBOR negative integer is -1 minus its argument.
		if h.value < previous {
			return previous - h.value - 1
		}
	} else if h.value <= math.MaxUint64-previous {
		return previous + h.value
	}
	c.check(false, OutOfRange)
	return 0
}
