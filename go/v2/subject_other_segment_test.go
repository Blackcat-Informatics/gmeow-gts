// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestSubjectOtherSegmentIsDerivedFromTwoWrittenSegments rebuilds P6/subject-other-segment.bin.
// The file is two segments. The first is a header, two ordinary frames and a commit signed by the
// key of seed 01..01; the writer writes it. A resync opener then begins the second segment at the
// offset where it lands, with lineage present (P1-85): its previous_head is the head of the first
// segment and its ordinal is 1. The second segment repeats the frames of the first with the same
// content and payloads. The writer writes those frames and an unsigned commit. The commit's
// signature list is then replaced by the list that the stored file holds, which is the signature
// of the first segment's commit: it is valid over that commit's identifier and invalid over the
// second segment's, so the edit is the case. The edit refuses an input whose last byte is not the
// empty signature list of the unsigned commit.
func TestSubjectOtherSegmentIsDerivedFromTwoWrittenSegments(t *testing.T) {
	stored := vectorBytes(t, "subject-other-segment.bin")
	headerEnd, reason := itemEnd(stored, 0)
	if reason != "" {
		t.Fatalf("header: %s", reason)
	}
	first, firstEnd := storedFramesFrom(t, stored, headerEnd)
	if len(first) != 3 {
		t.Fatalf("the first segment has %d frames, want 3", len(first))
	}
	resyncEnd, reason := itemEnd(stored, firstEnd)
	if reason != "" {
		t.Fatalf("resync: %s", reason)
	}
	second, secondEnd := storedFramesFrom(t, stored, resyncEnd)
	if len(second) != 3 || secondEnd != len(stored) {
		t.Fatalf("the second segment has %d frames ending at %d, want 3 ending at %d", len(second), secondEnd, len(stored))
	}
	signer := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{0x01}, ed25519.SeedSize))
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	for _, frame := range first[:2] {
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := writer.CommitSigned(signer); err != nil {
		t.Fatal(err)
	}
	if err := writer.Resync(); err != nil {
		t.Fatal(err)
	}
	for _, frame := range second[:2] {
		if _, err := writer.Append(frame.frameType, frame.content, frame.payload); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := writer.Commit(); err != nil {
		t.Fatal(err)
	}
	written := writer.Bytes()
	if last := written[len(written)-1]; last != 0x80 {
		t.Fatalf("the segment does not end with the empty signature list of its unsigned commit: % x", last)
	}
	edited := append(bytes.Clone(written[:len(written)-1]), first[2].signatures...)
	if !bytes.Equal(edited, stored) {
		t.Fatalf("the derived file differs from the stored one:\n got % x\nwant % x", edited, stored)
	}
}
