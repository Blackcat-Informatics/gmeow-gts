// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.Graph
import ca.blackcatinformatics.gts.RDF_DIR_LANG_STRING
import ca.blackcatinformatics.gts.RDF_LANG_STRING
import ca.blackcatinformatics.gts.Term
import ca.blackcatinformatics.gts.TermKind
import ca.blackcatinformatics.gts.XSD_STRING
import ca.blackcatinformatics.gts.toNQuads

private const val RDF_OUTPUT_LIMIT = 1L shl 28

internal fun renderRdfProjection(projection: RdfProjection): String {
    val graph = projection.graph
    val used = usedTerms(graph)
    val lengths = LongArray(used.size)
    for ((id, term) in graph.terms.withIndex()) {
        if (!used[id]) continue
        lengths[id] = when (term.kind) {
            TermKind.IRI -> {
                if (!validRdfIri(term.value)) throw RdfProjectionFailure("invalid RDF IRI")
                term.value.length + 2L
            }
            TermKind.BNODE -> term.value.length + 2L
            TermKind.LITERAL -> literalLength(graph, term)
            TermKind.TRIPLE -> {
                val triple = checkNotNull(term.triple)
                10 + lengths[triple.s] + lengths[triple.p] + lengths[triple.o]
            }
        }
        if (lengths[id] > RDF_OUTPUT_LIMIT) throw RdfProjectionFailure("N-Quads output exceeds the projection limit")
    }
    var total = 0L
    for (row in graph.quads) {
        val named = row.g?.let { lengths[it] + 1 } ?: 0
        total += lengths[row.s] + lengths[row.p] + lengths[row.o] + named + 5
        if (total > RDF_OUTPUT_LIMIT) throw RdfProjectionFailure("N-Quads output exceeds the projection limit")
    }
    return toNQuads(graph)
}

private fun usedTerms(graph: Graph): BooleanArray {
    val used = BooleanArray(graph.terms.size)
    for (row in graph.quads) {
        used[row.s] = true
        used[row.p] = true
        used[row.o] = true
        row.g?.let { used[it] = true }
    }
    for (id in graph.terms.indices.reversed()) {
        if (!used[id]) continue
        val term = graph.terms[id]
        term.datatype?.let { used[it] = true }
        term.triple?.let {
            used[it.s] = true
            used[it.p] = true
            used[it.o] = true
        }
    }
    return used
}

private fun literalLength(graph: Graph, term: Term): Long {
    val language = term.lang
    val suffix =
        if (language != null) {
            if (!validRdfLanguage(language)) throw RdfProjectionFailure("invalid RDF language tag")
            1L + language.length + (term.direction?.length?.plus(2) ?: 0)
        } else {
            val iri = graph.terms[checkNotNull(term.datatype)].value
            if (iri == RDF_LANG_STRING || iri == RDF_DIR_LANG_STRING) throw RdfProjectionFailure("RDF datatype requires a language tag")
            if (iri == XSD_STRING) 0L else 4L + iri.length
        }
    var length = suffix + 2
    for (ch in term.value) {
        length += when (ch) {
            '\\', '"', '\n', '\r', '\t' -> 2
            in '\u0000'..'\u001f' -> 6
            else -> 1
        }
    }
    return length
}
