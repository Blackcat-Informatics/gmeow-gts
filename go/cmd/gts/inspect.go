// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"io"
	"os"

	"go.blackcatinformatics.ca/gts/fromnquads"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/wire"
)

// load reads path and returns (data, 0) or (nil, 2) on IO error.
func load(path string) ([]byte, int) {
	//nolint:gosec // CLI explicitly reads the user-supplied input path.
	data, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts: cannot read %s: %v\n", path, err)
		return nil, 2
	}
	return data, 0
}

// printLedger prints a human-readable per-segment summary.
func printLedger(path string, fs *reader.FileSegments) {
	tornSuffix := ""
	if fs.Torn >= 0 {
		tornSuffix = fmt.Sprintf(", TORN at byte %d", fs.Torn)
	}
	fmt.Printf("%s: %d segment(s)%s\n", path, len(fs.Segments), tornSuffix)
	if fs.Fatal != nil {
		fmt.Printf("  FATAL %s: %s\n", fs.Fatal.Code, fs.Fatal.Detail)
		return
	}
	for idx, seg := range fs.Segments {
		head := "<none>"
		if len(seg.SegmentHeads) > 0 {
			head = wire.Hex(seg.SegmentHeads[0])
		}
		profile := "<none>"
		if len(seg.SegmentProfiles) > 0 {
			profile = seg.SegmentProfiles[0]
		}
		signers := 0
		for _, s := range seg.Signatures {
			if s.Status != "invalid" {
				signers++
			}
		}
		fmt.Printf(
			"  segment %d: head %s profile %s terms %d quads %d reifies %d annot %d blobs %d suppress %d opaque %d sigs %d\n",
			idx, head, profile,
			len(seg.Terms), len(seg.Quads), len(seg.Reifiers), len(seg.Annotations),
			len(seg.Blobs), len(seg.Suppressions), len(seg.Opaque), signers,
		)
		// Layout-state line (§3.3): declared streamable claims report their
		// covered boundary and any accretive tail.
		if len(seg.SegmentStreamable) > 0 && seg.SegmentStreamable[0].Claimed {
			layout := seg.SegmentStreamable[0]
			headHex := "<none>"
			if layout.Head != nil {
				headHex = wire.Hex(layout.Head)
			}
			tail := ""
			if layout.Tail > 0 {
				tail = fmt.Sprintf(", accretive tail %d frame(s)", layout.Tail)
			}
			fmt.Printf("    layout: streamable through frame %d (head %s)%s\n", layout.Covered, headHex, tail)
		}
		for _, o := range seg.Opaque {
			fmt.Printf("    opaque: %s (%s)\n", o.FrameType, o.Reason)
		}
		for _, d := range seg.Diagnostics {
			idxStr := ""
			if d.FrameIndex != nil {
				idxStr = fmt.Sprintf(" [item %d]", *d.FrameIndex)
			}
			fmt.Printf("    diagnostic %s: %s%s\n", d.Code, d.Detail, idxStr)
		}
	}
}

// hasProblems reports whether the file segments contain any fatal/torn/diagnostic issues.
func hasProblems(fs *reader.FileSegments) bool {
	if fs.Fatal != nil || fs.Torn >= 0 {
		return true
	}
	for _, seg := range fs.Segments {
		if len(seg.Diagnostics) > 0 {
			return true
		}
	}
	return false
}

func cmdInfo(paths []string) int {
	if len(paths) == 0 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	problems := false
	for _, path := range paths {
		data, code := load(path)
		if code != 0 {
			return code
		}
		fs := reader.ReadFileSegments(data)
		printLedger(path, fs)
		if hasProblems(fs) {
			problems = true
		}
	}
	if problems {
		return 1
	}
	return 0
}

func cmdFold(paths []string) int {
	if len(paths) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	data, code := load(paths[0])
	if code != 0 {
		return code
	}
	g := reader.Read(data, true, nil)
	for _, d := range g.Diagnostics {
		fmt.Fprintf(os.Stderr, "gts: diagnostic %s: %s\n", d.Code, d.Detail)
	}
	fmt.Print(nquads.ToNQuads(g))
	if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
		return 1
	}
	return 0
}

func cmdFromNQ(args []string) int {
	var outPath string
	var positional []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-o", "--out":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts from-nq: -o requires a path\n%s\n", usage)
				return 2
			}
			outPath = args[i+1]
			i++
		default:
			positional = append(positional, args[i])
		}
	}
	if len(positional) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}

	path := positional[0]
	var data []byte
	var err error
	if path == "-" {
		data, err = io.ReadAll(os.Stdin)
	} else {
		//nolint:gosec // CLI explicitly reads the user-supplied input path.
		data, err = os.ReadFile(path)
	}
	if err != nil {
		source := path
		if path == "-" {
			source = "stdin"
		}
		fmt.Fprintf(os.Stderr, "gts from-nq: cannot read %s: %v\n", source, err)
		return 2
	}

	out, err := fromnquads.FromNQuads(string(data))
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts from-nq: %v\n", err)
		return 1
	}
	if outPath != "" {
		//nolint:gosec // CLI writes the user-requested output file.
		if err := os.WriteFile(outPath, out, 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "gts from-nq: cannot write %s: %v\n", outPath, err)
			return 2
		}
		return 0
	}
	if _, err := os.Stdout.Write(out); err != nil {
		fmt.Fprintf(os.Stderr, "gts from-nq: cannot write stdout: %v\n", err)
		return 2
	}
	return 0
}
