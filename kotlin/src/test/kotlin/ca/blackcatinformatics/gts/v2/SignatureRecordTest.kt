// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import kotlin.io.path.Path
import kotlin.io.path.readBytes
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class SignatureRecordTest {
    private fun records(name: String): List<SignatureRecord> {
        val data = Path("../vectors/v2/P6/$name.bin").readBytes()
        val version = assertIs<WireInspection.VersionTwo>(inspectWire(data))
        return version.events
            .filterIsInstance<WireEvent.Item>()
            .filter { it.kind == ItemKind.FRAME && it.problem == null }
            .flatMap { readFrame(CborField(data, it.offset, it.end), 1uL).payload.signatures }
            .toList()
    }

    @Test
    fun exposesCompositeRecordsWithTheLengthsThatP6FixesForTheirAlgorithm() {
        val composite = records("composite-honest")
        assertTrue(composite.isNotEmpty(), "the composite vector carries a signature record")
        composite.forEach { record ->
            assertEquals(2uL, record.algorithm, "mldsa65_ed25519 is registry code 2")
            assertEquals(32, record.keyId.size, "key_id is a 32-byte digest (P6 section 1)")
            assertEquals(3373, record.value.size, "a composite value is 3309 plus 64 bytes (P6 section 4.2)")
        }
    }

    @Test
    fun exposesClassicalRecordsWithTheEd25519ValueLength() {
        val classical = records("ed25519-honest")
        assertTrue(classical.isNotEmpty(), "the Ed25519 vector carries a signature record")
        classical.forEach { record ->
            assertEquals(1uL, record.algorithm, "ed25519 is registry code 1")
            assertEquals(32, record.keyId.size, "key_id is a 32-byte digest (P6 section 1)")
            assertEquals(64, record.value.size, "an Ed25519 value is 64 bytes (P6 section 3)")
        }
    }
}
