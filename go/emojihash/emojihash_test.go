// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package emojihash

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestEmojihashVectors(t *testing.T) {
	dir := filepath.Join("..", "..", "vectors", "emojihash")
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("vectors/emojihash must exist: %v", err)
	}
	count := 0
	for _, e := range entries {
		if filepath.Ext(e.Name()) != ".json" {
			continue
		}
		raw, err := os.ReadFile(filepath.Join(dir, e.Name()))
		if err != nil {
			t.Fatal(err)
		}
		var c struct {
			Data    string `json:"data"`
			Length  int    `json:"length"`
			Indices []int  `json:"indices"`
			Emoji   string `json:"emoji"`
			Labels  string `json:"labels"`
		}
		if err := json.Unmarshal(raw, &c); err != nil {
			t.Fatal(err)
		}
		data, _ := hex.DecodeString(c.Data)
		if got := Indices(data, c.Length); !reflect.DeepEqual(got, c.Indices) {
			t.Errorf("%s: indices %v != %v", e.Name(), got, c.Indices)
		}
		if got := Emojihash(data, c.Length); got != c.Emoji {
			t.Errorf("%s: emoji %q != %q", e.Name(), got, c.Emoji)
		}
		if got := Labels(data, c.Length); got != c.Labels {
			t.Errorf("%s: labels %q != %q", e.Name(), got, c.Labels)
		}
		count++
	}
	if count < 4 {
		t.Fatalf("expected emojihash vectors, found %d", count)
	}
}
