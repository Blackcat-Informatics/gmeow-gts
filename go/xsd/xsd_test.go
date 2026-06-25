// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package xsd

import (
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
)

func xsdIRI(local string) string {
	return Namespace + local
}

func TestValidateLexicalRecognizedValidExamples(t *testing.T) {
	tests := []struct {
		name      string
		datatype  string
		lexical   string
		canonical string
	}{
		{"boolean true", "boolean", "1", "true"},
		{"boolean false", "boolean", "false", "false"},
		{"integer", "integer", "+00042", "42"},
		{"integer family", "unsignedByte", "255", "255"},
		{"decimal", "decimal", "-001.2300", "-1.23"},
		{"float", "float", "1.25E2", "125"},
		{"double", "double", "-INF", "-INF"},
		{"dateTime", "dateTime", "2026-06-10T20:00:00Z", "2026-06-10T20:00:00Z"},
		{"dateTime midnight", "dateTime", "2026-06-10T24:00:00.000Z", "2026-06-10T24:00:00.000Z"},
		{"date", "date", "2024-02-29", "2024-02-29"},
		{"time", "time", "23:59:59-07:00", "23:59:59-07:00"},
		{"time midnight", "time", "24:00:00", "24:00:00"},
		{"duration", "duration", "P1Y2M3DT4H5M6.7S", "P1Y2M3DT4H5M6.7S"},
		{"yearMonthDuration", "yearMonthDuration", "P1Y2M", "P1Y2M"},
		{"dayTimeDuration", "dayTimeDuration", "P3DT4H", "P3DT4H"},
		{"hexBinary", "hexBinary", "0a1B", "0A1B"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			status := ValidateLexical(xsdIRI(tc.datatype), tc.lexical)
			if !status.IsValid() {
				t.Fatalf("status = %#v, want valid", status)
			}
			if status.Canonical != tc.canonical {
				t.Fatalf("canonical = %q, want %q", status.Canonical, tc.canonical)
			}
		})
	}
}

func TestValidateLexicalRecognizedInvalidExamples(t *testing.T) {
	tests := []struct {
		name     string
		datatype string
		lexical  string
	}{
		{"boolean", "boolean", "maybe"},
		{"integer", "integer", "12.0"},
		{"integer family", "unsignedByte", "256"},
		{"decimal", "decimal", "1e2"},
		{"float", "float", "1e"},
		{"double", "double", "++1"},
		{"dateTime", "dateTime", "2026-02-31T00:00:00Z"},
		{"dateTime fractional midnight", "dateTime", "2026-06-10T24:00:00.001Z"},
		{"date", "date", "2026-02-31"},
		{"time", "time", "25:00:00"},
		{"time fractional midnight", "time", "24:00:00.001"},
		{"duration", "duration", "P"},
		{"yearMonthDuration", "yearMonthDuration", "P1D"},
		{"dayTimeDuration", "dayTimeDuration", "P1Y"},
		{"hexBinary odd", "hexBinary", "0A1"},
		{"hexBinary nonhex", "hexBinary", "0X"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			status := ValidateLexical(xsdIRI(tc.datatype), tc.lexical)
			if !status.IsInvalid() {
				t.Fatalf("status = %#v, want invalid", status)
			}
			if status.Reason == "" {
				t.Fatal("invalid status did not include a reason")
			}
		})
	}
}

func TestTokenWhitespaceCollapseOnlyUsesXMLWhitespace(t *testing.T) {
	status := ValidateLexical(xsdIRI("token"), " a\tb\u00a0 c  ")
	if !status.IsValid() {
		t.Fatalf("status = %#v, want valid", status)
	}
	if status.Canonical != "a b\u00a0 c" {
		t.Fatalf("canonical = %q, want XML-only whitespace collapse", status.Canonical)
	}
}

func TestValidateLexicalLeavesUnsupportedDatatypesAlone(t *testing.T) {
	status := ValidateLexical("https://example.org/customDatatype", "not our syntax")
	if !status.IsUnsupported() {
		t.Fatalf("status = %#v, want unsupported", status)
	}
}

func TestAnnotateIllTypedLiteralsAddsDiagnosticsAndMetadata(t *testing.T) {
	dt := 0
	custom := 3
	graph := &model.Graph{Terms: []model.Term{
		{Kind: model.Iri, Value: xsdIRI("boolean")},
		{Kind: model.Literal, Value: "maybe", Datatype: &dt},
		{Kind: model.Literal, Value: "true", Datatype: &dt},
		{Kind: model.Iri, Value: "https://example.org/customDatatype"},
		{Kind: model.Literal, Value: "not our syntax", Datatype: &custom},
	}}

	items := IllTypedLiterals(graph)
	if len(items) != 1 {
		t.Fatalf("items = %#v, want one ill-typed literal", items)
	}
	if items[0].TermID != 1 || items[0].Lexical != "maybe" || items[0].DatatypeIRI != xsdIRI("boolean") {
		t.Fatalf("unexpected item: %#v", items[0])
	}

	AnnotateIllTypedLiterals(graph)
	if len(graph.Diagnostics) != 1 {
		t.Fatalf("diagnostics = %#v, want one", graph.Diagnostics)
	}
	if graph.Diagnostics[0].Code != IllTypedLiteralCode {
		t.Fatalf("diagnostic code = %q, want %q", graph.Diagnostics[0].Code, IllTypedLiteralCode)
	}
	if !strings.Contains(graph.Diagnostics[0].Detail, "maybe") {
		t.Fatalf("diagnostic detail did not preserve lexical form: %q", graph.Diagnostics[0].Detail)
	}
	if len(graph.Meta) != 1 || graph.Meta[0].Key != IllTypedLiteralMetaKey {
		t.Fatalf("metadata = %#v, want %s sidecar", graph.Meta, IllTypedLiteralMetaKey)
	}
	sidecar, ok := graph.Meta[0].Value.(map[interface{}]interface{})
	if !ok {
		t.Fatalf("sidecar type = %T, want map", graph.Meta[0].Value)
	}
	if sidecar["version"] != int64(1) {
		t.Fatalf("version = %#v, want int64(1)", sidecar["version"])
	}
	rows, ok := sidecar["items"].([]interface{})
	if !ok || len(rows) != 1 {
		t.Fatalf("items = %#v, want one row", sidecar["items"])
	}
	row, ok := rows[0].(map[interface{}]interface{})
	if !ok {
		t.Fatalf("row type = %T, want map", rows[0])
	}
	if row["term"] != int64(1) || row["datatype"] != xsdIRI("boolean") || row["lexical"] != "maybe" {
		t.Fatalf("unexpected sidecar row: %#v", row)
	}
}
