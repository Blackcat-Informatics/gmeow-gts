// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Human-friendly **visual fingerprints** for keys, checksums, and any byte
//! string you need a human to compare out-of-band.
//!
//! Two complementary renderings, both pure functions of the input bytes:
//!
//! - [`emojihash`] / [`emojihash_labels`] — a BLAKE3-XOF digest sliced into
//!   6-bit symbols indexing a fixed, nameable 64-emoji alphabet. Short, glanceable,
//!   and speakable ("monkey pig apple …").
//! - [`randomart`] — the OpenSSH-style "Drunken Bishop" ASCII-art grid you see
//!   in `ssh-keygen -lv` output.
//!
//! ```
//! use visual_hashing::{emojihash, emojihash_labels, randomart};
//!
//! let key = b"\x00\x01\x02\x03";
//! println!("{}", emojihash(key, 11));        // 🐵 🐶 … (11 emoji)
//! println!("{}", emojihash_labels(key, 11)); // monkey dog …
//! println!("{}", randomart(key, "ED25519")); // +--[ED25519 …
//! ```
//!
//! Both renderings are byte-for-byte deterministic and gated by a frozen
//! conformance corpus, so independent implementations agree exactly.

mod emojihash;
mod randomart;

pub use emojihash::{emoji_indices, emojihash, emojihash_labels, ALPHABET_SIZE, EMOJI, LABELS};
pub use randomart::randomart;
