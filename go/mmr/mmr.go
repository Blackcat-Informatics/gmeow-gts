// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package mmr verifies detached Merkle-Mountain-Range proof JSON for GTS index.mmr roots.
package mmr

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math/bits"
	"strings"

	"go.blackcatinformatics.ca/gts/wire"
)

const (
	// ProofSchema is the stable detached proof JSON schema.
	ProofSchema = "gts-mmr-proof-v1"

	hashAlgorithm   = "blake3-256"
	preimageVersion = "gts-mmr-v1"
	leafDomain      = "gts-mmr-leaf-v1"
	parentDomain    = "gts-mmr-parent-v1"
	rootDomain      = "gts-mmr-root-v1"
)

// Peak is one MMR peak carried in a detached proof.
type Peak struct {
	Height int
	Hash   []byte
}

// Step is one sibling step from the leaf to its peak.
type Step struct {
	ParentHeight int
	Side         string
	Hash         []byte
}

// Proof is the stable detached inclusion-proof data model.
type Proof struct {
	Count     int
	LeafIndex int
	FrameID   []byte
	Root      []byte
	PeakIndex int
	Peaks     []Peak
	Path      []Step
}

type jsonPeak struct {
	Height *int    `json:"height"`
	Hash   *string `json:"hash"`
}

type jsonStep struct {
	Side         *string `json:"side"`
	ParentHeight *int    `json:"parent_height"`
	Hash         *string `json:"hash"`
}

type jsonProof struct {
	Schema    *string     `json:"schema"`
	Hash      *string     `json:"hash"`
	Preimage  *string     `json:"preimage"`
	Count     *int        `json:"count"`
	LeafIndex *int        `json:"leaf_index"`
	FrameID   *string     `json:"frame_id"`
	Root      *string     `json:"root"`
	PeakIndex *int        `json:"peak_index"`
	Peaks     *[]jsonPeak `json:"peaks"`
	Path      *[]jsonStep `json:"path"`
}

func leafHash(index int, frameID []byte) []byte {
	return wire.Blake3_256(wire.MustEncode([]interface{}{leafDomain, int64(index), frameID}))
}

func parentHash(parentHeight int, left, right []byte) []byte {
	return wire.Blake3_256(wire.MustEncode([]interface{}{
		parentDomain,
		int64(parentHeight),
		left,
		right,
	}))
}

func rootHash(count int, peaks []Peak) []byte {
	peakValues := make([]interface{}, len(peaks))
	for i, peak := range peaks {
		peakValues[i] = []interface{}{int64(peak.Height), peak.Hash}
	}
	return wire.Blake3_256(wire.MustEncode([]interface{}{rootDomain, int64(count), peakValues}))
}

func expectedPeakHeights(count int) []int {
	var heights []int
	for remaining := count; remaining > 0; {
		height := bits.Len(uint(remaining)) - 1
		heights = append(heights, height)
		remaining -= 1 << height
	}
	return heights
}

func peakIndexForLeaf(count int, heights []int, leafIndex int) (int, error) {
	if leafIndex >= count {
		return 0, fmt.Errorf("leaf_index %d is outside covered count %d", leafIndex, count)
	}
	start := 0
	for index, height := range heights {
		end := start + (1 << height)
		if leafIndex >= start && leafIndex < end {
			return index, nil
		}
		start = end
	}
	return 0, fmt.Errorf("peak ranges do not cover leaf_index %d for count %d", leafIndex, count)
}

// ParseHex32 parses a raw 32-byte hex id, accepting an optional blake3: prefix.
func ParseHex32(input string) ([]byte, error) {
	raw := strings.TrimSpace(input)
	raw = strings.TrimPrefix(raw, "blake3:")
	if len(raw) != 64 {
		return nil, fmt.Errorf("expected a 32-byte hex value")
	}
	out, err := hex.DecodeString(raw)
	if err != nil {
		return nil, fmt.Errorf("hex value contains a non-hex character")
	}
	if len(out) != 32 {
		return nil, fmt.Errorf("expected a 32-byte hex value")
	}
	return out, nil
}

func requiredString(value *string, key string) (string, error) {
	if value == nil {
		return "", fmt.Errorf("%q must be a string", key)
	}
	return *value, nil
}

func requiredInt(value *int, key string) (int, error) {
	if value == nil || *value < 0 {
		return 0, fmt.Errorf("%q must be an unsigned integer", key)
	}
	return *value, nil
}

