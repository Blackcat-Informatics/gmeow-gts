// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"encoding/hex"
	"testing"
)

// brokenUnitDigest is the digest that every broken-link case of the corpus stores in place of a
// blob's unit digest. It is one fixed value, whatever the payload of the frame.
const brokenUnitDigest = "cfe9c934ea816aec8c50db7bae45248dc74c556e21323379a476189b5f055a7e"

// TestAuthBrokenMiddleIsDerivedFromAWrittenSegment rebuilds P6/auth-broken-middle.bin. The writer
// writes the first four frames of auth-two-commits-two-signers.bin with their signatures, the
// commit of the first frame signed by the key of seed 00..1f and the second commit by the keys of
// seed 20..3f and seed 00..1f, then the stored index frame. F3 is then altered after the writing:
// the unit digest in its content is replaced by brokenUnitDigest. Its identifier, and every byte
// after it, is left as written, so the link of F3 is broken.
func TestAuthBrokenMiddleIsDerivedFromAWrittenSegment(t *testing.T) {
	stored := vectorBytes(t, "auth-broken-middle.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 5 {
		t.Fatalf("got %d frames, want 5", len(frames))
	}
	honest := storedFrames(t, vectorBytes(t, "auth-two-commits-two-signers.bin"))
	keyA, keyB := recoveredKeys()
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(honest[0].frameType, honest[0].content, honest[0].payload); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.CommitSigned(keyB); err != nil {
		t.Fatal(err)
	}
	f3 := honest[2]
	if _, err := writer.Append(f3.frameType, f3.content, f3.payload); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.CommitSigned(keyA, keyB); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(frames[4].frameType, frames[4].content, frames[4].payload); err != nil {
		t.Fatal(err)
	}
	broken, err := hex.DecodeString(brokenUnitDigest)
	if err != nil {
		t.Fatal(err)
	}
	unit := hash("gts2 unit", f3.payload)
	written := replaceOnce(t, writer.Bytes(), append([]byte{0x58, 0x20}, unit[:]...), append([]byte{0x58, 0x20}, broken...))
	if !bytes.Equal(written, stored) {
		t.Fatalf("the derived file differs from the stored one:\n got % x\nwant % x", written, stored)
	}
}
