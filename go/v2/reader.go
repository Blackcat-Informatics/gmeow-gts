// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Package v2 reads GTS v2 item boundaries, segment openers, frame links and
// payloads. Its wire observations are inputs to a content and recovery reader.
package v2

// Reason is a wire finding reason or the reason forward framing stopped.
type Reason string

// Wire diagnostics use the spellings in the specification.
const (
	TornTail             Reason = "torn_tail"
	UndelimitableInput   Reason = "undelimitable_input"
	ItemLimit            Reason = "items_per_read"
	NotDeterministic     Reason = "not_deterministic"
	WrongType            Reason = "wrong_type"
	WrongLength          Reason = "wrong_length"
	OutOfRange           Reason = "out_of_range"
	UnregisteredValue    Reason = "unregistered_value"
	ForbiddenCombination Reason = "forbidden_combination"
	UnorderedOrRepeated  Reason = "unordered_or_repeated"
	UnknownItemKind      Reason = "unknown_item_kind"
	StoredSize           Reason = "stored_size"
	DecodedSize          Reason = "decoded_size"
	TotalDecoded         Reason = "total_decoded"
	UnknownCodec         Reason = "unknown_codec"
	CodecConstraint      Reason = "codec_constraint"
	DecodeFailed         Reason = "decode_failed"
	DecodedLength        Reason = "decoded_length"
	PayloadDigest        Reason = "payload_digest"
)

// Grammar identifies the grammar that supplied the first item's generation.
type Grammar string

// The two generation field grammars.
const (
	RecordGrammar Grammar = "record"
	LegacyGrammar Grammar = "first_generation"
)

// Dispatch describes the generation decision made before reading v2 segments.
type Dispatch string

// Generation dispatch outcomes.
const (
	Empty                 Dispatch = "empty_input"
	FirstGeneration       Dispatch = "first_generation"
	SecondGeneration      Dispatch = "second_generation"
	UnsupportedGeneration Dispatch = "unsupported_generation"
	NotThisFormat         Dispatch = "not_this_format"
)

// Kind classifies a complete item within a v2 segment.
type Kind string

// Item kinds, including observations for unknown or unchecked items.
const (
	HeaderItem    Kind = "header"
	ResyncItem    Kind = "resync"
	FrameItem     Kind = "frame"
	OffsetsItem   Kind = "offsets"
	LocatorItem   Kind = "locator"
	OpaqueItem    Kind = "opaque"
	UncheckedItem Kind = "unchecked"
)

// Item describes one outer CBOR item. Frame is populated for frame items in a
// readable segment.
type Item struct {
	Offset int
	End    int
	Kind   Kind
	Reason Reason
	Frame  *Frame
}

// Segment begins with one opener, followed by all its delimited items in
// wire order. Head is nil when the opener could not supply an identifier.
type Segment struct {
	Opener Opener
	Items  []Item
	Head   *Digest
}

// WireResult holds the generation decision and observations for the delimited
// prefix. Dispatch can be empty if the first item could not be delimited.
type WireResult struct {
	Dispatch   Dispatch
	Grammar    Grammar
	Generation uint64
	Segments   []Segment
	Stop       Reason
	StopOffset int
}

// WireInspection holds wire observations without retaining decoded payloads.
type WireInspection WireResult

// ReadWire walks a byte input through the first incomplete or undelimitable
// item, or the item limit. It checks frame links and stored and decoded payloads
// in that prefix. The input buffer must remain unchanged while the result is
// in use because encoded content and plain payloads borrow slices of it.
//
// ReadWire reports a first generation input for the legacy reader to process.
// Content folding, signature verification, offset claims, and recovery after
// framing loss belong to layers above this function.
func ReadWire(data []byte, capacity ReadCapacity) WireResult {
	return readWire(data, capacity, true)
}

// InspectWire performs the same wire checks as ReadWire and releases each
// decoded payload after verification. Its result carries the payload statuses
// and encoded frame content, which borrows the input buffer.
func InspectWire(data []byte, capacity ReadCapacity) WireInspection {
	return WireInspection(readWire(data, capacity, false))
}

