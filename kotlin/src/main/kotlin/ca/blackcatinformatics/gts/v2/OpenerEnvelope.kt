// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

private val resyncMarker = "fcf3eb32a2f0267dd16a53f09bff8792".hexToByteArray()

internal data class OpenerEnvelope(val registry: ULong, val hashAlgorithm: ULong)

internal data class ParsedOpener(val envelope: OpenerEnvelope, val identifier: ByteArray?)

internal fun readOpener(item: CborField, resync: Boolean): ParsedOpener {
    val bound = if (resync) item.start + minOf(4096, item.end - item.start) else item.end
    val fields = item.fields(if (resync) 11 else 7, bound)
    fields.next().uint()
    if (fields.next().uint() != 2uL) malformed("out_of_range")
    if (resync) {
        val marker = fields.next()
        val bytes = marker.bytes()
        if (marker.end - bytes != resyncMarker.size ||
            resyncMarker.indices.any { index -> item.data[bytes + index] != resyncMarker[index] }
        ) {
            malformed("out_of_range")
        }
    }
    val registry = fields.next().uint()
    if (registry == 0uL) malformed("out_of_range")
    val hash = fields.next().uint()
    if (hash == 0uL || (registry == 1uL && hash != 1uL)) malformed("unregistered_value")
    val digestLength = if (hash == 1uL) 32 else null
    fields.next().option(bound) { if (it.uint() == 0uL) malformed("unregistered_value") }
    if (!resync) {
        fields.next().option(bound) {
            val canonical = it.uint()
            if (canonical == 0uL || (registry == 1uL && canonical != 1uL)) malformed("unregistered_value")
        }
    }
    fields.next().option(bound) {
        val lineage = it.fields(2, bound)
        lineage.next().bytes(digestLength)
        if (lineage.next().uint() == 0uL) malformed("out_of_range")
    }
    var identityEnd = item.end
    if (resync) {
        fields.next().uint()
        fields.next().option(bound) { it.bytes(digestLength) }
        checkMarks(fields.next(), bound)
        identityEnd = fields.offset
        fields.next().bytes(digestLength)
        if (item.end > bound) malformed("out_of_range")
    }
    val identifier = if (hash == 1uL) openerIdentifier(CborField(item.data, item.start, identityEnd), resync) else null
    return ParsedOpener(OpenerEnvelope(registry, hash), identifier)
}

private fun checkMarks(field: CborField, bound: Int) {
    val marks = field.list()
    var count = 0
    var previous: CborField? = null
    while (marks.hasNext()) {
        if (marks.offset >= bound) malformed("out_of_range")
        val mark = marks.next()
        val array = mark.typed(4)
        if (array.argument == 0uL || readHead(mark.data, array.end, mark.end).major != 0) malformed("wrong_type")
        if (count == 64) malformed("out_of_range")
        if (previous != null && previous.compare(mark) >= 0) malformed("unordered_or_repeated")
        mark.grammar(bound)
        previous = mark
        count++
    }
}
