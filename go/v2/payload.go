// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"encoding/binary"
	"io"
	"math"

	"github.com/klauspost/compress/zstd"
	"github.com/zeebo/blake3"
)

// ReadCapacity bounds verification over one ReadWire call.
type ReadCapacity struct {
	StoredBytesPerFrame  uint64
	DecodedBytesPerFrame uint64
	DecodedBytesPerRead  uint64
	ItemsPerRead         uint64
}

// DefaultReadCapacity returns the P7 registry 1 capacity for these wire checks.
func DefaultReadCapacity() ReadCapacity {
	return ReadCapacity{
		StoredBytesPerFrame: 1 << 26, DecodedBytesPerFrame: 1 << 26,
		DecodedBytesPerRead: 1 << 36, ItemsPerRead: 1 << 26,
	}
}

// PayloadState identifies whether the decoded bytes could be verified.
type PayloadState string

// Payload check outcomes.
const (
	Verified  PayloadState = "verified"
	Damaged   PayloadState = "damaged"
	Unchecked PayloadState = "unchecked"
)

// Payload contains the outcome of verifying a frame's decoded bytes.
// Count is the measured value for a capacity refusal.
type Payload struct {
	State   PayloadState
	Reason  Reason
	Count   uint64
	Decoded []byte
}

type storage struct {
	tag          uint64
	codec        uint64
	encodedCodec []byte
	check        []byte
}

func decodeStorage(item []byte, newer bool) (storage, Reason) {
	var s storage
	h, reason := typedHead(item, 4)
	if reason != "" {
		return s, reason
	}
	if h.value == 0 {
		return s, WrongLength
	}
	fields := itemValues{remaining: item[h.end:]}
	s.tag, reason = uintValue(fields.next())
	if reason != "" {
		return s, reason
	}
	switch {
	case s.tag == 0 || (!newer && s.tag > 2):
		return s, UnregisteredValue
	case s.tag == 1 && h.value != 1, s.tag == 2 && h.value != 3:
		return s, WrongLength
	case s.tag == 1:
		return s, ""
	case s.tag > 2:
		return s, grammar(item)
	}

	s.encodedCodec = fields.next()
	s.codec, reason = uintValue(s.encodedCodec)
	if reason != "" {
		return s, reason
	}
	if s.codec == 0 || (!newer && s.codec != 1) {
		return s, UnregisteredValue
	}
	s.check, reason = byteValue(fields.next(), 16)
	return s, reason
}

func (f *frameFields) checkStored() {
	if f.storage.check == nil {
		return
	}
	h, _ := readHead(f.encodedPayload, 0)
	if h.major != 2 {
		return
	}
	hash := blake3.NewDeriveKey("gts2 stored")
	_, _ = hash.Write([]byte{0x81})
	_, _ = hash.Write(f.storage.encodedCodec)
	for chunk := range byteChunks(f.encodedPayload) {
		_, _ = hash.Write(chunk)
	}
	sum := hash.Sum(nil)
	match := bytes.Equal(sum[:16], f.storage.check)
	f.StoredMatches = &match
}

type payloadBudget struct {
	capacity  ReadCapacity
	total     uint64
	exhausted bool
}

func (b *payloadBudget) admit(f *frameFields) (Reason, uint64) {
	if !b.exhausted {
		for _, length := range [...]int{len(f.storedPayload), len(f.Content), f.signaturesSize} {
			if uint64(length) > b.capacity.StoredBytesPerFrame {
				return StoredSize, uint64(length)
			}
		}
		if f.DecodedLength > b.capacity.DecodedBytesPerFrame {
			return DecodedSize, f.DecodedLength
		}
	}

	next := b.total + f.DecodedLength
	if next < b.total {
		next = math.MaxUint64
		b.exhausted = true
	}
	if b.exhausted || next > b.capacity.DecodedBytesPerRead {
		b.exhausted = true
		return TotalDecoded, next
	}
	b.total = next
	return "", 0
}

func (b *payloadBudget) check(f *frameFields) Payload {
	f.checkStored()
	if !f.payloadTyped {
		return Payload{State: Damaged}
	}
	if reason, count := b.admit(f); reason != "" {
		return Payload{State: Unchecked, Reason: reason, Count: count}
	}

	decoded := f.storedPayload
	switch {
	case f.storage.tag == 1:
		if uint64(len(decoded)) != f.DecodedLength {
			return Payload{State: Damaged, Reason: DecodedLength}
		}
	case f.storage.tag == 2 && f.storage.codec == 1:
		header := zstdHeader(f.DecodedLength)
		blocks, ok := bytes.CutPrefix(decoded, header)
		if f.DecodedLength == 0 || !ok {
			if f.Malformed == "" {
				f.Malformed = CodecConstraint
			}
			return Payload{State: Damaged, Reason: CodecConstraint}
		}
		decoded = decodeZstd(decoded, blocks, f.DecodedLength)
		if decoded == nil {
			return Payload{State: Damaged, Reason: DecodeFailed}
		}
	default:
		return Payload{State: Unchecked, Reason: UnknownCodec}
	}

	digest := hash("gts2 payload", decoded)
	if !bytes.Equal(digest[:], f.payloadDigest) {
		return Payload{State: Damaged, Reason: PayloadDigest}
	}
	return Payload{State: Verified, Decoded: decoded}
}

func zstdHeader(size uint64) []byte {
	header := []byte{0x28, 0xb5, 0x2f, 0xfd}
	sizeBytes := binary.LittleEndian.AppendUint64(nil, size)
	switch {
	case size < 256:
		return append(header, 0x20, sizeBytes[0])
	case size <= 65791:
		return binary.LittleEndian.AppendUint16(append(header, 0x60), uint16(size-256))
	case size <= 1<<26:
		return append(append(header, 0xa0), sizeBytes[:4]...)
	case size < 1<<32-1:
		return append(append(header, 0x80, 0x80), sizeBytes[:4]...)
	default:
		return append(append(header, 0xc0, 0x80), sizeBytes...)
	}
}

// The zstd decoder accepts concatenated frames. Check the block boundaries to
// ensure this payload contains exactly the single frame declared by the header.
func singleZstdFrame(blocks []byte) bool {
	for len(blocks) >= 3 {
		header := int(blocks[0]) | int(blocks[1])<<8 | int(blocks[2])<<16
		size := header >> 3
		switch (header >> 1) & 3 {
		case 1:
			size = 1 // RLE stores one octet regardless of its expanded size.
		case 3:
			return false
		}
		blocks = blocks[3:]
		if size > len(blocks) {
			return false
		}
		blocks = blocks[size:]
		if header&1 != 0 {
			return len(blocks) == 0
		}
	}
	return false
}

func decodeZstd(stored, blocks []byte, size uint64) []byte {
	if size > uint64(math.MaxInt) || !singleZstdFrame(blocks) {
		return nil
	}
	decoder, err := zstd.NewReader(bytes.NewReader(stored), zstd.WithDecoderConcurrency(1),
		zstd.WithDecoderMaxWindow(1<<26), zstd.WithDecoderMaxMemory(1<<26))
	if err != nil {
		return nil
	}
	defer decoder.Close()
	decoded, err := io.ReadAll(io.LimitReader(decoder, int64(size)))
	if err != nil || uint64(len(decoded)) != size {
		return nil
	}
	var extra [1]byte
	if _, err := io.ReadFull(decoder, extra[:]); err != io.EOF {
		return nil
	}
	return decoded
}
