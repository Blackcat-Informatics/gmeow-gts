// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import java.io.ByteArrayOutputStream
import java.nio.charset.StandardCharsets
import org.bouncycastle.crypto.digests.Blake3Digest

const val SELF_DESCRIBE_TAG: Long = 55799L
const val MAGIC = "GTS1"
const val VERSION = 1

data class ByteString(val bytes: ByteArray) {
    override fun equals(other: Any?): Boolean = other is ByteString && bytes.contentEquals(other.bytes)

    override fun hashCode(): Int = bytes.contentHashCode()

    override fun toString(): String = "h'${hex(bytes)}'"
}

sealed interface CborValue

data class CborUInt(val value: Long) : CborValue

data class CborNInt(val value: Long) : CborValue

data class CborBytes(val value: ByteString) : CborValue

data class CborText(val value: String) : CborValue

data class CborArray(val value: List<CborValue>) : CborValue

data class CborMap(val value: List<Pair<CborValue, CborValue>>) : CborValue

data class CborTag(val tag: Long, val value: CborValue) : CborValue

data class CborBool(val value: Boolean) : CborValue

data object CborNull : CborValue

data object CborUndefined : CborValue

data class CborItem(val offset: Int, val item: CborValue)

class CborDecodeException(message: String) : RuntimeException(message)

fun bytes(data: ByteArray): CborBytes = CborBytes(ByteString(data.copyOf()))

fun text(value: String): CborText = CborText(value)

fun uint(value: Int): CborUInt = CborUInt(value.toLong())

fun uint(value: Long): CborUInt = CborUInt(value)

fun cborArray(vararg values: CborValue): CborArray = CborArray(values.toList())

fun cborMap(vararg entries: Pair<CborValue, CborValue>): CborMap = CborMap(entries.toList())

fun encode(value: CborValue): ByteArray {
    val out = ByteArrayOutputStream()
    encodeInto(out, value)
    return out.toByteArray()
}

fun blake3_256(data: ByteArray): ByteArray {
    val digest = Blake3Digest(256)
    digest.update(data, 0, data.size)
    val out = ByteArray(32)
    digest.doFinal(out, 0)
    return out
}

fun hex(data: ByteArray): String = data.joinToString(separator = "") { "%02x".format(it.toInt() and 0xff) }

fun parseHex(text: String): ByteArray {
    val s = text.removePrefix("blake3:")
    require(s.length % 2 == 0) { "hex string must have even length" }
    return ByteArray(s.length / 2) { i ->
        s.substring(i * 2, i * 2 + 2).toInt(16).toByte()
    }
}

fun digestStr(data: ByteArray): String = "blake3:${hex(blake3_256(data))}"

fun normalizeDigest(digest: String): String = if (digest.startsWith("blake3:")) digest else "blake3:$digest"

fun contentId(frame: CborMap): ByteArray = hashExcluding(frame, setOf("id", "sig"))

fun headerId(header: CborMap): ByteArray = hashExcluding(header, setOf("id"))

private fun hashExcluding(map: CborMap, excluded: Set<String>): ByteArray {
    val filtered = CborMap(
        map.value.filterNot { (key, _) ->
            key is CborText && key.value in excluded
        },
    )
    return blake3_256(encode(filtered))
}

private fun encodeInto(out: ByteArrayOutputStream, value: CborValue) {
    when (value) {
        is CborUInt -> encodeMajor(out, 0, value.value)
        is CborNInt -> encodeMajor(out, 1, -1L - value.value)
        is CborBytes -> {
            encodeMajor(out, 2, value.value.bytes.size.toLong())
            out.write(value.value.bytes)
        }
        is CborText -> {
            val encoded = value.value.toByteArray(StandardCharsets.UTF_8)
            encodeMajor(out, 3, encoded.size.toLong())
            out.write(encoded)
        }
        is CborArray -> {
            encodeMajor(out, 4, value.value.size.toLong())
            value.value.forEach { encodeInto(out, it) }
        }
        is CborMap -> {
            val encoded = value.value.map { (key, entryValue) ->
                val keyBytes = encode(key)
                EncodedMapEntry(keyBytes, encode(entryValue))
            }.sortedWith { a, b -> compareCborKeys(a.key, b.key) }
            encodeMajor(out, 5, encoded.size.toLong())
            encoded.forEach {
                out.write(it.key)
                out.write(it.value)
            }
        }
        is CborTag -> {
            encodeMajor(out, 6, value.tag)
            encodeInto(out, value.value)
        }
        is CborBool -> out.write(if (value.value) 0xf5 else 0xf4)
        CborNull -> out.write(0xf6)
        CborUndefined -> out.write(0xf7)
    }
}

