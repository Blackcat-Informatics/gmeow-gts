// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package main

import (
	"fmt"
	"maps"
	"os"
	"slices"
	"strconv"

	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/v2"
)

// Registry 1 frame types, indexed by their wire code.
var frameNamesV2 = [...]string{
	"", "terms", "quads", "reifier_bindings", "annotations", "empty_graphs",
	"bindings", "suppress", "chunk", "blob", "recipe", "value", "external",
	"pack", "commit", "index", "parity",
}

func inspectFile(path string, data []byte) bool {
	result := v2.InspectWire(data, v2.DefaultReadCapacity())
	switch result.Dispatch {
	case v2.FirstGeneration:
		fs := reader.ReadFileSegments(data)
		printLedger(path, fs)
		return !hasProblems(fs)
	case v2.Empty:
		fmt.Printf("%s: empty input, 0 segments\n", path)
		return false
	case v2.UnsupportedGeneration:
		fmt.Fprintf(os.Stderr, "gts info: %s: unsupported GTS generation %d\n", path, result.Generation)
		return false
	case v2.NotThisFormat:
		fmt.Fprintf(os.Stderr, "gts info: %s: input does not identify a GTS generation\n", path)
		return false
	case v2.SecondGeneration:
		fmt.Printf("%s: GTS v2 wire ledger, %d segment(s)\n", path, len(result.Segments))
	default:
		fmt.Printf("%s: GTS generation unavailable\n", path)
	}
	clean := result.Dispatch == v2.SecondGeneration && result.Stop == ""
	for ordinal, segment := range result.Segments {
		if !printSegmentV2(ordinal, segment) {
			clean = false
		}
	}
	if result.Stop != "" {
		fmt.Printf("  framing stopped at byte %d: %s\n", result.StopOffset, result.Stop)
	}
	return clean
}

func printOpenerV2(ordinal int, segment v2.Segment) bool {
	item := segment.Items[0]
	if item.Reason != "" {
		fmt.Printf("  segment %d: unreadable %s at byte %d: %s\n", ordinal, item.Kind, item.Offset, item.Reason)
		return false
	}
	opener := segment.Opener
	head := "<unavailable>"
	if segment.Head != nil {
		head = fmt.Sprintf("%x", *segment.Head)
	}
	profile := "<none>"
	if opener.Profile != 0 {
		profile = strconv.FormatUint(opener.Profile, 10)
	}
	fmt.Printf("  segment %d: %s, effective head %s, registry %d, hash algorithm %d, profile %s\n",
		ordinal, item.Kind, head, opener.Registry, opener.HashAlgorithm, profile)
	if opener.Canonical != 0 {
		fmt.Printf("    canonical claim %d\n", opener.Canonical)
	}
	if opener.Resync != nil {
		fmt.Printf("    resync claimed offset %d\n", opener.Resync.Offset)
		if opener.Resync.Displaced {
			fmt.Printf("    resync displaced to byte %d\n", item.Offset)
		}
	}
	if opener.Registry != 1 {
		fmt.Printf("    unsupported registry version %d\n", opener.Registry)
	}
	if opener.ID == nil {
		fmt.Println("    opener identifier unavailable")
	}
	return opener.Registry == 1 && opener.ID != nil
}

func printSegmentV2(ordinal int, segment v2.Segment) bool {
	clean := printOpenerV2(ordinal, segment)
	counts := make(map[uint64]int)
	frames := 0
	for _, item := range segment.Items {
		if item.Frame == nil {
			continue
		}
		frames++
		if item.Frame.Type != 0 {
			counts[item.Frame.Type]++
		}
	}
	fmt.Printf("    %d item(s), %d frame(s)\n", len(segment.Items), frames)
	for _, kind := range slices.Sorted(maps.Keys(counts)) {
		if kind < uint64(len(frameNamesV2)) {
			fmt.Printf("    %s %d\n", frameNamesV2[kind], counts[kind])
		} else {
			fmt.Printf("    frame type %d: %d\n", kind, counts[kind])
		}
	}
	for _, item := range segment.Items[1:] {
		if reason := itemProblemV2(item); reason != "" {
			fmt.Printf("    %s item at byte %d: %s\n", item.Kind, item.Offset, reason)
			clean = false
		}
		if item.Frame != nil && !reportFrameProblemsV2(item.Frame, func(reason string) {
			fmt.Printf("    frame at byte %d: %s\n", item.Offset, reason)
		}) {
			clean = false
		}
	}
	return clean
}

func itemProblemV2(item v2.Item) string {
	if item.ClosingClaimFailure != "" {
		return fmt.Sprintf("index_claim_failed (%s)", item.ClosingClaimFailure)
	}
	return string(item.Reason)
}

func reportFrameProblemsV2(frame *v2.Frame, report func(string)) bool {
	clean := true
	if frame.Malformed != "" {
		report(fmt.Sprintf("malformed envelope: %s", frame.Malformed))
		clean = false
	}
	if frame.Link != v2.Linked {
		report(fmt.Sprintf("link %s", frame.Link))
		clean = false
	}
	if frame.StoredMatches != nil && !*frame.StoredMatches {
		report("stored check mismatch")
		clean = false
	}
	if frame.Payload.State != v2.Verified {
		report(fmt.Sprintf("%s payload: %s", frame.Payload.State, frame.Payload.Reason))
		clean = false
	}
	return clean
}
