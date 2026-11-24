// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

internal data class FrameEnvelope(val type: ULong?, val problem: String?)

internal fun readFrame(item: CborField, registry: ULong): FrameEnvelope {
    var type: ULong? = null
    val newer = registry > 1uL
    return try {
        val fields = item.fields(11)
        fields.next().uint()
        val code = fields.next().uint()
        type = code
        if (code == 0uL || (!newer && code > 16uL)) malformed("unregistered_value")
        val critical = fields.next().flag()
        val known = code <= 16uL
        if (known && critical != (code < 14uL)) malformed("forbidden_combination")
        val transparent = fields.next().flag()
        val parity = code == 16uL
        if (known && transparent != parity) malformed("forbidden_combination")
        checkContent(code, fields.next())
        val length = fields.next().uint()
        fields.next().bytes(32)
        val storage = storageTag(fields.next(), newer)
        if ((storage == 2uL && length == 0uL) || (parity && storage != 1uL)) malformed("forbidden_combination")
        fields.next().bytes()
        fields.next().bytes(32)
        checkSignatures(fields.next(), newer, parity)
        if ((code == 8uL && length == 0uL) || ((code == 12uL || code == 14uL) && length != 0uL)) {
            malformed("out_of_range")
        }
        FrameEnvelope(code, null)
    } catch (error: EnvelopeFailure) {
        FrameEnvelope(type, error.reason)
    }
}

private fun checkContent(type: ULong, item: CborField) {
    when (type) {
        1uL -> {
            val fields = item.fields(4)
            fields.next().uint()
            val runs = fields.next().list()
            while (runs.hasNext()) checkUints(runs.next(), 2)
            fields.next().uint()
            fields.next().uint()
        }
        2uL, 3uL, 4uL, 13uL -> checkUints(item, 2)
        5uL, 6uL, 7uL -> checkUints(item, 1)
        8uL -> item.fields(1).next().bytes(32)
        9uL -> {
            val fields = item.fields(2)
            fields.next().bytes(32)
            textOption(fields.next())
        }
        10uL -> {
            val fields = item.fields(4)
            fields.next().bytes(32)
            fields.next().uint()
            textOption(fields.next())
            fields.next().uint()
        }
        11uL -> {
            val fields = item.fields(3)
            fields.next().bytes(32)
            repeat(2) { fields.next().uint() }
        }
        12uL -> {
            val fields = item.fields(3)
            val name = fields.next().fields(2)
            val algorithm = name.next().uint()
            name.next().bytes(if (algorithm == 1uL) 32 else null)
            fields.next().uint()
            textOption(fields.next())
        }
        14uL, 15uL -> {
            val fields = item.fields(2)
            fields.next().uint()
            fields.next().bytes(32)
        }
        16uL -> {
            val fields = item.fields(5)
            repeat(3) { fields.next().uint() }
            repeat(2) { fields.next().bytes(32) }
        }
        else -> item.grammar()
    }
}

private fun checkUints(item: CborField, count: Int) {
    val fields = item.fields(count)
    repeat(count) { fields.next().uint() }
}

private fun textOption(item: CborField) {
    val array = item.typed(4)
    if (array.argument == 0uL) malformed("wrong_length")
    val fields = item.list()
    val tag = fields.next().uint()
    if (tag == 1uL || tag == 2uL) {
        if (array.argument != tag) malformed("wrong_length")
        if (tag == 2uL) fields.next().typed(3)
    }
    item.grammar()
}

private fun storageTag(item: CborField, newer: Boolean): ULong {
    val array = item.typed(4)
    if (array.argument == 0uL) malformed("wrong_length")
    val fields = item.list()
    val tag = fields.next().uint()
    if (tag == 0uL || (!newer && tag > 2uL)) malformed("unregistered_value")
    when (tag) {
        1uL -> if (array.argument != 1uL) malformed("wrong_length")
        2uL -> {
            if (array.argument != 3uL) malformed("wrong_length")
            val codec = fields.next().uint()
            if (codec == 0uL || (!newer && codec != 1uL)) malformed("unregistered_value")
            fields.next().bytes(16)
        }
        else -> item.grammar()
    }
    return tag
}

private fun checkSignatures(item: CborField, newer: Boolean, parity: Boolean) {
    val signatures = item.list()
    var previous: CborField? = null
    while (signatures.hasNext()) {
        val record = signatures.next()
        val fields = record.fields(3)
        val algorithm = fields.next().uint()
        if (algorithm == 0uL || (!newer && algorithm > 2uL)) malformed("unregistered_value")
        fields.next().bytes(32)
        fields.next().bytes()
        if (previous != null && previous.compare(record) >= 0) malformed("unordered_or_repeated")
        previous = record
    }
    if (parity && previous != null) malformed("forbidden_combination")
}
