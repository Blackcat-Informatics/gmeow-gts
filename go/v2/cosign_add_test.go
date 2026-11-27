// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"testing"
)

// TestCosignAddSignatureIsWrittenByteForByte rebuilds P6/cosign-add-signature.bin: the ordinary
// frames of the stored file, then one commit signed by the keys of seed 20..3f and seed 00..1f.
// The commit carries two signatures, the one the first commit of the file held and a second one
// added by the same frame.
func TestCosignAddSignatureIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "cosign-add-signature.bin")
	frames := storedFrames(t, stored)
	if len(frames) < 2 {
		t.Fatalf("got %d frames, want at least 2", len(frames))
	}
	keyA, keyB := recoveredKeys()
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	for _, frame := range frames[:len(frames)-1] {
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := writer.CommitSigned(keyA, keyB); err != nil {
		t.Fatal(err)
	}
	if got := writer.Bytes(); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}
