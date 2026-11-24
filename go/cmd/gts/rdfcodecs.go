// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package main

import (
	"fmt"
	"io"
	"net/url"
	"os"
	"path/filepath"
	"strings"

	"go.blackcatinformatics.ca/gts/fromnquads"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/rdfcodecs"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/v2"
)

func cmdToRDFText(command string, args []string) int {
	if len(args) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	data, code := load(args[0])
	if code != 0 {
		return code
	}
	var (
		text string
		err  error
	)
	result := v2.ReadWire(data, v2.DefaultReadCapacity())
	if result.Dispatch == v2.FirstGeneration {
		g := reader.Read(data, true, nil)
		text, err = exportRDFLegacy(command, g)
		if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
			code = 1
		}
	} else {
		text, err = exportRDFV2(command, result)
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts %s: %v\n", command, err)
		return 1
	}
	if _, err := io.WriteString(os.Stdout, text); err != nil {
		fmt.Fprintf(os.Stderr, "gts %s: cannot write stdout: %v\n", command, err)
		return 2
	}
	return code
}

func exportRDFLegacy(command string, g *model.Graph) (string, error) {
	switch command {
	case "to-nt":
		return rdfcodecs.ToNTriples(g)
	case "to-trig":
		return rdfcodecs.ToTriG(g)
	case "to-turtle":
		return rdfcodecs.ToTurtle(g)
	case "to-rdfxml":
		return rdfcodecs.ToRDFXML(g)
	default:
		return "", fmt.Errorf("unknown RDF text export command %s", command)
	}
}

func exportRDFV2(command string, result v2.WireResult) (string, error) {
	projection, err := result.WholeRDFProjection()
	if err != nil {
		return "", err
	}
	switch command {
	case "to-nt":
		return rdfcodecs.ExportNTriplesV2(projection)
	case "to-trig":
		return projection.TriG()
	case "to-turtle":
		return rdfcodecs.ExportTurtleV2(projection)
	case "to-rdfxml":
		return rdfcodecs.ExportRDFXMLV2(projection)
	default:
		return "", fmt.Errorf("unknown RDF text export command %s", command)
	}
}

func cmdFromRDFText(command string, args []string) int {
	var outPath string
	var positional []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-o", "--out":
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "gts %s: -o requires a path\n%s\n", command, usage)
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
		fmt.Fprintf(os.Stderr, "gts %s: cannot read %s: %v\n", command, source, err)
		return 2
	}

	var out []byte
	var base string
	if command != "from-nt" && path != "-" {
		base, err = rdfFileBase(path)
		if err != nil {
			fmt.Fprintf(os.Stderr, "gts %s: cannot determine base IRI for %s: %v\n", command, path, err)
			return 2
		}
	}
	switch command {
	case "from-nt":
		out, err = fromnquads.ImportTriplesV2(string(data))
	case "from-trig":
		out, err = rdfcodecs.ImportTriGV2(string(data), base)
	case "from-turtle":
		out, err = rdfcodecs.ImportTurtleV2(string(data), base)
	case "from-rdfxml":
		out, err = rdfcodecs.ImportRDFXMLV2(string(data), base)
	default:
		err = fmt.Errorf("unknown RDF text import command %s", command)
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts %s: %v\n", command, err)
		return 1
	}
	if outPath != "" {
		//nolint:gosec // CLI writes the user-requested output file.
		if err := os.WriteFile(outPath, out, 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "gts %s: cannot write %s: %v\n", command, outPath, err)
			return 2
		}
		return 0
	}
	if _, err := os.Stdout.Write(out); err != nil {
		fmt.Fprintf(os.Stderr, "gts %s: cannot write stdout: %v\n", command, err)
		return 2
	}
	return 0
}

func rdfFileBase(path string) (string, error) {
	absolute, err := filepath.Abs(path)
	if err != nil {
		return "", err
	}
	normalized := filepath.ToSlash(absolute)
	base := url.URL{Scheme: "file", Path: normalized}
	if unc, ok := strings.CutPrefix(normalized, "//"); ok {
		if host, rest, ok := strings.Cut(unc, "/"); ok && host != "" {
			base.Host, base.Path = host, "/"+rest
		}
	} else if !strings.HasPrefix(normalized, "/") {
		base.Path = "/" + normalized
	}
	return base.String(), nil
}
