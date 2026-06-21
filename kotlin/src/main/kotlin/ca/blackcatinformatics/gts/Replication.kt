// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

data class ByteRange(val start: Int, val end: Int)

data class FrameInventory(
    val itemIndex: Int,
    val frameIndex: Int,
    val start: Int,
    val end: Int,
    val id: ByteArray,
    val frameType: String,
    val valid: Boolean,
)

data class SegmentInventory(
    val index: Int,
    val itemStart: Int,
    val itemEnd: Int,
    val start: Int,
    val end: Int,
    val profile: String,
    val head: ByteArray?,
    val frameCount: Int,
    val layout: StreamableInfo,
    val diagnostics: List<Diagnostic>,
    val frames: List<FrameInventory>,
)

data class Inventory(
    val segments: List<SegmentInventory>,
    val fatal: Diagnostic?,
    val torn: Int,
    val cleanEnd: Int,
    val itemCount: Int,
) {
    fun hasProblems(): Boolean = fatal != null || torn >= 0 || segments.any { it.diagnostics.isNotEmpty() }

    fun problemDetail(): String =
        when {
            fatal != null -> "${fatal.code}: ${fatal.detail}"
            torn >= 0 -> "torn at offset $torn"
            else -> segments.firstOrNull { it.diagnostics.isNotEmpty() }?.diagnostics?.firstOrNull()?.let { "${it.code}: ${it.detail}" }.orEmpty()
        }
}

data class MissingResult(
    val status: String,
    val fromHead: ByteArray,
    val ranges: List<ByteRange>,
    val scanRequired: Boolean,
    val detail: String = "",
)

fun inventoryFor(data: ByteArray): Inventory {
    val (items, torn) = iterItems(data)
    val cleanEnd = if (torn >= 0) torn else data.size
    val fileSegments = readFileSegments(data)
    if (items.isEmpty() || fileSegments.fatal != null) {
        return Inventory(emptyList(), fileSegments.fatal, torn, cleanEnd, items.size)
    }
    val bounds = items.indices.filter { isReplicationHeader(items[it].item) }
    if (bounds.isEmpty() || bounds.first() != 0) {
        return Inventory(emptyList(), fileSegments.fatal, torn, cleanEnd, items.size)
    }
    val segments = mutableListOf<SegmentInventory>()
    for ((segmentIndex, startItem) in bounds.withIndex()) {
        val endItem = if (segmentIndex + 1 < bounds.size) bounds[segmentIndex + 1] else items.size
        val graph = fileSegments.segments.getOrNull(segmentIndex) ?: break
        val start = items[startItem].offset
        val end = if (endItem < items.size) items[endItem].offset else cleanEnd
        segments +=
            SegmentInventory(
                index = segmentIndex,
                itemStart = startItem,
                itemEnd = endItem,
                start = start,
                end = end,
                profile = graph.segmentProfiles.firstOrNull() ?: headerProfile(items[startItem].item),
                head = graph.segmentHeads.firstOrNull(),
                frameCount = endItem - startItem - 1,
                layout = graph.segmentStreamable.firstOrNull() ?: StreamableInfo(),
                diagnostics = graph.diagnostics.toList(),
                frames = collectFrames(items, torn, data.size, startItem, endItem),
            )
    }
    return Inventory(segments, fileSegments.fatal, torn, cleanEnd, items.size)
}

fun headsJson(inventory: Inventory): String {
    val heads = inventory.segments.mapNotNull { it.head?.let(::hex) }
    val fileHead = inventory.segments.lastOrNull()?.head
    return "{" +
        "\"schema\":\"gts-replication-heads-v1\"," +
        "\"clean\":${!inventory.hasProblems()}," +
        "\"segment_heads\":[${heads.joinToString(",") { "\"$it\"" }}]," +
        "\"aggregate\":{\"schema\":\"gts-segment-heads-v1\",\"count\":${heads.size},\"digest\":\"${hex(aggregateDigest(inventory))}\",\"file_head\":${jsonNullableHex(fileHead)}}," +
        "\"torn_at\":${jsonNullableInt(inventory.torn)}," +
        "\"fatal\":${diagnosticJsonNullable(inventory.fatal)}" +
        "}\n"
}

fun segmentsJson(inventory: Inventory): String =
    "{" +
        "\"schema\":\"gts-replication-segments-v1\"," +
        "\"clean\":${!inventory.hasProblems()}," +
        "\"segments\":[${inventory.segments.joinToString(",") { segmentJson(it) }}]," +
        "\"item_count\":${inventory.itemCount}," +
        "\"torn_at\":${jsonNullableInt(inventory.torn)}," +
        "\"fatal\":${diagnosticJsonNullable(inventory.fatal)}" +
        "}\n"

fun missing(inventory: Inventory, fromHead: ByteArray): MissingResult {
    if (inventory.hasProblems()) {
        return MissingResult("error", fromHead, emptyList(), false, inventory.problemDetail())
    }
    for (segment in inventory.segments) {
        if (segment.head?.contentEquals(fromHead) == true) {
            val ranges = if (segment.end < inventory.cleanEnd) listOf(ByteRange(segment.end, inventory.cleanEnd)) else emptyList()
            return MissingResult(if (ranges.isEmpty()) "complete" else "ranges", fromHead, ranges, false)
        }
        for (frame in segment.frames) {
            if (frame.valid && frame.id.contentEquals(fromHead)) {
                val ranges = if (frame.end < inventory.cleanEnd) listOf(ByteRange(frame.end, inventory.cleanEnd)) else emptyList()
                return MissingResult(if (ranges.isEmpty()) "complete" else "ranges", fromHead, ranges, false)
            }
        }
    }
    return MissingResult("unknown", fromHead, emptyList(), true, "unknown peer head; scan required")
}

