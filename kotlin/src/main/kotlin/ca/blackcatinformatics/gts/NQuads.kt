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
                add("${renderTerm(graph, reifier.rid)} <$RDF_REIFIES> $quoted .")
            }
            graph.annotations.forEach { annotation ->
                add(
                    "${renderTerm(graph, annotation.s)} ${renderTerm(graph, annotation.p)} " +
                        "${renderTerm(graph, annotation.o)} .",
                )
            }
        }.sorted()
    return if (lines.isEmpty()) "" else lines.joinToString("\n") + "\n"
}

private fun renderTerm(graph: Graph, termId: Int): String {
    val term = graph.terms[termId]
    return when (term.kind) {
        TermKind.IRI -> "<${escapeIri(term.value)}>"
        TermKind.BNODE -> "_:${term.value.ifEmpty { "b$termId" }}"
        TermKind.LITERAL -> renderLiteral(graph, term)
        TermKind.TRIPLE -> {
            val rf = term.reifier?.let { graph.reifier(it) }
            if (rf != null) {
                "<<( ${renderTerm(graph, rf.s)} ${renderTerm(graph, rf.p)} ${renderTerm(graph, rf.o)} )>>"
            } else {
                "_:unbound_triple_$termId"
            }
        }
    }
}

private fun renderLiteral(graph: Graph, term: Term): String {
    val base = "\"${escapeLiteral(term.value)}\""
    if (!term.lang.isNullOrEmpty()) {
        val direction = term.direction?.takeIf { it.isNotEmpty() }?.let { "--$it" }.orEmpty()
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
