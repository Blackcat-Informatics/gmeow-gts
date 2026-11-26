// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString

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

    /** The bytes between the first byte that cannot be delimited and the parity frame at which reading resumes. */
    DAMAGED,
}

internal sealed interface WireEvent {
    data class Item(
        val offset: Int,
        val end: Int,
        val kind: ItemKind,
        val problem: String? = null,
        val opener: OpenerEnvelope? = null,
        val frameType: ULong? = null,
        val link: FrameLink? = null,
        val storedMatches: Boolean? = null,
        val payload: PayloadObservation? = null,
        val closingClaimFailure: ClosingClaimFailure? = null,
        val effectiveId: ByteString? = null,
        val commitClaim: CommitClaim? = null,
        val content: CborField? = null,
        val signatureResults: List<SignatureResult>? = null,
    ) : WireEvent

    data class Stop(val offset: Int, val reason: StopReason) : WireEvent, WireInspection
}

/**
 * Reads the items of a second-generation input. When [heldKeys] is given, each ordinary frame
 * that has an effective identifier, in a segment whose opener is readable and that carries
 * signature records, also gets one [SignatureResult] per record in `signatureResults`
 * (P6 section 6.2); a frame without an effective identifier, a frame of a segment whose opener
 * is not readable and a frame without signature records get none. Without [heldKeys] no
 * signature is verified.
 */
