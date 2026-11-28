// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"encoding/hex"
	"math/big"
	"testing"
)

// TestCraftedSignatureFilesAreDerivedFromWrittenSegments rebuilds the P6 input files that are a
// written segment with one named edit of its signature record. The Ed25519 files start from
// P6/ed25519-honest.bin, which the writer reproduces in TestEd25519HonestIsWrittenByteForByte: a
// blob and a commit signed by the key of seed 00..1f. The composite files start from
// P6/composite-honest.bin, which the writer reproduces in TestCompositeHonestIsWrittenByteForByte.
// Every edit refuses an input in which the bytes it replaces do not occur exactly once.
func TestCraftedSignatureFilesAreDerivedFromWrittenSegments(t *testing.T) {
	_, key := recoveredKeys()
	honest := vectorBytes(t, "ed25519-honest.bin")
	signed := KeyIdentifier(Ed25519Algorithm, key.Public().(ed25519.PublicKey))
	value := signatureValueAfter(t, honest, signatureRecordPrefix(signed))

	// A held key whose identifier the record names in place of the signer's; the value is left
	// as written.
	relabel := func(t *testing.T, data []byte, from, to Digest) []byte {
		return replaceOnce(t, data, append([]byte{0x58, 0x20}, from[:]...), append([]byte{0x58, 0x20}, to[:]...))
	}
	// The scalar S is value[32:64], a little-endian integer (P6 section 11.4).
	withScalar := func(t *testing.T, scalar []byte) []byte {
		edited := append(bytes.Clone(value[:32]), scalar...)
		return replaceOnce(t, honest, append(signatureRecordPrefix(signed), value...), append(signatureRecordPrefix(signed), edited...))
	}
	scalarPlusOne := func() []byte {
		little := bytes.Clone(value[32:])
		reverse(little)
		sum := new(big.Int).Add(new(big.Int).SetBytes(little), big.NewInt(1)).FillBytes(make([]byte, 32))
		reverse(sum)
		return sum
	}
	// The encoding of y = p, an alias of y = 0 that does not decode (P6 section 11.4, case 9).
	fieldPrime := new(big.Int).Sub(new(big.Int).Lsh(big.NewInt(1), 255), big.NewInt(19)).FillBytes(make([]byte, 32))
	reverse(fieldPrime)

	heldKeys := map[string]string{
		"ed25519-key-non-canonical-y.bin": "f2ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
		"ed25519-key-off-curve.bin":       "0200000000000000000000000000000000000000000000000000000000000000",
		"ed25519-key-small-order-2.bin":   "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
		"ed25519-key-small-order-3.bin":   "0000000000000000000000000000000000000000000000000000000000000000",
		"ed25519-key-small-order-4.bin":   "0000000000000000000000000000000000000000000000000000000000000080",
		"ed25519-key-small-order-7.bin":   "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
		"ed25519-key-small-order-8.bin":   "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
	}
	for name, encoded := range heldKeys {
		t.Run(name, func(t *testing.T) {
			public, err := hex.DecodeString(encoded)
			if err != nil {
				t.Fatal(err)
			}
			derived := relabel(t, honest, signed, KeyIdentifier(Ed25519Algorithm, public))
			compareDerived(t, derived, name)
		})
	}

	t.Run("subject-relabel-algorithm.bin", func(t *testing.T) {
		// The record's algorithm is 2; its key identifier and value are unchanged.
		prefix := signatureRecordPrefix(signed)
		relabelled := append([]byte{0x83, 0x02}, prefix[2:]...)
		compareDerived(t, replaceOnce(t, honest, prefix, relabelled), "subject-relabel-algorithm.bin")
	})
	t.Run("ed25519-scalar-max.bin", func(t *testing.T) {
		compareDerived(t, withScalar(t, bytes.Repeat([]byte{0xff}, 32)), "ed25519-scalar-max.bin")
	})
	t.Run("proc-case-8.bin and proc-case-9.bin", func(t *testing.T) {
		// Case 8: value[32:64] is S + 1, still below L. Case 9: as case 8, under the identifier
		// of the alias y = p.
		case8 := withScalar(t, scalarPlusOne())
		compareDerived(t, case8, "proc-case-8.bin")
		compareDerived(t, relabel(t, case8, signed, KeyIdentifier(Ed25519Algorithm, fieldPrime)), "proc-case-9.bin")
	})
	t.Run("ed25519-equation-message.bin", func(t *testing.T) {
		// A valid signature by the same key over the subject whose frame identifier has the low
		// bit of its first byte changed.
		frames := storedFrames(t, honest)
		writer, err := NewSegmentWriter(SegmentOptions{})
		if err != nil {
			t.Fatal(err)
		}
		if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
			t.Fatal(err)
		}
		commit, err := writer.CommitSigned(key)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(writer.Bytes(), honest) {
			t.Fatal("the written segment is not P6/ed25519-honest.bin")
		}
		changed := commit
		changed[0] ^= 0x01
		message := SignedMessage(SignatureRecord{Algorithm: Ed25519Algorithm, KeyID: signed}, writer.opener, changed)
		other := ed25519.Sign(key, message[:])
		derived := replaceOnce(t, honest, append(signatureRecordPrefix(signed), value...), append(signatureRecordPrefix(signed), other...))
		compareDerived(t, derived, "ed25519-equation-message.bin")
	})

	composite := vectorBytes(t, "composite-honest.bin")
	seed := make([]byte, 64)
	for i := range seed {
		seed[i] = byte(0x40 + i)
	}
	compositeKey, err := NewCompositeKey(seed)
	if err != nil {
		t.Fatal(err)
	}
	compositePublic := compositeKey.PublicKey()
	compositeID := KeyIdentifier(CompositeAlgorithm, compositePublic)
	t.Run("composite-key-1983.bin and composite-key-1985.bin", func(t *testing.T) {
		// The held key is one byte short, or has one zero byte appended, under its own identifier.
		short := KeyIdentifier(CompositeAlgorithm, compositePublic[:len(compositePublic)-1])
		long := KeyIdentifier(CompositeAlgorithm, append(bytes.Clone(compositePublic), 0))
		compareDerived(t, relabel(t, composite, compositeID, short), "composite-key-1983.bin")
		compareDerived(t, relabel(t, composite, compositeID, long), "composite-key-1985.bin")
	})
	t.Run("composite-hint-count-above-omega.bin", func(t *testing.T) {
		// The ML-DSA-65 half is 3309 bytes; its last k = 6 bytes are the hint counts. The first
		// count, at byte 3303 of the value, is 56, above omega = 55.
		prefix := append(arrayHead(3), byte(CompositeAlgorithm))
		prefix = appendBytes(prefix, compositeID[:])
		prefix = append(prefix, 0x59, 0x0d, 0x2d)
		if count := bytes.Count(composite, prefix); count != 1 {
			t.Fatalf("the composite record occurs %d times, want once", count)
		}
		derived := bytes.Clone(composite)
		derived[bytes.Index(composite, prefix)+len(prefix)+3303] = 56
		compareDerived(t, derived, "composite-hint-count-above-omega.bin")
	})
}

// compareDerived fails the test unless derived is the stored P6 file name.
func compareDerived(t *testing.T, derived []byte, name string) {
	t.Helper()
	if stored := vectorBytes(t, name); !bytes.Equal(derived, stored) {
		t.Fatalf("the derived file differs from P6/%s:\n got % x\nwant % x", name, derived, stored)
	}
}

func reverse(data []byte) {
	for i, j := 0, len(data)-1; i < j; i, j = i+1, j-1 {
		data[i], data[j] = data[j], data[i]
	}
}
