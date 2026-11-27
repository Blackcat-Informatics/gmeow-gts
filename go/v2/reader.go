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
	DamagedRegion Kind = "damaged_region"
)

// Item describes one outer CBOR item. Frame is populated for frame items in a
// readable segment.
type Item struct {
	Offset int
	End    int
	Kind   Kind
	Reason Reason
	Frame  *Frame
	// ClosingClaimFailure is separate from malformed item grammar or placement.
	ClosingClaimFailure Reason
}

// Segment begins with one opener, followed by all its delimited items in
// wire order. Head is nil when the opener could not supply an identifier.
// End is the exclusive segment boundary. A complete item can reach past End
// when a scan finds the next opener inside that item.
type Segment struct {
	Opener        Opener
	Items         []Item
	Head          *Digest
	End           int
	Scanned       bool
	FramingGap    *FramingGap
	ResyncRefusal *ResyncRefusal
}

// FramingGap identifies the bytes between a framing failure and a recovered opener.
type FramingGap struct {
	Offset, Length int
	Reason         Reason
}

// WireResult holds the generation decision and observations from the wire walk.
// Dispatch can be empty if the first item could not be delimited.
type WireResult struct {
	Dispatch      Dispatch
	Grammar       Grammar
	Generation    uint64
	Segments      []Segment
	Stop          Reason
	StopOffset    int
	PrefixGap     *FramingGap
	ResyncRefusal *ResyncRefusal
}

// WireInspection holds wire observations without retaining decoded payloads.
type WireInspection WireResult

// ReadWire walks a byte input, scanning for valid resync openers when framing
// fails. It checks frame links, stored and decoded payloads, and offsets and
// locator claims. The input buffer must remain unchanged while the result is
// in use because encoded content and plain payloads borrow slices of it.
//
// ReadWire reports a first generation input for the legacy reader to process.
// Content folding, signature verification, and parity recovery belong to
// layers above this function.
func ReadWire(data []byte, capacity ReadCapacity, keys ...HeldKey) WireResult {
	return readWire(data, capacity, true, keys)
}

// InspectWire performs the same wire checks as ReadWire and releases each
// decoded payload after verification. Its result carries the payload statuses
// and encoded frame content, which borrows the input buffer.
func InspectWire(data []byte, capacity ReadCapacity, keys ...HeldKey) WireInspection {
	return WireInspection(readWire(data, capacity, false, keys))
}

func readWire(data []byte, capacity ReadCapacity, retainPayload bool, keys []HeldKey) WireResult {
	var result WireResult
	if len(data) == 0 {
		result.Dispatch = Empty
		return result
	}

	walk := itemWalk{data: data, limit: capacity.ItemsPerRead}
	scanner := resyncScanner{data: data}
	budget := payloadBudget{capacity: capacity}
	segment := result.firstSegment(&walk, &scanner, &budget)
	if segment == nil {
		return result
	}
	for current, ok := walk.take(); ok; current, ok = walk.take() {
		item, next, failed := segment.inspect(&walk, current, &budget, retainPayload, keys)
		if failed {
			last := &segment.Items[len(segment.Items)-1]
			start := last.Offset + 1
			if last.Frame != nil && last.Frame.Link == Healed {
				start = current.offset
			}
			recovered, refusal := walk.recover(&scanner, start, current, &budget)
			if refusal != nil {
				refusal.Position = len(segment.Items)
				if refusal.Offset < current.offset {
					refusal.Position--
				}
				segment.ResyncRefusal = refusal
			}
			if recovered != nil {
				segment.endAtResync(data, current.offset, recovered.Items[0].Offset)
				result.addSegment(data, segment)
				segment = recovered
				continue
			}
		}
		if current.reason != "" {
			result.Stop, result.StopOffset = current.reason, current.offset
			break
		}
		if next != nil {
			result.addSegment(data, segment)
			segment = next
			continue
		}
		segment.appendItem(item)
	}
	if result.Stop == "" && walk.offset < len(data) {
		result.Stop, result.StopOffset = ItemLimit, walk.offset
	}
	result.addSegment(data, segment)
	return result
}

