// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"bytes"
	"context"
	"os"
	"path/filepath"
	"testing"
)

func benchmarkVector(b *testing.B) []byte {
	b.Helper()
	path := filepath.Join("..", "..", "vectors", "25b-streamable-compacted.gts")
	data, err := os.ReadFile(path)
	if err != nil {
		b.Fatal(err)
	}
	return data
}

func BenchmarkReadFullCorpusVector(b *testing.B) {
	data := benchmarkVector(b)
	b.ReportAllocs()
	for b.Loop() {
		_ = Read(data, true, nil)
	}
}

func BenchmarkReadToSinkCorpusVector(b *testing.B) {
	data := benchmarkVector(b)
	sink := StreamingSinkFunc(func(StreamingEvent) error { return nil })
	b.ReportAllocs()
	for b.Loop() {
		if _, err := ReadToSink(context.Background(), bytes.NewReader(data), Options{
			AllowSegments: true,
		}, sink); err != nil {
			b.Fatal(err)
		}
	}
}
