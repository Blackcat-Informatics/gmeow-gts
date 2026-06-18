// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package nested implements bounded nested-GTS discovery for Full Reader callers.
package nested

import (
	"fmt"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/wire"
)

// GTSMediaType is the declared media type for a nested GTS CBOR Sequence blob.
const GTSMediaType = "application/vnd.blackcat.gts+cbor-seq"

// ReadResult is a root fold plus nested folds addressed by containing blob digest.
type ReadResult struct {
	Graph       *model.Graph
	Subgraphs   map[string]*model.Graph
	Diagnostics []model.Diagnostic
}

// Subgraph returns the nested graph carried by digest, when one was exposed.
func (r *ReadResult) Subgraph(digest string) (*model.Graph, bool) {
	if r == nil || r.Subgraphs == nil {
		return nil, false
	}
	g, ok := r.Subgraphs[digest]
	return g, ok
}

// ReadNested reads a GTS file and boundedly recurses into nested-GTS blobs.
//
// Baseline readers treat nested GTS as ordinary blobs. Full Reader callers use
// this helper to expose subgraphs by blob digest while enforcing the recursion
// and decoded-size budgets required by the v1 security policy.
func ReadNested(data []byte, maxDepth int, maxDecodedBytes int) *ReadResult {
	if maxDepth < 0 {
		maxDepth = 0
	}
	if maxDecodedBytes < 0 {
		maxDecodedBytes = 0
	}
	remaining := maxDecodedBytes
	seen := map[string]struct{}{}
	subgraphs := map[string]*model.Graph{}
	root := visit(data, 0, maxDepth, &remaining, seen, subgraphs)
	graphs := []*model.Graph{root}
	for _, graph := range subgraphs {
		graphs = append(graphs, graph)
	}
	diagnostics := []model.Diagnostic{}
	for _, graph := range graphs {
		diagnostics = append(diagnostics, graph.Diagnostics...)
	}
	return &ReadResult{
		Graph:       root,
		Subgraphs:   subgraphs,
		Diagnostics: diagnostics,
	}
}

func visit(
	data []byte,
	depth int,
	maxDepth int,
	remaining *int,
	seen map[string]struct{},
	subgraphs map[string]*model.Graph,
) *model.Graph {
	graph := reader.Read(data, true, nil)
	for _, entry := range graph.BlobMeta {
		if blobMediaType(entry.Meta) != GTSMediaType {
			continue
		}
		if _, ok := seen[entry.Digest]; ok {
			continue
		}
		nestedBytes, ok := blobBytes(graph, entry.Digest)
		if !ok {
			continue
		}
		if depth >= maxDepth {
			graph.Diagnostics = append(graph.Diagnostics, model.Diagnostic{
				Code: "RecursionLimit",
				Detail: fmt.Sprintf(
					"nested GTS blob %s exceeds max depth %d",
					entry.Digest,
					maxDepth,
				),
			})
			continue
		}
		if len(nestedBytes) > *remaining {
			graph.Diagnostics = append(graph.Diagnostics, model.Diagnostic{
				Code: "RecursionLimit",
				Detail: fmt.Sprintf(
					"nested GTS decoded-size budget exceeded at %s: %d > %d",
					entry.Digest,
					len(nestedBytes),
					*remaining,
				),
			})
			continue
		}
		*remaining -= len(nestedBytes)
		seen[entry.Digest] = struct{}{}
		child := visit(nestedBytes, depth+1, maxDepth, remaining, seen, subgraphs)
		if len(child.SegmentHeads) == 0 {
			graph.Diagnostics = append(graph.Diagnostics, model.Diagnostic{
				Code:   "DamagedFrame",
				Detail: fmt.Sprintf("nested GTS blob %s could not be parsed", entry.Digest),
			})
			continue
		}
		subgraphs[entry.Digest] = child
	}
	return graph
}

func blobBytes(graph *model.Graph, digest string) ([]byte, bool) {
	for _, blob := range graph.Blobs {
		if blob.Digest == digest {
			return blob.Data, true
		}
	}
	return nil, false
}

func blobMediaType(meta interface{}) string {
	entries, ok := meta.(map[interface{}]interface{})
	if !ok {
		return ""
	}
	value, ok := wire.MapGet(entries, "mt")
	if !ok {
		return ""
	}
	mt, _ := wire.AsText(value)
	return mt
}
