// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

internal data class FrameEnvelope(
    val type: ULong?,
    val problem: String?,
    val identity: CborField?,
    val transparent: Boolean,
    val storedId: ByteArray?,
    val content: CborField,
    val payload: FramePayload,
)

internal data class FramePayload(
    val length: ULong?,
    val digest: CborField?,
    val storage: FrameStorage?,
    val stored: CborField,
    val deterministicBytes: Boolean,
    val contentSize: Int,
    val signaturesSize: Int,
    val signatures: List<SignatureRecord>,
)

/**
 * One signature record of a frame's `signatures` list (P6 section 1): the algorithm code, the
 * `key_id` of the public key and the signature value, each exactly as the frame carries it.
 * Empty when the list is malformed; the frame's problem reports that.
 */
internal class SignatureRecord(val algorithm: ULong, val keyId: ByteArray, val value: ByteArray)

internal data class FrameStorage(val tag: ULong, val codec: CborField? = null, val check: CborField? = null)

internal fun readFrame(item: CborField, registry: ULong): FrameEnvelope {
    val problems = FrameProblems()
    val newer = registry > 1uL
    val fields = item.fields(11)
    fields.next()
    val identityStart = fields.offset
    val code = problems.observe { fields.next().uint() }
    problems.record(code == 0uL || (!newer && code != null && code > 16uL), "unregistered_value")
    val known = code != null && code in 1uL..16uL
    val critical = problems.observe { fields.next().flag() }
    problems.record(known && critical != (checkNotNull(code) < 14uL), "forbidden_combination")
    val transparent = problems.observe { fields.next().flag() }
    val parity = code == 16uL
    problems.record(known && transparent != parity, "forbidden_combination")
    val content = fields.next()
    problems.observe { checkContent(code ?: 0uL, content) }
    val length = problems.observe { fields.next().uint() }
    val digest = problems.observe { fields.next().also { it.bytes(32) } }
    val identity = if (problems.fieldFailed) null else CborField(item.data, identityStart, fields.offset)
    val storage = problems.observe { readStorage(fields.next(), newer) }
    problems.record(
        storage != null && ((storage.tag == 2uL && length == 0uL) || (parity && storage.tag != 1uL)),
        "forbidden_combination",
    )
    val payload = fields.next()
    val deterministicBytes = problems.observe { payload.bytes() } != null
    val identifier = fields.next()
    problems.observe { identifier.bytes(32) }
    val signatures = fields.next()
    val records = problems.observe { readSignatures(signatures, newer, parity) } ?: emptyList()
    problems.record(
        invalidPayloadLength(code, length),
        "out_of_range",
    )
    val framePayload =
        FramePayload(
            length,
            digest,
            storage,
            payload,
            deterministicBytes,
            content.end - content.start,
            signatures.end - signatures.start,
            records,
        )
    return FrameEnvelope(code, problems.first, identity, transparent == true, identifier.observedIdentifier(), content, framePayload)
}

private class FrameProblems {
    var first: String? = null
        private set
    var fieldFailed = false
        private set

    fun record(failed: Boolean, reason: String) {
        if (failed && first == null) first = reason
    }

    fun <T> observe(read: () -> T): T? =
        try {
            read()
        } catch (error: EnvelopeFailure) {
            fieldFailed = true
            record(true, error.reason)
            null
        }
}

internal fun invalidPayloadLength(type: ULong?, length: ULong?): Boolean =
    (type == 8uL && length == 0uL) || ((type == 12uL || type == 14uL) && length != null && length != 0uL)

internal fun checkContent(type: ULong, item: CborField) {
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

private fun readStorage(item: CborField, newer: Boolean): FrameStorage {
    val array = item.typed(4)
    if (array.argument == 0uL) malformed("wrong_length")
    val fields = item.list()
    val tag = fields.next().uint()
    if (tag == 0uL || (!newer && tag > 2uL)) malformed("unregistered_value")
    when (tag) {
        1uL -> if (array.argument != 1uL) malformed("wrong_length")
        2uL -> {
            if (array.argument != 3uL) malformed("wrong_length")
            val codec = fields.next()
            val algorithm = codec.uint()
            if (algorithm == 0uL || (!newer && algorithm != 1uL)) malformed("unregistered_value")
            val check = fields.next().also { it.bytes(16) }
            return FrameStorage(tag, codec, check)
        }
        else -> item.grammar()
    }
    return FrameStorage(tag)
}

private fun readSignatures(item: CborField, newer: Boolean, parity: Boolean): List<SignatureRecord> {
    val signatures = item.list()
    val records = mutableListOf<SignatureRecord>()
    var previous: CborField? = null
    while (signatures.hasNext()) {
        val record = signatures.next()
        val fields = record.fields(3)
        val algorithm = fields.next().uint()
        if (algorithm == 0uL || (!newer && algorithm > 2uL)) malformed("unregistered_value")
        val keyId = fields.next()
        val keyIdStart = keyId.bytes(32)
        val value = fields.next()
        val valueStart = value.bytes()
        if (previous != null && previous.compare(record) >= 0) malformed("unordered_or_repeated")
        previous = record
        records +=
            SignatureRecord(
                algorithm,
                keyId.data.copyOfRange(keyIdStart, keyId.end),
                value.data.copyOfRange(valueStart, value.end),
            )
    }
    if (parity && previous != null) malformed("forbidden_combination")
    return records
}
