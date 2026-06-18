// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package mmr

import (
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

func TestVerifyProofJSONReturnsVerifiedProof(t *testing.T) {
	proof, err := VerifyProofJSON(proofFixture(t, "mmr-basic-proof.json"))
	if err != nil {
		t.Fatal(err)
	}
	if got := len(proof.Root); got != 32 {
		t.Fatalf("root length = %d, want 32", got)
	}
}
