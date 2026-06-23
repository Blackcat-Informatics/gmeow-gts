// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"os"
	"time"

	"go.blackcatinformatics.ca/gts/compact"
	"go.blackcatinformatics.ca/gts/reader"
)

func cmdCat(args []string) int {
	var outPath string
	var inputs []string
	for i := 0; i < len(args); i++ {
		if args[i] == "-o" {
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts: -o requires a path\n%s\n", usage)
				return 2
			}
			outPath = args[i+1]
			i++
		} else {
			inputs = append(inputs, args[i])
		}
	}
	if len(inputs) < 2 {
		fmt.Fprintf(os.Stderr, "gts: cat needs at least two inputs\n%s\n", usage)
		return 2
	}

	var combined []byte
	for _, path := range inputs {
		data, code := load(path)
		if code != 0 {
			return code
		}
		fs := reader.ReadFileSegments(data)
		if hasProblems(fs) {
			fmt.Fprintf(os.Stderr, "gts: refusing %s: not a clean GTS input\n", path)
			printLedger(path, fs)
			return 1
		}
		for idx, seg := range fs.Segments {
			contributes := len(seg.Quads) > 0 || len(seg.Blobs) > 0 ||
				len(seg.Reifiers) > 0 || len(seg.Annotations) > 0 ||
				len(seg.Suppressions) > 0
			if !contributes {
				fmt.Fprintf(os.Stderr, "gts: refusing %s: segment %d folds to nothing (no quads/blobs/reifies/annot/suppress) — wiring bug?\n", path, idx)
				return 1
			}
		}
		combined = append(combined, data...)
	}

	folded := reader.Read(combined, true, nil)
	if allQuadsSuppressed(folded) {
		fmt.Fprintln(os.Stderr, "gts: refusing composition: suppressions hide every quad in the folded output")
		return 1
	}

	if outPath != "" {
		//nolint:gosec // CLI writes the user-requested output file.
		if err := os.WriteFile(outPath, combined, 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "gts: cannot write %s: %v\n", outPath, err)
			return 2
		}
	} else {
		if _, err := os.Stdout.Write(combined); err != nil {
			fmt.Fprintf(os.Stderr, "gts: cannot write stdout: %v\n", err)
			return 2
		}
	}
	return 0
}

// cmdCompact rewrites a GTS file into the streamable layout state (§10.1, §14.1).
func cmdCompact(args []string) int {
	var outPath, timestamp string
	streamable := false
	sealOriginal := false
	var positional []string
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch a {
		case "-o", "--out":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts: -o requires a path\n%s\n", usage)
				return 2
			}
			outPath = args[i+1]
			i++
		case "--streamable":
			streamable = true
		case "--seal-original":
			sealOriginal = true
		case "--timestamp":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts: --timestamp requires a value\n%s\n", usage)
				return 2
			}
			timestamp = args[i+1]
			i++
		default:
			positional = append(positional, a)
		}
	}
	if len(positional) != 1 || outPath == "" {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	if !streamable {
		// The verb is reserved for layout rewrites; a future --snapshot mode
		// (§10) would land here. Without a mode the request is ambiguous.
		fmt.Fprintln(os.Stderr, "gts: compact requires --streamable")
		return 2
	}
	data, code := load(positional[0])
	if code != 0 {
		return code
	}
	if timestamp == "" {
		timestamp = time.Now().UTC().Format("2006-01-02T15:04:05Z")
	}
	out, err := compact.Streamable(data, timestamp, sealOriginal)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts: refusing compact: %v\n", err)
		return 1
	}
	//nolint:gosec // CLI writes the user-requested output file.
	if err := os.WriteFile(outPath, out, 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "gts: cannot write %s: %v\n", outPath, err)
		return 2
	}
	return 0
}
