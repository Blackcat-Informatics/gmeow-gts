// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package reader

import (
	"bytes"
	"testing"

	"github.com/klauspost/compress/zstd"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
	"go.blackcatinformatics.ca/gts/writer"
)

// zstdBlobFile builds a file of a terms frame, a quads frame and a blob frame whose
// payload is raw and names the zstd codec (catalog id 2), then a second terms frame.
func zstdBlobFile(raw []byte, declared []byte) []byte {
	w := writer.New("generic")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://example.org/a"},
		{Kind: model.Iri, Value: "https://example.org/p"},
		{Kind: model.Iri, Value: "https://example.org/b"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	frame := map[interface{}]interface{}{
		"t":    "blob",
		"x":    []interface{}{2},
		"d":    raw,
		"pub":  map[interface{}]interface{}{"digest": writer.DigestString(declared), "mt": "text/plain"},
		"prev": w.Head(),
	}
	frame["id"] = wire.ContentID(frame)
	return append(w.ToBytes(), wire.MustEncode(frame)...)
}

// windowFrame is a zstd frame written by hand: a window descriptor for 2^windowLog,
// no content size or checksum, and one raw block.
func windowFrame(windowLog int, content []byte) []byte {
	block := 1 | len(content)<<3
	out := []byte{0x28, 0xb5, 0x2f, 0xfd, 0x00, byte((windowLog - 10) << 3)}
	out = append(out, byte(block), byte(block>>8), byte(block>>16))
	return append(out, content...)
}

func TestZstdRsyncableConcatenatedFramesDecodeToTheEnd(t *testing.T) {
	enc, err := zstd.NewWriter(nil)
	if err != nil {
		t.Fatal(err)
	}
	units := [][]byte{[]byte("alpha "), []byte("beta "), []byte("gamma")}
	var payload []byte
	for _, u := range units {
		payload = append(payload, enc.EncodeAll(u, nil)...)
	}
	content := bytes.Join(units, nil)

	g := Read(zstdBlobFile(payload, content), true, nil)
	if len(g.Diagnostics) != 0 || len(g.Opaque) != 0 {
		t.Fatalf("concatenated frames should decode, got diagnostics %v opaque %v", g.Diagnostics, g.Opaque)
	}
	if len(g.Blobs) != 1 || !bytes.Equal(g.Blobs[0].Data, content) {
		t.Fatalf("blob = %v, want %q", g.Blobs, content)
	}
	if g.Blobs[0].Digest != writer.DigestString(content) {
		t.Fatalf("blob digest %s does not match the content", g.Blobs[0].Digest)
	}
}

func TestZstdPayloadTheLibraryRefusesIsOneOpaqueNode(t *testing.T) {
	enc, err := zstd.NewWriter(nil)
	if err != nil {
		t.Fatal(err)
	}
	content := bytes.Repeat([]byte("the bytes it should have held "), 30)
	whole := enc.EncodeAll(content, nil)
	cases := map[string][]byte{
		"not zstd":          []byte("not zstd frame data"),
		"truncated":         whole[:len(whole)/2],
		"window above 2^29": windowFrame(30, []byte("declared window")),
	}
	for name, payload := range cases {
		g := Read(zstdBlobFile(payload, content), true, nil)
		if len(g.Opaque) != 1 || g.Opaque[0].Reason != "damaged" || g.Opaque[0].FrameType != "blob" {
			t.Errorf("%s: opaque = %+v, want one damaged blob node", name, g.Opaque)
		}
		if len(g.Diagnostics) != 1 || g.Diagnostics[0].Code != "DamagedFrame" {
			t.Errorf("%s: diagnostics = %+v, want one DamagedFrame", name, g.Diagnostics)
		}
		if len(g.Blobs) != 0 {
			t.Errorf("%s: blobs = %v, want none", name, g.Blobs)
		}
		if len(g.Quads) != 1 {
			t.Errorf("%s: quads = %d, folding must continue", name, len(g.Quads))
		}
	}
}
