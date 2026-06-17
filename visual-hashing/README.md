<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# visual-hashing

[![crates.io](https://img.shields.io/crates/v/visual-hashing.svg)](https://crates.io/crates/visual-hashing)
[![docs.rs](https://docs.rs/visual-hashing/badge.svg)](https://docs.rs/visual-hashing)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**Human-friendly visual fingerprints** for keys, checksums, and any byte string you
need a person to compare out-of-band — the *"is this the right key?"* glance.

Comparing 64 hex characters by eye is error-prone and nobody actually does it.
`visual-hashing` renders the same bytes two ways a human can *actually* check:

- **emojihash** — a BLAKE3-XOF digest sliced into 6-bit symbols indexing a fixed,
  nameable 64-emoji alphabet. Short, glanceable, and *speakable* (`pig duck monkey …`),
  so two people can verify a fingerprint over the phone.
- **randomart** — the OpenSSH-style "Drunken Bishop" ASCII-art grid you already know
  from `ssh-keygen -lv`.

Both are pure, deterministic, byte-for-byte stable functions of the input.

## What it looks like

Given a 32-byte Ed25519 public key, `visual-hashing` renders:

```text
emojihash   🐷 🦆 🐵 🦋 🍎 🍐 🦊 🐸 🐟 🍒 🍎
  (spoken)  pig duck monkey butterfly apple pear fox frog fish cherries apple

randomart   +--[ED25519 256   ]+
            |      =  .o .    |
            |     = +   o . ..|
            |      * . .   .o+|
            |     . + o     +*|
            |    . o S   . +.=|
            |     o     . . BE|
            |          .   = O|
            |           . o X=|
            |           .*+@O=|
            +----------------+
```

Flip a single bit of the key and both renderings change completely — that is the point.

## Install

```bash
cargo add visual-hashing
```

## Usage

```rust
use visual_hashing::{emojihash, emojihash_labels, randomart};

let key: &[u8] = b"\x00\x01\x02\x03"; // any bytes: a public key, a file digest, …

// A short, speakable emoji fingerprint (11 digits is the conventional length).
println!("{}", emojihash(key, 11));        // 🍑 🦂 🥥 🦉 🐌 🦀 🌽 🐳 🐻 🍒 🐶
println!("{}", emojihash_labels(key, 11)); // peach scorpion coconut …  (the same digits, named)

// A bigger ASCII-art fingerprint; the label only annotates the header.
println!("{}", randomart(key, "ED25519 256"));
```

A handful of emoji is enough for a human to spot a mismatch, while staying short
enough to print in a CLI banner, a log line, or a chat message:

```rust
// Tune the length to the surface: a 6-emoji chip for a tight UI…
assert_eq!(visual_hashing::emojihash(b"hello", 6).split(' ').count(), 6);
// …or the full 11 for a key-verification prompt.
```

## Why

A fingerprint only helps if a human can read it back, so the emoji alphabet favours
common animals and then familiar foods over abstract, confusable symbols (no
`🜲`/`⊕`/`◈`). Because both renderings are **byte-for-byte deterministic** and pinned
by a frozen conformance corpus, independent implementations — in any language — agree
exactly. That matters when the *same* key fingerprint must look identical on a CLI, in
a server log, and in a mobile app, so a user can compare across all three.

## API

| Function | Returns |
| --- | --- |
| `emojihash(data, length)` | `length` space-joined emoji digits |
| `emojihash_labels(data, length)` | the same digits as space-joined names |
| `emoji_indices(data, length)` | the raw `0..64` symbol indices |
| `randomart(data, label)` | a 17×9 drunken-bishop grid; `label` annotates the header (`""` for none) |

`EMOJI`, `LABELS`, and `ALPHABET_SIZE` expose the 64-entry alphabet directly, in case
you want to render it yourself.

## How it works

**emojihash.** BLAKE3 in extendable-output (XOF) mode produces exactly
`ceil(length × 6 / 8)` bytes; those bits are consumed six at a time, most-significant
first, and each 6-bit symbol (`0..64`) selects one entry from the alphabet. Using a XOF
rather than a truncated fixed hash means any `length` is well-defined and a prefix of a
longer fingerprint is *not* a shorter one (the whole digest shifts).

**randomart.** A bishop starts in the centre of a 17×9 grid and makes four diagonal
moves per input byte (two bits each), incrementing a visit counter on every square it
lands on. Counts render through the OpenSSH character ramp `" .o+=*BOX@%&#/^"` (a leading
space for unvisited cells); the start and end squares are marked `S` and `E`.

## Stability

The 64-emoji alphabet and the randomart character ramp are a **wire contract**: once
`1.0` ships they will not change, because a fingerprint that renders differently across
versions is worse than useless. Pre-`1.0` the alphabet is considered stable but reserves
the right to fix outright mistakes.

The only dependency is [`blake3`](https://crates.io/crates/blake3). No `unsafe`, no I/O,
`wasm32`-friendly.

## Provenance

`visual-hashing` was factored out of [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts),
where the same fingerprints identify embedded transport keys. The crate's conformance
corpus is generated by a Python reference implementation and shared across languages, so
the renderings are portable beyond Rust.

## License

Licensed under either of [MIT](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSE-MIT)
or [Apache-2.0](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSE-APACHE)
at your option. © Blackcat Informatics® Inc.
