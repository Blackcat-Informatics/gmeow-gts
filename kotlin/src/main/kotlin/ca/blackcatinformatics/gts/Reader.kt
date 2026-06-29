// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

private const val STREAM_DIGEST = "https://w3id.org/gts/stream#digest"

data class FileSegments(
    val segments: List<Graph>,
    val torn: Int,
    val fatal: Diagnostic? = null,
)

private data class PayloadError(
    val unavailable: Boolean,
    val reason: String,
    val detail: String,
    val damaged: Boolean,
)

private data class IndexRecord(
    val pos: Int,
    val count: Int,
    val head: ByteArray,
)

private data class BlobEvent(
    val pos: Int,
    val digest: String,
    val described: Boolean,
)

private class Folder(
    private val graph: Graph,
    private val catalog: Map<Long, Codec>,
) {
    val indexRecords: MutableList<IndexRecord> = mutableListOf()
    val described: MutableSet<String> = mutableSetOf()
    val blobEvents: MutableList<BlobEvent> = mutableListOf()

    fun diag(code: String, detail: String, index: Int? = null) {
        graph.diagnostics += Diagnostic(code, detail, index)
    }

    fun foldFrame(frame: CborMap, index: Int) {
        val frameType = frame.getTextKey("t").asText().orEmpty()
        val payload = payload(frame, frameType == "blob")
        if (payload.error != null) {
            val perr = payload.error
            if (perr.unavailable) {
                opaque(frame, frameType, perr.reason)
                diag(diagCodeFor(perr.reason), perr.detail, index)
            } else {
                opaque(frame, frameType, "damaged")
                diag("DamagedFrame", "payload decode failed: ${perr.detail}", index)
            }
            return
        }
        try {
            when (frameType) {
                "terms" -> hTerms(payload.value, index)
                "quads" -> hQuads(payload.value, index)
                "reifies" -> hReifies(payload.value, index)
                "annot" -> hAnnot(payload.value, index)
                "blob" -> hBlob(payload.value as? CborBytes, frame, index)
                "meta" -> hMeta(payload.value)
                "suppress" -> hSuppress(payload.value)
                "snapshot" -> hSnapshot(payload.value, index)
                "index" -> hIndex(payload.value, index)
                "opaque" -> hOpaque(payload.value)
                else -> {
                    opaque(frame, frameType, "unknown-frame-type")
                    diag("UnknownFrameType", "unsupported frame type '$frameType'", index)
                }
            }
        } catch (err: RuntimeException) {
            opaque(frame, frameType, "damaged")
            diag("DamagedFrame", "fold failed: ${err.message ?: err.toString()}", index)
        }
    }

    private data class PayloadResult(val value: CborValue?, val error: PayloadError? = null)

    private fun payload(frame: CborMap, isBlob: Boolean): PayloadResult {
        val d = frame.getTextKey("d")
        val x = frame.getTextKey("x")
        if (x is CborArray && x.value.isNotEmpty()) {
            val bytes = d.asBytes()
                ?: return PayloadResult(
                    null,
                    PayloadError(false, "", "transformed frame 'd' must be a byte string", true),
                )
            val chain = resolveCodecs(x.value)
            if (chain.error != null) return PayloadResult(null, chain.error)
            val decoded =
                try {
                    decodeChain(chain.codecs, bytes)
                } catch (err: CodecException) {
                    return PayloadResult(null, PayloadError(!err.failed, err.reason, err.message.orEmpty(), err.failed))
                } catch (err: Exception) {
                    return PayloadResult(null, PayloadError(false, "", err.message.orEmpty(), true))
                }
            if (isBlob) return PayloadResult(bytes(decoded))
            return try {
                PayloadResult(decode(decoded))
            } catch (err: RuntimeException) {
                PayloadResult(null, PayloadError(false, "", "payload decode failed: ${err.message}", true))
            }
        }
        return PayloadResult(d)
    }

    private data class CodecResolution(val codecs: List<Codec>, val error: PayloadError? = null)

    private fun resolveCodecs(ids: List<CborValue>): CodecResolution {
        val out = mutableListOf<Codec>()
        for (id in ids) {
            val n = id.asInt()?.toLong()
                ?: return CodecResolution(
                    emptyList(),
                    PayloadError(true, "unknown-codec", "codec id $id not an integer", false),
                )
            val codec = catalog[n]
                ?: return CodecResolution(
                    emptyList(),
                    PayloadError(true, "unknown-codec", "codec id $n not in catalog", false),
                )
            out += codec
        }
        return CodecResolution(out)
    }

    private fun hTerms(payload: CborValue?, index: Int) {
        val rows = (payload as? CborArray)?.value ?: return
        for (raw in rows) {
            val entries = raw as? CborMap ?: continue
            val kind = termKindFromWire(entries.getTextKey("k").asInt() ?: 0)
            val value = entries.getTextKey("v").asText().orEmpty()
            val lang = entries.getTextKey("l").asText()
            val direction = entries.getTextKey("dir").asText()?.takeIf { it == "ltr" || it == "rtl" }
            val termId = graph.terms.size
            fun sanitize(v: CborValue?): Int? {
                val n = v.asInt() ?: return null
                return if (n in 0 until termId) n else null
            }
            fun outOfRange(v: CborValue?): Boolean {
                val n = v.asInt() ?: return false
                return n !in 0 until termId
            }
            val datatype = sanitize(entries.getTextKey("dt"))
            val reifier = sanitize(entries.getTextKey("rf"))
            if (outOfRange(entries.getTextKey("dt")) || outOfRange(entries.getTextKey("rf"))) {
                diag("ForwardReference", "term $termId has an out-of-range ref", index)
            }
            graph.terms += Term(kind, value, datatype, lang, reifier, direction)
        }
    }

    private fun hQuads(payload: CborValue?, index: Int) {
        val rows = (payload as? CborArray)?.value ?: return
        for (row in rows) {
            val items = (row as? CborArray)?.value ?: continue
            if (items.size < 3) continue
            val s = items[0].asInt()
            val p = items[1].asInt()
            val o = items[2].asInt()
            val hasGraph = items.size >= 4
            val g = if (hasGraph) items[3].asInt() else null
            if (s == null || p == null || o == null || (hasGraph && g == null)) {
                diag("DamagedFrame", "quad has non-integer term ids", index)
                continue
            }
            if (!checkPositions(s, p, o, g, index)) continue
            graph.quads += Quad(s, p, o, g)
            if (graph.terms[p].value == STREAM_DIGEST) {
                val obj = graph.terms[o]
                if (obj.value.isNotEmpty()) described += obj.value
            }
        }
    }

    private fun hReifies(payload: CborValue?, index: Int) {
        val rows =
            (payload as? CborArray)?.value
                ?: run {
                    diag("DamagedFrame", "reifies payload must be a row array", index)
                    return
                }
        for (row in rows) {
            val items = (row as? CborArray)?.value ?: continue
            if (items.size != 4 && items.size != 5) continue
            val rid = items[0].asInt()
            val s = items[1].asInt()
            val p = items[2].asInt()
            val o = items[3].asInt()
            val hasGraph = items.size == 5
            val g = if (hasGraph) items[4].asInt() else null
            val n = graph.terms.size
            if (
                rid == null ||
                s == null ||
                p == null ||
                o == null ||
                (hasGraph && g == null) ||
                rid !in 0 until n ||
                s !in 0 until n ||
                p !in 0 until n ||
                o !in 0 until n ||
                (g != null && g !in 0 until n)
            ) {
                diag("DamagedFrame", "reifies row has bad/out-of-range ids", index)
                continue
            }
            val spo = Triple(s, p, o)
            val existing = graph.reifier(rid)
            if (existing != null && existing != spo) {
                diag("ConflictingReifier", "reifier $rid rebound", index)
                continue
            }
            if (!checkReifierPositions(s, p, o, g, index)) continue
            graph.setReifier(rid, spo, g)
        }
    }

    private fun hAnnot(payload: CborValue?, index: Int) {
        val rows = (payload as? CborArray)?.value ?: return
        for (row in rows) {
            val items = (row as? CborArray)?.value ?: continue
            if (items.size != 3 && items.size != 4) continue
            val r = items[0].asInt()
            val p = items[1].asInt()
            val v = items[2].asInt()
            val hasGraph = items.size == 4
            val g = if (hasGraph) items[3].asInt() else null
            val n = graph.terms.size
            if (
                r == null ||
                p == null ||
                v == null ||
                (hasGraph && g == null) ||
                r !in 0 until n ||
                p !in 0 until n ||
                v !in 0 until n ||
                (g != null && g !in 0 until n)
            ) {
                diag("DamagedFrame", "annot row has bad/out-of-range ids", index)
                continue
            }
            if (graph.terms[p].kind != TermKind.IRI) {
                diag("PositionConstraint", "annot predicate $p not an IRI", index)
                continue
            }
            if (g != null) {
                val graphKind = graph.terms[g].kind
                if (graphKind == TermKind.LITERAL || graphKind == TermKind.TRIPLE) {
                    diag("PositionConstraint", "annot graph name is not an IRI or blank node", index)
                    continue
                }
            }
            graph.annotations += AnnotationEntry(r, p, v, g)
        }
    }

    private fun hBlob(payload: CborBytes?, frame: CborMap, index: Int) {
        val data = payload?.value?.bytes ?: return
        val digest = digestStr(data)
        (frame.getTextKey("pub") as? CborMap)?.let { graph.setBlobMeta(digest, it) }
        graph.setBlob(digest, data)
        blobEvents += BlobEvent(index, digest, digest in described)
    }

    private fun hMeta(payload: CborValue?) {
        val entries = (payload as? CborMap)?.value ?: return
        for ((key, value) in entries) graph.setMeta(key.asText() ?: key.toString(), value)
    }

    private fun hSuppress(payload: CborValue?) {
        val entries = payload as? CborMap ?: return
        val targets = (entries.getTextKey("targets") as? CborArray)?.value?.filterIsInstance<CborMap>() ?: return
        val by = entries.getTextKey("by").asInt()
        graph.suppressions += Suppression(targets, entries.getTextKey("reason").asText().orEmpty(), by)
    }

    private fun hSnapshot(payload: CborValue?, index: Int) {
        val entries = payload as? CborMap ?: return
        val base = graph.terms.size
        fun shift(v: CborValue): CborValue = v.asInt()?.let { uint(it + base) } ?: v
        fun shiftRow(row: CborValue): CborValue {
            val items = (row as? CborArray)?.value ?: return row
            return CborArray(items.map { shift(it) })
        }
        (entries.getTextKey("terms") as? CborArray)?.let { terms ->
            hTerms(
                CborArray(
                    terms.value.map { raw ->
                        val m = raw as? CborMap ?: return@map raw
                        CborMap(
                            m.value.map { (k, v) ->
                                val key = k.asText()
                                k to if (key == "dt" || key == "rf") shift(v) else v
                            },
                        )
                    },
                ),
                index,
            )
        }
        (entries.getTextKey("quads") as? CborArray)?.let { hQuads(CborArray(it.value.map(::shiftRow)), index) }
        val reifies = entries.getTextKey("reifies")
        if (reifies is CborMap) {
            diag("DamagedFrame", "snapshot reifies payload must be a row array", index)
        }
        if (reifies is CborArray) {
            hReifies(CborArray(reifies.value.map(::shiftRow)), index)
        }
        (entries.getTextKey("annot") as? CborArray)?.let { hAnnot(CborArray(it.value.map(::shiftRow)), index) }
        (entries.getTextKey("blobs") as? CborMap)?.value?.forEach { (_, v) ->
            v.asBytes()?.let { graph.setBlob(digestStr(it), it) }
        }
        (entries.getTextKey("meta") as? CborMap)?.value?.forEach { (k, v) ->
            graph.setMeta(k.asText() ?: k.toString(), v)
        }
    }

    private fun hIndex(payload: CborValue?, index: Int) {
        val entries = payload as? CborMap ?: return
        val count = entries.getTextKey("count").asInt()
        val head = entries.getTextKey("head").asBytes()
        if (count != null && head != null) indexRecords += IndexRecord(index, count, head)
    }

    private fun hOpaque(payload: CborValue?) {
        val entries = payload as? CborMap ?: return
        graph.opaque +=
            OpaqueNode(
                id = entries.getTextKey("id").asBytes() ?: ByteArray(0),
                frameType = entries.getTextKey("type").asText() ?: "opaque",
                reason = entries.getTextKey("reason").asText() ?: "unknown-codec",
                sigStat = entries.getTextKey("sigstat").asText() ?: "none",
                pubMeta = entries.getTextKey("pub"),
            )
    }

    private fun checkPositions(s: Int, p: Int, o: Int, g: Int?, index: Int): Boolean {
        val n = graph.terms.size
        val inBounds = s in 0 until n && p in 0 until n && o in 0 until n && (g == null || g in 0 until n)
        if (!inBounds) {
            diag("PositionConstraint", "quad ($s,$p,$o,${g ?: "None"}) has out-of-range term ids", index)
            return false
        }
        var ok = graph.terms[p].kind == TermKind.IRI
        if (graph.terms[s].kind == TermKind.LITERAL) ok = false
        if (g != null) {
            val kind = graph.terms[g].kind
            if (kind == TermKind.LITERAL || kind == TermKind.TRIPLE) ok = false
        }
        if (!ok) diag("PositionConstraint", "quad ($s,$p,$o,${g ?: "None"}) violates positions", index)
        return ok
    }

    private fun checkReifierPositions(s: Int, p: Int, o: Int, g: Int?, index: Int): Boolean {
        val n = graph.terms.size
        val inBounds = s in 0 until n && p in 0 until n && o in 0 until n && (g == null || g in 0 until n)
        if (!inBounds) {
            diag("PositionConstraint", "reifier row has out-of-range term ids", index)
            return false
        }
        var ok = graph.terms[p].kind == TermKind.IRI
        if (graph.terms[s].kind == TermKind.LITERAL) ok = false
        if (g != null) {
            val kind = graph.terms[g].kind
            if (kind == TermKind.LITERAL || kind == TermKind.TRIPLE) ok = false
        }
        if (!ok) diag("PositionConstraint", "reifier row violates term positions", index)
        return ok
    }

    fun opaque(frame: CborMap, frameType: String, reason: String) {
        val recipients = (frame.getTextKey("to") as? CborArray)?.value?.filterIsInstance<CborMap>().orEmpty()
        graph.opaque +=
            OpaqueNode(
                id = frame.getTextKey("id").asBytes() ?: ByteArray(0),
                frameType = frameType,
                reason = reason,
                sigStat = if (frame.getTextKey("sig") != null) "unverified" else "none",
                pubMeta = frame.getTextKey("pub"),
                recipients = recipients,
            )
    }
}

