// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts

import ca.blackcatinformatics.gts.v2.rdfBlankEnd
import ca.blackcatinformatics.gts.v2.validRdfIri
import ca.blackcatinformatics.gts.v2.validRdfLanguage
import ca.blackcatinformatics.gts.v2.writeRdfSegment

private const val RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

class NQuadsParseException(message: String) : RuntimeException(message)

private data class Atom(
    val kind: TermKind,
    val value: String,
    val lang: String? = null,
    val direction: String? = null,
    val datatype: String? = null,
)

private sealed interface Node

private data class AtomNode(val atom: Atom) : Node

private data class TripleNode(val s: Node, val p: Node, val o: Node) : Node

private class Tokenizer(private val line: String, private val rdf12: Boolean = false) {
    private var i = 0

    fun atEnd(): Boolean {
        skipWs()
        return i >= line.length || line[i] == '.'
    }

    fun requireTerminator() {
        skipWs()
        if (i >= line.length || line[i] != '.') throw NQuadsParseException("missing statement terminator: $line")
        i++
        skipWs()
        if (i != line.length && (!rdf12 || line[i] != '#')) {
            throw NQuadsParseException("trailing tokens after statement terminator: $line")
        }
    }

    fun versionHint() {
        i = "VERSION".length
        skipWs()
        if (i >= line.length || line[i] != '"') throw NQuadsParseException("expected a VERSION string")
        literal(suffix = false)
        skipWs()
        if (i != line.length && line[i] != '#') throw NQuadsParseException("trailing tokens after VERSION")
    }

    fun node(): Node {
        skipWs()
        if (line.startsWith("<<(", i)) return if (rdf12) rdfTriple() else quotedTriple()
        return atomNode()
    }

    private fun atomNode(): AtomNode {
        skipWs()
        if (i >= line.length) throw NQuadsParseException("unexpected end of line: $line")
        return when (line[i]) {
            '<' -> AtomNode(Atom(TermKind.IRI, iri()))
            '_' -> AtomNode(Atom(TermKind.BNODE, bnode()))
            '"' -> AtomNode(literal())
            else -> throw NQuadsParseException("unexpected token at $i in $line")
        }
    }

    private fun skipWs() {
        while (i < line.length && (line[i] == ' ' || line[i] == '\t')) i++
    }

    private fun iri(): String {
        if (line[i] != '<') throw NQuadsParseException("bad IRI in $line")
        val end = line.indexOf('>', i + 1)
        if (end < 0) throw NQuadsParseException("unterminated IRI in $line")
        val value = line.substring(i + 1, end)
        i = end + 1
        if (!rdf12) return value
        val escaped = Tokenizer(value, true)
        val decoded = StringBuilder()
        while (escaped.i < value.length) {
            val slash = value.indexOf('\\', escaped.i)
            if (slash < 0) {
                decoded.append(value, escaped.i, value.length)
                break
            }
            decoded.append(value, escaped.i, slash)
            escaped.i = slash + 1
            if (value.getOrNull(escaped.i) !in listOf('u', 'U')) throw NQuadsParseException("invalid IRI escape")
            decoded.appendCodePoint(escaped.escape())
        }
        val iri = decoded.toString()
        if (!validRdfIri(iri)) throw NQuadsParseException("invalid RDF IRI")
        return iri
    }

    private fun bnode(): String {
        if (!line.startsWith("_:", i)) throw NQuadsParseException("bad blank node in $line")
        i += 2
        val start = i
        if (rdf12) {
            i = rdfBlankEnd(line, i)
        } else {
            while (i < line.length && isBNodeChar(line[i])) i++
            if (i > start && line[i - 1] == '.') i--
        }
        if (i == start) throw NQuadsParseException("empty blank node label in $line")
        return line.substring(start, i)
    }

