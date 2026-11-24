// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package main

import (
	"fmt"
	"os"

	"go.blackcatinformatics.ca/gts/v2"
)

func verifyWireV2(path string, wire v2.WireInspection, hasKeys bool) bool {
	recoveryClean := reportRecoveryV2(wire.PrefixGap, wire.ResyncRefusal, func(offset int, message string) {
		fmt.Fprintf(os.Stderr, "gts verify: %s: byte %d: %s\n", path, offset, message)
	})
	switch wire.Dispatch {
	case v2.SecondGeneration:
		if hasKeys {
			fmt.Fprintf(os.Stderr, "gts verify: %s: --key is unsupported for GTS v2\n", path)
			return false
		}
	case v2.UnsupportedGeneration:
		fmt.Fprintf(os.Stderr, "gts verify: %s: unsupported GTS generation %d\n", path, wire.Generation)
		return false
	case v2.Empty:
		fmt.Fprintf(os.Stderr, "gts verify: %s: empty input\n", path)
		return false
	case v2.NotThisFormat:
		fmt.Fprintf(os.Stderr, "gts verify: %s: input does not identify a GTS generation\n", path)
		return false
	}

	clean := wire.Dispatch == v2.SecondGeneration
	if clean {
		fmt.Printf("%s: GTS v2 wire and commit integrity\n", path)
	}
	clean = clean && recoveryClean
	for ordinal, segment := range wire.Segments {
		if !verifySegmentV2(path, ordinal, segment) {
			clean = false
		}
	}
	if wire.Stop != "" {
		fmt.Fprintf(os.Stderr, "gts verify: %s: framing stopped at byte %d: %s\n", path, wire.StopOffset, wire.Stop)
		return false
	}
	return clean
}

func verifySegmentV2(path string, ordinal int, segment v2.Segment) bool {
	problem := func(offset int, reason string) {
		fmt.Fprintf(os.Stderr, "gts verify: %s: byte %d: %s\n", path, offset, reason)
	}
	clean := reportRecoveryV2(segment.FramingGap, segment.ResyncRefusal, problem)
	opener := segment.Items[0]
	switch {
	case opener.Reason != "":
		problem(opener.Offset, string(opener.Reason))
		return false
	case segment.Opener.Registry != 1 || segment.Opener.ID == nil:
		problem(opener.Offset, fmt.Sprintf("unsupported registry version %d", segment.Opener.Registry))
		return false
	}

	for _, item := range segment.Items[1:] {
		switch {
		case item.Frame != nil:
			if !reportFrameProblemsV2(item.Frame, func(reason string) {
				problem(item.Offset, reason)
			}) {
				clean = false
			}
		case item.Kind == v2.OffsetsItem || item.Kind == v2.LocatorItem:
			if reason := itemProblemV2(item); reason != "" {
				problem(item.Offset, reason)
				clean = false
			}
		case item.Kind == v2.DamagedRegion:
			// The framing gap already covers this item.
		default:
			problem(item.Offset, fmt.Sprintf("%s item %s", item.Kind, item.Reason))
			clean = false
		}
	}
	summary := segment.CheckCommits()
	for _, mismatch := range summary.Mismatches {
		if mismatch.CountMismatch {
			problem(mismatch.Offset, fmt.Sprintf("commit_count_mismatch (expected %d)", mismatch.ExpectedCount))
		}
		if mismatch.RootMismatch {
			problem(mismatch.Offset, "commit_root_mismatch")
		}
	}
	fmt.Printf("  segment %d: ", ordinal)
	if summary.CommittedFrames == 0 {
		fmt.Print("no_commit")
	} else {
		fmt.Printf("committed_frames %d", summary.CommittedFrames)
	}
	if summary.TailFrames != 0 {
		fmt.Printf(", uncommitted_tail %d", summary.TailFrames)
	}
	if summary.HasIndex {
		fmt.Print(", index payload claims unchecked")
	}
	fmt.Println()
	return clean && len(summary.Mismatches) == 0
}
