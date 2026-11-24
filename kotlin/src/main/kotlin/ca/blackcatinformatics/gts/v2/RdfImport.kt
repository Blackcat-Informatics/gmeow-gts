// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.CborArray
import ca.blackcatinformatics.gts.CborBytes
import ca.blackcatinformatics.gts.CborNInt
import ca.blackcatinformatics.gts.CborValue
import ca.blackcatinformatics.gts.Quad
import ca.blackcatinformatics.gts.Term
import ca.blackcatinformatics.gts.TermKind
import ca.blackcatinformatics.gts.cborArray
import ca.blackcatinformatics.gts.encode
import ca.blackcatinformatics.gts.text
import ca.blackcatinformatics.gts.uint
import java.io.ByteArrayOutputStream

private const val TERMS_PER_FRAME = 4096
private const val ROWS_PER_FRAME = 16384
private const val RESTART = 64
private const val REBUILT_CAPACITY = 1L shl 28

private data class ImportTerm(val term: Term, val lexical: ByteArray, val languageLength: Int)

internal fun writeRdfSegment(terms: List<Term>, rows: List<Quad>): ByteArray {
    val writer = RdfSegmentWriter()
    val prepared = terms.map { ImportTerm(it, it.value.encodeToByteArray(), it.lang?.encodeToByteArray()?.size ?: 0) }
    val values = linkedMapOf<String, ByteArray>()
    var rebuilt = 0L
    for (entry in prepared) {
        val (term, lexical, languageLength) = entry
        require(languageLength.toULong() < frameCapacity) { "RDF language exceeds the frame capacity" }
        val lexicalLimit = if (term.kind == TermKind.LITERAL) REBUILT_CAPACITY else frameCapacity.toLong()
        require(lexical.size.toLong() <= lexicalLimit) { "RDF term exceeds the import capacity" }
        val usePack = term.kind == TermKind.LITERAL && lexical.isNotEmpty() && !entry.fitsInline()
        if (usePack && values.putIfAbsent(term.value, lexical) == null) {
            rebuilt += lexical.size
            require(rebuilt <= REBUILT_CAPACITY) { "RDF values exceed the import rebuild capacity" }
        }
    }
    val references = writeValuePacks(values) { content, payload -> writer.append(13, content, payload) }
    var base = 0
    var budget = 0L
    for ((index, entry) in prepared.withIndex()) {
        val backed = entry.term.kind == TermKind.LITERAL && entry.term.value in references
        val size = entry.languageLength.toLong() + if (backed) 0 else entry.lexical.size.toLong()
        if (index > base && (index - base == TERMS_PER_FRAME || budget + size > frameCapacity.toLong())) {
            writer.terms(base, prepared.subList(base, index), references)
            base = index
            budget = 0
        }
        budget += size
    }
    if (base < prepared.size) writer.terms(base, prepared.subList(base, prepared.size), references)
    for (batch in rows.chunked(ROWS_PER_FRAME)) writer.quads(batch)
    return writer.finish()
}

private fun ImportTerm.fitsInline(): Boolean {
    val limit = frameCapacity.toLong()
    val rawSize = lexical.size.toLong() + languageLength
    if (rawSize > limit) return false
    // Far from the ceiling, all heads and singleton columns fit in this bound.
    if (rawSize <= limit - 64) return true
    val stub = TermPayload(emptyMap())
    val empty = ImportTerm(term.copy(lang = term.lang?.let { "" }), ByteArray(0), 0)
    stub.encodeTerms(0, listOf(empty))
    val lexicalHeadGrowth = encode(uint(lexical.size)).size - 1
    val languageHeadGrowth = encode(uint(languageLength)).size - 1
    return stub.body().size + rawSize + 2 * lexicalHeadGrowth + languageHeadGrowth <= limit
}

private class RdfSegmentWriter {
    private val segment = V2SegmentWriter()
    private var items = 2 // Reserve the opener and commit.
    private var decoded = 0L
    private var formed = 0L

    fun append(type: Int, content: CborArray, payload: ByteArray, copied: Long = 0) {
        require(items < (1 shl 26)) { "RDF import exceeds the item capacity" }
        require(decoded + payload.size <= (1L shl 36) && formed + copied <= (1L shl 37)) {
            "RDF import exceeds the decoded or formed bytes capacity"
        }
        segment.append(type, encode(content), payload)
        items++
        decoded += payload.size
        formed += copied
    }

    fun terms(base: Int, terms: List<ImportTerm>, references: Map<String, ValueReference>) {
        val encoded = TermPayload(references)
        val content = encoded.encodeTerms(base, terms)
        val payload = encoded.body()
        if (payload.size.toULong() <= frameCapacity) {
            append(1, content, payload, encoded.formed)
        } else {
            require(terms.size > 1) { "RDF term $base exceeds the terms payload capacity" }
            val middle = terms.size / 2
            terms(base, terms.subList(0, middle), references)
            terms(base + middle, terms.subList(middle, terms.size), references)
        }
    }

