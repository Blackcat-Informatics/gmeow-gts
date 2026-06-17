// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package cose implements the GTS COSE_Sign1 subset (detached payload,
// EdDSA/Ed25519) over a frame id — GTS-SPEC §9.2. It is byte-compatible with the
// Python reference and gated by vectors/cose/*.json. Ed25519 is deterministic
// (RFC 8032), so the same key + id always yields the same signature.
package cose

import (
	"crypto/ed25519"

	"github.com/fxamacker/cbor/v2"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

const (
	algLabel = 1
	kidLabel = 4
	algEdDSA = -8
	tagSign1 = 18
)

// SigStatus is the verification outcome for a detached COSE_Sign1.
type SigStatus int

const (
	// Invalid: present but malformed or failed verification.
	Invalid SigStatus = iota
	// Valid: cryptographically valid under the resolved key.
	Valid
	// Unverified: well-formed, but no key was resolved to check it.
	Unverified
)

func protectedHeader() []byte {
	return wire.MustEncode(map[int]int{algLabel: algEdDSA})
}

// sigStructure is the COSE Sig_structure to be signed/verified (RFC 9052 §4.4).
func sigStructure(protected, frameID []byte) []byte {
	return wire.MustEncode([]interface{}{"Signature1", protected, []byte{}, frameID})
}

// SignID returns a detached COSE_Sign1 over frameID with the given Ed25519 key.
func SignID(frameID []byte, priv ed25519.PrivateKey, kid string) []byte {
	protected := protectedHeader()
	signature := ed25519.Sign(priv, sigStructure(protected, frameID))
	c := cbor.Tag{
		Number: tagSign1,
		Content: []interface{}{
			protected,
			map[int]interface{}{kidLabel: []byte(kid)},
			nil,
			signature,
		},
	}
	return wire.MustEncode(c)
}

// Parse extracts (kid, protected, signature) from a COSE_Sign1, ok=false if malformed.
func Parse(sig []byte) (kid string, protected, signature []byte, ok bool) {
	var tag cbor.Tag
	if err := cbor.Unmarshal(sig, &tag); err != nil {
		return "", nil, nil, false
	}
	arr, isArr := tag.Content.([]interface{})
	if !isArr || len(arr) != 4 {
		return "", nil, nil, false
	}
	protected, ok = arr[0].([]byte)
	if !ok {
		return "", nil, nil, false
	}
	signature, ok = arr[3].([]byte)
	if !ok {
		return "", nil, nil, false
	}
	unprotected, ok := arr[1].(map[interface{}]interface{})
	if !ok {
		return "", nil, nil, false
	}
	// CBOR integer map keys decode as uint64/int64/int depending on the codec;
	// match the kid label (4) numerically.
	for k, v := range unprotected {
		var key int64
		switch kk := k.(type) {
		case uint64:
			key = int64(kk)
		case int64:
			key = kk
		case int:
			key = int64(kk)
		default:
			continue
		}
		if key == kidLabel {
			if b, isBytes := v.([]byte); isBytes {
				return string(b), protected, signature, true
			}
		}
	}
	return "", nil, nil, false
}

// SignatureKID returns the kid of a COSE_Sign1 (for key lookup).
func SignatureKID(sig []byte) (string, bool) {
	kid, _, _, ok := Parse(sig)
	return kid, ok
}

// VerifySig verifies a detached COSE_Sign1 over frameID against pub.
func VerifySig(sig, frameID []byte, pub ed25519.PublicKey) SigStatus {
	_, protected, signature, ok := Parse(sig)
	if !ok || len(signature) != ed25519.SignatureSize {
		return Invalid
	}
	if ed25519.Verify(pub, sigStructure(protected, frameID), signature) {
		return Valid
	}
	return Invalid
}

// VerifySignatures verifies the COSE signatures recorded in a folded graph
// against keys resolved by kid. It updates each signature's Kid and Status in
// place: "valid"/"invalid" when a key resolves, "unverified" otherwise (§9.2).
func VerifySignatures(sigs []model.Signature, resolve func(kid string) (ed25519.PublicKey, bool)) {
	for i := range sigs {
		if sigs[i].Cose == nil {
			continue
		}
		kid, ok := SignatureKID(sigs[i].Cose)
		if !ok {
			sigs[i].Status = "invalid"
			continue
		}
		sigs[i].Kid = kid
		pub, found := resolve(kid)
		if !found {
			sigs[i].Status = "unverified"
			continue
		}
		if VerifySig(sigs[i].Cose, sigs[i].FrameID, pub) == Valid {
			sigs[i].Status = "valid"
		} else {
			sigs[i].Status = "invalid"
		}
	}
}
