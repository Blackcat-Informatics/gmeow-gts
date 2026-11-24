// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"encoding/hex"
	"os"
	"path/filepath"
	"slices"
	"testing"

	"go.blackcatinformatics.ca/gts/v2"
)

func vector(t *testing.T, name string) []byte {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("..", "..", "vectors", "v2", "P1", name+".bin"))
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func TestInspectWireChecksCompressedPayloadWithoutRetainingIt(t *testing.T) {
	data := vector(t, "zstd-ok")
	capacity := v2.DefaultReadCapacity()
	inspected := v2.InspectWire(data, capacity).Segments[0].Items[1].Frame.Payload
	read := v2.ReadWire(data, capacity).Segments[0].Items[1].Frame.Payload
	if inspected.State != v2.Verified || inspected.Decoded != nil {
		t.Fatalf("inspection payload = %+v", inspected)
	}
	if read.State != v2.Verified || len(read.Decoded) == 0 {
		t.Fatalf("reader payload = %+v", read)
	}
}

func TestCommitClaimsRequireContentWithinStoredCapacity(t *testing.T) {
	path := filepath.Join("..", "..", "vectors", "v2", "P5", "commit-root-mismatch.bin")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	capacity := v2.DefaultReadCapacity()
	segment := v2.InspectWire(data, capacity).Segments[0]
	check := segment.CheckCommits()
	if len(check.Mismatches) != 1 || !check.Mismatches[0].RootMismatch {
		t.Fatalf("readable commit: %+v", check)
	}
	for _, item := range segment.Items {
		if item.Offset == check.Mismatches[0].Offset {
			capacity.StoredBytesPerFrame = uint64(len(item.Frame.Content)) - 1
			break
		}
	}
	check = v2.InspectWire(data, capacity).Segments[0].CheckCommits()
	if len(check.Mismatches) != 0 || check.CommittedFrames != 0 {
		t.Fatalf("commit with content beyond capacity: %+v", check)
	}
}

func TestDispatchUsesFirstTypedGeneration(t *testing.T) {
	for _, tc := range []struct {
		name       string
		dispatch   v2.Dispatch
		generation uint64
	}{
		{"gen-first-generation", v2.FirstGeneration, 1},
		{"gen-first-generation-tagged", v2.FirstGeneration, 1},
		{"gen-first-generation-v-twice", v2.NotThisFormat, 0},
		{"gen-first-generation-two", v2.UnsupportedGeneration, 2},
		{"gen-record-one", v2.NotThisFormat, 1},
		{"gen-record-two", v2.SecondGeneration, 2},
		{"gen-record-three", v2.UnsupportedGeneration, 3},
		{"header-long-form-first-item", v2.SecondGeneration, 2},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got := v2.ReadWire(vector(t, tc.name), v2.DefaultReadCapacity())
			if got.Dispatch != tc.dispatch || got.Generation != tc.generation {
				t.Fatalf("dispatch %q, generation %d", got.Dispatch, got.Generation)
			}
			if tc.dispatch != v2.SecondGeneration && len(got.Segments) != 0 {
				t.Fatal("dispatch continued past the first item")
			}
		})
	}

	got := v2.ReadWire(vector(t, "header-long-form-first-item"), v2.DefaultReadCapacity())
	if segment := got.Segments[0]; segment.Items[0].Reason != v2.NotDeterministic || segment.Head != nil {
		t.Fatalf("long-form header was handled as %+v", segment)
	}
}