func readWire(data []byte, capacity ReadCapacity, retainPayload bool) WireResult {
	var result WireResult
	if len(data) == 0 {
		result.Dispatch = Empty
		return result
	}

	var count uint64
	for offset := 0; offset < len(data); {
		if count == capacity.ItemsPerRead {
			result.Stop, result.StopOffset = ItemLimit, offset
			break
		}
		end, reason := itemEnd(data, offset)
		if reason != "" {
			result.Stop, result.StopOffset = reason, offset
			break
		}
		body := data[offset:end]
		if offset == 0 && !result.dispatch(body) {
			return result
		}

		// Opener recognition by value applies even in an unreadable segment.
		var segment *Segment
		if length := len(result.Segments); length != 0 {
			segment = &result.Segments[length-1]
		}
		item := classifyItem(body, segment)
		item.Offset, item.End = offset, end
		if item.Kind == HeaderItem || item.Kind == ResyncItem {
			opener, reason := readOpener(body, offset, item.Kind == ResyncItem)
			item.Reason = reason
			result.Segments = append(result.Segments, Segment{
				Opener: opener, Head: opener.ID, Items: []Item{item},
			})
		} else if segment != nil {
			segment.Items = append(segment.Items, item)
		}

		offset = end
		count++
	}

	budget := payloadBudget{capacity: capacity}
	for i := range result.Segments {
		result.Segments[i].checkFrames(data, &budget, retainPayload)
	}
	return result
}

func (result *WireResult) dispatch(item []byte) bool {
	result.Grammar, result.Generation = generation(item)
	switch {
	case result.Grammar == RecordGrammar && result.Generation == 2:
		result.Dispatch = SecondGeneration
		return true
	case result.Grammar == LegacyGrammar && result.Generation == 1:
		result.Dispatch = FirstGeneration
	case result.Grammar == LegacyGrammar || (result.Grammar == RecordGrammar && result.Generation > 2):
		result.Dispatch = UnsupportedGeneration
	default:
		result.Dispatch = NotThisFormat
	}
	return false
}

func classifyItem(body []byte, segment *Segment) Item {
	h, _ := readHead(body, 0)
	var kind head
	if h.major == 4 && (h.indefinite || h.value != 0) {
		kind, _ = readHead(body, h.end)
		if kind.major == 0 {
			switch kind.value {
			case 1:
				return Item{Kind: HeaderItem}
			case 5:
				return Item{Kind: ResyncItem}
			}
		}
	}
	if segment == nil || segment.Head == nil {
		return Item{Kind: UncheckedItem}
	}

	item := Item{Kind: OpaqueItem}
	switch {
	case h.major != 4 || (!h.indefinite && h.value == 0) || kind.major != 0:
		item.Reason = WrongType
	case !h.shortest || !kind.shortest:
		item.Reason = NotDeterministic
	case kind.value == 2 && h.value == 11:
		item.Kind = FrameItem
	case kind.value == 2:
		item.Reason = WrongLength
	case kind.value == 3:
		item.Kind = LocatorItem
	case kind.value == 4:
		item.Kind = OffsetsItem
	case kind.value != 0 && segment.Opener.Registry > 1:
		item.Reason = UnknownItemKind
	default:
		item.Reason = UnregisteredValue
	}
	return item
}

func (s *Segment) checkFrames(data []byte, budget *payloadBudget, retainPayload bool) {
	if s.Head == nil {
		return
	}
	newer := s.Opener.Registry > 1
	var witness *frameFields
	for i := 1; i < len(s.Items); i++ {
		item := &s.Items[i]
		if item.Kind != FrameItem {
			continue
		}

		fields := witness
		witness = nil
		if fields == nil {
			f := decodeFrame(data[item.Offset:item.End], newer)
			fields = &f
		}
		if fields.identity != nil && fields.StoredID != nil && !fields.Transparent &&
			i+1 < len(s.Items) && s.Items[i+1].Kind == FrameItem {
			// Use the next envelope, including a transparent frame, as the
			// only possible witness. Reuse its parsed fields on the next step.
			next := s.Items[i+1]
			f := decodeFrame(data[next.Offset:next.End], newer)
			witness = &f
		}
		fields.checkLink(*s.Head, witness)
		if fields.EffectiveID != nil {
			s.Head = fields.EffectiveID
		}
		fields.Payload = budget.check(fields)
		if !retainPayload {
			fields.Payload.Decoded = nil
		}
		fields.contentWithinCapacity = uint64(len(fields.Content)) <= budget.capacity.StoredBytesPerFrame
		frame := fields.Frame
		item.Frame = &frame
	}
}
