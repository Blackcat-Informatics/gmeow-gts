// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts

import java.security.SecureRandom
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer

private const val COSE_ALG_LABEL = 1L
private const val COSE_KID_LABEL = 4L
private const val COSE_IV_LABEL = 5L
private const val COSE_ALG_EDDSA = -8L
private const val COSE_ALG_A256GCM = 3L
private const val COSE_TAG_SIGN1 = 18L
private const val COSE_TAG_ENCRYPT0 = 16L

data class CoseSigner(val kid: String, val seed: ByteArray) {
    init {
        require(seed.size == 32) { "Ed25519 seed must be 32 bytes" }
    }

    override fun equals(other: Any?): Boolean = other is CoseSigner && kid == other.kid && seed.contentEquals(other.seed)

    override fun hashCode(): Int = 31 * kid.hashCode() + seed.contentHashCode()
}

/**
 * A well-formed COSE_Sign1. [algorithm] is the `alg` declared in the protected
 * header: an integer ([CborUInt] or [CborNInt]) or a text string ([CborText]).
 */
data class Sign1Parts(
    val kid: String,
    val algorithm: CborValue,
    val protected: ByteArray,
    val signature: ByteArray,
) {
    override fun equals(other: Any?): Boolean =
        other is Sign1Parts &&
            kid == other.kid &&
            algorithm == other.algorithm &&
            protected.contentEquals(other.protected) &&
            signature.contentEquals(other.signature)

    override fun hashCode(): Int {
        var result = kid.hashCode()
        result = 31 * result + algorithm.hashCode()
        result = 31 * result + protected.contentHashCode()
        result = 31 * result + signature.contentHashCode()
        return result
    }
}

enum class SignatureStatus(val wire: String) {
    INVALID("invalid"),
    VALID("valid"),
    UNVERIFIED("unverified"),
}

class Encrypt0Exception(
    message: String,
    val reason: String,
) : RuntimeException(message)

fun signId(frameId: ByteArray, signer: CoseSigner): ByteArray {
    val protected = sign1Protected()
    val payload = sigStructure(protected, frameId)
    val key = Ed25519PrivateKeyParameters(signer.seed, 0)
    val sign = Ed25519Signer()
    sign.init(true, key)
    sign.update(payload, 0, payload.size)
    return encode(
        CborTag(
            COSE_TAG_SIGN1,
            cborArray(
                bytes(protected),
                cborMap(uint(COSE_KID_LABEL) to bytes(signer.kid.encodeToByteArray())),
                CborNull,
                bytes(sign.generateSignature()),
            ),
        ),
    )
}

fun publicKeyFromSeed(seed: ByteArray): ByteArray {
    require(seed.size == 32) { "Ed25519 seed must be 32 bytes" }
    return Ed25519PrivateKeyParameters(seed, 0).generatePublicKey().encoded
}

fun parseSign1(cose: ByteArray): Sign1Parts? {
    val tag = decodeOrNull(cose) as? CborTag ?: return null
    if (tag.tag != COSE_TAG_SIGN1) return null
    val body = tag.value as? CborArray ?: return null
    if (body.value.size != 4) return null
    val protected = body.value[0].asBytes() ?: return null
    val algorithm = declaredAlgorithm(protected) ?: return null
    val unprotected = body.value[1] as? CborMap ?: return null
    val signature = body.value[3].asBytes() ?: return null
    val kid = unprotected.getIntKey(COSE_KID_LABEL)?.asBytes()?.decodeToString() ?: return null
    return Sign1Parts(kid, algorithm, protected, signature)
}

private const val MAX_HEADER_DEPTH = 8

/** One scanned item head: major type, additional info, argument (never above Long.MAX_VALUE) and next offset. */
private class Head(val major: Int, val info: Int, val arg: Long, val next: Int)

/**
 * Read one item head at [pos]. Null for an indefinite length, a reserved additional info, a truncated head, an
 * argument that is not in shortest form, or an argument of 2^63 or more (no valid header item needs one).
 */
