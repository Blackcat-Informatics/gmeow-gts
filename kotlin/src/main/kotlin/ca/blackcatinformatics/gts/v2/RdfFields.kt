// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import java.nio.charset.CharacterCodingException

internal fun rdfRequire(condition: Boolean, reason: String) {
    if (!condition) malformed(reason)
}

internal fun CborField.completeRdfItem(): CborField {
    rdfRequire(itemEnd(data, start, end) == end, "wrong_length")
    return this
}

internal fun <T> CborField.readColumn(read: (CborField) -> T): List<T> {
    val fields = list()
    val result = mutableListOf<T>()
    while (fields.hasNext()) result += read(fields.next())
    return result
}

internal fun CborField.uColumn(): List<ULong> = readColumn(CborField::uint)
internal fun CborField.body(): CborField = CborField(data, bytes(), end)

internal val CborField.size: Int get() = end - start

internal fun CborField.digest(): ByteString = ByteString(data.copyOfRange(bytes(32), end))

internal fun CborField.rdfText(): String = CborField(data, typed(3).end, end).decodeRdfText()

internal fun CborField.decodeRdfText(): String =
    try {
        data.decodeToString(start, end, throwOnInvalidSequence = true)
    } catch (_: CharacterCodingException) {
        malformed("ill_formed_term")
    }

internal fun CborField.positive(): ULong =
    uint().also { rdfRequire(it != 0uL, "out_of_range") }

internal fun CborField.code(maximum: Int): Int =
    uint().also { rdfRequire(it in 1uL..maximum.toULong(), "unregistered_value") }.toInt()

internal fun <T> List<T>.valueAt(id: ULong): T {
    if (id >= size.toULong()) malformed("out_of_range")
    return this[id.toInt()]
}

internal fun CborField.deltaColumn(minimum: ULong = 0uL): List<ULong> {
    var previous = 0uL
    return readColumn { item ->
        val major = item.head().major
        val magnitude = checkNotNull(item.typed(if (major == 1) 1 else 0).argument)
        previous =
            if (major == 1) {
                rdfRequire(magnitude < previous, "out_of_range")
                previous - magnitude - 1uL
            } else {
                rdfRequire(magnitude <= ULong.MAX_VALUE - previous, "out_of_range")
                previous + magnitude
            }
        rdfRequire(previous >= minimum, "out_of_range")
        previous
    }
}