    private fun literal(suffix: Boolean = true): Atom {
        i++
        val value = StringBuilder()
        while (i < line.length) {
            val ch = line[i++]
            if (ch == '\\') {
                value.appendCodePoint(escape())
                continue
            }
            if (ch == '"') {
                if (!suffix) return Atom(TermKind.LITERAL, value.toString())
                if (rdf12) skipWs()
                var lang: String? = null
                var direction: String? = null
                var datatype: String? = null
                if (i < line.length && line[i] == '@') {
                    i++
                    val start = i
                    while (i < line.length && isLangChar(line[i])) i++
                    lang = line.substring(start, i)
                    if (lang.isEmpty()) throw NQuadsParseException("empty language tag in $line")
                    val sep = lang.lastIndexOf("--")
                    if (sep >= 0) {
                        val rawDirection = lang.substring(sep + 2)
                        if (rawDirection != "ltr" && rawDirection != "rtl") {
                            throw NQuadsParseException("invalid literal direction in $line")
                        }
                        val language = lang.substring(0, sep)
                        if (language.isEmpty()) throw NQuadsParseException("empty language tag in $line")
                        lang = language
                        direction = rawDirection
                    }
                    if (rdf12 && !validRdfLanguage(lang)) throw NQuadsParseException("invalid RDF language tag")
                } else if (line.startsWith("^^", i)) {
                    i += 2
                    skipWs()
                    datatype = iri()
                    if (rdf12 && datatype in setOf(RDF_LANG_STRING, RDF_DIR_LANG_STRING)) {
                        throw NQuadsParseException("the RDF datatype requires a language tag")
                    }
                }
                return Atom(TermKind.LITERAL, value.toString(), lang, direction, datatype)
            }
            value.append(ch)
        }
        throw NQuadsParseException("unterminated literal in $line")
    }

    private fun escape(): Int {
        if (i >= line.length) throw NQuadsParseException("bad escape at end of $line")
        val ch = line[i++]
        if (rdf12 && ch == '\'') return ch.code
        return when (ch) {
            '\\' -> '\\'.code
            '"' -> '"'.code
            'b' -> '\b'.code
            'f' -> '\u000c'.code
            'n' -> '\n'.code
            'r' -> '\r'.code
            't' -> '\t'.code
            'u', 'U' -> {
                val width = if (ch == 'u') 4 else 8
                val raw = line.substring(i, (i + width).coerceAtMost(line.length))
                val validDigits = raw.all { it in '0'..'9' || it.lowercaseChar() in 'a'..'f' || (!rdf12 && it.isDigit()) }
                if (raw.length != width || !validDigits) {
                    throw NQuadsParseException("bad unicode escape \\$ch$raw in $line")
                }
                i += width
                val points = if (rdf12) raw.toLong(16) else raw.toInt(16).toLong()
                val codePoint = points.toInt()
                if (points > 0x10ffffL || (rdf12 && codePoint in 0xd800..0xdfff)) {
                    throw NQuadsParseException("bad unicode escape \\$ch$raw in $line")
                }
                codePoint
            }
            else -> throw NQuadsParseException("unsupported escape \\$ch in $line")
        }
    }

    private fun rdfTriple(): TripleNode {
        val parents = mutableListOf<Pair<AtomNode, AtomNode>>()
        while (line.startsWith("<<(", i)) {
            i += 3
            val subject = atomNode()
            val predicate = atomNode()
            if (subject.atom.kind !in setOf(TermKind.IRI, TermKind.BNODE) || predicate.atom.kind != TermKind.IRI) {
                throw NQuadsParseException("invalid RDF triple term roles")
            }
            parents += subject to predicate
            skipWs()
        }
        var objectNode: Node = atomNode()
        for ((subject, predicate) in parents.asReversed()) {
            skipWs()
            if (!line.startsWith(")>>", i)) throw NQuadsParseException("unterminated RDF triple term")
            i += 3
            objectNode = TripleNode(subject, predicate, objectNode)
        }
        return objectNode as TripleNode
    }

