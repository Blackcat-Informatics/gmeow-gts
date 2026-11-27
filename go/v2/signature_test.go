// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

type signCase struct {
	PublicKeyFile string `json:"public_key_file"`
	ValueFile     string `json:"value_file"`
	Signer        struct {
		PublicKey string `json:"public_key"`
		Algorithm uint64 `json:"algorithm"`
		KeyID     string `json:"key_id"`
	} `json:"signer"`
	SubjectFields struct {
		HeaderIdentifier string `json:"header_identifier"`
		FrameIdentifier  string `json:"frame_identifier"`
	} `json:"subject_fields"`
	Result struct {
		Subject     string `json:"subject"`
		Message     string `json:"message"`
		Value       string `json:"value"`
		MPrime      string `json:"m_prime"`
		ValueLength int    `json:"value_length"`
		ValueSHA256 string `json:"value_sha256"`
	} `json:"result"`
}

// loadSignCase reads the single case of the P6 reproduction vector `name`.
func loadSignCase(t *testing.T, name string) signCase {
	t.Helper()
	text, err := os.ReadFile(filepath.Join("..", "..", "vectors", "v2", "P6", name+".json"))
	if err != nil {
		t.Fatal(err)
	}
	var file struct {
		Cases []signCase `json:"cases"`
	}
	if err := json.Unmarshal(text, &file); err != nil {
		t.Fatal(err)
	}
	if len(file.Cases) != 1 {
		t.Fatalf("%s: want one case, have %d", name, len(file.Cases))
	}
	return file.Cases[0]
}

