// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import java.time.LocalDateTime
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.Locale

private const val XSD = "http://www.w3.org/2001/XMLSchema#"
internal const val XSD_HEX_BINARY = XSD + "hexBinary"
private const val HEX_DIGITS = "0123456789ABCDEF"
private val civilTime = DateTimeFormatter.ofPattern("uuuu-MM-dd'T'HH:mm:ss", Locale.ROOT)

internal sealed interface RdfTypedValue {
    val datatype: String
    fun lexical(budget: RdfTermBudget): String

    data class Scalar(
        val kind: Int,
        val argument: ULong,
        val negative: Boolean = false,
        val nanos: ULong = 0uL,
    ) : RdfTypedValue {
        override val datatype: String
            get() = XSD + when (kind) {
                1 -> "boolean"
                2 -> "nonNegativeInteger"
                3 -> "integer"
                else -> "dateTime"
            }

        override fun lexical(budget: RdfTermBudget): String {
            val text = when {
                kind == 1 -> (argument != 0uL).toString()
                kind == 4 -> formatTime(argument, negative, nanos)
                !negative -> argument.toString()
                argument == ULong.MAX_VALUE -> "-18446744073709551616"
                else -> "-${argument + 1uL}"
            }
            budget.admitText(text.length)
            return text
        }
    }

    data class Octets(val body: CborField) : RdfTypedValue {
        override val datatype = XSD_HEX_BINARY

        override fun lexical(budget: RdfTermBudget): String {
            budget.admitText(body.size * 2)
            return buildString(body.size * 2) { appendHex(body) }
        }
    }
}

internal fun readRdfTypedValue(item: CborField): RdfTypedValue {
    val tag = item.list()
    rdfRequire(tag.hasNext(), "wrong_length")
    val kind = tag.next().code(5)
    val fields = item.fields(if (kind == 4) 3 else 2)
    fields.next()
    val value = fields.next()
    return when (kind) {
        1 -> RdfTypedValue.Scalar(kind, if (value.flag()) 1uL else 0uL)
        2 -> RdfTypedValue.Scalar(kind, value.uint())
        3, 4 -> {
            val head = value.typed(if (value.head().major == 1) 1 else 0)
            val argument = checkNotNull(head.argument)
            val negative = head.major == 1
            val nanos = if (kind == 4) readTimeFraction(argument, negative, fields.next()) else 0uL
            RdfTypedValue.Scalar(kind, argument, negative, nanos)
        }
        else -> RdfTypedValue.Octets(value.body())
    }
}

private fun readTimeFraction(argument: ULong, negative: Boolean, fraction: CborField): ULong {
    rdfRequire(argument <= if (negative) 62135596799uL else 253402300799uL, "out_of_range")
    return fraction.uint().also { rdfRequire(it < 1000000000uL, "out_of_range") }
}

private fun formatTime(magnitude: ULong, negative: Boolean, nanos: ULong): String {
    val epoch = if (negative) -1 - magnitude.toLong() else magnitude.toLong()
    val civil = civilTime.format(LocalDateTime.ofEpochSecond(epoch, 0, ZoneOffset.UTC))
    val fraction = if (nanos == 0uL) "" else "." + nanos.toString().padStart(9, '0').trimEnd('0')
    return civil + fraction + "Z"
}

internal fun StringBuilder.appendHex(body: CborField) {
    for (offset in body.start until body.end) {
        val octet = body.data.octet(offset)
        append(HEX_DIGITS[octet ushr 4]).append(HEX_DIGITS[octet and 15])
    }
}
