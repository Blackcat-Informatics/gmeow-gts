// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.Term
import ca.blackcatinformatics.gts.TermKind

private data class TermRun(val kind: Int, val count: ULong)

private class LanguageTable(private val languages: List<String>) {
    private val seen = mutableSetOf<String>()
    private var used = 0

    fun use(index: ULong): String {
        val spelling = languages.valueAt(index)
        rdfRequire(index <= used.toULong(), "not_deterministic")
        if (index == used.toULong()) {
            rdfRequire(seen.add(spelling), "not_deterministic")
            used++
        }
        return spelling
    }

    fun finish() = rdfRequire(used == languages.size, "not_deterministic")
}

private class LiteralForms(
    private val forms: List<Int>,
    private val datatypes: List<ULong>,
    private val languages: List<ULong>,
    count: Int,
) {
    private var typed = 0
    private var tagged = 0

    init {
        val typedCount = forms.count { it == 1 }
        rdfRequire(forms.size == count && datatypes.size == typedCount && languages.size == count - typedCount, "inconsistent_counts")
    }

    fun literal(index: Int, lexical: String, dictionary: RdfDictionary, languageTable: LanguageTable): Term {
        val form = forms[index]
        if (form == 1) {
            val datatype = dictionary.resolve(datatypes[typed++], TermKind.IRI)
            return Term(TermKind.LITERAL, lexical, datatype = datatype)
        }
        val language = languageTable.use(languages[tagged++])
        return Term(TermKind.LITERAL, lexical, lang = language, direction = when (form) {
            3 -> "ltr"
            4 -> "rtl"
            else -> null
        })
    }
}

private fun Fields.forms(count: Int): LiteralForms =
    LiteralForms(next().readColumn { it.code(4) }, next().uColumn(), next().uColumn(), count)

private sealed interface TermBlock {
    val kind: Int
    val count: Int

    fun add(dictionary: RdfDictionary, languages: LanguageTable, values: RdfValueIndex, restart: ULong, budget: RdfTermBudget)

    data class Texts(
        override val kind: Int,
        val prefixes: List<ULong>,
        val lengths: List<ULong>,
        val suffixes: CborField,
        val forms: LiteralForms?,
    ) : TermBlock {
        override val count: Int get() = prefixes.size

        init {
            rdfRequire(prefixes.size == lengths.size, "inconsistent_counts")
            var remaining = suffixes.size.toULong()
            for (length in lengths) {
                rdfRequire(length <= remaining, "inconsistent_counts")
                remaining -= length
            }
            rdfRequire(remaining == 0uL, "inconsistent_counts")
        }

        override fun add(dictionary: RdfDictionary, languages: LanguageTable, values: RdfValueIndex, restart: ULong, budget: RdfTermBudget) {
            var offset = suffixes.start
            var previous = ByteArray(0)
            for ((index, prefix) in prefixes.withIndex()) {
                val reset = index.toULong() % restart == 0uL
                if (index != 0) rdfRequire(prefix <= previous.size.toULong(), "out_of_range")
                if (reset) rdfRequire(prefix == 0uL, "forbidden_combination")
                val shared = prefix.toInt()
                val length = lengths[index].toInt()
                rdfRequire(length <= Int.MAX_VALUE - shared, "out_of_range")
                if (!reset && shared < previous.size && length != 0) {
                    rdfRequire(previous[shared] != suffixes.data[offset], "not_deterministic")
                }
                budget.admitText(shared + length)
                val rebuilt = previous.copyOf(shared + length)
                suffixes.data.copyInto(rebuilt, shared, offset, offset + length)
                previous = rebuilt
                offset += length
                val lexical = CborField(rebuilt, 0, rebuilt.size).decodeRdfText()
                if (forms == null) {
                    dictionary.add(kind, Term(TermKind.IRI, lexical))
                } else {
                    dictionary.add(kind, forms.literal(index, lexical, dictionary, languages))
                }
            }
        }
    }

    data class Blanks(val labels: List<CborField>) : TermBlock {
        override val kind = 3
        override val count: Int get() = labels.size

