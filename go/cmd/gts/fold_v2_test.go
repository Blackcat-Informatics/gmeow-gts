// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package main

import (
	"strings"
	"testing"
)

func TestFoldWritesV2NQuads(t *testing.T) {
	cmd, stdout, stderr := run(t, "fold", vector(t, "v2/P3/project__round-trip-rdf.bin"))
	if cmd.ProcessState.ExitCode() != 0 || stderr.Len() != 0 {
		t.Fatalf("fold exit %d: %s", cmd.ProcessState.ExitCode(), stderr)
	}
	if text := stdout.String(); strings.Count(text, "\n") != 5 || !strings.Contains(text, `"v"^^<http://e/dt> <http://e/g> .`) {
		t.Fatalf("unexpected fold: %s", text)
	}
}

func TestFoldRefusesBeforeWritingV2NQuads(t *testing.T) {
	for _, tc := range []struct {
		file, diagnostic string
	}{
		{"P1/gen-record-three.bin", "unsupported GTS generation 3"},
		{"P1/file-first-incomplete.bin", "torn_tail"},
		{"P3/project__malformed-iri.bin", "invalid RDF IRI"},
		{"P3/project__value-pair.bin", "unsupported_term_kind"},
		{"P3/terms-front-term-zero-prefix.prefix-5-at-term-0.bin", "forbidden_combination"},
	} {
		t.Run(tc.file, func(t *testing.T) {
			cmd, stdout, stderr := run(t, "fold", vector(t, "v2/"+tc.file))
			if cmd.ProcessState.ExitCode() != 1 || stdout.Len() != 0 || !strings.Contains(stderr.String(), tc.diagnostic) {
				t.Fatalf("fold exit %d, stdout %q, stderr %q", cmd.ProcessState.ExitCode(), stdout, stderr)
			}
		})
	}
}
