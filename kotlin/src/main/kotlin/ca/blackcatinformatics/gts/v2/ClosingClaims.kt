// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

internal enum class ClosingClaimFailure { INDEX_OFFSETS, INDEX_LOCATOR }

internal class ClosingClaims(private val data: ByteArray) {
    private var segmentStart = 0
    private var indexOffset: Int? = null
    private var offsetsSeen = false
    private var locatorSeen = false
    private val damaged = mutableMapOf<Int, Int>()

    /** Records a damaged region of the segment, which counts as one item of its byte length (P5 section 7.2A). */
    fun damage(offset: Int, end: Int) {
        damaged[offset] = end
    }

    fun observe(item: WireEvent.Item, next: WireEvent?, afterLocator: WireEvent?): WireEvent.Item =
        when (item.kind) {
            ItemKind.HEADER, ItemKind.RESYNC -> {
                segmentStart = item.offset
                indexOffset = null
                offsetsSeen = false
                locatorSeen = false
                damaged.clear()
                item
            }
            ItemKind.FRAME -> {
                if (item.frameType == 15uL) indexOffset = item.offset
                item
            }
            ItemKind.OFFSETS -> {
                val finalLocator = (next as? WireEvent.Item)?.kind == ItemKind.LOCATOR && segmentBoundary(afterLocator)
                val misplaced = offsetsSeen || locatorSeen || !(segmentBoundary(next) || finalLocator)
                offsetsSeen = true
                check(item, misplaced, ClosingClaimFailure.INDEX_OFFSETS) { offsetsHold(item.offset, it) }
            }
            ItemKind.LOCATOR -> {
                val misplaced = locatorSeen || !segmentBoundary(next)
                locatorSeen = true
                check(item, misplaced, ClosingClaimFailure.INDEX_LOCATOR) { locatorHolds(item.offset, it) }
            }
            else -> item
        }

    private fun check(
        item: WireEvent.Item,
        misplaced: Boolean,
        failure: ClosingClaimFailure,
        holds: (CborField) -> Boolean,
    ): WireEvent.Item {
        if (misplaced) return item.copy(problem = "unordered_or_repeated")
        return try {
            val fields = CborField(data, item.offset, item.end).fields(2)
            fields.next()
            if (holds(fields.next())) item else item.copy(closingClaimFailure = failure)
        } catch (error: EnvelopeFailure) {
            item.copy(problem = error.reason)
        } catch (_: FramingException) {
            item.copy(closingClaimFailure = failure)
        }
    }

    private fun offsetsHold(offset: Int, lengths: CborField): Boolean {
        val claims = lengths.list()
        var observed = segmentStart
        var matches = true
        // Walk the observed prefix without retaining a length table or using claimed offsets.
        while (claims.hasNext()) {
            val claimed = claims.next().uint()
            if (observed == offset) {
                matches = false
            } else {
                // P5 7.2A: "For the check of an `offsets` item the damaged region is one item whose length is its number of bytes".
                val end = damaged[observed] ?: itemEnd(data, observed, offset)
                matches = matches && claimed == (end - observed).toULong()
                observed = end
            }
        }
        return matches && observed == offset
    }

    private fun locatorHolds(offset: Int, distance: CborField): Boolean {
        val head = distance.head()
        if (head.major != 0) distance.typed(0)
        if (data.octet(distance.start) != 0x1b) malformed("not_deterministic")
        return indexOffset?.let { head.argument == (offset - it).toULong() } == true
    }
}

private fun segmentBoundary(event: WireEvent?): Boolean =
    event == null || event is WireEvent.Stop ||
        (event is WireEvent.Item && (event.kind == ItemKind.HEADER || event.kind == ItemKind.RESYNC))