func (result *WireResult) firstSegment(walk *itemWalk, scanner *resyncScanner, budget *payloadBudget) *Segment {
	first, ok := walk.take()
	if !ok {
		result.Stop = ItemLimit
		return nil
	}
	var segment *Segment
	switch {
	case first.reason != "":
		result.Stop = first.reason
	case result.dispatch(walk.data[:first.end]):
		item := classifyItem(walk.data[:first.end], true)
		item.End = first.end
		segment = openSegment(walk.data, item)
		if segment.Items[0].Reason == "" {
			return segment
		}
	case result.Dispatch != NotThisFormat:
		return nil
	}

	recovered, refusal := walk.recover(scanner, 0, first, budget)
	if recovered != nil {
		result.Dispatch, result.Grammar, result.Generation = SecondGeneration, RecordGrammar, 2
		result.Stop = ""
		result.PrefixGap = gapBefore(walk.data, 0, recovered.Items[0].Offset)
		return recovered
	}
	if segment != nil {
		segment.ResyncRefusal = refusal
	} else {
		result.ResyncRefusal = refusal
	}
	return segment
}

func (result *WireResult) addSegment(data []byte, s *Segment) {
	s.checkClosing(data)
	result.Segments = append(result.Segments, *s)
}

func openSegment(data []byte, item Item) *Segment {
	opener, reason := readOpener(data[item.Offset:item.End], item.Offset, item.Kind == ResyncItem)
	item.Reason = reason
	return &Segment{Opener: opener, Head: opener.ID, End: item.End, Items: []Item{item}}
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

func classifyItem(body []byte, newer bool) Item {
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
	case kind.value != 0 && newer:
		item.Reason = UnknownItemKind
	default:
		item.Reason = UnregisteredValue
	}
	return item
}

func (s *Segment) inspect(walk *itemWalk, current boundary, budget *payloadBudget, retainPayload bool, keys []HeldKey) (Item, *Segment, bool) {
	if current.reason != "" {
		return Item{}, nil, true
	}
	newer := s.Opener.Registry > 1
	body := walk.data[current.offset:current.end]
	item := classifyItem(body, newer || s.Opener.ID == nil)
	item.Offset, item.End = current.offset, current.end
	if item.Kind == HeaderItem || item.Kind == ResyncItem {
		next := openSegment(walk.data, item)
		return item, next, next.Items[0].Reason != ""
	}
	failed := item.Kind == OpaqueItem && item.Reason != UnknownItemKind
	if s.Opener.ID == nil {
		item.Kind, item.Reason = UncheckedItem, ""
		return item, nil, failed
	}
	switch item.Kind {
	case FrameItem:
		fields := decodeFrame(body, newer)
		fields.checkLink(*s.Head, nil)
		if !fields.Transparent && fields.Link == Broken && fields.ComputedID != nil && fields.StoredID != nil {
			if next, ok := walk.peek(); ok && next.reason == "" {
				witness := walk.data[next.offset:next.end]
				if len(witness) >= 2 && witness[0] == 0x8b && witness[1] == 2 {
					f := decodeFrame(witness, newer)
					fields.checkLink(*s.Head, &f)
				}
			}
		}
		fields.Payload = budget.check(&fields)
		if !retainPayload {
			fields.Payload.Decoded = nil
		}
		fields.contentWithinCapacity = uint64(len(fields.Content)) <= budget.capacity.StoredBytesPerFrame
		if !fields.Transparent && fields.EffectiveID != nil && uint64(fields.signaturesSize) <= budget.capacity.StoredBytesPerFrame {
			fields.Signatures = verifyFrame(fields.records, *s.Opener.ID, *fields.EffectiveID, keys)
		}
		item.Frame = &fields.Frame
		failed = fields.framingFailed || fields.Link == Broken
	case OffsetsItem:
		_, reason := checkOffsets(body, nil)
		failed = reason != ""
	case LocatorItem:
		_, reason := locatorDistance(body)
		failed = reason != ""
	}
	return item, nil, failed
}

func (s *Segment) appendItem(item Item) {
	s.End = item.End
	s.Items = append(s.Items, item)
	if item.Frame != nil && item.Frame.EffectiveID != nil {
		s.Head = item.Frame.EffectiveID
	}
}

func (s *Segment) endAtResync(data []byte, failedAt, resumedAt int) {
	if previous := s.Items[len(s.Items)-1].Frame; previous != nil && previous.Link == Healed {
		previous.Link, previous.EffectiveID = Broken, previous.StoredID
		s.Head = previous.StoredID
	}
	s.End = resumedAt
	s.FramingGap = gapBefore(data, failedAt, resumedAt)
	if s.FramingGap == nil {
		return
	}
	if s.FramingGap.Reason == TornTail {
		s.End = failedAt
	} else {
		s.Items = append(s.Items, Item{Kind: DamagedRegion, Offset: failedAt, End: resumedAt})
	}
}

func gapBefore(data []byte, start, end int) *FramingGap {
	if start < end {
		if _, reason := itemEnd(data[:end], start); reason != "" {
			return &FramingGap{Offset: start, Length: end - start, Reason: reason}
		}
	}
	return nil
}
