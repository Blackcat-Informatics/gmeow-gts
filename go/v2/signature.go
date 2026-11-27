// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"crypto/ed25519"
	"crypto/mldsa"
	"crypto/sha512"
	"encoding/binary"
	"encoding/hex"
)

// Signature algorithms of registry version 1 (P6 section 3.1).
const (
	Ed25519Algorithm   uint64 = 1
	CompositeAlgorithm uint64 = 2
)

const (
	mldsa65KeyLength     = 1952
	mldsa65ValueLength   = 3309
	ed25519KeyLength     = 32
	ed25519ValueLength   = 64
	compositeKeyLength   = mldsa65KeyLength + ed25519KeyLength
	compositeValueLength = mldsa65ValueLength + ed25519ValueLength
)

var (
	compositePrefix = []byte("CompositeAlgorithmSignatures2025")
	compositeLabel  = []byte("COMPSIG-MLDSA65-Ed25519-SHA512")
)

// groupOrder is the group order L of Ed25519 as 32 little-endian bytes.
var groupOrder = [32]byte{
	0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2,
	0xde, 0xf9, 0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
}

// smallOrderEncodings holds the eight encodings of the points of small order
// (P6 section 4.1), in lower-case hexadecimal.
var smallOrderEncodings = map[string]bool{
	"0100000000000000000000000000000000000000000000000000000000000000": true,
	"ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f": true,
	"0000000000000000000000000000000000000000000000000000000000000000": true,
	"0000000000000000000000000000000000000000000000000000000000000080": true,
	"26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05": true,
	"26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85": true,
	"c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a": true,
	"c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa": true,
}

// SignatureStatus is the status of one signature (P6 section 6.1).
type SignatureStatus string

// Signature statuses.
const (
	SignatureValid      SignatureStatus = "valid"
	SignatureInvalid    SignatureStatus = "invalid"
	SignatureUnverified SignatureStatus = "unverified"
)

// SignatureReason is why a signature is not valid (P2, the reason enumeration).
type SignatureReason string

// Signature reasons.
const (
	ReasonUnknownAlgorithm     SignatureReason = "unknown_signature_algorithm"
	ReasonKeyNotHeld           SignatureReason = "key_not_held"
	ReasonKeyMismatch          SignatureReason = "key_mismatch"
	ReasonValueLength          SignatureReason = "value_length"
	ReasonNonCanonicalScalar   SignatureReason = "non_canonical_scalar"
	ReasonSmallOrderKey        SignatureReason = "small_order_key"
	ReasonSmallOrderNonce      SignatureReason = "small_order_nonce"
	ReasonEquationFailed       SignatureReason = "equation_failed"
	ReasonCompositeClassical   SignatureReason = "composite_classical_failed"
	ReasonCompositePostQuantum SignatureReason = "composite_post_quantum_failed"
)

// HeldKey is a public key a verifier can check a signature with (P6 section 3.3).
type HeldKey struct {
	KeyID     Digest
	Algorithm uint64
	PublicKey []byte
}

// SignatureRecord is one signature record of a frame: algorithm, key identifier and value.
type SignatureRecord struct {
	Algorithm uint64
	KeyID     Digest
	Value     []byte
}

// SignatureResult is the outcome of one signature of one frame (P6 section 6.1).
// Reason is empty exactly when Status is SignatureValid.
type SignatureResult struct {
	Status    SignatureStatus
	Reason    SignatureReason
	Algorithm uint64
	KeyID     Digest
}

// KeyIdentifier returns H_ctx("gts2 key id", E([algorithm, public_key])) (P6 section 3.2).
func KeyIdentifier(algorithm uint64, publicKey []byte) Digest {
	return hash("gts2 key id", cborEncode(algorithm, publicKey))
}

// SubjectPreimage returns E([algorithm, key_id, h0, frame_identifier]), the
// deterministic encoding that P6 section 2 hashes into the signed message.
func SubjectPreimage(record SignatureRecord, h0, frame Digest) []byte {
	return cborEncode(record.Algorithm, record.KeyID[:], h0[:], frame[:])
}

// SignedMessage returns H_ctx("gts2 signature subject", E(subject)), the message
// that each algorithm signs (P6 section 2).
func SignedMessage(record SignatureRecord, h0, frame Digest) Digest {
	return hash("gts2 signature subject", SubjectPreimage(record, h0, frame))
}

// cborHead returns the head of a CBOR item of major type `major` with `argument`
// in its shortest form (RFC 8949 section 3).
func cborHead(major byte, argument uint64) []byte {
	m := major << 5
	switch {
	case argument < 24:
		return []byte{m | byte(argument)}
	case argument <= 0xff:
		return []byte{m | 24, byte(argument)}
	case argument <= 0xffff:
		return []byte{m | 25, byte(argument >> 8), byte(argument)}
	case argument <= 0xffffffff:
		return binary.BigEndian.AppendUint32([]byte{m | 26}, uint32(argument))
	default:
		return binary.BigEndian.AppendUint64([]byte{m | 27}, argument)
	}
}