    fun quads(rows: List<Quad>) {
        val graphRuns = runs(rows, Quad::g).map { run ->
            val graph = run.key?.let { cborArray(uint(2), uint(it)) } ?: cborArray(uint(1))
            cborArray(graph, uint(run.end - run.start))
        }
        val predicateRuns = runs(rows, Quad::p).map { run -> cborArray(uint(run.key), uint(run.end - run.start)) }
        val payload = cborArray(CborArray(graphRuns), CborArray(predicateRuns), delta(rows, Quad::o), delta(rows, Quad::s))
        append(2, cborArray(uint(2), uint(rows.size)), encode(payload))
    }

    fun finish(): ByteArray {
        segment.commit()
        return segment.toBytes()
    }
}

private data class Run<K>(val key: K, val start: Int, val end: Int)

private fun <T, K> runs(rows: List<T>, keyOf: (T) -> K): List<Run<K>> {
    val result = mutableListOf<Run<K>>()
    var start = 0
    while (start < rows.size) {
        val key = keyOf(rows[start])
        var end = start + 1
        while (end < rows.size && keyOf(rows[end]) == key) end++
        result += Run(key, start, end)
        start = end
    }
    return result
}

private fun delta(rows: List<Quad>, field: (Quad) -> Int): CborArray {
    var previous = 0
    return CborArray(
        rows.map { row ->
            val value = field(row)
            val difference = value.toLong() - previous
            previous = value
            if (difference < 0) CborNInt(difference) else uint(difference)
        },
    )
}

private class TermPayload(private val references: Map<String, ValueReference>) {
    private val blocks = mutableListOf<CborArray>()
    private val languages = linkedMapOf<String, Int>()
    var formed = 0L
        private set

    fun encodeTerms(base: Int, terms: List<ImportTerm>): CborArray {
        val termRuns = runs(terms, ::kind)
        for (run in termRuns) blocks += block(run.key, terms.subList(run.start, run.end))
        val counts = termRuns.map { cborArray(uint(it.key), uint(it.end - it.start)) }
        return cborArray(uint(base), CborArray(counts), uint(2), uint(RESTART))
    }

    fun body(): ByteArray = encode(cborArray(CborArray(blocks), CborArray(languages.keys.map(::text))))

    private fun kind(entry: ImportTerm): Int =
        if (entry.term.kind == TermKind.LITERAL && entry.term.value in references) 5 else entry.term.kind.wire + 1

    private fun block(kind: Int, entries: List<ImportTerm>): CborArray {
        val fields =
            when (kind) {
                3 -> listOf(CborArray(entries.map { CborBytes(ByteString(it.lexical)) }))
                4 -> {
                    val triples = entries.map { checkNotNull(it.term.triple) }
                    listOf(CborArray(triples.map { uint(it.s) }), CborArray(triples.map { uint(it.p) }), CborArray(triples.map { uint(it.o) }))
                }
                5 -> {
                    val packed = entries.map { references.getValue(it.term.value) }
                    listOf(CborArray(packed.map { CborBytes(it.digest) }), CborArray(packed.map { uint(it.length) }))
                }
                else -> frontCode(entries)
            }
        val literal = if (kind == 2 || kind == 5) literalColumns(entries) else emptyList()
        return CborArray(listOf(uint(kind)) + fields + literal)
    }

    private fun frontCode(entries: List<ImportTerm>): List<CborValue> {
        val prefixes = mutableListOf<CborValue>()
        val lengths = mutableListOf<CborValue>()
        val suffixes = ByteArrayOutputStream()
        var previous = ByteArray(0)
        for ((index, entry) in entries.withIndex()) {
            val lexical = entry.lexical
            var prefix = 0
            if (index % RESTART != 0) {
                val limit = minOf(previous.size, lexical.size)
                while (prefix < limit && previous[prefix] == lexical[prefix]) prefix++
            }
            prefixes += uint(prefix)
            lengths += uint(lexical.size - prefix)
            suffixes.write(lexical, prefix, lexical.size - prefix)
            formed += prefix
            previous = lexical
        }
        return listOf(CborArray(prefixes), CborArray(lengths), CborBytes(ByteString(suffixes.toByteArray())))
    }

    private fun literalColumns(entries: List<ImportTerm>): List<CborArray> {
        val forms = mutableListOf<CborValue>()
        val datatypes = mutableListOf<CborValue>()
        val languageIds = mutableListOf<CborValue>()
        for ((term) in entries) {
            val language = term.lang
            if (language == null) {
                forms += uint(1)
                datatypes += uint(checkNotNull(term.datatype))
            } else {
                val form =
                    when (term.direction) {
                        "ltr" -> 3
                        "rtl" -> 4
                        else -> 2
                    }
                forms += uint(form)
                languageIds += uint(languages.getOrPut(language) { languages.size })
            }
        }
        return listOf(CborArray(forms), CborArray(datatypes), CborArray(languageIds))
    }
}