internal fun inspectWire(
    data: ByteArray,
    retainDecoded: Boolean = false,
    heldKeys: List<HeldKey>? = null,
): WireInspection {
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
            version.argument == 2uL -> WireInspection.VersionTwo(items(data, end, retainDecoded, heldKeys))
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

private data class ScannedItem(
    val event: WireEvent,
    val frame: FrameEnvelope? = null,
    val gap: Boolean = false,
)

/** Where the item scanner continues after a damaged region; negative when it ends there. */
private class ScanResume {
    var offset = -1
}

private fun items(data: ByteArray, firstEnd: Int, retainDecoded: Boolean, heldKeys: List<HeldKey>?): Sequence<WireEvent> =
    sequence {
        val resume = ScanResume()
        val source = scanItems(data, firstEnd, resume).iterator()
        fun advance(): ScannedItem? = if (source.hasNext()) source.next() else null

        val budget = PayloadBudget()
        val closing = ClosingClaims(data)
        var predecessor: ByteArray? = null
        var segmentOpener: ByteArray? = null
        var segmentRegistry = 0uL
        var spans = ParitySpans()
        var pending: ParityCandidate? = null
        var resumed: ParityCandidate? = null
        var current = advance()
        var next = advance()
        while (current != null) {
            val item = current.event
            if (item !is WireEvent.Item) {
                val candidate = pending
                if (!current.gap || candidate == null) {
                    yield(item)
                    break
                }
                pending = null
                resumed = candidate
                val stop = item as WireEvent.Stop
                // The finding of the damaged region stands (P5 section 7.2A), its length running to the resumption point.
                closing.damage(stop.offset, candidate.offset)
                yield(WireEvent.Item(stop.offset, candidate.offset, ItemKind.DAMAGED, stop.reason.name.lowercase()))
                current = next
                next = advance()
                continue
            }
            if (item.kind == ItemKind.HEADER || item.kind == ItemKind.RESYNC) {
                predecessor = item.effectiveId?.bytes
                segmentOpener = predecessor
                segmentRegistry = item.opener?.registry ?: 0uL
                spans = ParitySpans()
            }
            val following = next?.event
            val afterLocator = if (item.kind == ItemKind.OFFSETS && (following as? WireEvent.Item)?.kind == ItemKind.LOCATOR) advance() else null
            val event = closing.observe(item, following, afterLocator?.event)
            val frame = current.frame
            if (frame != null && predecessor != null) {
                val resumption = resumed?.takeIf { it.offset == item.offset }
                if (resumption != null) {
                    // P1 section 3.4, step 4: at a parity frame at which reading resumes the predecessor is its `anchor`.
                    predecessor = resumption.anchor
                    resumed = null
                }
                val link = checkLink(frame, predecessor, next?.frame)
                predecessor = link.effective ?: predecessor
                val storedMatches = checkStored(frame.payload)
                // A parity frame at which reading resumes was charged to the total in the scan, and is not charged again.
                val payload = resumption?.payload ?: budget.check(frame.payload, retainDecoded)
                spans.remember(frame, link.state, segmentRegistry > 1uL)
                val problem = event.problem ?: payload.reason?.takeIf { it == "codec_constraint" }
                val claim =
                    frame.content.takeIf { event.frameType == 14uL && problem == null && frame.payload.contentSize.toULong() <= frameCapacity }
                        ?.readCommitClaim()
                yield(
                    event.copy(
                        problem = problem,
                        link = link.state,
                        storedMatches = storedMatches,
                        payload = payload,
                        effectiveId = link.effective?.let(::ByteString),
                        commitClaim = claim,
                        content = frame.content.takeIf { retainDecoded },
                        signatureResults = verifyFrameSignatures(frame, link.effective, segmentOpener, heldKeys),
                    ),
                )
            } else {
                yield(event)
            }
            current = next
            if (current?.gap == true) {
                val stop = current.event as WireEvent.Stop
                val opener = segmentOpener
                pending =
                    opener?.let { ParityScan(data, it, segmentRegistry, spans, budget, retainDecoded).find(stop.offset) }
                resume.offset = pending?.offset ?: -1
            }
            next = afterLocator ?: advance()
        }
    }

private fun verifyFrameSignatures(
    frame: FrameEnvelope,
    frameIdentifier: ByteArray?,
    headerIdentifier: ByteArray?,
    heldKeys: List<HeldKey>?,
): List<SignatureResult>? {
    val records = frame.payload.signatures
    if (frame.transparent || heldKeys == null || frameIdentifier == null || headerIdentifier == null || records.isEmpty()) {
        return null
    }
    return records.map { verifySignature(it, frameIdentifier, headerIdentifier, heldKeys) }
}

private fun scanItems(data: ByteArray, firstEnd: Int, resume: ScanResume): Sequence<ScannedItem> =
    sequence {
        var offset = 0
        var count = 0
        var registry: ULong? = null
        while (offset < data.size) {
            if (count == (1 shl 26)) {
                yield(ScannedItem(WireEvent.Stop(offset, StopReason.ITEMS_PER_READ)))
                break
            }
            val end =
                try {
                    if (offset == 0) firstEnd else itemEnd(data, offset)
                } catch (error: FramingException) {
                    yield(ScannedItem(WireEvent.Stop(offset, error.reason), gap = true))
                    if (resume.offset < 0) break
                    // The damaged region counts as one item (P5 section 7.2A).
                    offset = resume.offset
                    count++
                    continue
                }
            val item = classify(data, offset, end, registry)
            val event = item.event as WireEvent.Item
            if (event.kind == ItemKind.HEADER || event.kind == ItemKind.RESYNC) {
                registry = event.opener?.takeIf { event.effectiveId != null }?.registry
            }
            yield(item)
            offset = end
            count++
        }
    }

private fun classify(data: ByteArray, offset: Int, end: Int, registry: ULong?): ScannedItem {
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
            ScannedItem(
                WireEvent.Item(offset, end, kind, opener = opener.envelope, effectiveId = opener.identifier?.let(::ByteString)),
            )
        } catch (error: EnvelopeFailure) {
            ScannedItem(WireEvent.Item(offset, end, kind, error.reason))
        }
    }
    return when {
        registry == null -> ScannedItem(WireEvent.Item(offset, end, ItemKind.UNCHECKED))
        code == null -> ScannedItem(WireEvent.Item(offset, end, ItemKind.OPAQUE, "wrong_type"))
        !array.shortest || !code.shortest -> ScannedItem(WireEvent.Item(offset, end, ItemKind.OPAQUE, "not_deterministic"))
        kind == ItemKind.FRAME && array.argument != 11uL -> ScannedItem(WireEvent.Item(offset, end, ItemKind.OPAQUE, "wrong_length"))
        kind == ItemKind.FRAME -> {
            val frame = readFrame(CborField(data, offset, end), registry)
            ScannedItem(WireEvent.Item(offset, end, kind, frame.problem, frameType = frame.type), frame)
        }
        kind == ItemKind.OPAQUE -> {
            val reason = if (code.argument != 0uL && registry > 1uL) "unknown_item_kind" else "unregistered_value"
            ScannedItem(WireEvent.Item(offset, end, kind, reason))
        }
        else -> ScannedItem(WireEvent.Item(offset, end, kind))
    }
}
