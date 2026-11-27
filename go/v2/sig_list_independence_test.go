// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"bytes"
	"crypto/ed25519"
	"testing"
)

// TestSigListIndependenceIsWrittenByteForByte rebuilds P6/sig-list-independence.bin. The frame is
// signed by the writer with the keys of seed 20..3f and seed 00..1f. The first signature value
// is then byte-edited to 64 zero bytes, which is not a signature. The second is the writer's
// valid signature, so one result is invalid beside one valid result: each is judged as if alone.
func TestSigListIndependenceIsWrittenByteForByte(t *testing.T) {
	stored := vectorBytes(t, "sig-list-independence.bin")
	frames := storedFrames(t, stored)
	if len(frames) != 1 {
		t.Fatalf("got %d frames, want 1", len(frames))
	}
	keyA, keyB := recoveredKeys()
	writer, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	frame := frames[0]
	if _, err := writer.AppendSigned(frame.frameType, frame.content, frame.payload, keyA, keyB); err != nil {
		t.Fatal(err)
	}
	written := writer.Bytes()

	idA := KeyIdentifier(Ed25519Algorithm, keyA.Public().(ed25519.PublicKey))
	signedA := signatureValueAfter(t, written, signatureRecordPrefix(idA))
	edited := replaceOnce(t, written, append(signatureRecordPrefix(idA), signedA...), append(signatureRecordPrefix(idA), make([]byte, 64)...))
	if !bytes.Equal(edited, stored) {
		t.Fatalf("the edited segment differs from the stored file:\n got % x\nwant % x", edited, stored)
	}
}
