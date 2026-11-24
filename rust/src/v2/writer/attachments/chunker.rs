// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::sync::OnceLock;

const WINDOW: usize = 64;
const MIN: usize = 1 << 14;
const AVERAGE: usize = 1 << 16;
const MAX: usize = 1 << 18;
const STRICT_SHIFT: u32 = 46;
const LOOSE_SHIFT: u32 = 50;

static GEAR: OnceLock<[u64; 256]> = OnceLock::new();

fn gear() -> &'static [u64; 256] {
    GEAR.get_or_init(|| {
        let mut stream = blake3::Hasher::new_derive_key("gts2 gear table").finalize_xof();
        std::array::from_fn(|_| {
            let mut word = [0; 8];
            stream.fill(&mut word);
            u64::from_be_bytes(word)
        })
    })
}

fn cut(bytes: &[u8]) -> usize {
    if bytes.len() <= MIN {
        return bytes.len();
    }
    let end = bytes.len().min(MAX);
    let table = gear();
    let mut fingerprint = 0_u64;
    // Begin with the bytes that affect the first test. The final byte ends
    // the chunk due to the input or maximum length and needs no mask test.
    for (index, &octet) in bytes[MIN - WINDOW..end - 1].iter().enumerate() {
        fingerprint = (fingerprint << 1).wrapping_add(table[usize::from(octet)]);
        let length = MIN - WINDOW + index + 1;
        let shift = if length < AVERAGE {
            STRICT_SHIFT
        } else {
            LOOSE_SHIFT
        };
        if length >= MIN && fingerprint >> shift == 0 {
            return length;
        }
    }
    end
}

/// Yield the P4 chunks of a byte string, restarting at every cut.
pub(super) fn chunks(mut remaining: &[u8]) -> impl Iterator<Item = &[u8]> {
    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }
        let (chunk, rest) = remaining.split_at(cut(remaining));
        remaining = rest;
        Some(chunk)
    })
}