fun read(data: ByteArray, allowSegments: Boolean = true, expectedHead: ByteArray? = null): Graph {
    val (items, torn) = iterItems(data)
    if (items.isEmpty()) {
        return Graph().also { it.diagnostics += Diagnostic("EmptyFile", "no CBOR items", 0) }
    }
    val bounds = items.indices.filter { isHeaderItem(items[it].item) }
    if (bounds.isEmpty() || bounds.first() != 0) {
        return Graph().also { it.diagnostics += Diagnostic("DamagedFrame", "first item is not a header", 0) }
    }
    if (bounds.size > 1 && !allowSegments) {
        return readSegment(items.subList(0, bounds[1]), 0).also {
            it.diagnostics +=
                Diagnostic(
                    "SegmentBoundary",
                    "segment boundary at item ${bounds[1]} but reader is in pre-segment mode; remainder of file NOT folded",
                    bounds[1],
                )
        }
    }
    val folded =
        bounds.indices.map { i ->
            val a = bounds[i]
            val b = if (i + 1 < bounds.size) bounds[i + 1] else items.size
            readSegment(items.subList(a, b), a)
        }
    val out = if (folded.size == 1) folded.single() else unionSegments(folded)
    if (expectedHead != null) {
        val lastHead = out.segmentHeads.lastOrNull() ?: ByteArray(0)
        if (!lastHead.contentEquals(expectedHead)) {
            out.diagnostics += Diagnostic("TruncatedLog", "observed head does not match expected head")
        }
    }
    if (torn >= 0) out.diagnostics += Diagnostic("TornAppendError", "torn at offset $torn")
    return out
}

