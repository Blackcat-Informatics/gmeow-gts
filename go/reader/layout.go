// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"fmt"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

// indexRecord is one intact index frame (§6.2): its absolute item position
// plus the covered-region boundary (count, head).
type indexRecord struct {
	pos   int
	count int
	head  []byte
}

// blobEvent is one inline blob arrival (§3.3): its absolute item position,
// digest, and whether a stream:digest description preceded it.
type blobEvent struct {
	pos       int
	digest    string
	described bool
}

// hIndex records an intact index frame (§6.2) for the layout check (§3.3).
//
// The index stays an accelerator for the fold itself; only "count" and
// "head" are consumed here, as the covered-region boundary. A payload
// without a valid count/head pair is simply not an intact index.
func (f *folder) hIndex(payload interface{}, index int) {
	entries, ok := payload.(map[interface{}]interface{})
	if !ok {
		return
	}
	countRaw, ok := wire.MapGet(entries, "count")
	if !ok {
		return
	}
	count, ok := asIdx(countRaw)
	if !ok {
		return
	}
	headRaw, ok := wire.MapGet(entries, "head")
	if !ok {
		return
	}
	head, ok := headRaw.([]byte)
	if !ok {
		return
	}
	f.indexRecords = append(f.indexRecords, indexRecord{pos: index, count: count, head: head})
}

// layoutCheck computes one segment's layout state and checks its claim (§3.3).
//
// For a segment claiming "layout": "streamable": (a) it must carry an intact
// index footer, (b) the last index's head must be the id of frame count, and
// (c) every covered inline blob must arrive after the stream:digest quad
// describing it. Frames after the last index are the legal accretive tail —
// boundary info, never a diagnostic. Unknown layout values impose no check (§5).
func layoutCheck(
	header map[interface{}]interface{},
	fld *folder,
	frameIDs [][]byte,
	indexOffset int,
) model.StreamableInfo {
	layout, _ := wire.MapGet(header, "layout")
	claimed := textOr(layout, "") == "streamable"
	total := len(frameIDs)
	if !claimed {
		return model.StreamableInfo{}
	}
	if len(fld.indexRecords) == 0 {
		fld.diag(
			"StreamableLayoutError",
			"segment claims layout 'streamable' but carries no intact index footer (§3.3)",
			nil,
		)
		return model.StreamableInfo{Claimed: true, Covered: 0, Tail: total}
	}
	last := fld.indexRecords[len(fld.indexRecords)-1]
	absPos, count, head := last.pos, last.count, last.head
	relPos := absPos - indexOffset // 1-based frame position of the index
	tail := total - relPos
	// The footer must IMMEDIATELY follow the frames it covers (§3.3): a
	// permissive count <= relPos-1 would let frames sit between the covered
	// prefix and the footer, counted neither as covered nor as tail.
	if count != relPos-1 || count < 1 || !bytesEqual(frameIDs[count-1], head) {
		pos := absPos
		fld.diag(
			"StreamableLayoutError",
			fmt.Sprintf("index footer contradicts the frames it covers: count %d "+
				"must name the frame immediately before the footer and head "+
				"must be that frame's id (§3.3)", count),
			&pos,
		)
	}
	for _, ev := range fld.blobEvents {
		blobRel := ev.pos - indexOffset
		if blobRel <= count && !ev.described {
			pos := ev.pos
			fld.diag(
				"StreamableLayoutError",
				fmt.Sprintf("covered blob %s delivered before its stream:digest "+
					"description (catalog-before-payload, §3.3)", ev.digest),
				&pos,
			)
		}
	}
	return model.StreamableInfo{Claimed: true, Covered: count, Tail: tail, Head: head}
}
