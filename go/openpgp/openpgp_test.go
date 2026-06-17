// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package openpgp

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"go.blackcatinformatics.ca/gts/emojihash"
)

// TestParseFrozenVector checks the parser against the Python-generated vector:
// raw key, v4 fingerprint, and emojihash must all match byte-for-byte.
func TestParseFrozenVector(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "vectors", "openpgp", "test-key.json"))
	if err != nil {
		t.Fatalf("vectors/openpgp/test-key.json must exist: %v", err)
	}
	var c struct {
		Armored     string `json:"armored"`
		RawPub      string `json:"raw_pub"`
		Fingerprint string `json:"fingerprint"`
		Emojihash   string `json:"emojihash"`
	}
	if err := json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}

	key, err := ParseTransportKey(c.Armored)
	if err != nil {
		t.Fatalf("parse: %v", err)
	}
	if got := hex.EncodeToString(key.RawPublic); got != c.RawPub {
		t.Errorf("raw_pub = %s, want %s", got, c.RawPub)
	}
	if key.Fingerprint != c.Fingerprint {
		t.Errorf("fingerprint = %s, want %s", key.Fingerprint, c.Fingerprint)
	}
	if got := emojihash.Emojihash(key.RawPublic, 11); got != c.Emojihash {
		t.Errorf("emojihash = %s, want %s", got, c.Emojihash)
	}
}

func TestRejectsNonPGP(t *testing.T) {
	if _, err := ParseTransportKey("not a key"); err == nil {
		t.Error("expected an error for non-armored input")
	}
}
