// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"fmt"

	"go.blackcatinformatics.ca/gts/wire"
)

// Package-local spellings of the wire coercions, so the fold reads as prose.

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
