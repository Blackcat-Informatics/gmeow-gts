// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"encoding/hex"
	"testing"
)

// TestSigGrammarFilesAreFramesAroundTheirEntries rebuilds eleven P6 files whose blob frame carries a
// signatures element that is not a valid list of records (their INDEX rows: the signatures element
// is the bytes of the entry). Each file is the segment that the writer writes for the blob frame of
// P6/ed25519-honest.bin with an empty signatures element, with that element replaced by the entry
// written here as hex. The writer's header, identifier and frame layout must equal the stored file,
// so an entry that is changed, truncated or extended fails the comparison.
func TestSigGrammarFilesAreFramesAroundTheirEntries(t *testing.T) {
	frames := storedFrames(t, vectorBytes(t, "ed25519-honest.bin"))
	if len(frames) != 2 {
		t.Fatalf("got %d frames, want 2", len(frames))
	}
	blob := frames[0]
	entries := map[string]string{
		"sig-grammar-two-fields.bin":             "8182015820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
		"sig-grammar-key-id-text.bin":            "818301616b584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
		"sig-grammar-key-id-short.bin":           "818301581f00000000000000000000000000000000000000000000000000000000000000584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
		"sig-grammar-value-text.bin":             "8183015820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f6176",
		"sig-grammar-long-head.bin":              "819803015820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
		"sig-grammar-not-array.bin":              "4178",
		"sig-grammar-element-not-array.bin":      "8105",
		"sig-grammar-four-fields.bin":            "8184015820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f58400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
		"sig-grammar-algorithm-text.bin":         "818361615820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
		"sig-grammar-algorithm-zero.bin":         "8183005820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
		"sig-grammar-algorithm-unregistered.bin": "8183035820000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
	}
	for name, encoded := range entries {
		t.Run(name, func(t *testing.T) {
			entry, err := hex.DecodeString(encoded)
			if err != nil {
				t.Fatal(err)
			}
			writer, err := NewSegmentWriter(SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			if _, err := writer.append(blob.frameType, blob.content, blob.payload, func(Digest) ([]byte, error) { return entry, nil }); err != nil {
				t.Fatal(err)
			}
			stored := vectorBytes(t, name)
			if got := writer.Bytes(); !bytes.Equal(got, stored) {
				t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
			}
		})
	}
}
