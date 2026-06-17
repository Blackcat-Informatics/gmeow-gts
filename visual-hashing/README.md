<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# visual-hashing

Human-friendly **visual fingerprints** for keys, checksums, and any byte string
you need a person to compare out-of-band — the "is this the right key?" glance.

Two complementary renderings, both pure, deterministic functions of the input:

- **emojihash** — a BLAKE3-XOF digest sliced into 6-bit symbols indexing a fixed,
  nameable 64-emoji alphabet. Short, glanceable, and *speakable*
  (`monkey pig apple …`), so two people can verify a fingerprint over the phone.
- **randomart** — the OpenSSH-style "Drunken Bishop" ASCII-art grid you already
  know from `ssh-keygen -lv`.

```rust
use visual_hashing::{emojihash, emojihash_labels, randomart};

let key = b"\x00\x01\x02\x03";
println!("{}", emojihash(key, 11));        // 🐵 🐶 … (11 emoji digits)
println!("{}", emojihash_labels(key, 11)); // monkey dog … (the same digits, named)
println!("{}", randomart(key, "ED25519")); // +--[ED25519       ]+ …
```

## Why

A fingerprint only helps if a human can read it back, so the emoji alphabet
favours common animals and then familiar foods over abstract, confusable
symbols. Both renderings are **byte-for-byte deterministic** and pinned by a
frozen conformance corpus, so independent implementations (in any language)
agree exactly — useful when the same key fingerprint must render identically on
a CLI, a server log, and a mobile app.

## API

| Function | Returns |
| --- | --- |
| `emojihash(data, length)` | `length` space-joined emoji (default usage: 11) |
| `emojihash_labels(data, length)` | the same digits as space-joined names |
| `emoji_indices(data, length)` | the raw `0..64` symbol indices |
| `randomart(data, label)` | a 17×9 drunken-bishop grid; `label` annotates the header (`""` for none) |

`EMOJI`, `LABELS`, and `ALPHABET_SIZE` expose the 64-entry alphabet directly.

## Stability

The 64-emoji alphabet and the randomart character ramp are a **wire contract**:
once `1.0` ships they will not change, because a fingerprint that renders
differently across versions is worse than useless. Pre-`1.0`, the alphabet is
considered stable but reserves the right to fix outright mistakes.

The only dependency is [`blake3`](https://crates.io/crates/blake3). No `unsafe`,
no I/O, `wasm32`-friendly.

## License

Licensed under either of [MIT](../LICENSE-MIT) or
[Apache-2.0](../LICENSE-APACHE) at your option.
