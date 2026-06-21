// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import java.security.MessageDigest
import java.util.Base64

private const val ED25519_ALGO = 22
private val ED25519_OID = byteArrayOf(0x2b, 0x06, 0x01, 0x04, 0x01, 0xda.toByte(), 0x47, 0x0f, 0x01)
private val EMOJI =
    arrayOf(
        "🐵", "🐶", "🐺", "🦊", "🐱", "🦁", "🐯", "🐴",
        "🦄", "🦓", "🦌", "🐮", "🐷", "🐗", "🐭", "🐹",
        "🐰", "🐻", "🐼", "🐨", "🐸", "🐲", "🐔", "🐧",
        "🦆", "🦅", "🦉", "🦇", "🐢", "🐍", "🦎", "🐊",
        "🐳", "🐬", "🐟", "🐠", "🐡", "🦈", "🐙", "🦑",
        "🦀", "🦞", "🦐", "🦋", "🐌", "🐞", "🐝", "🐜",
        "🦂", "🍎", "🍐", "🍊", "🍋", "🍌", "🍉", "🍇",
        "🍓", "🍒", "🍍", "🥝", "🍑", "🥥", "🥕", "🌽",
    )

data class TransportKey(val rawPublic: ByteArray, val fingerprint: String) {
    override fun equals(other: Any?): Boolean =
        other is TransportKey && rawPublic.contentEquals(other.rawPublic) && fingerprint == other.fingerprint

    override fun hashCode(): Int = 31 * rawPublic.contentHashCode() + fingerprint.hashCode()
}

fun parseTransportKey(armored: String): TransportKey {
    val packets = iterPackets(stripArmor(armored))
    for (packet in packets) {
        when (packet.tag) {
            6 -> {
                val parsed = parseEd25519PublicMaterial(packet.body)
                return TransportKey(parsed.raw, fingerprint(packet.body))
            }
            5 -> {
                val parsed = parseEd25519PublicMaterial(packet.body)
                return TransportKey(parsed.raw, fingerprint(packet.body.copyOfRange(0, parsed.end)))
            }
        }
    }
    error("no public-key packet found")
}

fun formatFingerprint(fingerprint: String): String {
    val compact = fingerprint.filterNot { it.isWhitespace() }.uppercase()
    if (compact.isEmpty() || compact.any { it !in "0123456789ABCDEF" }) return fingerprint
    return compact.chunked(4).joinToString(" ")
}

fun emojihash(data: ByteArray, length: Int = 11): String {
    val wanted = length.coerceAtLeast(1)
    val nbytes = (wanted * 6 + 7) / 8
    val digest = blake3_256(data).copyOfRange(0, nbytes)
    val out = mutableListOf<Int>()
    var acc = 0L
    var bits = 0
    for (b in digest) {
        acc = (acc shl 8) or (b.toLong() and 0xff)
        bits += 8
        while (bits >= 6 && out.size < wanted) {
            bits -= 6
            out += ((acc shr bits) and 0x3f).toInt()
        }
        acc = acc and ((1L shl bits) - 1)
    }
    return out.take(wanted).joinToString(" ") { EMOJI[it] }
}

private data class Packet(val tag: Int, val body: ByteArray)

private data class PublicMaterial(val raw: ByteArray, val end: Int)

private fun stripArmor(text: String): ByteArray {
    val lines = text.split('\n')
    val start = lines.indexOfFirst { it.startsWith("-----BEGIN PGP") }
    require(start >= 0) { "missing armor BEGIN line" }
    val end = lines.drop(start + 1).indexOfFirst { it.startsWith("-----END PGP") }.let {
        if (it < 0) -1 else it + start + 1
    }
    require(end >= 0) { "missing armor END line" }
    var idx = start + 1
    while (idx < end && lines[idx].trim().isNotEmpty() && ":" in lines[idx]) idx++
    val body = StringBuilder()
    while (idx < end) {
        val line = lines[idx].trimEnd('\r')
        if (line.startsWith("=")) break
        body.append(line)
        idx++
    }
    require(body.isNotEmpty()) { "empty armor body" }
    return try {
        Base64.getDecoder().decode(body.toString())
    } catch (_: IllegalArgumentException) {
        throw IllegalArgumentException("invalid base64 armor body")
    }
}

private fun iterPackets(data: ByteArray): List<Packet> {
    val packets = mutableListOf<Packet>()
    var offset = 0
    while (offset < data.size) {
        val next = nextPacket(data, offset)
        packets += Packet(next.tag, next.body)
        offset = next.next
    }
    return packets
}