private data class EncodedMapEntry(val key: ByteArray, val value: ByteArray)

private fun compareCborKeys(a: ByteArray, b: ByteArray): Int {
    if (a.size != b.size) return a.size - b.size
    for (i in a.indices) {
        val diff = (a[i].toInt() and 0xff) - (b[i].toInt() and 0xff)
        if (diff != 0) return diff
    }
    return 0
}

private fun encodeMajor(out: ByteArrayOutputStream, major: Int, value: Long) {
    require(value >= 0) { "negative CBOR length" }
    val prefix = major shl 5
    when {
        value <= 23L -> out.write(prefix or value.toInt())
        value <= 0xffL -> {
            out.write(prefix or 24)
            out.write(value.toInt())
        }
        value <= 0xffffL -> {
            out.write(prefix or 25)
            out.write(((value ushr 8) and 0xff).toInt())
            out.write((value and 0xff).toInt())
        }
        value <= 0xffffffffL -> {
            out.write(prefix or 26)
            for (shift in listOf(24, 16, 8, 0)) out.write(((value ushr shift) and 0xff).toInt())
        }
        else -> {
            out.write(prefix or 27)
            for (shift in listOf(56, 48, 40, 32, 24, 16, 8, 0)) {
                out.write(((value ushr shift) and 0xff).toInt())
            }
        }
    }
}

fun iterItems(data: ByteArray): Pair<List<CborItem>, Int> {
    val out = mutableListOf<CborItem>()
    var offset = 0
    while (offset < data.size) {
        val start = offset
        val length =
            try {
                cborItemLength(data, offset)
            } catch (_: RuntimeException) {
                return out to start
            }
        val decoder = CborDecoder(data, start, start + length)
        val item =
            try {
                decoder.decode()
            } catch (_: RuntimeException) {
                return out to start
            }
        out += CborItem(start, item)
        offset += length
    }
    return out to -1
}

fun decode(data: ByteArray): CborValue {
    val decoder = CborDecoder(data, 0, data.size)
    val value = decoder.decode()
    if (decoder.position != data.size) throw CborDecodeException("trailing CBOR bytes")
    return value
}

private class CborDecoder(
    private val data: ByteArray,
    start: Int,
    private val end: Int,
) {
    var position: Int = start
        private set

    fun decode(): CborValue {
        if (position >= end) throw CborDecodeException("unexpected EOF")
        val initial = readByte()
        val major = initial ushr 5
        val info = initial and 0x1f
        return when (major) {
            0 -> CborUInt(readArgument(info))
            1 -> CborNInt(-1L - readArgument(info))
            2 -> CborBytes(ByteString(readDefiniteBytes(info)))
            3 -> CborText(readDefiniteBytes(info).toString(StandardCharsets.UTF_8))
            4 -> {
                val n = readArgument(info).toIntExact()
                CborArray((0 until n).map { decode() })
            }
            5 -> {
                val n = readArgument(info).toIntExact()
                CborMap((0 until n).map { decode() to decode() })
            }
            6 -> CborTag(readArgument(info), decode())
            7 -> decodeSimple(info)
            else -> throw CborDecodeException("unsupported CBOR major type $major")
        }
    }

    private fun decodeSimple(info: Int): CborValue =
        when (info) {
            20 -> CborBool(false)
            21 -> CborBool(true)
            22 -> CborNull
            23 -> CborUndefined
            else -> throw CborDecodeException("unsupported CBOR simple value $info")
        }

    private fun readDefiniteBytes(info: Int): ByteArray {
        if (info == 31) throw CborDecodeException("indefinite strings are not supported")
        val n = readArgument(info).toIntExact()
        if (position + n > end) throw CborDecodeException("unexpected EOF")
        val out = data.copyOfRange(position, position + n)
        position += n
        return out
    }

    private fun readArgument(info: Int): Long =
        when {
            info <= 23 -> info.toLong()
            info == 24 -> readByte().toLong()
            info == 25 -> (readByte().toLong() shl 8) or readByte().toLong()
            info == 26 -> {
                var n = 0L
                repeat(4) { n = (n shl 8) or readByte().toLong() }
                n
            }
            info == 27 -> {
                var n = 0L
                repeat(8) { n = (n shl 8) or readByte().toLong() }
                n
            }
            else -> throw CborDecodeException("unsupported additional info $info")
        }

    private fun readByte(): Int {
        if (position >= end) throw CborDecodeException("unexpected EOF")
        return data[position++].toInt() and 0xff
    }
}

