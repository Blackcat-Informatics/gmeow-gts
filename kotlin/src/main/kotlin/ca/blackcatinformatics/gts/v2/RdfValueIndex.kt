// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString

internal data class RdfValueId(val digest: ByteString, val length: ULong)

private data class ValueDeclaration(val type: Int, val id: RdfValueId, val count: ULong)

private data class PackMember(val declaration: ValueDeclaration, val size: ULong)

internal class RdfValueIndex {
    private val units = mutableMapOf<ByteString, CborField>()
    private val candidates = mutableMapOf<RdfValueId, List<CborField>>()
    private val texts = mutableMapOf<RdfValueId, String>()
    private var rebuilt = 0uL
    private var refused = false
    private var remainingText = 1uL shl 28

    fun beginSegment() = units.clear()

    fun read(type: Int, content: CborField, body: CborField) {
        if (type == 13) {
            readPack(content, body.completeRdfItem())
            return
        }
        val declaration = declaration(type, content)
        if (type < 10) {
            if (type == 8) rdfRequire(body.size > 0, "out_of_range")
            readUnit(declaration.id.digest, body)
        } else {
            rebuild(declaration, entries(declaration, body))
        }
    }

    fun lexical(id: RdfValueId): String =
        texts.getOrPut(id) {
            val parts = candidates[id] ?: malformed("missing_value")
            rdfRequire(id.length <= remainingText, "projection_value_limit")
            remainingText -= id.length
            val text = ByteArray(id.length.toInt())
            var offset = 0
            for (part in parts) {
                part.data.copyInto(text, offset, part.start, part.end)
                offset += part.size
            }
            CborField(text, 0, text.size).decodeRdfText()
        }

    private fun readUnit(digest: ByteString, body: CborField) {
        if (digest in units) return
        val computed = contextHash("gts2 unit") { update(body.data, body.start, body.size) }
        if (computed.contentEquals(digest.bytes)) units[digest] = body
    }

    private fun rebuild(declaration: ValueDeclaration, entries: List<RdfValueId>) {
        val id = declaration.id
        val remaining = (1uL shl 37) - rebuilt
        if (refused || id.length > remaining) {
            refused = true
            return
        }
        rebuilt += id.length
        if (declaration.type != 11 || id in candidates) return
        val parts = mutableListOf<CborField>()
        for (entry in entries) {
            val unit = units[entry.digest] ?: return
            if (unit.size.toULong() != entry.length) return
            parts += unit
        }
        val computed = contextHash("gts2 unit") {
            for (part in parts) update(part.data, part.start, part.size)
        }
        if (computed.contentEquals(id.digest.bytes)) candidates[id] = parts
    }

    private fun readPack(content: CborField, payload: CborField) {
        val fields = content.fields(2)
        val count = fields.next().positive()
        val expectedUnits = fields.next().uint()
        val record = payload.fields(2)
        val directory = record.next().readColumn { field ->
            val member = field.fields(3)
            val type = member.next().code(4) + 7
            PackMember(declaration(type, member.next()), member.next().uint())
        }
        val packed = record.next().body()
        rdfRequire(directory.size.toULong() == count, "inconsistent_counts")
        var offset = packed.start
        var totalUnits = 0uL
        val bodies = directory.map { member ->
            rdfRequire(member.size <= (packed.end - offset).toULong(), "inconsistent_counts")
            val end = offset + member.size.toInt()
            val span = CborField(packed.data, offset, end)
            offset = end
            if (member.declaration.type < 10) totalUnits += member.size
            span
        }
        rdfRequire(offset == packed.end && expectedUnits == totalUnits, "inconsistent_counts")
        val recipes = directory.mapIndexed { index, member ->
            val recordFields = member.declaration
            val body = bodies[index]
            if (recordFields.type == 8) rdfRequire(body.size > 0, "out_of_range")
            if (recordFields.type >= 10) entries(recordFields, body) else null
        }
        for ((index, member) in directory.withIndex()) {
            if (member.declaration.type < 10) readUnit(member.declaration.id.digest, bodies[index])
        }
        for ((index, member) in directory.withIndex()) {
            recipes[index]?.let { rebuild(member.declaration, it) }
        }
    }
}

private fun declaration(type: Int, content: CborField): ValueDeclaration {
    val arity = when (type) {
        8 -> 1
        9 -> 2
        10 -> 4
        else -> 3
    }
    val fields = content.fields(arity)
    val digest = fields.next().digest()
    val length = if (type >= 10) fields.next().uint() else 0uL
    if (type == 9 || type == 10) fields.next().option { it.rdfText() }
    val count = if (type >= 10) fields.next().uint() else 0uL
    return ValueDeclaration(type, RdfValueId(digest, length), count)
}

private fun entries(declaration: ValueDeclaration, body: CborField): List<RdfValueId> {
    rdfRequire(declaration.count >= if (declaration.type == 10) 2uL else 1uL, "out_of_range")
    val entries = body.completeRdfItem().readColumn {
        val pair = it.fields(2)
        RdfValueId(pair.next().digest(), pair.next().uint())
    }
    rdfRequire(entries.size.toULong() == declaration.count, "inconsistent_counts")
    var total = 0uL
    for (entry in entries) {
        rdfRequire(entry.length > 0uL && entry.length <= ULong.MAX_VALUE - total, "out_of_range")
        total += entry.length
    }
    rdfRequire(total == declaration.id.length, "inconsistent_counts")
    return entries
}
