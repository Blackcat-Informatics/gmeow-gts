// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

data class Sign1Parts(val kid: String, val protected: ByteArray, val signature: ByteArray) {
    override fun equals(other: Any?): Boolean =
        other is Sign1Parts &&
            kid == other.kid &&
            protected.contentEquals(other.protected) &&
            signature.contentEquals(other.signature)

    override fun hashCode(): Int {
        var result = kid.hashCode()
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
    val unprotected = body.value[1] as? CborMap ?: return null
    val signature = body.value[3].asBytes() ?: return null
    val kid = unprotected.getIntKey(COSE_KID_LABEL)?.asBytes()?.decodeToString() ?: return null
    return Sign1Parts(kid, protected, signature)
}

fun signatureKid(cose: ByteArray): String? = parseSign1(cose)?.kid

fun verifySig(cose: ByteArray, frameId: ByteArray, publicKey: ByteArray): SignatureStatus {
    val parts = parseSign1(cose) ?: return SignatureStatus.INVALID
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
