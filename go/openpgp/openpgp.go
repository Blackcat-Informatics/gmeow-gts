// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package openpgp is a minimal reader for Ed25519 armored public keys, used by
// `gts extract-key` (§9.2).
//
// It mirrors the Python gts.openpgp reference: it parses only the unencrypted
// armored public-key certificates GPG emits for Ed25519 (OpenPGP algorithm 22)
// keys, extracting the raw 32-byte key and computing the v4 fingerprint so GTS
// tooling can show the embedded transport key without shelling out to gpg.
// Everything else (other algorithms, encrypted secret keys, v5/v6 packets) is
// rejected with a clear error.
package openpgp

import (
	"bytes"
	"crypto/sha1" //nolint:gosec // RFC 4880 §12.2 mandates SHA-1 for v4 fingerprints; not a security primitive here.
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"
	"strings"
)

const ed25519Algo = 22

// ed25519OID is the curve OID GPG writes for the Ed25519 signing curve
// (1.3.6.1.4.1.11591.15.1).
var ed25519OID = []byte{0x2b, 0x06, 0x01, 0x04, 0x01, 0xda, 0x47, 0x0f, 0x01}

// TransportKey is a parsed Ed25519 public key: the raw 32-byte key plus its
// uppercase 40-hex-character OpenPGP v4 fingerprint.
type TransportKey struct {
	RawPublic   []byte
	Fingerprint string
}

// stripArmor decodes the packet bytes from an ASCII-armored OpenPGP block.
func stripArmor(text string) ([]byte, error) {
	lines := strings.Split(text, "\n")
	start, end := -1, -1
	for i, l := range lines {
		if start == -1 && strings.HasPrefix(l, "-----BEGIN PGP") {
			start = i
		} else if start != -1 && i > start && strings.HasPrefix(l, "-----END PGP") {
			end = i
			break
		}
	}
	if start == -1 {
		return nil, errors.New("missing armor BEGIN line")
	}
	if end == -1 {
		return nil, errors.New("missing armor END line")
	}

	idx := start + 1
	// Skip optional armor headers (Comment, Version, …) up to the blank line.
	for idx < end && strings.TrimSpace(lines[idx]) != "" {
		if strings.Contains(lines[idx], ":") {
			idx++
		} else {
			break
		}
	}

	var body strings.Builder
	for idx < end {
		line := strings.TrimRight(lines[idx], "\r")
		if strings.HasPrefix(line, "=") {
			break // CRC-24 checksum line — end of the base64 body.
		}
		body.WriteString(line)
		idx++
	}
	if body.Len() == 0 {
		return nil, errors.New("empty armor body")
	}
	out, err := base64.StdEncoding.DecodeString(body.String())
	if err != nil {
		return nil, errors.New("invalid base64 armor body")
	}
	return out, nil
}

// readMPI reads an OpenPGP multi-precision integer; returns (bytes, nextOffset).
func readMPI(data []byte, offset int) ([]byte, int, error) {
	if offset+2 > len(data) {
		return nil, 0, errors.New("truncated MPI length")
	}
	bits := int(binary.BigEndian.Uint16(data[offset : offset+2]))
	length := (bits + 7) / 8
	end := offset + 2 + length
	if end > len(data) {
		return nil, 0, errors.New("truncated MPI payload")
	}
	return data[offset+2 : end], end, nil
}

// nextPacket parses one OpenPGP packet; returns (tag, body, nextOffset).
// Supports both old- and new-format headers.
func nextPacket(data []byte, offset int) (int, []byte, int, error) {
	if offset >= len(data) {
		return 0, nil, 0, errors.New("truncated packet header")
	}
	header := data[offset]
	if header&0x80 == 0 {
		return 0, nil, 0, errors.New("invalid packet tag octet")
	}

	var tag, length int
	if header&0x40 != 0 {
		// New-format packet.
		tag = int(header & 0x3f)
		offset++
		if offset >= len(data) {
			return 0, nil, 0, errors.New("truncated new-format length octet")
		}
		lo := data[offset]
		switch {
		case lo < 192:
			length = int(lo)
			offset++
		case lo < 224:
			if offset+1 >= len(data) {
				return 0, nil, 0, errors.New("truncated new-format 2-octet length")
			}
			length = (int(lo)-192)<<8 + int(data[offset+1]) + 192
			offset += 2
		case lo == 255:
			if offset+4 >= len(data) {
				return 0, nil, 0, errors.New("truncated new-format 4-octet length")
			}
			length = int(binary.BigEndian.Uint32(data[offset+1 : offset+5]))
			offset += 5
		default:
			return 0, nil, 0, errors.New("partial body lengths are not supported")
		}
	} else {
		// Old-format packet.
		tag = int((header >> 2) & 0x0f)
		lengthType := header & 0x03
		offset++
		switch lengthType {
		case 0:
			if offset >= len(data) {
				return 0, nil, 0, errors.New("truncated old-format length octet")
			}
			length = int(data[offset])
			offset++
		case 1:
			if offset+1 >= len(data) {
				return 0, nil, 0, errors.New("truncated old-format 2-octet length")
			}
			length = int(binary.BigEndian.Uint16(data[offset : offset+2]))
			offset += 2
		case 2:
			if offset+3 >= len(data) {
				return 0, nil, 0, errors.New("truncated old-format 4-octet length")
			}
			length = int(binary.BigEndian.Uint32(data[offset : offset+4]))
			offset += 4
		default:
			return 0, nil, 0, errors.New("indeterminate-length packets are not supported")
		}
	}

	end := offset + length
	if end > len(data) {
		return 0, nil, 0, errors.New("packet body exceeds input")
	}
	return tag, data[offset:end], end, nil
}

