// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package reader implements the GTS Baseline Reader contract (§2.1).
package reader

import (
	"bytes"
	"fmt"

	"github.com/fxamacker/cbor/v2"
	"go.blackcatinformatics.ca/gts/codec"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

// asInt64 coerces a decoded CBOR value to int64.
func asInt64(v interface{}) (int64, bool) {
	return wire.AsInt64(v)
}

// asIdx coerces a decoded CBOR value to a non-negative int (term index).
func asIdx(v interface{}) (int, bool) {
	return wire.AsInt(v)
}

// asText coerces a decoded CBOR value to a string.
func asText(v interface{}) (string, bool) {
	return wire.AsText(v)
}

// textOr returns a text value or a default.
func textOr(v interface{}, def string) string {
	return wire.TextOr(v, def)
}

// fmtOpt formats an optional graph slot for diagnostics.
func fmtOpt(g *int) string {
	if g == nil {
		return "None"
	}
	return fmt.Sprintf("%d", *g)
}

// diagCodeFor maps a codec failure reason to a diagnostic code.
func diagCodeFor(reason string) string {
	if reason == "missing-key" {
		return "MissingKey"
	}
	return "UnknownCodec"
}

type payloadError struct {
	unavailable bool
	reason      string
	detail      string
	damaged     bool
}

// codecErrToPayload translates a codec.Error into a reader payloadError.
func codecErrToPayload(e error) *payloadError {
	if ce, ok := e.(*codec.Error); ok {
		return &payloadError{
			unavailable: !ce.Failed,
			reason:      ce.Reason,
			detail:      ce.Detail,
			damaged:     ce.Failed,
		}
	}
	return &payloadError{damaged: true, detail: e.Error()}
}

type folder struct {
	g            *model.Graph
	catalog      map[int64]*codec.Codec
	segmentIndex int
	sink         StreamingSink
	materialize  bool
	eventErr     error
	// Layout-state bookkeeping (§3.3): intact index frames seen, digests the
	// graph has described via stream:digest so far, and each inline blob's
	// arrival.
	indexRecords []indexRecord
	described    map[string]struct{}
	blobEvents   []blobEvent
}

func (f *folder) emit(event StreamingEvent) {
	if f.sink == nil || f.eventErr != nil {
		return
	}
	event.SegmentIndex = f.segmentIndex
	if err := f.sink.Accept(event); err != nil {
		f.eventErr = err
	}
}

// diag appends a diagnostic to the folded graph.
func (f *folder) diag(code, detail string, index *int) {
	diag := model.Diagnostic{
		Code:       code,
		Detail:     detail,
		FrameIndex: index,
	}
	f.g.Diagnostics = append(f.g.Diagnostics, diag)
	f.emit(StreamingEvent{
		Kind:       StreamingEventDiagnostic,
		FrameIndex: eventFrameIndex(index),
		Diagnostic: diag,
	})
}

func (f *folder) pushOpaque(opaque model.OpaqueNode, index int) {
	if f.materialize {
		f.g.Opaque = append(f.g.Opaque, opaque)
	}
	f.emit(StreamingEvent{
		Kind:       StreamingEventOpaque,
		FrameIndex: index,
		Opaque:     opaque,
	})
}

func (f *folder) pushSignature(signature model.Signature, index int) {
	if f.materialize {
		f.g.Signatures = append(f.g.Signatures, signature)
	}
	f.emit(StreamingEvent{
		Kind:       StreamingEventSignature,
		FrameIndex: index,
		Signature:  signature,
	})
}

// resolveCodecs looks up codec ids in the header catalog.
func (f *folder) resolveCodecs(ids []interface{}) ([]*codec.Codec, *payloadError) {
	var chain []*codec.Codec
	for _, cid := range ids {
		n, ok := asInt64(cid)
		if !ok {
			return nil, &payloadError{
				unavailable: true,
				reason:      "unknown-codec",
				detail:      fmt.Sprintf("codec id %v not an integer", cid),
			}
		}
		c, ok := f.catalog[n]
		if !ok {
			return nil, &payloadError{
				unavailable: true,
				reason:      "unknown-codec",
				detail:      fmt.Sprintf("codec id %v not in catalog", cid),
			}
		}
		chain = append(chain, c)
	}
	return chain, nil
}

func (f *folder) payload(frame map[interface{}]interface{}, isBlob bool) (interface{}, *payloadError) {
	d := frame["d"]
	if x, ok := frame["x"]; ok {
		if ids, ok := x.([]interface{}); ok && len(ids) > 0 {
			b, ok := d.([]byte)
			if !ok {
				return nil, &payloadError{damaged: true, detail: "transformed frame 'd' must be a byte string"}
			}
			chain, err := f.resolveCodecs(ids)
			if err != nil {
				return nil, err
			}
			decoded, derr := codec.DecodeChain(chain, b)
			if derr != nil {
				return nil, codecErrToPayload(derr)
			}
			if isBlob {
				return decoded, nil
			}
			var out interface{}
			if uerr := cbor.Unmarshal(decoded, &out); uerr != nil {
				return nil, &payloadError{damaged: true, detail: fmt.Sprintf("payload decode failed: %v", uerr)}
			}
			return out, nil
		}
	}
	if d == nil {
		return nil, nil
	}
	return d, nil
}

func (f *folder) foldFrame(frame map[interface{}]interface{}, index int) {
	ftype := textOr(frame["t"], "")
	payload, perr := f.payload(frame, ftype == "blob")
	if perr != nil {
		if perr.unavailable {
			f.opaque(frame, ftype, perr.reason, index)
			f.diag(diagCodeFor(perr.reason), perr.detail, &index)
		} else {
			f.opaque(frame, ftype, "damaged", index)
			f.diag("DamagedFrame", fmt.Sprintf("payload decode failed: %s", perr.detail), &index)
		}
		return
	}
	switch ftype {
	case "terms":
		f.hTerms(payload, index)
	case "quads":
		f.hQuads(payload, index)
	case "reifies":
		f.hReifies(payload, index)
	case "annot":
		f.hAnnot(payload, index)
	case "blob":
		f.hBlob(payload, frame, index)
	case "meta":
		f.hMeta(payload)
	case "suppress":
		f.hSuppress(payload, index)
	case "snapshot":
		f.hSnapshot(payload, index)
	case "index":
		f.hIndex(payload, index)
	case "opaque":
		f.hOpaque(payload, index)
	default:
		f.opaque(frame, ftype, "unknown-frame-type", index)
		f.diag("UnknownFrameType", fmt.Sprintf("unsupported frame type %q", ftype), &index)
	}
}

func isHeaderItem(item interface{}) bool {
	inner := item
	if tag, ok := item.(cbor.Tag); ok {
		inner = tag.Content
	}
	m, ok := inner.(map[interface{}]interface{})
	if !ok {
		return false
	}
	_, hasGts := wire.MapGet(m, "gts")
	_, hasT := wire.MapGet(m, "t")
	return hasGts && !hasT
}

func catalogFrom(header map[interface{}]interface{}) map[int64]*codec.Codec {
	out := make(map[int64]*codec.Codec)
	cat, ok := wire.MapGet(header, "cat")
	if !ok {
		return out
	}
	raw, ok := cat.(map[interface{}]interface{})
	if !ok {
		return out
	}
	for cid, entry := range raw {
		n, ok := asInt64(cid)
		if !ok {
			continue
		}
		fields, ok := entry.(map[interface{}]interface{})
		if !ok {
			continue
		}
		out[n] = &codec.Codec{
			Name: textOr(fields["name"], ""),
			Cls:  textOr(fields["cls"], "encode"),
		}
	}
	return out
}

// Read folds a GTS file into a Graph.
//
// With allowSegments=false the reader emulates a pre-§3.1 reader: a segment
// boundary is a FATAL SegmentBoundary diagnostic and nothing past it is folded.
// expectedHead, when given, is compared against the LAST segment's head.
func Read(data []byte, allowSegments bool, expectedHead []byte) *model.Graph {
	items, torn := wire.IterItems(data)
	if len(items) == 0 {
		g := emptyGraph()
		idx := 0
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{Code: "EmptyFile", Detail: "no CBOR items", FrameIndex: &idx})
		return g
	}

	var bounds []int
	for i, it := range items {
		if isHeaderItem(it.Item) {
			bounds = append(bounds, i)
		}
	}

	g := emptyGraph()
	if len(bounds) == 0 || bounds[0] != 0 {
		idx := 0
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{Code: "DamagedFrame", Detail: "first item is not a header", FrameIndex: &idx})
		return g
	}

	if len(bounds) > 1 && !allowSegments {
		g = readSegment(items[:bounds[1]], 0)
		idx := bounds[1]
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{
			Code:       "SegmentBoundary",
			Detail:     fmt.Sprintf("segment boundary at item %d but reader is in pre-segment mode; remainder of file NOT folded", idx),
			FrameIndex: &idx,
		})
		return g
	}

	var folded []*model.Graph
	for i, a := range bounds {
		b := len(items)
		if i+1 < len(bounds) {
			b = bounds[i+1]
		}
		folded = append(folded, readSegment(items[a:b], a))
	}

	if len(folded) == 1 {
		g = folded[0]
	} else {
		g = unionSegments(folded)
	}

	if expectedHead != nil {
		var lastHead []byte
		if len(g.SegmentHeads) > 0 {
			lastHead = g.SegmentHeads[len(g.SegmentHeads)-1]
		}
		if !bytesEqual(lastHead, expectedHead) {
			g.Diagnostics = append(g.Diagnostics, model.Diagnostic{Code: "TruncatedLog", Detail: "observed head does not match expected head"})
		}
	}
	if torn >= 0 {
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{Code: "TornAppendError", Detail: fmt.Sprintf("torn at offset %d", torn)})
	}
	return g
}

