// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"fmt"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/stream"
	"go.blackcatinformatics.ca/gts/wire"
)

func (f *folder) hTerms(payload interface{}, index int) {
	rows, ok := payload.([]interface{})
	if !ok {
		return
	}
	for _, raw := range rows {
		entries, ok := raw.(map[interface{}]interface{})
		if !ok {
			continue
		}
		kind := model.FromWire(func() int64 {
			if v, ok := wire.MapGet(entries, "k"); ok {
				if n, ok := asInt64(v); ok {
					return n
				}
			}
			return -1
		}())
		value := ""
		if v, ok := wire.MapGet(entries, "v"); ok {
			if s, ok := asText(v); ok {
				value = s
			}
		}
		lang := ""
		if v, ok := wire.MapGet(entries, "l"); ok {
			if s, ok := asText(v); ok {
				lang = s
			}
		}
		direction := ""
		if v, ok := wire.MapGet(entries, "dir"); ok {
			if s, ok := asText(v); ok && (s == "ltr" || s == "rtl") {
				direction = s
			}
		}
		dtRaw, hasDt := wire.MapGet(entries, "dt")
		rfRaw, hasRf := wire.MapGet(entries, "rf")
		tid := len(f.g.Terms)
		sanitize := func(r interface{}) *int {
			if r == nil {
				return nil
			}
			n, ok := asInt64(r)
			if !ok || n < 0 || n >= int64(tid) {
				return nil
			}
			i := int(n)
			return &i
		}
		dt := sanitize(dtRaw)
		rf := sanitize(rfRaw)
		outOfRange := func(r interface{}) bool {
			if r == nil {
				return false
			}
			n, ok := asInt64(r)
			return ok && n >= int64(tid)
		}
		if (hasDt && outOfRange(dtRaw)) || (hasRf && outOfRange(rfRaw)) {
			f.diag("ForwardReference", fmt.Sprintf("term %d has an out-of-range ref", tid), &index)
		}
		f.g.Terms = append(f.g.Terms, model.Term{
			Kind:      kind,
			Value:     value,
			Datatype:  dt,
			Lang:      lang,
			Direction: direction,
			Reifier:   rf,
		})
		f.emit(StreamingEvent{
			Kind:       StreamingEventTerm,
			FrameIndex: index,
			TermID:     tid,
			Term:       f.g.Terms[tid],
		})
		if f.eventErr != nil {
			return
		}
	}
}

func (f *folder) hQuads(payload interface{}, index int) {
	rows, ok := payload.([]interface{})
	if !ok {
		return
	}
	for _, row := range rows {
		items, ok := row.([]interface{})
		if !ok || len(items) < 3 {
			continue
		}
		s, sOk := asIdx(items[0])
		p, pOk := asIdx(items[1])
		o, oOk := asIdx(items[2])
		var gslot *int
		hasGraph := len(items) >= 4
		if hasGraph {
			if g, ok := asIdx(items[3]); ok {
				gslot = &g
			}
		}
		if !sOk || !pOk || !oOk || (hasGraph && gslot == nil) {
			f.diag("DamagedFrame", "quad has non-integer term ids", &index)
			continue
		}
		if !f.checkPositions(s, p, o, gslot, index) {
			continue
		}
		quad := model.Quad{S: s, P: p, O: o, G: gslot}
		if f.materialize {
			f.g.Quads = append(f.g.Quads, quad)
		}
		f.emit(StreamingEvent{
			Kind:       StreamingEventQuad,
			FrameIndex: index,
			Quad:       quad,
		})
		if f.eventErr != nil {
			return
		}
		// Layout bookkeeping (§3.3): a stream:digest quad describes an
		// upcoming manifestation — record the IOU for the blob check.
		if f.g.Terms[p].Value == stream.Digest {
			if obj := &f.g.Terms[o]; obj.Value != "" {
				f.described[obj.Value] = struct{}{}
			}
		}
	}
}

func (f *folder) hReifies(payload interface{}, index int) {
	entries, ok := payload.(map[interface{}]interface{})
	if !ok {
		return
	}
	for k, spo := range entries {
		rid, ok := asInt64(k)
		if !ok {
			continue
		}
		items, ok := spo.([]interface{})
		if !ok || len(items) != 3 {
			continue
		}
		s, sOk := asIdx(items[0])
		p, pOk := asIdx(items[1])
		o, oOk := asIdx(items[2])
		n := len(f.g.Terms)
		ridOk := rid >= 0 && int(rid) < n
		spoOk := sOk && pOk && oOk && s < n && p < n && o < n
		if !ridOk || !spoOk {
			f.diag("DamagedFrame", fmt.Sprintf("reifier %d has bad/out-of-range ids", rid), &index)
			continue
		}
		irid := int(rid)
		spo := model.Triple3{S: s, P: p, O: o}
		if existing, ok := f.g.Reifier(irid); ok {
			if existing != spo {
				f.diag("ConflictingReifier", fmt.Sprintf("reifier %d rebound", irid), &index)
				continue
			}
		}
		f.g.SetReifier(irid, spo)
		f.emit(StreamingEvent{
			Kind:       StreamingEventReifier,
			FrameIndex: index,
			ReifierID:  irid,
			Triple:     spo,
		})
		if f.eventErr != nil {
			return
		}
	}
}

