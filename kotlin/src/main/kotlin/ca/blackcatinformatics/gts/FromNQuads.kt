// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

private const val RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

class NQuadsParseException(message: String) : RuntimeException(message)

private data class Atom(
    val kind: TermKind,
    val value: String,
    val lang: String? = null,
    val datatype: String? = null,
)

private sealed interface Node

private data class AtomNode(val atom: Atom) : Node

private data class TripleNode(val s: Node, val p: Node, val o: Node) : Node

private class Tokenizer(private val line: String) {
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
        if (i != line.length) throw NQuadsParseException("trailing tokens after statement terminator: $line")
    }

    fun node(): Node {
        skipWs()
        if (i >= line.length) throw NQuadsParseException("unexpected end of line: $line")
        if (line.startsWith("<<(", i)) return quotedTriple()
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
        return value
    }

    private fun bnode(): String {
        if (!line.startsWith("_:", i)) throw NQuadsParseException("bad blank node in $line")
        i += 2
        val start = i
        while (i < line.length && isBNodeChar(line[i])) i++
        if (i > start && line[i - 1] == '.') i--
        if (i == start) throw NQuadsParseException("empty blank node label in $line")
        return line.substring(start, i)
    }

    private fun literal(): Atom {
        i++
        val value = StringBuilder()
        while (i < line.length) {
            val ch = line[i++]
            if (ch == '\\') {
                value.appendCodePoint(escape())
                continue
            }
            if (ch == '"') {
                var lang: String? = null
                var datatype: String? = null
                if (i < line.length && line[i] == '@') {
                    i++
                    val start = i
                    while (i < line.length && isLangChar(line[i])) i++
                    lang = line.substring(start, i)
                    if (lang.isEmpty()) throw NQuadsParseException("empty language tag in $line")
                } else if (line.startsWith("^^", i)) {
                    i += 2
                    skipWs()
                    datatype = iri()
                }
                return Atom(TermKind.LITERAL, value.toString(), lang, datatype)
            }
            value.append(ch)
        }
        throw NQuadsParseException("unterminated literal in $line")
    }

    private fun escape(): Int {
        if (i >= line.length) throw NQuadsParseException("bad escape at end of $line")
        return when (val ch = line[i++]) {
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
                if (raw.length != width || !raw.all { it.isDigit() || it.lowercaseChar() in 'a'..'f' }) {
                    throw NQuadsParseException("bad unicode escape \\$ch$raw in $line")
                }
                i += width
                val codePoint = raw.toInt(16)
                if (!Character.isValidCodePoint(codePoint)) {
                    throw NQuadsParseException("bad unicode escape \\$ch$raw in $line")
                }
                codePoint
            }
            else -> throw NQuadsParseException("unsupported escape \\$ch in $line")
        }
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
        val key = listOf("atom", atom.kind, atom.value, atom.lang, atom.datatype)
        ids[key]?.let { return it }
        val datatype = atom.datatype?.let { atom(Atom(TermKind.IRI, it)) }
        val id = terms.size
        terms += Term(atom.kind, atom.value, datatype, atom.lang)
        ids[key] = id
        return id
    }

    fun node(node: Node, reifiers: MutableList<ReifierEntry>): Int =
        when (node) {
            is AtomNode -> atom(node.atom)
            is TripleNode -> {
                val s = node(node.s, reifiers)
                val p = node(node.p, reifiers)
                val o = node(node.o, reifiers)
                val key = listOf("triple", s, p, o)
                ids[key]?.let { return it }
                val rid = terms.size
                terms += Term(TermKind.TRIPLE, "", reifier = rid)
                ids[key] = rid
                setReifier(reifiers, rid, Triple(s, p, o))
                rid
            }
        }
}

fun fromNQuads(input: String): ByteArray {
    val statements = mutableListOf<List<Node>>()
    for (raw in input.lineSequence()) {
        val line = raw.trim()
        if (line.isEmpty() || line.startsWith("#")) continue
        val tokenizer = Tokenizer(line)
        val nodes = mutableListOf<Node>()
        while (!tokenizer.atEnd()) nodes += tokenizer.node()
        tokenizer.requireTerminator()
        if (nodes.size != 3 && nodes.size != 4) {
            throw NQuadsParseException("expected 3 or 4 terms, got ${nodes.size}: $line")
        }
        validateStatement(nodes, line)
        statements += nodes
    }
    val interner = Interner()
    val reifiers = mutableListOf<ReifierEntry>()
    val quads = mutableListOf<Quad>()
    for (nodes in statements) {
        val s = nodes[0]
        val p = nodes[1]
        val o = nodes[2]
        val g = nodes.getOrNull(3)
        if (g == null && s is AtomNode && p is AtomNode && p.atom.kind == TermKind.IRI && p.atom.value == RDF_REIFIES && o is TripleNode) {
            val rid = interner.atom(s.atom)
            setReifier(reifiers, rid, Triple(interner.node(o.s, reifiers), interner.node(o.p, reifiers), interner.node(o.o, reifiers)))
            continue
        }
        quads += Quad(interner.node(s, reifiers), interner.node(p, reifiers), interner.node(o, reifiers), g?.let { interner.node(it, reifiers) })
    }
    val writer = Writer("dist")
    if (interner.terms.isNotEmpty()) writer.addTerms(interner.terms)
    if (quads.isNotEmpty()) writer.addQuads(quads)
    if (reifiers.isNotEmpty()) writer.addReifies(reifiers)
    return writer.toBytes()
}

private fun setReifier(reifiers: MutableList<ReifierEntry>, rid: Int, spo: Triple) {
    val idx = reifiers.indexOfFirst { it.rid == rid }
    if (idx >= 0) reifiers[idx] = ReifierEntry(rid, spo) else reifiers += ReifierEntry(rid, spo)
}

private fun validateStatement(nodes: List<Node>, line: String) {
    fun atom(node: Node, kind: TermKind? = null): Boolean = node is AtomNode && (kind == null || node.atom.kind == kind)
    fun triple(node: Node): Boolean = node is TripleNode
    if (!(atom(nodes[0], TermKind.IRI) || atom(nodes[0], TermKind.BNODE) || triple(nodes[0]))) {
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
