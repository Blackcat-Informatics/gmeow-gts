// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// frameSpan is one stored frame of a corpus file: the content record and the decoded payload,
// as the writer takes them, the signature list exactly as stored, and the end of the frame in
// the file.
type frameSpan struct {
	frameType  uint64
	content    []byte
	payload    []byte
	signatures []byte
	end        int
}

// byteStringBody returns the bytes of the byte string encoded in item, which holds its head and
// its body and nothing else.
func byteStringBody(t *testing.T, item []byte) []byte {
	t.Helper()
	if len(item) == 0 || item[0]>>5 != 2 {
		t.Fatalf("payload is not a byte string")
	}
	headLength := map[byte]int{24: 2, 25: 3, 26: 5, 27: 9}[item[0]&0x1f]
	if item[0]&0x1f < 24 {
		headLength = 1
	}
	if headLength == 0 || len(item) < headLength {
		t.Fatalf("payload head is not supported")
	}
	return item[headLength:]
}

// storedFrames returns the frames that follow the header of a corpus file, in order. The file
// must end with its last frame.
func storedFrames(t *testing.T, data []byte) []frameSpan {
	t.Helper()
	frames, end := storedFramesBeforeClosing(t, data)
	if end != len(data) {
		t.Fatalf("no frame envelope at %d", end)
	}
	return frames
}

// storedFramesBeforeClosing returns the frames that follow the header of a corpus file, in order,
// and the offset of the first byte after them. That byte starts the closing items, if any.
func storedFramesBeforeClosing(t *testing.T, data []byte) ([]frameSpan, int) {
	t.Helper()
	headerEnd, reason := itemEnd(data, 0)
	if reason != "" {
		t.Fatalf("header: %s", reason)
	}
	return storedFramesFrom(t, data, headerEnd)
}

// storedFramesFrom returns the frames that begin at start, in order, and the offset of the first
// byte after them. Frames are read from start until a byte that does not begin a frame.
func storedFramesFrom(t *testing.T, data []byte, start int) ([]frameSpan, int) {
	t.Helper()
	var frames []frameSpan
	position := start
	for position < len(data) {
		if data[position] != 0x8b || position+1 >= len(data) || data[position+1] != 2 {
			break
		}
		typePosition := position + 2
		typeEnd, reason := itemEnd(data, typePosition)
		if reason != "" {
			t.Fatalf("type: %s", reason)
		}
		frameType := uint64(data[typePosition] & 0x1f)
		contentStart := typeEnd + 2 // critical and transparent
		contentEnd, reason := itemEnd(data, contentStart)
		if reason != "" {
			t.Fatalf("content: %s", reason)
		}
		lengthEnd, reason := itemEnd(data, contentEnd)
		if reason != "" {
			t.Fatalf("decoded length: %s", reason)
		}
		digestEnd, reason := itemEnd(data, lengthEnd)
		if reason != "" {
			t.Fatalf("payload digest: %s", reason)
		}
		payloadStart := digestEnd + 2 // storage [1]
		payloadEnd, reason := itemEnd(data, payloadStart)
		if reason != "" {
			t.Fatalf("payload: %s", reason)
		}
		identifierEnd, reason := itemEnd(data, payloadEnd)
		if reason != "" {
			t.Fatalf("identifier: %s", reason)
		}
		end, reason := itemEnd(data, identifierEnd)
		if reason != "" {
			t.Fatalf("signatures: %s", reason)
		}
		frames = append(frames, frameSpan{
			frameType:  frameType,
			content:    data[contentStart:contentEnd],
			payload:    byteStringBody(t, data[payloadStart:payloadEnd]),
			signatures: data[identifierEnd:end],
			end:        end,
		})
		position = end
	}
	return frames, position
}

// writeAppendSegment writes the segment of P6/cosign-append-commit.bin: the two ordinary frames
// stored in `stored`, then one commit signed with the key of seed 01 x32 and one commit signed
// with the key of seed 02 x32. They are the two recovered seeds that sign this file.
func writeAppendSegment(t *testing.T, stored []byte) []byte {
	t.Helper()
	first := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{0x01}, ed25519.SeedSize))
	second := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{0x02}, ed25519.SeedSize))
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	headerEnd, _ := itemEnd(stored, 0)
	if got := writer.Bytes(); !bytes.Equal(got, stored[:headerEnd]) {
		t.Fatalf("the header differs: % x", got)
	}
	frames := storedFrames(t, stored)
	if len(frames) != 4 {
		t.Fatalf("got %d frames, want 4", len(frames))
	}
	for _, frame := range frames[:2] {
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := writer.CommitSigned(first); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.CommitSigned(second); err != nil {
		t.Fatal(err)
	}
	return writer.Bytes()
}

// TestCosignAppendCommitIsWrittenByteForByte rebuilds P6/cosign-append-commit.bin.
func TestCosignAppendCommitIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "cosign-append-commit.bin")
	if got := writeAppendSegment(t, stored); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}

// TestCosignCapacityFilesAreWrittenByteForByte rebuilds P6/cosign-capacity-spent.bin and
// P6/cosign-item-limit.bin. The two files hold the same bytes: two ordinary frames, then one
// commit signed with the key of seed 01 x32. The cases differ in the limit they set on the
// co-signer, which is recorded in their case files and not in the segment.
func TestCosignCapacityFilesAreWrittenByteForByte(t *testing.T) {
	for _, name := range []string{"cosign-capacity-spent.bin", "cosign-item-limit.bin"} {
		t.Run(name, func(t *testing.T) {
			stored := vectorBytes(t, name)
			if got := writeCapacitySegment(t, stored); !bytes.Equal(got, stored) {
				t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
			}
		})
	}
}

// writeCapacitySegment writes the two ordinary frames stored in `stored` and one commit signed
// with the key of seed 01 x32.
func writeCapacitySegment(t *testing.T, stored []byte) []byte {
	t.Helper()
	publisher := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{0x01}, ed25519.SeedSize))
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	frames := storedFrames(t, stored)
	if len(frames) != 3 {
		t.Fatalf("got %d frames, want 3", len(frames))
	}
	for _, frame := range frames[:2] {
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := writer.CommitSigned(publisher); err != nil {
		t.Fatal(err)
	}
	return writer.Bytes()
}

// TestCosignAfterTornFilesAreDerivedFromWrittenSegments rebuilds the two damaged inputs of
// P6/cosign-after-torn-nothing. The torn file is the written cosign-append-commit segment cut
// inside the signature value of its first commit, which is the last value of the prefix H T Q C.
// The undelimitable file is the written capacity segment followed by the single byte 0x1c,
// which starts no item.
func TestCosignAfterTornFilesAreDerivedFromWrittenSegments(t *testing.T) {
	const tornLength = 415
	appended := writeAppendSegment(t, vectorBytes(t, "cosign-append-commit.bin"))
	torn := vectorBytes(t, "cosign-after-torn-nothing.torn.bin")
	if len(torn) != tornLength || !bytes.Equal(torn, appended[:tornLength]) {
		t.Fatalf("the torn file is not the first %d bytes of cosign-append-commit.bin", tornLength)
	}
	capacity := vectorBytes(t, "cosign-capacity-spent.bin")
	written := append(writeCapacitySegment(t, capacity), 0x1c)
	if stored := vectorBytes(t, "cosign-after-torn-nothing.undelimitable.bin"); !bytes.Equal(written, stored) {
		t.Fatalf("the derived file differs from the stored one:\n got % x\nwant % x", written, stored)
	}
}
