/*
 * SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
 * SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
 */

#include <stddef.h>
#include <stdint.h>

#include <openssl/core_names.h>
#include <openssl/err.h>
#include <openssl/evp.h>
#include <openssl/evperr.h>
#include <openssl/params.h>
#include <sodium.h>

#if OPENSSL_VERSION_NUMBER < 0x30500010L
#error "OpenSSL 3.5.1 or later is required for ML-DSA-65 verification"
#endif

int gts_sodium_init(void) {
    return sodium_init();
}

int gts_sha512(const uint8_t *message, size_t message_len, uint8_t *digest) {
    return crypto_hash_sha512(digest, message, (unsigned long long)message_len);
}

int gts_mldsa65_verify(
    const uint8_t *signature,
    const uint8_t *message,
    size_t message_len,
    const uint8_t *public_key,
    const uint8_t *context,
    size_t context_len
) {
    EVP_PKEY_CTX *import = NULL;
    EVP_PKEY_CTX *verify = NULL;
    EVP_PKEY *key = NULL;
    EVP_SIGNATURE *algorithm = NULL;
    int result = -1;
    unsigned long error;
    OSSL_PARAM public_params[] = {
        OSSL_PARAM_octet_string(OSSL_PKEY_PARAM_PUB_KEY, (void *)public_key, 1952),
        OSSL_PARAM_END
    };
    OSSL_PARAM signature_params[] = {
        OSSL_PARAM_octet_string(OSSL_SIGNATURE_PARAM_CONTEXT_STRING, (void *)context, context_len),
        OSSL_PARAM_END
    };

    /* This shim owns the OpenSSL error queue for the duration of the call. */
    ERR_clear_error();
    import = EVP_PKEY_CTX_new_from_name(NULL, "ML-DSA-65", NULL);
    if (import == NULL || EVP_PKEY_fromdata_init(import) <= 0 ||
        EVP_PKEY_fromdata(import, &key, EVP_PKEY_PUBLIC_KEY, public_params) <= 0) {
        goto cleanup;
    }

    algorithm = EVP_SIGNATURE_fetch(NULL, "ML-DSA-65", NULL);
    verify = EVP_PKEY_CTX_new_from_pkey(NULL, key, NULL);
    if (algorithm == NULL || verify == NULL ||
        EVP_PKEY_verify_message_init(verify, algorithm, signature_params) <= 0) {
        goto cleanup;
    }

    ERR_clear_error();
    result = EVP_PKEY_verify(verify, signature, 3309, message, message_len);
    /* EVP also queues a generic provider failure for an ordinary rejection.
     * A specific underlying error instead means the verification failed to run. */
    while ((error = ERR_get_error()) != 0) {
        if (result == 0 &&
            (ERR_GET_LIB(error) != ERR_LIB_EVP || ERR_GET_REASON(error) != EVP_R_PROVIDER_SIGNATURE_FAILURE)) {
            result = -1;
        }
    }

cleanup:
    EVP_PKEY_CTX_free(verify);
    EVP_SIGNATURE_free(algorithm);
    EVP_PKEY_free(key);
    EVP_PKEY_CTX_free(import);
    ERR_clear_error();
    return result;
}

size_t gts_ed25519_publickeybytes(void) {
    return crypto_sign_ed25519_PUBLICKEYBYTES;
}

size_t gts_ed25519_secretkeybytes(void) {
    return crypto_sign_ed25519_SECRETKEYBYTES;
}

size_t gts_ed25519_seedbytes(void) {
    return crypto_sign_ed25519_SEEDBYTES;
}

size_t gts_ed25519_signaturebytes(void) {
    return crypto_sign_ed25519_BYTES;
}

int gts_ed25519_seed_keypair(const uint8_t *seed, uint8_t *public_key, uint8_t *secret_key) {
    return crypto_sign_ed25519_seed_keypair(public_key, secret_key, seed);
}

int gts_ed25519_sign_detached(
    const uint8_t *message,
    size_t message_len,
    const uint8_t *secret_key,
    uint8_t *signature
) {
    unsigned long long signature_len = 0;
    int result = crypto_sign_ed25519_detached(
        signature,
        &signature_len,
        message,
        (unsigned long long)message_len,
        secret_key
    );
    if (result != 0) {
        return result;
    }
    return signature_len == crypto_sign_ed25519_BYTES ? 0 : -1;
}

int gts_ed25519_verify_detached(
    const uint8_t *signature,
    const uint8_t *message,
    size_t message_len,
    const uint8_t *public_key
) {
    return crypto_sign_ed25519_verify_detached(
        signature,
        message,
        (unsigned long long)message_len,
        public_key
    );
}

int gts_aes256gcm_is_available(void) {
    return crypto_aead_aes256gcm_is_available();
}

size_t gts_aes256gcm_keybytes(void) {
    return crypto_aead_aes256gcm_KEYBYTES;
}

size_t gts_aes256gcm_noncebytes(void) {
    return crypto_aead_aes256gcm_NPUBBYTES;
}

size_t gts_aes256gcm_macbytes(void) {
    return crypto_aead_aes256gcm_ABYTES;
}

int gts_aes256gcm_encrypt(
    const uint8_t *message,
    size_t message_len,
    const uint8_t *additional_data,
    size_t additional_data_len,
    const uint8_t *nonce,
    const uint8_t *key,
    uint8_t *ciphertext
) {
    unsigned long long ciphertext_len = 0;
    int result;
    if (!crypto_aead_aes256gcm_is_available()) {
        return -2;
    }
    result = crypto_aead_aes256gcm_encrypt(
        ciphertext,
        &ciphertext_len,
        message,
        (unsigned long long)message_len,
        additional_data,
        (unsigned long long)additional_data_len,
        NULL,
        nonce,
        key
    );
    if (result != 0) {
        return result;
    }
    return ciphertext_len == message_len + crypto_aead_aes256gcm_ABYTES ? 0 : -1;
}

int gts_aes256gcm_decrypt(
    const uint8_t *ciphertext,
    size_t ciphertext_len,
    const uint8_t *additional_data,
    size_t additional_data_len,
    const uint8_t *nonce,
    const uint8_t *key,
    uint8_t *message
) {
    unsigned long long message_len = 0;
    int result;
    if (!crypto_aead_aes256gcm_is_available()) {
        return -2;
    }
    result = crypto_aead_aes256gcm_decrypt(
        message,
        &message_len,
        NULL,
        ciphertext,
        (unsigned long long)ciphertext_len,
        additional_data,
        (unsigned long long)additional_data_len,
        nonce,
        key
    );
    if (result != 0) {
        return result;
    }
    return message_len + crypto_aead_aes256gcm_ABYTES == ciphertext_len ? 0 : -1;
}