fun readFileSegments(data: ByteArray): FileSegments {
    val (items, torn) = iterItems(data)
    if (items.isEmpty()) {
        return FileSegments(emptyList(), torn, Diagnostic("EmptyFile", "no CBOR items", 0))
    }
    val bounds = items.indices.filter { isHeaderItem(items[it].item) }
    if (bounds.isEmpty() || bounds.first() != 0) {
        return FileSegments(emptyList(), torn, Diagnostic("DamagedFrame", "first item is not a header", 0))
    }
    return FileSegments(
        bounds.indices.map { i ->
            val a = bounds[i]
            val b = if (i + 1 < bounds.size) bounds[i + 1] else items.size
            readSegment(items.subList(a, b), a)
        },
        torn,
    )
}

private fun readSegment(items: List<CborItem>, indexOffset: Int): Graph {
    val graph = Graph()
    val header =
        try {
            unwrapHeader(items[0].item)
        } catch (err: RuntimeException) {
            graph.diagnostics += Diagnostic("DamagedFrame", "invalid header: ${err.message}", indexOffset)
            return graph
        }
    val storedHeaderId = header.getTextKey("id").asBytes()
    if (storedHeaderId == null || !storedHeaderId.contentEquals(headerId(header))) {
        graph.diagnostics += Diagnostic("DamagedFrame", "header self-hash mismatch", indexOffset)
    }
    if (header.getTextKey("gts").asText() != MAGIC || header.getTextKey("v").asInt() != VERSION) {
        graph.diagnostics +=
            Diagnostic(
                "DamagedFrame",
                "unsupported header magic/version ${header.getTextKey("gts")}/${header.getTextKey("v")}",
                indexOffset,
            )
    }
    var expectedPrev = storedHeaderId ?: ByteArray(0)
    val folder = Folder(graph, catalogFrom(header))
    val frameIds = mutableListOf<ByteArray>()
    for (idx in 1 until items.size) {
        val absIndex = idx + indexOffset
        val frame = items[idx].item as? CborMap
        if (frame == null) {
            folder.diag("DamagedFrame", "frame is not a map", absIndex)
            frameIds += ByteArray(0)
            continue
        }
        val storedId = frame.getTextKey("id").asBytes()
        val computed = contentId(frame)
        if (storedId == null || !storedId.contentEquals(computed)) {
            folder.diag("DamagedFrame", "frame self-hash mismatch", absIndex)
            folder.opaque(frame, frame.getTextKey("t").asText().orEmpty(), "damaged")
            expectedPrev = storedId ?: computed
            frameIds += expectedPrev
            continue
        }
        val prev = frame.getTextKey("prev").asBytes()
        if (prev == null || !prev.contentEquals(expectedPrev)) folder.diag("BrokenChain", "prev does not match", absIndex)
        expectedPrev = computed
        frameIds += expectedPrev
        frame.getTextKey("sig")?.let { sig ->
            graph.signatures +=
                Signature(
                    frameId = computed,
                    kid = "",
                    status = if (sig is CborBytes) "unverified" else "invalid",
                    cose = sig.asBytes(),
                )
        }
        folder.foldFrame(frame, absIndex)
    }
    graph.segmentHeads += expectedPrev
    graph.segmentMeta += graph.meta.toList()
    graph.segmentProfiles += header.getTextKey("prof").asText() ?: "generic"
    graph.segmentStreamable += layoutCheck(graph, header, folder, frameIds, indexOffset)
    return graph
}

