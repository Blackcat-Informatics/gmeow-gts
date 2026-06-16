// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"os"
	"path/filepath"
	"testing"
)

// FuzzRead exercises the reader against arbitrary bytes. The contract is that
// reading untrusted input never panics: a damaged, truncated, or hostile log
// must fold to whatever is recoverable and record diagnostics, never crash.
func FuzzRead(f *testing.F) {
	// Seed with the frozen conformance corpus so the fuzzer starts from valid,
	// structurally interesting inputs and mutates outward.
	for _, p := range globVectors() {
		if b, err := os.ReadFile(p); err == nil {
			f.Add(b)
		}
	}
	f.Fuzz(func(_ *testing.T, data []byte) {
		_ = Read(data, false, nil)
		_ = Read(data, true, nil)
	})
}

func globVectors() []string {
	matches, err := filepath.Glob(filepath.Join("..", "..", "vectors", "*.gts"))
	if err != nil {
		return nil
	}
	return matches
}
