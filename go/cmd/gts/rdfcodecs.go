// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"io"
	"os"

	"go.blackcatinformatics.ca/gts/rdfcodecs"
	"go.blackcatinformatics.ca/gts/reader"
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
	g := reader.Read(data, true, nil)
	var (
		text string
		err  error
	)
	switch command {
	case "to-nt":
		text, err = rdfcodecs.ToNTriples(g)
	case "to-trig":
		text, err = rdfcodecs.ToTriG(g)
	case "to-turtle":
		text, err = rdfcodecs.ToTurtle(g)
	case "to-rdfxml":
		text, err = rdfcodecs.ToRDFXML(g)
	default:
		err = fmt.Errorf("unknown RDF text export command %s", command)
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts %s: %v\n", command, err)
		return 1
	}
	if _, err := io.WriteString(os.Stdout, text); err != nil {
		fmt.Fprintf(os.Stderr, "gts %s: cannot write stdout: %v\n", command, err)
		return 2
	}
	if len(g.Diagnostics) > 0 || len(g.SegmentHeads) == 0 {
		return 1
	}
	return 0
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
	switch command {
	case "from-nt":
		out, err = rdfcodecs.FromNTriples(string(data))
	case "from-trig":
		out, err = rdfcodecs.FromTriG(string(data))
	case "from-turtle":
		out, err = rdfcodecs.FromTurtle(string(data))
	case "from-rdfxml":
		out, err = rdfcodecs.FromRDFXML(string(data))
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
