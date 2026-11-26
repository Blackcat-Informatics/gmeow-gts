// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.CborArray
import ca.blackcatinformatics.gts.CborBytes
import ca.blackcatinformatics.gts.CborUInt
import ca.blackcatinformatics.gts.CborValue
import ca.blackcatinformatics.gts.decode
import ca.blackcatinformatics.gts.hex
import ca.blackcatinformatics.gts.parseHex
import kotlin.io.path.Path
import kotlin.io.path.readBytes
import kotlin.io.path.readText
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * Checks `Verify(signature, frame)` of P6 section 6.2 at its boundary: each signature record of a
 * P6 vector, the frame's effective identifier, the `h0` of its segment and the held keys of the
 * vector's recorded read. The expected outcomes are the signature results of that recorded read.
 */
class SignatureVerifyTest {
    private data class Outcome(val status: String, val reason: String?, val algorithm: ULong, val keyId: String)

    private class Case(val name: String, val data: ByteArray, val heldKeys: List<HeldKey>, val expected: Map<Int, List<Outcome>>)

    private val reasons =
        mapOf(
            53uL to "unknown_signature_algorithm",
            54uL to "key_not_held",
            57uL to "key_mismatch",
            58uL to "value_length",
            59uL to "non_canonical_scalar",
            60uL to "small_order_key",
            61uL to "small_order_nonce",
            62uL to "equation_failed",
            63uL to "composite_classical_failed",
            64uL to "composite_post_quantum_failed",
        )
    private val statuses = mapOf(1uL to "valid", 2uL to "invalid", 3uL to "unverified")

    private fun CborValue.items(): List<CborValue> = assertIs<CborArray>(this).value

    private fun CborValue.uint(): ULong = assertIs<CborUInt>(this).value.toULong()

    private fun CborValue.bytes(): ByteArray = assertIs<CborBytes>(this).value.bytes

    private fun cases(name: String): List<Case> {
        val vector = Json.parseToJsonElement(Path("../vectors/v2/P6/$name.json").readText()).jsonObject
        return vector.getValue("cases").jsonArray.map { element ->
            val case = element.jsonObject
            val input = case.getValue("input_file").jsonPrimitive.content
            val read = decode(parseHex(case.getValue("read_result").jsonPrimitive.content)).items()
            val held =
                read[5].items()[1].items().map {
                    val key = it.items()
                    HeldKey(key[1].uint(), key[0].bytes(), key[2].bytes())
                }
            val file = read[6].items()
            assertEquals(2uL, file[0].uint(), "$input records a second-generation read")
            val expected =
                file[1].items()
                    .flatMap { it.items()[16].items() }
                    .map { it.items() }
                    .filter { it[7].items().isNotEmpty() }
                    .associate { frame ->
                        frame[1].uint().toInt() to
                            frame[7].items().map { result ->
                                val fields = result.items()
                                val reason = fields[1].items().getOrNull(1)?.let { reasons.getValue(it.uint()) }
                                Outcome(statuses.getValue(fields[0].uint()), reason, fields[2].uint(), hex(fields[3].bytes()))
                            }
                    }
            Case(input, Path("../vectors/v2/P6/$input").readBytes(), held, expected)
        }
    }

    /** Verifies every signature of every ordinary frame that has an effective identifier, keyed by frame offset. */
    private fun verifyAll(case: Case, heldKeys: List<HeldKey> = case.heldKeys): Map<Int, List<Outcome>> {
        val version = assertIs<WireInspection.VersionTwo>(inspectWire(case.data))
        var headerIdentifier: ByteArray? = null
        var registry = 1uL
        val outcomes = mutableMapOf<Int, List<Outcome>>()
        version.events.filterIsInstance<WireEvent.Item>().forEach { item ->
            when (item.kind) {
                ItemKind.HEADER, ItemKind.RESYNC -> {
                    headerIdentifier = item.effectiveId?.bytes
                    registry = item.opener?.registry ?: registry
                }
                ItemKind.FRAME -> {
                    val frameIdentifier = item.effectiveId?.bytes
                    val h0 = headerIdentifier
                    if (frameIdentifier != null && h0 != null) {
                        val records = readFrame(CborField(case.data, item.offset, item.end), registry).payload.signatures
                        if (records.isNotEmpty()) {
                            outcomes[item.offset] =
                                records.map { record ->
                                    val result = verifySignature(record, frameIdentifier, h0, heldKeys)
                                    Outcome(result.status.name.lowercase(), result.reason, result.algorithm, hex(result.keyId))
                                }
                        }
                    }
                }
                else -> Unit
            }
        }
        return outcomes
    }

    private fun assertVectors(vararg names: String) {
        names.flatMap(::cases).forEach { case ->
            assertTrue(case.expected.isNotEmpty(), "${case.name} records at least one signature result")
            assertEquals(case.expected, verifyAll(case), case.name)
        }
    }

