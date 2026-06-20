// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

fun toNQuads(graph: Graph): String {
    val lines =
        graph.quads.map { quad ->
            val parts = mutableListOf(renderTerm(graph, quad.s), renderTerm(graph, quad.p), renderTerm(graph, quad.o))
            quad.g?.let { parts += renderTerm(graph, it) }
            parts.joinToString(" ") + " ."
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
                "<< ${renderTerm(graph, rf.s)} ${renderTerm(graph, rf.p)} ${renderTerm(graph, rf.o)} >>"
            } else {
                "<< <urn:gts:missing> <urn:gts:missing> <urn:gts:missing> >>"
            }
        }
    }
}

private fun renderLiteral(graph: Graph, term: Term): String {
    val base = "\"${escapeLiteral(term.value)}\""
    if (!term.lang.isNullOrEmpty()) return "$base@${term.lang}"
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
                else -> append(ch)
            }
        }
    }
