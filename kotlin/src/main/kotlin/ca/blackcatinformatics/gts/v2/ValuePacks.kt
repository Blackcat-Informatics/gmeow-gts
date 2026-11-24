// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.CborArray
import ca.blackcatinformatics.gts.CborBytes
import ca.blackcatinformatics.gts.cborArray
import ca.blackcatinformatics.gts.encode
import ca.blackcatinformatics.gts.uint
import java.io.ByteArrayOutputStream

private const val UNIT_BYTES = 256 * 1024
private const val PACK_BODY_BYTES = 16 * 1024 * 1024
private const val MEMBERS_PER_PACK = 4096

internal data class ValueReference(val digest: ByteString, val length: Int)

internal fun writeValuePacks(
    values: Map<String, ByteArray>,
    append: (CborArray, ByteArray) -> Unit,
): Map<String, ValueReference> {
    if (values.isEmpty()) return emptyMap()
    val packs = ValuePacks(append)
    val references = values.mapValues { (_, value) -> packs.writeValue(value) }
    packs.flush()
    return references
}

private class ValuePacks(private val append: (CborArray, ByteArray) -> Unit) {
    private val units = mutableSetOf<ByteString>()
    private val directory = mutableListOf<CborArray>()
    private var bodies = ByteArrayOutputStream()
    private var storedUnits = 0

    fun writeValue(value: ByteArray): ValueReference {
        val entries = mutableListOf<CborArray>()
        var offset = 0
        while (offset < value.size) {
            val length = minOf(UNIT_BYTES, value.size - offset)
            val digest = unitDigest(value, offset, length)
            entries += cborArray(CborBytes(digest), uint(length))
            if (units.add(digest)) {
                push(1, cborArray(CborBytes(digest)), value, offset, length)
            }
            offset += length
        }
        val digest = unitDigest(value, 0, value.size)
        val declaration = encode(CborArray(entries))
        val content = cborArray(CborBytes(digest), uint(value.size), uint(entries.size))
        push(4, content, declaration, 0, declaration.size)
        return ValueReference(digest, value.size)
    }

    fun flush() {
        if (directory.isEmpty()) return
        val content = cborArray(uint(directory.size), uint(storedUnits))
        val payload = cborArray(CborArray(directory), CborBytes(ByteString(bodies.toByteArray())))
        append(content, encode(payload))
        directory.clear()
        bodies = ByteArrayOutputStream()
        storedUnits = 0
    }

    private fun push(type: Int, content: CborArray, bytes: ByteArray, offset: Int, length: Int) {
        // A separate directory limit leaves room for member records and the array heads.
        if (bodies.size() + length > PACK_BODY_BYTES || directory.size == MEMBERS_PER_PACK) flush()
        directory += cborArray(uint(type), content, uint(length))
        bodies.write(bytes, offset, length)
        if (type == 1) storedUnits += length
    }
}

private fun unitDigest(bytes: ByteArray, start: Int, length: Int): ByteString =
    ByteString(contextHash("gts2 unit") { update(bytes, start, length) })
