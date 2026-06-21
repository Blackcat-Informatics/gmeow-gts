// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

fun foldSummaryJson(graph: Graph): String {
    val nquads = toNQuads(graph).trimEnd().lines().filter { it.isNotEmpty() }
    val blobItems =
        graph.blobs.sortedBy { it.digest }.joinToString(",") { blob ->
            val meta = graph.blobMeta.firstOrNull { it.digest == blob.digest }?.meta as? CborMap
            val mt = meta?.getTextKey("mt").asText() ?: "application/octet-stream"
            "\"${json(blob.digest)}\":{\"mt\":\"${json(mt)}\",\"size\":${blob.data.size}}"
        }
    val diagnostics = graph.diagnostics.joinToString(",") { "\"${json(it.code)}\"" }
    val opaque = graph.opaque.map { it.reason }.sorted().joinToString(",") { "\"${json(it)}\"" }
    val profiles = graph.segmentProfiles.joinToString(",") { "\"${json(it)}\"" }
    val heads = graph.segmentHeads.joinToString(",") { "\"${hex(it)}\"" }
    val streamable =
        graph.segmentStreamable.joinToString(",") { s ->
            val head = s.head?.let { ",\"head\":\"${hex(it)}\"" } ?: ""
            "{\"claimed\":${s.claimed},\"covered\":${s.covered},\"tail\":${s.tail}$head}"
        }
    val nq = nquads.joinToString(",") { "\"${json(it)}\"" }
    return "{" +
        "\"blobs\":{$blobItems}," +
        "\"diagnostics\":[$diagnostics]," +
        "\"mode\":\"default\"," +
        "\"nquads\":[$nq]," +
        "\"opaque_reasons\":[$opaque]," +
        "\"profiles\":[$profiles]," +
        "\"quads\":${graph.quads.size}," +
        "\"segment_heads\":[$heads]," +
        "\"segments\":${graph.segmentHeads.size}," +
        "\"streamable\":[$streamable]," +
        "\"suppressions\":${graph.suppressions.size}," +
        "\"terms\":${graph.terms.size}" +
        "}"
}

fun json(value: String): String =
    buildString {
        for (ch in value) {
            when (ch) {
                '\\' -> append("\\\\")
                '"' -> append("\\\"")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                else -> append(ch)
            }
        }
    }
