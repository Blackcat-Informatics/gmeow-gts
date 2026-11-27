// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
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

// heldReproductionKeys are the keys of the reproduction vectors, which the frame
// vectors named below are signed with.
func heldReproductionKeys(t *testing.T) []HeldKey {
	t.Helper()
	composite := loadSignCase(t, "sign-composite-reproduce")
	compositeKey := vectorBytes(t, composite.PublicKeyFile)
	ed := loadSignCase(t, "sign-ed25519-reproduce")
	edKey := mustHex(t, ed.Signer.PublicKey)
	return []HeldKey{
		{KeyID: KeyIdentifier(CompositeAlgorithm, compositeKey), Algorithm: CompositeAlgorithm, PublicKey: compositeKey},
		{KeyID: KeyIdentifier(Ed25519Algorithm, edKey), Algorithm: Ed25519Algorithm, PublicKey: edKey},
	}
}

// TestFrameSignaturesMatchTheVectors reads each frame vector through the reader with
// the reproduction keys, and checks the status and reason of its signature against
// the vector's recorded result. Each vector's signature is the first one of its frame.
func TestFrameSignaturesMatchTheVectors(t *testing.T) {
	cases := []struct {
		name   string
		status SignatureStatus
		reason SignatureReason
	}{
		{"composite-honest", SignatureValid, ""},
		{"composite-both", SignatureInvalid, ReasonCompositeClassical},
		{"composite-classical-scalar", SignatureInvalid, ReasonCompositeClassical},
		{"composite-classical-small-order", SignatureInvalid, ReasonCompositeClassical},
		{"composite-label-wrong", SignatureInvalid, ReasonCompositeClassical},
		{"composite-post-quantum", SignatureInvalid, ReasonCompositePostQuantum},
		{"composite-stripped-half", SignatureInvalid, ReasonCompositePostQuantum},
		{"ed25519-honest", SignatureValid, ""},
	}
	keys := heldReproductionKeys(t)
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			result := ReadWire(vectorBytes(t, tc.name+".bin"), DefaultReadCapacity(), keys...)
			var found *SignatureResult
			for i := range result.Segments {
				for _, item := range result.Segments[i].Items {
					if item.Frame != nil && len(item.Frame.Signatures) > 0 && found == nil {
						found = &item.Frame.Signatures[0]
					}
				}
			}
			if found == nil {
				t.Fatal("no frame carries a signature result")
			}
			if found.Status != tc.status || found.Reason != tc.reason {
				t.Fatalf("got %s %q, want %s %q", found.Status, found.Reason, tc.status, tc.reason)
			}
		})
	}
}

// smallOrderEncodingList is the eight encodings of the points of small order (P6 section 4.1).
var smallOrderEncodingList = []string{
	"0100000000000000000000000000000000000000000000000000000000000000",
	"ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
	"0000000000000000000000000000000000000000000000000000000000000000",
	"0000000000000000000000000000000000000000000000000000000000000080",
	"26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
	"26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
	"c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
	"c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
}

// TestEd25519SmallOrderKeysAreRefused: an Ed25519 signature whose held key is one of the
// eight points of small order is refused as small_order_key, whatever its value, and the
// corpus gives that outcome for each of the eight vectors.
func TestEd25519SmallOrderKeysAreRefused(t *testing.T) {
	var keys []HeldKey
	for _, encoding := range smallOrderEncodingList {
		point := mustHex(t, encoding)
		keys = append(keys, HeldKey{KeyID: KeyIdentifier(Ed25519Algorithm, point), Algorithm: Ed25519Algorithm, PublicKey: point})
	}
	for index := 1; index <= 8; index++ {
		name := fmt.Sprintf("ed25519-key-small-order-%d", index)
		result := ReadWire(vectorBytes(t, name+".bin"), DefaultReadCapacity(), keys...)
		var found *SignatureResult
		for i := range result.Segments {
			for _, item := range result.Segments[i].Items {
				if item.Frame != nil && len(item.Frame.Signatures) > 0 && found == nil {
					found = &item.Frame.Signatures[0]
				}
			}
		}
		if found == nil {
			t.Fatalf("%s: no frame carries a signature result", name)
		}
		if found.Status != SignatureInvalid || found.Reason != ReasonSmallOrderKey {
			t.Fatalf("%s: got %s %q, want invalid small_order_key", name, found.Status, found.Reason)
		}
	}
}

