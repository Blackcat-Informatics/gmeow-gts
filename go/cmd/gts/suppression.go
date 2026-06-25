// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"strings"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

// suppressedBlobDigests returns the set of blob digests targeted by suppressions.
func suppressedBlobDigests(g *model.Graph) map[string]struct{} {
	out := make(map[string]struct{})
	for _, sup := range g.Suppressions {
		for _, target := range sup.Targets {
			m, ok := target.(map[interface{}]interface{})
			if !ok {
				continue
			}
			kind := ""
			var digest string
			haveDigest := false
			for k, v := range m {
				switch wire.TextOr(k, "") {
				case "kind":
					kind = wire.TextOr(v, "")
				case "digest":
					if d := digestFromValue(v); d != "" {
						digest = d
						haveDigest = true
					}
				}
			}
			if kind == "blob" && haveDigest {
				out[digest] = struct{}{}
			}
		}
	}
	return out
}

// digestFromValue coerces a decoded CBOR value to a normalised blake3 digest.
func digestFromValue(v interface{}) string {
	if s, ok := v.(string); ok {
		return normalizeDigest(s)
	}
	if b, ok := v.([]byte); ok {
		return "blake3:" + wire.Hex(b)
	}
	return ""
}

// targetKind returns the "kind" field of a suppression target map.
func targetKind(target interface{}) string {
	m, ok := target.(map[interface{}]interface{})
	if !ok {
		return ""
	}
	if v, ok := wire.MapGet(m, "kind"); ok {
		return wire.TextOr(v, "")
	}
	return ""
}

// targetIdx returns the "id" field of a suppression target map as an int.
func targetIdx(target interface{}) (int, bool) {
	m, ok := target.(map[interface{}]interface{})
	if !ok {
		return 0, false
	}
	if v, ok := wire.MapGet(m, "id"); ok {
		return wire.AsInt(v)
	}
	return 0, false
}

// allQuadsSuppressed reports whether every quad is hidden by a suppression.
func allQuadsSuppressed(g *model.Graph) bool {
	if len(g.Quads) == 0 || len(g.Suppressions) == 0 {
		return false
	}
	termSup := make(map[int]struct{})
	quadSup := make(map[string]struct{})
	for _, sup := range g.Suppressions {
		collectSuppressed(sup, termSup, quadSup)
	}
	for _, q := range g.Quads {
		key := quadKey(q)
		if _, ok := quadSup[key]; ok {
			continue
		}
		if _, ok := termSup[q.S]; ok {
			continue
		}
		if _, ok := termSup[q.P]; ok {
			continue
		}
		if _, ok := termSup[q.O]; ok {
			continue
		}
		if q.G != nil {
			if _, ok := termSup[*q.G]; ok {
				continue
			}
		}
		return false
	}
	return true
}

// quadKey returns a stable string key for a quad (including graph if present).
func quadKey(q model.Quad) string {
	if q.G != nil {
		return fmt.Sprintf("%d,%d,%d,%d", q.S, q.P, q.O, *q.G)
	}
	return fmt.Sprintf("%d,%d,%d", q.S, q.P, q.O)
}

// collectSuppressed expands a suppression into term and quad key sets.
func collectSuppressed(sup model.Suppression, termSup map[int]struct{}, quadSup map[string]struct{}) {
	for _, target := range sup.Targets {
		switch targetKind(target) {
		case "term", "reifier":
			if id, ok := targetIdx(target); ok {
				termSup[id] = struct{}{}
			}
		case "quad":
			m, ok := target.(map[interface{}]interface{})
			if !ok {
				continue
			}
			if v, ok := wire.MapGet(m, "q"); ok {
				if ids, ok := v.([]interface{}); ok {
					parts := make([]string, len(ids))
					valid := true
					for i, x := range ids {
						n, ok := wire.AsInt64(x)
						if !ok {
							valid = false
							break
						}
						parts[i] = fmt.Sprintf("%d", n)
					}
					if valid {
						quadSup[strings.Join(parts, ",")] = struct{}{}
					}
				}
			}
		}
	}
}
