// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import "bytes"

const resyncMax = 4096

// Opener contains the wire fields shared by a header and a resync item.
// Zero Profile and Canonical values represent absent options.
type Opener struct {
	Registry      uint64
	HashAlgorithm uint64
	Profile       uint64
	Canonical     uint64
	Lineage       *Lineage
	Resync        *Resync
	// ID is nil when the hash algorithm is unavailable or the opener is malformed.
	ID *Digest
}

// Lineage is an opener's claim about the preceding segment.
type Lineage struct {
	PreviousHead []byte
	Ordinal      uint64
}

// Resync holds the wire fields unique to a resync opener.
type Resync struct {
	Offset    uint64
	Past      []byte
	Marks     []byte
	Displaced bool
}

var resyncMarker = [16]byte{0xfc, 0xf3, 0xeb, 0x32, 0xa2, 0xf0, 0x26, 0x7d, 0xd1, 0x6a, 0x53, 0xf0, 0x9b, 0xff, 0x87, 0x92}

func readOpener(item []byte, offset int, resync bool) (Opener, Reason) {
	var o Opener
	var length uint64 = 7
	if resync {
		length = 11
	}
	values, reason := array(item, length)
	if reason != "" {
		return o, reason
	}
	if _, reason := uintValue(values.next()); reason != "" {
		return o, reason
	}
	generation, reason := uintValue(values.next())
	if reason != "" {
		return o, reason
	}
	if generation != 2 {
		return o, OutOfRange
	}
	if resync {
		marker, reason := byteValue(values.next(), -1)
		if reason != "" {
			return o, reason
		}
		if !bytes.Equal(marker, resyncMarker[:]) {
			return o, OutOfRange
		}
	}
	digestLen, reason := readOpenerMetadata(&o, &values, len(item), resync)
	if reason != "" {
		return o, reason
	}
	if resync {
		o.Resync, reason = readResync(&values, len(item), offset, digestLen)
		if reason != "" {
			return o, reason
		}
	}
	if digestLen != -1 {
		var id Digest
		if resync {
			id = hash("gts2 header", []byte{0x8a}, item[1:len(item)-len(values.remaining)])
		} else {
			id = hash("gts2 header", item)
		}
		o.ID = &id
	}
	return o, ""
}

func readOpenerMetadata(o *Opener, values *itemValues, size int, resync bool) (int, Reason) {
	var reason Reason
	o.Registry, reason = uintValue(values.next())
	if reason != "" {
		return 0, reason
	}
	if o.Registry == 0 {
		return 0, OutOfRange
	}
	newer := o.Registry > 1
	o.HashAlgorithm, reason = uintValue(values.next())
	if reason != "" {
		return 0, reason
	}
	if o.HashAlgorithm == 0 || (!newer && o.HashAlgorithm != 1) {
		return 0, UnregisteredValue
	}
	digestLen := -1
	if o.HashAlgorithm == 1 {
		digestLen = 32
	}
	o.Profile, reason = optionEnum(values.next(), 0, true)
	if reason != "" {
		return 0, reason
	}
	if !resync {
		o.Canonical, reason = optionEnum(values.next(), 1, newer)
		if reason != "" {
			return 0, reason
		}
	}
	lineage := values.next()
	lineageLimit := len(lineage)
	if resync {
		lineageLimit = resyncMax - (size - len(values.remaining) - len(lineage))
	}
	o.Lineage, reason = readLineage(lineage, digestLen, lineageLimit)
	return digestLen, reason
}

func readLineage(item []byte, digestLen, limit int) (*Lineage, Reason) {
	value, reason := optionBefore(item, limit)
	if value == nil || reason != "" {
		return nil, reason
	}
	fields, reason := array(value, 2)
	if reason != "" {
		return nil, reason
	}
	if len(item)-len(fields.remaining) >= limit {
		return nil, OutOfRange
	}
	previousHead, reason := byteValue(fields.next(), digestLen)
	if reason != "" {
		return nil, reason
	}
	if len(item)-len(fields.remaining) >= limit {
		return nil, OutOfRange
	}
	ordinal, reason := uintValue(fields.next())
	if reason != "" {
		return nil, reason
	}
	if ordinal == 0 {
		return nil, OutOfRange
	}
	return &Lineage{PreviousHead: previousHead, Ordinal: ordinal}, ""
}

func readResync(values *itemValues, size, offset, digestLen int) (*Resync, Reason) {
	r := &Resync{}
	var reason Reason
	if size-len(values.remaining) >= resyncMax {
		return nil, OutOfRange
	}
	r.Offset, reason = uintValue(values.next())
	if reason != "" {
		return nil, reason
	}
	r.Displaced = r.Offset != uint64(offset)
	pastLimit := resyncMax - (size - len(values.remaining))
	if pastLimit <= 0 {
		return nil, OutOfRange
	}
	past, reason := optionBefore(values.next(), pastLimit)
	if reason != "" {
		return nil, reason
	}
	if past != nil {
		r.Past, reason = byteValue(past, digestLen)
		if reason != "" {
			return nil, reason
		}
	}
	marksOffset := size - len(values.remaining)
	if marksOffset >= resyncMax {
		return nil, OutOfRange
	}
	r.Marks = values.next()
	if reason := checkMarks(r.Marks, resyncMax-marksOffset); reason != "" {
		return nil, reason
	}
	if size-len(values.remaining) >= resyncMax {
		return nil, OutOfRange
	}
	if _, reason := byteValue(values.remaining, digestLen); reason != "" {
		return nil, reason
	}
	if size > resyncMax {
		return nil, OutOfRange
	}
	return r, ""
}

func checkMarks(encoded []byte, limit int) Reason {
	marks, reason := typedHead(encoded, 4)
	if reason != "" {
		return reason
	}
	list := itemValues{remaining: encoded[marks.end:]}
	var previous []byte
	for i := 0; len(list.remaining) != 0; i++ {
		remaining := limit - (len(encoded) - len(list.remaining))
		if remaining <= 0 {
			return OutOfRange
		}
		item := list.next()
		h, reason := typedHead(item, 4)
		if reason != "" {
			return reason
		}
		if h.value == 0 {
			return WrongType
		}
		first, _ := readHead(item, h.end)
		if first.major != 0 {
			return WrongType
		}
		if i == 64 {
			return OutOfRange
		}
		if previous != nil && bytes.Compare(previous, item) >= 0 {
			return UnorderedOrRepeated
		}
		if reason := grammarBefore(item, remaining); reason != "" {
			return reason
		}
		previous = item
	}
	return ""
}
