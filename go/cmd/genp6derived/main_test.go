// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

var derivedEntries = []string{
	"sign-ed25519-reproduce.json",
	"sign-composite-reproduce.json",
	"sign-composite-reproduce.public-key.bin",
	"sign-composite-reproduce.value.bin",
	"key-id-ed25519.json",
	"key-id-ed25519.bin",
	"key-id-composite.json",
	"key-id-composite.bin",
	"subject-ed25519.json",
	"subject-ed25519.bin",
	"subject-composite.json",
	"subject-composite.bin",
	"composite-message-representative.json",
	"composite-message-representative.bin",
}

// TestDerivedEntriesMatchCorpus: every derived P6 entry regenerated from its stated inputs is
// byte for byte the file in the corpus, JSON layout included.
func TestDerivedEntriesMatchCorpus(t *testing.T) {
	dir := filepath.Join("..", "..", "..", "vectors", "v2", "P6")
	files, err := build()
	if err != nil {
		t.Fatal(err)
	}
	for _, name := range derivedEntries {
		want, ok := files[name]
		if !ok {
			t.Errorf("%s is not generated", name)
			continue
		}
		got, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			t.Errorf("%s: %v", name, err)
			continue
		}
		if !bytes.Equal(got, want) {
			t.Errorf("%s differs from its regeneration: %d bytes on disk, %d regenerated", name, len(got), len(want))
		}
	}
	if len(files) != len(derivedEntries) {
		t.Errorf("generated %d files, want %d", len(files), len(derivedEntries))
	}
}