private fun scanHead(buf: ByteArray, pos: Int): Head? {
    if (pos >= buf.size) return null
    val major = (buf[pos].toInt() and 0xff) shr 5
    val info = buf[pos].toInt() and 0x1f
    if (info < 24) return Head(major, info, info.toLong(), pos + 1)
    if (info > 27) return null
    val width = 1 shl (info - 24)
    if (buf.size - pos - 1 < width) return null
    if (width == 8 && buf[pos + 1].toInt() < 0) return null
    var arg = 0L
    for (i in 0 until width) arg = (arg shl 8) or (buf[pos + 1 + i].toLong() and 0xff)
    val minimum = longArrayOf(24L, 1L shl 8, 1L shl 16, 1L shl 32)[info - 24]
    if (arg < minimum) return null
    return Head(major, info, arg, pos + 1 + width)
}

private fun compareBytes(a: ByteArray, b: ByteArray): Int {
    for (i in 0 until minOf(a.size, b.size)) {
        val d = (a[i].toInt() and 0xff) - (b[i].toInt() and 0xff)
        if (d != 0) return d
    }
    return a.size - b.size
}

private fun validUtf8(buf: ByteArray, from: Int, to: Int): Boolean =
    try {
        Charsets.UTF_8.newDecoder()
            .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
            .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
            .decode(java.nio.ByteBuffer.wrap(buf, from, to - from))
        true
    } catch (e: java.nio.charset.CharacterCodingException) {
        false
    }

/**
 * Check one header item against the grammar; returns the offset after it, or -1. [depth] is the level of this
 * item if it is a container. Recursion is bounded by [MAX_HEADER_DEPTH].
 */
private fun scanItem(buf: ByteArray, pos: Int, depth: Int, isKey: Boolean): Int {
    val head = scanHead(buf, pos) ?: return -1
    if (isKey && head.major != 0 && head.major != 1 && head.major != 3) return -1
    when (head.major) {
        // Integers lie in -2^63 .. 2^63-1; scanHead already refused an argument above Long.MAX_VALUE.
        0, 1 -> return head.next
        2, 3 -> {
            if (head.arg > (buf.size - head.next).toLong()) return -1
            val end = head.next + head.arg.toInt()
            if (head.major == 3 && !validUtf8(buf, head.next, end)) return -1
            return end
        }
        4, 5 -> {
            if (depth > MAX_HEADER_DEPTH) return -1
            var at = head.next
            var previous: ByteArray? = null
            var i = 0L
            while (i < head.arg) {
                if (head.major == 4) {
                    at = scanItem(buf, at, depth + 1, false)
                    if (at < 0) return -1
                } else {
                    val keyEnd = scanItem(buf, at, depth + 1, true)
                    if (keyEnd < 0) return -1
                    val key = buf.copyOfRange(at, keyEnd)
                    // Strictly ascending bytewise: sorted, and no duplicate key.
                    if (previous != null && compareBytes(previous, key) >= 0) return -1
                    previous = key
                    at = scanItem(buf, keyEnd, depth + 1, false)
                    if (at < 0) return -1
                }
                i++
            }
            return at
        }
        // null, true and false only.
        7 -> return if (head.info in 20..22) head.next else -1
        else -> return -1
    }
}

/**
 * The `alg` (label 1) of a protected header (RFC 9052 section 3.1).
 *
 * The protected header is a byte string whose content is deterministically encoded CBOR (definite lengths,
 * shortest-form integers and lengths, map keys in bytewise order of their encodings, no duplicate keys, no
 * trailing bytes) consisting of exactly one map. Every map key, at any depth, is an integer or a text string.
 * Every value, at any depth, is an integer, a byte string, a text string, an array, a map, null, true or false.
 * Every integer, key or value, lies in the range -2^63 to 2^63-1 inclusive. Text strings are valid UTF-8.
 * Nesting is at most 8 levels deep, counting the header map itself as level 1. The map contains the integer
 * key 1, whose value is an integer or a text string. A header that does not meet all of this is malformed and
 * the signature is invalid.
 *
 * Enforced by scanning the raw bytes with [scanItem], so no decoder behaviour is involved. Null if malformed
 * or no algorithm is declared; never throws.
 */
