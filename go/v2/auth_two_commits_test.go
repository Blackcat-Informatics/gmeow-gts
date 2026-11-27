// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"testing"
)

// TestAuthTwoCommitsTwoSignersIsWrittenByteForByte rebuilds P6/auth-two-commits-two-signers.bin.
// Each frame is written in the order of the stored file. The signatures of each frame are those
// its stored signature list names: the commit of the first frame is signed by the key of seed
// 00..1f, the second commit by the keys of seed 20..3f and seed 00..1f, and the ordinary frame
// after it by the key of seed 00..1f. The other frames carry no signature.
func TestAuthTwoCommitsTwoSignersIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "auth-two-commits-two-signers.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 6 {
		t.Fatalf("got %d frames, want 6", len(frames))
	}
	keyA, keyB := recoveredKeys()
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	appendFrame := func(frame frameSpan) {
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	appendFrame(frames[0])
	if _, err := writer.CommitSigned(keyB); err != nil {
		t.Fatal(err)
	}
	appendFrame(frames[2])
	if _, err := writer.CommitSigned(keyA, keyB); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.AppendSigned(frames[4].frameType, frames[4].content, frames[4].payload, keyB); err != nil {
		t.Fatal(err)
	}
	appendFrame(frames[5])
	if got := writer.Bytes(); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}
