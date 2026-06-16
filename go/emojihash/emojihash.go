// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package emojihash is a human-friendly emoji visual hash for keys and
// checksums: a BLAKE3-XOF digest sliced into 6-bit symbols indexing a fixed
// 64-emoji alphabet. Byte-compatible with the Python reference; gated by
// vectors/emojihash/*.json.
package emojihash

import (
	"io"
	"strings"

	"github.com/zeebo/blake3"
)

var emoji = [64]string{"🐵", "🐶", "🐺", "🦊", "🐱", "🦁", "🐯", "🐴", "🦄", "🦓", "🦌", "🐮", "🐷", "🐗", "🐭", "🐹", "🐰", "🐻", "🐼", "🐨", "🐸", "🐲", "🐔", "🐧", "🦆", "🦅", "🦉", "🦇", "🐢", "🐍", "🦎", "🐊", "🐳", "🐬", "🐟", "🐠", "🐡", "🦈", "🐙", "🦑", "🦀", "🦞", "🦐", "🦋", "🐌", "🐞", "🐝", "🐜", "🦂", "🍎", "🍐", "🍊", "🍋", "🍌", "🍉", "🍇", "🍓", "🍒", "🍍", "🥝", "🍑", "🥥", "🥕", "🌽"}

var labels = [64]string{"monkey", "dog", "wolf", "fox", "cat", "lion", "tiger", "horse", "unicorn", "zebra", "deer", "cow", "pig", "boar", "mouse", "hamster", "rabbit", "bear", "panda", "koala", "frog", "dragon", "chicken", "penguin", "duck", "eagle", "owl", "bat", "turtle", "snake", "lizard", "crocodile", "whale", "dolphin", "fish", "tropical-fish", "blowfish", "shark", "octopus", "squid", "crab", "lobster", "shrimp", "butterfly", "snail", "lady-beetle", "bee", "ant", "scorpion", "apple", "pear", "orange", "lemon", "banana", "watermelon", "grapes", "strawberry", "cherries", "pineapple", "kiwi", "peach", "coconut", "carrot", "corn"}

// Indices returns length 6-bit digest symbols (each in 0..64).
func Indices(data []byte, length int) []int {
	wanted := max(length, 1)
	nbytes := (wanted*6 + 7) / 8
	h := blake3.New()
	_, _ = h.Write(data)
	digest := make([]byte, nbytes)
	_, _ = io.ReadFull(h.Digest(), digest)

	out := make([]int, 0, wanted)
	var acc uint64
	var bits uint
	for _, b := range digest {
		acc = (acc << 8) | uint64(b)
		bits += 8
		for bits >= 6 && len(out) < wanted {
			bits -= 6
			out = append(out, int((acc>>bits)&0x3f))
		}
		acc &= (1 << bits) - 1
	}
	return out[:wanted]
}

// Emojihash maps data to a space-joined string of length emoji digits.
func Emojihash(data []byte, length int) string {
	idx := Indices(data, length)
	parts := make([]string, len(idx))
	for i, x := range idx {
		parts[i] = emoji[x]
	}
	return strings.Join(parts, " ")
}

// Labels returns the stable label names for Emojihash output.
func Labels(data []byte, length int) string {
	idx := Indices(data, length)
	parts := make([]string, len(idx))
	for i, x := range idx {
		parts[i] = labels[x]
	}
	return strings.Join(parts, " ")
}
