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

// TestCraftedSignatureFilesAreDerivedFromWrittenSegments rebuilds 31 P6 input files, each a
// written segment with named edits of its signature record. The Ed25519 files start from
// P6/ed25519-honest.bin, which the writer reproduces in TestEd25519HonestIsWrittenByteForByte: a
// blob and a commit signed by the key of seed 00..1f. The composite files start from
// P6/composite-honest.bin, which the writer reproduces in TestCompositeHonestIsWrittenByteForByte.
// Every edit finds its target through a pattern that must occur exactly once.
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
		"ed25519-key-non-canonical-y.bin":    "f2ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
		"ed25519-key-off-curve.bin":          "0200000000000000000000000000000000000000000000000000000000000000",
		"ed25519-key-small-order-1.bin":      "0100000000000000000000000000000000000000000000000000000000000000",
		"ed25519-key-small-order-5.bin":      "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
		"ed25519-key-small-order-6.bin":      "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
		"ed25519-key-non-canonical.bin":      "0100000000000000000000000000000000000000000000000000000000000080",
		"ed25519-key-non-canonical-sign.bin": "0100000000000000000000000000000000000000000000000000000000000080",
		"ed25519-key-alias.bin":              "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
		"proc-case-6.bin":                    "0100000000000000000000000000000000000000000000000000000000000000",
		"ed25519-key-small-order-2.bin":      "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
		"ed25519-key-small-order-3.bin":      "0000000000000000000000000000000000000000000000000000000000000000",
		"ed25519-key-small-order-4.bin":      "0000000000000000000000000000000000000000000000000000000000000080",
		"ed25519-key-small-order-7.bin":      "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
		"ed25519-key-small-order-8.bin":      "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
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
	t.Run("scalars", func(t *testing.T) {
		// S is replaced by L, by L - 1, and by S + L, which is at least L (P6 section 11.4, case 5).
		order, _ := new(big.Int).SetString("7237005577332262213973186563042994240857116359379907606001950938285454250989", 10)
		littleEndian := func(n *big.Int) []byte {
			out := n.FillBytes(make([]byte, 32))
			reverse(out)
			return out
		}
		little := bytes.Clone(value[32:])
		reverse(little)
		plusOrder := littleEndian(new(big.Int).Add(new(big.Int).SetBytes(little), order))
		compareDerived(t, withScalar(t, littleEndian(order)), "ed25519-scalar-equals-l.bin")
		compareDerived(t, withScalar(t, littleEndian(new(big.Int).Sub(order, big.NewInt(1)))), "ed25519-scalar-below-l.bin")
		compareDerived(t, withScalar(t, plusOrder), "ed25519-scalar-above-l.bin")
		compareDerived(t, withScalar(t, plusOrder), "proc-case-5.bin")
		// Case 10: as case 6, the identity point as the held key, and also S + L.
		identity := KeyIdentifier(Ed25519Algorithm, append([]byte{1}, make([]byte, 31)...))
		compareDerived(t, relabel(t, withScalar(t, plusOrder), signed, identity), "proc-case-10.bin")
		compareDerived(t, relabel(t, withScalar(t, plusOrder), signed, identity), "ed25519-order-scalar-key.bin")
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
	t.Run("composite hint block rewrites", func(t *testing.T) {
		// The hint block is the last omega + k = 61 bytes of the ML-DSA-65 half: omega index
		// bytes, then k counts. The first two files replace the whole block with the one described
		// by its leading index bytes (the rest are zero) and the six counts. The third edits the
		// block in place: it sets one index byte and clears the counts.
		prefix := append(arrayHead(3), byte(CompositeAlgorithm))
		prefix = appendBytes(prefix, compositeID[:])
		prefix = append(prefix, 0x59, 0x0d, 0x2d)
		if count := bytes.Count(composite, prefix); count != 1 {
			t.Fatalf("the composite record occurs %d times, want once", count)
		}
		hintStart := bytes.Index(composite, prefix) + len(prefix) + 3248
		withHints := func(indices []byte, counts [6]byte) []byte {
			block := make([]byte, 55, 61)
			copy(block, indices)
			block = append(block, counts[:]...)
			derived := bytes.Clone(composite)
			copy(derived[hintStart:], block)
			return derived
		}
		// Counts 2 then 1: the second count is below the index reached by the first.
		compareDerived(t, withHints([]byte{0, 1}, [6]byte{2, 1}), "composite-hint-count-decreasing.bin")
		// Two indices of the first polynomial, 5 then 3: not strictly increasing.
		compareDerived(t, withHints([]byte{5, 3}, [6]byte{2}), "composite-hint-indices-unordered.bin")
		// Every count 0 and the first index byte set to 1, the other index bytes as written: a
		// non-zero byte that no count reaches.
		padding := bytes.Clone(composite)
		padding[hintStart] = 1
		clear(padding[hintStart+55 : hintStart+61])
		compareDerived(t, padding, "composite-hint-padding-nonzero.bin")
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
