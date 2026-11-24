// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import java.nio.ByteBuffer
import java.nio.CharBuffer
import java.nio.charset.StandardCharsets

internal class EnvelopeFailure(val reason: String) : RuntimeException(reason)

internal fun malformed(reason: String): Nothing = throw EnvelopeFailure(reason)

// All ranges have been delimited by the boundary walker.
internal class CborField(val data: ByteArray, val start: Int, val end: Int) {
    fun head(): CborHead = readHead(data, start, end)

    fun typed(major: Int): CborHead {
        val head = head()
        if (head.major <= 4 && !head.shortest) malformed("not_deterministic")
        if (head.major != major) malformed("wrong_type")
        return head
    }

    fun uint(): ULong = checkNotNull(typed(0).argument)

    fun bytes(length: Int? = null): Int {
        val head = typed(2)
        if (length != null && head.argument != length.toULong()) malformed("wrong_length")
        return head.end
    }

    fun flag(): Boolean {
        typed(7)
        return when (data.octet(start)) {
            0xf4 -> false
            0xf5 -> true
            else -> malformed("wrong_type")
        }
    }

    fun fields(size: Int, bound: Int = end): Fields {
        val array = typed(4)
        if (array.argument != size.toULong()) malformed("wrong_length")
        return Fields(data, array.end, end, bound)
    }

    fun list(): Fields = Fields(data, typed(4).end, end)

    fun option(bound: Int = end, visit: (CborField) -> Unit) {
        val array = typed(4)
        if (array.argument == 0uL) malformed("wrong_length")
        val fields = Fields(data, array.end, end, bound)
        val tag = fields.next().uint()
        if (tag != 1uL && tag != 2uL) malformed("unregistered_value")
        if (array.argument != tag) malformed("wrong_length")
        if (tag == 2uL) visit(fields.next())
    }

    fun compare(other: CborField): Int {
        val leftLength = end - start
        val rightLength = other.end - other.start
        for (index in 0 until minOf(leftLength, rightLength)) {
            val diff = data.octet(start + index) - other.data.octet(other.start + index)
            if (diff != 0) return diff
        }
        return leftLength.compareTo(rightLength)
    }

    fun grammar(bound: Int = end) {
        var offset = start
        while (offset < end) {
            if (offset >= bound) malformed("out_of_range")
            val head = readHead(data, offset, end)
            if (head.major <= 4 && !head.shortest) malformed("not_deterministic")
            if (head.major in 5..6 || (head.major == 7 && data.octet(offset) !in 0xf4..0xf5)) {
                malformed("wrong_type")
            }
            offset = head.end
            if (head.major in 2..3) {
                offset += checkNotNull(head.argument).toInt()
                if (head.major == 3) checkUtf8(data, head.end, offset)
            }
        }
    }
}

internal class Fields(
    private val data: ByteArray,
    var offset: Int,
    private val end: Int,
    private val bound: Int = end,
) {
    fun hasNext(): Boolean = offset < end

    fun next(): CborField {
        val start = offset
        if (start >= bound) malformed("out_of_range")
        offset = itemEnd(data, start, end)
        return CborField(data, start, offset)
    }
}

private fun checkUtf8(data: ByteArray, start: Int, end: Int) {
    val decoder = StandardCharsets.UTF_8.newDecoder()
    val input = ByteBuffer.wrap(data, start, end - start)
    val output = CharBuffer.allocate(256)
    while (true) {
        val result = decoder.decode(input, output, true)
        if (result.isError) malformed("wrong_type")
        if (!result.isOverflow) return
        output.clear()
    }
}
