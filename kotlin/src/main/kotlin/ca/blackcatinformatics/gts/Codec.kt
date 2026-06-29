// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import com.github.luben.zstd.Zstd
import com.github.luben.zstd.ZstdInputStream
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.util.zip.GZIPInputStream
import java.util.zip.GZIPOutputStream

private const val DEFAULT_ZSTD_STREAM_CAPACITY = 64 * 1024
private const val MAX_ZSTD_PREALLOCATED_BYTES = 16 * 1024 * 1024

data class Codec(val name: String, val cls: String)

class CodecException(
    message: String,
    val reason: String,
    val failed: Boolean,
    cause: Throwable? = null,
) : RuntimeException(message, cause)

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
    try {
        GZIPInputStream(ByteArrayInputStream(data)).use { input ->
            val out = ByteArrayOutputStream()
            val buf = ByteArray(8192)
            while (true) {
                val n = input.read(buf)
                if (n < 0) break
                out.write(buf, 0, n)
            }
            out.toByteArray()
        }
    } catch (err: IOException) {
        throw CodecException("gzip decode failed: ${err.message}", "damaged", true, err)
    }

private fun zstdDecompress(data: ByteArray): ByteArray =
    try {
        ZstdInputStream(ByteArrayInputStream(data)).use { input ->
            val declared = Zstd.getFrameContentSize(data)
            val initialCapacity =
                if (declared in 1..MAX_ZSTD_PREALLOCATED_BYTES.toLong()) {
                    declared.toInt()
                } else {
                    DEFAULT_ZSTD_STREAM_CAPACITY
                }
            val out = ByteArrayOutputStream(initialCapacity)
            val buf = ByteArray(8192)
            while (true) {
                val n = input.read(buf)
                if (n < 0) break
                out.write(buf, 0, n)
            }
            out.toByteArray()
        }
    } catch (err: IOException) {
        throw CodecException("zstd decode failed: ${err.message}", "damaged", true, err)
    }
