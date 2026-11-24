// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package nquads

import "strings"

// WellFormedLanguageTag checks the BCP 47 syntax without consulting the
// language registry. The caller retains the original spelling.
func WellFormedLanguageTag(tag string) bool {
	if len(tag) <= len("i-enochian") && allChars(tag, func(ch rune) bool { return asciiLetter(ch) || ch == '-' }) {
		switch strings.ToLower(tag) {
		case "en-gb-oed", "i-ami", "i-bnn", "i-default", "i-enochian", "i-hak", "i-klingon", "i-lux", "i-mingo",
			"i-navajo", "i-pwn", "i-tao", "i-tay", "i-tsu", "sgn-be-fr", "sgn-be-nl", "sgn-ch-de":
			return true
		}
	}
	p := languageSubtags{rest: tag, pending: true}
	p.next()
	if p.privateUse() {
		return p.finishPrivateUse()
	}
	primary := len(p.current)
	if primary < 2 || primary > 8 || !allChars(p.current, asciiLetter) {
		return false
	}
	p.next()
	if primary <= 3 {
		for range 3 {
			if len(p.current) != 3 || !allChars(p.current, asciiLetter) {
				break
			}
			p.next()
		}
	}
	if len(p.current) == 4 && allChars(p.current, asciiLetter) {
		p.next()
	}
	if len(p.current) == 2 && allChars(p.current, asciiLetter) ||
		len(p.current) == 3 && allChars(p.current, asciiDigit) {
		p.next()
	}
	for p.variant() {
		p.next()
	}
	for len(p.current) == 1 && !p.privateUse() && asciiAlphanumeric(rune(p.current[0])) {
		p.next()
		if !p.alphanumeric(2) {
			return false
		}
		for p.alphanumeric(2) {
			p.next()
		}
	}
	if p.privateUse() {
		return p.finishPrivateUse()
	}
	return p.done
}

type languageSubtags struct {
	current, rest string
	pending, done bool
}

func (p *languageSubtags) next() {
	if p.pending {
		p.current, p.rest, p.pending = strings.Cut(p.rest, "-")
	} else {
		p.current, p.done = "", true
	}
}

func (p *languageSubtags) alphanumeric(minimum int) bool {
	return len(p.current) >= minimum && len(p.current) <= 8 && allChars(p.current, asciiAlphanumeric)
}

func (p *languageSubtags) variant() bool {
	return p.alphanumeric(5) || len(p.current) == 4 && asciiDigit(rune(p.current[0])) && allChars(p.current, asciiAlphanumeric)
}

func (p *languageSubtags) privateUse() bool { return p.current == "x" || p.current == "X" }

func (p *languageSubtags) finishPrivateUse() bool {
	p.next()
	if !p.alphanumeric(1) {
		return false
	}
	for p.alphanumeric(1) {
		p.next()
	}
	return p.done
}
