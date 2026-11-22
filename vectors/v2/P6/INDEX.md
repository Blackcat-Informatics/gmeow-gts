# Vectors of P6

| vector | outcome | notes |
|---|---|---|
| `sig-record-valid` | built and matches | a blob frame with one signature of each algorithm |
| `sig-grammar-not-array` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-element-not-array` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-two-fields` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-four-fields` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-algorithm-text` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-algorithm-zero` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-algorithm-unregistered` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-key-id-text` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-key-id-short` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-value-text` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-grammar-long-head` | built and matches | the `signatures` element is the bytes of the entry; the frame is opaque `malformed`, no signature result |
| `sig-transparent-signature` | built and matches | a parity frame with one signature |
| `sig-list-order` | built and matches | three signatures of one frame in two arrival orders: one list, in ascending bytewise order of the records (P6-52) |
| `sig-list-unordered` | built and matches | the three signatures with two exchanged: malformed_value, unordered_or_repeated, once; opaque; no signature result |
| `sig-list-key-id-decides` | built and matches | key_ids that differ in the last byte: ascending is accepted, exchanged is unordered_or_repeated |
| `sig-list-grammar-first` | built and matches | two records out of order whose second has a key_id of 31 bytes: wrong_length |
| `sig-list-independence` | built and matches | a valid signature beside an invalid one: each result as if alone |
| `sig-duplicate-records` | built and matches | two equal records on one frame: unordered_or_repeated once, no signature result |
| `sig-same-key-two-values` | built and matches | two records with one algorithm and key_id and different values, in ascending order: two results (P6-5 is a writer rule) |
| `subject-ed25519` | built and matches | the typed subject [1, K, H0, F]: the 104 bytes and the 32-byte message |
| `subject-composite` | built and matches | the same record with the code 2 |
| `subject-healed-frame` | built and matches | a frame whose identifier field is damaged and healed, signed before the damage |
| `subject-other-frame` | built and matches | a valid signature moved to another frame |
| `subject-relabel-algorithm` | built and matches | a valid ed record with algorithm 2, the held key unchanged |
| `subject-relabel-key` | built and matches | a valid record with key_id of another held key |
| `key-id-ed25519` | built and matches | E([1, PK]) is 36 bytes |
| `key-id-composite` | built and matches | E([2, PK2]) is 1989 bytes |
| `key-held-order` | built and matches | held_keys and trusted_keys reordered and repeated: the same results |
| `key-held-not-trusted` | built and matches | valid, authenticated false |
| `key-trusted-not-held` | built and matches | unverified, key_not_held |
| `key-mismatch-bytes` | built and matches | a held entry of the signature's key_id with another public_key |
| `key-mismatch-algorithm` | built and matches | a held entry of the signature's key_id with the other algorithm |
| `key-mismatch-two-entries` | built and matches | two held entries with one key_id, one inconsistent, one consistent: valid |
| `key-length-short` | built and matches | a consistent held ed key of 31 bytes: value_length |
| `ed25519-honest` | built and matches | a signature over a fixed subject |
| `ed25519-honest-reproduce` | built and matches | the value of ed25519-honest equals the value of the `cryptography` module (OpenSSL) for the same seed and message |
| `ed25519-length-63` | built and matches |  |
| `ed25519-length-65` | built and matches |  |
| `ed25519-scalar-equals-l` | built and matches |  |
| `ed25519-scalar-above-l` | built and matches | S equal to S0 + L |
| `ed25519-scalar-max` | built and matches |  |
| `ed25519-scalar-below-l` | built and matches | S = L - 1 with a failing equation |
| `ed25519-key-small-order-1` | built and matches |  |
| `ed25519-nonce-small-order-1` | built and matches |  |
| `ed25519-key-small-order-2` | built and matches |  |
| `ed25519-nonce-small-order-2` | built and matches |  |
| `ed25519-key-small-order-3` | built and matches |  |
| `ed25519-nonce-small-order-3` | built and matches |  |
| `ed25519-key-small-order-4` | built and matches |  |
| `ed25519-nonce-small-order-4` | built and matches |  |
| `ed25519-key-small-order-5` | built and matches |  |
| `ed25519-nonce-small-order-5` | built and matches |  |
| `ed25519-key-small-order-6` | built and matches |  |
| `ed25519-nonce-small-order-6` | built and matches |  |
| `ed25519-key-small-order-7` | built and matches |  |
| `ed25519-nonce-small-order-7` | built and matches |  |
| `ed25519-key-small-order-8` | built and matches |  |
| `ed25519-nonce-small-order-8` | built and matches |  |
| `ed25519-key-alias` | built and matches | the key y = p (ed ff .. 7f) and y = p + 1: equation_failed, not small_order_key |
| `ed25519-key-non-canonical-y` | built and matches | a key with y at least p |
| `ed25519-key-non-canonical-sign` | built and matches | 01 with the sign bit set |
| `ed25519-key-off-curve` | built and matches | a key with no square root |
| `ed25519-nonce-non-canonical` | built and matches | R with y at least p |
| `ed25519-nonce-off-curve` | built and matches | R that does not decode |
| `ed25519-order-scalar-key` | built and matches | S at least L and a small-order key |
| `ed25519-order-key-nonce` | built and matches | a small-order key and a small-order R |
| `ed25519-equation-message` | built and matches | a valid signature over a subject with one bit changed |
| `ed25519-mixed-order-valid` | built and matches | a key A + T with T of order 8, a signature satisfying [S]B = R + [k mod L]A |
| `ed25519-mixed-order-cofactored-only` | built and matches | a signature that satisfies only the cofactored equation |
| `ed25519-subgroup-forms-agree` | built and matches | honest signatures: valid under the strict equation, the cofactored equation and OpenSSL's verification |
| `composite-message-representative` | built and matches | the 127 bytes of M' for the message 00..1f |
| `composite-honest` | built and matches | a composite signature over a fixed subject |
| `composite-length-3372` | built and matches |  |
| `composite-length-3374` | built and matches |  |
| `composite-key-1983` | built and matches | a held key one byte short (consistent with its identifier) |
| `composite-key-1985` | built and matches | a held key one byte long |
| `composite-classical-nonce` | built and matches | the classical half altered in R |
| `composite-classical-scalar` | built and matches | S at least L |
| `composite-classical-small-order` | built and matches | R a small-order encoding |
| `composite-post-quantum` | built and matches | the ML-DSA half altered |
| `composite-both` | built and matches | both halves altered |
| `composite-stripped-half` | built and matches | the ML-DSA half all zero |
| `composite-halves-two-subjects` | built and matches | the ML-DSA half of one subject, the Ed25519 half of another |
| `composite-context-nonempty` | built and matches | M' with len(ctx) 1 and ctx 78 |
| `composite-prehash-wrong` | built and matches | M' with SHA-256 as the pre-hash |
| `composite-label-wrong` | built and matches | M' with another label |
| `composite-mldsa-context` | built and matches | an ML-DSA half made with an empty context |
| `composite-hint-count-above-omega` | built and matches | the first count byte of the hint is 56, above omega 55: refused at byte 3303 |
| `composite-hint-count-decreasing` | built and matches | counts 2 then 1: the second count is below the index reached, refused at byte 3304 |
| `composite-hint-indices-unordered` | built and matches | two indices 5 then 3 of one polynomial: refused at byte 3249 |
| `composite-hint-padding-nonzero` | built and matches | all six counts 0 and index byte 3248 set: a non-zero byte that no count reaches, refused at byte 3248 |
| `proc-case-5` | built and matches | case 5 of 11.4 |
| `proc-case-6` | built and matches | case 6 of 11.4 |
| `proc-case-7` | built and matches | case 7 of 11.4 |
| `proc-case-8` | built and matches | case 8 of 11.4 |
| `proc-case-9` | built and matches | case 9 of 11.4 |
| `proc-case-10` | built and matches | case 10 of 11.4 |
| `proc-case-11` | built and matches | case 11 of 11.4 |
| `proc-case-12` | built and matches | case 12 of 11.4 |
| `proc-case-13` | built and matches | case 13 of 11.4 |
| `proc-case-14` | built and matches | case 14 of 11.4 |
| `proc-unknown-algorithm` | built and matches | case 1 of 11.4: h 2, r 1, algorithm 3: unverified, one signature_unverified, unsupported_registry_version once, no unknown_enumeration_value |
| `proc-key-not-held` | built and matches | case 2 of 11.4 |
| `proc-key-mismatch` | built and matches | case 3 of 11.4 |
| `proc-value-length` | built and matches | case 4 of 11.4 |
| `reg-unknown-algorithm-version` | built and matches | h 2, r 1, algorithm 3: case 1 of 11.4 |
| `proc-unknown-algorithm-long` | built and matches | an unknown algorithm with a wrong-length value and a held key of that key_id: step 1 first |
| `proc-lengths-key-first` | built and matches | a short key and a short value: value_length |
| `proc-mismatch-before-length` | built and matches | a key that is not consistent and a short value: key_mismatch |
| `proc-length-before-policy` | built and matches | a short value over a small-order key: value_length |
| `proc-no-hash` | built and matches | hash_algorithm 9, registry_version 2: no signature result, authenticity unauthenticated |
| `proc-opaque-frame` | built and matches | a signed frame of an unknown non-critical type (h 2) and a signed quads frame with a malformed content |
| `proc-trust-not-status` | built and matches | one signature under four trusted_keys sets: the same status and reason |
| `cov-simple` | built and matches | 11.4 unchanged |
| `cov-broken-signed-frame` | built and matches | a valid signature on a frame whose link is broken: coverage [o, o), authenticated false |
| `cov-break-before` | built and matches | a broken frame before the signed frame: coverage starts after the break |
| `cov-break-after` | built and matches | a broken frame after the signed frame: unaffected |
| `cov-healed` | built and matches | a healed frame inside the run: the run includes it |
| `cov-transparent-between` | built and matches | a parity frame inside the run: the run is unchanged |
| `cov-not-valid` | built and matches | an invalid and an unverified signature: coverage [o, o) each |
| `auth-commit-valid` | built and matches | a signed commit that is valid |
| `auth-commit-bad-root` | built and matches | a signed commit with a wrong root |
| `auth-commit-bad-count` | built and matches | a signed commit with a wrong count |
| `auth-commit-malformed` | built and matches | a signed commit whose content has three fields |
| `auth-untrusted` | built and matches | a valid signature by a key not in trusted_keys |
| `auth-non-commit` | built and matches | a signature on a quads frame by a trusted key |
| `auth-tail` | built and matches | a tail after the last signed frame |
| `auth-all` | built and matches | a signed last frame |
| `auth-none` | built and matches | no signature |
| `auth-empty-segment` | built and matches | a segment with no counted frame |
| `auth-empty-segment-index` | built and matches | a segment whose only frame is a signed index: no counted frame |
| `auth-canonical-end` | built and matches | the canonical file of 7.5 whose commit carries an authenticating signature: authenticated, authenticated_frames 5 |
| `auth-two-commits-two-signers` | built and matches | 11.5: the table, the tail and the signature of A on the tail |
| `auth-broken-middle` | built and matches | 11.5, the second case: F3 altered after the writing |
| `auth-transparent-never` | built and matches | a parity frame inside a signed prefix: in transparent_frames and in no count |
| `auth-payload-damaged` | built and matches | an authenticated frame whose payload is damaged: authenticated and integrity damaged |
| `auth-proof-only` | built and matches | a root that holds (a valid commit over its frames) and no signature: authenticated false |
| `cosign-add-signature` | built and matches | a frame rewritten with a second signature: identifiers, roots and the first result unchanged |
| `cosign-two-algorithms` | built and matches | one commit signed by ed and by cp, different keys |
| `sign-composite-reproduce` | built and matches | the 3373 bytes for a composite key and a subject: ML-DSA half by gts2ref/mldsa.py (FIPS 204 deterministic) reproduced by `openssl pkeyutl -pkeyopt deterministic:1`, Ed25519 half equal to OpenSSL's |
| `sign-ed25519-reproduce` | built and matches | the signature bytes for a key and a subject, equal to those of OpenSSL's Ed25519 |
| `ed25519-key-non-canonical` | built and matches | a key with y at least p, and 01 with the sign bit set: equation_failed |
| `cosign-after-torn-nothing` | built and matches | the file H T Q C cut inside the last value of C: no byte is written (P6-54) |
| `cosign-append-commit` | built and matches | a second commit signed by a second key: the second signature authenticated with coverage over both commits (P6) |
| `cosign-capacity-spent` | built and matches | an undamaged file whose total decoded length exceeds the decoded_bytes_per_read of the co-signer: the commit is appended (P6-54) |
| `cosign-item-limit` | built and matches | an undamaged file of more items than the items_per_read of the co-signer: the commit is appended (P6-54) |
| `subject-other-segment` | built and matches | a valid signature copied to the same frame content in a segment of another opener: invalid, equation_failed (P6) |
| `parity-transparent-not-bool-signed` | built and matches | a signed `parity` frame whose `transparent` is not a `bool` is ordinary, has an effective identifier and has a signature result: `signature_unverified` `key_not_held` beside `parity_damaged` (P1 3.4, P6-3) |
| `sig-detached-frame` | derived from the text | a valid signature on the frame that takes the anchor of a parity frame at which reading resumes, the anchor not being the identifier of the frame before the damaged region: the frame is detached, the coverage is [1, 2), one of two frames is authenticated and anchored_frames is 1 |
| `sig-resumed-frame-anchor-true` | built and matches | a valid signature on the frame after a parity frame at which reading resumes, the anchor being the identifier of the frame before the damaged region: no frame is detached, the coverage is [0, 2), both frames are authenticated and anchored_frames is 2 |
| `sig-commit-after-detached-frame` | derived from the text | a signed valid commit after a detached frame, its root over the frames present: the three frames are authenticated through the root, the coverage of the signature is [1, 3) and anchored_frames is 1 |
| `sig-two-after-detached-frame` | derived from the text | a signature on a detached frame and one on the frame after it: the coverages [1, 2) and [1, 3) begin at the detached frame, the frame before it is not authenticated and anchored_frames is 1 |
| `signature-added-other-key-trusted` | built and matches | a commit with one signature and the same commit with a second signature, read with the key of the added signature alone trusted: both signatures are valid, only the added key authenticates, and the file without it is unauthenticated |

Totals: built and matches: 151; derived from the text: 3 (154 entries).
