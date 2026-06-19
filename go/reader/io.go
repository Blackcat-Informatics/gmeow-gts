// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"bytes"
	"context"
	"errors"
	"io"

	"go.blackcatinformatics.ca/gts/model"
)

// ErrReadLimitExceeded is returned when Options.MaxBytes is positive and
// ReadFrom or ReadToSink observes more than that many input bytes.
var ErrReadLimitExceeded = errors.New("gts reader: input exceeds MaxBytes")

// Options configures total and streaming reads.
type Options struct {
	// AllowSegments permits concatenated multi-segment files. When false,
	// the first later header emits SegmentBoundary and the remainder is not folded.
	AllowSegments bool
	// ExpectedHead surfaces TruncatedLog when the observed final segment head
	// differs. Nil means no caller-supplied head expectation.
	ExpectedHead []byte
	// MaxBytes caps bytes consumed from r before folding or streaming. Zero means
	// no explicit limit; negative values are rejected before any read.
	MaxBytes int64
}

// ReadFrom reads a GTS file from r, honoring ctx cancellation and MaxBytes,
// then folds it with the total reader.
//
// This is an idiomatic service boundary for Go callers that receive bytes from
// HTTP bodies, object stores, or pipes and want a *model.Graph result. Use
// ReadToSink when the caller wants an incremental streaming fold instead.
func ReadFrom(ctx context.Context, r io.Reader, opts Options) (*model.Graph, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	if r == nil {
		return nil, errors.New("gts reader: nil reader")
	}
	data, err := readAllContext(ctx, r, opts.MaxBytes)
	if err != nil {
		return nil, err
	}
	return Read(data, opts.AllowSegments, opts.ExpectedHead), nil
}

func readAllContext(ctx context.Context, r io.Reader, maxBytes int64) ([]byte, error) {
	if maxBytes < 0 {
		return nil, errors.New("gts reader: MaxBytes must be >= 0")
	}
	if maxBytes > 0 {
		r = io.LimitReader(r, maxBytes+1)
	}
	var out bytes.Buffer
	buf := make([]byte, 32*1024)
	for {
		if err := ctx.Err(); err != nil {
			return nil, err
		}
		n, err := r.Read(buf)
		if n > 0 {
			if maxBytes > 0 && int64(out.Len())+int64(n) > maxBytes {
				return nil, ErrReadLimitExceeded
			}
			out.Write(buf[:n])
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				return out.Bytes(), nil
			}
			return nil, err
		}
	}
}
