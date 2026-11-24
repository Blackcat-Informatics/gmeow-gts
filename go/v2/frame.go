// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"

	"github.com/zeebo/blake3"
)

const (
	commitType = 14
	parityType = 16
)

// Digest is a GTS v2 BLAKE3 identifier.
type Digest [32]byte

func hash(context string, parts ...[]byte) (digest Digest) {
	h := blake3.NewDeriveKey(context)
	for _, part := range parts {
		_, _ = h.Write(part)
	}
	h.Sum(digest[:0])
	return digest
}

// Link identifies the outcome of comparing a computed and stored frame ID.
type Link string

// Link check outcomes.
const (
	Linked Link = "linked"
	Healed Link = "healed"
	Broken Link = "broken"
)

// Frame holds envelope observations and the independent link and payload checks.
// A nil StoredMatches means the stored check was unavailable or did not apply.
type Frame struct {
	Type          uint64 // The typed uint value, or zero if the field is ill typed.
	Critical      bool
	Transparent   bool
	Content       []byte
	DecodedLength uint64
	StoredID      *Digest
	ComputedID    *Digest
	EffectiveID   *Digest
	Link          Link
	StoredMatches *bool
	Payload       Payload
	// Malformed is the first envelope field violation or codec constraint.
	Malformed Reason
}

type frameFields struct {
	Frame
	identity       []byte
	storage        storage
	payloadDigest  []byte
	storedPayload  []byte
	encodedPayload []byte
	signaturesSize int
	payloadTyped   bool
}

func decodeFrame(item []byte, newer bool) frameFields {
	var f frameFields
	values := itemValues{remaining: item[2:]}
	var fields [10][]byte
	for i := range fields {
		fields[i] = values.next()
	}

	var typeReason, criticalReason, transparentReason, lengthReason Reason
	f.Type, typeReason = uintValue(fields[0])
	if typeReason != "" {
		f.Type = 0
	}
	f.Critical, criticalReason = boolValue(fields[1])
	f.Transparent, transparentReason = boolValue(fields[2])
	f.Content = fields[3]
	contentReason := checkContent(f.Type, f.Content)
	f.DecodedLength, lengthReason = uintValue(fields[4])
	f.payloadDigest, _ = byteValue(fields[5], 32)
	digestReason := checkDigest(fields[5])

	if firstReason(typeReason, criticalReason, transparentReason, contentReason, lengthReason, digestReason) == "" {
		f.identity = item[2 : len(item)-len(fields[6])-len(fields[7])-len(fields[8])-len(fields[9])]
	}
	if typeReason == "" && (f.Type == 0 || (!newer && f.Type > parityType)) {
		typeReason = UnregisteredValue
	}
	if f.Type > 0 && f.Type <= parityType {
		if criticalReason == "" && f.Critical != (f.Type < commitType) {
			criticalReason = ForbiddenCombination
		}
		if transparentReason == "" && f.Transparent != (f.Type == parityType) {
			transparentReason = ForbiddenCombination
		}
	}
	var storageReason, payloadReason Reason
	f.storage, storageReason = decodeStorage(fields[6], newer)
	f.encodedPayload = fields[7]
	f.storedPayload, payloadReason = byteValue(fields[7], -1)
	f.payloadTyped = firstReason(lengthReason, digestReason, storageReason, payloadReason) == ""
	if storageReason == "" && ((f.storage.tag == 2 && f.DecodedLength == 0 && lengthReason == "") ||
		(f.Type == parityType && f.storage.tag != 1)) {
		storageReason = ForbiddenCombination
	}

	f.StoredID = storedID(fields[8])
	identifierReason := checkDigest(fields[8])
	f.signaturesSize = len(fields[9])
	signatureReason := checkSignatures(fields[9], newer)
	if signatureReason == "" && f.Type == parityType && fields[9][0] != 0x80 {
		signatureReason = ForbiddenCombination
	}
	f.Malformed = firstReason(typeReason, criticalReason, transparentReason, contentReason, lengthReason,
		digestReason, storageReason, payloadReason, identifierReason, signatureReason)
	if f.Malformed == "" {
		f.Malformed = checkPayloadLength(f.Type, f.DecodedLength)
	}
	return f
}

func checkPayloadLength(frameType, length uint64) Reason {
	if (frameType == 8 && length == 0) ||
		((frameType == 12 || frameType == commitType) && length != 0) {
		return OutOfRange
	}
	return ""
}

func firstReason(reasons ...Reason) Reason {
	for _, reason := range reasons {
		if reason != "" {
			return reason
		}
	}
	return ""
}

