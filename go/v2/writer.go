// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"math"
)

// SegmentOptions supplies the optional fields of a registry version 1 header.
// The caller prepares the segment content for any profile or canonical claim.
type SegmentOptions struct {
	Profile   uint64
	Canonical bool
	Lineage   *Lineage
}

// SegmentWriter assembles one header-led GTS v2 segment with unsigned, plain
// frames. It derives frame identifiers and commits over the segment prefix.
// Use the pointer returned by NewSegmentWriter for the life of the segment.
type SegmentWriter struct {
	data        []byte
	head        Digest
	opener      Digest
	rangePrefix prefixRange
	// itemEnds holds the end offset of each item written so far, the header first.
	itemEnds []int
	// indexStart is the offset of the latest index frame, or -1 when there is none.
	indexStart int
}

// NewSegmentWriter writes a header and starts a segment with its opener as
// the first leaf of the prefix range.
func NewSegmentWriter(options SegmentOptions) (*SegmentWriter, error) {
	if options.Profile > 6 {
		return nil, fmt.Errorf("cannot write GTS v2 profile %d", options.Profile)
	}
	data := appendEnumOption([]byte{0x87, 1, 2, 1, 1}, options.Profile)
	if options.Canonical {
		data = appendEnumOption(data, 1)
	} else {
		data = appendEnumOption(data, 0)
	}
	if lineage := options.Lineage; lineage != nil {
		if len(lineage.PreviousHead) != len(Digest{}) || lineage.Ordinal == 0 {
			return nil, fmt.Errorf("lineage requires a 32-byte previous head and a positive ordinal")
		}
		data = append(data, 0x82, 2, 0x82)
		data = appendByteString(data, lineage.PreviousHead)
		data = appendHead(data, 0, lineage.Ordinal)
	} else {
		data = append(data, 0x81, 1)
	}

	writer := &SegmentWriter{data: data, head: hash("gts2 header", data), itemEnds: []int{len(data)}, indexStart: -1}
	writer.opener = writer.head
	writer.rangePrefix.push(writer.head)
	return writer, nil
}

// Head returns the identifier of the opener or latest ordinary frame.
func (w *SegmentWriter) Head() Digest {
	return w.head
}

// Bytes returns a snapshot of the segment in wire order.
func (w *SegmentWriter) Bytes() []byte {
	return bytes.Clone(w.data)
}

// Append writes a frame with one encoded content record and a decoded payload.
// The caller constructs the payload and applies the frame type's content rules.
// Use Commit to add a commit over the current prefix.
func (w *SegmentWriter) Append(frameType uint64, content, payload []byte) (Digest, error) {
	if err := checkFrame(frameType, content, payload); err != nil {
		return Digest{}, err
	}
	return w.append(frameType, content, payload, nil)
}

// checkFrame applies the content and payload rules that Append and AppendSigned share.
func checkFrame(frameType uint64, content, payload []byte) error {
	if frameType == commitType {
		return fmt.Errorf("use Commit to append a commit frame")
	}
	if frameType == 0 || frameType > parityType {
		return fmt.Errorf("cannot write GTS v2 frame type %d", frameType)
	}
	if end, reason := itemEnd(content, 0); reason != "" || end != len(content) {
		return fmt.Errorf("frame type %d requires one complete content record", frameType)
	}
	if reason := grammar(content); reason != "" {
		return fmt.Errorf("frame type %d content: %s", frameType, reason)
	}
	if reason := checkContent(frameType, content); reason != "" {
		return fmt.Errorf("frame type %d content: %s", frameType, reason)
	}
	if reason := checkPayloadLength(frameType, uint64(len(payload))); reason != "" {
		return fmt.Errorf("frame type %d payload length: %s", frameType, reason)
	}
	return nil
}

// Commit appends a commitment to the opener and all preceding ordinary
// frames, including any earlier commits in the segment.
func (w *SegmentWriter) Commit() (Digest, error) {
	if w.rangePrefix.count == 0 {
		return Digest{}, fmt.Errorf("initialize the segment writer with NewSegmentWriter")
	}
	root := w.rangePrefix.root()
	content := appendHead(appendHead(nil, 4, 2), 0, w.rangePrefix.count-1)
	content = appendByteString(content, root[:])
	return w.append(commitType, content, nil, nil)
}

// WriteOffsets appends the offsets item: the length of the header and of every frame written so
// far, opener first. It is written after the last frame and before WriteLocator.
func (w *SegmentWriter) WriteOffsets() error {
	if len(w.itemEnds) < 2 {
		return fmt.Errorf("offsets require the header and at least one frame")
	}
	lengths := make([]uint64, 0, len(w.itemEnds))
	previous := 0
	for _, end := range w.itemEnds {
		lengths = append(lengths, uint64(end-previous))
		previous = end
	}
	item := appendUint(arrayHead(2), 4)
	item = append(item, arrayHead(uint64(len(lengths)))...)
	for _, length := range lengths {
		item = appendUint(item, length)
	}
	w.data = append(w.data, item...)
	w.itemEnds = append(w.itemEnds, len(w.data))
	return nil
}

