// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Package cose implements the GTS COSE_Sign1 subset (detached payload,
// EdDSA/Ed25519) over a frame id — GTS-SPEC §9.2. It is byte-compatible with the
// Python reference and gated by vectors/cose/*.json. Ed25519 is deterministic
// (RFC 8032), so the same key + id always yields the same signature.
// Verification dispatches on the algorithm declared in the protected header:
// EdDSA is checked as Ed25519, any other algorithm is left unverified.
package cose

import (
	"bytes"
	"crypto/ed25519"
	"math"
	"unicode/utf8"

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
	// Invalid means the signature is present but malformed or failed verification.
	Invalid SigStatus = iota
	// Valid means the signature is cryptographically valid under the resolved key.
	Valid
	// Unverified means the signature is well-formed, but was not checked: no key
	// was resolved, or the declared algorithm is not supported by this engine.
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

// maxHeaderDepth is the deepest nesting a protected header may have; the header
// map is level 1.
const maxHeaderDepth = 8

// scanHead reads one item head at pos: major type, additional info, argument and
// the next position. ok=false for an indefinite length, a reserved additional
// info, a truncated head, or an argument that is not in shortest form.
func scanHead(buf []byte, pos int) (major, info byte, arg uint64, next int, ok bool) {
	if pos >= len(buf) {
		return 0, 0, 0, 0, false
	}
	major, info = buf[pos]>>5, buf[pos]&0x1f
	width := 0
	switch {
	case info < 24:
		return major, info, uint64(info), pos + 1, true
	case info == 24:
		width = 1
	case info == 25:
		width = 2
	case info == 26:
		width = 4
	case info == 27:
		width = 8
	default:
		return 0, 0, 0, 0, false
	}
	if len(buf)-pos-1 < width {
		return 0, 0, 0, 0, false
	}
	for _, b := range buf[pos+1 : pos+1+width] {
		arg = arg<<8 | uint64(b)
	}
	minimum := [...]uint64{24, 1 << 8, 1 << 16, 1 << 32}[info-24]
	if arg < minimum {
		return 0, 0, 0, 0, false
	}
	return major, info, arg, pos + 1 + width, true
}

// scanHeaderItem checks one item of a protected header against the grammar and
// returns the position after it. depth is the level of this item if it is a
// container.
func scanHeaderItem(buf []byte, pos, depth int, isKey bool) (end int, ok bool) {
	major, info, arg, next, ok := scanHead(buf, pos)
	if !ok || (isKey && major != 0 && major != 1 && major != 3) {
		return 0, false
	}
	switch major {
	case 0, 1:
		// Integers lie in -2^63 .. 2^63-1.
		return next, arg <= math.MaxInt64
	case 2, 3:
		if arg > uint64(len(buf)-next) {
			return 0, false
		}
		end = next + int(arg) // #nosec G115 -- bounded by len(buf)
		if major == 3 && !utf8.Valid(buf[next:end]) {
			return 0, false
		}
		return end, true
	case 4, 5:
		if depth > maxHeaderDepth {
			return 0, false
		}
		at := next
		var previous []byte
		for i := uint64(0); i < arg; i++ {
			if major == 4 {
				if at, ok = scanHeaderItem(buf, at, depth+1, false); !ok {
					return 0, false
				}
				continue
			}
			keyEnd, keyOK := scanHeaderItem(buf, at, depth+1, true)
			if !keyOK {
				return 0, false
			}
			key := buf[at:keyEnd]
			// Strictly ascending bytewise: sorted, and no duplicate key.
			if previous != nil && bytes.Compare(previous, key) >= 0 {
				return 0, false
			}
			previous = key
			if at, ok = scanHeaderItem(buf, keyEnd, depth+1, false); !ok {
				return 0, false
			}
		}
		return at, true
	case 7:
		// null, true and false only.
		return next, info >= 20 && info <= 22
	}
	return 0, false
}

// declaredAlgorithm returns the alg (label 1) of a protected header.
//
// The protected header is a byte string whose content is deterministically
// encoded CBOR (definite lengths, shortest-form integers and lengths, map keys in
// bytewise order of their encodings, no duplicate keys, no trailing bytes)
// consisting of exactly one map. Every map key, at any depth, is an integer or a
// text string. Every value, at any depth, is an integer, a byte string, a text
// string, an array, a map, null, true or false. Every integer, key or value, lies
// in the range -2^63 to 2^63-1 inclusive. Text strings are valid UTF-8. Nesting
// is at most 8 levels deep, counting the header map itself as level 1. The map
// contains the integer key 1, whose value is an integer or a text string. A
// header that does not meet all of this is malformed and the signature is
// invalid.
//
// Enforced by scanning the raw bytes (scanHeaderItem), so no decoder behaviour
// (float, tag, bignum, depth or width limit) is involved. The algorithm is an
// int64 or a string; ok=false if malformed or no algorithm is declared.
func declaredAlgorithm(protected []byte) (algorithm interface{}, ok bool) {
	if len(protected) == 0 || protected[0]>>5 != 5 {
		return nil, false
	}
	if end, valid := scanHeaderItem(protected, 0, 1, false); !valid || end != len(protected) {
		return nil, false
	}
	_, _, count, at, _ := scanHead(protected, 0)
	for i := uint64(0); i < count; i++ {
		keyEnd, _ := scanHeaderItem(protected, at, 2, true)
		valueEnd, _ := scanHeaderItem(protected, keyEnd, 2, false)
		if bytes.Equal(protected[at:keyEnd], []byte{0x01}) {
			major, _, arg, next, _ := scanHead(protected, keyEnd)
			switch major {
			case 0:
				return int64(arg), true // #nosec G115 -- scanned: arg <= MaxInt64
			case 1:
				return -1 - int64(arg), true // #nosec G115 -- scanned: arg <= MaxInt64
			case 3:
				return string(protected[next:valueEnd]), true
			}
			return nil, false
		}
		at = valueEnd
	}
	return nil, false
}

// Parse extracts (kid, algorithm, protected, signature) from a COSE_Sign1,
// ok=false if malformed: not a four-element array, a protected header that is
// not a bstr holding a map that declares an alg, a non-bstr signature, or no
// kid. algorithm is the declared COSE algorithm identifier, an int64 or a string.
func Parse(sig []byte) (kid string, algorithm interface{}, protected, signature []byte, ok bool) {
	var tag cbor.Tag
	if err := cbor.Unmarshal(sig, &tag); err != nil {
		return "", nil, nil, nil, false
	}
	arr, isArr := tag.Content.([]interface{})
	if !isArr || len(arr) != 4 {
		return "", nil, nil, nil, false
	}
	protected, ok = arr[0].([]byte)
	if !ok {
		return "", nil, nil, nil, false
	}
	algorithm, ok = declaredAlgorithm(protected)
	if !ok {
		return "", nil, nil, nil, false
	}
	signature, ok = arr[3].([]byte)
	if !ok {
		return "", nil, nil, nil, false
	}
	unprotected, ok := arr[1].(map[interface{}]interface{})
	if !ok {
		return "", nil, nil, nil, false
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
				return string(b), algorithm, protected, signature, true
			}
		}
	}
	return "", nil, nil, nil, false
}