private fun declaredAlgorithm(protected: ByteArray): CborValue? =
    try {
        scanDeclaredAlgorithm(protected)
    } catch (e: RuntimeException) {
        null
    }

private fun scanDeclaredAlgorithm(protected: ByteArray): CborValue? {
    if (protected.isEmpty() || (protected[0].toInt() and 0xff) shr 5 != 5) return null
    if (scanItem(protected, 0, 1, false) != protected.size) return null
    val root = scanHead(protected, 0) ?: return null
    var at = root.next
    var i = 0L
    while (i < root.arg) {
        val keyEnd = scanItem(protected, at, 2, true)
        val valueEnd = scanItem(protected, keyEnd, 2, false)
        if (keyEnd - at == 1 && protected[at].toInt() == 0x01) {
            val value = scanHead(protected, keyEnd) ?: return null
            return when (value.major) {
                0 -> CborUInt(value.arg)
                1 -> CborNInt(-1L - value.arg)
                3 -> CborText(String(protected, value.next, valueEnd - value.next, Charsets.UTF_8))
                else -> null
            }
        }
        at = valueEnd
        i++
    }
    return null
}

fun signatureKid(cose: ByteArray): String? = parseSign1(cose)?.kid

/**
 * Verify a detached COSE_Sign1, dispatching on the algorithm declared in the protected header (GTS-SPEC 9.2):
 * EdDSA is verified as Ed25519, any other algorithm is UNVERIFIED, and a malformed COSE_Sign1 is INVALID.
 */
fun verifySig(cose: ByteArray, frameId: ByteArray, publicKey: ByteArray): SignatureStatus {
    val parts = parseSign1(cose) ?: return SignatureStatus.INVALID
    if (parts.algorithm != CborNInt(COSE_ALG_EDDSA)) return SignatureStatus.UNVERIFIED
    if (parts.signature.size != 64 || publicKey.size != 32) return SignatureStatus.INVALID
    val payload = sigStructure(parts.protected, frameId)
    val verify = Ed25519Signer()
    verify.init(false, Ed25519PublicKeyParameters(publicKey, 0))
    verify.update(payload, 0, payload.size)
    return if (verify.verifySignature(parts.signature)) SignatureStatus.VALID else SignatureStatus.INVALID
}

fun verifySignatures(signatures: MutableList<Signature>, resolve: (String) -> ByteArray?) {
    for (idx in signatures.indices) {
        val cose = signatures[idx].cose ?: continue
        val kid = signatureKid(cose)
        if (kid == null) {
            signatures[idx] = signatures[idx].copy(kid = "", status = SignatureStatus.INVALID.wire)
            continue
        }
        val publicKey = resolve(kid)
        if (publicKey == null) {
            signatures[idx] = signatures[idx].copy(kid = kid, status = SignatureStatus.UNVERIFIED.wire)
            continue
        }
        val status = verifySig(cose, signatures[idx].frameId, publicKey).wire
        signatures[idx] = signatures[idx].copy(kid = kid, status = status)
    }
}

fun encrypt0WithIv(plaintext: ByteArray, kid: String, key: ByteArray, iv: ByteArray): ByteArray {
    require(key.size == 32) { "COSE_Encrypt0 content key must be 32 bytes" }
    require(iv.size == 12) { "COSE_Encrypt0 IV must be 12 bytes" }
    val protected = encrypt0Protected()
    val ciphertext = aesGcmEncrypt(plaintext, key, iv, encStructure(protected))
    return encode(
        CborTag(
            COSE_TAG_ENCRYPT0,
            cborArray(
                bytes(protected),
                cborMap(
                    uint(COSE_KID_LABEL) to bytes(kid.encodeToByteArray()),
                    uint(COSE_IV_LABEL) to bytes(iv),
                ),
                bytes(ciphertext),
            ),
        ),
    )
}