    private fun quotedTriple(): TripleNode {
        i += 3
        val s = node()
        val p = node()
        val o = node()
        skipWs()
        if (!line.startsWith(")>>", i)) throw NQuadsParseException("unterminated quoted triple in $line")
        i += 3
        return TripleNode(s, p, o)
    }
}

private class Interner {
    private val ids = mutableMapOf<List<Any?>, Int>()
    val terms = mutableListOf<Term>()

    fun atom(atom: Atom): Int {
        val key = listOf("atom", atom.kind, atom.value, atom.lang, atom.direction, atom.datatype)
        ids[key]?.let { return it }
        val datatype = atom.datatype?.let { atom(Atom(TermKind.IRI, it)) }
        val id = terms.size
        terms += Term(atom.kind, atom.value, datatype, atom.lang, direction = atom.direction)
        ids[key] = id
        return id
    }

    fun node(node: Node, reifiers: MutableList<ReifierEntry>): Int =
        when (node) {
            is AtomNode -> atom(node.atom)
            // A bare quoted triple as a quad component interns as a SELF-DESCRIBING triple term
            // carrying its own (s, p, o) (§7.3). An unreified triple term is a value, not a
            // statement — it mints no reifier and emits no `reifies` row, so it round-trips as
            // itself.
            is TripleNode -> {
                val s = node(node.s, reifiers)
                val p = node(node.p, reifiers)
                val o = node(node.o, reifiers)
                triple(s, p, o)
            }
        }

    fun nodeV2(first: Node): Int {
        val parents = mutableListOf<Pair<Int, Int>>()
        var node = first
        while (node is TripleNode) {
            parents += rdfAtom((node.s as AtomNode).atom) to rdfAtom((node.p as AtomNode).atom)
            node = node.o
        }
        var id = rdfAtom((node as AtomNode).atom)
        for ((subject, predicate) in parents.asReversed()) id = triple(subject, predicate, id)
        return id
    }

    private fun rdfAtom(value: Atom): Int =
        atom(if (value.kind == TermKind.LITERAL && value.lang == null) value.copy(datatype = value.datatype ?: XSD_STRING) else value)

    private fun triple(s: Int, p: Int, o: Int): Int {
        val key = listOf("triple", s, p, o)
        ids[key]?.let { return it }
        val id = terms.size
        terms += Term(TermKind.TRIPLE, "", triple = Triple(s, p, o))
        ids[key] = id
        return id
    }
}

private fun statements(input: String, rdf12: Boolean = false): Sequence<List<Node>> =
    sequence {
        for (raw in input.lineSequence()) {
            val line = if (rdf12) raw.trimStart(' ', '\t') else raw.trim()
            if (line.isEmpty() || line.startsWith("#")) continue
            val tokenizer = Tokenizer(line, rdf12)
            if (rdf12 && line.startsWith("VERSION")) {
                tokenizer.versionHint()
                continue
            }
            val nodes = mutableListOf<Node>()
            while (!tokenizer.atEnd()) nodes += tokenizer.node()
            tokenizer.requireTerminator()
            if (nodes.size != 3 && nodes.size != 4) {
                throw NQuadsParseException("expected 3 or 4 terms, got " + nodes.size + ": " + line)
            }
            validateStatement(nodes, line, rdf12)
            yield(nodes)
        }
    }

/** Import RDF N-Quads into a committed GTS v2 segment. */
fun fromNQuadsV2(input: String): ByteArray {
    if (input.codePoints().anyMatch { it in 0xd800..0xdfff }) {
        throw NQuadsParseException("input contains a non-scalar Unicode value")
    }
    val interner = Interner()
    val rows = linkedSetOf<Quad>()
    for (nodes in statements(input, rdf12 = true)) {
        rows += Quad(
            interner.nodeV2(nodes[0]), interner.nodeV2(nodes[1]), interner.nodeV2(nodes[2]),
            nodes.getOrNull(3)?.let(interner::nodeV2),
        )
    }
    return writeRdfSegment(interner.terms, rows.toList())
}

