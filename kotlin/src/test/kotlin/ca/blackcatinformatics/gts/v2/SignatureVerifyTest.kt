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
 * P6 vector through the segment reader, which takes the `h0` of the opener of the segment and the
 * effective identifier of the frame and is given the held keys of the vector's recorded read. The
 * expected outcomes are the signature results of that recorded read.
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

    /** Reads the signature results of every ordinary frame through the segment reader, keyed by frame offset. */
    private fun verifyAll(case: Case, heldKeys: List<HeldKey> = case.heldKeys): Map<Int, List<Outcome>> =
        readSignatureResults(case.data, heldKeys).mapValues { (_, results) ->
            results.map { Outcome(it.status.name.lowercase(), it.reason, it.algorithm, hex(it.keyId)) }
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
    fun framesAfterAParityFrameAtWhichReadingResumesAreVerifiedAgainstTheAnchor() {
        // P5 7.2 step 6 and P1 3.4 step 4: the reader resumes at the parity frame after a damaged region
        // and takes its `anchor` as the predecessor, so the signed frame after it links and its subject is
        // its effective identifier, whether or not the anchor is the identifier of the frame before the region.
        assertVectors(
            "sig-detached-frame",
            "sig-two-after-detached-frame",
            "sig-commit-after-detached-frame",
            "sig-resumed-frame-anchor-true",
        )
    }

    @Test
    fun theParityFrameAtWhichReadingResumesIsTransparentAndCarriesNoSignatureResult() {
        // P6-3: a transparent frame is covered by no signature, and P5 7.2 7.2A: the damaged region is one item.
        val case = cases("sig-detached-frame").single()
        val items = (assertIs<WireInspection.VersionTwo>(inspectWire(case.data, heldKeys = case.heldKeys)).events).toList()
        val kinds = items.filterIsInstance<WireEvent.Item>().map { it.kind }
        assertEquals(1, kinds.count { it == ItemKind.DAMAGED })
        val parity = items.filterIsInstance<WireEvent.Item>().single { it.frameType == 16uL }
        assertEquals(FrameLink.LINKED, parity.link)
        assertEquals(null, parity.signatureResults)
        assertTrue(items.none { it is WireEvent.Stop }, "reading resumes and reaches the end of the input")
    }

    @Test
    fun heldKeysAreASetWhateverOrderOrRepetitionTheCallerGives() {
        listOf("key-held-order", "key-mismatch-two-entries", "sig-record-valid").flatMap(::cases).forEach { case ->
            val scrambled = case.heldKeys.reversed() + case.heldKeys
            assertEquals(case.expected, verifyAll(case, scrambled), case.name)
        }
    }

    @Test
    fun aFrameWithNoEffectiveIdentifierHasNoSubjectAndNoSignatureResult() {
        // P6 6.2 precondition: a transparent frame has no effective identifier, so its signature has no subject.
        val case = cases("sig-transparent-signature").single()
        assertEquals(emptyMap(), case.expected, "the recorded read carries no signature result")
        assertEquals(emptyMap(), verifyAll(case), case.name)
    }

    @Test
    fun aSignedFrameWhoseTransparentFieldIsNotABoolIsOrdinaryAndHasAResult() {
        // P1 3.4 and P6-3: the frame is ordinary, has an effective identifier, and so has a signature result.
        assertVectors("parity-transparent-not-bool-signed")
    }

    @Test
    fun aSegmentWhoseOpenerIsNotReadableGetsNoSignatureResult() {
        // P6 6.2 precondition: without h0 there is no subject. The registry hash of the opener is zero, which no opener may carry.
        val case = cases("sig-record-valid").single()
        assertTrue(verifyAll(case).isNotEmpty(), "the unmodified input has signature results")
        val data = case.data.copyOf().also { it[4] = 0 }
        assertEquals(emptyMap(), readSignatureResults(data, case.heldKeys))
    }

    @Test
    fun noHeldKeysArgumentMeansNoSignatureIsVerified() {
        val case = cases("sig-record-valid").single()
        val wire = assertIs<WireInspection.VersionTwo>(inspectWire(case.data))
        assertTrue(wire.events.filterIsInstance<WireEvent.Item>().all { it.signatureResults == null })
    }

    /**
     * The halves of two valid composite signatures over one subject, swapped: the ML-DSA half of
     * the first with the Ed25519 half of the second. The classical half refuses the composite
     * first (P6-26). The second signature is recorded in the engine tests of each engine, not as a
     * corpus vector.
     */
    @Test
    fun compositeHalvesOfTwoValidSignaturesDoNotCombine() {
        val value = Path("../vectors/v2/P6/sign-composite-reproduce.value.bin").readBytes()
        val publicKey = Path("../vectors/v2/P6/sign-composite-reproduce.public-key.bin").readBytes()
        val held = HeldKey(2uL, parseHex(COMPOSITE_KEY_ID), publicKey)
        val frame = ByteArray(32) { (32 + it).toByte() }
        val opener = ByteArray(32) { it.toByte() }
        val valid =verifySignature(SignatureRecord(2uL, held.keyId, value), frame, opener, listOf(held))
        assertEquals(SignatureStatus.VALID, valid.status)
        val swapped = value.copyOfRange(0, 3309) + parseHex(SECOND_ED25519_HALF)
        val outcome = verifySignature(SignatureRecord(2uL, held.keyId, swapped), frame, opener, listOf(held))
        assertEquals(SignatureStatus.INVALID, outcome.status)
        assertEquals("composite_classical_failed", outcome.reason)
    }

    /**
     * A composite is not verified from one of its halves: under the Ed25519 key of its own half
     * alone it has no key of its identifier, and under its identifier with the Ed25519 bytes the
     * key is inconsistent (P6-33, key-mismatch-bytes).
     */
    @Test
    fun compositeHeldUnderOneComponentKeyIsNotVerified() {
        val value = Path("../vectors/v2/P6/sign-composite-reproduce.value.bin").readBytes()
        val keyId = parseHex(COMPOSITE_KEY_ID)
        val frame = ByteArray(32) { (32 + it).toByte() }
        val opener = ByteArray(32) { it.toByte() }
        val record =SignatureRecord(2uL, keyId, value)
        val edKey = HeldKey(1uL, parseHex(ED25519_KEY_ID), parseHex(ED25519_PUBLIC_KEY))
        val notHeld = verifySignature(record, frame, opener, listOf(edKey))
        assertEquals(SignatureStatus.UNVERIFIED, notHeld.status)
        assertEquals("key_not_held", notHeld.reason)
        val mismatch = HeldKey(2uL, keyId, parseHex(ED25519_PUBLIC_KEY))
        val inconsistent = verifySignature(record, frame, opener, listOf(mismatch))
        assertEquals(SignatureStatus.INVALID, inconsistent.status)
        assertEquals("key_mismatch", inconsistent.reason)
    }

    private companion object {
        const val COMPOSITE_KEY_ID = "84df8ff51c012ca6b492adcb07f47a642690fddb128749db3a8de31b76ac3301"
        const val ED25519_KEY_ID = "b9022e9133a94884b0a42a97fb0a59ee47d77837824da3323b7be9af27ee1133"
        const val ED25519_PUBLIC_KEY = "174553b456dddfc6908ecab1c101fe6ab21e2baa0617795b7d43a63482993fd5"
        const val SECOND_ED25519_HALF =
            "b904b9acfa7c6c9194fe0e754027fb95919fd0c604de640a0f0027de0fa877c2" +
                "98d2a55a54cb9a46e1f0774392c725d8003e8a6894efbc14a27755155a990e02"
    }
}
