// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import com.github.luben.zstd.Zstd
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.util.zip.GZIPInputStream
import java.util.zip.GZIPOutputStream

const val MAX_DECOMPRESSED_BYTES = 16 * 1024 * 1024

data class Codec(val name: String, val cls: String)

class CodecException(
    message: String,
    val reason: String,
    val failed: Boolean,
) : RuntimeException(message)

fun decodeChain(chain: List<Codec>, payload: ByteArray): ByteArray {
    var out = payload
    for (codec in chain.asReversed()) {
        out =
            when (codec.name) {
                "identity" -> out
                "gzip" -> gunzip(out)
                "zstd", "zstd-rsyncable" -> zstdDecompress(out)
                "cose-encrypt0" -> throw CodecException("missing content key for cose-encrypt0", "missing-key", false)
                else -> throw CodecException("unsupported codec ${codec.name}", "unknown-codec", false)
            }
    }
    return out
}

fun encodeChain(chain: List<Codec>, payload: ByteArray): ByteArray {
    var out = payload
    for (codec in chain) {
        out =
            when (codec.name) {
                "identity" -> out
                "gzip" -> gzip(out)
                "zstd", "zstd-rsyncable" -> Zstd.compress(out)
                else -> throw CodecException("unsupported codec ${codec.name}", "unknown-codec", true)
            }
    }
    return out
}

private fun gzip(data: ByteArray): ByteArray {
    val out = ByteArrayOutputStream()
    GZIPOutputStream(out).use { it.write(data) }
    return out.toByteArray()
}

private fun gunzip(data: ByteArray): ByteArray =
    GZIPInputStream(ByteArrayInputStream(data)).use { input ->
        val out = ByteArrayOutputStream()
        val buf = ByteArray(8192)
        while (true) {
            val n = input.read(buf)
            if (n < 0) break
            out.write(buf, 0, n)
            if (out.size() > MAX_DECOMPRESSED_BYTES) {
                throw CodecException("decoded payload exceeds safety limit", "damaged", true)
            }
        }
        out.toByteArray()
    }

private fun zstdDecompress(data: ByteArray): ByteArray {
    val declared = Zstd.decompressedSize(data)
    if (declared > MAX_DECOMPRESSED_BYTES) {
        throw CodecException("decoded payload exceeds safety limit", "damaged", true)
    }
    if (declared > 0) {
        val out = Zstd.decompress(data, declared.toInt())
        if (Zstd.isError(out.size.toLong())) {
            throw CodecException("zstd decode failed", "damaged", true)
        }
        return out
    }
    val out = Zstd.decompress(data, MAX_DECOMPRESSED_BYTES)
    if (Zstd.isError(out.size.toLong())) {
        throw CodecException("zstd decode failed", "damaged", true)
    }
    return out
}