fun encrypt0(plaintext: ByteArray, kid: String, key: ByteArray): ByteArray {
    val iv = ByteArray(12)
    SecureRandom().nextBytes(iv)
    return encrypt0WithIv(plaintext, kid, key, iv)
}

fun recipientKid(cose: ByteArray): String? = parseEncrypt0(cose)?.kid

fun decrypt0(cose: ByteArray, resolve: (String) -> ByteArray?): ByteArray {
    val parts = parseEncrypt0(cose) ?: throw Encrypt0Exception("malformed COSE_Encrypt0", "malformed")
    val key = resolve(parts.kid)
    if (key == null || key.size != 32) {
        throw Encrypt0Exception("no content key for ${parts.kid}", "missing-key")
    }
    if (parts.iv.size != 12) {
        throw Encrypt0Exception("bad COSE_Encrypt0 IV length", "malformed")
    }
    return try {
        aesGcmDecrypt(parts.ciphertext, key, parts.iv, encStructure(parts.protected))
    } catch (_: AEADBadTagException) {
        throw Encrypt0Exception("authentication failed (AES-GCM tag mismatch)", "auth-failed")
    }
}

private data class Encrypt0Parts(
    val kid: String,
    val protected: ByteArray,
    val iv: ByteArray,
    val ciphertext: ByteArray,
)

private fun sign1Protected(): ByteArray = encode(cborMap(uint(COSE_ALG_LABEL) to CborNInt(COSE_ALG_EDDSA)))

private fun sigStructure(protected: ByteArray, frameId: ByteArray): ByteArray =
    encode(cborArray(text("Signature1"), bytes(protected), bytes(ByteArray(0)), bytes(frameId)))

private fun encrypt0Protected(): ByteArray = encode(cborMap(uint(COSE_ALG_LABEL) to uint(COSE_ALG_A256GCM)))

private fun encStructure(protected: ByteArray): ByteArray =
    encode(cborArray(text("Encrypt0"), bytes(protected), bytes(ByteArray(0))))

private fun parseEncrypt0(cose: ByteArray): Encrypt0Parts? {
    val tag = decodeOrNull(cose) as? CborTag ?: return null
    if (tag.tag != COSE_TAG_ENCRYPT0) return null
    val body = tag.value as? CborArray ?: return null
    if (body.value.size != 3) return null
    val protected = body.value[0].asBytes() ?: return null
    val unprotected = body.value[1] as? CborMap ?: return null
    val ciphertext = body.value[2].asBytes() ?: return null
    val kid = unprotected.getIntKey(COSE_KID_LABEL)?.asBytes()?.decodeToString() ?: return null
    val iv = unprotected.getIntKey(COSE_IV_LABEL)?.asBytes() ?: return null
    return Encrypt0Parts(kid, protected, iv, ciphertext)
}

private fun CborMap.getIntKey(key: Long): CborValue? =
    value.firstOrNull { (k, _) ->
        when (k) {
            is CborUInt -> k.value == key
            is CborNInt -> k.value == key
            else -> false
        }
    }?.second

private fun decodeOrNull(data: ByteArray): CborValue? =
    try {
        decode(data)
    } catch (_: RuntimeException) {
        null
    }

private fun aesGcmEncrypt(plaintext: ByteArray, key: ByteArray, iv: ByteArray, aad: ByteArray): ByteArray {
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(128, iv))
    cipher.updateAAD(aad)
    return cipher.doFinal(plaintext)
}

private fun aesGcmDecrypt(ciphertext: ByteArray, key: ByteArray, iv: ByteArray, aad: ByteArray): ByteArray {
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(128, iv))
    cipher.updateAAD(aad)
    return cipher.doFinal(ciphertext)
}