func TestOpenerHashesAndSegmentIsolation(t *testing.T) {
	header := v2.ReadWire(vector(t, "gen-record-two"), v2.DefaultReadCapacity())
	id := header.Segments[0].Opener.ID
	const want = "e9dc54662899695160ab279ede7b16ce9807b2d696e097645943288916e3fedc"
	if id == nil || hex.EncodeToString(id[:]) != want {
		t.Fatalf("header identifier = %x, want %s", id, want)
	}

	result := v2.ReadWire(vector(t, "resync-displaced-join"), v2.DefaultReadCapacity())
	if len(result.Segments) != 2 || result.Stop != "" {
		t.Fatalf("displaced join = %+v", result)
	}
	segment := result.Segments[1]
	if segment.Opener.Resync == nil || !segment.Opener.Resync.Displaced || segment.Opener.ID == nil {
		t.Fatalf("displaced resync opener = %+v", segment.Opener)
	}
	if got := links(segment); len(got) == 0 || slices.ContainsFunc(got, func(link v2.Link) bool {
		return link != v2.Linked
	}) {
		t.Fatalf("resync segment links = %v", got)
	}
	for _, tc := range []struct {
		name   string
		reason v2.Reason
	}{
		{"resync-too-long.walked", v2.OutOfRange},
		{"resync-too-long.exactly-4096", ""},
	} {
		framed := v2.ReadWire(vector(t, tc.name), v2.DefaultReadCapacity())
		if item := framed.Segments[len(framed.Segments)-1].Items[0]; item.Reason != tc.reason {
			t.Fatalf("%s: resync opener = %+v", tc.name, item)
		}
	}

	unavailable := v2.ReadWire(vector(t, "header-hash-newer"), v2.DefaultReadCapacity()).Segments[0]
	if unavailable.Head != nil || unavailable.Items[0].Reason != "" || unavailable.Opener.HashAlgorithm == 1 {
		t.Fatalf("opener with unavailable hash algorithm = %+v", unavailable)
	}
	for _, item := range unavailable.Items[1:] {
		if item.Kind != v2.UncheckedItem || item.Frame != nil {
			t.Fatalf("judged an item under an unavailable hash algorithm: %+v", item)
		}
	}
}

func TestResyncSizeChecksNestedOptionFieldStarts(t *testing.T) {
	prefix, err := hex.DecodeString("8b050250fcf3eb32a2f0267dd16a53f09bff879202098101820282")
	if err != nil {
		t.Fatal(err)
	}
	for _, tc := range []struct {
		size   int
		past   []byte
		reason v2.Reason
	}{
		{4061, []byte{0x82, 2, 0}, v2.WrongType},
		{4062, []byte{0x82, 2, 0}, v2.OutOfRange},
		{4062, []byte{0x81, 0}, v2.UnregisteredValue},
		{4063, []byte{0x81, 0}, v2.OutOfRange},
		{4061, []byte{0x82, 2, 0x18, 0}, v2.NotDeterministic},
	} {
		data := slices.Concat(
			prefix, []byte{0x59, byte(tc.size >> 8), byte(tc.size)},
			make([]byte, tc.size), []byte{1, 0}, tc.past, []byte{0x80, 0x40},
		)
		wire := v2.InspectWire(data, v2.DefaultReadCapacity())
		if wire.Stop != "" || len(wire.Segments) != 1 {
			t.Fatalf("size %d and past %x: unexpected framing: %+v", tc.size, tc.past, wire)
		}
		opener := wire.Segments[0].Items[0]
		if opener.Kind != v2.ResyncItem || opener.Reason != tc.reason {
			t.Fatalf("size %d and past %x: got %+v, want %s", tc.size, tc.past, opener, tc.reason)
		}
	}
}

func TestFramingKeepsCompleteItemsAndDistinguishesFailure(t *testing.T) {
	for _, tc := range []struct {
		name string
		stop v2.Reason
	}{
		{"file-first-incomplete", v2.TornTail},
		{"file-first-undelimitable", v2.UndelimitableInput},
	} {
		got := v2.ReadWire(vector(t, tc.name), v2.DefaultReadCapacity())
		if got.Stop != tc.stop || got.StopOffset != 0 || len(got.Segments) != 0 {
			t.Fatalf("%s: first-item framing = %+v", tc.name, got)
		}
	}

	base := vector(t, "link-ok")
	for _, tc := range []struct {
		tail []byte
		stop v2.Reason
	}{
		{[]byte{0x58}, v2.TornTail},
		{[]byte{0xbf, 0, 0xff}, v2.UndelimitableInput},
		{[]byte{0x5f, 0x78}, v2.UndelimitableInput},
		{[]byte{0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}, ""},
	} {
		got := v2.ReadWire(append(slices.Clone(base), tc.tail...), v2.DefaultReadCapacity())
		if got.Stop != tc.stop || (tc.stop != "" && got.StopOffset != len(base)) {
			t.Fatalf("tail %x: stopped at %d with %q", tc.tail, got.StopOffset, got.Stop)
		}
		items := got.Segments[0].Items
		if tc.stop == "" {
			if last := items[len(items)-1]; last.Kind != v2.OpaqueItem || last.End != len(base)+len(tc.tail) {
				t.Fatalf("tail %x: delimited item = %+v", tc.tail, last)
			}
		} else if last := items[len(items)-1]; last.End != len(base) {
			t.Fatalf("tail %x: last complete item = %+v", tc.tail, last)
		}
	}
}

