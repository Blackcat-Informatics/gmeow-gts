// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"crypto/mldsa"
	"fmt"
	"slices"
)

// SigningKey is a private key that the writer signs with. The two implementations are
// Ed25519Key and CompositeKey; the method set is closed to this package.
type SigningKey interface {
	// Algorithm returns the algorithm code of the signature the key makes.
	Algorithm() uint64
	// PublicKey returns the encoded public key whose identifier the signature names.
	PublicKey() []byte
	// sign returns the signature value over the signed message of P6 section 2.
	sign(message []byte) ([]byte, error)
}

// Ed25519Key is an Ed25519 private key that the writer signs with.
type Ed25519Key struct {
	key ed25519.PrivateKey
}

// NewEd25519Key returns the signing key of an Ed25519 private key. It fails unless the key has
// the length of an Ed25519 private key.
func NewEd25519Key(key ed25519.PrivateKey) (Ed25519Key, error) {
	if len(key) != ed25519.PrivateKeySize {
		return Ed25519Key{}, fmt.Errorf("an ed25519 private key is %d bytes, not %d", len(key), ed25519.PrivateKeySize)
	}
	return Ed25519Key{key: key}, nil
}

// Algorithm returns Ed25519Algorithm.
func (k Ed25519Key) Algorithm() uint64 { return Ed25519Algorithm }

// PublicKey returns the 32-byte public key.
func (k Ed25519Key) PublicKey() []byte { return k.key.Public().(ed25519.PublicKey) }

func (k Ed25519Key) sign(message []byte) ([]byte, error) {
	return ed25519.Sign(k.key, message), nil
}

// CompositeKey is a composite ML-DSA-65 and Ed25519 signing key (P6 section 4.2). Its signatures
// are deterministic: the ML-DSA-65 half is made with the composite label as its context and an
// empty randomiser, and the Ed25519 half is made over the same representative.
type CompositeKey struct {
	ml     *mldsa.PrivateKey
	ed     ed25519.PrivateKey
	public []byte
}

// NewCompositeKey returns the composite signing key of a 64-byte seed: the first 32 bytes seed the
// ML-DSA-65 key and the last 32 bytes seed the Ed25519 key. It fails unless the seed is 64 bytes.
func NewCompositeKey(seed []byte) (CompositeKey, error) {
	if len(seed) != 64 {
		return CompositeKey{}, fmt.Errorf("a composite seed is 64 bytes, not %d", len(seed))
	}
	ml, err := mldsa.NewPrivateKey(mldsa.MLDSA65(), seed[:32])
	if err != nil {
		return CompositeKey{}, err
	}
	ed := ed25519.NewKeyFromSeed(seed[32:])
	public := append(bytes.Clone(ml.PublicKey().Bytes()), ed.Public().(ed25519.PublicKey)...)
	return CompositeKey{ml: ml, ed: ed, public: public}, nil
}

// Algorithm returns the composite algorithm code.
func (k CompositeKey) Algorithm() uint64 { return CompositeAlgorithm }

// PublicKey returns the 1984-byte composite public key: the ML-DSA-65 key, then the Ed25519 key.
func (k CompositeKey) PublicKey() []byte { return k.public }

func (k CompositeKey) sign(message []byte) ([]byte, error) {
	representative := compositeRepresentative(message)
	mlHalf, err := k.ml.SignDeterministic(representative, &mldsa.Options{Context: string(compositeLabel)})
	if err != nil {
		return nil, err
	}
	return append(mlHalf, ed25519.Sign(k.ed, representative)...), nil
}

// AppendSigned writes a frame whose signatures element holds one `ed25519` signature by each
// key. It is AppendSignedWith for Ed25519 keys.
func (w *SegmentWriter) AppendSigned(frameType uint64, content, payload []byte, keys ...ed25519.PrivateKey) (Digest, error) {
	signers, err := ed25519Signers(keys)
	if err != nil {
		return Digest{}, err
	}
	return w.AppendSignedWith(frameType, content, payload, signers...)
}

