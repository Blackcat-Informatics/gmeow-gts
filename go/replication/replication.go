// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package replication provides byte-range inventory helpers for the Go CLI.
package replication

import (
	"bytes"
	"encoding/json"
	"fmt"

	"github.com/fxamacker/cbor/v2"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/wire"
)

// FrameInventory records one frame's byte boundary and verified id status.
type FrameInventory struct {
	ItemIndex  int
	FrameIndex int
	Start      int
	End        int
	ID         []byte
	FrameType  string
	Valid      bool
}

// SegmentInventory records one segment's byte and fold metadata.
type SegmentInventory struct {
	Index       int
	ItemStart   int
	ItemEnd     int
	Start       int
	End         int
	Profile     string
	Head        []byte
	FrameCount  int
	Layout      model.StreamableInfo
	Diagnostics []model.Diagnostic
	Frames      []FrameInventory
}

// Inventory records the replication view of a GTS file.
type Inventory struct {
	Segments  []SegmentInventory
	Fatal     *model.Diagnostic
	Torn      int
	CleanEnd  int
	ItemCount int
}

// ByteRange is a half-open byte range.
type ByteRange struct {
	Start int
	End   int
}

// MissingStatus names the peer-head comparison result.
type MissingStatus string

// MissingComplete, MissingRanges, MissingUnknown, and MissingError describe
// the result of comparing a peer head against local inventory.
const (
	MissingComplete MissingStatus = "complete"
	MissingRanges   MissingStatus = "ranges"
	MissingUnknown  MissingStatus = "unknown"
	MissingError    MissingStatus = "error"
)

// MissingResult records byte ranges needed after a peer head.
type MissingResult struct {
	Status       MissingStatus
	FromHead     []byte
	Ranges       []ByteRange
	ScanRequired bool
	Detail       string
}

// HasProblems reports whether the inventory is not clean enough for byte resume.
func (inv *Inventory) HasProblems() bool {
	if inv.Fatal != nil || inv.Torn >= 0 {
		return true
	}
	for _, segment := range inv.Segments {
		if len(segment.Diagnostics) > 0 {
			return true
		}
	}
	return false
}

func (inv *Inventory) problemDetail() string {
	if inv.Fatal != nil {
		return fmt.Sprintf("%s: %s", inv.Fatal.Code, inv.Fatal.Detail)
	}
	if inv.Torn >= 0 {
		return fmt.Sprintf("torn at offset %d", inv.Torn)
	}
	for _, segment := range inv.Segments {
		if len(segment.Diagnostics) > 0 {
			d := segment.Diagnostics[0]
			return fmt.Sprintf("%s: %s", d.Code, d.Detail)
		}
	}
	return ""
}

func asText(v interface{}) (string, bool) {
	return wire.AsText(v)
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
	_, hasGTS := wire.MapGet(m, "gts")
	_, hasType := wire.MapGet(m, "t")
	return hasGTS && !hasType
}

func itemEnd(items []struct {
	Offset int
	Item   interface{}
}, torn, dataLen, index int) int {
	if index+1 < len(items) {
		return items[index+1].Offset
	}
	if torn >= 0 {
		return torn
	}
	return dataLen
}

func headerProfile(item interface{}) string {
	header, err := wire.UnwrapHeader(item)
	if err != nil {
		return "generic"
	}
	if value, ok := wire.MapGet(header, "prof"); ok {
		if text, ok := asText(value); ok {
			return text
		}
	}
	return "generic"
}

func headerStoredID(item interface{}) []byte {
	header, err := wire.UnwrapHeader(item)
	if err != nil {
		return nil
	}
	if value, ok := wire.MapGet(header, "id"); ok {
		if id, ok := wire.AsBytes(value); ok {
			return id
		}
	}
	return nil
}

func headerComputedID(item interface{}) []byte {
	header, err := wire.UnwrapHeader(item)
	if err != nil {
		return nil
	}
	return wire.HeaderID(header)
}