func TestNonminimalFrameArrayHeadIsOpaque(t *testing.T) {
	got := v2.ReadWire(vector(t, "frame-test-array-long-form"), v2.DefaultReadCapacity())
	segment := got.Segments[0]
	items := segment.Items
	if len(items) != 2 || items[1].Kind != v2.OpaqueItem ||
		items[1].Reason != v2.NotDeterministic || items[1].Frame != nil {
		t.Fatalf("long-form frame item was not opaque: %+v", items)
	}
	if segment.Head == nil || segment.Opener.ID == nil || *segment.Head != *segment.Opener.ID {
		t.Fatalf("opaque item changed the segment head: %+v", segment)
	}
}

func links(segment v2.Segment) []v2.Link {
	var states []v2.Link
	for _, item := range segment.Items {
		if item.Frame != nil {
			states = append(states, item.Frame.Link)
		}
	}
	return states
}

func TestLinkHealingRequiresTheImmediatelyFollowingEnvelope(t *testing.T) {
	capacity := v2.DefaultReadCapacity()
	input := vector(t, "link-healed-before-transparent")
	segment := v2.ReadWire(input, capacity).Segments[0]
	if got := links(segment); !slices.Equal(got, []v2.Link{v2.Healed, v2.Linked}) {
		t.Fatalf("links with transparent witness = %v", got)
	}
	first, transparent := segment.Items[1].Frame, segment.Items[2].Frame
	if first.ComputedID == nil || segment.Head == nil || *segment.Head != *first.ComputedID ||
		!transparent.Transparent || transparent.EffectiveID != nil {
		t.Fatalf("transparent witness changed predecessor: %#v, head %x", transparent, segment.Head)
	}

	opaque := v2.ReadWire(vector(t, "link-heal-opaque-between"), capacity).Segments[0]
	if got := links(opaque); !slices.Equal(got, []v2.Link{v2.Linked, v2.Broken, v2.Broken, v2.Linked}) {
		t.Fatalf("links across opaque item = %v", got)
	}

	capacity.ItemsPerRead = 2
	bounded := v2.ReadWire(input, capacity)
	if got := links(bounded.Segments[0]); bounded.Stop != v2.ItemLimit || !slices.Equal(got, []v2.Link{v2.Broken}) {
		t.Fatalf("item limit = %q, links = %v", bounded.Stop, got)
	}
}

func TestStoredAndDecodedChecksAreIndependent(t *testing.T) {
	for _, tc := range []struct {
		name    string
		matches bool
		state   v2.PayloadState
		reason  v2.Reason
	}{
		{"zstd-ok", true, v2.Verified, ""},
		{"stored-mismatch", false, v2.Damaged, v2.DecodeFailed},
		{"zstd-header-short", true, v2.Damaged, v2.CodecConstraint},
		{"zstd-trailing-byte", true, v2.Damaged, v2.DecodeFailed},
		{"zstd-two-frames", true, v2.Damaged, v2.DecodeFailed},
		{"payload-unknown-codec", true, v2.Unchecked, v2.UnknownCodec},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got := v2.ReadWire(vector(t, tc.name), v2.DefaultReadCapacity())
			frame := got.Segments[0].Items[1].Frame
			if frame.Link != v2.Linked || frame.StoredMatches == nil || *frame.StoredMatches != tc.matches ||
				frame.Payload.State != tc.state || frame.Payload.Reason != tc.reason {
				t.Fatalf("link %q, stored %v, payload %+v", frame.Link, frame.StoredMatches, frame.Payload)
			}
			if tc.state == v2.Verified && len(frame.Payload.Decoded) != 300 {
				t.Fatalf("got %d decoded bytes, want 300", len(frame.Payload.Decoded))
			}
		})
	}
}

func TestTotalDecodedRefusalLatchesAcrossLaterFrames(t *testing.T) {
	capacity := v2.DefaultReadCapacity()
	capacity.DecodedBytesPerRead = 8
	items := v2.ReadWire(vector(t, "payload-total-after-refusal"), capacity).Segments[0].Items
	for _, item := range items[1:3] {
		if item.Frame.Payload.State != v2.Verified {
			t.Fatalf("in-budget payload = %+v", item.Frame.Payload)
		}
	}
	for i, want := range []uint64{9, 8} {
		if payload := items[i+3].Frame.Payload; payload.State != v2.Unchecked || payload.Reason != v2.TotalDecoded || payload.Count != want {
			t.Fatalf("frame %d: payload = %+v, want total refusal at %d", i+3, payload, want)
		}
	}
}
