// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

type rebuildMember struct {
	record rebuildRecord
	length uint64
	body   []byte
}

func (r *rdfRebuild) pack(content, payload []byte) Reason {
	fields := rdfCursor{data: content}
	fields.record(2)
	count := fields.positive()
	unitLength := fields.uint()
	if reason := fields.end(); reason != "" {
		return reason
	}
	c := rdfCursor{data: payload}
	c.record(2)
	members := rdfColumn(&c, func() rebuildMember {
		size := c.list()
		c.check(size > 0, WrongLength)
		tag := c.code(4)
		c.check(size == 3, WrongLength)
		record := c.rebuildRecord(tag + chunkFrameType - 1)
		return rebuildMember{record: record, length: c.uint()}
	})
	bodies := c.body(2)
	if reason := c.end(); reason != "" {
		return reason
	}
	if uint64(len(members)) != count {
		return inconsistentCounts
	}
	if reason := checkRebuildMembers(members, bodies, unitLength); reason != "" {
		return reason
	}
	// Every unit in a pack stands at the same position as its declarations.
	for _, member := range members {
		if member.record.kind <= blobFrameType {
			r.unit(member.record.target.digest, member.body)
		}
	}
	for _, member := range members {
		if member.record.kind >= recipeFrameType {
			r.declare(member.record)
		}
	}
	return ""
}

func checkRebuildMembers(members []rebuildMember, bodies []byte, unitLength uint64) Reason {
	var unitBytes uint64
	for i := range members {
		member := &members[i]
		if member.length > uint64(len(bodies)) {
			return inconsistentCounts
		}
		member.body = bodies[:member.length]
		bodies = bodies[member.length:]
		if member.record.kind <= blobFrameType {
			unitBytes += member.length
		}
	}
	if len(bodies) != 0 || unitBytes != unitLength {
		return inconsistentCounts
	}
	for i := range members {
		member := &members[i]
		switch member.record.kind {
		case chunkFrameType:
			if member.length == 0 {
				return OutOfRange
			}
		case recipeFrameType, valueFrameType:
			if reason := member.record.readEntries(member.body); reason != "" {
				return reason
			}
		}
	}
	return ""
}
