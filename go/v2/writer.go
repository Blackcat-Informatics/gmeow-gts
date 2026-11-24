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
	rangePrefix prefixRange
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

	writer := &SegmentWriter{data: data, head: hash("gts2 header", data)}
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
	if frameType == commitType {
		return Digest{}, fmt.Errorf("use Commit to append a commit frame")
	}
	if frameType == 0 || frameType > parityType {
		return Digest{}, fmt.Errorf("cannot write GTS v2 frame type %d", frameType)
	}
	if end, reason := itemEnd(content, 0); reason != "" || end != len(content) {
		return Digest{}, fmt.Errorf("frame type %d requires one complete content record", frameType)
	}
	if reason := grammar(content); reason != "" {
		return Digest{}, fmt.Errorf("frame type %d content: %s", frameType, reason)
	}
	if reason := checkContent(frameType, content); reason != "" {
		return Digest{}, fmt.Errorf("frame type %d content: %s", frameType, reason)
	}
	if reason := checkPayloadLength(frameType, uint64(len(payload))); reason != "" {
		return Digest{}, fmt.Errorf("frame type %d payload length: %s", frameType, reason)
	}
	return w.append(frameType, content, payload)
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
	return w.append(commitType, content, nil)
}

func (w *SegmentWriter) append(frameType uint64, content, payload []byte) (Digest, error) {
	if w.rangePrefix.count == 0 {
		return Digest{}, fmt.Errorf("initialize the segment writer with NewSegmentWriter")
	}
	transparent := frameType == parityType
	if !transparent && w.rangePrefix.count == math.MaxUint64 {
		return Digest{}, fmt.Errorf("segment prefix exceeds the maximum leaf count")
	}

	w.data = append(w.data, 0x8b, 2)
	start := len(w.data)
	w.data = appendHead(w.data, 0, frameType)
	w.data = append(w.data, boolByte(frameType < commitType), boolByte(transparent))
	w.data = append(w.data, content...)
	w.data = appendHead(w.data, 0, uint64(len(payload)))
	payloadID := hash("gts2 payload", payload)
	w.data = appendByteString(w.data, payloadID[:])
	id := frameID(w.head, w.data[start:])

	w.data = append(w.data, 0x81, 1)
	w.data = appendByteString(w.data, payload)
	w.data = appendByteString(w.data, id[:])
	w.data = append(w.data, 0x80)
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