func (f *folder) hAnnot(payload interface{}, index int) {
	rows, ok := payload.([]interface{})
	if !ok {
		return
	}
	for _, row := range rows {
		items, ok := row.([]interface{})
		if !ok || len(items) != 3 {
			continue
		}
		r, rOk := asIdx(items[0])
		p, pOk := asIdx(items[1])
		v, vOk := asIdx(items[2])
		n := len(f.g.Terms)
		if !rOk || !pOk || !vOk || r >= n || p >= n || v >= n {
			f.diag("DamagedFrame", "annot row has bad/out-of-range ids", &index)
			continue
		}
		if f.g.Terms[p].Kind != model.Iri {
			f.diag("PositionConstraint", fmt.Sprintf("annot predicate %d not an IRI", p), &index)
			continue
		}
		annotation := model.Triple3{S: r, P: p, O: v}
		if f.materialize {
			f.g.Annotations = append(f.g.Annotations, annotation)
		}
		f.emit(StreamingEvent{
			Kind:       StreamingEventAnnotation,
			FrameIndex: index,
			Annotation: annotation,
		})
		if f.eventErr != nil {
			return
		}
	}
}

func (f *folder) hBlob(payload interface{}, frame map[interface{}]interface{}, index int) {
	if b, ok := payload.([]byte); ok {
		digest := wire.DigestStr(b)
		var meta interface{}
		if pub, ok := wire.MapGet(frame, "pub"); ok {
			if _, ok := pub.(map[interface{}]interface{}); ok {
				meta = pub
				if f.materialize {
					f.g.SetBlobMeta(digest, pub)
				}
			}
		}
		if f.materialize {
			f.g.SetBlob(digest, b)
		}
		f.emit(StreamingEvent{
			Kind:       StreamingEventBlob,
			FrameIndex: index,
			BlobDigest: digest,
			BlobData:   b,
			BlobMeta:   meta,
		})
		if f.eventErr != nil {
			return
		}
		// Layout bookkeeping (§3.3): was this delivery presaged by a
		// stream:digest description in an earlier frame?
		_, described := f.described[digest]
		f.blobEvents = append(f.blobEvents, blobEvent{pos: index, digest: digest, described: described})
	}
	// else: external blob — bytes live elsewhere, referenced by "pub".digest (§12).
}

func (f *folder) hMeta(payload interface{}) {
	entries, ok := payload.(map[interface{}]interface{})
	if !ok {
		return
	}
	for k, v := range entries {
		key := fmt.Sprintf("%v", k)
		if s, ok := asText(k); ok {
			key = s
		}
		f.g.SetMeta(key, v)
	}
}

func (f *folder) hSuppress(payload interface{}, index int) {
	entries, ok := payload.(map[interface{}]interface{})
	if !ok {
		return
	}
	targetsRaw, ok := wire.MapGet(entries, "targets")
	if !ok {
		return
	}
	targets, ok := targetsRaw.([]interface{})
	if !ok {
		return
	}
	var filtered []interface{}
	for _, t := range targets {
		if _, ok := t.(map[interface{}]interface{}); ok {
			filtered = append(filtered, t)
		}
	}
	s := model.Suppression{Targets: filtered}
	if reason, ok := wire.MapGet(entries, "reason"); ok {
		s.Reason = textOr(reason, "")
	}
	if by, ok := wire.MapGet(entries, "by"); ok {
		if b, ok := asIdx(by); ok {
			s.By = &b
		}
	}
	if f.materialize {
		f.g.Suppressions = append(f.g.Suppressions, s)
	}
	f.emit(StreamingEvent{
		Kind:        StreamingEventSuppression,
		FrameIndex:  index,
		Suppression: s,
	})
}

