// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestSigListOrderIsWrittenByteForByte rebuilds P6/sig-list-order.bin: one blob frame whose
// signature list holds three records, by the Ed25519 keys of seed 00..1f and seed 20..3f and the
// composite key of seed 40..7f. The writer sorts the records into ascending bytewise order
// (P6-52), so the order in which the keys are passed must not change the bytes. The test writes
// the frame twice, with the keys in two arrival orders, and both writes must equal the stored file.
func TestSigListOrderIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "sig-list-order.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 1 {
		t.Fatalf("got %d frames, want 1", len(frames))
	}
	keyA, keyB := recoveredKeys()
	ed, err := ed25519Signers([]ed25519.PrivateKey{keyA, keyB})
	if err != nil {
		t.Fatal(err)
	}
	seed := make([]byte, 64)
	for i := range seed {
		seed[i] = byte(0x40 + i)
	}
	composite, err := NewCompositeKey(seed)
	if err != nil {
		t.Fatal(err)
	}
	orders := map[string][]SigningKey{
		"ed25519 then composite": {ed[1], ed[0], composite},
		"composite then ed25519": {composite, ed[0], ed[1]},
	}
	for name, keys := range orders {
		t.Run(name, func(t *testing.T) {
			writer, err := NewSegmentWriter(SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			if _, err := writer.AppendSignedWith(frames[0].frameType, frames[0].content, frames[0].payload, keys...); err != nil {
				t.Fatal(err)
			}
			if got := writer.Bytes(); !bytes.Equal(got, stored) {
				t.Fatalf("the written segment differs from the stored file:\n got % x\nwant % x", got, stored)
			}
		})
	}
}