private fun layoutCheck(
    graph: Graph,
    header: CborMap,
    folder: Folder,
    frameIds: List<ByteArray>,
    indexOffset: Int,
): StreamableInfo {
    val claimed = header.getTextKey("layout").asText() == "streamable"
    val total = frameIds.size
    if (!claimed) return StreamableInfo(false, 0, 0)
    if (folder.indexRecords.isEmpty()) {
        graph.diagnostics +=
            Diagnostic(
                "StreamableLayoutError",
                "segment claims layout 'streamable' but carries no intact index footer (§3.3)",
            )
        return StreamableInfo(true, 0, total)
    }
    val last = folder.indexRecords.last()
    val relPos = last.pos - indexOffset
    val tail = total - relPos
    if (last.count != relPos - 1 || last.count < 1 || !frameIds[last.count - 1].contentEquals(last.head)) {
        graph.diagnostics +=
            Diagnostic(
                "StreamableLayoutError",
                "index footer contradicts the frames it covers: count ${last.count} must name the frame immediately before the footer and head must be that frame's id (§3.3)",
                last.pos,
            )
    }
    for (event in folder.blobEvents) {
        val blobRel = event.pos - indexOffset
        if (blobRel <= last.count && !event.described) {
            graph.diagnostics +=
                Diagnostic(
                    "StreamableLayoutError",
                    "covered blob ${event.digest} delivered before its stream:digest description (catalog-before-payload, §3.3)",
                    event.pos,
                )
        }
    }
    return StreamableInfo(true, last.count, tail, last.head)
}

