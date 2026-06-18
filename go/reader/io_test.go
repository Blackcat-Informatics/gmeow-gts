// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"bytes"
	"context"
	"errors"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/writer"
)

func minimalGTS() []byte {
	w := writer.New("generic")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://example.org/Cat"},
		{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	return w.ToBytes()
}

func TestReadFromReadsBoundedInput(t *testing.T) {
	data := minimalGTS()
	g, err := ReadFrom(context.Background(), bytes.NewReader(data), Options{
		AllowSegments: true,
		MaxBytes:      int64(len(data)),
	})
	if err != nil {
		t.Fatalf("ReadFrom returned error: %v", err)
	}
	if len(g.Diagnostics) > 0 {
		t.Fatalf("unexpected diagnostics: %v", g.Diagnostics)
	}
	if len(g.Quads) != 1 {
		t.Fatalf("expected 1 quad, got %d", len(g.Quads))
	}
}

func TestReadFromHonorsMaxBytes(t *testing.T) {
	data := minimalGTS()
	_, err := ReadFrom(context.Background(), bytes.NewReader(data), Options{
		MaxBytes: int64(len(data) - 1),
	})
	if !errors.Is(err, ErrReadLimitExceeded) {
		t.Fatalf("expected ErrReadLimitExceeded, got %v", err)
	}
}

func TestReadFromHonorsCanceledContext(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	_, err := ReadFrom(ctx, bytes.NewReader(minimalGTS()), Options{})
	if !errors.Is(err, context.Canceled) {
		t.Fatalf("expected context.Canceled, got %v", err)
	}
}

func TestReadToSinkReadsBoundedInput(t *testing.T) {
	data := minimalGTS()
	var terms, quads int
	result, err := ReadToSink(context.Background(), bytes.NewReader(data), Options{
		AllowSegments: true,
		MaxBytes:      int64(len(data)),
	}, StreamingSinkFunc(func(event StreamingEvent) error {
		switch event.Kind {
		case StreamingEventTerm:
			terms++
		case StreamingEventQuad:
			quads++
		}
		return nil
	}))
	if err != nil {
		t.Fatalf("ReadToSink returned error: %v", err)
	}
	if len(result.Diagnostics) > 0 {
		t.Fatalf("unexpected diagnostics: %v", result.Diagnostics)
	}
	if terms != 3 {
		t.Fatalf("expected 3 term events, got %d", terms)
	}
	if quads != 1 {
		t.Fatalf("expected 1 quad event, got %d", quads)
	}
	if len(result.SegmentHeads) != 1 {
		t.Fatalf("expected 1 segment head, got %d", len(result.SegmentHeads))
	}
}

func TestReadToSinkHonorsMaxBytes(t *testing.T) {
	data := minimalGTS()
	_, err := ReadToSink(context.Background(), bytes.NewReader(data), Options{
		MaxBytes: int64(len(data) - 1),
	}, StreamingSinkFunc(func(StreamingEvent) error { return nil }))
	if !errors.Is(err, ErrReadLimitExceeded) {
		t.Fatalf("expected ErrReadLimitExceeded, got %v", err)
	}
}

func TestReadToSinkHonorsCanceledContext(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	_, err := ReadToSink(ctx, bytes.NewReader(minimalGTS()), Options{}, StreamingSinkFunc(func(StreamingEvent) error {
		return nil
	}))
	if !errors.Is(err, context.Canceled) {
		t.Fatalf("expected context.Canceled, got %v", err)
	}
}

func TestReadToSinkRejectsNilSink(t *testing.T) {
	_, err := ReadToSink(context.Background(), bytes.NewReader(minimalGTS()), Options{}, nil)
	if err == nil {
		t.Fatal("expected nil sink error")
	}
}
