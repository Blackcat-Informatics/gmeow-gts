// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

private const val SPAN_LENGTH = 4_194_304uL

// The longest content record of a parity frame: an array head, three uints and two 32-byte digests.
private const val CONTENT_WINDOW = 128
private val parityPattern = byteArrayOf(0x8b.toByte(), 0x02, 0x10, 0xf4.toByte(), 0xf5.toByte())
private val resyncPattern = byteArrayOf(0x8b.toByte(), 0x05, 0x02, 0x50) + "fcf3eb32a2f0267dd16a53f09bff8792".hexToByteArray()
private val tablePrefix = byteArrayOf(0x83.toByte(), 0x58, 0x20)

/** The content record of a `parity` frame (P5 section 6.5): `[geometry, first, end, anchor, table_digest]`. */
internal class ParityContent(val geometry: ULong, val first: ULong, val end: ULong, val anchor: ByteArray, val tableDigest: ByteArray)

internal fun readParityContent(content: CborField): ParityContent? =
    try {
        val fields = content.fields(5)
        val geometry = fields.next().uint()
        val first = fields.next().uint()
        val end = fields.next().uint()
        ParityContent(geometry, first, end, fields.next().digestBytes(), fields.next().digestBytes())
    } catch (_: EnvelopeFailure) {
        null
    }

private fun CborField.digestBytes(): ByteArray = data.copyOfRange(bytes(32), end)

/**
 * The spans of the frames of type `parity` that stand before a damaged region of one segment, for
 * the condition of P5 section 6.6 that no earlier parity frame has the same `first` or a short
 * span. A frame whose content record is ill typed or whose link is broken has no span.
 */
internal class ParitySpans {
    private val starts = mutableSetOf<ULong>()
    private var short = false

    fun remember(frame: FrameEnvelope, link: FrameLink, newer: Boolean) {
        if (frame.type != 16uL || link == FrameLink.BROKEN) return
        val content = readParityContent(frame.content) ?: return
        if (newer && content.geometry > 1uL) return
        starts += content.first
        short = short || (content.end > content.first && content.end - content.first < SPAN_LENGTH)
    }

    fun admits(content: ParityContent): Boolean = !short && content.first !in starts
}

/** A parity frame at which reading resumes: its offset, its `anchor` and its payload check, charged once in the scan. */
internal class ParityCandidate(val offset: Int, val anchor: ByteArray, val payload: PayloadObservation)

/**
 * The scan for a parity frame at which reading resumes (P5 section 7.2, step 6) in one segment:
 * its input, its opener identifier `h0`, its registry version, the spans of its earlier parity
 * frames and the total decoded length of the read.
 */
internal class ParityScan(
    private val data: ByteArray,
    private val opener: ByteArray,
    private val registry: ULong,
    private val spans: ParitySpans,
    private val budget: PayloadBudget,
    private val retainDecoded: Boolean,
) {
    /**
     * Scans forward from [from], with no bound, for the first parity candidate that passes: the
     * envelope, the link with its `anchor` as predecessor, the capacity charge, the payload
     * grammar and the `table_digest` under the opener. Returns null when none passes. A resync
     * item pattern met before a passing parity candidate also gives null: this reader does not
     * test resync candidates, so it does not read past one.
     */
    fun find(from: Int): ParityCandidate? {
        for (offset in from..data.size - parityPattern.size) {
            if (data[offset] != parityPattern[0]) continue
            if (data.matches(offset, resyncPattern)) return null
            if (!data.matches(offset, parityPattern)) continue
            test(offset)?.let { return it }
        }
        return null
    }

    private fun test(offset: Int): ParityCandidate? {
        val end = envelopeEnd(data, offset) ?: return null
        val frame =
            try {
                readFrame(CborField(data, offset, end), registry)
            } catch (_: EnvelopeFailure) {
                return null
            }
        val content = readParityContent(frame.content) ?: return null
        val layout = spanLayout(content) ?: return null
        val payload = frame.payload
        val identity = frame.identity ?: return null
        val stored = frame.storedId ?: return null
        val shaped =
            frame.problem == null && spans.admits(content) && payload.length == layout.payloadLength.toULong() &&
                payload.stored.end - payload.stored.head().end == layout.payloadLength
        if (!shaped || !frameIdentifier(content.anchor, identity).contentEquals(stored)) return null
        val observation = budget.check(payload, retainDecoded)
        if (observation.state != PayloadState.VERIFIED || !layout.holds(payload.stored, opener, content.tableDigest)) return null
        return ParityCandidate(offset, content.anchor, observation)
    }
}

