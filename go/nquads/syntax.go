// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package nquads

import (
	"net/netip"
	"strings"
	"unicode/utf8"
)

func asciiLetter(ch rune) bool { return ch >= 'A' && ch <= 'Z' || ch >= 'a' && ch <= 'z' }
func asciiDigit(ch rune) bool  { return ch >= '0' && ch <= '9' }
func asciiHex(ch rune) bool {
	return asciiDigit(ch) || ch >= 'A' && ch <= 'F' || ch >= 'a' && ch <= 'f'
}

func asciiAlphanumeric(ch rune) bool { return asciiLetter(ch) || asciiDigit(ch) }

func allChars(text string, allowed func(rune) bool) bool {
	for _, ch := range text {
		if !allowed(ch) {
			return false
		}
	}
	return true
}

// ValidIRI reports whether text has the RFC 3987 generic IRI syntax with a
// scheme. It checks the original spelling and allows a fragment.
func ValidIRI(text string) bool {
	scheme, remainder, ok := strings.Cut(text, ":")
	if !ok || scheme == "" || !asciiLetter(rune(scheme[0])) || !allChars(scheme[1:], func(ch rune) bool {
		return asciiAlphanumeric(ch) || strings.ContainsRune("+-.", ch)
	}) {
		return false
	}
	pathQuery, fragment, _ := strings.Cut(remainder, "#")
	path, query, _ := strings.Cut(pathQuery, "?")
	if authorityAndPath, found := strings.CutPrefix(path, "//"); found {
		authority, rest, _ := strings.Cut(authorityAndPath, "/")
		if !validAuthority(authority) {
			return false
		}
		path = rest
	}
	return validIRIComponent(path, ":@/", false) && validIRIComponent(query, ":@/?", true) &&
		validIRIComponent(fragment, ":@/?", false)
}

func validAuthority(text string) bool {
	hostPort := text
	if user, rest, ok := strings.Cut(text, "@"); ok {
		if !validIRIComponent(user, ":", false) {
			return false
		}
		hostPort = rest
	}
	if literal, ok := strings.CutPrefix(hostPort, "["); ok {
		host, port, closing := strings.Cut(literal, "]")
		if !closing || !validIPLiteral(host) {
			return false
		}
		if port == "" {
			return true
		}
		digits, colon := strings.CutPrefix(port, ":")
		return colon && allChars(digits, asciiDigit)
	}
	host, port, _ := strings.Cut(hostPort, ":")
	return validIRIComponent(host, "", false) && allChars(port, asciiDigit)
}

func validIPLiteral(text string) bool {
	if address, err := netip.ParseAddr(text); err == nil {
		return address.Is6() && address.Zone() == ""
	}
	if text == "" || text[0] != 'v' && text[0] != 'V' {
		return false
	}
	version, address, dot := strings.Cut(text[1:], ".")
	return dot && version != "" && allChars(version, asciiHex) && address != "" && allChars(address, func(ch rune) bool {
		return asciiIRIChar(ch) || ch == ':'
	})
}

func asciiIRIChar(ch rune) bool {
	return asciiAlphanumeric(ch) || strings.ContainsRune("-._~!$&'()*+,;=", ch)
}

func unicodeIRIChar(ch rune) bool {
	return ch >= 0xa0 && ch <= 0xd7ff || ch >= 0xf900 && ch <= 0xfdcf || ch >= 0xfdf0 && ch <= 0xffef ||
		ch >= 0x10000 && ch <= 0xdfffd && ch&0xffff <= 0xfffd || ch >= 0xe1000 && ch <= 0xefffd
}

func privateIRIChar(ch rune) bool {
	return ch >= 0xe000 && ch <= 0xf8ff || ch >= 0xf0000 && ch <= 0xffffd || ch >= 0x100000 && ch <= 0x10fffd
}

func validIRIComponent(text, extra string, private bool) bool {
	for text != "" {
		if text[0] == '%' {
			if len(text) < 3 || !asciiHex(rune(text[1])) || !asciiHex(rune(text[2])) {
				return false
			}
			text = text[3:]
			continue
		}
		ch, width := utf8.DecodeRuneInString(text)
		allowed := asciiIRIChar(ch) || strings.ContainsRune(extra, ch) || unicodeIRIChar(ch) || private && privateIRIChar(ch)
		if !allowed || ch == utf8.RuneError && width == 1 {
			return false
		}
		text = text[width:]
	}
	return true
}
