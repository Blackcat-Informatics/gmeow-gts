/*
 * SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

#include <stddef.h>
#include <stdint.h>

#include <sodium.h>

int gts_sodium_init(void) {
    return sodium_init();
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