    @Test
    fun honestSignaturesOfBothAlgorithmsAreValid() {
        assertVectors(
            "ed25519-honest",
            "ed25519-honest-reproduce",
            "composite-honest",
            "composite-hedged",
            "sig-record-valid",
            "cosign-two-algorithms",
        )
    }

    @Test
    fun ed25519StrictReportsTheFirstFailingCheckInPolicyOrder() {
        assertVectors(
            "ed25519-scalar-above-l",
            "ed25519-scalar-equals-l",
            "ed25519-scalar-max",
            "ed25519-scalar-below-l",
            "ed25519-order-scalar-key",
            "ed25519-order-key-nonce",
            "ed25519-key-small-order-1",
            "ed25519-key-small-order-2",
            "ed25519-key-small-order-3",
            "ed25519-key-small-order-4",
            "ed25519-key-small-order-5",
            "ed25519-key-small-order-6",
            "ed25519-key-small-order-7",
            "ed25519-key-small-order-8",
            "ed25519-nonce-small-order-1",
            "ed25519-nonce-small-order-2",
            "ed25519-nonce-small-order-3",
            "ed25519-nonce-small-order-4",
            "ed25519-nonce-small-order-5",
            "ed25519-nonce-small-order-6",
            "ed25519-nonce-small-order-7",
            "ed25519-nonce-small-order-8",
            "proc-case-5",
            "proc-case-6",
            "proc-case-7",
        )
    }

    @Test
    fun ed25519EquationDecodesTheKeyStrictlyAndIsCofactorless() {
        assertVectors(
            "ed25519-key-off-curve",
            "ed25519-key-alias",
            "ed25519-key-non-canonical",
            "ed25519-key-non-canonical-sign",
            "ed25519-key-non-canonical-y",
            "ed25519-nonce-non-canonical",
            "ed25519-nonce-off-curve",
            "ed25519-equation-message",
            "ed25519-mixed-order-valid",
            "ed25519-mixed-order-cofactored-only",
            "proc-case-8",
            "proc-case-9",
        )
    }

    @Test
    fun compositeReportsTheClassicalHalfOverMPrimeBeforeThePostQuantumHalf() {
        assertVectors(
            "composite-both",
            "composite-classical-nonce",
            "composite-classical-scalar",
            "composite-classical-small-order",
            "composite-halves-two-subjects",
            "composite-label-wrong",
            "composite-prehash-wrong",
            "composite-context-nonempty",
            "proc-case-11",
            "proc-case-13",
        )
    }

    @Test
    fun compositePostQuantumHalfIsMlDsaVerifyWithTheLabelContext() {
        assertVectors(
            "composite-stripped-half",
            "composite-post-quantum",
            "composite-mldsa-context",
            "composite-hint-count-above-omega",
            "composite-hint-count-decreasing",
            "composite-hint-indices-unordered",
            "composite-hint-padding-nonzero",
            "proc-case-12",
        )
    }

    @Test
    fun wrongKeyOrValueLengthIsValueLengthBeforeAnyAlgorithmCheck() {
        assertVectors(
            "ed25519-length-63",
            "ed25519-length-65",
            "composite-length-3372",
            "composite-length-3374",
            "composite-key-1983",
            "composite-key-1985",
            "key-length-short",
            "proc-lengths-key-first",
            "proc-length-before-policy",
            "proc-value-length",
            "proc-case-14",
        )
    }

    @Test
    fun heldKeyStepsGiveKeyNotHeldOrKeyMismatchBeforeTheLengths() {
        assertVectors(
            "proc-key-not-held",
            "key-trusted-not-held",
            "proc-key-mismatch",
            "key-mismatch-algorithm",
            "key-mismatch-bytes",
            "subject-relabel-algorithm",
            "proc-mismatch-before-length",
            "key-mismatch-two-entries",
        )
    }

    @Test
    fun unknownAlgorithmIsUnverifiedWithItsKeyIdKept() {
        assertVectors("proc-unknown-algorithm", "proc-unknown-algorithm-long", "reg-unknown-algorithm-version")
    }

    @Test
    fun subjectBindsTheOpenerOfTheSegmentAndTheEffectiveIdentifierOfTheFrame() {
        assertVectors(
            "subject-healed-frame",
            "subject-other-frame",
            "subject-other-segment",
            "subject-relabel-key",
            "cov-healed",
            "cov-broken-signed-frame",
            "sig-list-independence",
            "sig-same-key-two-values",
        )
    }

    @Test
    fun heldKeysAreASetWhateverOrderOrRepetitionTheCallerGives() {
        listOf("key-held-order", "key-mismatch-two-entries", "sig-record-valid").flatMap(::cases).forEach { case ->
            val scrambled = case.heldKeys.reversed() + case.heldKeys
            assertEquals(case.expected, verifyAll(case, scrambled), case.name)
        }
    }
}