        override fun add(dictionary: RdfDictionary, languages: LanguageTable, values: RdfValueIndex, restart: ULong, budget: RdfTermBudget) {
            for (label in labels) dictionary.addBlank(ByteString(label.data.copyOfRange(label.start, label.end)))
        }
    }

    data class Triples(val subjects: List<ULong>, val predicates: List<ULong>, val objects: List<ULong>) : TermBlock {
        override val kind = 4
        override val count: Int get() = subjects.size

        init {
            rdfRequire(count == predicates.size && count == objects.size, "inconsistent_counts")
        }

        override fun add(dictionary: RdfDictionary, languages: LanguageTable, values: RdfValueIndex, restart: ULong, budget: RdfTermBudget) {
            for (index in subjects.indices) dictionary.addTriple(subjects[index], predicates[index], objects[index])
        }
    }

    data class References(val ids: List<RdfValueId>, val forms: LiteralForms) : TermBlock {
        override val kind = 5
        override val count: Int get() = ids.size

        override fun add(dictionary: RdfDictionary, languages: LanguageTable, values: RdfValueIndex, restart: ULong, budget: RdfTermBudget) {
            for ((index, id) in ids.withIndex()) {
                val lexical = values.lexical(id)
                dictionary.add(kind, forms.literal(index, lexical, dictionary, languages), id)
            }
        }
    }
}

internal fun readRdfTerms(
    dictionary: RdfDictionary,
    values: RdfValueIndex,
    budget: RdfTermBudget,
    content: CborField,
    payload: CborField,
) {
    val fields = content.fields(4)
    rdfRequire(fields.next().uint() == dictionary.size.toULong(), "inconsistent_counts")
    var previous = 0
    val runs = fields.next().readColumn {
        val run = it.fields(2)
        val kind = run.next().code(7)
        rdfRequire(kind != previous, "not_deterministic")
        previous = kind
        TermRun(kind, run.next().positive())
    }
    rdfRequire(runs.isNotEmpty(), "out_of_range")
    fields.next().code(2)
    val restart = fields.next().positive()
    val record = payload.fields(2)
    val blocks = record.next().readColumn(::readBlock)
    val languages = LanguageTable(record.next().readColumn(CborField::rdfText))
    rdfRequire(blocks.size == runs.size, "inconsistent_counts")
    for ((index, block) in blocks.withIndex()) {
        val run = runs[index]
        rdfRequire(run.count == block.count.toULong(), "inconsistent_counts")
        rdfRequire(run.kind == block.kind, "forbidden_combination")
    }
    for (block in blocks) if (block is TermBlock.Texts) budget.charge(block.prefixes)
    for (block in blocks) block.add(dictionary, languages, values, restart, budget)
    languages.finish()
}

private fun readBlock(field: CborField): TermBlock {
    val tag = field.list()
    rdfRequire(tag.hasNext(), "wrong_length")
    val kind = tag.next().code(7)
    val arity = when (kind) {
        1, 4 -> 4
        2 -> 7
        3 -> 2
        5 -> 6
        else -> malformed("unsupported_term_kind")
    }
    val fields = field.fields(arity)
    fields.next()
    return when (kind) {
        1, 2 -> {
            val prefixes = fields.next().uColumn()
            val lengths = fields.next().uColumn()
            val suffixes = fields.next().body()
            TermBlock.Texts(kind, prefixes, lengths, suffixes, if (kind == 2) fields.forms(prefixes.size) else null)
        }
        3 -> TermBlock.Blanks(fields.next().readColumn(CborField::body))
        4 -> TermBlock.Triples(fields.next().uColumn(), fields.next().uColumn(), fields.next().uColumn())
        else -> {
            val digests = fields.next().readColumn(CborField::digest)
            val lengths = fields.next().uColumn()
            rdfRequire(digests.size == lengths.size, "inconsistent_counts")
            TermBlock.References(digests.zip(lengths, ::RdfValueId), fields.forms(digests.size))
        }
    }
}
