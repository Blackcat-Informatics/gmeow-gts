// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// COSE_Encrypt0 (AES-256-GCM, keyed by kid) — GTS-SPEC §9.3. Byte-compatible
// with the Python reference and gated by vectors/encrypt0/basic.json. Unlike
// signing, encryption uses a random 12-byte IV, so production sealing is not
// reproducible; the fixed-IV Encrypt0WithIV transform is what the vector freezes.

package cose

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"

	"github.com/fxamacker/cbor/v2"

	"go.blackcatinformatics.ca/gts/wire"
)

const (
	ivLabel     = 5
	algA256GCM  = 3
	tagEncrypt0 = 16
)

// Errors returned by Decrypt0. They mirror the Python reference's failure modes.
var (
	// ErrMalformed: the COSE_Encrypt0 structure could not be parsed.
	ErrMalformed = errors.New("malformed COSE_Encrypt0")
	// ErrMissingKey: no content key was resolved for the recipient kid.
	ErrMissingKey = errors.New("no content key for recipient")
	// ErrAuthFailed: AES-GCM authentication failed (wrong key or tampering).
	ErrAuthFailed = errors.New("authentication failed (AES-GCM tag mismatch)")
)

func encrypt0Protected() []byte {
	return wire.MustEncode(map[int]int{algLabel: algA256GCM})
}

// encStructure is the COSE Enc_structure bound as AAD (RFC 9052 §5.3).
func encStructure(protected []byte) []byte {
	return wire.MustEncode([]interface{}{"Encrypt0", protected, []byte{}})
}

// Encrypt0WithIV seals plaintext as a COSE_Encrypt0 with an explicit 12-byte iv
// (§9.3). The split-out IV keeps the transform deterministic so it can be frozen
// in vectors/encrypt0; Encrypt0 is the production entry point with a random IV.
func Encrypt0WithIV(plaintext []byte, kid string, key, iv []byte) ([]byte, error) {
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	protected := encrypt0Protected()
	aad := encStructure(protected)
	ciphertext := aead.Seal(nil, iv, plaintext, aad)
	c := cbor.Tag{
		Number: tagEncrypt0,
		Content: []interface{}{
			protected,
			// Canonical encoding sorts the unprotected keys: kid (4) before iv (5).
			map[int]interface{}{kidLabel: []byte(kid), ivLabel: iv},
			ciphertext,
		},
	}
	return wire.MustEncode(c), nil
}

// Encrypt0 seals plaintext as a COSE_Encrypt0 to the recipient kid (§9.3),
// drawing a fresh random 12-byte IV from crypto/rand.
func Encrypt0(plaintext []byte, kid string, key []byte) ([]byte, error) {
	iv := make([]byte, 12)
	if _, err := rand.Read(iv); err != nil {
		return nil, err
	}
	return Encrypt0WithIV(plaintext, kid, key, iv)
}

// parseEncrypt0 extracts the cleartext fields of a COSE_Encrypt0.
func parseEncrypt0(blob []byte) (kid string, protected, iv, ciphertext []byte, ok bool) {
	var tag cbor.Tag
	if err := cbor.Unmarshal(blob, &tag); err != nil {
		return "", nil, nil, nil, false
	}
	arr, isArr := tag.Content.([]interface{})
	if !isArr || len(arr) != 3 {
		return "", nil, nil, nil, false
	}
	protected, ok = arr[0].([]byte)
	if !ok {
		return "", nil, nil, nil, false
	}
	ciphertext, ok = arr[2].([]byte)
	if !ok {
		return "", nil, nil, nil, false
	}
	unprotected, isMap := arr[1].(map[interface{}]interface{})
	if !isMap {
		return "", nil, nil, nil, false
	}
	// CBOR integer map keys decode as uint64/int64/int; match labels numerically.
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
		b, isBytes := v.([]byte)
		if !isBytes {
			continue
		}
		switch key {
		case kidLabel:
			kid = string(b)
		case ivLabel:
			iv = b
		}
	}
	if kid == "" || iv == nil {
		return "", nil, nil, nil, false
	}
	return kid, protected, iv, ciphertext, true
}

// RecipientKID returns the recipient kid of a COSE_Encrypt0 (for key lookup).
func RecipientKID(blob []byte) (string, bool) {
	kid, _, _, _, ok := parseEncrypt0(blob)
	return kid, ok
}

// Decrypt0 opens a COSE_Encrypt0 using a content key resolved by kid (§9.3).
func Decrypt0(blob []byte, resolve func(kid string) ([]byte, bool)) ([]byte, error) {
	kid, protected, iv, ciphertext, ok := parseEncrypt0(blob)
	if !ok {
		return nil, ErrMalformed
	}
	key, found := resolve(kid)
	if !found || len(key) != 32 {
		return nil, ErrMissingKey
	}
	if len(iv) != 12 {
		return nil, ErrMalformed
	}
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, ErrMissingKey
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, ErrMalformed
	}
	aad := encStructure(protected)
	plaintext, err := aead.Open(nil, iv, ciphertext, aad)
	if err != nil {
		return nil, ErrAuthFailed
	}
	return plaintext, nil
}