private data class PacketRead(val tag: Int, val body: ByteArray, val next: Int)

private fun nextPacket(data: ByteArray, initialOffset: Int): PacketRead {
    var offset = initialOffset
    require(offset < data.size) { "truncated packet header" }
    val header = data[offset].toInt() and 0xff
    require(header and 0x80 != 0) { "invalid packet tag octet" }
    var tag: Int
    var length: Int
    if (header and 0x40 != 0) {
        tag = header and 0x3f
        offset++
        require(offset < data.size) { "truncated new-format length octet" }
        val lo = data[offset].toInt() and 0xff
        when {
            lo < 192 -> {
                length = lo
                offset++
            }
            lo < 224 -> {
                require(offset + 1 < data.size) { "truncated new-format 2-octet length" }
                length = ((lo - 192) shl 8) + (data[offset + 1].toInt() and 0xff) + 192
                offset += 2
            }
            lo == 255 -> {
                require(offset + 4 < data.size) { "truncated new-format 4-octet length" }
                length =
                    ((data[offset + 1].toInt() and 0xff) shl 24) or
                        ((data[offset + 2].toInt() and 0xff) shl 16) or
                        ((data[offset + 3].toInt() and 0xff) shl 8) or
                        (data[offset + 4].toInt() and 0xff)
                offset += 5
            }
            else -> error("partial body lengths are not supported")
        }
    } else {
        tag = (header shr 2) and 0x0f
        val lengthType = header and 0x03
        offset++
        when (lengthType) {
            0 -> {
                require(offset < data.size) { "truncated old-format length octet" }
                length = data[offset].toInt() and 0xff
                offset++
            }
            1 -> {
                require(offset + 1 < data.size) { "truncated old-format 2-octet length" }
                length = ((data[offset].toInt() and 0xff) shl 8) or (data[offset + 1].toInt() and 0xff)
                offset += 2
            }
            2 -> {
                require(offset + 3 < data.size) { "truncated old-format 4-octet length" }
                length =
                    ((data[offset].toInt() and 0xff) shl 24) or
                        ((data[offset + 1].toInt() and 0xff) shl 16) or
                        ((data[offset + 2].toInt() and 0xff) shl 8) or
                        (data[offset + 3].toInt() and 0xff)
                offset += 4
            }
            else -> error("indeterminate-length packets are not supported")
        }
    }
    val end = offset + length
    require(end <= data.size) { "packet body exceeds input" }
    return PacketRead(tag, data.copyOfRange(offset, end), end)
}

private fun parseEd25519PublicMaterial(body: ByteArray): PublicMaterial {
    require(body.size >= 6 && body[0].toInt() == 4) { "only OpenPGP v4 public keys are supported" }
    require((body[5].toInt() and 0xff) == ED25519_ALGO) { "unsupported public-key algorithm ${body[5].toInt() and 0xff}" }
    var offset = 6
    require(offset < body.size) { "truncated public-key packet" }
    val oidLen = body[offset].toInt() and 0xff
    offset++
    require(offset + oidLen <= body.size) { "truncated OID" }
    val oid = body.copyOfRange(offset, offset + oidLen)
    offset += oidLen
    require(oid.contentEquals(ED25519_OID)) { "unsupported curve OID ${hex(oid)}" }
    val mpi = readMpi(body, offset)
    val raw =
        when (mpi.bytes.size) {
            33 -> mpi.bytes.copyOfRange(1, 33)
            32 -> mpi.bytes
            else -> error("unexpected Ed25519 public MPI length ${mpi.bytes.size}")
        }
    return PublicMaterial(raw, mpi.next)
}

private data class Mpi(val bytes: ByteArray, val next: Int)

private fun readMpi(data: ByteArray, offset: Int): Mpi {
    require(offset + 2 <= data.size) { "truncated MPI length" }
    val bits = ((data[offset].toInt() and 0xff) shl 8) or (data[offset + 1].toInt() and 0xff)
    val length = (bits + 7) / 8
    val end = offset + 2 + length
    require(end <= data.size) { "truncated MPI payload" }
    return Mpi(data.copyOfRange(offset + 2, end), end)
}

private fun fingerprint(pubKeyBody: ByteArray): String {
    val digest = MessageDigest.getInstance("SHA-1")
    digest.update(0x99.toByte())
    digest.update(((pubKeyBody.size ushr 8) and 0xff).toByte())
    digest.update((pubKeyBody.size and 0xff).toByte())
    digest.update(pubKeyBody)
    return hex(digest.digest()).uppercase()
}
