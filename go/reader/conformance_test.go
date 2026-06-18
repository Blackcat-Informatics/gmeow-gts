// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/wire"
)

func vectorsDir(t *testing.T) string {
	dir, err := filepath.Abs("../../vectors")
	if err != nil {
		t.Fatal(err)
	}
	return dir
}

func summarize(g *model.Graph, mode string) map[string]interface{} {
	lines := strings.Split(strings.TrimSuffix(nquads.ToNQuads(g), "\n"), "\n")
	if len(lines) == 1 && lines[0] == "" {
		lines = []string{}
	}
	sort.Strings(lines)
	opaqueReasons := []string{}
	for _, o := range g.Opaque {
		opaqueReasons = append(opaqueReasons, o.Reason)
	}
	sort.Strings(opaqueReasons)

	blobs := map[string]interface{}{}
	for _, b := range g.Blobs {
		mt := ""
		for _, bm := range g.BlobMeta {
			if bm.Digest == b.Digest {
				if m, ok := bm.Meta.(map[interface{}]interface{}); ok {
					if v, ok := m["mt"]; ok {
						if s, ok := v.(string); ok {
							mt = s
						}
					}
				}
			}
		}
		blobs[b.Digest] = map[string]interface{}{"size": len(b.Data), "mt": mt}
	}

	heads := []string{}
	for _, h := range g.SegmentHeads {
		heads = append(heads, wire.Hex(h))
	}

	// Per-segment layout state (§3.3) — pins the streamable claim, its
	// covered boundary, and the accretive tail across implementations.
	streamable := []map[string]interface{}{}
	for _, s := range g.SegmentStreamable {
		streamable = append(streamable, map[string]interface{}{
			"claimed": s.Claimed,
			"covered": s.Covered,
			"tail":    s.Tail,
		})
	}

	diags := []string{}
	for _, d := range g.Diagnostics {
		diags = append(diags, d.Code)
	}

	return map[string]interface{}{
		"mode":           mode,
		"diagnostics":    diags,
		"terms":          len(g.Terms),
		"quads":          len(g.Quads),
		"segments":       len(g.SegmentHeads),
		"segment_heads":  heads,
		"profiles":       g.SegmentProfiles,
		"streamable":     streamable,
		"opaque_reasons": opaqueReasons,
		"suppressions":   len(g.Suppressions),
		"blobs":          blobs,
		"nquads":         lines,
	}
}

type streamingEventCounts struct {
	Terms             int
	Quads             int
	Reifiers          int
	Annotations       int
	Suppressions      int
	Blobs             int
	Opaque            int
	Signatures        int
	Diagnostics       int
	SegmentHeads      int
	StreamableLayouts int
}

func (c *streamingEventCounts) Accept(event StreamingEvent) error {
	switch event.Kind {
	case StreamingEventTerm:
		c.Terms++
	case StreamingEventQuad:
		c.Quads++
	case StreamingEventReifier:
		c.Reifiers++
	case StreamingEventAnnotation:
		c.Annotations++
	case StreamingEventSuppression:
		c.Suppressions++
	case StreamingEventBlob:
		c.Blobs++
	case StreamingEventOpaque:
		c.Opaque++
	case StreamingEventSignature:
		c.Signatures++
	case StreamingEventDiagnostic:
		c.Diagnostics++
	case StreamingEventSegmentHead:
		c.SegmentHeads++
	case StreamingEventStreamableLayout:
		c.StreamableLayouts++
	}
	return nil
}

func segmentEventCounts(data []byte, allowSegments bool) streamingEventCounts {
	fs := ReadFileSegments(data)
	if fs.Fatal != nil {
		return streamingEventCounts{}
	}
	segments := fs.Segments
	if !allowSegments && len(segments) > 1 {
		segments = segments[:1]
	}
	var counts streamingEventCounts
	for _, seg := range segments {
		counts.Terms += len(seg.Terms)
		counts.Quads += len(seg.Quads)
		counts.Reifiers += len(seg.Reifiers)
		counts.Annotations += len(seg.Annotations)
		counts.Suppressions += len(seg.Suppressions)
		counts.Blobs += len(seg.Blobs)
		counts.Opaque += len(seg.Opaque)
		counts.Signatures += len(seg.Signatures)
	}
	return counts
}

func diagnosticSummary(diags []model.Diagnostic) []string {
	out := make([]string, len(diags))
	for i, diag := range diags {
		out[i] = diag.Code
		if diag.FrameIndex != nil {
			out[i] += ":" + strconv.Itoa(*diag.FrameIndex)
		}
	}
	return out
}

