// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestSignatureAddedOtherKeyTrustedIsWrittenByteForByte rebuilds the two files of
// P6/signature-added-other-key-trusted. Each holds a blob, a commit and an index frame, then the
// offsets item and the locator. The commit of the first file is signed by the key of seed 00..1f
// alone; the commit of the second file is signed by the keys of seed 20..3f and seed 00..1f, the
// key of the added signature being the one that is trusted in the read. The writer produces the
// commit, the index frame as stored, and the closing items.
func TestSignatureAddedOtherKeyTrustedIsWrittenByteForByte(t *testing.T) {
	keyA, keyB := recoveredKeys()
	cases := []struct {
		name   string
		signed []ed25519.PrivateKey
	}{
		{"a-added-key-trusted", []ed25519.PrivateKey{keyB}},
		{"b1-added-key-trusted", []ed25519.PrivateKey{keyA, keyB}},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			stored := vectorBytes(t, "signature-added-other-key-trusted."+c.name+".bin")
			frames, closing := storedFramesBeforeClosing(t, stored)
			if len(frames) != 3 {
				t.Fatalf("got %d frames, want 3", len(frames))
			}
			writer, err := NewSegmentWriter(SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
				t.Fatal(err)
			}
			if _, err := writer.CommitSigned(c.signed...); err != nil {
				t.Fatal(err)
			}
			if _, err := writer.Append(frames[2].frameType, frames[2].content, frames[2].payload); err != nil {
				t.Fatal(err)
			}
			if err := writer.WriteOffsets(); err != nil {
				t.Fatal(err)
			}
			if err := writer.WriteLocator(); err != nil {
				t.Fatal(err)
			}
			if got := writer.Bytes(); !bytes.Equal(got, stored) {
				t.Fatalf("the written segment differs from the stored file (closing at %d):\n got % x\nwant % x", closing, got, stored)
			}
		})
	}
}

// TestClosingItemsEndTheSegmentOnce checks that the closing items come after the last frame, once
// each, with the locator last: a segment refuses a locator without an index frame, and once the
// offsets item and the locator are written it refuses every further offsets item, locator, frame
// and commit.
func TestClosingItemsEndTheSegmentOnce(t *testing.T) {
	stored := vectorBytes(t, "signature-added-other-key-trusted.a-added-key-trusted.bin")
	frames, _ := storedFramesBeforeClosing(t, stored)

	bare, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := bare.Commit(); err != nil {
		t.Fatal(err)
	}
	if err := bare.WriteLocator(); err == nil {
		t.Fatal("a locator without an index frame is accepted")
	}

	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Commit(); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(frames[2].frameType, frames[2].content, frames[2].payload); err != nil {
		t.Fatal(err)
	}
	if err := writer.WriteOffsets(); err != nil {
		t.Fatal(err)
	}
	if err := writer.WriteLocator(); err != nil {
		t.Fatal(err)
	}
	if err := writer.WriteOffsets(); err == nil {
		t.Fatal("an offsets item after the locator is accepted")
	}
	if err := writer.WriteLocator(); err == nil {
		t.Fatal("a second locator is accepted")
	}
	if _, err := writer.Append(frames[2].frameType, frames[2].content, frames[2].payload); err == nil {
		t.Fatal("a frame after the closing items is accepted")
	}
	if _, err := writer.Commit(); err == nil {
		t.Fatal("a commit after the closing items is accepted")
	}
}
