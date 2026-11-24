// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import java.util.IllformedLocaleException
import java.util.Locale

private val iriParts = Regex("""[A-Za-z][A-Za-z0-9+.-]*:([^?#]*)(?:\?([^#]*))?(?:#([\s\S]*))?""")
private val futureAddress = Regex("""[vV][0-9a-fA-F]+\.(.+)""")
private const val ASCII_PUNCTUATION = "._~!$&'()*+,;=-"

internal fun validRdfIri(text: String): Boolean {
    val match = iriParts.matchEntire(text) ?: return false
    var (path, query, fragment) = match.destructured
    if (path.startsWith("//")) {
        val end = path.indexOf('/', 2).takeIf { it >= 0 } ?: path.length
        if (!validAuthority(path.substring(2, end))) return false
        path = path.substring(end)
    }
    return validComponent(path, ":@/") && validComponent(query, ":@/?", allowPrivate = true) && validComponent(fragment, ":@/?")
}

private fun validAuthority(authority: String): Boolean {
    val at = authority.indexOf('@')
    if (at >= 0 && !validComponent(authority.substring(0, at), ":")) return false
    val hostPort = authority.substring(at + 1)
    if (hostPort.startsWith("[")) {
        val end = hostPort.indexOf(']')
        if (end < 0) return false
        val address = hostPort.substring(1, end)
        val future = futureAddress.matchEntire(address)?.groupValues?.get(1)
        val hostValid = if (future != null) future.all { asciiIri(it.code) || it == ':' } else validIpv6(address)
        val port = hostPort.substring(end + 1)
        return hostValid && (port.isEmpty() || (port.startsWith(':') && port.drop(1).all { it in '0'..'9' }))
    }
    val colon = hostPort.indexOf(':')
    if (colon < 0) return validComponent(hostPort, "")
    return validComponent(hostPort.substring(0, colon), "") && hostPort.drop(colon + 1).all { it in '0'..'9' }
}

private fun validIpv6(address: String): Boolean {
    val halves = address.split("::")
    if (halves.size > 2) return false
    val groups = halves.flatMap { if (it.isEmpty()) emptyList() else it.split(':') }
    var width = 0
    for ((index, field) in groups.withIndex()) {
        if ('.' in field && index == groups.lastIndex && address.endsWith(field)) {
            val octets = field.split('.')
            if (octets.size != 4 || octets.any { it.isEmpty() || it.length > 3 ||
                    (it.length > 1 && it.startsWith('0')) || it.any { ch -> ch !in '0'..'9' } ||
                    it.toInt() > 255
                }
            ) {
                return false
            }
            width += 2
        } else {
            if (field.length !in 1..4 || !field.all(::hexDigit)) return false
            width++
        }
    }
    return if (halves.size == 2) width < 8 else width == 8
}

private fun validComponent(text: String, allowed: String, allowPrivate: Boolean = false): Boolean {
    var offset = 0
    while (offset < text.length) {
        val point = text.codePointAt(offset)
        if (point == '%'.code) {
            if (offset + 2 >= text.length || !hexDigit(text[offset + 1]) || !hexDigit(text[offset + 2])) return false
            offset += 3
            continue
        }
        val unicode = point in 0xa0..0xd7ff || point in 0xf900..0xfdcf || point in 0xfdf0..0xffef ||
            (point in 0x10000..0xdfffd && (point and 0xffff) <= 0xfffd) || point in 0xe1000..0xefffd
        val privateUse = allowPrivate && (point in 0xe000..0xf8ff || point in 0xf0000..0xffffd || point in 0x100000..0x10fffd)
        if (!unicode && !privateUse && !asciiIri(point) && (point > 127 || point.toChar() !in allowed)) return false
        offset += Character.charCount(point)
    }
    return true
}

private fun asciiIri(point: Int): Boolean =
    point in 'A'.code..'Z'.code || point in 'a'.code..'z'.code || point in '0'.code..'9'.code ||
        (point <= 127 && point.toChar() in ASCII_PUNCTUATION)

private fun hexDigit(ch: Char): Boolean = ch in '0'..'9' || ch in 'a'..'f' || ch in 'A'..'F'

internal fun validRdfLanguage(text: String): Boolean =
    try {
        Locale.Builder().setLanguageTag(text)
        text.isNotEmpty()
    } catch (_: IllformedLocaleException) {
        false
    }

internal fun rdfBlankEnd(line: String, start: Int): Int {
    var offset = start
    var end = start
    while (offset < line.length) {
        val point = line.codePointAt(offset)
        if (!blankStart(point) && (offset == start || !blankExtra(point))) break
        offset += Character.charCount(point)
        if (point != '.'.code) end = offset
    }
    return end
}

private fun blankStart(point: Int): Boolean =
    point in 'A'.code..'Z'.code || point in 'a'.code..'z'.code || point in '0'.code..'9'.code || point == '_'.code ||
        point in 0xc0..0xd6 || point in 0xd8..0xf6 || point in 0xf8..0x2ff || point in 0x370..0x37d ||
        point in 0x37f..0x1fff || point in 0x200c..0x200d || point in 0x2070..0x218f || point in 0x2c00..0x2fef ||
        point in 0x3001..0xd7ff || point in 0xf900..0xfdcf || point in 0xfdf0..0xfffd || point in 0x10000..0xeffff

private fun blankExtra(point: Int): Boolean =
    point == '-'.code || point == '.'.code || point == 0xb7 || point in 0x300..0x36f || point in 0x203f..0x2040