func collectFrames(items []struct {
	Offset int
	Item   interface{}
}, torn, dataLen, start, end int) []FrameInventory {
	frames := make([]FrameInventory, 0, end-start-1)
	expectedPrev := headerStoredID(items[start].Item)
	if expectedPrev == nil {
		expectedPrev = headerComputedID(items[start].Item)
	}
	if expectedPrev == nil {
		expectedPrev = []byte{}
	}
	for itemIndex := start + 1; itemIndex < end; itemIndex++ {
		itemStart := items[itemIndex].Offset
		itemStop := itemEnd(items, torn, dataLen, itemIndex)
		frameIndex := itemIndex - start - 1
		frame, ok := items[itemIndex].Item.(map[interface{}]interface{})
		if !ok {
			frames = append(frames, FrameInventory{
				ItemIndex:  itemIndex,
				FrameIndex: frameIndex,
				Start:      itemStart,
				End:        itemStop,
				ID:         []byte{},
				FrameType:  "<non-map>",
				Valid:      false,
			})
			continue
		}
		computed := wire.ContentID(frame)
		var storedID []byte
		if value, ok := wire.MapGet(frame, "id"); ok {
			storedID, _ = wire.AsBytes(value)
		}
		frameID := computed
		if storedID != nil {
			frameID = storedID
		}
		var prev []byte
		if value, ok := wire.MapGet(frame, "prev"); ok {
			prev, _ = wire.AsBytes(value)
		}
		ftype := "<unknown>"
		if value, ok := wire.MapGet(frame, "t"); ok {
			if text, ok := asText(value); ok {
				ftype = text
			}
		}
		frames = append(frames, FrameInventory{
			ItemIndex:  itemIndex,
			FrameIndex: frameIndex,
			Start:      itemStart,
			End:        itemStop,
			ID:         frameID,
			FrameType:  ftype,
			Valid:      storedID != nil && bytes.Equal(storedID, computed) && bytes.Equal(prev, expectedPrev),
		})
		expectedPrev = frameID
	}
	return frames
}

// InventoryFor scans data into the replication inventory shape.
func InventoryFor(data []byte) *Inventory {
	items, torn := wire.IterItems(data)
	cleanEnd := len(data)
	if torn >= 0 {
		cleanEnd = torn
	}
	fs := reader.ReadFileSegments(data)
	if len(items) == 0 || fs.Fatal != nil {
		return &Inventory{
			Segments:  []SegmentInventory{},
			Fatal:     fs.Fatal,
			Torn:      torn,
			CleanEnd:  cleanEnd,
			ItemCount: len(items),
		}
	}
	var bounds []int
	for i, item := range items {
		if isHeaderItem(item.Item) {
			bounds = append(bounds, i)
		}
	}
	if len(bounds) == 0 || bounds[0] != 0 {
		return &Inventory{
			Segments:  []SegmentInventory{},
			Fatal:     fs.Fatal,
			Torn:      torn,
			CleanEnd:  cleanEnd,
			ItemCount: len(items),
		}
	}
	segments := make([]SegmentInventory, 0, len(bounds))
	for index, startItem := range bounds {
		endItem := len(items)
		if index+1 < len(bounds) {
			endItem = bounds[index+1]
		}
		if index >= len(fs.Segments) {
			break
		}
		graph := fs.Segments[index]
		start := items[startItem].Offset
		end := cleanEnd
		if endItem < len(items) {
			end = items[endItem].Offset
		}
		profile := headerProfile(items[startItem].Item)
		if len(graph.SegmentProfiles) > 0 {
			profile = graph.SegmentProfiles[0]
		}
		var head []byte
		if len(graph.SegmentHeads) > 0 {
			head = graph.SegmentHeads[0]
		}
		layout := model.StreamableInfo{}
		if len(graph.SegmentStreamable) > 0 {
			layout = graph.SegmentStreamable[0]
		}
		segments = append(segments, SegmentInventory{
			Index:       index,
			ItemStart:   startItem,
			ItemEnd:     endItem,
			Start:       start,
			End:         end,
			Profile:     profile,
			Head:        head,
			FrameCount:  endItem - startItem - 1,
			Layout:      layout,
			Diagnostics: graph.Diagnostics,
			Frames:      collectFrames(items, torn, len(data), startItem, endItem),
		})
	}
	return &Inventory{
		Segments:  segments,
		Fatal:     fs.Fatal,
		Torn:      torn,
		CleanEnd:  cleanEnd,
		ItemCount: len(items),
	}
}

func aggregateDigest(inv *Inventory) []byte {
	heads := []interface{}{}
	for _, segment := range inv.Segments {
		if segment.Head != nil {
			heads = append(heads, segment.Head)
		}
	}
	return wire.Blake3_256(wire.MustEncode([]interface{}{"gts-segment-heads-v1", heads}))
}

func optionalHex(bytes []byte) *string {
	if bytes == nil {
		return nil
	}
	text := wire.Hex(bytes)
	return &text
}

