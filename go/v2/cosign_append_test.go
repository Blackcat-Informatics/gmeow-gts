// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// frameSpan is one stored frame of a corpus file: the content record and the decoded payload,
// as the writer takes them, and the end of the frame in the file.
type frameSpan struct {
	frameType uint64
	content   []byte
	payload   []byte
	end       int
}

// storedFrames returns the frames that follow the header of a corpus file, in order.
func storedFrames(t *testing.T, data []byte) []frameSpan {
	t.Helper()
	headerEnd, reason := itemEnd(data, 0)
	if reason != "" {
		t.Fatalf("header: %s", reason)
	}
	var frames []frameSpan
	for position := headerEnd; position < len(data); {
		if data[position] != 0x8b || data[position+1] != 2 {
			t.Fatalf("no frame envelope at %d", position)
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
			frameType: frameType,
			content:   data[contentStart:contentEnd],
			payload:   data[payloadStart+1 : payloadEnd],
			end:       end,
		})
		position = end
	}
	return frames
}

// TestCosignAppendCommitIsWrittenByteForByte rebuilds P6/cosign-append-commit.bin: two ordinary
// frames, then one commit signed with the key of seed 01 x32 and one commit signed with the key
// of seed 02 x32. The keys are the two recovered seeds that sign this file.
func TestCosignAppendCommitIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "cosign-append-commit.bin")
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
	if got := writer.Bytes(); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}
