// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

const (
	// IndexOffsets identifies an offsets table that disagrees with the item walk.
	IndexOffsets Reason = "index_offsets"
	// IndexLocator identifies a locator that does not name the last typed index.
	IndexLocator Reason = "index_locator"
)

func (s *Segment) checkClosing(data []byte) {
	lastIndex := -1
	var offsetsSeen, locatorSeen bool
	for i := range s.Items {
		item := &s.Items[i]
		final := i == len(s.Items)-1
		switch item.Kind {
		case FrameItem:
			if frame := item.Frame; frame != nil && frame.Type == indexType {
				lastIndex = item.Offset
			}
		case OffsetsItem:
			finalLocator := i == len(s.Items)-2 && s.Items[i+1].Kind == LocatorItem
			if offsetsSeen || locatorSeen || (!final && !finalLocator) {
				item.Reason = UnorderedOrRepeated
			} else {
				var holds bool
				holds, item.Reason = checkOffsets(data[item.Offset:item.End], s.Items[:i])
				if item.Reason == "" && !holds {
					item.ClosingClaimFailure = IndexOffsets
				}
			}
			offsetsSeen = true
		case LocatorItem:
			if locatorSeen || !final {
				item.Reason = UnorderedOrRepeated
			} else {
				var distance uint64
				distance, item.Reason = locatorDistance(data[item.Offset:item.End])
				if item.Reason == "" && (lastIndex < 0 || distance != uint64(item.Offset-lastIndex)) {
					item.ClosingClaimFailure = IndexLocator
				}
			}
			locatorSeen = true
		}
	}
}

func checkOffsets(body []byte, preceding []Item) (bool, Reason) {
	fields, reason := array(body, 2)
	if reason != "" {
		return false, reason
	}
	fields.next() // The classifier checked the item kind.
	list := fields.next()
	h, reason := typedHead(list, 4)
	if reason != "" {
		return false, reason
	}
	lengths := itemValues{remaining: list[h.end:]}
	matches := h.value == uint64(len(preceding))
	for ordinal := 0; len(lengths.remaining) != 0; ordinal++ {
		claimed, reason := uintValue(lengths.next())
		if reason != "" {
			return false, reason
		}
		if matches && claimed != uint64(preceding[ordinal].End-preceding[ordinal].Offset) {
			matches = false
		}
	}
	return matches, ""
}

func locatorDistance(body []byte) (uint64, Reason) {
	fields, reason := array(body, 2)
	if reason != "" {
		return 0, reason
	}
	fields.next()
	raw := fields.next()
	h, _ := readHead(raw, 0)
	if h.major != 0 {
		_, reason := typedHead(raw, 0)
		return 0, reason
	}
	// The locator is the exception to the shortest unsigned head rule.
	if raw[0] != 0x1b {
		return 0, NotDeterministic
	}
	return h.value, ""
}