// frameSignatureOutcomes returns each signature result of each frame of the vector, in order.
func frameSignatureOutcomes(t *testing.T, name string, keys []HeldKey) [][2]string {
	t.Helper()
	result := ReadWire(vectorBytes(t, name), DefaultReadCapacity(), keys...)
	var outcomes [][2]string
	for i := range result.Segments {
		for _, item := range result.Segments[i].Items {
			if item.Frame == nil {
				continue
			}
			for _, signature := range item.Frame.Signatures {
				outcomes = append(outcomes, [2]string{string(signature.Status), string(signature.Reason)})
			}
		}
	}
	return outcomes
}

// TestEachRecordOfAFrameGetsItsOwnResult: a frame with one record of each algorithm, or two
// records of one held key, gives one result per record, in the order the records are listed.
func TestEachRecordOfAFrameGetsItsOwnResult(t *testing.T) {
	keys := heldReproductionKeys(t)
	valid := [2]string{string(SignatureValid), ""}
	cases := []struct {
		name string
		want [][2]string
	}{
		{"sig-record-valid.bin", [][2]string{valid, valid}},
		{"composite-hedged.both.bin", [][2]string{valid, valid}},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := frameSignatureOutcomes(t, tc.name, keys)
			if len(got) != len(tc.want) {
				t.Fatalf("got %d results %v, want %d", len(got), got, len(tc.want))
			}
			for i := range got {
				if got[i] != tc.want[i] {
					t.Fatalf("result %d: got %v, want %v", i, got[i], tc.want[i])
				}
			}
		})
	}
}

// TestRepeatedRecordsAreRefusedOnceAndGiveNoResult: two equal records in one list are
// unordered_or_repeated, and the frame has no signature result (P6 section 1).
func TestRepeatedRecordsAreRefusedOnceAndGiveNoResult(t *testing.T) {
	result := ReadWire(vectorBytes(t, "sig-duplicate-records.bin"), DefaultReadCapacity(), heldReproductionKeys(t)...)
	var frames int
	for i := range result.Segments {
		for _, item := range result.Segments[i].Items {
			if item.Frame == nil {
				continue
			}
			frames++
			if item.Frame.Malformed != UnorderedOrRepeated {
				t.Fatalf("frame malformed %q, want %q", item.Frame.Malformed, UnorderedOrRepeated)
			}
			if len(item.Frame.Signatures) != 0 {
				t.Fatalf("frame has %d signature results, want none", len(item.Frame.Signatures))
			}
		}
	}
	if frames != 1 {
		t.Fatalf("got %d frames, want 1", frames)
	}
}

// TestRecordsOfOneKeyAreEachJudged: two records of one held key in one list give two results,
// and each is judged alone (P6-5): a refused record does not change its neighbour's result.
func TestRecordsOfOneKeyAreEachJudged(t *testing.T) {
	keys := heldReproductionKeys(t)
	outcomes := frameSignatureOutcomes(t, "sig-same-key-two-values.bin", keys)
	if len(outcomes) != 2 {
		t.Fatalf("got %d results %v, want 2", len(outcomes), outcomes)
	}
	valid := 0
	for _, outcome := range outcomes {
		if outcome[0] == string(SignatureValid) {
			valid++
		}
	}
	if valid != 1 {
		t.Fatalf("got %d valid results in %v, want exactly one", valid, outcomes)
	}
}

// TestAValidRecordBesideAnUnheldKeyIsStillValid: a record under a key the verifier does not hold
// is unverified, and the valid record beside it keeps its own result.
func TestAValidRecordBesideAnUnheldKeyIsStillValid(t *testing.T) {
	outcomes := frameSignatureOutcomes(t, "sig-list-independence.bin", heldReproductionKeys(t))
	if len(outcomes) != 2 {
		t.Fatalf("got %d results %v, want 2", len(outcomes), outcomes)
	}
	if outcomes[0] != [2]string{string(SignatureUnverified), string(ReasonKeyNotHeld)} {
		t.Fatalf("first result %v, want unverified key_not_held", outcomes[0])
	}
	if outcomes[1] != [2]string{string(SignatureValid), ""} {
		t.Fatalf("second result %v, want valid", outcomes[1])
	}
}
