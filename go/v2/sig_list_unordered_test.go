// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestSigListUnorderedIsDerivedFromAWrittenFrame rebuilds P6/sig-list-unordered.bin. The writer
// writes the frame of P6/sig-list-order.bin, signed by the Ed25519 keys of seed 00..1f and seed
// 20..3f and by the composite key of seed 40..7f, with its records in ascending bytewise order.
// The file is that segment with the two Ed25519 records exchanged: each record is 102 bytes, and
// the edit refuses an input in which either record does not occur exactly once.
func TestSigListUnorderedIsDerivedFromAWrittenFrame(t *testing.T) {
	stored := vectorBytes(t, "sig-list-unordered.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 1 {
		t.Fatalf("got %d frames, want 1", len(frames))
	}
	keyA, keyB := recoveredKeys()
	ed, err := ed25519Signers([]ed25519.PrivateKey{keyA, keyB})
	if err != nil {
		t.Fatal(err)
	}
	seed := make([]byte, 64)
	for i := range seed {
		seed[i] = byte(0x40 + i)
	}
	composite, err := NewCompositeKey(seed)
	if err != nil {
		t.Fatal(err)
	}
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.AppendSignedWith(frames[0].frameType, frames[0].content, frames[0].payload, ed[0], ed[1], composite); err != nil {
		t.Fatal(err)
	}
	ordered := writer.Bytes()

	recordA := ed25519Record(t, ordered, ed[0])
	recordB := ed25519Record(t, ordered, ed[1])
	if len(recordA) != len(recordB) {
		t.Fatalf("the Ed25519 records differ in length: %d and %d", len(recordA), len(recordB))
	}
	positionA := occurrenceOnce(t, ordered, recordA)
	positionB := occurrenceOnce(t, ordered, recordB)
	unordered := bytes.Clone(ordered)
	copy(unordered[positionA:positionA+len(recordA)], recordB)
	copy(unordered[positionB:positionB+len(recordB)], recordA)
	if !bytes.Equal(unordered, stored) {
		t.Fatalf("the derived file differs from the stored one:\n got % x\nwant % x", unordered, stored)
	}
}

// ed25519Record returns the encoded signature record of key within data: its prefix and its
// 64-byte value, as written by the signer.
func ed25519Record(t *testing.T, data []byte, key SigningKey) []byte {
	t.Helper()
	keyID := KeyIdentifier(key.Algorithm(), key.PublicKey())
	prefix := signatureRecordPrefix(keyID)
	return append(bytes.Clone(prefix), signatureValueAfter(t, data, prefix)...)
}

// occurrenceOnce returns the offset of the only occurrence of pattern in data, failing the test
// unless there is exactly one.
func occurrenceOnce(t *testing.T, data, pattern []byte) int {
	t.Helper()
	if count := bytes.Count(data, pattern); count != 1 {
		t.Fatalf("the pattern occurs %d times, want once", count)
	}
	return bytes.Index(data, pattern)
}