private fun Long.toIntExact(): Int {
    if (this < 0 || this > Int.MAX_VALUE) throw CborDecodeException("CBOR length exceeds JVM int range")
    return toInt()
}

private fun cborItemLength(data: ByteArray, offset: Int): Int {
    if (offset >= data.size) throw CborDecodeException("EOF")
    val start = offset
    var pos = offset
    val stack = ArrayDeque<Long>()

    fun complete() {
        while (stack.isNotEmpty()) {
            val remaining = stack.removeLast() - 1
            if (remaining > 0) {
                stack.addLast(remaining)
                break
            }
        }
    }

    while (true) {
        if (pos >= data.size) throw CborDecodeException("unexpected EOF")
        val b = data[pos++].toInt() and 0xff
        val major = b ushr 5
        val info = b and 0x1f
        if (info == 31) throw CborDecodeException("indefinite CBOR item not supported")
        val length = readLengthForScan(data, pos, info)
        pos += length.second
        when (major) {
            0, 1, 7 -> complete()
            2, 3 -> {
                if (data.size - pos < length.first) throw CborDecodeException("unexpected EOF")
                pos += length.first.toIntExact()
                complete()
            }
            4 -> if (length.first == 0L) complete() else stack.addLast(length.first)
            5 -> if (length.first == 0L) complete() else stack.addLast(length.first * 2)
            6 -> stack.addLast(1)
            else -> throw CborDecodeException("unsupported CBOR major type $major")
        }
        if (stack.isEmpty()) return pos - start
    }
}

private fun readLengthForScan(data: ByteArray, offset: Int, info: Int): Pair<Long, Int> =
    when {
        info <= 23 -> info.toLong() to 0
        info == 24 -> {
            if (offset >= data.size) throw CborDecodeException("unexpected EOF")
            (data[offset].toInt() and 0xff).toLong() to 1
        }
        info == 25 -> {
            if (offset + 2 > data.size) throw CborDecodeException("unexpected EOF")
            (((data[offset].toInt() and 0xff) shl 8) or (data[offset + 1].toInt() and 0xff)).toLong() to 2
        }
        info == 26 -> {
            if (offset + 4 > data.size) throw CborDecodeException("unexpected EOF")
            var n = 0L
            for (i in 0 until 4) n = (n shl 8) or (data[offset + i].toLong() and 0xff)
            n to 4
        }
        info == 27 -> {
            if (offset + 8 > data.size) throw CborDecodeException("unexpected EOF")
            var n = 0L
            for (i in 0 until 8) n = (n shl 8) or (data[offset + i].toLong() and 0xff)
            n to 8
        }
        else -> throw CborDecodeException("reserved CBOR additional info $info")
    }

fun CborMap.getTextKey(key: String): CborValue? =
    value.firstOrNull { (k, _) -> k is CborText && k.value == key }?.second

fun CborValue?.asText(): String? = (this as? CborText)?.value

fun CborValue?.asBytes(): ByteArray? = (this as? CborBytes)?.value?.bytes?.copyOf()

fun CborValue?.asInt(): Int? =
    when (this) {
        is CborUInt -> if (value <= Int.MAX_VALUE) value.toInt() else null
        is CborNInt -> if (value >= Int.MIN_VALUE && value <= Int.MAX_VALUE) value.toInt() else null
        else -> null
    }

fun CborValue?.asNonNegativeInt(): Int? = (this as? CborUInt)?.value?.takeIf { it <= Int.MAX_VALUE }?.toInt()

fun unwrapHeader(item: CborValue): CborMap {
    val inner =
        when (item) {
            is CborTag -> {
                if (item.tag != SELF_DESCRIBE_TAG) throw CborDecodeException("unexpected CBOR tag ${item.tag} on header")
                item.value
            }
            else -> item
        }
    return inner as? CborMap ?: throw CborDecodeException("header item is not a CBOR map")
}
