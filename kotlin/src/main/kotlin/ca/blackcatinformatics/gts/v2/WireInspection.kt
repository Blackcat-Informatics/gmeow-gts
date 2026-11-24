// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

internal sealed interface WireInspection {
    data object Legacy : WireInspection

    data object Unknown : WireInspection

    data object Empty : WireInspection

    data class Unsupported(val generation: ULong) : WireInspection

    data class VersionTwo(val events: Sequence<WireEvent>) : WireInspection
}

internal enum class ItemKind {
    HEADER,
    RESYNC,
    FRAME,
    OFFSETS,
    LOCATOR,
    OPAQUE,
    UNCHECKED,
}

internal sealed interface WireEvent {
    data class Item(
        val offset: Int,
        val end: Int,
        val kind: ItemKind,
        val problem: String? = null,
        val opener: OpenerEnvelope? = null,
        val frameType: ULong? = null,
    ) : WireEvent

    data class Stop(val offset: Int, val reason: StopReason) : WireEvent, WireInspection
}

internal fun inspectWire(data: ByteArray): WireInspection {
    if (data.isEmpty()) return WireInspection.Empty
    val end =
        try {
            itemEnd(data, 0)
        } catch (error: FramingException) {
            return WireEvent.Stop(0, error.reason)
        }
    val first = readHead(data, 0, end)
    val kind = readKind(data, first, end)
    if (kind?.argument == 1uL || kind?.argument == 5uL) {
        val version = nextHead(data, kind.end, end)
        return when {
            version?.major != 0 -> WireInspection.Unknown
            version.argument == 2uL -> WireInspection.VersionTwo(items(data, end))
            version.argument != null && version.argument > 2uL -> WireInspection.Unsupported(version.argument)
            else -> WireInspection.Unknown
        }
    }
    val generation = legacyGeneration(data, first, end)
    return when (generation) {
        1uL -> WireInspection.Legacy
        null -> WireInspection.Unknown
        else -> WireInspection.Unsupported(generation)
    }
}

private fun nextHead(data: ByteArray, offset: Int, end: Int): CborHead? =
    if (offset == end || data.octet(offset) == 0xff) null else readHead(data, offset, end)

private fun readKind(data: ByteArray, array: CborHead, end: Int): CborHead? =
    if (array.major == 4 && array.argument != 0uL) {
        nextHead(data, array.end, end)?.takeIf { it.major == 0 }
    } else {
        null
    }

private fun legacyGeneration(data: ByteArray, top: CborHead, end: Int): ULong? {
    val map = if (top.major == 6 && top.argument == 55799uL) readHead(data, top.end, end) else top
    if (map.major != 5) return null
    val bodyEnd = if (map.argument == null) end - 1 else end
    var offset = map.end
    while (offset < bodyEnd) {
        val keyEnd = itemEnd(data, offset, bodyEnd)
        if (isVersionKey(data, offset, keyEnd)) {
            val value = readHead(data, keyEnd, bodyEnd)
            return value.argument.takeIf { value.major == 0 }
        }
        offset = itemEnd(data, keyEnd, bodyEnd)
    }
    return null
}

private fun isVersionKey(data: ByteArray, start: Int, end: Int): Boolean {
    val text = readHead(data, start, end)
    if (text.major != 3) return false
    if (text.argument != null) return text.argument == 1uL && data.octet(text.end) == 0x76
    var offset = text.end
    var found = false
    while (offset < end - 1) {
        val chunk = readHead(data, offset, end)
        when (chunk.argument) {
            0uL -> offset = chunk.end
            1uL -> {
                if (found || data.octet(chunk.end) != 0x76) return false
                found = true
                offset = chunk.end + 1
            }
            else -> return false
        }
    }
    return found
}

private fun items(data: ByteArray, firstEnd: Int): Sequence<WireEvent> =
    sequence {
        var offset = 0
        var count = 0
        var registry: ULong? = null
        while (offset < data.size) {
            if (count == (1 shl 26)) {
                yield(WireEvent.Stop(offset, StopReason.ITEMS_PER_READ))
                break
            }
            val end =
                try {
                    if (offset == 0) firstEnd else itemEnd(data, offset)
                } catch (error: FramingException) {
                    yield(WireEvent.Stop(offset, error.reason))
                    break
                }
            val item = classify(data, offset, end, registry)
            if (item.kind == ItemKind.HEADER || item.kind == ItemKind.RESYNC) {
                registry = item.opener?.takeIf { it.hashAlgorithm == 1uL }?.registry
            }
            yield(item)
            offset = end
            count++
        }
    }

private fun classify(data: ByteArray, offset: Int, end: Int, registry: ULong?): WireEvent.Item {
    val array = readHead(data, offset, end)
    val code = readKind(data, array, end)
    val kind =
        when (code?.argument) {
            1uL -> ItemKind.HEADER
            2uL -> ItemKind.FRAME
            3uL -> ItemKind.LOCATOR
            4uL -> ItemKind.OFFSETS
            5uL -> ItemKind.RESYNC
            else -> ItemKind.OPAQUE
        }
    if (kind == ItemKind.HEADER || kind == ItemKind.RESYNC) {
        return try {
            val opener = readOpener(CborField(data, offset, end), kind == ItemKind.RESYNC)
            WireEvent.Item(offset, end, kind, opener = opener)
        } catch (error: EnvelopeFailure) {
            WireEvent.Item(offset, end, kind, error.reason)
        }
    }
    return when {
        registry == null -> WireEvent.Item(offset, end, ItemKind.UNCHECKED)
        code == null -> WireEvent.Item(offset, end, ItemKind.OPAQUE, "wrong_type")
        !array.shortest || !code.shortest -> WireEvent.Item(offset, end, ItemKind.OPAQUE, "not_deterministic")
        kind == ItemKind.FRAME && array.argument != 11uL -> WireEvent.Item(offset, end, ItemKind.OPAQUE, "wrong_length")
        kind == ItemKind.FRAME -> {
            val frame = readFrame(CborField(data, offset, end), registry)
            WireEvent.Item(offset, end, kind, frame.problem, frameType = frame.type)
        }
        kind == ItemKind.OPAQUE -> {
            val reason = if (code.argument != 0uL && registry > 1uL) "unknown_item_kind" else "unregistered_value"
            WireEvent.Item(offset, end, kind, reason)
        }
        else -> WireEvent.Item(offset, end, kind)
    }
}
