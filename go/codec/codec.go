// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package codec provides the GTS transform catalog decoder (§8).
//
// It intentionally implements the baseline decode path only: identity, gzip,
// zstd, and zstd-rsyncable are decoded locally, while encrypt-class entries
// degrade to missing-key unless a higher layer supplies key-aware handling.
package codec

import (
	"bytes"
	"fmt"
	"io"

	"github.com/klauspost/compress/gzip"
	"github.com/klauspost/compress/zstd"
)

const maxZstdDecodedSize = 16 * 1024 * 1024

// Codec is a catalog entry (§5, §8.5).
type Codec struct {
	// Name is the canonical codec name looked up from the segment catalog.
	Name string
	// Cls is "encode", "compress", or "encrypt".
	Cls string
}

// Error describes why a transform chain could not be reversed.
type Error struct {
	// Reason is "unknown-codec" or "missing-key" for opaque degradation.
	Reason string
	// Detail is a human-readable diagnostic.
	Detail string
	// Failed is true when the codec is known but the bytes are corrupt.
	Failed bool
}

func (e *Error) Error() string {
	return e.Detail
}

// decodeOne reverses a single codec entry on data (identity, gzip, or zstd).
// Encrypt-class codecs cannot be reversed without a key.
func decodeOne(codec *Codec, data []byte) ([]byte, error) {
	if codec == nil {
		return nil, &Error{Failed: true, Detail: "codec chain contains nil entry"}
	}
	if codec.Cls == "encrypt" {
		return nil, &Error{
			Reason: "missing-key",
			Detail: fmt.Sprintf("no key for encrypt codec '%s'", codec.Name),
		}
	}
	switch codec.Name {
	case "identity":
		return data, nil
	case "gzip":
		r, err := gzip.NewReader(bytes.NewReader(data))
		if err != nil {
			return nil, &Error{Failed: true, Detail: fmt.Sprintf("gzip decode failed: %v", err)}
		}
		out, err := io.ReadAll(r)
		_ = r.Close()
		if err != nil {
			return nil, &Error{Failed: true, Detail: fmt.Sprintf("gzip decode failed: %v", err)}
		}
		return out, nil
	case "zstd", "zstd-rsyncable":
		r, err := zstd.NewReader(bytes.NewReader(data))
		if err != nil {
			return nil, &Error{Failed: true, Detail: fmt.Sprintf("zstd decoder init failed: %v", err)}
		}
		defer r.Close()
		out, err := io.ReadAll(io.LimitReader(r, int64(maxZstdDecodedSize+1)))
		if err != nil {
			return nil, &Error{Failed: true, Detail: fmt.Sprintf("zstd decode failed: %v", err)}
		}
		if len(out) > maxZstdDecodedSize {
			return nil, &Error{Failed: true, Detail: "zstd decode failed: decompressed size exceeds safety bound"}
		}
		return out, nil
	default:
		return nil, &Error{
			Reason: "unknown-codec",
			Detail: fmt.Sprintf("unknown codec '%s'", codec.Name),
		}
	}
}

// DecodeChain reverses a resolved codec chain, last to first (§6.1, §8.2).
//
// The baseline carries no keys, so every encrypt-class codec degrades to
// missing-key (matching the Python reader with keys=None).
func DecodeChain(chain []*Codec, data []byte) ([]byte, error) {
	current := data
	for i := len(chain) - 1; i >= 0; i-- {
		if chain[i] == nil {
			return nil, &Error{Failed: true, Detail: fmt.Sprintf("codec chain contains nil entry at index %d", i)}
		}
		var err error
		current, err = decodeOne(chain[i], current)
		if err != nil {
			return nil, err
		}
	}
	return current, nil
}
