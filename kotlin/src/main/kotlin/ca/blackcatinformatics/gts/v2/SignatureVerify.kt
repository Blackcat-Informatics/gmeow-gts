// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.encodeUnsignedHead
import java.io.ByteArrayOutputStream
import java.math.BigInteger
import java.security.MessageDigest
import java.util.Arrays
import org.bouncycastle.crypto.params.MLDSAParameters
import org.bouncycastle.crypto.params.MLDSAPublicKeyParameters
import org.bouncycastle.crypto.params.ParametersWithContext
import org.bouncycastle.crypto.signers.MLDSASigner

private const val ED25519 = 1uL
private const val COMPOSITE = 2uL
private const val DIGEST_LENGTH = 32
private const val MLDSA_KEY_LENGTH = 1952
private const val MLDSA_VALUE_LENGTH = 3309
private val keyLengths = mapOf(ED25519 to 32, COMPOSITE to 1984)
private val valueLengths = mapOf(ED25519 to 64, COMPOSITE to 3373)
private val compositePrefix = "CompositeAlgorithmSignatures2025".encodeToByteArray()
private val compositeLabel = "COMPSIG-MLDSA65-Ed25519-SHA512".encodeToByteArray()

/**
 * A key the verifier can check a signature with (P6 section 3.3): the `key_id` the caller files
 * it under, its algorithm code and its public key bytes. The `key_id` is the caller's claim; a
 * held key is used for a signature only when it is consistent with it.
 */
internal class HeldKey(val algorithm: ULong, val keyId: ByteArray, val publicKey: ByteArray) {
    /** `E(held_key)`, the encoding `[key_id, algorithm, public_key]` that orders the set of held keys. */
    val encoding: ByteArray =
        ByteArrayOutputStream().apply {
            write(0x83)
            writeByteString(keyId)
            encodeUnsignedHead(this, 0, algorithm)
            writeByteString(publicKey)
        }.toByteArray()
}

internal enum class SignatureStatus { VALID, INVALID, UNVERIFIED }

/**
 * The outcome of one signature (P6 section 6.1): its status, the P2 reason spelling (absent for
 * [SignatureStatus.VALID]), and the algorithm code and `key_id` of the record as read.
 */
internal class SignatureResult(val status: SignatureStatus, val reason: String?, val algorithm: ULong, val keyId: ByteArray)

/**
 * Runs `Verify(signature, frame)` of P6 section 6.2 for one signature of one ordinary frame and
 * returns the outcome of the first step that gives one. [frameIdentifier] is the frame's
 * effective identifier and [headerIdentifier] the `h0` of the opener of its segment, both 32
 * bytes. [heldKeys] is read as a set: it is sorted by `E(held_key)` and repeats are dropped
 * before any step, so the result does not depend on the order or repetition the caller gives.
 *
 * Throws [IllegalArgumentException] when either identifier is not 32 bytes.
 */
internal fun verifySignature(
    record: SignatureRecord,
    frameIdentifier: ByteArray,
    headerIdentifier: ByteArray,
    heldKeys: List<HeldKey>,
): SignatureResult {
    require(frameIdentifier.size == DIGEST_LENGTH && headerIdentifier.size == DIGEST_LENGTH) {
        "a signature subject takes 32-byte identifiers"
    }
    fun outcome(status: SignatureStatus, reason: String?) = SignatureResult(status, reason, record.algorithm, record.keyId)
    val algorithm = record.algorithm
    val keyLength = keyLengths[algorithm] ?: return outcome(SignatureStatus.UNVERIFIED, "unknown_signature_algorithm")
    val filed = heldKeySet(heldKeys).filter { it.keyId.contentEquals(record.keyId) }
    if (filed.isEmpty()) return outcome(SignatureStatus.UNVERIFIED, "key_not_held")
    val key =
        filed.firstOrNull { it.algorithm == algorithm && keyIdentifier(algorithm, it.publicKey).contentEquals(record.keyId) }
            ?: return outcome(SignatureStatus.INVALID, "key_mismatch")
    if (key.publicKey.size != keyLength || record.value.size != valueLengths.getValue(algorithm)) {
        return outcome(SignatureStatus.INVALID, "value_length")
    }
    val message = signatureMessage(algorithm, record.keyId, headerIdentifier, frameIdentifier)
    val reason =
        if (algorithm == ED25519) {
            Ed25519Strict.check(key.publicKey, message, record.value)
        } else {
            compositeStrict(key.publicKey, message, record.value)
        }
    return outcome(if (reason == null) SignatureStatus.VALID else SignatureStatus.INVALID, reason)
}

