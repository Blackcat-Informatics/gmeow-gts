// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package codec

import (
	"bytes"
	"testing"

	"github.com/klauspost/compress/zstd"
)

func TestZstdDecodeAcceptsOutputsOverFormerSafetyBound(t *testing.T) {
	payload := make([]byte, 16*1024*1024+1)
	encoder, err := zstd.NewWriter(nil)
	if err != nil {
		t.Fatalf("zstd writer: %v", err)
	}
	encoded := encoder.EncodeAll(payload, nil)
	if err := encoder.Close(); err != nil {
		t.Fatalf("close zstd writer: %v", err)
	}

	out, err := DecodeChain([]*Codec{{Name: "zstd", Cls: "compress"}}, encoded)
	if err != nil {
		t.Fatalf("expected over-former-limit zstd decode to succeed: %v", err)
	}
	if !bytes.Equal(out, payload) {
		t.Fatalf("decoded payload mismatch: got %d bytes, want %d", len(out), len(payload))
	}
}
