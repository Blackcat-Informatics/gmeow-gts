// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

private sealed interface TermRender {
    data class Visit(val id: Int) : TermRender
    data class Close(val id: Int) : TermRender
    data object Space : TermRender
}

private fun renderTerm(graph: Graph, termId: Int): String =
    buildString {
        val pending = ArrayDeque<TermRender>()
        val open = mutableSetOf<Int>()
        pending.addLast(TermRender.Visit(termId))
        while (pending.isNotEmpty()) {
            when (val visit = pending.removeLast()) {
                TermRender.Space -> append(' ')
                is TermRender.Close -> {
                    open.remove(visit.id)
                    append(" )>>")
                }
                is TermRender.Visit -> {
                    val id = visit.id
                    val term = graph.terms[id]
                    when (term.kind) {
                        TermKind.IRI -> append('<').append(escapeIri(term.value)).append('>')
                        TermKind.BNODE -> append("_:").append(term.value.ifEmpty { "b$id" })
                        TermKind.LITERAL -> append(renderLiteral(graph, term))
                        TermKind.TRIPLE -> {
                            val triple = if (id in open) null else graph.tripleOf(id)
                            if (triple == null) {
                                append("_:unbound_triple_").append(id)
                            } else {
                                open.add(id)
                                append("<<( ")
                                pending.addLast(TermRender.Close(id))
                                pending.addLast(TermRender.Visit(triple.o))
                                pending.addLast(TermRender.Space)
                                pending.addLast(TermRender.Visit(triple.p))
                                pending.addLast(TermRender.Space)
                                pending.addLast(TermRender.Visit(triple.s))
                            }
                        }
                    }
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
