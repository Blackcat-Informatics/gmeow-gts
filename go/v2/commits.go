// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import "bytes"

const indexType = 15

// CommitMismatch reports the failed claims of one commit. ExpectedCount is the
// number of ordinary frames before it, including earlier commits and indexes.
type CommitMismatch struct {
	Offset        int
	ExpectedCount uint64
	CountMismatch bool
	RootMismatch  bool
}

// CommitSummary describes the observed prefix of a segment. CommittedFrames
// includes the last valid commit, or is zero when there is none. TailFrames
// excludes indexes and frames without effective IDs.
type CommitSummary struct {
	CommittedFrames uint64
	TailFrames      uint64
	HasIndex        bool
	Mismatches      []CommitMismatch
}

// CheckCommits checks P5 commit claims in a segment produced by ReadWire.
// Link, stored, and payload damage are reported independently by ReadWire.
func (s Segment) CheckCommits() CommitSummary {
	var summary CommitSummary
	if s.Opener.ID == nil {
		return summary
	}
	var prefix prefixRange
	prefix.push(*s.Opener.ID)
	for _, item := range s.Items {
		frame := item.Frame
		if frame == nil {
			continue
		}
		if frame.Type == indexType {
			summary.HasIndex = true
		} else if frame.EffectiveID != nil {
			summary.TailFrames++
		}
		if frame.Type == commitType && frame.Malformed == "" && frame.contentWithinCapacity {
			check := prefix.checkCommit(item.Offset, frame.Content)
			if check.CountMismatch || check.RootMismatch {
				summary.Mismatches = append(summary.Mismatches, check)
			} else if frame.Link == Linked || frame.Link == Healed {
				summary.CommittedFrames = prefix.count
				summary.TailFrames = 0
			}
		}
		if frame.EffectiveID != nil {
			prefix.push(*frame.EffectiveID)
		}
	}
	return summary
}

func (r *prefixRange) checkCommit(offset int, content []byte) CommitMismatch {
	// ReadWire checked the complete [count, digest] record.
	values, _ := array(content, 2)
	count, _ := uintValue(values.next())
	root, _ := byteValue(values.next(), len(Digest{}))
	expected := r.root()
	return CommitMismatch{
		Offset: offset, ExpectedCount: r.count - 1,
		CountMismatch: count != r.count-1, RootMismatch: !bytes.Equal(root, expected[:]),
	}
}
