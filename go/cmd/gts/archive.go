// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"os"
	"strings"

	"go.blackcatinformatics.ca/gts/files"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/wire"
)

// blobMT returns the declared media type for an inline blob, if any.
func blobMT(g *model.Graph, digest string) string {
	for _, bm := range g.BlobMeta {
		if bm.Digest != digest {
			continue
		}
		m, ok := bm.Meta.(map[interface{}]interface{})
		if !ok {
			continue
		}
		if v, ok := m["mt"]; ok {
			if s, ok := v.(string); ok {
				return s
			}
		}
	}
	return ""
}

func cmdLs(paths []string) int {
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
	for _, b := range g.Blobs {
		mt := blobMT(g, b.Digest)
		if mt == "" {
			mt = "-"
		}
		fmt.Printf("%s  %10d  %s\n", b.Digest, len(b.Data), mt)
	}
	if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
		return 1
	}
	return 0
}

// normalizeDigest ensures digest is prefixed with "blake3:".
func normalizeDigest(digest string) string {
	if strings.HasPrefix(digest, "blake3:") {
		return digest
	}
	return "blake3:" + digest
}

func cmdExtract(args []string) int {
	var outPath, mt string
	includeSuppressed := false
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
		case "--mt":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts: --mt requires a media type\n%s\n", usage)
				return 2
			}
			mt = args[i+1]
			i++
		case "--include-suppressed":
			includeSuppressed = true
		default:
			positional = append(positional, a)
		}
	}
	if len(positional) != 2 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	path, digest := positional[0], positional[1]
	data, code := load(path)
	if code != 0 {
		return code
	}
	g := reader.Read(data, true, nil)
	for _, d := range g.Diagnostics {
		fmt.Fprintf(os.Stderr, "gts: diagnostic %s: %s\n", d.Code, d.Detail)
	}
	if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
		fmt.Fprintln(os.Stderr, "gts: refusing extract: archive did not read cleanly")
		return 1
	}
	digest = normalizeDigest(digest)

	var blobData []byte
	foundBlob := false
	for _, b := range g.Blobs {
		if b.Digest == digest {
			blobData = b.Data
			foundBlob = true
			break
		}
	}
	if !foundBlob {
		fmt.Fprintf(os.Stderr, "gts: no inline blob %s in %s\n", digest, path)
		return 1
	}
	if !includeSuppressed {
		if _, suppressed := suppressedBlobDigests(g)[digest]; suppressed {
			fmt.Fprintf(os.Stderr, "gts: refusing %s: suppressed (§11); pass --include-suppressed to extract anyway\n", digest)
			return 1
		}
	}
	if mt != "" {
		declared := blobMT(g, digest)
		if declared != mt {
			fmt.Fprintf(os.Stderr, "gts: refusing %s: declared media type %q does not match asserted %q\n", digest, declared, mt)
			return 1
		}
	}
	if wire.DigestStr(blobData) != digest {
		fmt.Fprintf(os.Stderr, "gts: integrity failure: %s bytes re-hash differently\n", digest)
		return 1
	}
	if outPath != "" {
		//nolint:gosec // CLI writes the user-requested output file.
		if err := os.WriteFile(outPath, blobData, 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "gts: cannot write %s: %v\n", outPath, err)
			return 2
		}
	} else {
		if _, err := os.Stdout.Write(blobData); err != nil {
			fmt.Fprintf(os.Stderr, "gts: cannot write stdout: %v\n", err)
			return 2
		}
	}
	return 0
}

func cmdPack(args []string) int {
	var outPath string
	var sources []string
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
		default:
			sources = append(sources, a)
		}
	}
	if len(sources) == 0 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	if outPath == "" {
		fmt.Fprintf(os.Stderr, "gts: pack requires -o\n%s\n", usage)
		return 2
	}

	data, err := files.Pack(sources)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts: refusing pack: %v\n", err)
		return 1
	}
	//nolint:gosec // CLI writes the user-requested output file.
	if err := os.WriteFile(outPath, data, 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "gts: cannot write %s: %v\n", outPath, err)
		return 2
	}
	return 0
}

func cmdUnpack(args []string) int {
	var dest string
	includeSuppressed := false
	var positional []string
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch a {
		case "-C":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts: -C requires a directory\n%s\n", usage)
				return 2
			}
			dest = args[i+1]
			i++
		case "--include-suppressed":
			includeSuppressed = true
		default:
			positional = append(positional, a)
		}
	}
	if len(positional) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	path := positional[0]
	data, code := load(path)
	if code != 0 {
		return code
	}
	g := reader.Read(data, true, nil)
	for _, d := range g.Diagnostics {
		fmt.Fprintf(os.Stderr, "gts: diagnostic %s: %s\n", d.Code, d.Detail)
	}
	if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
		fmt.Fprintln(os.Stderr, "gts: refusing unpack: archive did not read cleanly")
		return 1
	}
	destPath := dest
	if destPath == "" {
		destPath = "."
	}
	if err := files.Unpack(g, destPath, includeSuppressed); err != nil {
		fmt.Fprintf(os.Stderr, "gts: refusing unpack: %v\n", err)
		return 1
	}
	return 0
}

func cmdDiff(args []string) int {
	if len(args) != 2 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	archive, directory := args[0], args[1]
	data, code := load(archive)
	if code != 0 {
		return code
	}
	g := reader.Read(data, true, nil)
	for _, d := range g.Diagnostics {
		fmt.Fprintf(os.Stderr, "gts: diagnostic %s: %s\n", d.Code, d.Detail)
	}
	if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
		fmt.Fprintln(os.Stderr, "gts: refusing diff: archive did not read cleanly")
		return 1
	}
	lines, err := files.Diff(g, directory)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts: refusing diff: %v\n", err)
		return 1
	}
	hasChanges := false
	for _, line := range lines {
		fmt.Println(line)
		hasChanges = true
	}
	if hasChanges {
		return 1
	}
	return 0
}