/**
 * The end of the item at a parity candidate when its envelope has the shape of a `parity` frame
 * (P5 section 7.2, step 6), else null. The test follows the fields in wire order and stops at the
 * first that fails (P5-38). Each field is delimited within a window no longer than a valid parity
 * field, and the payload is jumped by its head, so a candidate that fails costs a bounded number
 * of heads whatever follows it: the scan over a damaged region is linear in its length.
 */
private fun envelopeEnd(data: ByteArray, offset: Int): Int? {
    var position = offset + parityPattern.size
    fun field(window: Int): Boolean =
        try {
            position = itemEnd(data, position, minOf(data.size, position + window))
            true
        } catch (_: FramingException) {
            false
        }
    // content, length, digest, storage
    if (!field(CONTENT_WINDOW) || !field(9) || !field(34) || !field(16)) return null
    val head =
        try {
            readHead(data, position, data.size)
        } catch (_: FramingException) {
            return null
        }
    val size = head.argument
    if (head.major != 2 || size == null || size > (data.size - head.end).toULong()) return null
    position = head.end + size.toInt()
    // identifier, then the empty signatures list of a parity frame
    if (!field(34) || position >= data.size || data[position] != 0x80.toByte()) return null
    return position + 1
}

private fun ByteArray.matches(offset: Int, pattern: ByteArray): Boolean =
    offset + pattern.size <= size && pattern.indices.all { this[offset + it] == pattern[it] }

private class SpanLayout(val dataShards: Int, val parityShards: Int, val shardLength: Int) {
    val parityBytes = parityShards * shardLength
    val payloadLength =
        1 + headSize(dataShards) + 17 * dataShards + headSize(parityShards) + 17 * parityShards + headSize(parityBytes) + parityBytes

    /** Whether the stored payload is the deterministic `parity_payload` of this layout and its table matches [digest] under [opener]. */
    fun holds(stored: CborField, opener: ByteArray, digest: ByteArray): Boolean {
        val data = stored.data
        var offset = stored.head().end
        val end = stored.end
        fun head(major: Int, argument: Int): Boolean {
            val read = runCatching { readHead(data, offset, end) }.getOrNull() ?: return false
            offset = read.end
            return read.major == major && read.shortest && read.argument == argument.toULong()
        }
        if (!head(4, 3)) return false
        val tableStart = offset
        for (count in intArrayOf(dataShards, parityShards)) {
            if (!head(4, count)) return false
            repeat(count) {
                if (!head(2, 16)) return false
                offset += 16
            }
        }
        val tableEnd = offset
        if (!head(2, parityBytes) || offset + parityBytes != end) return false
        val computed =
            contextHash("gts2 parity table") {
                update(tablePrefix, 0, tablePrefix.size)
                update(opener, 0, opener.size)
                update(data, tableStart, tableEnd - tableStart)
            }
        return computed.contentEquals(digest)
    }
}

private fun spanLayout(content: ParityContent): SpanLayout? {
    if (content.geometry != 1uL || content.first % SPAN_LENGTH != 0uL || content.end <= content.first) return null
    val length = content.end - content.first
    if (length > SPAN_LENGTH) return null
    val blocks = (length.toLong() - 1) / 64
    val shardLength = maxOf(64, minOf(4096, 1 shl (64 - java.lang.Long.numberOfLeadingZeros(blocks))))
    val dataShards = ((length.toLong() + shardLength - 1) / shardLength).toInt()
    return SpanLayout(dataShards, 4 * ((dataShards + 63) / 64), shardLength)
}

private fun headSize(value: Int): Int =
    when {
        value < 24 -> 1
        value < 256 -> 2
        value < 65536 -> 3
        else -> 5
    }