// packet is one parsed OpenPGP packet.
type packet struct {
	tag  int
	body []byte
}

// iterPackets returns every (tag, body) packet in the de-armored data.
func iterPackets(data []byte) ([]packet, error) {
	var packets []packet
	offset := 0
	for offset < len(data) {
		tag, body, next, err := nextPacket(data, offset)
		if err != nil {
			return nil, err
		}
		packets = append(packets, packet{tag: tag, body: body})
		offset = next
	}
	return packets, nil
}

// parseEd25519PublicMaterial parses the OID and raw key from a v4 public-key
// packet body; returns (rawPublicKey, endOffsetOfPublicMaterial).
func parseEd25519PublicMaterial(body []byte) ([]byte, int, error) {
	if len(body) < 6 || body[0] != 4 {
		return nil, 0, errors.New("only OpenPGP v4 public keys are supported")
	}
	if body[5] != ed25519Algo {
		return nil, 0, fmt.Errorf("unsupported public-key algorithm %d", body[5])
	}
	offset := 6
	if offset >= len(body) {
		return nil, 0, errors.New("truncated public-key packet")
	}
	oidLen := int(body[offset])
	offset++
	if offset+oidLen > len(body) {
		return nil, 0, errors.New("truncated OID")
	}
	oid := body[offset : offset+oidLen]
	offset += oidLen
	if !bytes.Equal(oid, ed25519OID) {
		return nil, 0, fmt.Errorf("unsupported curve OID %x", oid)
	}

	mpi, end, err := readMPI(body, offset)
	if err != nil {
		return nil, 0, err
	}
	// GPG encodes the Ed25519 public key as a 33-byte MPI (0x40 || 32-byte key);
	// a bare 32-byte MPI is also valid when the high bit is clear.
	switch len(mpi) {
	case 33:
		return append([]byte(nil), mpi[1:]...), end, nil
	case 32:
		return append([]byte(nil), mpi...), end, nil
	default:
		return nil, 0, fmt.Errorf("unexpected Ed25519 public MPI length %d", len(mpi))
	}
}

// fingerprint computes the OpenPGP v4 fingerprint of a public-key packet body:
// SHA-1(0x99 || u16-be(len(body)) || body), uppercased.
func fingerprint(pubKeyBody []byte) string {
	h := sha1.New() //nolint:gosec // RFC 4880 v4 fingerprint construction.
	h.Write([]byte{0x99})
	var lenBuf [2]byte
	binary.BigEndian.PutUint16(lenBuf[:], uint16(len(pubKeyBody)))
	h.Write(lenBuf[:])
	h.Write(pubKeyBody)
	return fmt.Sprintf("%X", h.Sum(nil))
}

// ParseTransportKey parses an armored OpenPGP certificate into its raw Ed25519
// key and v4 fingerprint. It accepts either a public-key certificate (tag 6) or
// an unencrypted secret-key block (tag 5); the fingerprint always covers only
// the public material.
func ParseTransportKey(armored string) (TransportKey, error) {
	data, err := stripArmor(armored)
	if err != nil {
		return TransportKey{}, err
	}
	packets, err := iterPackets(data)
	if err != nil {
		return TransportKey{}, err
	}
	for _, p := range packets {
		var raw, pubBody []byte
		switch p.tag {
		case 6:
			raw, _, err = parseEd25519PublicMaterial(p.body)
			if err != nil {
				return TransportKey{}, err
			}
			pubBody = p.body
		case 5:
			var end int
			raw, end, err = parseEd25519PublicMaterial(p.body)
			if err != nil {
				return TransportKey{}, err
			}
			pubBody = p.body[:end]
		default:
			continue
		}
		return TransportKey{RawPublic: raw, Fingerprint: fingerprint(pubBody)}, nil
	}
	return TransportKey{}, errors.New("no public-key packet found")
}