// ProofFromJSON parses the stable detached proof JSON form.
func ProofFromJSON(data []byte) (Proof, error) {
	var raw jsonProof
	if err := json.Unmarshal(data, &raw); err != nil {
		return Proof{}, err
	}
	schema, err := requiredString(raw.Schema, "schema")
	if err != nil {
		return Proof{}, err
	}
	if schema != ProofSchema {
		return Proof{}, fmt.Errorf("unsupported proof schema %q", schema)
	}
	hashAlg, err := requiredString(raw.Hash, "hash")
	if err != nil {
		return Proof{}, err
	}
	if hashAlg != hashAlgorithm {
		return Proof{}, fmt.Errorf("unsupported hash algorithm %q", hashAlg)
	}
	preimage, err := requiredString(raw.Preimage, "preimage")
	if err != nil {
		return Proof{}, err
	}
	if preimage != preimageVersion {
		return Proof{}, fmt.Errorf("unsupported preimage version %q", preimage)
	}
	count, err := requiredInt(raw.Count, "count")
	if err != nil {
		return Proof{}, err
	}
	leafIndex, err := requiredInt(raw.LeafIndex, "leaf_index")
	if err != nil {
		return Proof{}, err
	}
	peakIndex, err := requiredInt(raw.PeakIndex, "peak_index")
	if err != nil {
		return Proof{}, err
	}
	frameIDHex, err := requiredString(raw.FrameID, "frame_id")
	if err != nil {
		return Proof{}, err
	}
	frameID, err := ParseHex32(frameIDHex)
	if err != nil {
		return Proof{}, err
	}
	rootHex, err := requiredString(raw.Root, "root")
	if err != nil {
		return Proof{}, err
	}
	root, err := ParseHex32(rootHex)
	if err != nil {
		return Proof{}, err
	}
	if raw.Peaks == nil {
		return Proof{}, fmt.Errorf("%q must be a JSON array", "peaks")
	}
	peaks := make([]Peak, len(*raw.Peaks))
	for i, peak := range *raw.Peaks {
		height, err := requiredInt(peak.Height, "height")
		if err != nil {
			return Proof{}, err
		}
		hashHex, err := requiredString(peak.Hash, "hash")
		if err != nil {
			return Proof{}, err
		}
		hash, err := ParseHex32(hashHex)
		if err != nil {
			return Proof{}, err
		}
		peaks[i] = Peak{Height: height, Hash: hash}
	}
	if raw.Path == nil {
		return Proof{}, fmt.Errorf("%q must be a JSON array", "path")
	}
	path := make([]Step, len(*raw.Path))
	for i, step := range *raw.Path {
		parentHeight, err := requiredInt(step.ParentHeight, "parent_height")
		if err != nil {
			return Proof{}, err
		}
		side, err := requiredString(step.Side, "side")
		if err != nil {
			return Proof{}, err
		}
		if side != "left" && side != "right" {
			return Proof{}, fmt.Errorf("unsupported proof side %q", side)
		}
		hashHex, err := requiredString(step.Hash, "hash")
		if err != nil {
			return Proof{}, err
		}
		hash, err := ParseHex32(hashHex)
		if err != nil {
			return Proof{}, err
		}
		path[i] = Step{ParentHeight: parentHeight, Side: side, Hash: hash}
	}
	return Proof{
		Count:     count,
		LeafIndex: leafIndex,
		FrameID:   frameID,
		Root:      root,
		PeakIndex: peakIndex,
		Peaks:     peaks,
		Path:      path,
	}, nil
}

// VerifyProof verifies a detached proof without access to the original GTS file.
func VerifyProof(proof Proof) error {
	if len(proof.FrameID) != 32 {
		return fmt.Errorf("frame_id must be 32 bytes")
	}
	if len(proof.Root) != 32 {
		return fmt.Errorf("root must be 32 bytes")
	}
	if proof.LeafIndex >= proof.Count {
		return fmt.Errorf("leaf_index %d is outside covered count %d", proof.LeafIndex, proof.Count)
	}
	if proof.PeakIndex >= len(proof.Peaks) {
		return fmt.Errorf("peak_index %d is out of range", proof.PeakIndex)
	}
	expectedHeights := expectedPeakHeights(proof.Count)
	actualHeights := make([]int, len(proof.Peaks))
	for i, peak := range proof.Peaks {
		actualHeights[i] = peak.Height
	}
	if !intSlicesEqual(actualHeights, expectedHeights) {
		return fmt.Errorf("peak heights %v do not match count %d", actualHeights, proof.Count)
	}
	computedPeakIndex, err := peakIndexForLeaf(proof.Count, actualHeights, proof.LeafIndex)
	if err != nil {
		return err
	}
	if computedPeakIndex != proof.PeakIndex {
		return fmt.Errorf(
			"leaf_index %d belongs to peak %d, not %d",
			proof.LeafIndex,
			computedPeakIndex,
			proof.PeakIndex,
		)
	}
	for _, peak := range proof.Peaks {
		if len(peak.Hash) != 32 {
			return fmt.Errorf("peak hash must be 32 bytes")
		}
	}

	carried := leafHash(proof.LeafIndex, proof.FrameID)
	height := 0
	for _, step := range proof.Path {
		if len(step.Hash) != 32 {
			return fmt.Errorf("path hash must be 32 bytes")
		}
		if step.ParentHeight != height+1 {
			return fmt.Errorf(
				"path parent height %d does not follow height %d",
				step.ParentHeight,
				height,
			)
		}
		switch step.Side {
		case "left":
			carried = parentHash(step.ParentHeight, step.Hash, carried)
		case "right":
			carried = parentHash(step.ParentHeight, carried, step.Hash)
		default:
			return fmt.Errorf("unsupported proof side %q", step.Side)
		}
		height = step.ParentHeight
	}

	peak := proof.Peaks[proof.PeakIndex]
	if height != peak.Height {
		return fmt.Errorf("path height %d does not reach peak height %d", height, peak.Height)
	}
	if !bytes.Equal(carried, peak.Hash) {
		return fmt.Errorf("proof path does not reconstruct the selected peak")
	}
	if !bytes.Equal(rootHash(proof.Count, proof.Peaks), proof.Root) {
		return fmt.Errorf("proof peaks do not reconstruct the declared root")
	}
	return nil
}

// VerifyProofJSON parses and verifies a detached proof JSON document.
func VerifyProofJSON(data []byte) (Proof, error) {
	proof, err := ProofFromJSON(data)
	if err != nil {
		return Proof{}, err
	}
	if err := VerifyProof(proof); err != nil {
		return Proof{}, err
	}
	return proof, nil
}

func intSlicesEqual(left, right []int) bool {
	if len(left) != len(right) {
		return false
	}
	for i := range left {
		if left[i] != right[i] {
			return false
		}
	}
	return true
}
