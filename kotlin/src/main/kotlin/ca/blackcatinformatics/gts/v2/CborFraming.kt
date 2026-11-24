// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

internal enum class StopReason {
    TORN_TAIL,
    UNDELIMITABLE_INPUT,
    ITEMS_PER_READ,
}

internal class FramingException(val reason: StopReason) : RuntimeException(reason.name)

internal data class CborHead(
    val major: Int,
    val argument: ULong?,
    val end: Int,
    val shortest: Boolean,
)

internal fun ByteArray.octet(offset: Int): Int = this[offset].toInt() and 0xff

internal fun readHead(data: ByteArray, offset: Int, limit: Int): CborHead {
    if (offset == limit) throw FramingException(StopReason.TORN_TAIL)
    val first = data.octet(offset)
    val major = first ushr 5
    val info = first and 31
    val start = offset + 1
    return when {
        info < 24 -> CborHead(major, info.toULong(), start, true)
        info <= 27 -> {
            val width = 1 shl (info - 24)
            if (width > limit - start) throw FramingException(StopReason.TORN_TAIL)
            var value = 0uL
            for (index in start until start + width) {
                value = (value shl 8) or data.octet(index).toULong()
            }
            if (major == 7 && info == 24 && value < 32uL) {
                throw FramingException(StopReason.UNDELIMITABLE_INPUT)
            }
            val minimum =
                when (width) {
                    1 -> 24uL
                    2 -> 256uL
                    4 -> 65536uL
                    else -> 1uL shl 32
                }
            CborHead(major, value, start + width, value >= minimum)
        }
        info == 31 && (major in 2..5 || major == 7) -> CborHead(major, null, start, false)
        else -> throw FramingException(StopReason.UNDELIMITABLE_INPUT)
    }
}

private class Container(
    val major: Int,
    var remaining: ULong?,
    var needsValue: Boolean = false,
)

internal fun itemEnd(data: ByteArray, start: Int, limit: Int = data.size): Int {
    val pending = ArrayDeque<Container>()
    pending.addLast(Container(4, 1uL))
    var offset = start
    while (pending.isNotEmpty()) {
        val parent = pending.last()
        if (parent.remaining == 0uL) {
            pending.removeLast()
            continue
        }
        if (offset == limit) throw FramingException(StopReason.TORN_TAIL)
        val first = data.octet(offset)
        if (first == 0xff) {
            if (parent.remaining != null || parent.needsValue) {
                throw FramingException(StopReason.UNDELIMITABLE_INPUT)
            }
            pending.removeLast()
            offset++
            continue
        }
        if (parent.major in 2..3 && (first ushr 5 != parent.major || first and 31 == 31)) {
            throw FramingException(StopReason.UNDELIMITABLE_INPUT)
        }

        val head = readHead(data, offset, limit)
        offset = head.end
        if (parent.major == 5) parent.needsValue = !parent.needsValue
        // Count map pairs without doubling a uint64 length.
        if (!parent.needsValue) parent.remaining = parent.remaining?.minus(1uL)
        val size = head.argument
        when {
            size == null || head.major == 4 || head.major == 5 || head.major == 6 -> {
                if (head.major == 7) throw FramingException(StopReason.UNDELIMITABLE_INPUT)
                pending.addLast(Container(head.major, if (head.major == 6) 1uL else size))
            }
            head.major in 2..3 -> {
                if (size > (limit - offset).toULong()) throw FramingException(StopReason.TORN_TAIL)
                offset += size.toInt()
            }
        }
    }
    return offset
}
