// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"math"
	"strings"
	"unicode/utf8"
)

const (
	chunkFrameType  = 8
	blobFrameType   = 9
	recipeFrameType = 10
	valueFrameType  = 11
	packFrameType   = 13

	unitContext          = "gts2 unit"
	totalRebuilt         = Reason("total_rebuilt")
	illFormedTerm        = Reason("ill_formed_term")
	projectionValueLimit = Reason("projection_value_limit")
	rdfRebuiltCapacity   = 1 << 37
)

type rebuildRef struct {
	digest Digest
	length uint64
}

type rebuildRecord struct {
	kind    uint64
	target  rebuildRef
	count   uint64
	entries []rebuildRef
}

func (c *rdfCursor) digest() Digest {
	raw := c.body(2)
	var digest Digest
	c.check(len(raw) == len(digest), WrongLength)
	copy(digest[:], raw)
	return digest
}

func (c *rdfCursor) discardTextOption() {
	size := c.list()
	c.check(size > 0, WrongLength)
	tag := c.code(2)
	c.check(size == tag, WrongLength)
	if tag == 2 {
		c.check(utf8.Valid(c.body(3)), WrongType)
	}
}

func (c *rdfCursor) rebuildRecord(kind uint64) rebuildRecord {
	switch kind {
	case chunkFrameType:
		c.record(1)
	case blobFrameType:
		c.record(2)
	case recipeFrameType:
		c.record(4)
	case valueFrameType:
		c.record(3)
	}
	record := rebuildRecord{kind: kind}
	record.target.digest = c.digest()
	if kind >= recipeFrameType {
		record.target.length = c.uint()
	}
	if kind == blobFrameType || kind == recipeFrameType {
		c.discardTextOption()
	}
	if kind >= recipeFrameType {
		record.count = c.uint()
	}
	return record
}

func (r *rebuildRecord) readEntries(body []byte) Reason {
	if r.count == 0 || r.kind == recipeFrameType && r.count < 2 {
		return OutOfRange
	}
	c := rdfCursor{data: body}
	r.entries = rdfColumn(&c, func() rebuildRef {
		c.record(2)
		return rebuildRef{digest: c.digest(), length: c.uint()}
	})
	if reason := c.end(); reason != "" {
		return reason
	}
	if uint64(len(r.entries)) != r.count {
		return inconsistentCounts
	}
	var total uint64
	for _, entry := range r.entries {
		if entry.length == 0 || entry.length > math.MaxUint64-total {
			return OutOfRange
		}
		total += entry.length
	}
	if total != r.target.length {
		return inconsistentCounts
	}
	return ""
}

type rebuiltValue struct {
	parts     [][]byte
	reason    Reason
	text      string
	hex       string
	validText bool
}

func (v *rebuiltValue) lexical(length uint64, octets bool, remaining *uint64) (string, Reason) {
	value := &v.text
	bytesPerOctet := uint64(1)
	if octets {
		value = &v.hex
		bytesPerOctet = 2
	}
	if *value != "" {
		return *value, ""
	}
	if length > min(*remaining, math.MaxInt)/bytesPerOctet {
		return "", projectionValueLimit
	}
	size := length * bytesPerOctet
	var text strings.Builder
	text.Grow(int(size))
	for _, part := range v.parts {
		if octets {
			writeRDFHex(&text, part)
		} else {
			_, _ = text.Write(part)
		}
	}
	*value = text.String()
	*remaining -= size
	return *value, ""
}

// The unit index is reset at every header. Value candidates and the rebuild
// budget span the whole file, in item order.
type rdfRebuild struct {
	units   map[Digest][]byte
	values  map[rebuildRef]*rebuiltValue
	charged uint64
	refused bool
}

func (r *rdfRebuild) unit(digest Digest, body []byte) {
	if hash(unitContext, body) != digest {
		return
	}
	if r.units == nil {
		r.units = make(map[Digest][]byte)
	}
	if _, exists := r.units[digest]; !exists {
		r.units[digest] = body
	}
}

func (r *rdfRebuild) declare(record rebuildRecord) {
	admitted := !r.refused && record.target.length <= rdfRebuiltCapacity-r.charged
	if admitted {
		r.charged += record.target.length
	} else {
		r.refused = true
	}
	// Recipe declarations consume capacity. Only values feed RDF terms.
	if record.kind != valueFrameType {
		return
	}
	reason := totalRebuilt
	var parts [][]byte
	if admitted {
		parts, reason = r.rebuild(record)
	}
	current := r.values[record.target]
	if current == nil || current.reason != "" && reason == "" {
		if r.values == nil {
			r.values = make(map[rebuildRef]*rebuiltValue)
		}
		r.values[record.target] = &rebuiltValue{parts: parts, reason: reason}
	}
}

func (r *rdfRebuild) rebuild(record rebuildRecord) ([][]byte, Reason) {
	parts := make([][]byte, 0, min(len(record.entries), rdfDecodeBatch))
	for _, entry := range record.entries {
		unit, exists := r.units[entry.digest]
		if !exists {
			return nil, "chunk"
		}
		if uint64(len(unit)) != entry.length {
			return nil, "blob_length"
		}
		parts = append(parts, unit)
	}
	if hash(unitContext, parts...) != record.target.digest {
		return nil, "blob_digest"
	}
	return parts, ""
}

func (r *rdfRebuild) frame(frame *Frame) Reason {
	payload := frame.Payload.Decoded
	if frame.Type == packFrameType {
		return r.pack(frame.Content, payload)
	}
	c := rdfCursor{data: frame.Content}
	record := c.rebuildRecord(frame.Type)
	if reason := c.end(); reason != "" {
		return reason
	}
	if record.kind <= blobFrameType {
		r.unit(record.target.digest, payload)
		return ""
	}
	if reason := record.readEntries(payload); reason != "" {
		return reason
	}
	r.declare(record)
	return ""
}