// WriteLocator appends the locator: the distance from the latest index frame to the locator
// itself. It fails when the segment has no index frame. It is the last item of the segment.
func (w *SegmentWriter) WriteLocator() error {
	if w.indexStart < 0 {
		return fmt.Errorf("a locator requires an index frame")
	}
	distance := uint64(len(w.data) - w.indexStart)
	item := appendUint(arrayHead(2), 3)
	item = append(item, 0x1b)
	item = binary.BigEndian.AppendUint64(item, distance)
	w.data = append(w.data, item...)
	w.itemEnds = append(w.itemEnds, len(w.data))
	return nil
}

// append writes one frame. The signatures element is the list that sign returns for the
// frame's effective identifier, or the empty list when sign is nil. The identity of the
// frame is complete before sign runs, so nothing is written when sign fails.
func (w *SegmentWriter) append(frameType uint64, content, payload []byte, sign func(Digest) ([]byte, error)) (Digest, error) {
	if w.rangePrefix.count == 0 {
		return Digest{}, fmt.Errorf("initialize the segment writer with NewSegmentWriter")
	}
	transparent := frameType == parityType
	if !transparent && w.rangePrefix.count == math.MaxUint64 {
		return Digest{}, fmt.Errorf("segment prefix exceeds the maximum leaf count")
	}

	identity := appendHead(nil, 0, frameType)
	identity = append(identity, boolByte(frameType < commitType), boolByte(transparent))
	identity = append(identity, content...)
	identity = appendHead(identity, 0, uint64(len(payload)))
	payloadID := hash("gts2 payload", payload)
	identity = appendByteString(identity, payloadID[:])
	id := frameID(w.head, identity)

	signatures := []byte{0x80}
	if sign != nil {
		var err error
		if signatures, err = sign(id); err != nil {
			return Digest{}, err
		}
	}
	frame := append([]byte{0x8b, 2}, identity...)
	frame = append(frame, 0x81, 1)
	frame = appendByteString(frame, payload)
	frame = appendByteString(frame, id[:])
	frame = append(frame, signatures...)
	if frameType == indexType {
		w.indexStart = len(w.data)
	}
	w.data = append(w.data, frame...)
	w.itemEnds = append(w.itemEnds, len(w.data))
	if !transparent {
		w.rangePrefix.push(id)
		w.head = id
	}
	return id, nil
}

func appendEnumOption(data []byte, code uint64) []byte {
	if code == 0 {
		return append(data, 0x81, 1)
	}
	return appendHead(append(data, 0x82, 2), 0, code)
}

func boolByte(value bool) byte {
	if value {
		return 0xf5
	}
	return 0xf4
}

func appendByteString(data, value []byte) []byte {
	return append(appendHead(data, 2, uint64(len(value))), value...)
}

func appendHead(data []byte, major byte, value uint64) []byte {
	tag := major << 5
	switch {
	case value < 24:
		return append(data, tag|byte(value))
	case value <= math.MaxUint8:
		return append(data, tag|24, byte(value))
	case value <= math.MaxUint16:
		return binary.BigEndian.AppendUint16(append(data, tag|25), uint16(value))
	case value <= math.MaxUint32:
		return binary.BigEndian.AppendUint32(append(data, tag|26), uint32(value))
	default:
		return binary.BigEndian.AppendUint64(append(data, tag|27), value)
	}
}

type rangePeak struct {
	height int
	digest Digest
}

type prefixRange struct {
	count uint64
	peaks []rangePeak
}

func (r *prefixRange) push(identifier Digest) {
	var encoded [69]byte
	leaf := appendByteString(appendHead(encoded[:0], 4, 1), identifier[:])
	peak := rangePeak{digest: hash("gts2 range leaf", leaf)}
	for len(r.peaks) != 0 && r.peaks[len(r.peaks)-1].height == peak.height {
		left := r.peaks[len(r.peaks)-1].digest
		r.peaks = r.peaks[:len(r.peaks)-1]
		node := appendByteString(appendHead(encoded[:0], 4, 2), left[:])
		node = appendByteString(node, peak.digest[:])
		peak.digest = hash("gts2 range node", node)
		peak.height++
	}
	r.peaks = append(r.peaks, peak)
	r.count++
}

func (r *prefixRange) root() Digest {
	record := appendHead(appendHead(nil, 4, 2), 0, r.count)
	record = appendHead(record, 4, uint64(len(r.peaks)))
	for _, peak := range r.peaks {
		record = appendByteString(record, peak.digest[:])
	}
	return hash("gts2 range root", record)
}
