// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"testing"
)

// TestCosignRemoveSecondRemovedIsWrittenByteForByte rebuilds
// P6/cosign-remove-signature.second-removed.bin: the ordinary frames of the stored file, then one
// commit signed by the key of seed 20..3f alone. The commit has one signature, the copy from
// which the second signature of the two-signature commit was removed.
func TestCosignRemoveSecondRemovedIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "cosign-remove-signature.second-removed.bin")
	frames := storedFrames(t, stored)
	if len(frames) < 2 {
		t.Fatalf("got %d frames, want at least 2", len(frames))
	}
	keyA, _ := recoveredKeys()
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	for _, frame := range frames[:len(frames)-1] {
		if frame.frameType == commitType {
			t.Fatalf("an ordinary frame of type %d is a commit", frame.frameType)
		}
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := writer.CommitSigned(keyA); err != nil {
		t.Fatal(err)
	}
	if got := writer.Bytes(); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}
