// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

private const val PROOF_SCHEMA = "gts-mmr-proof-v1"
private const val HASH_ALGORITHM = "blake3-256"
private const val PREIMAGE_VERSION = "gts-mmr-v1"
private const val LEAF_DOMAIN = "gts-mmr-leaf-v1"
private const val PARENT_DOMAIN = "gts-mmr-parent-v1"
private const val ROOT_DOMAIN = "gts-mmr-root-v1"

data class Peak(val height: Int, val hash: ByteArray) {
    override fun equals(other: Any?): Boolean = other is Peak && height == other.height && hash.contentEquals(other.hash)

    override fun hashCode(): Int = 31 * height + hash.contentHashCode()
}

data class Step(val parentHeight: Int, val side: String, val hash: ByteArray) {
    override fun equals(other: Any?): Boolean =
        other is Step && parentHeight == other.parentHeight && side == other.side && hash.contentEquals(other.hash)

    override fun hashCode(): Int = ((31 * parentHeight) + side.hashCode()) * 31 + hash.contentHashCode()
}

data class Proof(
    val count: Int,
    val leafIndex: Int,
    val frameId: ByteArray,
    val root: ByteArray,
    val peakIndex: Int,
    val peaks: List<Peak>,
    val path: List<Step>,
) {
    override fun equals(other: Any?): Boolean =
        other is Proof &&
            count == other.count &&
            leafIndex == other.leafIndex &&
            frameId.contentEquals(other.frameId) &&
            root.contentEquals(other.root) &&
            peakIndex == other.peakIndex &&
            peaks == other.peaks &&
            path == other.path

    override fun hashCode(): Int {
        var result = count
        result = 31 * result + leafIndex
        result = 31 * result + frameId.contentHashCode()
        result = 31 * result + root.contentHashCode()
        result = 31 * result + peakIndex
        result = 31 * result + peaks.hashCode()
        result = 31 * result + path.hashCode()
        return result
    }
}

fun parseHex32(input: String): ByteArray {
    val raw = input.trim().removePrefix("blake3:")
    require(raw.length == 64) { "expected a 32-byte hex value" }
    return try {
        parseHex(raw)
    } catch (_: RuntimeException) {
        throw IllegalArgumentException("hex value contains a non-hex character")
    }.also {
        require(it.size == 32) { "expected a 32-byte hex value" }
    }
}

fun proofFromJson(data: ByteArray): Proof {
    val root = Json.parseToJsonElement(data.decodeToString()).jsonObject
    val schema = requiredString(root, "schema")
    require(schema == PROOF_SCHEMA) { "unsupported proof schema \"$schema\"" }
    val hash = requiredString(root, "hash")
    require(hash == HASH_ALGORITHM) { "unsupported hash algorithm \"$hash\"" }
    val preimage = requiredString(root, "preimage")
    require(preimage == PREIMAGE_VERSION) { "unsupported preimage version \"$preimage\"" }
    val count = requiredInt(root, "count")
    val leafIndex = requiredInt(root, "leaf_index")
    val peakIndex = requiredInt(root, "peak_index")
    val frameId = parseHex32(requiredString(root, "frame_id"))
    val proofRoot = parseHex32(requiredString(root, "root"))
    val peaks =
        requiredArray(root, "peaks").map { value ->
            val obj = value.jsonObject
            Peak(requiredInt(obj, "height"), parseHex32(requiredString(obj, "hash")))
        }
    val path =
        requiredArray(root, "path").map { value ->
            val obj = value.jsonObject
            val side = requiredString(obj, "side")
            require(side == "left" || side == "right") { "unsupported proof side \"$side\"" }
            Step(requiredInt(obj, "parent_height"), side, parseHex32(requiredString(obj, "hash")))
        }
    return Proof(count, leafIndex, frameId, proofRoot, peakIndex, peaks, path)
}