type jsonDiagnostic struct {
	Code       string `json:"code"`
	Detail     string `json:"detail"`
	FrameIndex *int   `json:"frame_index"`
}

func diagnosticJSON(d model.Diagnostic) jsonDiagnostic {
	return jsonDiagnostic{Code: d.Code, Detail: d.Detail, FrameIndex: d.FrameIndex}
}

func diagnosticsJSON(diagnostics []model.Diagnostic) []jsonDiagnostic {
	out := make([]jsonDiagnostic, 0, len(diagnostics))
	for _, diagnostic := range diagnostics {
		out = append(out, diagnosticJSON(diagnostic))
	}
	return out
}

type jsonLayout struct {
	Claimed bool    `json:"claimed"`
	Covered int     `json:"covered"`
	Tail    int     `json:"tail"`
	Head    *string `json:"head"`
}

func layoutJSON(layout model.StreamableInfo) jsonLayout {
	return jsonLayout{
		Claimed: layout.Claimed,
		Covered: layout.Covered,
		Tail:    layout.Tail,
		Head:    optionalHex(layout.Head),
	}
}

type jsonRange struct {
	Start  int `json:"start"`
	End    int `json:"end"`
	Length int `json:"length"`
}

func rangeJSON(r ByteRange) jsonRange {
	length := r.End - r.Start
	if length < 0 {
		length = 0
	}
	return jsonRange{Start: r.Start, End: r.End, Length: length}
}

func jsonLine(v interface{}) string {
	data, err := json.Marshal(v)
	if err != nil {
		panic(err)
	}
	return string(data) + "\n"
}

type jsonItemRange struct {
	Start int `json:"start"`
	End   int `json:"end"`
}

type jsonSegment struct {
	Index       int              `json:"index"`
	ByteRange   jsonRange        `json:"byte_range"`
	ItemRange   jsonItemRange    `json:"item_range"`
	Profile     string           `json:"profile"`
	Head        *string          `json:"head"`
	FrameCount  int              `json:"frame_count"`
	Layout      jsonLayout       `json:"layout"`
	Diagnostics []jsonDiagnostic `json:"diagnostics"`
}

// HeadsJSON returns the gts-replication-heads-v1 JSON document.
func HeadsJSON(inv *Inventory) string {
	segmentHeads := []string{}
	for _, segment := range inv.Segments {
		if segment.Head != nil {
			segmentHeads = append(segmentHeads, wire.Hex(segment.Head))
		}
	}
	var fileHead []byte
	if len(inv.Segments) > 0 {
		fileHead = inv.Segments[len(inv.Segments)-1].Head
	}
	var tornAt *int
	if inv.Torn >= 0 {
		torn := inv.Torn
		tornAt = &torn
	}
	var fatal *jsonDiagnostic
	if inv.Fatal != nil {
		doc := diagnosticJSON(*inv.Fatal)
		fatal = &doc
	}
	return jsonLine(struct {
		Schema       string   `json:"schema"`
		Clean        bool     `json:"clean"`
		SegmentHeads []string `json:"segment_heads"`
		Aggregate    struct {
			Schema   string  `json:"schema"`
			Count    int     `json:"count"`
			Digest   string  `json:"digest"`
			FileHead *string `json:"file_head"`
		} `json:"aggregate"`
		TornAt *int            `json:"torn_at"`
		Fatal  *jsonDiagnostic `json:"fatal"`
	}{
		Schema:       "gts-replication-heads-v1",
		Clean:        !inv.HasProblems(),
		SegmentHeads: segmentHeads,
		Aggregate: struct {
			Schema   string  `json:"schema"`
			Count    int     `json:"count"`
			Digest   string  `json:"digest"`
			FileHead *string `json:"file_head"`
		}{
			Schema:   "gts-segment-heads-v1",
			Count:    len(segmentHeads),
			Digest:   wire.Hex(aggregateDigest(inv)),
			FileHead: optionalHex(fileHead),
		},
		TornAt: tornAt,
		Fatal:  fatal,
	})
}

