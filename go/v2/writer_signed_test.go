// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestEd25519HonestIsWrittenByteForByte rebuilds the corpus file P6/ed25519-honest.bin: its
// header, its blob frame and its commit signed with the key of seed 00..1f. The writer's bytes
// must equal the stored file, which the verifier also accepts.
func TestEd25519HonestIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "ed25519-honest.bin")
	seed := make([]byte, ed25519.SeedSize)
	for i := range seed {
		seed[i] = byte(i)
	}
	key := ed25519.NewKeyFromSeed(seed)

	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	const headerLength = 11
	frame := stored[headerLength:]
	if frame[0] != 0x8b || frame[1] != 2 {
		t.Fatalf("the blob frame does not start at the header end: % x", frame[:2])
	}
	// The blob frame: 8b 02, type, critical, transparent, content, decoded length, payload digest,
	// storage [1], payload, identifier, signatures.
	position := 2 + 3
	contentEnd, reason := itemEnd(frame, position)
	if reason != "" {
		t.Fatalf("content: %s", reason)
	}
	content := frame[position:contentEnd]
	position = contentEnd + 1 // decoded length: one byte, the payload length of two
	digestEnd, reason := itemEnd(frame, position)
	if reason != "" {
		t.Fatalf("payload digest: %s", reason)
	}
	position = digestEnd + 2 // storage [1]
	if frame[position] != 0x42 {
		t.Fatalf("the payload is not a two byte string: % x", frame[position])
	}
	payload := frame[position+1 : position+3]
	if _, err := writer.Append(9, content, payload); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.CommitSigned(key); err != nil {
		t.Fatal(err)
	}
	if got := writer.Bytes(); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}