func bytesEqual(a, b []byte) bool {
	return bytes.Equal(a, b)
}

// emptyGraph returns a Graph whose slice fields are non-nil so JSON
// serialization and downstream consumers see empty arrays instead of null.
func emptyGraph() *model.Graph {
	return &model.Graph{
		Terms:             []model.Term{},
		Quads:             []model.Quad{},
		Reifiers:          []model.ReifierEntry{},
		Annotations:       []model.AnnotationEntry{},
		Blobs:             []model.BlobEntry{},
		BlobMeta:          []model.BlobMetaEntry{},
		Meta:              []model.MetaEntry{},
		Suppressions:      []model.Suppression{},
		Opaque:            []model.OpaqueNode{},
		Signatures:        []model.Signature{},
		Diagnostics:       []model.Diagnostic{},
		SegmentHeads:      [][]byte{},
		SegmentProfiles:   []string{},
		SegmentMeta:       [][]model.MetaEntry{},
		SegmentStreamable: []model.StreamableInfo{},
	}
}

// FileSegments is the per-segment view of a file.
type FileSegments struct {
	Segments []*model.Graph
	Torn     int // -1 for clean end
	Fatal    *model.Diagnostic
}

// ReadFileSegments folds a file segment-by-segment WITHOUT unioning.
func ReadFileSegments(data []byte) *FileSegments {
	items, torn := wire.IterItems(data)
	if len(items) == 0 {
		idx := 0
		return &FileSegments{
			Torn:  torn,
			Fatal: &model.Diagnostic{Code: "EmptyFile", Detail: "no CBOR items", FrameIndex: &idx},
		}
	}
	var bounds []int
	for i, it := range items {
		if isHeaderItem(it.Item) {
			bounds = append(bounds, i)
		}
	}
	if len(bounds) == 0 || bounds[0] != 0 {
		idx := 0
		return &FileSegments{
			Torn:  torn,
			Fatal: &model.Diagnostic{Code: "DamagedFrame", Detail: "first item is not a header", FrameIndex: &idx},
		}
	}
	var segments []*model.Graph
	for i, a := range bounds {
		b := len(items)
		if i+1 < len(bounds) {
			b = bounds[i+1]
		}
		segments = append(segments, readSegment(items[a:b], a))
	}
	return &FileSegments{Segments: segments, Torn: torn}
}

