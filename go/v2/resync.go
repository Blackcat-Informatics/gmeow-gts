// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import "bytes"

const resyncPrefixMax = 104

// ResyncRefusal records the first scanned opener refused by the read budget.
type ResyncRefusal struct {
	Position int
	Offset   int
	Length   int
	Count    uint64
}

type resyncScanner struct {
	data       []byte
	scannedEnd int
}

func (s *resyncScanner) scan(start, end int, budget *payloadBudget) (*Segment, *ResyncRefusal) {
	if budget.exhausted {
		return nil, nil
	}
	for at := max(start, s.scannedEnd); at < end; {
		index := bytes.IndexByte(s.data[at:end], 0x8b)
		if index < 0 {
			break
		}
		offset := at + index
		at = offset + 1
		if !resyncClaimsPlace(s.data[offset:offset+min(len(s.data)-offset, resyncPrefixMax)], offset) {
			continue
		}
		if reason, count := budget.charge(resyncMax); reason != "" {
			return nil, &ResyncRefusal{Offset: offset, Length: resyncMax, Count: count}
		}
		source := s.data[offset : offset+min(len(s.data)-offset, resyncMax)]
		size, reason := itemEnd(source, 0)
		if reason != "" {
			continue
		}
		opener, reason := readOpener(source[:size], offset, true)
		if reason == "" && opener.ID != nil && bytes.Equal(opener.ID[:], source[size-32:size]) {
			s.scannedEnd = at
			return &Segment{
				Opener: opener, Head: opener.ID, End: offset + size, Scanned: true,
				Items: []Item{{Kind: ResyncItem, Offset: offset, End: offset + size}},
			}, nil
		}
	}
	s.scannedEnd = max(s.scannedEnd, end)
	return nil, nil
}

type boundary struct {
	offset, end int
	reason      Reason
}

type itemWalk struct {
	data   []byte
	offset int
	count  uint64
	limit  uint64
	held   []boundary
}

func (w *itemWalk) delimit(limit int) (boundary, bool) {
	offset := w.offset
	if offset >= limit || w.count == w.limit {
		return boundary{}, false
	}
	end, reason := itemEnd(w.data[:limit], offset)
	if reason == TornTail && limit < len(w.data) {
		return boundary{}, false
	}
	if reason != "" {
		end = len(w.data)
	}
	if reason != TornTail {
		w.count++
	}
	w.offset = end
	return boundary{offset, end, reason}, true
}

func (w *itemWalk) take() (boundary, bool) {
	if len(w.held) != 0 {
		first := w.held[0]
		w.held = w.held[1:]
		return first, true
	}
	return w.delimit(len(w.data))
}

func (w *itemWalk) peek() (boundary, bool) {
	if len(w.held) == 0 {
		next, ok := w.delimit(len(w.data))
		if !ok {
			return boundary{}, false
		}
		w.held = append(w.held, next)
	}
	return w.held[0], true
}

func (w *itemWalk) recover(s *resyncScanner, start int, failed boundary, budget *payloadBudget) (*Segment, *ResyncRefusal) {
	if failed.reason == "" {
		w.peek()
		limit := failed.end + min(len(w.data)-failed.end, resyncMax)
		for next, ok := w.delimit(limit); ok; next, ok = w.delimit(limit) {
			w.held = append(w.held, next)
		}
	}
	if w.count == w.limit {
		return nil, nil
	}
	segment, refusal := s.scan(start, failed.end, budget)
	if segment != nil {
		w.count++
		w.offset = segment.End
		w.held = w.held[:0]
	}
	return segment, refusal
}

func resyncClaimsPlace(prefix []byte, offset int) bool {
	if len(prefix) < 20 || !bytes.HasPrefix(prefix, []byte{0x8b, 5, 2, 0x50}) ||
		!bytes.Equal(prefix[4:20], resyncMarker[:]) {
		return false
	}
	// Walk just the five fields through the claim of place before charging.
	end := 20
	for range 5 {
		next, reason := itemEnd(prefix, end)
		if reason != "" {
			return false
		}
		end = next
	}
	values := itemValues{remaining: prefix[20:end]}
	var opener Opener
	digestLen, reason := readOpenerMetadata(&opener, &values, end, true)
	if reason != "" || digestLen != 32 {
		return false
	}
	claimed, reason := uintValue(values.remaining)
	return reason == "" && claimed == uint64(offset)
}
