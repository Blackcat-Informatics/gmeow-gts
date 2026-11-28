// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"math/big"
	"testing"
)

// TestCompositeCraftedFilesAreDerivedFromTheWriter rebuilds 13 P6 composite input files in 11
// subtests. The composite key is the one of seed 40..7f. Nine files edit P6/composite-honest.bin,
// which the writer reproduces in TestCompositeHonestIsWrittenByteForByte: the record head, the
// ML-DSA-65 half (the first 3309 bytes of the value) or the Ed25519 half (the last 64). The
// cosign file is written afresh. The last three are the composite public key, the deterministic
// signature value of a stated subject, and a stated message. Each edit refuses an input in which
// the composite record does not occur exactly once. Declared inputs are named at each file.
func TestCompositeCraftedFilesAreDerivedFromTheWriter(t *testing.T) {
	seed := make([]byte, 64)
	for i := range seed {
		seed[i] = byte(0x40 + i)
	}
	key, err := NewCompositeKey(seed)
	if err != nil {
		t.Fatal(err)
	}
	keyID := KeyIdentifier(CompositeAlgorithm, key.PublicKey())
	honest := vectorBytes(t, "composite-honest.bin")
	frames := storedFrames(t, honest)
	prefix := compositeRecordPrefix(keyID)
	if count := bytes.Count(honest, prefix); count != 1 {
		t.Fatalf("the composite record occurs %d times, want once", count)
	}
	// The value follows the record head and the three bytes 59 0d 2d of its length, 3373.
	valueStart := bytes.Index(honest, prefix) + len(prefix) + 3
	edStart := valueStart + mldsa65ValueLength
	flip := func(at int) []byte {
		out := bytes.Clone(honest)
		out[at] ^= 0x01
		return out
	}
	// Input: the lowest bit of the fourth byte of the Ed25519 half, which is a byte of R, and the
	// lowest bit of byte 100 of the ML-DSA-65 half.
	edBit := edStart + 3
	mlBit := valueStart + 100

	t.Run("composite-classical-nonce.bin", func(t *testing.T) {
		compareDerivedComposite(t, flip(edBit), "composite-classical-nonce.bin")
	})
	t.Run("composite-post-quantum.bin", func(t *testing.T) {
		compareDerivedComposite(t, flip(mlBit), "composite-post-quantum.bin")
	})
	t.Run("composite-both.bin", func(t *testing.T) {
		both := flip(edBit)
		both[mlBit] ^= 0x01
		compareDerivedComposite(t, both, "composite-both.bin")
	})
	t.Run("composite-classical-scalar.bin", func(t *testing.T) {
		// S, the last 32 bytes of the Ed25519 half as a little-endian integer, is replaced by
		// S + L, which is at least L and below 2^256.
		order, _ := new(big.Int).SetString("7237005577332262213973186563042994240857116359379907606001950938285454250989", 10)
		little := bytes.Clone(honest[edStart+32 : edStart+64])
		reverse(little)
		sum := new(big.Int).Add(new(big.Int).SetBytes(little), order).FillBytes(make([]byte, 32))
		reverse(sum)
		compareDerivedComposite(t, splice(honest, edStart+32, sum), "composite-classical-scalar.bin")
	})
	t.Run("composite-classical-small-order.bin", func(t *testing.T) {
		// Input: R is the encoding of the identity point, 01 followed by 31 zero bytes.
		identity := append([]byte{1}, make([]byte, 31)...)
		compareDerivedComposite(t, splice(honest, edStart, identity), "composite-classical-small-order.bin")
	})
	t.Run("composite-ed25519-zeroed.bin", func(t *testing.T) {
		compareDerivedComposite(t, splice(honest, edStart, make([]byte, ed25519.SignatureSize)), "composite-ed25519-zeroed.bin")
	})
	t.Run("composite-halves-two-subjects.bin", func(t *testing.T) {
		// The ML-DSA half is the honest one. The Ed25519 half signs the representative of the
		// subject whose frame identifier is input as 32 zero bytes, the record and the segment
		// opener being the honest ones.
		writer, err := NewSegmentWriter(SegmentOptions{})
		if err != nil {
			t.Fatal(err)
		}
		if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
			t.Fatal(err)
		}
		record := SignatureRecord{Algorithm: CompositeAlgorithm, KeyID: keyID}
		other := SignedMessage(record, writer.opener, Digest{})
		edOther := ed25519.Sign(key.ed, compositeRepresentative(other[:]))
		compareDerivedComposite(t, splice(honest, edStart, edOther), "composite-halves-two-subjects.bin")
	})
	t.Run("composite-length-3372.bin and composite-length-3374.bin", func(t *testing.T) {
		// The record head states the length of the value and the value has that length: the last
		// byte is removed under the length 3372, and one zero byte (input) is appended under 3374.
		short := bytes.Clone(honest)
		short[valueStart-1] = 0x2c
		short = append(short[:valueStart+3372], short[valueStart+3373:]...)
		compareDerivedComposite(t, short, "composite-length-3372.bin")
		long := bytes.Clone(honest)
		long[valueStart-1] = 0x2e
		long = append(long[:valueStart+3373], append([]byte{0}, long[valueStart+3373:]...)...)
		compareDerivedComposite(t, long, "composite-length-3374.bin")
	})
	t.Run("cosign-two-algorithms.bin", func(t *testing.T) {
		// The honest blob, then a commit signed by the composite key and by the Ed25519 key of
		// seed 00..1f, which is an input.
		_, edKey := recoveredKeys()
		signer, err := NewEd25519Key(edKey)
		if err != nil {
			t.Fatal(err)
		}
		writer, err := NewSegmentWriter(SegmentOptions{})
		if err != nil {
			t.Fatal(err)
		}
		if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
			t.Fatal(err)
		}
		if _, err := writer.CommitSignedWith(key, signer); err != nil {
			t.Fatal(err)
		}
		compareDerivedComposite(t, writer.Bytes(), "cosign-two-algorithms.bin")
	})
	t.Run("sign-composite-reproduce", func(t *testing.T) {
		// Inputs: the header identifier 00..1f and the frame identifier 20..3f of the subject. The
		// value is the composite signature of the message of that subject, made deterministically.
		var header, frame Digest
		for i := range header {
			header[i] = byte(i)
			frame[i] = byte(0x20 + i)
		}
		record := SignatureRecord{Algorithm: CompositeAlgorithm, KeyID: keyID}
		message := SignedMessage(record, header, frame)
		value, err := key.sign(message[:])
		if err != nil {
			t.Fatal(err)
		}
		compareDerivedComposite(t, key.PublicKey(), "sign-composite-reproduce.public-key.bin")
		compareDerivedComposite(t, value, "sign-composite-reproduce.value.bin")
	})
	t.Run("composite-message-representative.bin", func(t *testing.T) {
		// Input: the message is the 32 bytes 00..1f.
		message := make([]byte, 32)
		for i := range message {
			message[i] = byte(i)
		}
		compareDerivedComposite(t, message, "composite-message-representative.bin")
	})
}
