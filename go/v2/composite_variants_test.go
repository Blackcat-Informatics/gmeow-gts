// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"crypto/mldsa"
	"crypto/sha256"
	"testing"
)

// TestCompositeVariantFilesAreDerivedFromTheCompositeSigning rebuilds three P6 composite inputs
// from P6/composite-honest.bin, the commit the writer reproduces in TestCompositeHonestIsWrittenByteForByte.
// The composite key is the one of seed 40..7f. Each file changes one half or the representative
// that the halves sign:
//   - composite-prehash-wrong: both halves sign the representative with SHA-256 of the subject in
//     place of the SHA-512 pre-hash PH that the honest representative names.
//   - composite-mldsa-context: the ML-DSA-65 half is made with an empty context; the Ed25519 half
//     is unchanged.
//   - composite-stripped-half: the ML-DSA-65 half is all zero bytes; the Ed25519 half is unchanged.
//
// Each edit refuses an input in which the composite record does not occur exactly once.
func TestCompositeVariantFilesAreDerivedFromTheCompositeSigning(t *testing.T) {
	seed := make([]byte, 64)
	for i := range seed {
		seed[i] = byte(0x40 + i)
	}
	key, err := NewCompositeKey(seed)
	if err != nil {
		t.Fatal(err)
	}
	honest := vectorBytes(t, "composite-honest.bin")
	frames := storedFrames(t, honest)
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
	subject := SignedMessage(record, writer.opener, commit)
	prefix := compositeRecordPrefix(record.KeyID)
	if count := bytes.Count(honest, prefix); count != 1 {
		t.Fatalf("the composite record occurs %d times, want once", count)
	}
	valueStart := bytes.Index(honest, prefix) + len(prefix) + 3
	value := honest[valueStart : valueStart+compositeValueLength]
	edHalf := value[mldsa65ValueLength:]

	// The honest signature's representative, and the one with SHA-256 in place of SHA-512.
	label := append(append(append([]byte{}, compositePrefix...), compositeLabel...), 0)
	digest := sha256.Sum256(subject[:])
	prehash := append(bytes.Clone(label), digest[:]...)

	mlPrehash, err := key.ml.SignDeterministic(prehash, &mldsa.Options{Context: string(compositeLabel)})
	if err != nil {
		t.Fatal(err)
	}
	edPrehash := ed25519.Sign(key.ed, prehash)
	prehashed := splice(honest, valueStart, append(mlPrehash, edPrehash...))
	compareDerivedComposite(t, prehashed, "composite-prehash-wrong.bin")

	mlNoContext, err := key.ml.SignDeterministic(compositeRepresentative(subject[:]), &mldsa.Options{Context: ""})
	if err != nil {
		t.Fatal(err)
	}
	noContext := splice(honest, valueStart, append(mlNoContext, edHalf...))
	compareDerivedComposite(t, noContext, "composite-mldsa-context.bin")

	stripped := splice(honest, valueStart, append(make([]byte, mldsa65ValueLength), edHalf...))
	compareDerivedComposite(t, stripped, "composite-stripped-half.bin")
}

// compositeRecordPrefix returns the encoded head of a composite signature record: its algorithm and
// key identifier, before the value.
func compositeRecordPrefix(keyID Digest) []byte {
	out := arrayHead(3)
	out = appendUint(out, CompositeAlgorithm)
	return appendBytes(out, keyID[:])
}

// compareDerivedComposite fails the test unless derived is the stored P6 file name.
func compareDerivedComposite(t *testing.T, derived []byte, name string) {
	t.Helper()
	if stored := vectorBytes(t, name); !bytes.Equal(derived, stored) {
		t.Fatalf("the derived file differs from P6/%s:\n got % x\nwant % x", name, derived, stored)
	}
}

// splice returns a copy of data with the bytes at start replaced by replacement, which has the same length.
func splice(data []byte, start int, replacement []byte) []byte {
	out := bytes.Clone(data)
	copy(out[start:], replacement)
	return out
}
