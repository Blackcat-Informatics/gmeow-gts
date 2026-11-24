/*
 * SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
 * SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
 */

#include <stddef.h>
#include <stdint.h>

#include "blake3.h"

void gts_blake3_hash(const void *input, size_t input_len, uint8_t *out) {
    blake3_hasher hasher;

    blake3_hasher_init(&hasher);
    blake3_hasher_update(&hasher, input, input_len);
    blake3_hasher_finalize(&hasher, out, BLAKE3_OUT_LEN);
}

void gts_blake3_derive(const void *context, size_t context_len,
                     const void *prefix, size_t prefix_len,
                     const uint8_t *input, size_t start, size_t length,
                     uint8_t *out) {
    blake3_hasher hasher;

    blake3_hasher_init_derive_key_raw(&hasher, context, context_len);
    if (prefix_len != 0) {
        blake3_hasher_update(&hasher, prefix, prefix_len);
    }
    if (length != 0) {
        blake3_hasher_update(&hasher, input + start, length);
    }
    blake3_hasher_finalize(&hasher, out, BLAKE3_OUT_LEN);
}

size_t gts_blake3_out_len(void) {
    return BLAKE3_OUT_LEN;
}
