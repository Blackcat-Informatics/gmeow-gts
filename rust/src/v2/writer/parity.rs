// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Encode P5 geometry 1 spans over the stored byte stream.

use std::sync::OnceLock;

use ciborium::value::Value;

use super::Digest;
use crate::{
    v2::frame::{encode_plain_frame, FrameError, PARITY},
    wire,
};

pub(super) const SPAN_BYTES: usize = 1 << 22;
const DATA_COLUMNS: usize = 64;
const PARITY_ROWS: usize = 4;
const MIN_SHARD: usize = 64;
const MAX_SHARD: usize = 4096;
const FIELD_ORDER: usize = 255;
const ZERO_SHARD: [u8; MAX_SHARD] = [0; MAX_SHARD];

static CAUCHY_PRODUCTS: OnceLock<[[u8; 256]; DATA_COLUMNS]> = OnceLock::new();

fn cauchy_products() -> &'static [[u8; 256]; DATA_COLUMNS] {
    CAUCHY_PRODUCTS.get_or_init(|| {
        let mut powers = [0; FIELD_ORDER];
        let mut logarithms = [0; 256];
        let mut value = 1_u8;
        for (exponent, power) in powers.iter_mut().enumerate() {
            *power = value;
            logarithms[usize::from(value)] = exponent;
            let overflow = value & 0x80 != 0;
            value <<= 1;
            if overflow {
                value ^= 0x1d;
            }
        }
        // The denominator (255 - row) XOR column is 255 XOR (row XOR column).
        // Each distinct denominator needs just one byte multiplication table.
        std::array::from_fn(|i| {
            let inverse = FIELD_ORDER - logarithms[255 ^ i];
            std::array::from_fn(|octet| {
                if octet == 0 {
                    0
                } else {
                    powers[(inverse + logarithms[octet]) % FIELD_ORDER]
                }
            })
        })
    })
}

fn shard_size(length: usize) -> usize {
    length
        .div_ceil(DATA_COLUMNS)
        .next_power_of_two()
        .clamp(MIN_SHARD, MAX_SHARD)
}

fn shard_digest(bytes: &[u8], size: usize) -> Value {
    let mut hash = blake3::Hasher::new_derive_key("gts2 shard");
    hash.update(bytes);
    hash.update(&ZERO_SHARD[..size - bytes.len()]);
    Value::Bytes(hash.finalize().as_bytes()[..16].to_vec())
}

fn parity_shards(bytes: &[u8], size: usize) -> Vec<u8> {
    let depth = bytes.len().div_ceil(size).div_ceil(DATA_COLUMNS);
    let stripe_bytes = PARITY_ROWS * size;
    let mut parity = vec![0; depth * stripe_bytes];
    let products = cauchy_products();

    for (index, source) in bytes.chunks(size).enumerate() {
        let start = (index % depth) * stripe_bytes;
        let column = index / depth;
        let stripe = &mut parity[start..start + stripe_bytes];
        for (row, target) in stripe.chunks_exact_mut(size).enumerate() {
            let product = &products[row ^ column];
            for (&octet, parity) in source.iter().zip(target) {
                *parity ^= product[usize::from(octet)];
            }
        }
    }
    parity
}

pub(super) fn encode_span(
    bytes: &[u8],
    first: usize,
    opener: &Digest,
    anchor: &Digest,
) -> Result<Vec<u8>, FrameError> {
    let size = shard_size(bytes.len());
    let parity = parity_shards(bytes, size);
    let digests = |bytes: &[u8]| {
        Value::Array(
            bytes
                .chunks(size)
                .map(|shard| shard_digest(shard, size))
                .collect(),
        )
    };
    let data_digests = digests(bytes);
    let parity_digests = digests(&parity);
    let table = wire::encode(&Value::Array(vec![
        Value::Bytes(opener.to_vec()),
        data_digests.clone(),
        parity_digests.clone(),
    ]));
    let digest = blake3::derive_key("gts2 parity table", &table);
    let payload = wire::encode(&Value::Array(vec![
        data_digests,
        parity_digests,
        Value::Bytes(parity),
    ]));
    let content = wire::encode(&Value::Array(vec![
        Value::from(1),
        Value::from(first as u64),
        Value::from((first + bytes.len()) as u64),
        Value::Bytes(anchor.to_vec()),
        Value::Bytes(digest.to_vec()),
    ]));
    encode_plain_frame(anchor, PARITY, &content, &payload)
}
