// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import java.util.Base64

private const val STREAM_NS = "https://w3id.org/gts/stream#"
private const val RDF_TYPE_IRI = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
private const val XSD_INTEGER_IRI = "http://www.w3.org/2001/XMLSchema#integer"
private const val XSD_DATETIME_IRI = "http://www.w3.org/2001/XMLSchema#dateTime"
private const val COMPACT_AGENT = "gts-compact"

class CompactRefusedException(message: String) : RuntimeException(message)

fun streamableCompact(data: ByteArray, timestamp: String, sealOriginal: Boolean): ByteArray {
    val (graph, profile) = refusalGate(data, sealOriginal)
    val keyed = graph.blobs.map { it.data.size to it.digest }.sortedWith(compareBy<Pair<Int, String>> { it.first }.thenBy { it.second })
    var blobOrder = keyed.map { it.second }
    var sealedDigest = ""
    if (sealOriginal) {
        sealedDigest = digestStr(data)
        blobOrder = blobOrder.filterNot { it == sealedDigest } + sealedDigest
    }

    val index = streamingIndex(graph, blobOrder, timestamp, sealedDigest, data.size)
    val base = index.terms.size
    val writer = Writer(profile, "streamable")
    if (index.terms.isNotEmpty()) writer.addTerms(index.terms)
    if (index.quads.isNotEmpty()) writer.addQuads(index.quads)
    if (graph.terms.isNotEmpty()) writer.addTerms(graph.terms.map { shiftTerm(it, base) })
    if (graph.quads.isNotEmpty()) {
        writer.addQuads(
            graph.quads.map {
                Quad(it.s + base, it.p + base, it.o + base, it.g?.let { g -> g + base })
            },
        )
    }
    if (graph.reifiers.isNotEmpty()) {
        writer.addReifies(
            graph.reifiers.map {
                ReifierEntry(it.rid + base, Triple(it.spo.s + base, it.spo.p + base, it.spo.o + base))
            },
        )
    }
    if (graph.annotations.isNotEmpty()) {
        writer.addAnnot(graph.annotations.map { Triple(it.s + base, it.p + base, it.o + base) })
    }
    for (suppression in shiftedSuppressions(graph, base)) writer.addSuppress(suppression.targets, suppression.reason, suppression.by)
    for (digest in blobOrder) {
        if (digest == sealedDigest) {
            writer.addBlob(data, "application/vnd.blackcat.gts+cbor-seq", "source")
        } else {
            val blobData = graph.blobs.firstOrNull { it.digest == digest }?.data ?: ByteArray(0)
            writer.addBlob(blobData, blobMetaString(graph, digest, "mt"), blobMetaString(graph, digest, "rep"))
        }
    }
    writer.addIndex()
    return writer.toBytes()
}

private data class CompactInput(val graph: Graph, val profile: String)

private fun refusalGate(data: ByteArray, sealOriginal: Boolean): CompactInput {
    val fileSegments = readFileSegments(data)
    fileSegments.fatal?.let { refused("input is not a clean GTS file: ${it.code}: ${it.detail}") }
    if (fileSegments.torn >= 0) refused("input has a torn append at byte ${fileSegments.torn}")
    for ((idx, segment) in fileSegments.segments.withIndex()) {
        if (segment.diagnostics.isNotEmpty()) {
            val first = segment.diagnostics.first()
            refused("segment $idx does not verify cleanly: ${first.code}: ${first.detail}")
        }
    }
    val profiles = fileSegments.segments.flatMap { it.segmentProfiles }.distinct()
    if (profiles.size > 1) {
        val quoted = profiles.sorted().joinToString(", ") { "'$it'" }
        refused("mixed segment profiles [$quoted] are not compactable (v1)")
    }
    val profile = profiles.singleOrNull() ?: "generic"
    if (profile == "evidence" && !sealOriginal) {
        refused("an 'evidence' artifact's signed chain IS the artifact; refusing to re-order it without --seal-original (§10.1)")
    }
    val graph = read(data, true)
    for (suppression in graph.suppressions) {
        if (suppression.targets.any { targetKind(it) == "frame" }) {
            refused(
                "input carries a frame-addressed suppression; the rewrite assigns new frame ids, so the target would silently dangle (§10.1)",
            )
        }
    }
    return CompactInput(graph, profile)
}

private data class GraphBuilder(val terms: MutableList<Term> = mutableListOf(), val quads: MutableList<Quad> = mutableListOf()) {
    fun add(term: Term): Int {
        terms += term
        return terms.lastIndex
    }

    fun literal(value: String, datatype: Int? = null): Int = add(Term(TermKind.LITERAL, value, datatype = datatype))

    fun quad(s: Int, p: Int, o: Int) {
        quads += Quad(s, p, o)
    }
}

