// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

private const val RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

fun toNQuads(graph: Graph): String {
    val lines =
        buildList {
            graph.quads.forEach { quad ->
                val parts = mutableListOf(renderTerm(graph, quad.s), renderTerm(graph, quad.p), renderTerm(graph, quad.o))
                quad.g?.let { parts += renderTerm(graph, it) }
                add(parts.joinToString(" ") + " .")
            }
            graph.reifiers.forEach { reifier ->
                val quoted =
                    "<<( ${renderTerm(graph, reifier.spo.s)} ${renderTerm(graph, reifier.spo.p)} " +
                        "${renderTerm(graph, reifier.spo.o)} )>>"
                val parts = mutableListOf(renderTerm(graph, reifier.rid), "<$RDF_REIFIES>", quoted)
                reifier.g?.let { parts += renderTerm(graph, it) }
                add(parts.joinToString(" ") + " .")
            }
            graph.annotations.forEach { annotation ->
                val parts = mutableListOf(renderTerm(graph, annotation.s), renderTerm(graph, annotation.p), renderTerm(graph, annotation.o))
                annotation.g?.let { parts += renderTerm(graph, it) }
                add(parts.joinToString(" ") + " .")
            }
        }.sorted()
    return if (lines.isEmpty()) "" else lines.joinToString("\n") + "\n"
}

/**
 * Render a term as its N-Quads surface form.
 *
 * [open] carries the triple terms whose rendering is still in progress. A `reifies` row may name
 * the very term that resolves through it (§7.3), so a projection MUST NOT recurse blindly: a
 * self-reaching term degrades to the same blank node an unbound triple term already produces.
 */
private fun renderTerm(graph: Graph, termId: Int, open: List<Int> = emptyList()): String {
    if (termId in open) return "_:unbound_triple_$termId"
    val term = graph.terms[termId]
    return when (term.kind) {
        TermKind.IRI -> "<${escapeIri(term.value)}>"
        TermKind.BNODE -> "_:${term.value.ifEmpty { "b$termId" }}"
        TermKind.LITERAL -> renderLiteral(graph, term)
        // Quoted triple (RDF 1.2 triple term): its own "tt" is authoritative; a legacy "tt"-less
        // term still resolves through its reifier (§7.3). A term that states no triple at all
        // degrades to a syntactically valid blank node.
        TermKind.TRIPLE -> {
            val spo = graph.tripleOf(termId)
            if (spo != null) {
                val inner = open + termId
                "<<( ${renderTerm(graph, spo.s, inner)} ${renderTerm(graph, spo.p, inner)} " +
                    "${renderTerm(graph, spo.o, inner)} )>>"
            } else {
                "_:unbound_triple_$termId"
            }
        }
    }
}

private fun renderLiteral(graph: Graph, term: Term): String {
    val base = "\"${escapeLiteral(term.value)}\""
    if (!term.lang.isNullOrEmpty()) {
        val direction = term.direction?.takeIf { isLiteralDirection(it) }?.let { "--$it" }.orEmpty()
        return "$base@${term.lang}$direction"
    }
    val datatype = graph.datatypeIri(term)
    return if (datatype == XSD_STRING) base else "$base^^<${escapeIri(datatype)}>"
}

private fun escapeIri(value: String): String =
    buildString {
        for (ch in value) {
            when (ch) {
                '\\' -> append("\\\\")
                '>' -> append("\\>")
                else -> append(ch)
            }
        }
    }

private fun escapeLiteral(value: String): String =
    buildString {
        for (ch in value) {
            when (ch) {
                '\\' -> append("\\\\")
                '"' -> append("\\\"")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                in '\u0000'..'\u001f' -> append("\\u%04X".format(ch.code))
                else -> append(ch)
            }
        }
    }