fun missingJson(result: MissingResult): String =
    "{" +
        "\"schema\":\"gts-replication-missing-v1\"," +
        "\"status\":\"${result.status}\"," +
        "\"from_head\":\"${hex(result.fromHead)}\"," +
        "\"ranges\":[${result.ranges.joinToString(",") { rangeJson(it) }}]," +
        "\"scan_required\":${result.scanRequired}," +
        "\"detail\":${if (result.detail.isEmpty()) "null" else "\"${json(result.detail)}\""}" +
        "}\n"

fun resumeAfter(data: ByteArray, frameId: ByteArray): ByteArray {
    val inventory = inventoryFor(data)
    if (inventory.hasProblems()) error(inventory.problemDetail().ifEmpty { "input is not clean" })
    for (segment in inventory.segments) {
        for (frame in segment.frames) {
            if (frame.valid && frame.id.contentEquals(frameId)) return data.copyOfRange(frame.end, inventory.cleanEnd)
        }
    }
    error("frame ${hex(frameId)} not found")
}

private fun collectFrames(
    items: List<CborItem>,
    torn: Int,
    dataLen: Int,
    start: Int,
    end: Int,
): List<FrameInventory> {
    var expectedPrev = headerStoredId(items[start].item) ?: headerComputedId(items[start].item) ?: ByteArray(0)
    val frames = mutableListOf<FrameInventory>()
    for (itemIndex in start + 1 until end) {
        val itemStart = items[itemIndex].offset
        val itemStop = itemEnd(items, torn, dataLen, itemIndex)
        val frameIndex = itemIndex - start - 1
        val frame = items[itemIndex].item as? CborMap
        if (frame == null) {
            frames += FrameInventory(itemIndex, frameIndex, itemStart, itemStop, ByteArray(0), "<non-map>", false)
            continue
        }
        val computed = contentId(frame)
        val stored = frame.getTextKey("id").asBytes()
        val frameId = stored ?: computed
        val prev = frame.getTextKey("prev").asBytes()
        frames +=
            FrameInventory(
                itemIndex,
                frameIndex,
                itemStart,
                itemStop,
                frameId,
                frame.getTextKey("t").asText() ?: "<unknown>",
                stored != null && stored.contentEquals(computed) && prev != null && prev.contentEquals(expectedPrev),
            )
        expectedPrev = frameId
    }
    return frames
}

private fun isReplicationHeader(item: CborValue): Boolean {
    val inner = if (item is CborTag) item.value else item
    val map = inner as? CborMap ?: return false
    return map.getTextKey("gts") != null && map.getTextKey("t") == null
}

private fun headerProfile(item: CborValue): String =
    try {
        unwrapHeader(item).getTextKey("prof").asText() ?: "generic"
    } catch (_: RuntimeException) {
        "generic"
    }

private fun headerStoredId(item: CborValue): ByteArray? =
    try {
        unwrapHeader(item).getTextKey("id").asBytes()
    } catch (_: RuntimeException) {
        null
    }

private fun headerComputedId(item: CborValue): ByteArray? =
    try {
        headerId(unwrapHeader(item))
    } catch (_: RuntimeException) {
        null
    }

private fun itemEnd(items: List<CborItem>, torn: Int, dataLen: Int, index: Int): Int =
    when {
        index + 1 < items.size -> items[index + 1].offset
        torn >= 0 -> torn
        else -> dataLen
    }

private fun aggregateDigest(inventory: Inventory): ByteArray =
    blake3_256(
        encode(
            cborArray(
                text("gts-segment-heads-v1"),
                CborArray(inventory.segments.mapNotNull { it.head?.let(::bytes) }),
            ),
        ),
    )

private fun segmentJson(segment: SegmentInventory): String =
    "{" +
        "\"index\":${segment.index}," +
        "\"byte_range\":${rangeJson(ByteRange(segment.start, segment.end))}," +
        "\"item_range\":{\"start\":${segment.itemStart},\"end\":${segment.itemEnd}}," +
        "\"profile\":\"${json(segment.profile)}\"," +
        "\"head\":${jsonNullableHex(segment.head)}," +
        "\"frame_count\":${segment.frameCount}," +
        "\"layout\":${layoutJson(segment.layout)}," +
        "\"diagnostics\":[${segment.diagnostics.joinToString(",") { diagnosticJson(it) }}]" +
        "}"

private fun rangeJson(range: ByteRange): String = "{\"start\":${range.start},\"end\":${range.end},\"length\":${(range.end - range.start).coerceAtLeast(0)}}"

private fun layoutJson(layout: StreamableInfo): String =
    "{\"claimed\":${layout.claimed},\"covered\":${layout.covered},\"tail\":${layout.tail},\"head\":${jsonNullableHex(layout.head)}}"

private fun diagnosticJson(diagnostic: Diagnostic): String =
    "{\"code\":\"${json(diagnostic.code)}\",\"detail\":\"${json(diagnostic.detail)}\",\"frame_index\":${diagnostic.frameIndex?.toString() ?: "null"}}"

private fun diagnosticJsonNullable(diagnostic: Diagnostic?): String = diagnostic?.let(::diagnosticJson) ?: "null"

private fun jsonNullableHex(bytes: ByteArray?): String = bytes?.let { "\"${hex(it)}\"" } ?: "null"

private fun jsonNullableInt(value: Int): String = if (value >= 0) value.toString() else "null"
