// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package codec

import (
	"strings"
	"testing"

	"github.com/klauspost/compress/zstd"
)

func TestZstdDecodeRejectsOutputsOverSafetyBound(t *testing.T) {
	payload := make([]byte, maxZstdDecodedSize+1)
	encoder, err := zstd.NewWriter(nil)
	if err != nil {
		t.Fatalf("zstd writer: %v", err)
	}
	encoded := encoder.EncodeAll(payload, nil)
	if err := encoder.Close(); err != nil {
		t.Fatalf("close zstd writer: %v", err)
	}

	_, err = DecodeChain([]*Codec{{Name: "zstd", Cls: "compress"}}, encoded)
	if err == nil {
		t.Fatal("expected over-limit zstd decode to fail")
	}
	if !strings.Contains(err.Error(), "decompressed size exceeds safety bound") {
		t.Fatalf("unexpected error: %v", err)
	}
}