func TestCorpus(t *testing.T) {
	dir := vectorsDir(t)
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	var names []string
	for _, e := range entries {
		if ext := filepath.Ext(e.Name()); ext == ".gts" {
			names = append(names, e.Name()[:len(e.Name())-4])
		}
	}
	if len(names) < 16 {
		t.Fatalf("corpus too small: %d vectors", len(names))
	}
	for _, name := range names {
		t.Run(name, func(t *testing.T) {
			//nolint:gosec // test reads frozen conformance vectors by directory-listed name.
			data, err := os.ReadFile(filepath.Join(dir, name+".gts"))
			if err != nil {
				t.Fatal(err)
			}
			//nolint:gosec // test reads frozen conformance expectations by directory-listed name.
			expectedRaw, err := os.ReadFile(filepath.Join(dir, name+".expected.json"))
			if err != nil {
				t.Fatal(err)
			}
			var expected map[string]interface{}
			if err := json.Unmarshal(expectedRaw, &expected); err != nil {
				t.Fatal(err)
			}
			mode, _ := expected["mode"].(string)
			g := Read(data, mode != "pre-segment", nil)
			actual := summarize(g, mode)
			actualJSON, _ := json.Marshal(actual)
			expectedJSON, _ := json.Marshal(expected)
			if string(actualJSON) != string(expectedJSON) {
				t.Fatalf("divergence\nactual:   %s\nexpected: %s", actualJSON, expectedJSON)
			}
		})
	}
}

func TestStreamingFoldCorpusEquivalence(t *testing.T) {
	dir := vectorsDir(t)
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	var names []string
	for _, e := range entries {
		if ext := filepath.Ext(e.Name()); ext == ".gts" {
			names = append(names, e.Name()[:len(e.Name())-4])
		}
	}
	for _, name := range names {
		t.Run(name, func(t *testing.T) {
			//nolint:gosec // test reads frozen conformance vectors by directory-listed name.
			data, err := os.ReadFile(filepath.Join(dir, name+".gts"))
			if err != nil {
				t.Fatal(err)
			}
			//nolint:gosec // test reads frozen conformance expectations by directory-listed name.
			expectedRaw, err := os.ReadFile(filepath.Join(dir, name+".expected.json"))
			if err != nil {
				t.Fatal(err)
			}
			var expected map[string]interface{}
			if err := json.Unmarshal(expectedRaw, &expected); err != nil {
				t.Fatal(err)
			}
			mode, _ := expected["mode"].(string)
			allowSegments := mode != "pre-segment"
			full := Read(data, allowSegments, nil)
			var counts streamingEventCounts
			streamed, err := ReadToSink(context.Background(), bytes.NewReader(data), Options{
				AllowSegments: allowSegments,
			}, &counts)
			if err != nil {
				t.Fatalf("ReadToSink returned error: %v", err)
			}
			if !reflect.DeepEqual(diagnosticSummary(streamed.Diagnostics), diagnosticSummary(full.Diagnostics)) {
				t.Fatalf("diagnostics differ\nstreamed: %#v\nfull:     %#v", streamed.Diagnostics, full.Diagnostics)
			}
			if !reflect.DeepEqual(streamed.SegmentHeads, full.SegmentHeads) {
				t.Fatalf("segment heads differ\nstreamed: %x\nfull:     %x", streamed.SegmentHeads, full.SegmentHeads)
			}
			if !reflect.DeepEqual(streamed.SegmentProfiles, full.SegmentProfiles) {
				t.Fatalf("segment profiles differ\nstreamed: %#v\nfull:     %#v", streamed.SegmentProfiles, full.SegmentProfiles)
			}
			if !reflect.DeepEqual(streamed.SegmentMeta, full.SegmentMeta) {
				t.Fatalf("segment metadata differs\nstreamed: %#v\nfull:     %#v", streamed.SegmentMeta, full.SegmentMeta)
			}
			if !reflect.DeepEqual(streamed.SegmentStreamable, full.SegmentStreamable) {
				t.Fatalf("streamable layout differs\nstreamed: %#v\nfull:     %#v", streamed.SegmentStreamable, full.SegmentStreamable)
			}
			wantCounts := segmentEventCounts(data, allowSegments)
			if counts.Terms != wantCounts.Terms ||
				counts.Quads != wantCounts.Quads ||
				counts.Reifiers != wantCounts.Reifiers ||
				counts.Annotations != wantCounts.Annotations ||
				counts.Suppressions != wantCounts.Suppressions ||
				counts.Blobs != wantCounts.Blobs ||
				counts.Opaque != wantCounts.Opaque ||
				counts.Signatures != wantCounts.Signatures {
				t.Fatalf("fold event counts differ\nstreamed: %#v\nsegments: %#v", counts, wantCounts)
			}
			if counts.Diagnostics != len(streamed.Diagnostics) {
				t.Fatalf("diagnostic event count %d != result diagnostics %d", counts.Diagnostics, len(streamed.Diagnostics))
			}
			if counts.SegmentHeads != len(streamed.SegmentHeads) {
				t.Fatalf("segment-head event count %d != result heads %d", counts.SegmentHeads, len(streamed.SegmentHeads))
			}
			if counts.StreamableLayouts != len(streamed.SegmentStreamable) {
				t.Fatalf("streamable-layout event count %d != result layouts %d", counts.StreamableLayouts, len(streamed.SegmentStreamable))
			}
		})
	}
}
