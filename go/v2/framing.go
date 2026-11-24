// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import "bytes"

// A CBOR boundary is determined independently of the GTS typed grammar.
type head struct {
	major      byte
	value      uint64
	end        int
	indefinite bool
	shortest   bool
}

func readHead(data []byte, offset int) (head, Reason) {
	if offset == len(data) {
		return head{}, TornTail
	}
	first := data[offset]
	info := first & 31
	h := head{major: first >> 5, end: offset + 1}
	switch {
	case info < 24:
		h.value, h.shortest = uint64(info), true
	case info <= 27:
		width := 1 << (info - 24)
		if width > len(data)-h.end {
			return head{}, TornTail
		}
		for _, octet := range data[h.end : h.end+width] {
			h.value = h.value<<8 | uint64(octet)
		}
		h.end += width
		if h.major == 7 && info == 24 && h.value < 32 {
			return head{}, UndelimitableInput
		}
		h.shortest = h.value >= [...]uint64{24, 256, 65536, 1 << 32}[info-24]
	case info == 31 && h.major >= 2 && h.major != 6:
		h.indefinite = true
	default:
		return head{}, UndelimitableInput
	}
	return h, ""
}

type container struct {
	major      byte
	count      uint64
	indefinite bool
	needsValue bool
}

// itemEnd walks heads iteratively. Map counts track pairs to avoid doubling a
// count that may use the entire uint64 range.
func itemEnd(data []byte, offset int) (int, Reason) {
	open := []container{{count: 1}}
	for len(open) != 0 {
		parent := &open[len(open)-1]
		if !parent.indefinite && parent.count == 0 {
			open = open[:len(open)-1]
			continue
		}
		if offset == len(data) {
			return 0, TornTail
		}
		first := data[offset]
		if first == 0xff {
			if !parent.indefinite || parent.needsValue {
				return 0, UndelimitableInput
			}
			open = open[:len(open)-1]
			offset++
			continue
		}
		if parent.indefinite && (parent.major == 2 || parent.major == 3) &&
			(first>>5 != parent.major || first&31 == 31) {
			return 0, UndelimitableInput
		}
		h, reason := readHead(data, offset)
		if reason != "" {
			return 0, reason
		}
		offset = h.end
		if parent.major == 5 {
			parent.needsValue = !parent.needsValue
		}
		if !parent.indefinite && !parent.needsValue {
			parent.count--
		}
		switch {
		case h.indefinite:
			open = append(open, container{major: h.major, indefinite: true})
		case h.major == 2 || h.major == 3:
			if h.value > uint64(len(data)-offset) {
				return 0, TornTail
			}
			offset += int(h.value)
		case h.major == 4 || h.major == 5:
			open = append(open, container{major: h.major, count: h.value})
		case h.major == 6:
			open = append(open, container{count: 1})
		}
	}
	return offset, ""
}

// versionKey compares the decoded text with "v" without concatenating chunks.
func versionKey(item []byte) bool {
	h, _ := readHead(item, 0)
	if h.major != 3 {
		return false
	}
	if !h.indefinite {
		return bytes.Equal(item[h.end:], []byte("v"))
	}
	found := false
	for at := h.end; item[at] != 0xff; {
		chunk, _ := readHead(item, at)
		if chunk.value != 0 {
			if found || chunk.value != 1 || item[chunk.end] != 'v' {
				return false
			}
			found = true
		}
		at = chunk.end + int(chunk.value)
	}
	return found
}

func generation(item []byte) (grammar Grammar, version uint64) {
	h, _ := readHead(item, 0)
	if h.major == 4 && (h.indefinite || h.value >= 2) {
		kind, reason := readHead(item, h.end)
		if reason == "" && kind.major == 0 && (kind.value == 1 || kind.value == 5) {
			versionHead, reason := readHead(item, kind.end)
			if reason == "" && versionHead.major == 0 {
				return RecordGrammar, versionHead.value
			}
		}
	}
	if h.major == 6 && h.value == 55799 {
		h, _ = readHead(item, h.end)
	}
	if h.major != 5 {
		return "", 0
	}
	end := len(item)
	if h.indefinite {
		end--
	}
	for at := h.end; at < end; {
		value, _ := itemEnd(item, at)
		if versionKey(item[at:value]) {
			v, _ := readHead(item, value)
			if v.major == 0 {
				return LegacyGrammar, v.value
			}
			return "", 0
		}
		at, _ = itemEnd(item, value)
	}
	return "", 0
}
