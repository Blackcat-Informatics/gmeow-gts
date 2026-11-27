// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	v2 "go.blackcatinformatics.ca/gts/v2"
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

func corpusDir() string { return filepath.Join("..", "..", "..", "vectors", "v2", "P6") }

// readCase loads the first case of a corpus entry and the bytes of its declared input file.
func readCase(t *testing.T, entry string, result any) []byte {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(corpusDir(), entry+".json"))
	if err != nil {
		t.Fatal(err)
	}
	var doc struct {
		Cases []struct {
			InputFile string          `json:"input_file"`
			Result    json.RawMessage `json:"result"`
		} `json:"cases"`
	}
	if err := json.Unmarshal(text, &doc); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(doc.Cases[0].Result, result); err != nil {
		t.Fatal(err)
	}
	input, err := os.ReadFile(filepath.Join(corpusDir(), doc.Cases[0].InputFile))
	if err != nil {
		t.Fatal(err)
	}
	return input
}

// TestRecordedResultsFollowFromTheirInputFiles: the recorded result of each entry whose
// input is a file of derivable bytes is recomputed from the bytes of that file alone.
func TestRecordedResultsFollowFromTheirInputFiles(t *testing.T) {
	var key edKeyIDResult
	public := readCase(t, "key-id-ed25519", &key)
	id := v2.KeyIdentifier(v2.Ed25519Algorithm, public)
	if h(public) != key.PublicKey || key.PreimageLength != 4+len(public) || h(id[:]) != key.KeyID {
		t.Errorf("key-id-ed25519: the recorded result does not follow from its input file")
	}

	for _, s := range []struct {
		name      string
		algorithm uint64
	}{{"subject-ed25519", 1}, {"subject-composite", 2}} {
		var want subjectResult
		input := readCase(t, s.name, &want)
		if len(input) != 96 {
			t.Fatalf("%s: input file is %d bytes, want K, H0 and F of 32 bytes each", s.name, len(input))
		}
		record := v2.SignatureRecord{Algorithm: s.algorithm, KeyID: digestOf(input[:32])}
		h0, f := digestOf(input[32:64]), digestOf(input[64:])
		message := v2.SignedMessage(record, h0, f)
		got := subjectResult{
			Subject: h(v2.SubjectPreimage(record, h0, f)), Message: h(message[:]),
			K: h(input[:32]), H0: h(input[32:64]), F: h(input[64:]),
		}
		if got != want {
			t.Errorf("%s: recomputed %+v, recorded %+v", s.name, got, want)
		}
	}

	var rep representativeResult
	message := readCase(t, "composite-message-representative", &rep)
	if h(message) != rep.Message || h(representative(message)) != rep.MPrime {
		t.Errorf("composite-message-representative: the recorded result does not follow from its input file")
	}
}

// TestNoDeclaredInputIsEmpty: an entry whose input file has no bytes proves nothing, so
// every input file that any P6 entry declares holds at least one byte.
func TestNoDeclaredInputIsEmpty(t *testing.T) {
	entries, err := filepath.Glob(filepath.Join(corpusDir(), "*.json"))
	if err != nil || len(entries) == 0 {
		t.Fatalf("no P6 entries found: %v", err)
	}
	declared := 0
	for _, entry := range entries {
		text, err := os.ReadFile(entry)
		if err != nil {
			t.Fatal(err)
		}
		var doc struct {
			Cases []struct {
				InputFile string `json:"input_file"`
			} `json:"cases"`
		}
		if err := json.Unmarshal(text, &doc); err != nil {
			t.Fatalf("%s: %v", entry, err)
		}
		for i, c := range doc.Cases {
			if c.InputFile == "" {
				continue
			}
			declared++
			info, err := os.Stat(filepath.Join(corpusDir(), c.InputFile))
			if err != nil {
				t.Errorf("%s case %d: %v", filepath.Base(entry), i, err)
			} else if info.Size() == 0 {
				t.Errorf("%s case %d: input file %s is empty", filepath.Base(entry), i, c.InputFile)
			}
		}
	}
	if declared == 0 {
		t.Fatal("no declared input files were checked")
	}
}
