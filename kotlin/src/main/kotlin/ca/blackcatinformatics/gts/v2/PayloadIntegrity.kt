// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import com.github.luben.zstd.Zstd
import com.github.luben.zstd.ZstdException

internal enum class PayloadState { VERIFIED, DAMAGED, UNCHECKED }

internal data class PayloadObservation(val state: PayloadState, val reason: String? = null, val count: ULong? = null)

internal val frameCapacity = 1uL shl 26

internal class PayloadBudget(private val readCapacity: ULong = 1uL shl 36) {
    private var total = 0uL
    private var exhausted = false

    fun check(payload: FramePayload): PayloadObservation {
        val length = payload.length ?: return damaged()
        val digest = payload.digest ?: return damaged()
        val storage = payload.storage ?: return damaged()
        if (!payload.deterministicBytes) return damaged()
        val stored = payload.stored
        val start = stored.head().end
        val size = stored.end - start
        admit(payload, length, size)?.let { return it }
        return when {
            storage.tag == 1uL -> {
                if (size.toULong() != length) damaged("decoded_length")
                else checkDigest(digest, stored.data, start, stored.end)
            }
            storage.codec?.uint() == 1uL -> checkZstd(digest, stored, start, length.toInt())
            else -> PayloadObservation(PayloadState.UNCHECKED, "unknown_codec")
        }
    }

    private fun admit(payload: FramePayload, length: ULong, storedSize: Int): PayloadObservation? {
        if (!exhausted) {
            val oversized = listOf(storedSize, payload.contentSize, payload.signaturesSize).firstOrNull { it.toULong() > frameCapacity }
            if (oversized != null) return PayloadObservation(PayloadState.UNCHECKED, "stored_size", oversized.toULong())
            if (length > frameCapacity) return PayloadObservation(PayloadState.UNCHECKED, "decoded_size", length)
        }
        val overflow = ULong.MAX_VALUE - total < length
        val next = if (overflow) ULong.MAX_VALUE else total + length
        if (exhausted || overflow || next > readCapacity) {
            exhausted = true
            return PayloadObservation(PayloadState.UNCHECKED, "total_decoded", next)
        }
        total = next
        return null
    }
}

private fun damaged(reason: String? = null) = PayloadObservation(PayloadState.DAMAGED, reason)

private fun checkDigest(digest: CborField, data: ByteArray, start: Int, end: Int): PayloadObservation =
    if (digest.matchesBytes(payloadDigest(data, start, end))) PayloadObservation(PayloadState.VERIFIED)
    else damaged("payload_digest")

private fun checkZstd(digest: CborField, stored: CborField, start: Int, length: Int): PayloadObservation {
    val header = zstdHeader(length)
    if (length == 0 || stored.end - start < header.size || header.indices.any { stored.data[start + it] != header[it] }) {
        return damaged("codec_constraint")
    }
    if (!singleZstdFrame(stored, start + header.size)) return damaged("decode_failed")
    val decoded = ByteArray(length)
    val written =
        try {
            Zstd.decompressByteArray(decoded, 0, length, stored.data, start, stored.end - start)
        } catch (_: ZstdException) {
            return damaged("decode_failed")
        }
    if (Zstd.isError(written) || written != length.toLong()) return damaged("decode_failed")
    return checkDigest(digest, decoded, 0, length)
}

private fun zstdHeader(length: Int): ByteArray {
    val width =
        when {
            length < 256 -> 1
            length <= 65791 -> 2
            else -> 4
        }
    val descriptor = (0x20 + 0x40 * Integer.numberOfTrailingZeros(width)).toByte()
    val encodedLength = if (width == 2) length - 256 else length
    return ByteArray(5 + width) { index ->
        when (index) {
            0 -> 0x28
            1 -> 0xb5.toByte()
            2 -> 0x2f
            3 -> 0xfd.toByte()
            4 -> descriptor
            else -> (encodedLength ushr (8 * (index - 5))).toByte()
        }
    }
}

// After checking the header, delimit a single block stream before using the decoder.
private fun singleZstdFrame(stored: CborField, firstBlock: Int): Boolean {
    val data = stored.data
    var offset = firstBlock
    while (stored.end - offset >= 3) {
        val header = data.octet(offset) or (data.octet(offset + 1) shl 8) or (data.octet(offset + 2) shl 16)
        val kind = (header ushr 1) and 3
        if (kind == 3) return false
        val size = if (kind == 1) 1 else header ushr 3
        offset += 3
        if (size > stored.end - offset) return false
        offset += size
        if ((header and 1) != 0) return offset == stored.end
    }
    return false
}
