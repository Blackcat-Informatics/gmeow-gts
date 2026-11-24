// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"math"
	"testing"
)

func TestZstdLargeDeclarationDoesNotAllocateItsOutputUpFront(t *testing.T) {
	size := uint64(math.MaxInt)
	blocks := []byte{0x09, 0x00, 0x00, 'a'} // One final raw block of one byte.
	stored := append(zstdHeader(size), blocks...)
	if decoded := decodeZstd(stored, blocks, size); decoded != nil {
		t.Fatalf("decoded %d bytes with a declared length of %d", len(decoded), size)
	}
}
