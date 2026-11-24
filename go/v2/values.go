// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import "unicode/utf8"

// itemValues traverses an already delimited definite array.
type itemValues struct {
	remaining []byte
}

func (v *itemValues) next() []byte {
	end, _ := itemEnd(v.remaining, 0)
	item := v.remaining[:end]
	v.remaining = v.remaining[end:]
	return item
}

func typedHead(item []byte, major byte) (head, Reason) {
	h, _ := readHead(item, 0)
	if h.major <= 4 && !h.shortest {
		return h, NotDeterministic
	}
	if h.major != major {
		return h, WrongType
	}
	return h, ""
}

func array(item []byte, length uint64) (itemValues, Reason) {
	h, reason := typedHead(item, 4)
	if reason == "" && h.value != length {
		reason = WrongLength
	}
	return itemValues{remaining: item[h.end:]}, reason
}

func uintValue(item []byte) (uint64, Reason) {
	h, reason := typedHead(item, 0)
	return h.value, reason
}

func boolValue(item []byte) (bool, Reason) {
	h, reason := typedHead(item, 7)
	if reason == "" && item[0] != 0xf4 && item[0] != 0xf5 {
		reason = WrongType
	}
	return h.value == 21 && reason == "", reason
}

func byteValue(item []byte, length int) ([]byte, Reason) {
	h, reason := typedHead(item, 2)
	if reason != "" {
		return nil, reason
	}
	body := item[h.end:]
	if length >= 0 && len(body) != length {
		return nil, WrongLength
	}
	return body, ""
}

func option(item []byte) ([]byte, Reason) {
	return optionBefore(item, len(item))
}

// Only field starts are bounded. A head that starts before the limit is read
// in full, even when its argument extends beyond the limit.
func optionBefore(item []byte, limit int) ([]byte, Reason) {
	if limit <= 0 {
		return nil, OutOfRange
	}
	h, reason := typedHead(item, 4)
	if reason != "" {
		return nil, reason
	}
	if h.value == 0 {
		return nil, WrongLength
	}
	if h.end >= limit {
		return nil, OutOfRange
	}
	v := itemValues{remaining: item[h.end:]}
	tag, reason := uintValue(v.next())
	switch {
	case reason != "":
		return nil, reason
	case tag != 1 && tag != 2:
		return nil, UnregisteredValue
	case h.value != tag:
		return nil, WrongLength
	case tag == 1:
		return nil, ""
	case len(item)-len(v.remaining) >= limit:
		return nil, OutOfRange
	default:
		return v.next(), ""
	}
}

func optionEnum(item []byte, max uint64, newer bool) (uint64, Reason) {
	value, reason := option(item)
	if value == nil || reason != "" {
		return 0, reason
	}
	code, reason := uintValue(value)
	if reason == "" && (code == 0 || (!newer && code > max)) {
		reason = UnregisteredValue
	}
	return code, reason
}

// grammar checks every head in an already delimited item. Byte and text bodies
// can be skipped. An array contributes its children in byte order.
func grammar(item []byte) Reason {
	return grammarBefore(item, len(item))
}

func grammarBefore(item []byte, limit int) Reason {
	for at := 0; at < len(item); {
		if at >= limit {
			return OutOfRange
		}
		h, _ := readHead(item, at)
		if h.major <= 4 && !h.shortest {
			return NotDeterministic
		}
		if h.major == 5 || h.major == 6 || (h.major == 7 && item[at] != 0xf4 && item[at] != 0xf5) {
			return WrongType
		}
		at = h.end
		if h.major == 2 || h.major == 3 {
			end := at + int(h.value)
			if h.major == 3 && !utf8.Valid(item[at:end]) {
				return WrongType
			}
			at = end
		}
	}
	return ""
}

type valueCheck func([]byte) Reason

func checkUint(item []byte) Reason {
	_, reason := uintValue(item)
	return reason
}

func checkDigest(item []byte) Reason {
	_, reason := byteValue(item, 32)
	return reason
}

func checkTextOption(item []byte) Reason {
	h, reason := typedHead(item, 4)
	if reason != "" {
		return reason
	}
	if h.value == 0 {
		return WrongLength
	}
	values := itemValues{remaining: item[h.end:]}
	tag, reason := uintValue(values.next())
	if reason != "" {
		return reason
	}
	switch {
	case (tag == 1 || tag == 2) && tag != h.value:
		return WrongLength
	case tag == 2:
		if _, reason = typedHead(values.remaining, 3); reason != "" {
			return reason
		}
	}
	return grammar(item)
}

func checkRecord(item []byte, fields ...valueCheck) Reason {
	values, reason := array(item, uint64(len(fields)))
	if reason != "" {
		return reason
	}
	for _, field := range fields {
		if reason := field(values.next()); reason != "" {
			return reason
		}
	}
	return ""
}