private fun heldKeySet(keys: List<HeldKey>): List<HeldKey> {
    val sorted = keys.sortedWith { left, right -> Arrays.compareUnsigned(left.encoding, right.encoding) }
    return sorted.filterIndexed { index, key -> index == 0 || !key.encoding.contentEquals(sorted[index - 1].encoding) }
}

/** `H_ctx(key_id, E([algorithm, public_key]))` of P6 section 3.2. */
private fun keyIdentifier(algorithm: ULong, publicKey: ByteArray): ByteArray {
    val preimage =
        ByteArrayOutputStream().apply {
            write(0x82)
            encodeUnsignedHead(this, 0, algorithm)
            writeByteString(publicKey)
        }.toByteArray()
    return contextHash("gts2 key id") { update(preimage, 0, preimage.size) }
}

/** `H_ctx(signature_subject, E([algorithm, key_id, h0, frame_identifier]))` of P6 section 2. */
private fun signatureMessage(algorithm: ULong, keyId: ByteArray, headerIdentifier: ByteArray, frameIdentifier: ByteArray): ByteArray {
    val subject =
        ByteArrayOutputStream().apply {
            write(0x84)
            encodeUnsignedHead(this, 0, algorithm)
            writeByteString(keyId)
            writeByteString(headerIdentifier)
            writeByteString(frameIdentifier)
        }.toByteArray()
    return contextHash("gts2 signature subject") { update(subject, 0, subject.size) }
}

private fun ByteArrayOutputStream.writeByteString(bytes: ByteArray) {
    encodeUnsignedHead(this, 2, bytes.size.toULong())
    write(bytes, 0, bytes.size)
}

private fun sha512(vararg parts: ByteArray): ByteArray {
    val digest = MessageDigest.getInstance("SHA-512")
    parts.forEach(digest::update)
    return digest.digest()
}

/**
 * `Composite-Strict` of P6 section 4.2: the `Ed25519` half by [Ed25519Strict] over `M'`, then
 * the ML-DSA-65 half by FIPS 204 `ML-DSA.Verify` over `M'` with `Label` as its context.
 * Lengths are checked by the caller.
 */
private fun compositeStrict(publicKey: ByteArray, message: ByteArray, value: ByteArray): String? {
    val representative = compositePrefix + compositeLabel + byteArrayOf(0) + sha512(message)
    val classical =
        Ed25519Strict.check(
            publicKey.copyOfRange(MLDSA_KEY_LENGTH, publicKey.size),
            representative,
            value.copyOfRange(MLDSA_VALUE_LENGTH, value.size),
        )
    if (classical != null) return "composite_classical_failed"
    val verifier = MLDSASigner()
    val key = MLDSAPublicKeyParameters(MLDSAParameters.ml_dsa_65, publicKey.copyOfRange(0, MLDSA_KEY_LENGTH))
    verifier.init(false, ParametersWithContext(key, compositeLabel))
    verifier.update(representative, 0, representative.size)
    return if (verifier.verifySignature(value.copyOfRange(0, MLDSA_VALUE_LENGTH))) null else "composite_post_quantum_failed"
}

/**
 * `Ed25519-Strict` of P6 section 4.1, computed over the curve as that section states it rather
 * than through a library, so that the checks SCALAR, KEY, NONCE and EQUATION and their order are
 * exactly the policy's. The equation is the cofactorless `ENC([S]B - [k]A) == value[0:32]`.
 */
private object Ed25519Strict {
    private val p = BigInteger.ONE.shiftLeft(255) - BigInteger.valueOf(19)
    private val order = BigInteger.ONE.shiftLeft(252) + BigInteger("27742317777372353535851937790883648493")
    private val d = (BigInteger.valueOf(-121665) * BigInteger.valueOf(121666).modInverse(p)).mod(p)
    private val sqrtMinusOne = BigInteger.TWO.modPow((p - BigInteger.ONE).shiftRight(2), p)
    private val yMask = BigInteger.ONE.shiftLeft(255) - BigInteger.ONE
    private val identity = Point(BigInteger.ZERO, BigInteger.ONE, BigInteger.ONE, BigInteger.ZERO)
    private val base =
        BigInteger("15112221349535400772501151409588531511454012693041857206046113283949847762202").let { x ->
            val y = (BigInteger.valueOf(4) * BigInteger.valueOf(5).modInverse(p)).mod(p)
            Point(x, y, BigInteger.ONE, (x * y).mod(p))
        }
    private val smallOrder =
        listOf(
            "0100000000000000000000000000000000000000000000000000000000000000",
            "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080",
            "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
            "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
            "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
            "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
        ).map { hex -> ByteArray(32) { hex.substring(2 * it, 2 * it + 2).toInt(16).toByte() } }