func readSegment(items []struct {
	Offset int
	Item   interface{}
}, indexOffset int) *model.Graph {
	g := emptyGraph()
	rawHeader := items[0].Item
	header, err := wire.UnwrapHeader(rawHeader)
	if err != nil {
		idx := indexOffset
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{Code: "DamagedFrame", Detail: fmt.Sprintf("invalid header: %v", err), FrameIndex: &idx})
		return g
	}
	var storedHID []byte
	if v, ok := wire.MapGet(header, "id"); ok {
		storedHID, _ = v.([]byte)
	}
	if !bytesEqual(storedHID, wire.HeaderID(header)) {
		idx := indexOffset
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{Code: "DamagedFrame", Detail: "header self-hash mismatch", FrameIndex: &idx})
	}
	headerMagic, _ := wire.MapGet(header, "gts")
	headerVersion, _ := wire.MapGet(header, "v")
	version, versionOK := asInt64(headerVersion)
	if textOr(headerMagic, "") != wire.Magic || !versionOK || version != int64(wire.Version) {
		idx := indexOffset
		g.Diagnostics = append(g.Diagnostics, model.Diagnostic{
			Code:       "DamagedFrame",
			Detail:     fmt.Sprintf("unsupported header magic/version %v/%v", headerMagic, headerVersion),
			FrameIndex: &idx,
		})
	}
	expectedPrev := storedHID

	catalog := catalogFrom(header)
	fld := &folder{g: g, catalog: catalog, materialize: true, described: make(map[string]struct{})}
	var frameIDs [][]byte // per-frame chain ids, by 0-based frame position
	for idx, it := range items[1:] {
		absIndex := idx + 1 + indexOffset
		frame, ok := it.Item.(map[interface{}]interface{})
		if !ok {
			fld.diag("DamagedFrame", "frame is not a map", &absIndex)
			frameIDs = append(frameIDs, []byte{})
			continue
		}
		var storedID []byte
		if v, ok := frame["id"]; ok {
			storedID, _ = v.([]byte)
		}
		computed := wire.ContentID(frame)
		if !bytesEqual(storedID, computed) {
			fld.diag("DamagedFrame", "frame self-hash mismatch", &absIndex)
			ftype := textOr(frame["t"], "")
			fld.opaque(frame, ftype, "damaged", absIndex)
			if storedID != nil {
				expectedPrev = storedID
			} else {
				expectedPrev = computed
			}
			frameIDs = append(frameIDs, expectedPrev)
			continue
		}
		prevOk := false
		if v, ok := frame["prev"]; ok {
			if b, ok := v.([]byte); ok {
				prevOk = bytesEqual(b, expectedPrev)
			}
		}
		if !prevOk {
			fld.diag("BrokenChain", "prev does not match", &absIndex)
		}
		expectedPrev = computed
		frameIDs = append(frameIDs, expectedPrev)
		if sig, ok := frame["sig"]; ok {
			// The raw COSE bytes are retained so streamable compaction (§10.1)
			// can carry the signature detached.
			if cose, ok := sig.([]byte); ok {
				fld.pushSignature(model.Signature{FrameID: computed, Status: "unverified", Cose: cose}, absIndex)
			} else {
				// present but malformed — record as invalid, never silently drop
				fld.pushSignature(model.Signature{FrameID: computed, Status: "invalid"}, absIndex)
			}
		}
		fld.foldFrame(frame, absIndex)
	}

	g.SegmentHeads = append(g.SegmentHeads, expectedPrev)
	segMeta := make([]model.MetaEntry, len(g.Meta))
	copy(segMeta, g.Meta)
	g.SegmentMeta = append(g.SegmentMeta, segMeta)
	g.SegmentProfiles = append(g.SegmentProfiles, textOr(header["prof"], "generic"))
	g.SegmentStreamable = append(g.SegmentStreamable, layoutCheck(header, fld, frameIDs, indexOffset))
	return g
}
