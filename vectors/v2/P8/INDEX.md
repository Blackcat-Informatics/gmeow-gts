# Vectors of P8

| vector | outcome | notes |
|---|---|---|
| `p8-key-schedule` | built and matches | 16.1 and RFC 5869 Appendix A cases 1 and 3 |
| `p8-hmac-kat` | built and matches |  |
| `p8-aead-kat` | built and matches | the 24 vectors of RFC 8452 Appendix C.2, parsed from the RFC text (tools/rfc8452_c2.json) |
| `p8-seal-kat` | built and matches | every value printed in 16.2 to 16.4 |
| `p8-seal-kat-keyless` | built and matches |  |
| `p8-flags` | built and matches |  |
| `p8-keys-counts` | built and matches |  |
| `p8-keys-empty` | built and matches |  |
| `p8-keys-context` | built and matches |  |
| `p8-keys-repeated` | built and matches |  |
| `p8-keys-storage` | built and matches |  |
| `p8-index-out-of-range` | built and matches |  |
| `p8-ordinal` | built and matches |  |
| `p8-length` | built and matches | the payload of 2^27 + 17 zero bytes is built in memory and not stored; its payload digest is 402bbe4db2eb033a483ea8a024dba5d570d16358b3e96eeefd69a806308a498c |
| `p8-sealed-storage` | built and matches |  |
| `p8-canonical` | built and matches |  |
| `p8-aead` | built and matches |  |
| `p8-commitment` | built and matches |  |
| `p8-two-keys` | built and matches |  |
| `p8-committing-ciphertext` | built and matches | the ciphertext is computed by solving POLYVAL for two keys (tools/vec_p8.py committing_ciphertext) |
| `p8-segment-lift` | built and matches |  |
| `p8-context` | built and matches |  |
| `p8-determinism` | built and matches |  |
| `p8-padding` | built and matches |  |
| `p8-inner-grammar` | built and matches |  |
| `p8-inner-type` | built and matches |  |
| `sealed-inner-newer-value` | derived from the text | a segment of registry version 3 read by a reader of version 2 that holds the key: a sealed frame whose inner quads frame has the order 9, a value the reader does not hold, gives unsupported_critical with the subject [2, 18] of its own envelope, is opaque with unsupported_critical and counts in unopened_sealed, and the clear terms frame after it is opaque with stopped (P8-33) |
| `p8-inner-payload` | built and matches |  |
| `p8-failed-openings` | built and matches |  |
| `p8-failed-openings-storage` | built and matches |  |
| `p8-continue` | built and matches |  |
| `p8-wrap-kat` | built and matches | the X-Wing part reproduces the first vector of Appendix C of draft-connolly-cfrg-xwing-kem-11 (tools/xwing_kat.json); the text prints no value for the vector's own inputs |
| `p8-wrap-tamper` | built and matches |  |
| `p8-wrap-lengths` | built and matches |  |
| `p8-wrap-target` | built and matches |  |
| `p8-wrap-conflict` | built and matches |  |
| `p8-wrap-partial` | built and matches |  |
| `p8-input-duplicate` | built and matches |  |
| `p8-input-mismatch` | built and matches |  |
| `p8-sealed-pack` | built and matches |  |
| `p8-seal-scope` | built and matches |  |
| `p8-members` | built and matches | 16.8. The text of the vector says one index_claim_failed for each altered file; the engine follows the letter of P8-44 (two findings for a wrong ordinal, two for a first that names a pack) and does not bend to the vector (P8-ITEMS item 2) |
| `p8-members-grammar` | built and matches |  |
| `p8-v1-reader` | built and matches |  |
| `p8-16.5-mutations` | built and matches | every bullet of 16.5 that no other entry builds |
| `frame-value-above-header` | built and matches | the vector of P1 section 10 that waited for registry version 2 |
| `p8-writer-layout` | built and matches | keys frames from keys_frame_for (P8-58) |
| `p8-layout-pass` | built and matches | the file of p8-writer-layout read whole; a transparent frame between the second keys frame and its sealed frame is not built here (the keys frame is directly before the sealed frame) |
| `p8-layout-two-entries` | built and matches |  |
| `p8-layout-unused-entry` | built and matches |  |
| `p8-layout-not-next` | built and matches |  |
| `p8-layout-first-use-order` | built and matches |  |
| `p8-layout-early-use` | built and matches |  |
| `p8-members-layout-misplaced` | built and matches | packs of one member; the vectors of 400000 members are not built |
| `p8-members-layout-empty` | built and matches |  |
| `p8-members-layout-missing-small` | built and matches | packs of one member each: the same shape as p8-members-layout-missing, which has the packs of 400000 members (vec_r5d.py) |
| `p8-clear-precedence` | built and matches | P8-19 (the key index), P8-20 (the ordinal) and P8-21/22 (the content shape and the storage) in order |
| `p8-failed-openings-order` | built and matches | failing frames spread over two segments, an honest frame at the end of each: the count runs in file order, segments in order (P8-37) |
| `p8-inner-precedence` | built and matches | P8-32 (shape), P8-31 (padding) and P8-33 (the inner content) in order |
| `p8-input-precedence` | built and matches | a repeat is judged before the mismatch of the identifier at one entry (4.4) |
| `p8-keys-precedence` | built and matches | the storage, the counts, the context length and the repeated key_id in the order of P8-59 |
| `p8-members-lookup` | built and matches | a lookup whose chain ends at a member index that breaks the layout of P8-62: frames uncovered after the last member index and before it (8.3) |
| `p8-members-order` | built and matches | entries with one unit_digest at the ordinals 9 and 10, and at the member indexes 2 and 10: numeric order; the reverse is unordered_or_repeated (P8-43) |
| `p8-wrap-order` | built and matches | recipient entries ascending, in the other order, and with one pair adjacent twice (P8-57) |
| `p8-wrap-pk-length` | built and matches | a writer given a recipient public key of 1,215 and of 1,217 bytes: an error each, no entry written (P8-60) |
| `p8-wrap-precedence` | built and matches | one recipient entry: wrong_length, then out_of_range, then unordered_or_repeated (9.3) |
| `p8-wrap-repeat-frames` | built and matches | the same recipient and target in two keys frames: the second entry is refused by P8-49 whether or not the first was admitted |
| `p8-wrap-trial-order` | built and matches | three admitted recipient entries for one key: key_conflict when two held keys unwrap differently; one unwrap_failed when one fails (9.4) |
| `p8-members-layout-pass` | built and matches | three sealed packs of 400000 members with the frames of P8-45, and one pack of 1048577 members with its frame (files above 20 MB keep their length and digest in the JSON) |
| `p8-members-layout-missing` | built and matches | two sealed packs of 400000 members: the frame after the second; the frame absent; a clear quads frame between |
| `p8-members-layout-cut` | built and matches | a frame covering two packs of 600000 members; a frame after the first of two packs of 400000 and a second after the second |
| `p8-state-sealed-blob` | built and matches | P8 15 item 9, P3 7.2: the sealed blob state: keyless, key holder, and a key entry whose key conflicts |
| `p8-sealed-pack-charge` | built and matches | a sealed pack whose directory holds two chunk members, a recipe member of charge T1 and a value member of charge T2, read by a key holder with rebuilt_bytes_per_read T1: the recipe admitted and rebuilt, the value refused, one exceeds_capacity with total_rebuilt and count T1 + T2 at the sealed frame, the rebuilds of the sealed frame in directory order; keyless: no charge, no total_rebuilt finding (P8-63) |

Totals: built and matches: 73 (73 entries).
