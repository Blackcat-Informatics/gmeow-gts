// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"strings"
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
	got := writer.Bytes()
	if !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
	for _, name := range honestSegmentFiles {
		if !bytes.Equal(got, vectorBytes(t, name)) {
			t.Errorf("P6/%s is not the written honest segment", name)
		}
	}
}

// honestSegmentFiles are the P6 input files that hold the honest segment of
// TestEd25519HonestIsWrittenByteForByte unchanged. Their cases read it with other declared
// inputs, such as the held and trusted keys, or check another value of it; auth-commit-valid
// repeats the read of ed25519-honest.
var honestSegmentFiles = []string{
	"auth-all.bin",
	"auth-commit-valid.bin",
	"auth-untrusted.bin",
	"cov-simple.bin",
	"ed25519-honest-reproduce.bin",
	"key-held-not-trusted.bin",
	"key-held-order.bin",
	"key-mismatch-algorithm.bin",
	"key-mismatch-bytes.bin",
	"key-mismatch-two-entries.bin",
	"key-trusted-not-held.bin",
	"proc-key-mismatch.bin",
	"proc-key-not-held.bin",
	"proc-trust-not-status.bin",
}

// TestAppendSignedRefusesATransparentFrame: a transparent frame carries no signature (P6-3), so
// the writer refuses to sign one and writes nothing.
func TestAppendSignedRefusesATransparentFrame(t *testing.T) {
	_, key, err := ed25519.GenerateKey(nil)
	if err != nil {
		t.Fatal(err)
	}
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	before := writer.Bytes()
	if _, err := writer.AppendSigned(parityType, nil, nil, key); err == nil || !strings.Contains(err.Error(), "P6-3") {
		t.Fatalf("a transparent frame was not refused by P6-3: %v", err)
	}
	if !bytes.Equal(writer.Bytes(), before) {
		t.Fatal("a refused transparent frame changed the segment")
	}
}

// TestSignaturesOfTwoKeysAreWrittenInOneOrder: the records of two keys are written in ascending
// bytewise order of their encodings (P6-52), whatever order the caller gives the keys.
func TestSignaturesOfTwoKeysAreWrittenInOneOrder(t *testing.T) {
	first := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{0x01}, ed25519.SeedSize))
	second := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{0x02}, ed25519.SeedSize))
	// A blob content record: [digest of its unit, [1]].
	content := append(append([]byte{0x82, 0x58, 0x20}, bytes.Repeat([]byte{0x07}, 32)...), 0x81, 0x01)
	payload := []byte("ab")
	written := func(keys ...ed25519.PrivateKey) []byte {
		writer, err := NewSegmentWriter(SegmentOptions{})
		if err != nil {
			t.Fatal(err)
		}
		if _, err := writer.AppendSigned(9, content, payload, keys...); err != nil {
			t.Fatal(err)
		}
		return writer.Bytes()
	}
	if forward, reverse := written(first, second), written(second, first); !bytes.Equal(forward, reverse) {
		t.Fatalf("the signature order follows the key order:\n%x\n%x", forward, reverse)
	}
}
