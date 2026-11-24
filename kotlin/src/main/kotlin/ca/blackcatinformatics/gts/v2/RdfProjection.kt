// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.Graph
import ca.blackcatinformatics.gts.Term
import ca.blackcatinformatics.gts.TermKind
import ca.blackcatinformatics.gts.Triple

internal data class RdfProjection(val graph: Graph, val emptyGraphs: Set<Int>)

internal class RdfProjectionFailure(message: String, cause: Throwable? = null) : IllegalArgumentException(message, cause)

internal fun projectRdf(events: Sequence<WireEvent>): RdfProjection = RdfProjector().project(events)

internal const val RDF_REIFIES_IRI = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

private class RdfProjector {
    private val graph = Graph()
    private val pool = mutableMapOf<Term, Int>()
    private val rows = RdfRows()
    private val termBudget = RdfTermBudget()
    private val values = RdfValueIndex(termBudget)
    private var dictionary = RdfDictionary(graph, pool)

    fun project(events: Sequence<WireEvent>): RdfProjection {
        for (event in events) {
            when (event) {
                is WireEvent.Stop ->
                    throw RdfProjectionFailure("framing stopped at byte ${event.offset}: ${event.reason.name.lowercase()}")
                is WireEvent.Item -> {
                    try {
                        read(event)
                    } catch (error: EnvelopeFailure) {
                        throw RdfProjectionFailure("byte ${event.offset}: ${error.reason}", error)
                    } catch (error: FramingException) {
                        throw RdfProjectionFailure("byte ${event.offset}: wrong_length", error)
                    }
                }
            }
        }
        return rows.finish(graph)
    }

    private fun read(item: WireEvent.Item) {
        when (item.kind) {
            ItemKind.HEADER, ItemKind.RESYNC -> {
                if (item.problem != null) malformed(item.problem)
                rdfRequire(item.opener?.registry == 1uL && item.effectiveId != null, "unsupported_opener")
                dictionary = RdfDictionary(graph, pool)
                values.beginSegment()
            }
            ItemKind.OFFSETS, ItemKind.LOCATOR -> Unit
            ItemKind.FRAME -> readFrame(item)
            else -> malformed(item.problem ?: "unreadable_item")
        }
    }

    private fun readFrame(item: WireEvent.Item) {
        if (item.problem != null) malformed(item.problem)
        rdfRequire(item.link == FrameLink.LINKED || item.link == FrameLink.HEALED, "broken_link")
        rdfRequire(item.storedMatches != false, "stored_check_mismatch")
        val payload = item.payload?.decoded ?: malformed(item.payload?.reason ?: "payload_unavailable")
        val content = checkNotNull(item.content)
        when (val frameType = item.frameType?.toInt()) {
            1 -> readRdfTerms(dictionary, values, termBudget, content, payload.completeRdfItem())
            2, 3, 4, 5 -> rows.read(dictionary, frameType, content, payload.completeRdfItem())
            8, 9, 10, 11, 13 -> values.read(frameType, content, payload)
            // These item types do not contribute rows to the whole RDF projection.
            6, 7, 12, 14, 15, 16 -> Unit
            else -> malformed("unsupported_frame_type")
        }
    }
}

private data class WrittenTerm(val kind: Int, val rdf: Int, val ref: RdfValueId?, val components: Triple?)

private data class RdfDefinition(val rdf: Int, val written: Int)

internal class RdfDictionary(private val graph: Graph, private val pool: MutableMap<Term, Int>) {
    private val blanks = mutableMapOf<ByteString, Int>()
    private val entries = mutableListOf<RdfDefinition>()
    private val writtenIds = mutableMapOf<WrittenTerm, Int>()

    val size: Int get() = entries.size

    fun intern(term: Term): Int =
        pool.getOrPut(term) {
            val id = graph.terms.size
            graph.terms += term
            id
        }

    fun add(kind: Int, term: Term, reference: RdfValueId? = null) =
        append(WrittenTerm(kind, intern(term), reference, null))

    fun addGenerated(kind: Int, lexical: String, datatype: String, reference: RdfValueId? = null) {
        val iri = intern(Term(TermKind.IRI, datatype))
        add(kind, Term(TermKind.LITERAL, lexical, datatype = iri), reference)
    }

    fun addBlank(label: ByteString) {
        val id = blanks.getOrPut(label) { intern(Term(TermKind.BNODE, "b${graph.terms.size}")) }
        append(WrittenTerm(3, id, null, null))
    }

    fun addTriple(subject: ULong, predicate: ULong, objectId: ULong) {
        val triple = Triple(node(subject), resolve(predicate, TermKind.IRI), resolve(objectId))
        val written = Triple(written(subject), written(predicate), written(objectId))
        append(WrittenTerm(4, intern(Term(TermKind.TRIPLE, "", triple = triple)), null, written))
    }

    private fun append(key: WrittenTerm) {
        val written = writtenIds.getOrPut(key) { writtenIds.size }
        entries += RdfDefinition(key.rdf, written)
    }

    fun written(id: ULong): Int = entries.valueAt(id).written

    fun resolve(id: ULong, vararg kinds: TermKind): Int {
        val rdf = entries.valueAt(id).rdf
        rdfRequire(kinds.isEmpty() || graph.terms[rdf].kind in kinds, "forbidden_combination")
        return rdf
    }

    fun node(id: ULong): Int = resolve(id, TermKind.IRI, TermKind.BNODE)
}

internal class RdfTermBudget {
    private var remaining = 1uL shl 37
    private var remainingText = 1uL shl 28

    fun admitText(length: Int) {
        admit(length.toULong(), 1uL, "projection_text_limit")
    }

    fun admitReference(length: ULong, expansion: ULong): Int =
        admit(length, expansion, "projection_value_limit")

    private fun admit(length: ULong, expansion: ULong, reason: String): Int {
        rdfRequire(length <= remainingText / expansion, reason)
        val used = length * expansion
        remainingText -= used
        return used.toInt()
    }

    fun charge(prefixes: List<ULong>) {
        for (prefix in prefixes) {
            rdfRequire(prefix <= remaining, "total_formed")
            remaining -= prefix
        }
    }
}
