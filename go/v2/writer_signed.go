// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"fmt"
	"slices"
)

// AppendSigned writes a frame whose signatures element holds one `ed25519` signature by each
// key. Every signature is made over the effective identifier of the frame, with the signed
// subject of P6 section 2, and is verified by VerifySignature before it is written (P6-29).
// A key may appear once (P6-5). The records are written in ascending bytewise order of their
// encodings (P6-52).
func (w *SegmentWriter) AppendSigned(frameType uint64, content, payload []byte, keys ...ed25519.PrivateKey) (Digest, error) {
	if frameType == parityType {
		return Digest{}, fmt.Errorf("a transparent frame carries no signature (P6-3)")
	}
	if err := checkFrame(frameType, content, payload); err != nil {
		return Digest{}, err
	}
	return w.append(frameType, content, payload, w.signer(keys))
}

// CommitSigned writes a commit over the current prefix with the signatures of AppendSigned.
func (w *SegmentWriter) CommitSigned(keys ...ed25519.PrivateKey) (Digest, error) {
	if w.rangePrefix.count == 0 {
		return Digest{}, fmt.Errorf("initialize the segment writer with NewSegmentWriter")
	}
	root := w.rangePrefix.root()
	content := appendHead(appendHead(nil, 4, 2), 0, w.rangePrefix.count-1)
	content = appendByteString(content, root[:])
	return w.append(commitType, content, nil, w.signer(keys))
}

// signer returns the function that writes the signatures element of one frame.
func (w *SegmentWriter) signer(keys []ed25519.PrivateKey) func(Digest) ([]byte, error) {
	return func(frame Digest) ([]byte, error) {
		if len(keys) == 0 {
			return nil, fmt.Errorf("a signed frame needs at least one key")
		}
		seen := make(map[Digest]bool, len(keys))
		records := make([][]byte, 0, len(keys))
		for _, key := range keys {
			if len(key) != ed25519.PrivateKeySize {
				return nil, fmt.Errorf("an ed25519 private key is %d bytes, not %d", len(key), ed25519.PrivateKeySize)
			}
			public := key.Public().(ed25519.PublicKey)
			keyID := KeyIdentifier(Ed25519Algorithm, public)
			if seen[keyID] {
				return nil, fmt.Errorf("key %x signs the frame twice", keyID)
			}
			seen[keyID] = true
			record := SignatureRecord{Algorithm: Ed25519Algorithm, KeyID: keyID}
			message := SignedMessage(record, w.opener, frame)
			record.Value = ed25519.Sign(key, message[:])
			held := []HeldKey{{KeyID: keyID, Algorithm: Ed25519Algorithm, PublicKey: public}}
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