private fun isHeaderItem(item: CborValue): Boolean {
    val inner = if (item is CborTag) item.value else item
    val map = inner as? CborMap ?: return false
    return map.getTextKey("gts") != null && map.getTextKey("t") == null
}

private fun catalogFrom(header: CborMap): Map<Long, Codec> {
    val cat = header.getTextKey("cat") as? CborMap ?: return emptyMap()
    return cat.value.mapNotNull { (cid, entry) ->
        val n = cid.asInt()?.toLong() ?: return@mapNotNull null
        val map = entry as? CborMap ?: return@mapNotNull null
        n to Codec(map.getTextKey("name").asText().orEmpty(), map.getTextKey("cls").asText() ?: "encode")
    }.toMap()
}

private fun diagCodeFor(reason: String): String = if (reason == "missing-key") "MissingKey" else "UnknownCodec"

private data class InternKey(
    val type: Int,
    val a: String = "",
    val b: String = "",
    val c: String = "",
    val d: String = "",
    val segment: Int? = null,
    val reifier: Int? = null,
    val bnodeTid: Int? = null,
    val bnodeLabeled: Boolean? = null,
)

private class Unioner {
    val out = Graph()
    private val intern = mutableMapOf<InternKey, Int>()

    fun mapTerm(segment: Graph, segmentIndex: Int, termId: Int): Int {
        val key = keyFor(segment, segmentIndex, termId)
        intern[key]?.let { return it }
        val term = segment.terms[termId]
        val datatype = term.datatype?.let { mapTerm(segment, segmentIndex, it) }
        val reifier = term.reifier?.let { mapTerm(segment, segmentIndex, it) }
        val value =
            if (term.kind == TermKind.BNODE) {
                if (term.value.isNotEmpty()) "s$segmentIndex.${term.value}" else "s$segmentIndex._anon${out.terms.size}"
            } else {
                term.value
            }
        out.terms += Term(term.kind, value, datatype, term.lang, reifier, term.direction)
        val newId = out.terms.lastIndex
        intern[key] = newId
        return newId
    }

