// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import org.bouncycastle.crypto.digests.Blake3Digest
import org.bouncycastle.crypto.params.Blake3Parameters

private val framePrefix = byteArrayOf(0x87.toByte(), 0x58, 0x20)
private val resyncPrefix = byteArrayOf(0x8a.toByte())
private val codecPrefix = byteArrayOf(0x81.toByte())

internal enum class FrameLink { LINKED, HEALED, BROKEN }

internal data class LinkObservation(val state: FrameLink, val effective: ByteArray?)

internal fun checkLink(
    frame: FrameEnvelope,
    predecessor: ByteArray,
    next: FrameEnvelope?,
): LinkObservation {
    val computed = frame.identity?.let { frameIdentifier(predecessor, it) }
    val stored = frame.storedId
    val state =
        when {
            computed == null || stored == null -> FrameLink.BROKEN
            computed.contentEquals(stored) -> FrameLink.LINKED
            !frame.transparent && next?.storedId != null &&
                next.identity?.let { frameIdentifier(computed, it).contentEquals(next.storedId) } == true -> FrameLink.HEALED
            else -> FrameLink.BROKEN
        }
    val effective =
        when {
            frame.transparent -> null
            state == FrameLink.HEALED -> computed
            else -> stored ?: computed
        }
    return LinkObservation(state, effective)
}

internal fun openerIdentifier(item: CborField, resync: Boolean): ByteArray =
    contextHash("gts2 header") {
        if (resync) {
            update(resyncPrefix, 0, resyncPrefix.size)
            update(item.data, item.start + 1, item.end - item.start - 1)
        } else {
            update(item)
        }
    }

private fun frameIdentifier(predecessor: ByteArray, identity: CborField): ByteArray =
    contextHash("gts2 frame") {
        update(framePrefix, 0, framePrefix.size)
        update(predecessor, 0, predecessor.size)
        update(identity)
    }

internal fun checkStored(payload: FramePayload): Boolean? {
    val codec = payload.storage?.codec ?: return null
    val check = payload.storage.check ?: return null
    if (payload.stored.head().major != 2) return null
    val computed =
        contextHash("gts2 stored") {
            update(codecPrefix, 0, codecPrefix.size)
            update(codec)
            payload.stored.forEachByteChunk { start, end -> update(payload.stored.data, start, end - start) }
        }
    return check.matchesBytes(computed, 16)
}

internal fun payloadDigest(data: ByteArray, start: Int, end: Int): ByteArray =
    contextHash("gts2 payload") { update(data, start, end - start) }

internal fun contextHash(context: String, visit: Blake3Digest.() -> Unit): ByteArray {
    val digest = Blake3Digest(256)
    digest.init(Blake3Parameters.context(context.encodeToByteArray()))
    digest.visit()
    val result = ByteArray(digest.digestSize)
    digest.doFinal(result, 0)
    return result
}

private fun Blake3Digest.update(field: CborField) = update(field.data, field.start, field.end - field.start)

internal fun CborField.matchesBytes(expected: ByteArray, length: Int = expected.size): Boolean {
    val offset = head().end
    return end - offset == length && (0 until length).all { data[offset + it] == expected[it] }
}

internal fun CborField.observedIdentifier(): ByteArray? {
    if (head().major != 2) return null
    val id = ByteArray(32)
    var length = 0
    forEachByteChunk { start, end ->
        val size = end - start
        if (size > id.size - length) return null
        data.copyInto(id, length, start, end)
        length += size
    }
    return id.takeIf { length == it.size }
}

private inline fun CborField.forEachByteChunk(visit: (start: Int, end: Int) -> Unit) {
    val head = head()
    if (head.argument != null) {
        visit(head.end, end)
        return
    }
    var offset = head.end
    while (offset < end - 1) {
        val chunk = readHead(data, offset, end)
        offset = chunk.end + checkNotNull(chunk.argument).toInt()
        visit(chunk.end, offset)
    }
}
