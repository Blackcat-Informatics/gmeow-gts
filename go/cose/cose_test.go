// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package cose

import (
	"crypto/ed25519"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestCOSESign1Vectors(t *testing.T) {
	dir := filepath.Join("..", "..", "vectors", "cose")
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("vectors/cose must exist: %v", err)
	}
	count := 0
	for _, e := range entries {
		if filepath.Ext(e.Name()) != ".json" {
			continue
		}
		raw, err := os.ReadFile(filepath.Join(dir, e.Name()))
		if err != nil {
			t.Fatal(err)
		}
		var c struct {
			Seed    string `json:"seed"`
			Pub     string `json:"pub"`
			Kid     string `json:"kid"`
			FrameID string `json:"frame_id"`
			Cose    string `json:"cose"`
		}
		if err := json.Unmarshal(raw, &c); err != nil {
			t.Fatal(err)
		}
		seed, _ := hex.DecodeString(c.Seed)
		pub, _ := hex.DecodeString(c.Pub)
		frameID, _ := hex.DecodeString(c.FrameID)
		expected := c.Cose

		// Deterministic Ed25519: signing reproduces the frozen bytes.
		got := hex.EncodeToString(SignID(frameID, ed25519.NewKeyFromSeed(seed), c.Kid))
		if got != expected {
			t.Errorf("%s: sign mismatch\n got %s\nwant %s", e.Name(), got, expected)
		}

		// kid round-trips.
		if kid, ok := SignatureKID(SignID(frameID, ed25519.NewKeyFromSeed(seed), c.Kid)); !ok || kid != c.Kid {
			t.Errorf("%s: kid mismatch: %q ok=%v", e.Name(), kid, ok)
		}

		cose, _ := hex.DecodeString(expected)
		if VerifySig(cose, frameID, ed25519.PublicKey(pub)) != Valid {
			t.Errorf("%s: expected Valid", e.Name())
		}
		tampered := append(append([]byte{}, frameID...), 0xff)
		if VerifySig(cose, tampered, ed25519.PublicKey(pub)) != Invalid {
			t.Errorf("%s: tampered id should be Invalid", e.Name())
		}
		count++
	}
	if count < 2 {
		t.Fatalf("expected at least two COSE vectors, found %d", count)
	}
}