    private fun keyFor(segment: Graph, segmentIndex: Int, termId: Int): InternKey {
        val term = segment.terms[termId]
        return when (term.kind) {
            TermKind.IRI -> InternKey(0, term.value)
            TermKind.LITERAL -> InternKey(1, term.value, segment.datatypeIri(term), term.lang.orEmpty(), term.direction.orEmpty())
            TermKind.BNODE ->
                if (term.value.isNotEmpty()) {
                    InternKey(2, term.value, segment = segmentIndex, bnodeLabeled = true)
                } else {
                    InternKey(2, segment = segmentIndex, bnodeTid = termId)
                }
            TermKind.TRIPLE -> InternKey(3, reifier = term.reifier?.let { mapTerm(segment, segmentIndex, it) })
        }
    }

    fun remapSuppression(segment: Graph, segmentIndex: Int, suppression: Suppression): Suppression {
        val n = segment.terms.size
        val targets =
            suppression.targets.map { target ->
                val map = target as? CborMap ?: return@map target
                val kind = map.getTextKey("kind").asText().orEmpty()
                if (kind == "frame" || kind == "blob") return@map target
                CborMap(
                    map.value.map { (k, v) ->
                        val key = k.asText().orEmpty()
                        val remapped =
                            when {
                                (kind == "term" || kind == "reifier") && key == "id" -> {
                                    val tid = v.asInt()
                                    if (tid != null && tid < n) uint(mapTerm(segment, segmentIndex, tid)) else v
                                }
                                kind == "quad" && key == "q" && v is CborArray ->
                                    CborArray(
                                        v.value.map { x ->
                                            val tid = x.asInt()
                                            if (tid != null && tid < n) uint(mapTerm(segment, segmentIndex, tid)) else x
                                        },
                                    )
                                else -> v
                            }
                        k to remapped
                    },
                )
            }
        val by = suppression.by?.takeIf { it < n }?.let { mapTerm(segment, segmentIndex, it) }
        return Suppression(targets, suppression.reason, by)
    }
}

