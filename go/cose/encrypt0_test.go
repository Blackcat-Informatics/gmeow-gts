// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package cose

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// TestEncrypt0Vector checks the COSE_Encrypt0 transform against the Python-
// generated vector: a fixed IV reproduces the sealed bytes, and the frozen
// COSE opens back to the plaintext.
func TestEncrypt0Vector(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "vectors", "encrypt0", "basic.json"))
	if err != nil {
		t.Fatalf("vectors/encrypt0/basic.json must exist: %v", err)
	}
	var c struct {
		Key       string `json:"key"`
		IV        string `json:"iv"`
		Kid       string `json:"kid"`
		Plaintext string `json:"plaintext"`
		Cose      string `json:"cose"`
	}
	if err := json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	key, _ := hex.DecodeString(c.Key)
	iv, _ := hex.DecodeString(c.IV)
	plaintext, _ := hex.DecodeString(c.Plaintext)
	expected, _ := hex.DecodeString(c.Cose)

	// Fixed IV -> the sealed bytes reproduce the frozen vector exactly.
	sealed, err := Encrypt0WithIV(plaintext, c.Kid, key, iv)
	if err != nil {
		t.Fatalf("seal: %v", err)
	}
	if !bytes.Equal(sealed, expected) {
		t.Errorf("sealed mismatch:\n got %x\nwant %x", sealed, expected)
	}

	// The recipient kid round-trips out of the cleartext header.
	if kid, ok := RecipientKID(expected); !ok || kid != c.Kid {
		t.Errorf("RecipientKID = %q, %v; want %q", kid, ok, c.Kid)
	}

	// The frozen COSE opens back to the plaintext under the content key.
	got, err := Decrypt0(expected, func(kid string) ([]byte, bool) {
		return key, kid == c.Kid
	})
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	if !bytes.Equal(got, plaintext) {
		t.Errorf("opened = %x, want %x", got, plaintext)
	}

	// No key -> ErrMissingKey; wrong key -> ErrAuthFailed.
	if _, err := Decrypt0(expected, func(string) ([]byte, bool) { return nil, false }); err != ErrMissingKey {
		t.Errorf("missing key err = %v, want %v", err, ErrMissingKey)
	}
	wrong := make([]byte, 32)
	if _, err := Decrypt0(expected, func(string) ([]byte, bool) { return wrong, true }); err != ErrAuthFailed {
		t.Errorf("wrong key err = %v, want %v", err, ErrAuthFailed)
	}
}

func TestEncrypt0RandomIVRoundTrip(t *testing.T) {
	key := make([]byte, 32)
	for i := range key {
		key[i] = byte(i)
	}
	sealed, err := Encrypt0([]byte("verified id record"), "did:court", key)
	if err != nil {
		t.Fatalf("seal: %v", err)
	}
	got, err := Decrypt0(sealed, func(kid string) ([]byte, bool) {
		return key, kid == "did:court"
	})
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	if string(got) != "verified id record" {
		t.Errorf("round-trip = %q", got)
	}
}