func vectorBytes(t *testing.T, name string) []byte {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("..", "..", "vectors", "v2", "P6", name))
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func mustHex(t *testing.T, value string) []byte {
	t.Helper()
	data, err := hex.DecodeString(value)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func mustDigest(t *testing.T, value string) Digest {
	t.Helper()
	var digest Digest
	copy(digest[:], mustHex(t, value))
	return digest
}

// TestEd25519ReproductionMatchesTheVector checks the key identifier, the subject
// preimage, the signed message and the strict verification of the Ed25519 vector.
func TestEd25519ReproductionMatchesTheVector(t *testing.T) {
	c := loadSignCase(t, "sign-ed25519-reproduce")
	publicKey := mustHex(t, c.Signer.PublicKey)
	keyID := KeyIdentifier(Ed25519Algorithm, publicKey)
	if keyID != mustDigest(t, c.Signer.KeyID) {
		t.Fatalf("key id %x, want %s", keyID, c.Signer.KeyID)
	}
	h0 := mustDigest(t, c.SubjectFields.HeaderIdentifier)
	frame := mustDigest(t, c.SubjectFields.FrameIdentifier)
	record := SignatureRecord{Algorithm: Ed25519Algorithm, KeyID: keyID, Value: mustHex(t, c.Result.Value)}
	if got := hex.EncodeToString(SubjectPreimage(record, h0, frame)); got != c.Result.Subject {
		t.Fatalf("subject %s, want %s", got, c.Result.Subject)
	}
	if message := SignedMessage(record, h0, frame); hex.EncodeToString(message[:]) != c.Result.Message {
		t.Fatalf("message %x, want %s", message, c.Result.Message)
	}
	held := []HeldKey{{KeyID: keyID, Algorithm: Ed25519Algorithm, PublicKey: publicKey}}
	got := VerifySignature(record, h0, frame, held)
	if got.Status != SignatureValid || got.Reason != "" {
		t.Fatalf("honest Ed25519 signature: %+v", got)
	}
}

// TestCompositeReproductionVerifies checks that the composite vector's value
// verifies over its representative, and that the representative and the value
// match the recorded bytes.
func TestCompositeReproductionVerifies(t *testing.T) {
	c := loadSignCase(t, "sign-composite-reproduce")
	publicKey := vectorBytes(t, c.PublicKeyFile)
	value := vectorBytes(t, c.ValueFile)
	keyID := KeyIdentifier(CompositeAlgorithm, publicKey)
	if keyID != mustDigest(t, c.Signer.KeyID) {
		t.Fatalf("composite key id %x, want %s", keyID, c.Signer.KeyID)
	}
	h0 := mustDigest(t, c.SubjectFields.HeaderIdentifier)
	frame := mustDigest(t, c.SubjectFields.FrameIdentifier)
	record := SignatureRecord{Algorithm: CompositeAlgorithm, KeyID: keyID, Value: value}
	if got := hex.EncodeToString(SubjectPreimage(record, h0, frame)); got != c.Result.Subject {
		t.Fatalf("subject %s, want %s", got, c.Result.Subject)
	}
	message := SignedMessage(record, h0, frame)
	if got := hex.EncodeToString(compositeRepresentative(message[:])); got != c.Result.MPrime {
		t.Fatalf("representative %s, want %s", got, c.Result.MPrime)
	}
	if len(value) != c.Result.ValueLength {
		t.Fatalf("value length %d, want %d", len(value), c.Result.ValueLength)
	}
	if digest := sha256.Sum256(value); hex.EncodeToString(digest[:]) != c.Result.ValueSHA256 {
		t.Fatalf("value digest %x, want %s", digest, c.Result.ValueSHA256)
	}
	held := []HeldKey{{KeyID: keyID, Algorithm: CompositeAlgorithm, PublicKey: publicKey}}
	if got := VerifySignature(record, h0, frame, held); got.Status != SignatureValid {
		t.Fatalf("honest composite signature: %+v", got)
	}
}

// compositeFixture returns the honest composite record and the held key of the
// reproduction vector, so each negative case alters one part of a valid signature.
func compositeFixture(t *testing.T) (SignatureRecord, Digest, Digest, []HeldKey) {
	t.Helper()
	c := loadSignCase(t, "sign-composite-reproduce")
	publicKey := vectorBytes(t, c.PublicKeyFile)
	keyID := KeyIdentifier(CompositeAlgorithm, publicKey)
	record := SignatureRecord{
		Algorithm: CompositeAlgorithm,
		KeyID:     keyID,
		Value:     vectorBytes(t, c.ValueFile),
	}
	held := []HeldKey{{KeyID: keyID, Algorithm: CompositeAlgorithm, PublicKey: publicKey}}
	return record, mustDigest(t, c.SubjectFields.HeaderIdentifier), mustDigest(t, c.SubjectFields.FrameIdentifier), held
}

// TestCompositeRefusesEachAlteredComponent: a composite signature is valid only
// when both halves hold, and the reason names the half that fails.
func TestCompositeRefusesEachAlteredComponent(t *testing.T) {
	cases := []struct {
		name   string
		alter  func(value []byte) []byte
		reason SignatureReason
	}{
		{
			name: "ML-DSA half zeroed",
			alter: func(value []byte) []byte {
				out := bytes.Clone(value)
				clear(out[:mldsa65ValueLength])
				return out
			},
			reason: ReasonCompositePostQuantum,
		},
		{
			name: "Ed25519 half zeroed",
			alter: func(value []byte) []byte {
				out := bytes.Clone(value)
				clear(out[mldsa65ValueLength:])
				return out
			},
			reason: ReasonCompositeClassical,
		},
		{
			name: "ML-DSA half stripped",
			alter: func(value []byte) []byte {
				return value[mldsa65ValueLength:]
			},
			reason: ReasonValueLength,
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			record, h0, frame, held := compositeFixture(t)
			record.Value = tc.alter(record.Value)
			got := VerifySignature(record, h0, frame, held)
			if got.Status != SignatureInvalid || got.Reason != tc.reason {
				t.Fatalf("got %+v, want invalid %s", got, tc.reason)
			}
		})
	}
}

// TestCompositeRefusesAKeyOfTheWrongAlgorithm: a held key whose bytes do not
// match the record's key identifier is refused as a mismatch, not verified.
func TestCompositeRefusesAKeyOfTheWrongAlgorithm(t *testing.T) {
	record, h0, frame, held := compositeFixture(t)
	held[0].Algorithm = Ed25519Algorithm
	if got := VerifySignature(record, h0, frame, held); got.Reason != ReasonKeyMismatch {
		t.Fatalf("got %+v, want key_mismatch", got)
	}
}