fun fromNQuads(input: String): ByteArray {
    val statements = statements(input).toList()
    val interner = Interner()
    val reifiers = mutableListOf<ReifierEntry>()
    val pendingQuads = mutableListOf<Quad>()
    for (nodes in statements) {
        val s = nodes[0]
        val p = nodes[1]
        val o = nodes[2]
        val g = nodes.getOrNull(3)
        if (s is AtomNode && p is AtomNode && p.atom.kind == TermKind.IRI && p.atom.value == RDF_REIFIES && o is TripleNode) {
            val rid = interner.atom(s.atom)
            setReifier(
                reifiers,
                rid,
                Triple(interner.node(o.s, reifiers), interner.node(o.p, reifiers), interner.node(o.o, reifiers)),
                g?.let { interner.node(it, reifiers) },
            )
            continue
        }
        pendingQuads += Quad(interner.node(s, reifiers), interner.node(p, reifiers), interner.node(o, reifiers), g?.let { interner.node(it, reifiers) })
    }
    val reifierIds = reifiers.map { it.rid }.toSet()
    val quads = mutableListOf<Quad>()
    val annotations = mutableListOf<AnnotationEntry>()
    for (quad in pendingQuads) {
        if (quad.s in reifierIds) {
            annotations += AnnotationEntry(quad.s, quad.p, quad.o, quad.g)
        } else {
            quads += quad
        }
    }
    val writer = Writer("dist")
    if (interner.terms.isNotEmpty()) writer.addTerms(interner.terms)
    if (quads.isNotEmpty()) writer.addQuads(quads)
    if (reifiers.isNotEmpty()) writer.addReifies(reifiers)
    if (annotations.isNotEmpty()) writer.addAnnot(annotations)
    return writer.toBytes()
}

/**
 * Record one `r rdf:reifies <<( s p o )>>` statement.
 *
 * `rdf:reifies` is NOT functional (RDF 1.2 / §7.3): `r rdf:reifies <<( s p o1 )>>` and
 * `r rdf:reifies <<( s p o2 )>>` are both assertable and both are kept. Only a byte-identical
 * repeat of a row collapses.
 */
private fun setReifier(reifiers: MutableList<ReifierEntry>, rid: Int, spo: Triple, g: Int? = null) {
    val row = ReifierEntry(rid, spo, g)
    if (reifiers.none { it == row }) reifiers += row
}

private fun validateStatement(nodes: List<Node>, line: String, rdf12: Boolean) {
    fun atom(node: Node, kind: TermKind? = null): Boolean = node is AtomNode && (kind == null || node.atom.kind == kind)
    fun triple(node: Node): Boolean = node is TripleNode
    if (!(atom(nodes[0], TermKind.IRI) || atom(nodes[0], TermKind.BNODE) || (!rdf12 && triple(nodes[0])))) {
        throw NQuadsParseException("invalid subject term: $line")
    }
    if (!atom(nodes[1], TermKind.IRI)) throw NQuadsParseException("predicate must be IRI: $line")
    if (!(atom(nodes[2], TermKind.IRI) || atom(nodes[2], TermKind.BNODE) || atom(nodes[2], TermKind.LITERAL) || triple(nodes[2]))) {
        throw NQuadsParseException("invalid object term: $line")
    }
    nodes.getOrNull(3)?.let {
        if (!(atom(it, TermKind.IRI) || atom(it, TermKind.BNODE))) {
            throw NQuadsParseException("invalid graph name term: $line")
        }
    }
}

private fun isAsciiLetterOrDigit(ch: Char): Boolean = ch in '0'..'9' || ch in 'A'..'Z' || ch in 'a'..'z'

private fun isBNodeChar(ch: Char): Boolean = isAsciiLetterOrDigit(ch) || ch == '_' || ch == '-' || ch == '.'

private fun isLangChar(ch: Char): Boolean = isAsciiLetterOrDigit(ch) || ch == '-'