private fun streamingIndex(
    graph: Graph,
    blobOrder: List<String>,
    timestamp: String,
    sealedDigest: String,
    sealedSize: Int,
): GraphBuilder {
    val b = GraphBuilder()
    val tType = b.add(Term(TermKind.IRI, RDF_TYPE_IRI))
    val tInt = b.add(Term(TermKind.IRI, XSD_INTEGER_IRI))
    val tDt = b.add(Term(TermKind.IRI, XSD_DATETIME_IRI))
    val tManifestation = b.add(Term(TermKind.IRI, STREAM_NS + "Manifestation"))
    val tDigest = b.add(Term(TermKind.IRI, STREAM_NS + "digest"))
    val tMt = b.add(Term(TermKind.IRI, STREAM_NS + "mediaType"))
    val tSize = b.add(Term(TermKind.IRI, STREAM_NS + "size"))
    val tRole = b.add(Term(TermKind.IRI, STREAM_NS + "role"))
    val tOrder = b.add(Term(TermKind.IRI, STREAM_NS + "order"))
    val tCompaction = b.add(Term(TermKind.IRI, STREAM_NS + "Compaction"))
    val tAgent = b.add(Term(TermKind.IRI, STREAM_NS + "agent"))
    val tTimestamp = b.add(Term(TermKind.IRI, STREAM_NS + "timestamp"))
    val tSourceHead = b.add(Term(TermKind.IRI, STREAM_NS + "sourceHead"))
    val tSealedSource = b.add(Term(TermKind.IRI, STREAM_NS + "sealedSource"))
    val tDetachedSignature = b.add(Term(TermKind.IRI, STREAM_NS + "DetachedSignature"))
    val tSourceFrame = b.add(Term(TermKind.IRI, STREAM_NS + "sourceFrame"))
    val tCose = b.add(Term(TermKind.IRI, STREAM_NS + "cose"))

    for ((order, digest) in blobOrder.withIndex()) {
        val manifestation = b.add(Term(TermKind.BNODE, "m$order"))
        val sealed = digest == sealedDigest
        val size = if (sealed) sealedSize else graph.blobs.firstOrNull { it.digest == digest }?.data?.size ?: 0
        val mt = if (sealed) "application/vnd.blackcat.gts+cbor-seq" else blobMetaString(graph, digest, "mt")
        b.quad(manifestation, tType, tManifestation)
        b.quad(manifestation, tDigest, b.literal(digest))
        mt?.let { b.quad(manifestation, tMt, b.literal(it)) }
        b.quad(manifestation, tSize, b.literal(size.toString(), tInt))
        b.quad(manifestation, tRole, b.literal(if (sealed) "source" else "primary"))
        b.quad(manifestation, tOrder, b.literal(order.toString(), tInt))
    }

    val compaction = b.add(Term(TermKind.BNODE, "c"))
    b.quad(compaction, tType, tCompaction)
    b.quad(compaction, tAgent, b.literal(COMPACT_AGENT))
    b.quad(compaction, tTimestamp, b.literal(timestamp, tDt))
    for (head in graph.segmentHeads) b.quad(compaction, tSourceHead, b.literal("blake3:${hex(head)}"))
    if (sealedDigest.isNotEmpty()) b.quad(compaction, tSealedSource, b.literal(sealedDigest))

    var idx = 0
    for (signature in graph.signatures) {
        val cose = signature.cose ?: continue
        val node = b.add(Term(TermKind.BNODE, "s$idx"))
        idx++
        b.quad(node, tType, tDetachedSignature)
        b.quad(node, tSourceFrame, b.literal("blake3:${hex(signature.frameId)}"))
        b.quad(node, tCose, b.literal(Base64.getUrlEncoder().withoutPadding().encodeToString(cose)))
    }
    return b
}

private fun shiftTerm(term: Term, base: Int): Term =
    Term(
        term.kind,
        term.value,
        term.datatype?.let { it + base },
        term.lang,
        term.reifier?.let { it + base },
        term.direction,
    )

private fun shiftedSuppressions(graph: Graph, base: Int): List<Suppression> =
    graph.suppressions.map { suppression ->
        val targets =
            suppression.targets.map { target ->
                val map = target as? CborMap ?: return@map target
                val kind = targetKind(map)
                CborMap(
                    map.value.map { (key, value) ->
                        val k = key.asText()
                        val shifted =
                            when {
                                (kind == "term" || kind == "reifier") && k == "id" ->
                                    value.asInt()?.let { uint(it + base) } ?: value
                                kind == "quad" && k == "q" && value is CborArray ->
                                    CborArray(value.value.map { it.asInt()?.let { id -> uint(id + base) } ?: it })
                                else -> value
                            }
                        key to shifted
                    },
                )
            }
        Suppression(targets, suppression.reason, suppression.by?.let { it + base })
    }

private fun blobMetaString(graph: Graph, digest: String, key: String): String? {
    val meta = graph.blobMeta.firstOrNull { it.digest == digest }?.meta as? CborMap ?: return null
    return meta.getTextKey(key).asText()
}

private fun targetKind(target: CborValue): String = (target as? CborMap)?.getTextKey("kind").asText().orEmpty()

private fun refused(message: String): Nothing = throw CompactRefusedException(message)
