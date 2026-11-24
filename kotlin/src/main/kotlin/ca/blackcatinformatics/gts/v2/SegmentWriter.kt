// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.CborBool
import ca.blackcatinformatics.gts.CborBytes
import ca.blackcatinformatics.gts.cborArray
import ca.blackcatinformatics.gts.encode
import ca.blackcatinformatics.gts.encodeInto
import ca.blackcatinformatics.gts.encodeUnsignedHead
import ca.blackcatinformatics.gts.uint
import java.io.ByteArrayOutputStream

data class V2Lineage(val previousHead: ByteString, val ordinal: ULong)

/**
 * Writes a registry version 1 segment with plain, unsigned frames.
 *
 * The caller supplies frame content and payload semantics, satisfies any
 * profile or canonical claim, and supplies the correct lineage when appending.
 */
class V2SegmentWriter(
    profile: Int? = null,
    canonical: Boolean = false,
    lineage: V2Lineage? = null,
) {
    private val output = ByteArrayOutputStream()
    private val range = PrefixRange()
    private var itemCount = 1
    private var predecessor: ByteString

    init {
        val opener = encodeOpener(profile, canonical, lineage)
        output.write(opener)
        predecessor = ByteString(openerIdentifier(CborField(opener, 0, opener.size), resync = false))
        range.push(predecessor)
    }

    /** The current opener or ordinary frame identifier. */
    val head: ByteArray
        get() = predecessor.bytes.copyOf()

    /** An independent snapshot of the segment. */
    fun toBytes(): ByteArray = output.toByteArray()

    /** Append an envelope with a deterministic CBOR content record and decoded payload. */
    fun append(frameType: Int, content: ByteArray, payload: ByteArray): ByteArray {
        require(frameType in 1..16) { "Invalid GTS v2 frame type $frameType" }
        require(frameType != 14) { "Use commit() to append a GTS v2 commit" }
        require(payload.size.toULong() <= frameCapacity) { "GTS v2 payload exceeds the frame capacity" }
        require(!invalidPayloadLength(frameType.toULong(), payload.size.toULong())) {
            "Invalid payload length for GTS v2 frame type $frameType"
        }
        require(content.size.toULong() <= frameCapacity) { "GTS v2 content exceeds the frame capacity" }
        val encoded = content.copyOf()
        val field = CborField(encoded, 0, encoded.size)
        try {
            require(itemEnd(encoded, 0) == encoded.size) { "Expected one complete GTS v2 content record" }
            field.grammar()
            checkContent(frameType.toULong(), field)
        } catch (error: FramingException) {
            throw IllegalArgumentException("Invalid framing for GTS v2 frame type $frameType content: ${error.reason}", error)
        } catch (error: EnvelopeFailure) {
            throw IllegalArgumentException("Invalid content for GTS v2 frame type $frameType: ${error.reason}", error)
        }
        return writeFrame(frameType, encoded, payload.copyOf())
    }

    /** Commit the opener and preceding ordinary frames, then advance the head. */
    fun commit(): ByteArray {
        val claim = cborArray(uint(range.leafCount - 1), CborBytes(ByteString(range.root())))
        return writeFrame(14, encode(claim), ByteArray(0))
    }

    private fun writeFrame(frameType: Int, content: ByteArray, payload: ByteArray): ByteArray {
        require(itemCount < (1 shl 26)) { "GTS v2 segment exceeds the item capacity" }
        val transparent = frameType == 16
        val digest = CborBytes(ByteString(payloadDigest(payload, 0, payload.size)))
        val identity =
            ByteArrayOutputStream().also { out ->
                encodeInto(out, uint(frameType))
                encodeInto(out, CborBool(frameType < 14))
                encodeInto(out, CborBool(transparent))
                out.write(content)
                encodeInto(out, uint(payload.size))
                encodeInto(out, digest)
            }.toByteArray()
        val id = ByteString(frameIdentifier(predecessor.bytes, CborField(identity, 0, identity.size)))

        encodeUnsignedHead(output, 4, 11uL)
        encodeInto(output, uint(2))
        output.write(identity)
        encodeInto(output, cborArray(uint(1)))
        encodeInto(output, CborBytes(ByteString(payload)))
        encodeInto(output, CborBytes(id))
        encodeInto(output, cborArray())

        if (!transparent) {
            range.push(id)
            predecessor = id
        }
        itemCount++
        return id.bytes.copyOf()
    }
}

private fun encodeOpener(profile: Int?, canonical: Boolean, lineage: V2Lineage?): ByteArray {
    require(profile == null || profile in 1..6) { "Invalid GTS v2 profile $profile" }
    if (lineage != null) {
        require(lineage.previousHead.bytes.size == 32 && lineage.ordinal != 0uL) {
            "GTS v2 lineage requires a 32-byte previous head and a positive ordinal"
        }
    }
    val out = ByteArrayOutputStream()
    encodeUnsignedHead(out, 4, 7uL)
    val profileOption = if (profile == null) cborArray(uint(1)) else cborArray(uint(2), uint(profile))
    val canonicalOption = if (canonical) cborArray(uint(2), uint(1)) else cborArray(uint(1))
    val fields = listOf(uint(1), uint(2), uint(1), uint(1), profileOption, canonicalOption)
    fields.forEach { encodeInto(out, it) }
    if (lineage == null) {
        encodeInto(out, cborArray(uint(1)))
    } else {
        encodeUnsignedHead(out, 4, 2uL)
        encodeInto(out, uint(2))
        encodeUnsignedHead(out, 4, 2uL)
        encodeInto(out, CborBytes(lineage.previousHead))
        encodeUnsignedHead(out, 0, lineage.ordinal)
    }
    return out.toByteArray()
}
