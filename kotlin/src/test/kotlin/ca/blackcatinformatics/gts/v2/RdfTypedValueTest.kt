// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.bytes
import ca.blackcatinformatics.gts.cborArray
import ca.blackcatinformatics.gts.encode
import ca.blackcatinformatics.gts.uint
import kotlin.test.Test
import kotlin.test.assertEquals

class RdfTypedValueTest {
    @Test
    fun generatesUnsignedSignedAndUtcLexicalBoundaries() {
        val cases = listOf(
            Triple("8201f4", "false", "boolean"),
            Triple("82021bffffffffffffffff", "18446744073709551615", "nonNegativeInteger"),
            Triple("82031bffffffffffffffff", "18446744073709551615", "integer"),
            Triple("82033bffffffffffffffff", "-18446744073709551616", "integer"),
            Triple("8304001a1dcd6500", "1970-01-01T00:00:00.5Z", "dateTime"),
            Triple("83040001", "1970-01-01T00:00:00.000000001Z", "dateTime"),
            Triple("82054400aaff41", "00AAFF41", "hexBinary"),
            Triple("820540", "", "hexBinary"),
        )
        val budget = RdfTermBudget()
        for ((hex, lexical, datatype) in cases) {
            val value = readRdfTypedValue(hex.hexToByteArray().field())
            assertEquals(lexical, value.lexical(budget), hex)
            assertEquals("http://www.w3.org/2001/XMLSchema#$datatype", value.datatype, hex)
        }
    }

    @Test
    fun readsTheSameVerifiedValueAsTextAndAsOctets() {
        val raw = "abc".encodeToByteArray()
        val digest = bytes(contextHash("gts2 unit") { update(raw, 0, raw.size) })
        val index = RdfValueIndex(RdfTermBudget())
        index.read(8, encode(cborArray(digest)).field(), raw.field())
        index.read(
            11,
            encode(cborArray(digest, uint(3), uint(1))).field(),
            encode(cborArray(cborArray(digest, uint(3)))).field(),
        )
        val id = RdfValueId(digest.value, 3uL)
        assertEquals("abc", index.lexical(id))
        assertEquals("616263", index.octets(id))
        assertEquals("abc", index.lexical(id))
    }

    private fun ByteArray.field() = CborField(this, 0, size)
}
