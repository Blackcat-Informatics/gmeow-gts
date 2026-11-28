// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"crypto/mldsa"
	"crypto/sha256"
	"crypto/sha512"
	"encoding/hex"
	"math/big"
	"slices"
	"testing"
)

// TestCraftedSignatureFilesAreDerivedFromWrittenSegments rebuilds 50 P6 input files, each a
// written segment with named edits of its signature record. The Ed25519 files start from
// P6/ed25519-honest.bin, which the writer reproduces in TestEd25519HonestIsWrittenByteForByte: a
// blob and a commit signed by the key of seed 00..1f. The composite files start from
// P6/composite-honest.bin, which the writer reproduces in TestCompositeHonestIsWrittenByteForByte.
// Most edits find their target through a pattern that must occur exactly once. The registry
// version 2 files instead edit header bytes and splice a record onto the end of the segment, each
// guarded by an assertion on the byte it reads.
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
	t.Run("proc-case-7.bin", func(t *testing.T) {
		// Case 7: value[0:32] is the encoding ec, then 30 bytes ff, then 7f: a small-order point
		// as R. The key is the signer's and S is below L, so only the R check fails.
		r := append(append([]byte{0xec}, bytes.Repeat([]byte{0xff}, 30)...), 0x7f)
		edited := append(bytes.Clone(r), value[32:]...)
		derived := replaceOnce(t, honest, append(signatureRecordPrefix(signed), value...), append(signatureRecordPrefix(signed), edited...))
		compareDerived(t, derived, "proc-case-7.bin")
	})
	t.Run("proc-value-length.bin and ed25519-length-63.bin", func(t *testing.T) {
		// Case 4: the value has 63 bytes. The byte-string head declares 63 and the last byte of the
		// value is removed, so the record ends one byte earlier.
		prefix := signatureRecordPrefix(signed)
		short := append(bytes.Clone(prefix[:len(prefix)-1]), 0x3f)
		shortValue := bytes.Clone(value[:63])
		derived := replaceOnce(t, honest, append(bytes.Clone(prefix), value...), append(short, shortValue...))
		compareDerived(t, derived, "proc-value-length.bin")
		compareDerived(t, derived, "ed25519-length-63.bin")
	})
	t.Run("proc-length-before-policy.bin", func(t *testing.T) {
		// A value of 63 zero bytes over the small-order key 01 followed by 31 zero bytes, with its
		// own identifier: value_length is reported before the key's policy is checked.
		identity := make([]byte, ed25519.PublicKeySize)
		identity[0] = 1
		identityID := KeyIdentifier(Ed25519Algorithm, identity)
		honestRecord := append(bytes.Clone(signatureRecordPrefix(signed)), value...)
		shortHead := bytes.Clone(signatureRecordPrefix(identityID))
		shortHead[len(shortHead)-1] = 0x3f
		shortRecord := slices.Concat(shortHead, make([]byte, 63))
		compareDerived(t, replaceOnce(t, honest, honestRecord, shortRecord), "proc-length-before-policy.bin")
	})
	t.Run("key-length-short.bin and proc-lengths-key-first.bin", func(t *testing.T) {
		// A held Ed25519 key of 31 bytes: the first 31 bytes of the signer's public key, under its
		// own identifier. The value is 64 zero bytes in key-length-short, and 63 zero bytes in
		// proc-lengths-key-first, so the key and the value are both short.
		shortKey := key.Public().(ed25519.PublicKey)[:31]
		shortID := KeyIdentifier(Ed25519Algorithm, shortKey)
		honestRecord := append(bytes.Clone(signatureRecordPrefix(signed)), value...)
		keyOnly := append(bytes.Clone(signatureRecordPrefix(shortID)), make([]byte, 64)...)
		compareDerived(t, replaceOnce(t, honest, honestRecord, keyOnly), "key-length-short.bin")
		shortHead := bytes.Clone(signatureRecordPrefix(shortID))
		shortHead[len(shortHead)-1] = 0x3f
		both := slices.Concat(shortHead, make([]byte, 63))
		compareDerived(t, replaceOnce(t, honest, honestRecord, both), "proc-lengths-key-first.bin")
	})
	t.Run("proc-mismatch-before-length.bin", func(t *testing.T) {
		// The signer's record with a value of 10 zero bytes in place of the signature.
		short := encodeSignatureRecord(SignatureRecord{Algorithm: Ed25519Algorithm, KeyID: signed, Value: make([]byte, 10)})
		compareDerived(t, replaceOnce(t, honest, slices.Concat(signatureRecordPrefix(signed), value), short), "proc-mismatch-before-length.bin")
	})
	t.Run("proc-unknown-algorithm.bin, reg-unknown-algorithm-version.bin and proc-unknown-algorithm-long.bin", func(t *testing.T) {
		// The honest segment under a header of registry version 2: the writer starts from the
		// edited header, writes the blob frame and an unsigned commit, and the commit's empty
		// signature list is replaced by one record of algorithm 3 under the signer's key
		// identifier. The value is 64 zero bytes in proc-unknown-algorithm and in
		// reg-unknown-algorithm-version, and 10 zero bytes in proc-unknown-algorithm-long.
		headerEnd, reason := itemEnd(honest, 0)
		if reason != "" {
			t.Fatalf("header: %s", reason)
		}
		header := bytes.Clone(honest[:headerEnd])
		if header[3] != 1 {
			t.Fatalf("the honest header does not name registry version 1: % x", header[:4])
		}
		header[3] = 2
		frames := storedFrames(t, honest)
		for _, file := range []struct {
			name   string
			length int
		}{{"proc-unknown-algorithm.bin", 64}, {"reg-unknown-algorithm-version.bin", 64}, {"proc-unknown-algorithm-long.bin", 10}} {
			writer := segmentWriterFromHeader(bytes.Clone(header), 0)
			if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
				t.Fatal(err)
			}
			if _, err := writer.Commit(); err != nil {
				t.Fatal(err)
			}
			written := writer.Bytes()
			if last := written[len(written)-1]; last != 0x80 {
				t.Fatalf("the segment does not end with the empty signature list of its unsigned commit: % x", last)
			}
			record := encodeSignatureRecord(SignatureRecord{Algorithm: 3, KeyID: signed, Value: make([]byte, file.length)})
			compareDerived(t, slices.Concat(written[:len(written)-1], arrayHead(1), record), file.name)
		}
	})
	t.Run("proc-no-hash.bin", func(t *testing.T) {
		// The honest segment under a header of registry version 2 and hash_algorithm 9. The writer's
		// blob frame under that header is the stored frame; the stored file's signature list holds one
		// Ed25519 record of the signer's key identifier with a value of 64 zero bytes, and no commit follows.
		headerEnd, reason := itemEnd(honest, 0)
		if reason != "" {
			t.Fatalf("header: %s", reason)
		}
		header := bytes.Clone(honest[:headerEnd])
		if header[3] != 1 || header[4] != 1 {
			t.Fatalf("the honest header does not name registry version 1 and hash algorithm 1: % x", header[:5])
		}
		header[3] = 2
		header[4] = 9
		frames := storedFrames(t, honest)
		writer := segmentWriterFromHeader(header, 0)
		if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
			t.Fatal(err)
		}
		written := writer.Bytes()
		if last := written[len(written)-1]; last != 0x80 {
			t.Fatalf("the frame does not end with the empty signature list: % x", last)
		}
		record := encodeSignatureRecord(SignatureRecord{Algorithm: Ed25519Algorithm, KeyID: signed, Value: make([]byte, 64)})
		compareDerived(t, slices.Concat(written[:len(written)-1], arrayHead(1), record), "proc-no-hash.bin")
	})
	t.Run("proc-opaque-frame.bin", func(t *testing.T) {
		// Two signed frames under the registry version 2 header of proc-no-hash: a frame of the unknown
		// non-critical type 99 with empty content, and a critical quads frame whose 14-byte payload is
		// malformed. Both are signed by the signer's key. The writer refuses to write type 99, so the
		// first frame is written by the writer's append, which signs it as AppendSignedWith does.
		headerEnd, reason := itemEnd(honest, 0)
		if reason != "" {
			t.Fatalf("header: %s", reason)
		}
		header := bytes.Clone(honest[:headerEnd])
		if header[3] != 1 || header[4] != 1 {
			t.Fatalf("the honest header does not name registry version 1 and hash algorithm 1: % x", header[:5])
		}
		header[3] = 2
		signer, err := NewEd25519Key(key)
		if err != nil {
			t.Fatal(err)
		}
		writer := segmentWriterFromHeader(header, 0)
		if _, err := writer.append(99, []byte{0x80}, nil, writer.signer([]SigningKey{signer})); err != nil {
			t.Fatal(err)
		}
		malformed := []byte{0x84, 0x81, 0x82, 0x81, 0x01, 0x01, 0x81, 0x82, 0x00, 0x01, 0x81, 0x00, 0x81, 0x00}
		if _, err := writer.AppendSignedWith(2, []byte{0x82, 0x02, 0x05}, malformed, signer); err != nil {
			t.Fatal(err)
		}
		compareDerived(t, writer.Bytes(), "proc-opaque-frame.bin")
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
	t.Run("composite-label-wrong.bin", func(t *testing.T) {
		// M' with another label: both halves sign the representative under the label
		// COMPSIG-MLDSA65-Ed25519-SHA256. The ML-DSA half keeps the composite label as its context.
		message, _ := honestCompositeCommitMessage(t, compositeKey)
		rep := compositeRepresentativeWithLabel(message, "COMPSIG-MLDSA65-Ed25519-SHA256")
		ml, err := compositeKey.ml.SignDeterministic(rep, &mldsa.Options{Context: string(compositeLabel)})
		if err != nil {
			t.Fatal(err)
		}
		value := slices.Concat(ml, ed25519.Sign(compositeKey.ed, rep))
		prefix := append(arrayHead(3), byte(CompositeAlgorithm))
		prefix = appendBytes(prefix, compositeID[:])
		prefix = append(prefix, 0x59, 0x0d, 0x2d)
		honestValue := composite[bytes.Index(composite, prefix)+len(prefix):][:len(value)]
		compareDerived(t, replaceOnce(t, composite, honestValue, value), "composite-label-wrong.bin")
	})
	// compositeWithRepresentative replaces the composite value of the honest file with the halves
	// made over rep: the ML-DSA half under mlContext, and the Ed25519 half over rep.
	compositeWithRepresentative := func(t *testing.T, rep []byte, mlContext string, name string) {
		t.Helper()
		ml, err := compositeKey.ml.SignDeterministic(rep, &mldsa.Options{Context: mlContext})
		if err != nil {
			t.Fatal(err)
		}
		value := slices.Concat(ml, ed25519.Sign(compositeKey.ed, rep))
		prefix := append(arrayHead(3), byte(CompositeAlgorithm))
		prefix = appendBytes(prefix, compositeID[:])
		prefix = append(prefix, 0x59, 0x0d, 0x2d)
		honestValue := composite[bytes.Index(composite, prefix)+len(prefix):][:len(value)]
		compareDerived(t, replaceOnce(t, composite, honestValue, value), name)
	}
	t.Run("composite-context-nonempty.bin", func(t *testing.T) {
		// M' with len(ctx) 1 and the one-byte context 0x78: the representative carries a non-empty
		// context, and the ML-DSA half keeps the composite label as its context.
		message, _ := honestCompositeCommitMessage(t, compositeKey)
		digest := sha512.Sum512(message[:])
		rep := append(append(append([]byte{}, compositePrefix...), compositeLabel...), 1, 0x78)
		rep = append(rep, digest[:]...)
		compositeWithRepresentative(t, rep, string(compositeLabel), "composite-context-nonempty.bin")
	})
	t.Run("composite-prehash-wrong.bin", func(t *testing.T) {
		// M' with SHA-256 as the pre-hash: the representative holds SHA-256 of the message in place
		// of SHA-512.
		message, _ := honestCompositeCommitMessage(t, compositeKey)
		digest := sha256.Sum256(message[:])
		rep := append(append(append([]byte{}, compositePrefix...), compositeLabel...), 0)
		rep = append(rep, digest[:]...)
		compositeWithRepresentative(t, rep, string(compositeLabel), "composite-prehash-wrong.bin")
	})
	// The composite value starts after the record head, at offset 281 of the honest file: its
	// ML-DSA-65 half is the first 3309 bytes and its Ed25519 half the last 64.
	compositeValueStart := func(t *testing.T) int {
		t.Helper()
		prefix := append(arrayHead(3), byte(CompositeAlgorithm))
		prefix = appendBytes(prefix, compositeID[:])
		prefix = append(prefix, 0x59, 0x0d, 0x2d)
		at := bytes.Index(composite, prefix)
		if at < 0 || bytes.Count(composite, prefix) != 1 {
			t.Fatal("the composite record does not occur exactly once")
		}
		return at + len(prefix)
	}
	t.Run("proc-case-11.bin, proc-case-12.bin and proc-case-13.bin", func(t *testing.T) {
		// Case 11: one bit of the last 64 bytes is changed, the lowest bit of the fourth byte of
		// the Ed25519 half. Case 12: one bit of the ML-DSA-65 half, the lowest bit of its byte 100.
		// Case 13: both changes together.
		start := compositeValueStart(t)
		flip := func(data []byte, at int) []byte {
			out := bytes.Clone(data)
			out[at] ^= 0x01
			return out
		}
		edHalf := start + 3309 + 3
		mlHalf := start + 100
		compareDerived(t, flip(composite, edHalf), "proc-case-11.bin")
		compareDerived(t, flip(composite, mlHalf), "proc-case-12.bin")
		compareDerived(t, flip(flip(composite, edHalf), mlHalf), "proc-case-13.bin")
	})
	t.Run("proc-case-14.bin", func(t *testing.T) {
		// Case 14: the composite value has 3372 bytes. The record head declares that length, and
		// the last byte of the value is removed.
		start := compositeValueStart(t)
		out := bytes.Clone(composite)
		out[start-1] = 0x2c
		compareDerived(t, append(out[:start+3372], out[start+3373:]...), "proc-case-14.bin")
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

// honestCompositeCommitMessage returns the signed message of the honest composite commit: the
// message that the composite signature of P6/composite-honest.bin is made over.
func honestCompositeCommitMessage(t *testing.T, key CompositeKey) (Digest, Digest) {
	t.Helper()
	frames := storedFrames(t, vectorBytes(t, "composite-honest.bin"))
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
		t.Fatal(err)
	}
	commit, err := writer.CommitSignedWith(key)
	if err != nil {
		t.Fatal(err)
	}
	record := SignatureRecord{Algorithm: CompositeAlgorithm, KeyID: KeyIdentifier(CompositeAlgorithm, key.PublicKey())}
	return SignedMessage(record, writer.opener, commit), commit
}

// compositeRepresentativeWithLabel is M' of P6 section 4.2 under the given label, with the empty
// context: the prefix, the label, a zero length and the SHA-512 digest of the message.
func compositeRepresentativeWithLabel(message Digest, label string) []byte {
	digest := sha512.Sum512(message[:])
	out := append(append([]byte{}, compositePrefix...), label...)
	out = append(out, 0)
	return append(out, digest[:]...)
}
