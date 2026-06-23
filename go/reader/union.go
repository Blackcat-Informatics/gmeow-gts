// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"fmt"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

type internKey struct {
	typ          byte // 0=iri, 1=lit, 2=bnode, 3=qt
	a            string
	b            string
	c            string
	d            string
	seg          int
	rf           *int
	bnodeTID     int // anonymous bnode source term id (typ==2, value empty)
	bnodeLabeled bool
}

type unioner struct {
	out    *model.Graph
	intern map[internKey]int
}

func newUnioner() *unioner {
	return &unioner{
		out:    emptyGraph(),
		intern: make(map[internKey]int),
	}
}

func (u *unioner) keyFor(seg *model.Graph, segIdx, tid int) internKey {
	t := &seg.Terms[tid]
	switch t.Kind {
	case model.Iri:
		return internKey{typ: 0, a: t.Value}
	case model.Literal:
		return internKey{typ: 1, a: t.Value, b: seg.DatatypeIRI(t), c: t.Lang, d: t.Direction}
	case model.Bnode:
		if t.Value != "" {
			return internKey{typ: 2, seg: segIdx, a: t.Value, bnodeLabeled: true}
		}
		return internKey{typ: 2, seg: segIdx, bnodeTID: tid}
	case model.Triple:
		var rf *int
		if t.Reifier != nil {
			r := u.mapTerm(seg, segIdx, *t.Reifier)
			rf = &r
		}
		return internKey{typ: 3, rf: rf}
	}
	return internKey{}
}

func (u *unioner) mapTerm(seg *model.Graph, segIdx, tid int) int {
	key := u.keyFor(seg, segIdx, tid)
	if got, ok := u.intern[key]; ok {
		return got
	}
	t := seg.Terms[tid]
	var datatype *int
	if t.Datatype != nil {
		d := u.mapTerm(seg, segIdx, *t.Datatype)
		datatype = &d
	}
	var reifier *int
	if t.Reifier != nil {
		r := u.mapTerm(seg, segIdx, *t.Reifier)
		reifier = &r
	}
	value := t.Value
	if t.Kind == model.Bnode {
		if value != "" {
			value = fmt.Sprintf("s%d.%s", segIdx, value)
		} else {
			value = fmt.Sprintf("s%d._anon%d", segIdx, len(u.out.Terms))
		}
	}
	u.out.Terms = append(u.out.Terms, model.Term{
		Kind:      t.Kind,
		Value:     value,
		Datatype:  datatype,
		Lang:      t.Lang,
		Direction: t.Direction,
		Reifier:   reifier,
	})
	newID := len(u.out.Terms) - 1
	u.intern[key] = newID
	return newID
}

func (u *unioner) remapSuppression(sup model.Suppression, seg *model.Graph, segIdx int) model.Suppression {
	n := len(seg.Terms)
	newTargets := make([]interface{}, len(sup.Targets))
	for i, target := range sup.Targets {
		m, ok := target.(map[interface{}]interface{})
		if !ok {
			newTargets[i] = target
			continue
		}
		kind := ""
		if v, ok := wire.MapGet(m, "kind"); ok {
			kind = textOr(v, "")
		}
		if kind == "frame" || kind == "blob" {
			newTargets[i] = target
			continue
		}
		newMap := make(map[interface{}]interface{})
		for k, v := range m {
			newMap[k] = v
			key := ""
			if s, ok := asText(k); ok {
				key = s
			}
			if (kind == "term" || kind == "reifier") && key == "id" {
				if tid, ok := asIdx(v); ok && tid < n {
					newMap[k] = uint64(u.mapTerm(seg, segIdx, tid))
				}
			} else if kind == "quad" && key == "q" {
				if ids, ok := v.([]interface{}); ok {
					remapped := make([]interface{}, len(ids))
					for j, x := range ids {
						if tid, ok := asIdx(x); ok && tid < n {
							remapped[j] = uint64(u.mapTerm(seg, segIdx, tid))
						} else {
							remapped[j] = x
						}
					}
					newMap[k] = remapped
				}
			}
		}
		newTargets[i] = newMap
	}
	newSup := model.Suppression{Targets: newTargets, Reason: sup.Reason}
	if sup.By != nil && *sup.By < n {
		by := u.mapTerm(seg, segIdx, *sup.By)
		newSup.By = &by
	}
	return newSup
}

func unionQuadKey(q model.Quad) string {
	if q.G != nil {
		return fmt.Sprintf("%d,%d,%d,%d", q.S, q.P, q.O, *q.G)
	}
	return fmt.Sprintf("%d,%d,%d", q.S, q.P, q.O)
}

func unionSegments(segments []*model.Graph) *model.Graph {
	u := newUnioner()
	seen := make(map[string]struct{})
	for segIdx, seg := range segments {
		for _, q := range seg.Quads {
			uq := model.Quad{
				S: u.mapTerm(seg, segIdx, q.S),
				P: u.mapTerm(seg, segIdx, q.P),
				O: u.mapTerm(seg, segIdx, q.O),
			}
			if q.G != nil {
				g := u.mapTerm(seg, segIdx, *q.G)
				uq.G = &g
			}
			key := unionQuadKey(uq)
			if _, ok := seen[key]; !ok {
				seen[key] = struct{}{}
				u.out.Quads = append(u.out.Quads, uq)
			}
		}
		for _, r := range seg.Reifiers {
			newRf := u.mapTerm(seg, segIdx, r.RID)
			spo := model.Triple3{
				S: u.mapTerm(seg, segIdx, r.SPO.S),
				P: u.mapTerm(seg, segIdx, r.SPO.P),
				O: u.mapTerm(seg, segIdx, r.SPO.O),
			}
			u.out.SetReifier(newRf, spo)
		}
		for _, a := range seg.Annotations {
			u.out.Annotations = append(u.out.Annotations, model.Triple3{
				S: u.mapTerm(seg, segIdx, a.S),
				P: u.mapTerm(seg, segIdx, a.P),
				O: u.mapTerm(seg, segIdx, a.O),
			})
		}
		for _, b := range seg.Blobs {
			u.out.SetBlob(b.Digest, b.Data)
		}
		for _, bm := range seg.BlobMeta {
			u.out.SetBlobMeta(bm.Digest, bm.Meta)
		}
		for _, m := range seg.Meta {
			u.out.SetMeta(m.Key, m.Value)
		}
		u.out.SegmentMeta = append(u.out.SegmentMeta, seg.SegmentMeta...)
		for _, sup := range seg.Suppressions {
			u.out.Suppressions = append(u.out.Suppressions, u.remapSuppression(sup, seg, segIdx))
		}
		u.out.Opaque = append(u.out.Opaque, seg.Opaque...)
		u.out.Signatures = append(u.out.Signatures, seg.Signatures...)
		u.out.Diagnostics = append(u.out.Diagnostics, seg.Diagnostics...)
		u.out.SegmentHeads = append(u.out.SegmentHeads, seg.SegmentHeads...)
		u.out.SegmentProfiles = append(u.out.SegmentProfiles, seg.SegmentProfiles...)
		u.out.SegmentStreamable = append(u.out.SegmentStreamable, seg.SegmentStreamable...)
	}
	return u.out
}
