// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package cose_test

import (
	"bytes"
	"crypto/ed25519"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"go.blackcatinformatics.ca/gts/cose"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/writer"
)

const (
	cat   = "https://example.org/Cat"
	label = "http://www.w3.org/2000/01/rdf-schema#label"
)

func TestSignedFileVector(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "vectors", "signed", "basic.json"))
	if err != nil {
		t.Fatal(err)
	}
	var c struct {
		Seed string `json:"seed"`
		Pub  string `json:"pub"`
		Kid  string `json:"kid"`
		Gts  string `json:"gts"`
	}
	if err := json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	seed, _ := hex.DecodeString(c.Seed)
	pub, _ := hex.DecodeString(c.Pub)
	expected, _ := hex.DecodeString(c.Gts)

	// Writer signing reproduces the frozen signed file byte-for-byte.
	w := writer.New("dist")
	w.SignWith(ed25519.NewKeyFromSeed(seed), c.Kid)
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: cat},
		{Kind: model.Iri, Value: label},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	if got := w.ToBytes(); !bytes.Equal(got, expected) {
		t.Fatalf("writer signing mismatch: got %d bytes, want %d", len(got), len(expected))
	}

	right := func(kid string) (ed25519.PublicKey, bool) {
		if kid == c.Kid {
			return ed25519.PublicKey(pub), true
		}
		return nil, false
	}

	// Right key -> every signature valid.
	g := reader.Read(expected, false, nil)
	if len(g.Signatures) != 2 {
		t.Fatalf("expected 2 signatures, got %d", len(g.Signatures))
	}
	cose.VerifySignatures(g.Signatures, right)
	for _, s := range g.Signatures {
		if s.Status != "valid" || s.Kid != c.Kid {
			t.Errorf("expected valid/%s, got %s/%s", c.Kid, s.Status, s.Kid)
		}
	}

	// No key -> unverified.
	g = reader.Read(expected, false, nil)
	cose.VerifySignatures(g.Signatures, func(string) (ed25519.PublicKey, bool) { return nil, false })
	for _, s := range g.Signatures {
		if s.Status != "unverified" {
			t.Errorf("expected unverified, got %s", s.Status)
		}
	}

	// Wrong key -> invalid.
	_, wrongPriv, _ := ed25519.GenerateKey(bytes.NewReader(make([]byte, 64)))
	wrong := wrongPriv.Public().(ed25519.PublicKey)
	g = reader.Read(expected, false, nil)
	cose.VerifySignatures(g.Signatures, func(string) (ed25519.PublicKey, bool) { return wrong, true })
	for _, s := range g.Signatures {
		if s.Status != "invalid" {
			t.Errorf("expected invalid, got %s", s.Status)
		}
	}
}
