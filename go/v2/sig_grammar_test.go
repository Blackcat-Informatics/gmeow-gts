// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"testing"
)

// TestSigGrammarFilesAreFramesAroundTheirEntries rebuilds six P6 files whose blob frame carries a
// signatures element that is not a valid list of records (their INDEX rows: the signatures element
// is the bytes of the entry). Each file is the segment that the writer writes for the blob frame of
// P6/ed25519-honest.bin with an empty signatures element, with that element replaced by the entry's
// bytes. The entry bytes are authored data that the corpus records; the test rebuilds the frame
// around them, and the writer's identifier and header are checked against the stored file.
func TestSigGrammarFilesAreFramesAroundTheirEntries(t *testing.T) {
	frames := storedFrames(t, vectorBytes(t, "ed25519-honest.bin"))
	if len(frames) != 2 {
		t.Fatalf("got %d frames, want 2", len(frames))
	}
	blob := frames[0]
	for _, name := range []string{
		"sig-grammar-two-fields.bin",
		"sig-grammar-key-id-text.bin",
		"sig-grammar-key-id-short.bin",
		"sig-grammar-value-text.bin",
		"sig-grammar-long-head.bin",
		"sig-grammar-not-array.bin",
	} {
		t.Run(name, func(t *testing.T) {
			stored := vectorBytes(t, name)
			plain, err := NewSegmentWriter(SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			if _, err := plain.Append(blob.frameType, blob.content, blob.payload); err != nil {
				t.Fatal(err)
			}
			plainBytes := plain.Bytes()
			// The plain frame ends with the empty signatures element, one byte 0x80.
			if !bytes.HasPrefix(stored, plainBytes[:len(plainBytes)-1]) {
				t.Fatal("the stored file does not begin with the plain blob segment")
			}
			entry := stored[len(plainBytes)-1:]
			writer, err := NewSegmentWriter(SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			if _, err := writer.append(blob.frameType, blob.content, blob.payload, func(Digest) ([]byte, error) { return entry, nil }); err != nil {
				t.Fatal(err)
			}
			if got := writer.Bytes(); !bytes.Equal(got, stored) {
				t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
			}
		})
	}
}