// SegmentsJSON returns the gts-replication-segments-v1 JSON document.
func SegmentsJSON(inv *Inventory) string {
	segments := make([]jsonSegment, 0, len(inv.Segments))
	for _, segment := range inv.Segments {
		segments = append(segments, jsonSegment{
			Index:     segment.Index,
			ByteRange: rangeJSON(ByteRange{Start: segment.Start, End: segment.End}),
			ItemRange: jsonItemRange{
				Start: segment.ItemStart,
				End:   segment.ItemEnd,
			},
			Profile:     segment.Profile,
			Head:        optionalHex(segment.Head),
			FrameCount:  segment.FrameCount,
			Layout:      layoutJSON(segment.Layout),
			Diagnostics: diagnosticsJSON(segment.Diagnostics),
		})
	}
	var tornAt *int
	if inv.Torn >= 0 {
		torn := inv.Torn
		tornAt = &torn
	}
	var fatal *jsonDiagnostic
	if inv.Fatal != nil {
		doc := diagnosticJSON(*inv.Fatal)
		fatal = &doc
	}
	return jsonLine(struct {
		Schema    string          `json:"schema"`
		Clean     bool            `json:"clean"`
		Segments  []jsonSegment   `json:"segments"`
		ItemCount int             `json:"item_count"`
		TornAt    *int            `json:"torn_at"`
		Fatal     *jsonDiagnostic `json:"fatal"`
	}{
		Schema:    "gts-replication-segments-v1",
		Clean:     !inv.HasProblems(),
		Segments:  segments,
		ItemCount: inv.ItemCount,
		TornAt:    tornAt,
		Fatal:     fatal,
	})
}

// Missing compares a peer head against local segment and frame ancestry.
func Missing(inv *Inventory, fromHead []byte) MissingResult {
	if inv.HasProblems() {
		return MissingResult{
			Status:       MissingError,
			FromHead:     fromHead,
			Ranges:       []ByteRange{},
			ScanRequired: false,
			Detail:       inv.problemDetail(),
		}
	}
	for _, segment := range inv.Segments {
		if bytes.Equal(segment.Head, fromHead) {
			ranges := []ByteRange{}
			if segment.End < inv.CleanEnd {
				ranges = append(ranges, ByteRange{Start: segment.End, End: inv.CleanEnd})
			}
			status := MissingComplete
			if len(ranges) > 0 {
				status = MissingRanges
			}
			return MissingResult{Status: status, FromHead: fromHead, Ranges: ranges, ScanRequired: false}
		}
		for _, frame := range segment.Frames {
			if frame.Valid && bytes.Equal(frame.ID, fromHead) {
				ranges := []ByteRange{}
				if frame.End < inv.CleanEnd {
					ranges = append(ranges, ByteRange{Start: frame.End, End: inv.CleanEnd})
				}
				status := MissingComplete
				if len(ranges) > 0 {
					status = MissingRanges
				}
				return MissingResult{Status: status, FromHead: fromHead, Ranges: ranges, ScanRequired: false}
			}
		}
	}
	return MissingResult{
		Status:       MissingUnknown,
		FromHead:     fromHead,
		Ranges:       []ByteRange{},
		ScanRequired: true,
		Detail:       "unknown peer head; scan required",
	}
}

// MissingJSON returns the gts-replication-missing-v1 JSON document.
func MissingJSON(result MissingResult) string {
	ranges := make([]jsonRange, 0, len(result.Ranges))
	for _, r := range result.Ranges {
		ranges = append(ranges, rangeJSON(r))
	}
	var detail *string
	if result.Detail != "" {
		detail = &result.Detail
	}
	return jsonLine(struct {
		Schema       string        `json:"schema"`
		Status       MissingStatus `json:"status"`
		FromHead     string        `json:"from_head"`
		Ranges       []jsonRange   `json:"ranges"`
		ScanRequired bool          `json:"scan_required"`
		Detail       *string       `json:"detail"`
	}{
		Schema:       "gts-replication-missing-v1",
		Status:       result.Status,
		FromHead:     wire.Hex(result.FromHead),
		Ranges:       ranges,
		ScanRequired: result.ScanRequired,
		Detail:       detail,
	})
}

// ResumeAfter returns bytes after a verified frame id and before any torn tail.
func ResumeAfter(data, frameID []byte) ([]byte, error) {
	inv := InventoryFor(data)
	if inv.HasProblems() {
		detail := inv.problemDetail()
		if detail == "" {
			detail = "input is not clean"
		}
		return nil, fmt.Errorf("%s", detail)
	}
	for _, segment := range inv.Segments {
		for _, frame := range segment.Frames {
			if frame.Valid && bytes.Equal(frame.ID, frameID) {
				return data[frame.End:inv.CleanEnd], nil
			}
		}
	}
	return nil, fmt.Errorf("frame %s not found", wire.Hex(frameID))
}
