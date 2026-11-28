// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"testing"
)

// TestCompositeHonestIsWrittenByteForByte rebuilds P6/composite-honest.bin: an unsigned blob, then
// a commit signed by the composite key of the 64-byte seed 40..7f. The composite key identifier
// is 84df8ff5..., the key that the corpus names for that seed. The writer's bytes must equal the
// stored file.
func TestCompositeHonestIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "composite-honest.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 2 {
		t.Fatalf("got %d frames, want 2", len(frames))
	}
	seed := make([]byte, 64)
	for i := range seed {
		seed[i] = byte(0x40 + i)
	}
	key, err := NewCompositeKey(seed)
	if err != nil {
		t.Fatal(err)
	}
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(frames[0].frameType, frames[0].content, frames[0].payload); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.CommitSignedWith(key); err != nil {
		t.Fatal(err)
	}
	if got := writer.Bytes(); !bytes.Equal(got, stored) {
		t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
	}
}
