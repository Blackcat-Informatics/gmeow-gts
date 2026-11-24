// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"math"
	"strconv"
	"strings"
	"time"

	"go.blackcatinformatics.ca/gts/model"
)

const (
	rdfValueBool = iota + 1
	rdfValueUint
	rdfValueInt
	rdfValueTime
	rdfValueOctets
)

const rdfValueDatatype = "http://www.w3.org/2001/XMLSchema#"

// Inline values are checked with the other payload fields. Their lexical
// forms are generated only after the frame passes the counts and prefix budget.
type rdfInlineValue struct {
	argument    uint64
	octets      []byte
	nanoseconds uint32
	kind        uint8
	negative    bool
}

func (p *rdfProjector) addInlineValues(dictionary *rdfReadDictionary, block rdfBlock) Reason {
	c := rdfCursor{data: block.values}
	for range block.valueCount {
		value := c.inlineValue()
		if c.reason != "" {
			return c.reason
		}
		lexical, datatype, reason := value.literal()
		if reason != "" {
			return reason
		}
		datatypeID := p.intern(rdfTerm{kind: model.Iri, text: datatype})
		id := p.intern(rdfTerm{kind: model.Literal, text: lexical, form: 1, datatype: datatypeID})
		dictionary.append(rdfWrittenKey{kind: block.kind, rdf: id})
	}
	return c.end()
}

func (c *rdfCursor) inlineValue() rdfInlineValue {
	length := c.list()
	c.check(length > 0, WrongLength)
	value := rdfInlineValue{kind: uint8(c.code(rdfValueOctets))}
	if value.kind == rdfValueTime {
		c.check(length == 3, WrongLength)
	} else {
		c.check(length == 2, WrongLength)
	}
	if c.reason != "" {
		return value
	}
	switch value.kind {
	case rdfValueBool:
		start := c.pos
		h := c.takeHead(7, false)
		if c.reason == "" {
			octet := c.data[start]
			c.check(octet == 0xf4 || octet == 0xf5, WrongType)
			value.argument = h.value
		}
	case rdfValueUint:
		value.argument = c.uint()
	case rdfValueInt, rdfValueTime:
		h := c.takeHead(0, true)
		value.negative, value.argument = h.major == 1, h.value
		if value.kind == rdfValueTime {
			maxArgument := uint64(253402300799)
			if value.negative {
				maxArgument = 62135596799
			}
			c.check(value.argument <= maxArgument, OutOfRange)
			nanoseconds := c.uint()
			c.check(nanoseconds < 1_000_000_000, OutOfRange)
			value.nanoseconds = uint32(nanoseconds)
		}
	case rdfValueOctets:
		value.octets = c.body(2)
		c.check(len(value.octets) <= math.MaxInt/2, OutOfRange)
	}
	return value
}

func (v rdfInlineValue) literal() (lexical, datatype string, reason Reason) {
	switch v.kind {
	case rdfValueBool:
		return strconv.FormatBool(v.argument == 21), rdfValueDatatype + "boolean", ""
	case rdfValueUint:
		return strconv.FormatUint(v.argument, 10), rdfValueDatatype + "nonNegativeInteger", ""
	case rdfValueInt:
		return v.integerText(), rdfValueDatatype + "integer", ""
	case rdfValueTime:
		seconds := int64(v.argument)
		if v.negative {
			seconds = -1 - seconds
		}
		instant := time.Unix(seconds, int64(v.nanoseconds)).UTC()
		return instant.Format(time.RFC3339Nano), rdfValueDatatype + "dateTime", ""
	case rdfValueOctets:
		const digits = "0123456789ABCDEF"
		var text strings.Builder
		text.Grow(len(v.octets) * 2)
		for _, octet := range v.octets {
			text.WriteByte(digits[octet>>4])
			text.WriteByte(digits[octet&15])
		}
		return text.String(), rdfValueDatatype + "hexBinary", ""
	default:
		return "", "", UnregisteredValue
	}
}

func (v rdfInlineValue) integerText() string {
	if !v.negative {
		return strconv.FormatUint(v.argument, 10)
	}
	if v.argument == math.MaxUint64 {
		return "-18446744073709551616"
	}
	return "-" + strconv.FormatUint(v.argument+1, 10)
}