// AppendSignedWith writes a frame whose signatures element holds one signature by each key, in
// the algorithm of that key. Every signature is made over the effective identifier of the frame,
// with the signed subject of P6 section 2, and is verified by VerifySignature before it is
// written (P6-29). A key may appear once (P6-5). The records are written in ascending bytewise
// order of their encodings (P6-52).
func (w *SegmentWriter) AppendSignedWith(frameType uint64, content, payload []byte, keys ...SigningKey) (Digest, error) {
	if frameType == parityType {
		return Digest{}, fmt.Errorf("a transparent frame carries no signature (P6-3)")
	}
	if err := checkFrame(frameType, content, payload); err != nil {
		return Digest{}, err
	}
	return w.append(frameType, content, payload, w.signer(keys))
}

// CommitSigned writes a commit over the current prefix with the Ed25519 signatures of
// AppendSigned.
func (w *SegmentWriter) CommitSigned(keys ...ed25519.PrivateKey) (Digest, error) {
	signers, err := ed25519Signers(keys)
	if err != nil {
		return Digest{}, err
	}
	return w.CommitSignedWith(signers...)
}

// CommitSignedWith writes a commit over the current prefix with the signatures of
// AppendSignedWith.
func (w *SegmentWriter) CommitSignedWith(keys ...SigningKey) (Digest, error) {
	if w.rangePrefix.count == 0 {
		return Digest{}, fmt.Errorf("initialize the segment writer with NewSegmentWriter")
	}
	root := w.rangePrefix.root()
	content := appendHead(appendHead(nil, 4, 2), 0, w.rangePrefix.count-1)
	content = appendByteString(content, root[:])
	return w.append(commitType, content, nil, w.signer(keys))
}

func ed25519Signers(keys []ed25519.PrivateKey) ([]SigningKey, error) {
	signers := make([]SigningKey, 0, len(keys))
	for _, key := range keys {
		signer, err := NewEd25519Key(key)
		if err != nil {
			return nil, err
		}
		signers = append(signers, signer)
	}
	return signers, nil
}

// signer returns the function that writes the signatures element of one frame.
func (w *SegmentWriter) signer(keys []SigningKey) func(Digest) ([]byte, error) {
	return func(frame Digest) ([]byte, error) {
		if len(keys) == 0 {
			return nil, fmt.Errorf("a signed frame needs at least one key")
		}
		seen := make(map[Digest]bool, len(keys))
		records := make([][]byte, 0, len(keys))
		for _, key := range keys {
			public := key.PublicKey()
			keyID := KeyIdentifier(key.Algorithm(), public)
			if seen[keyID] {
				return nil, fmt.Errorf("key %x signs the frame twice", keyID)
			}
			seen[keyID] = true
			record := SignatureRecord{Algorithm: key.Algorithm(), KeyID: keyID}
			message := SignedMessage(record, w.opener, frame)
			value, err := key.sign(message[:])
			if err != nil {
				return nil, err
			}
			record.Value = value
			held := []HeldKey{{KeyID: keyID, Algorithm: key.Algorithm(), PublicKey: public}}
			if result := VerifySignature(record, w.opener, frame, held); result.Status != SignatureValid {
				return nil, fmt.Errorf("the signature fails its own verification: %s %s", result.Status, result.Reason)
			}
			records = append(records, encodeSignatureRecord(record))
		}
		slices.SortFunc(records, bytes.Compare)
		out := arrayHead(uint64(len(records)))
		for _, record := range records {
			out = append(out, record...)
		}
		return out, nil
	}
}

// encodeSignatureRecord returns E([algorithm, key_id, value]) (P6 section 1).
func encodeSignatureRecord(record SignatureRecord) []byte {
	out := arrayHead(3)
	out = appendUint(out, record.Algorithm)
	out = appendBytes(out, record.KeyID[:])
	return appendBytes(out, record.Value)
}