// cborEncode encodes an array of unsigned integers and byte strings, the only
// shapes the signature preimages take. Any other item is a caller error.
func cborEncode(items ...any) []byte {
	out := cborHead(4, uint64(len(items)))
	for _, item := range items {
		switch value := item.(type) {
		case uint64:
			out = append(out, cborHead(0, value)...)
		case []byte:
			out = append(out, cborHead(2, uint64(len(value)))...)
			out = append(out, value...)
		default:
			// INVARIANT: every caller passes uint64 or []byte items.
			panic("v2: unsupported item in a signature preimage")
		}
	}
	return out
}

// ed25519Strict is `Ed25519-Strict` of P6 section 4.1: the first failing check of
// the policy, or the empty reason when the signature holds. Lengths are checked
// by the caller.
func ed25519Strict(publicKey, message, value []byte) SignatureReason {
	nonce := value[:32]
	scalar := value[32:64]
	if !belowGroupOrder(scalar) {
		return ReasonNonCanonicalScalar
	}
	if smallOrderEncodings[hex.EncodeToString(publicKey)] {
		return ReasonSmallOrderKey
	}
	if smallOrderEncodings[hex.EncodeToString(nonce)] {
		return ReasonSmallOrderNonce
	}
	if !ed25519.Verify(publicKey, message, value) {
		return ReasonEquationFailed
	}
	return ""
}

// belowGroupOrder reports whether the 32-byte little-endian integer is below L.
func belowGroupOrder(scalar []byte) bool {
	for index := len(scalar) - 1; index >= 0; index-- {
		if scalar[index] != groupOrder[index] {
			return scalar[index] < groupOrder[index]
		}
	}
	return false
}

// compositeRepresentative is M' of the composite construction: the prefix, the
// label, a zero-length context and the SHA-512 digest of the message (P6 section 4.2).
func compositeRepresentative(message []byte) []byte {
	digest := sha512.Sum512(message)
	representative := make([]byte, 0, len(compositePrefix)+len(compositeLabel)+1+len(digest))
	representative = append(representative, compositePrefix...)
	representative = append(representative, compositeLabel...)
	representative = append(representative, 0)
	return append(representative, digest[:]...)
}

// compositeStrict is `Composite-Strict` of P6 section 4.2: the Ed25519 half over
// M', then the ML-DSA-65 half over M' with the label as its context. Lengths are
// checked by the caller. It returns the empty reason when the signature holds.
func compositeStrict(publicKey, message, value []byte) SignatureReason {
	representative := compositeRepresentative(message)
	edKey := publicKey[mldsa65KeyLength:]
	if ed25519Strict(edKey, representative, value[mldsa65ValueLength:]) != "" {
		return ReasonCompositeClassical
	}
	key, err := mldsa.NewPublicKey(mldsa.MLDSA65(), publicKey[:mldsa65KeyLength])
	if err != nil {
		return ReasonCompositePostQuantum
	}
	options := &mldsa.Options{Context: string(compositeLabel)}
	if mldsa.Verify(key, representative, value[:mldsa65ValueLength], options) != nil {
		return ReasonCompositePostQuantum
	}
	return ""
}

// VerifySignature runs `Verify(signature, frame)` of P6 section 6.2 for one
// signature. `h0` is the segment's opener identifier, `frame` the frame's
// identifier, and `keys` the keys the verifier holds. The outcome is that of the
// first step that gives one.
func VerifySignature(record SignatureRecord, h0, frame Digest, keys []HeldKey) SignatureResult {
	result := func(status SignatureStatus, reason SignatureReason) SignatureResult {
		return SignatureResult{Status: status, Reason: reason, Algorithm: record.Algorithm, KeyID: record.KeyID}
	}
	var keyLength, valueLength int
	switch record.Algorithm {
	case Ed25519Algorithm:
		keyLength, valueLength = ed25519KeyLength, ed25519ValueLength
	case CompositeAlgorithm:
		keyLength, valueLength = compositeKeyLength, compositeValueLength
	default:
		return result(SignatureUnverified, ReasonUnknownAlgorithm)
	}
	filed := false
	var key *HeldKey
	for index := range keys {
		candidate := &keys[index]
		if candidate.KeyID != record.KeyID {
			continue
		}
		filed = true
		if candidate.Algorithm == record.Algorithm &&
			KeyIdentifier(candidate.Algorithm, candidate.PublicKey) == record.KeyID {
			key = candidate
			break
		}
	}
	if !filed {
		return result(SignatureUnverified, ReasonKeyNotHeld)
	}
	if key == nil {
		return result(SignatureInvalid, ReasonKeyMismatch)
	}
	if len(key.PublicKey) != keyLength || len(record.Value) != valueLength {
		return result(SignatureInvalid, ReasonValueLength)
	}
	message := SignedMessage(record, h0, frame)
	var reason SignatureReason
	if record.Algorithm == Ed25519Algorithm {
		reason = ed25519Strict(key.PublicKey, message[:], record.Value)
	} else {
		reason = compositeStrict(key.PublicKey, message[:], record.Value)
	}
	if reason != "" {
		return result(SignatureInvalid, reason)
	}
	return result(SignatureValid, "")
}