// SignatureKID returns the kid of a COSE_Sign1 (for key lookup).
func SignatureKID(sig []byte) (string, bool) {
	kid, _, _, _, ok := Parse(sig)
	return kid, ok
}

// VerifySig verifies a detached COSE_Sign1 over frameID against pub,
// dispatching on the algorithm declared in the protected header (§9.2): EdDSA is
// verified as Ed25519, any other algorithm is Unverified, and a malformed
// COSE_Sign1 is Invalid.
func VerifySig(sig, frameID []byte, pub ed25519.PublicKey) SigStatus {
	_, algorithm, protected, signature, ok := Parse(sig)
	if !ok {
		return Invalid
	}
	if algorithm != interface{}(int64(algEdDSA)) {
		return Unverified
	}
	if len(signature) != ed25519.SignatureSize {
		return Invalid
	}
	if ed25519.Verify(pub, sigStructure(protected, frameID), signature) {
		return Valid
	}
	return Invalid
}

// VerifySignatures verifies the COSE signatures recorded in a folded graph
// against keys resolved by kid. It updates each signature's Kid and Status in
// place: "valid"/"invalid" when an EdDSA signature's key resolves, "unverified"
// when no key resolves or the declared algorithm is not supported, and "invalid"
// for a malformed COSE_Sign1 (§9.2).
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
		switch VerifySig(sigs[i].Cose, sigs[i].FrameID, pub) {
		case Valid:
			sigs[i].Status = "valid"
		case Unverified:
			sigs[i].Status = "unverified"
		default:
			sigs[i].Status = "invalid"
		}
	}
}
