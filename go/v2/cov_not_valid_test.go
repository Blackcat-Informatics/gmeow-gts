// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// signatureRecordPrefix returns the encoding of a signature record up to its value: the array
// head, the algorithm 1 and the 32-byte key identifier, followed by the byte-string head of the
// 64-byte value.
func signatureRecordPrefix(keyID Digest) []byte {
	out := arrayHead(3)
	out = appendUint(out, Ed25519Algorithm)
	out = appendBytes(out, keyID[:])
	return append(out, 0x58, 0x40)
}

// replaceOnce returns data with its single occurrence of old replaced by new. It fails the test
// unless old occurs exactly once, so an input whose layout has changed is refused.
func replaceOnce(t *testing.T, data, old, replacement []byte) []byte {
	t.Helper()
	if count := bytes.Count(data, old); count != 1 {
		t.Fatalf("the pattern occurs %d times, want once", count)
	}
	return bytes.Replace(data, old, replacement, 1)
}

// TestCovNotValidIsWrittenByteForByte rebuilds P6/cov-not-valid.bin. The frame is written by the
// writer, signed by the keys of seed 20..3f and seed 00..1f; both signature values are then
// byte-edited into the stored values, which are not signatures: the first is zero, and the
// second holds 00..3f, which the reader refuses as a non-canonical scalar. The identifiers do
// not depend on signature values, so the edit changes no other byte.
func TestCovNotValidIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "cov-not-valid.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 1 {
		t.Fatalf("got %d frames, want 1", len(frames))
	}
	keyA, keyB := recoveredKeys()

	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	frame := frames[0]
	if _, err := writer.AppendSigned(frame.frameType, frame.content, frame.payload, keyA, keyB); err != nil {
		t.Fatal(err)
	}
	written := writer.Bytes()

	idA := KeyIdentifier(Ed25519Algorithm, keyA.Public().(ed25519.PublicKey))
	idB := KeyIdentifier(Ed25519Algorithm, keyB.Public().(ed25519.PublicKey))
	signedA := signatureValueAfter(t, written, signatureRecordPrefix(idA))
	signedB := signatureValueAfter(t, written, signatureRecordPrefix(idB))

	edited := replaceOnce(t, written, append(signatureRecordPrefix(idA), signedA...), append(signatureRecordPrefix(idA), make([]byte, 64)...))
	second := make([]byte, 64)
	for i := range second {
		second[i] = byte(i)
	}
	edited = replaceOnce(t, edited, append(signatureRecordPrefix(idB), signedB...), append(signatureRecordPrefix(idB), second...))
	if !bytes.Equal(edited, stored) {
		t.Fatalf("the edited segment differs from the stored file:\n got % x\nwant % x", edited, stored)
	}
}

// recoveredKeys returns the Ed25519 keys of seed 20..3f (key identifier 03ea0025...) and seed
// 00..1f (key identifier 0b79463c...), the two recovered keys the corpus signs with.
func recoveredKeys() (ed25519.PrivateKey, ed25519.PrivateKey) {
	seed03ea := make([]byte, ed25519.SeedSize)
	seed0b79 := make([]byte, ed25519.SeedSize)
	for i := range seed03ea {
		seed03ea[i] = byte(0x20 + i)
		seed0b79[i] = byte(i)
	}
	return ed25519.NewKeyFromSeed(seed03ea), ed25519.NewKeyFromSeed(seed0b79)
}

// signatureValueAfter returns the 64-byte value that follows prefix in data, failing the test
// unless prefix occurs exactly once.
func signatureValueAfter(t *testing.T, data, prefix []byte) []byte {
	t.Helper()
	if count := bytes.Count(data, prefix); count != 1 {
		t.Fatalf("the signature record occurs %d times, want once", count)
	}
	start := bytes.Index(data, prefix) + len(prefix)
	return data[start : start+64]
}