fun verifyProof(proof: Proof) {
    require(proof.frameId.size == 32) { "frame_id must be 32 bytes" }
    require(proof.root.size == 32) { "root must be 32 bytes" }
    require(proof.leafIndex < proof.count) {
        "leaf_index ${proof.leafIndex} is outside covered count ${proof.count}"
    }
    require(proof.peakIndex < proof.peaks.size) { "peak_index ${proof.peakIndex} is out of range" }
    val expectedHeights = expectedPeakHeights(proof.count)
    val actualHeights = proof.peaks.map { it.height }
    require(actualHeights == expectedHeights) {
        "peak heights $actualHeights do not match count ${proof.count}"
    }
    val computedPeakIndex = peakIndexForLeaf(proof.count, actualHeights, proof.leafIndex)
    require(computedPeakIndex == proof.peakIndex) {
        "leaf_index ${proof.leafIndex} belongs to peak $computedPeakIndex, not ${proof.peakIndex}"
    }
    for (peak in proof.peaks) require(peak.hash.size == 32) { "peak hash must be 32 bytes" }

    var carried = leafHash(proof.leafIndex, proof.frameId)
    var height = 0
    for (step in proof.path) {
        require(step.hash.size == 32) { "path hash must be 32 bytes" }
        require(step.parentHeight == height + 1) {
            "path parent height ${step.parentHeight} does not follow height $height"
        }
        carried =
            when (step.side) {
                "left" -> parentHash(step.parentHeight, step.hash, carried)
                "right" -> parentHash(step.parentHeight, carried, step.hash)
                else -> error("unsupported proof side \"${step.side}\"")
            }
        height = step.parentHeight
    }

    val peak = proof.peaks[proof.peakIndex]
    require(height == peak.height) { "path height $height does not reach peak height ${peak.height}" }
    require(carried.contentEquals(peak.hash)) { "proof path does not reconstruct the selected peak" }
    require(rootHash(proof.count, proof.peaks).contentEquals(proof.root)) {
        "proof peaks do not reconstruct the declared root"
    }
}

private fun leafHash(index: Int, frameId: ByteArray): ByteArray =
    blake3_256(encode(cborArray(text(LEAF_DOMAIN), uint(index), bytes(frameId))))

private fun parentHash(parentHeight: Int, left: ByteArray, right: ByteArray): ByteArray =
    blake3_256(encode(cborArray(text(PARENT_DOMAIN), uint(parentHeight), bytes(left), bytes(right))))

private fun rootHash(count: Int, peaks: List<Peak>): ByteArray =
    blake3_256(
        encode(
            cborArray(
                text(ROOT_DOMAIN),
                uint(count),
                CborArray(peaks.map { cborArray(uint(it.height), bytes(it.hash)) }),
            ),
        ),
    )

private fun expectedPeakHeights(count: Int): List<Int> {
    val heights = mutableListOf<Int>()
    var remaining = count
    while (remaining > 0) {
        val height = Int.SIZE_BITS - Integer.numberOfLeadingZeros(remaining) - 1
        heights += height
        remaining -= 1 shl height
    }
    return heights
}

private fun peakIndexForLeaf(count: Int, heights: List<Int>, leafIndex: Int): Int {
    require(leafIndex < count) { "leaf_index $leafIndex is outside covered count $count" }
    var start = 0
    for ((index, height) in heights.withIndex()) {
        val end = start + (1 shl height)
        if (leafIndex in start until end) return index
        start = end
    }
    error("peak ranges do not cover leaf_index $leafIndex for count $count")
}

private fun requiredString(obj: JsonObject, key: String): String =
    (obj[key] as? JsonPrimitive)?.takeIf { it.isString }?.content
        ?: throw IllegalArgumentException("\"$key\" must be a string")

private fun requiredInt(obj: JsonObject, key: String): Int =
    obj[key]?.jsonPrimitive?.intOrNull?.takeIf { it >= 0 }
        ?: throw IllegalArgumentException("\"$key\" must be an unsigned integer")

private fun requiredArray(obj: JsonObject, key: String): JsonArray =
    (obj[key] as? JsonArray) ?: throw IllegalArgumentException("\"$key\" must be a JSON array")