func (f *folder) hSnapshot(payload interface{}, index int) {
	entries, ok := payload.(map[interface{}]interface{})
	if !ok {
		return
	}
	base := len(f.g.Terms)
	shift := func(v interface{}) interface{} {
		if n, ok := asIdx(v); ok {
			return uint64(n + base)
		}
		return v
	}
	shiftRow := func(row interface{}) interface{} {
		if items, ok := row.([]interface{}); ok {
			out := make([]interface{}, len(items))
			for i, it := range items {
				out[i] = shift(it)
			}
			return out
		}
		return row
	}

	if snapTerms, ok := wire.MapGet(entries, "terms"); ok {
		if terms, ok := snapTerms.([]interface{}); ok {
			shifted := make([]interface{}, len(terms))
			for i, raw := range terms {
				if termMap, ok := raw.(map[interface{}]interface{}); ok {
					newEntries := make(map[interface{}]interface{})
					for k, v := range termMap {
						newEntries[k] = v
						if s, ok := asText(k); ok && (s == "dt" || s == "rf") {
							newEntries[k] = shift(v)
						}
					}
					shifted[i] = newEntries
				} else {
					shifted[i] = raw
				}
			}
			f.hTerms(shifted, index)
		}
	}
	if quads, ok := wire.MapGet(entries, "quads"); ok {
		if q, ok := quads.([]interface{}); ok {
			shifted := make([]interface{}, len(q))
			for i, row := range q {
				shifted[i] = shiftRow(row)
			}
			f.hQuads(shifted, index)
		}
	}
	if reifies, ok := wire.MapGet(entries, "reifies"); ok {
		if r, ok := reifies.(map[interface{}]interface{}); ok {
			shifted := make(map[interface{}]interface{})
			for k, v := range r {
				shifted[shift(k)] = shiftRow(v)
			}
			f.hReifies(shifted, index)
		}
	}
	if annot, ok := wire.MapGet(entries, "annot"); ok {
		if a, ok := annot.([]interface{}); ok {
			shifted := make([]interface{}, len(a))
			for i, row := range a {
				shifted[i] = shiftRow(row)
			}
			f.hAnnot(shifted, index)
		}
	}
	if blobs, ok := wire.MapGet(entries, "blobs"); ok {
		if b, ok := blobs.(map[interface{}]interface{}); ok {
			for _, v := range b {
				if data, ok := v.([]byte); ok {
					digest := wire.DigestStr(data)
					if f.materialize {
						f.g.SetBlob(digest, data)
					}
					f.emit(StreamingEvent{
						Kind:       StreamingEventBlob,
						FrameIndex: index,
						BlobDigest: digest,
						BlobData:   data,
					})
					if f.eventErr != nil {
						return
					}
				}
			}
		}
	}
	if meta, ok := wire.MapGet(entries, "meta"); ok {
		if m, ok := meta.(map[interface{}]interface{}); ok {
			for k, v := range m {
				key := fmt.Sprintf("%v", k)
				if s, ok := asText(k); ok {
					key = s
				}
				f.g.SetMeta(key, v)
			}
		}
	}
}

func (f *folder) hOpaque(payload interface{}, index int) {
	entries, ok := payload.(map[interface{}]interface{})
	if !ok {
		return
	}
	var id []byte
	if v, ok := wire.MapGet(entries, "id"); ok {
		if b, ok := v.([]byte); ok {
			id = b
		}
	}
	f.pushOpaque(model.OpaqueNode{
		ID:        id,
		FrameType: textOr(entries["type"], "opaque"),
		Reason:    textOr(entries["reason"], "unknown-codec"),
		SigStat:   textOr(entries["sigstat"], "none"),
		PubMeta:   entries["pub"],
	}, index)
}

func (f *folder) checkPositions(s, p, o int, g *int, index int) bool {
	n := len(f.g.Terms)
	inBounds := s < n && p < n && o < n && (g == nil || *g < n)
	if !inBounds {
		f.diag("PositionConstraint", fmt.Sprintf("quad (%d,%d,%d,%s) has out-of-range term ids", s, p, o, fmtOpt(g)), &index)
		return false
	}
	ok := f.g.Terms[p].Kind == model.Iri
	if f.g.Terms[s].Kind == model.Literal {
		ok = false
	}
	if g != nil {
		kind := f.g.Terms[*g].Kind
		if kind == model.Literal || kind == model.Triple {
			ok = false
		}
	}
	if !ok {
		f.diag("PositionConstraint", fmt.Sprintf("quad (%d,%d,%d,%s) violates positions", s, p, o, fmtOpt(g)), &index)
	}
	return ok
}

func (f *folder) opaque(frame map[interface{}]interface{}, ftype, reason string, index int) {
	var id []byte
	if v, ok := frame["id"]; ok {
		if b, ok := v.([]byte); ok {
			id = b
		}
	}
	sigstat := "none"
	if _, ok := frame["sig"]; ok {
		sigstat = "unverified"
	}
	var recipients []interface{}
	if to, ok := frame["to"]; ok {
		if arr, ok := to.([]interface{}); ok {
			for _, it := range arr {
				if _, ok := it.(map[interface{}]interface{}); ok {
					recipients = append(recipients, it)
				}
			}
		}
	}
	f.pushOpaque(model.OpaqueNode{
		ID:         id,
		FrameType:  ftype,
		Reason:     reason,
		SigStat:    sigstat,
		PubMeta:    frame["pub"],
		Recipients: recipients,
	}, index)
}