    /** Extended twisted Edwards coordinates: `x = X/Z`, `y = Y/Z`, `x * y = T/Z`. */
    private class Point(val x: BigInteger, val y: BigInteger, val z: BigInteger, val t: BigInteger)

    /** The first failing check of the policy as its P2 reason, or `null` when the signature holds. */
    fun check(publicKey: ByteArray, message: ByteArray, value: ByteArray): String? {
        val nonce = value.copyOfRange(0, 32)
        val scalar = littleEndian(value.copyOfRange(32, 64))
        return when {
            scalar >= order -> "non_canonical_scalar"
            smallOrder.any { it.contentEquals(publicKey) } -> "small_order_key"
            smallOrder.any { it.contentEquals(nonce) } -> "small_order_nonce"
            else -> {
                val key = decode(publicKey) ?: return "equation_failed"
                val challenge = littleEndian(sha512(nonce, publicKey, message)).mod(order)
                val scaled = multiply(key, challenge)
                val negated = Point((p - scaled.x).mod(p), scaled.y, scaled.z, (p - scaled.t).mod(p))
                val recovered = add(multiply(base, scalar), negated)
                if (encode(recovered).contentEquals(nonce)) null else "equation_failed"
            }
        }
    }

    private fun littleEndian(bytes: ByteArray): BigInteger = BigInteger(1, bytes.reversedArray())

    /** The DECODING of P6 section 4.1 (RFC 8032 section 5.1.3), or `null` when it fails. */
    private fun decode(encoded: ByteArray): Point? {
        val number = littleEndian(encoded)
        val y = number.and(yMask)
        val sign = number.testBit(255)
        if (y >= p) return null
        val yy = (y * y).mod(p)
        val square = ((yy - BigInteger.ONE) * (d * yy + BigInteger.ONE).mod(p).modInverse(p)).mod(p)
        var x = square.modPow((p + BigInteger.valueOf(3)).shiftRight(3), p)
        if ((x * x).mod(p) != square) x = (x * sqrtMinusOne).mod(p)
        if ((x * x).mod(p) != square || (x.signum() == 0 && sign)) return null
        if (x.testBit(0) != sign) x = p - x
        return Point(x, y, BigInteger.ONE, (x * y).mod(p))
    }

    /** `ENC(P)`: the 32 little-endian bytes of `y + 2^255 * (x mod 2)`. */
    private fun encode(point: Point): ByteArray {
        val inverse = point.z.modInverse(p)
        val x = (point.x * inverse).mod(p)
        val y = (point.y * inverse).mod(p)
        val number = if (x.testBit(0)) y.setBit(255) else y
        val bigEndian = number.toByteArray()
        val out = ByteArray(32)
        for (index in 0 until minOf(32, bigEndian.size)) out[index] = bigEndian[bigEndian.size - 1 - index]
        return out
    }

    /** The complete addition of RFC 8032 section 5.1.4 in extended coordinates. */
    private fun add(left: Point, right: Point): Point {
        val a = ((left.y - left.x) * (right.y - right.x)).mod(p)
        val b = ((left.y + left.x) * (right.y + right.x)).mod(p)
        val c = (BigInteger.TWO * d * left.t * right.t).mod(p)
        val dd = (BigInteger.TWO * left.z * right.z).mod(p)
        val e = b - a
        val f = dd - c
        val g = dd + c
        val h = b + a
        return Point((e * f).mod(p), (g * h).mod(p), (f * g).mod(p), (e * h).mod(p))
    }

    private fun multiply(point: Point, scalar: BigInteger): Point {
        var result = identity
        var addend = point
        for (bit in 0 until scalar.bitLength()) {
            if (scalar.testBit(bit)) result = add(result, addend)
            addend = add(addend, addend)
        }
        return result
    }
}
