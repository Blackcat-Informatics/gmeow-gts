// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.Graph
import ca.blackcatinformatics.gts.Quad
import ca.blackcatinformatics.gts.Term
import ca.blackcatinformatics.gts.TermKind

private data class RowRun<T>(val value: T, val size: ULong)

private class RowRuns<T>(private val runs: List<RowRun<T>>, count: ULong) {
    private var index = 0
    private var left = runs.firstOrNull()?.size ?: 0uL

    init {
        var remaining = count
        for (run in runs) {
            rdfRequire(run.size <= remaining, "inconsistent_counts")
            remaining -= run.size
        }
        rdfRequire(remaining == 0uL, "inconsistent_counts")
    }

    fun next(): T {
        val result = runs[index].value
        left--
        if (left == 0uL) left = runs.getOrNull(++index)?.size ?: 0uL
        return result
    }
}

private fun <T> CborField.runs(count: ULong, read: (CborField) -> T): RowRuns<T> {
    val entries = readColumn {
        val pair = it.fields(2)
        RowRun(read(pair.next()), pair.next().positive())
    }
    for (index in 1 until entries.size) {
        rdfRequire(entries[index].value != entries[index - 1].value, "not_deterministic")
    }
    return RowRuns(entries, count)
}

private fun CborField.graphName(): ULong? {
    var id: ULong? = null
    option { id = it.uint() }
    return id
}

internal class RdfRows {
    private val rows = linkedSetOf<Quad>()
    private val emptyGraphs = linkedSetOf<Int>()

    fun finish(graph: Graph): RdfProjection {
        graph.quads.addAll(rows)
        for (row in rows) row.g?.let(emptyGraphs::remove)
        return RdfProjection(graph, emptyGraphs)
    }

    fun read(dictionary: RdfDictionary, type: Int, content: CborField, payload: CborField) {
        if (type == 5) {
            val count = content.fields(1).next().positive()
            val names = payload.fields(1).next().checkedDeltas(count)
            for (name in names) emptyGraphs += dictionary.node(name)
            return
        }
        val fields = content.fields(2)
        fields.next().code(2)
        val count = fields.next().positive()
        val body = payload.fields(if (type == 4) 5 else if (type == 3) 3 else 4)
        val graphs = body.next().runs(count, CborField::graphName)

        when (type) {
            2 -> {
                val predicates = body.next().runs(count, CborField::uint)
                val objects = body.next().checkedDeltas(count)
                val subjects = body.next().checkedDeltas(count)
                for ((index, subject) in subjects.withIndex()) {
                    rows += Quad(
                        dictionary.node(subject),
                        dictionary.resolve(predicates.next(), TermKind.IRI),
                        dictionary.resolve(objects[index]),
                        graphs.next()?.let(dictionary::node),
                    )
                }
            }
            3 -> {
                val subjects = body.next().checkedDeltas(count)
                val triples = body.next().checkedDeltas(count)
                val reifies = dictionary.intern(Term(TermKind.IRI, RDF_REIFIES_IRI))
                for ((index, subject) in subjects.withIndex()) {
                    rows += Quad(dictionary.node(subject), reifies, dictionary.resolve(triples[index], TermKind.TRIPLE), graphs.next()?.let(dictionary::node))
                }
            }
            4 -> {
                val subjects = body.next().checkedDeltas(count)
                val predicates = body.next().checkedDeltas(count)
                val objects = body.next().checkedDeltas(count)
                body.next().checkedDeltas(count, minimum = 1uL)
                val seen = mutableSetOf<Quad>()
                for ((index, subject) in subjects.withIndex()) {
                    val predicate = predicates[index]
                    val obj = objects[index]
                    val name = graphs.next()
                    val written = Quad(dictionary.written(subject), dictionary.written(predicate), dictionary.written(obj), name?.let(dictionary::written))
                    rdfRequire(seen.add(written), "unordered_or_repeated")
                    rows += Quad(dictionary.node(subject), dictionary.resolve(predicate, TermKind.IRI), dictionary.resolve(obj), name?.let(dictionary::node))
                }
            }
        }
    }
}

private fun CborField.checkedDeltas(count: ULong, minimum: ULong = 0uL): List<ULong> =
    deltaColumn(minimum).also { rdfRequire(it.size.toULong() == count, "inconsistent_counts") }
