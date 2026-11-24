// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"go.blackcatinformatics.ca/gts/v2"
)

func TestSegmentWriterReproducesCommittedCorpusSegments(t *testing.T) {
	for _, name := range []string{
		"P5/commit-valid.bin",
		"P5/range-transparent.no-parity.bin",
		"P5/range-transparent.with-parity.bin",
		"P3/state-prefix-fold.bin",
	} {
		t.Run(name, func(t *testing.T) {
			data, err := os.ReadFile(filepath.Join("..", "..", "vectors", "v2", name))
			if err != nil {
				t.Fatal(err)
			}
			result := v2.ReadWire(data, v2.DefaultReadCapacity())
			if result.Stop != "" || result.Dispatch != v2.SecondGeneration || len(result.Segments) == 0 {
				t.Fatalf("invalid source vector: %s at %d", result.Stop, result.StopOffset)
			}
			for _, segment := range result.Segments {
				writer, err := v2.NewSegmentWriter(v2.SegmentOptions{
					Profile: segment.Opener.Profile, Canonical: segment.Opener.Canonical == 1,
					Lineage: segment.Opener.Lineage,
				})
				if err != nil {
					t.Fatal(err)
				}
				for _, item := range segment.Items[1:] {
					frame := item.Frame
					if frame == nil || frame.Payload.State != v2.Verified || frame.StoredID == nil {
						t.Fatalf("unexpected fixture item at byte %d", item.Offset)
					}
					previous := writer.Head()
					var id v2.Digest
					if frame.Type == 14 {
						id, err = writer.Commit()
					} else {
						id, err = writer.Append(frame.Type, frame.Content, frame.Payload.Decoded)
					}
					if err != nil {
						t.Fatal(err)
					}
					if id != *frame.StoredID {
						t.Fatalf("identifier at byte %d = %x, want %x", item.Offset, id, *frame.StoredID)
					}
					if frame.Transparent && writer.Head() != previous {
						t.Fatalf("transparent frame at byte %d advanced the head", item.Offset)
					}
				}
				items := segment.Items
				if !bytes.Equal(writer.Bytes(), data[items[0].Offset:items[len(items)-1].End]) {
					t.Fatal("the generated segment differs from the published bytes")
				}
			}
		})
	}
}

func TestSegmentWriterRejectsBadInputWithoutChangingPrefix(t *testing.T) {
	writer, err := v2.NewSegmentWriter(v2.SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	before := writer.Bytes()
	head := writer.Head()
	for _, rejected := range []struct {
		kind    uint64
		content []byte
	}{
		{2, []byte{0x82, 2}},
		{2, []byte{0x82, 2, 1, 0}},
		{2, []byte{0x82, 2, 0xa0}},
		{2, []byte{0x82, 0x18, 2, 1}},
		{2, []byte{0x81, 2}},
		{14, []byte{0x80}},
		{17, []byte{0x80}},
	} {
		if _, err := writer.Append(rejected.kind, rejected.content, nil); err == nil {
			t.Fatalf("accepted type %d and content %x", rejected.kind, rejected.content)
		}
		if writer.Head() != head || !bytes.Equal(writer.Bytes(), before) {
			t.Fatal("a refused append changed the segment")
		}
	}
	if _, err := writer.Commit(); err != nil {
		t.Fatal(err)
	}
}