func storedID(item []byte) *Digest {
	var digest Digest
	length := 0
	for chunk := range byteChunks(item) {
		if len(chunk) > len(digest)-length {
			return nil
		}
		length += copy(digest[length:], chunk)
	}
	if length != len(digest) {
		return nil
	}
	return &digest
}

func byteChunks(item []byte) func(func([]byte) bool) {
	return func(yield func([]byte) bool) {
		h, _ := readHead(item, 0)
		if h.major != 2 {
			return
		}
		if !h.indefinite {
			yield(item[h.end:])
			return
		}
		for at := h.end; item[at] != 0xff; {
			chunk, _ := readHead(item, at)
			at = chunk.end + int(chunk.value)
			if !yield(item[chunk.end:at]) {
				return
			}
		}
	}
}

func (f *frameFields) computeID(pred Digest) *Digest {
	if f.identity == nil {
		return nil
	}
	id := frameID(pred, f.identity)
	return &id
}

func frameID(predecessor Digest, identity []byte) Digest {
	return hash("gts2 frame", []byte{0x87, 0x58, 0x20}, predecessor[:], identity)
}

func (f *frameFields) checkLink(pred Digest, witness *frameFields) {
	f.ComputedID = f.computeID(pred)
	f.Link = Broken
	f.EffectiveID = f.StoredID
	if f.StoredID == nil {
		f.EffectiveID = f.ComputedID
	} else if f.ComputedID != nil {
		if *f.ComputedID == *f.StoredID {
			f.Link = Linked
		} else if !f.Transparent && witness != nil && witness.StoredID != nil {
			witnessID := witness.computeID(*f.ComputedID)
			if witnessID != nil && *witnessID == *witness.StoredID {
				f.Link = Healed
				f.EffectiveID = f.ComputedID
			}
		}
	}
	if f.Transparent {
		f.EffectiveID = nil
	}
}

func checkContent(frameType uint64, item []byte) Reason {
	switch frameType {
	case 1:
		return checkRecord(item, checkUint, checkRuns, checkUint, checkUint)
	case 2, 3, 4, 13:
		return checkRecord(item, checkUint, checkUint)
	case 5, 6, 7:
		return checkRecord(item, checkUint)
	case 8:
		return checkRecord(item, checkDigest)
	case 9:
		return checkRecord(item, checkDigest, checkTextOption)
	case 10:
		return checkRecord(item, checkDigest, checkUint, checkTextOption, checkUint)
	case 11:
		return checkRecord(item, checkDigest, checkUint, checkUint)
	case 12:
		return checkRecord(item, checkQualifiedName, checkUint, checkTextOption)
	case commitType, 15:
		return checkRecord(item, checkUint, checkDigest)
	case parityType:
		return checkRecord(item, checkUint, checkUint, checkUint, checkDigest, checkDigest)
	default:
		return grammar(item)
	}
}

func checkRuns(item []byte) Reason {
	h, reason := typedHead(item, 4)
	if reason != "" {
		return reason
	}
	values := itemValues{remaining: item[h.end:]}
	for len(values.remaining) != 0 {
		if reason := checkRecord(values.next(), checkUint, checkUint); reason != "" {
			return reason
		}
	}
	return ""
}

func checkQualifiedName(item []byte) Reason {
	fields, reason := array(item, 2)
	if reason != "" {
		return reason
	}
	code, reason := uintValue(fields.next())
	if reason != "" {
		return reason
	}
	length := -1
	if code == 1 {
		length = 32
	}
	_, reason = byteValue(fields.next(), length)
	return reason
}

func checkSignatures(item []byte, newer bool) Reason {
	h, reason := typedHead(item, 4)
	if reason != "" {
		return reason
	}
	signatures := itemValues{remaining: item[h.end:]}
	var previous []byte
	for len(signatures.remaining) != 0 {
		encoded := signatures.next()
		fields, reason := array(encoded, 3)
		if reason != "" {
			return reason
		}
		algorithm, reason := uintValue(fields.next())
		if reason != "" {
			return reason
		}
		if algorithm == 0 || (!newer && algorithm > 2) {
			return UnregisteredValue
		}
		if reason := checkDigest(fields.next()); reason != "" {
			return reason
		}
		if _, reason := byteValue(fields.next(), -1); reason != "" {
			return reason
		}
		if previous != nil && bytes.Compare(previous, encoded) >= 0 {
			return UnorderedOrRepeated
		}
		previous = encoded
	}
	return ""
}