private fun unionSegments(segments: List<Graph>): Graph {
    val unioner = Unioner()
    val seenQuads = mutableSetOf<Quad>()
    for ((segmentIndex, segment) in segments.withIndex()) {
        for (quad in segment.quads) {
            val remapped =
                Quad(
                    unioner.mapTerm(segment, segmentIndex, quad.s),
                    unioner.mapTerm(segment, segmentIndex, quad.p),
                    unioner.mapTerm(segment, segmentIndex, quad.o),
                    quad.g?.let { unioner.mapTerm(segment, segmentIndex, it) },
                )
            if (seenQuads.add(remapped)) unioner.out.quads += remapped
        }
        for (reifier in segment.reifiers) {
            unioner.out.setReifier(
                unioner.mapTerm(segment, segmentIndex, reifier.rid),
                Triple(
                    unioner.mapTerm(segment, segmentIndex, reifier.spo.s),
                    unioner.mapTerm(segment, segmentIndex, reifier.spo.p),
                    unioner.mapTerm(segment, segmentIndex, reifier.spo.o),
                ),
                reifier.g?.let { unioner.mapTerm(segment, segmentIndex, it) },
            )
        }
        for (annotation in segment.annotations) {
            unioner.out.annotations +=
                AnnotationEntry(
                    unioner.mapTerm(segment, segmentIndex, annotation.s),
                    unioner.mapTerm(segment, segmentIndex, annotation.p),
                    unioner.mapTerm(segment, segmentIndex, annotation.o),
                    annotation.g?.let { unioner.mapTerm(segment, segmentIndex, it) },
                )
        }
        segment.blobs.forEach { unioner.out.setBlob(it.digest, it.data) }
        segment.blobMeta.forEach { unioner.out.setBlobMeta(it.digest, it.meta) }
        segment.meta.forEach { unioner.out.setMeta(it.key, it.value) }
        unioner.out.segmentMeta += segment.segmentMeta
        segment.suppressions.forEach { unioner.out.suppressions += unioner.remapSuppression(segment, segmentIndex, it) }
        unioner.out.opaque += segment.opaque
        unioner.out.signatures += segment.signatures
        unioner.out.diagnostics += segment.diagnostics
        unioner.out.segmentHeads += segment.segmentHeads
        unioner.out.segmentProfiles += segment.segmentProfiles
        unioner.out.segmentStreamable += segment.segmentStreamable
    }
    return unioner.out
}
