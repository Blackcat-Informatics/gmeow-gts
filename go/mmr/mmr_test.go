// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package mmr

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func proofFixture(t *testing.T, name string) []byte {
	t.Helper()
	path := filepath.Join("..", "..", "vectors", "proofs", name)
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func TestPositiveProofFixtureVerifies(t *testing.T) {
	proof, err := ProofFromJSON(proofFixture(t, "mmr-basic-proof.json"))
	if err != nil {
		t.Fatal(err)
	}
	if err := VerifyProof(proof); err != nil {
		t.Fatal(err)
	}
	if proof.Count != 4 || proof.LeafIndex != 2 {
		t.Fatalf("unexpected fixture target: count=%d leaf=%d", proof.Count, proof.LeafIndex)
	}
}

func TestNegativeProofFixtureFails(t *testing.T) {
	proof, err := ProofFromJSON(proofFixture(t, "mmr-basic-proof-bad-root.json"))
	if err != nil {
		t.Fatal(err)
	}
	if err := VerifyProof(proof); err == nil {
		t.Fatal("bad-root proof verified")
	}
}

func TestProofFromJSONRejectsMissingOrNullRequiredFields(t *testing.T) {
	fixture := proofFixture(t, "mmr-basic-proof.json")

	cases := map[string]func(map[string]interface{}){
		"missing count": func(doc map[string]interface{}) {
			delete(doc, "count")
		},
		"null path": func(doc map[string]interface{}) {
			doc["path"] = nil
		},
		"missing peak height": func(doc map[string]interface{}) {
			peaks := doc["peaks"].([]interface{})
			peak := peaks[0].(map[string]interface{})
			delete(peak, "height")
		},
	}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			var doc map[string]interface{}
			if err := json.Unmarshal(fixture, &doc); err != nil {
				t.Fatal(err)
			}
			mutate(doc)
			data, err := json.Marshal(doc)
			if err != nil {
				t.Fatal(err)
			}
			if _, err := ProofFromJSON(data); err == nil {
				t.Fatal("incomplete proof parsed")
			}
		})
	}
}

func TestVerifyProofJSONReturnsVerifiedProof(t *testing.T) {
	proof, err := VerifyProofJSON(proofFixture(t, "mmr-basic-proof.json"))
	if err != nil {
		t.Fatal(err)
	}
	if got := len(proof.Root); got != 32 {
		t.Fatalf("root length = %d, want 32", got)
	}
}
