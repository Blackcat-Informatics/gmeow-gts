// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestSubjectRelabelKeyIsDerivedFromASignedFrame rebuilds P6/subject-relabel-key.bin from
// P6/auth-all.bin. That file is the same bytes as P6/ed25519-honest.bin, which the writer
// reproduces in TestEd25519HonestIsWrittenByteForByte. The one signature record of the file names
// the key of seed 00..1f; the relabelled file names the key of seed 20..3f in its place, the other
// held key. The record's signature value is left as written. The edit refuses an input where the
// key identifier does not occur exactly once, after its byte-string head.
func TestSubjectRelabelKeyIsDerivedFromASignedFrame(t *testing.T) {
	stored := vectorBytes(t, "subject-relabel-key.bin")
	source := vectorBytes(t, "auth-all.bin")
	keyA, keyB := recoveredKeys()
	signed := KeyIdentifier(1, keyB.Public().(ed25519.PublicKey))
	relabelled := KeyIdentifier(1, keyA.Public().(ed25519.PublicKey))
	written := replaceOnce(t, source, append([]byte{0x58, 0x20}, signed[:]...), append([]byte{0x58, 0x20}, relabelled[:]...))
	if !bytes.Equal(written, stored) {
		t.Fatalf("the derived file differs from the stored one:\n got % x\nwant % x", written, stored)
	}
}
