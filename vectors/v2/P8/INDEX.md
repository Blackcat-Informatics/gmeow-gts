# Vectors of P8

| vector | outcome | notes |
|---|---|---|
| `p8-key-schedule` | built and matches | 16.1 and RFC 5869 Appendix A cases 1 and 3 |
| `p8-hmac-kat` | built and matches |  |
| `p8-aead-kat` | built and matches | the 24 vectors of RFC 8452 Appendix C.2 are the known answers of the AEAD of 16.2; the entry stores no input and no value, the vectors are those printed in the RFC |
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
| `p8-length` | built and matches | a sealed frame whose payload is 16 bytes: malformed with out_of_range; the case of a sealed payload of 2^27 + 17 zero bytes, whose payload digest is 402bbe4db2eb033a483ea8a024dba5d570d16358b3e96eeefd69a806308a498c, is not stored |
| `p8-sealed-storage` | built and matches |  |
| `p8-canonical` | built and matches |  |
| `p8-aead` | built and matches |  |
| `p8-commitment` | built and matches |  |
| `p8-two-keys` | built and matches |  |
| `p8-committing-ciphertext` | built and matches | a sealed frame whose ciphertext and tag the AEAD accepts under two keys with one nonce and one associated data (found by solving POLYVAL), read with a segment key: seal_failed with reason commitment |
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
| `p8-wrap-kat` | built and matches | the first vector of Appendix C of draft-connolly-cfrg-xwing-kem-11: the X-Wing public key, encapsulation and shared secret that result records reproduce it; the draft prints no value for the vector's own inputs; the stored file holds a keys frame with one recipient entry and one sealed frame |
| `p8-wrap-tamper` | built and matches |  |
| `p8-wrap-lengths` | built and matches |  |
| `p8-wrap-target` | built and matches |  |
| `p8-wrap-conflict` | built and matches | a keys frame with one recipient entry and one sealed frame that opens, read by a reader that holds the key: no key_conflict; the shapes with two entries that unwrap to different keys, and with a held key beside an entry that unwraps to another, are not stored |
| `p8-wrap-partial` | built and matches |  |
| `p8-input-duplicate` | built and matches |  |
| `p8-input-mismatch` | built and matches |  |
| `p8-sealed-pack` | built and matches |  |
| `p8-seal-scope` | built and matches |  |
| `p8-members` | built and matches | 16.8, the printed chain of member indexes of sealed packs: malformed_value unordered_or_repeated at position 5 and index_claim_failed with sealed_members at position 10 (P8-44, P8-62); the files with a wrong ordinal, a member left out and a first that is not a member index are not stored |
| `p8-members-grammar` | built and matches |  |
| `p8-v1-reader` | built and matches |  |
| `p8-16.5-mutations` | built and matches | 16.5, the bullet that changes byte 0 of the sealed value: seal_failed with reason aead at the sealed frame; the other bullets of 16.5 that no other entry builds are not stored |
| `frame-value-above-header` | built and matches | the vector of P1 section 10 that waited for registry version 2 |
| `p8-writer-layout` | built and matches | three sealed frames, the first and third under one key and the second under another, with one recipient entry for the second key: the first keys frame directly before the first sealed frame and the second directly before the second sealed frame; the three sealed frames open |
| `p8-layout-pass` | built and matches | two keys frames, each directly before the sealed frame under its entry, read whole: no malformed_value for any keys frame; a transparent frame between the second keys frame and its sealed frame is not stored |
| `p8-layout-two-entries` | built and matches |  |
| `p8-layout-unused-entry` | built and matches |  |
| `p8-layout-not-next` | built and matches |  |
| `p8-layout-first-use-order` | built and matches |  |
| `p8-layout-early-use` | built and matches |  |
| `p8-members-layout-misplaced` | built and matches | one sealed pack of one member, a sealed frame whose inner type is not a pack, and a sealed member index: malformed_value unordered_or_repeated at the index frame and one index_claim_failed with sealed_members at the pack; the shape with a clear quads frame between is not stored |
| `p8-members-layout-empty` | built and matches |  |
| `p8-members-layout-missing-small` | built and matches | two sealed packs of one member each, a clear quads frame, then a sealed member index covering both: malformed_value unordered_or_repeated at the index frame and index_claim_failed with sealed_members at the last pack; the shapes with the index directly after the second pack and with no index are not stored |
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
| `p8-members-layout-pass` | built and matches | three sealed packs of 400000 members with the frames of P8-45 (the input, listed in ABSENT.md); the second input, one pack of 1048577 members with its frame, is recorded in result by its length and BLAKE3 digest only |
| `p8-members-layout-missing` | built and matches | two sealed packs of 400000 members with a frame after the second covering both (the input, listed in ABSENT.md); the frame absent and a clear quads frame between the last pack and the frame are recorded in result by length and BLAKE3 digest only |
| `p8-members-layout-cut` | built and matches | a frame covering two sealed packs of 600000 members (the input, listed in ABSENT.md); the second input, a frame after the first of two packs of 400000 members and a second frame after the second, is recorded in result by its length and BLAKE3 digest only |
| `p8-state-sealed-blob` | built and matches | P8 15 item 9, P3 7.2: a segment whose commit names a state with one blob entry of length 17, in sealed frames; result records three readings of the stored file: keyless, with the segment key, and with a key entry whose key conflicts; the sealing input of the second and third readings is not recorded in the case |
| `p8-sealed-pack-charge` | built and matches | a sealed pack whose directory holds two chunk members, a recipe member of charge T1 and a value member of charge T2, read by a key holder with rebuilt_bytes_per_read T1: the recipe admitted and rebuilt, the value refused, one exceeds_capacity with total_rebuilt and count T1 + T2 at the sealed frame, the rebuilds of the sealed frame in directory order; keyless: no charge, no total_rebuilt finding (P8-63) |
| `p8-audiences` | built and matches | a file of public content, two private analyses and a deliberation, each private segment sealed under a key of its own and wrapped for its readers: the read of the public, of each analysis service, of the reviewer who holds every key, and of a mirror that holds the four public keys and trusts none |
| `p8-audiences-suppress` | built and matches | the file of p8-audiences with a sealed suppress frame in the deliberation whose one entry names a public row: a keyless reader does not hide the row and counts the frame in unopened_sealed, and the reader with every key holds the row in the fold and hides it |
| `p8-private-dictionary` | derived from the text | a sealed terms frame before a clear terms frame and after one: with the key the clear frame has the expected base that its writer wrote, and without it the clear frame that follows the sealed one is inconsistent_counts, its ids are unreadable terms and its rows are dropped or held |
| `p8-rotated-audience` | built and matches | two key entries in one segment, the first wrapped for two recipients and the second for the first of them and a third: each recipient opens the frames under the keys wrapped for it, and a recipient entry for the first key that stands in the frame of the second is unordered_or_repeated and is not tried |
| `p8-recipient-conflict` | built and matches | one key entry with two recipient entries that wrap different keys: a reader with both seeds has key_conflict and opens nothing, the recipient of the sealing key opens the frame, and the recipient of the other key has seal_failed with aead |
| `p8-two-audiences` | built and matches | one segment with clear rows, sealed rows under two keys and a signed commit: unopened_sealed counts the sealed frames of every key that the reader does not hold, and the view is complete only with both keys |
| `p8-inner-count` | built and matches | the sealed pack of the worked example sealed again with member_count 2, read with the segment key: the opening passes steps 1 to 9 and step 10 gives malformed_value with inconsistent_counts; the frame is opaque with malformed, its payload state is verified, it is not folded and counts in unopened_sealed |
| `p8-inner-count-and-digest` | built and matches | the sealed pack of the worked example sealed again with member_count 2 and one byte of its inner payload_digest changed, read with the segment key: damaged_frame with payload_digest at step 9; the frame is opaque with damaged, its payload state is damaged, the count is not judged and the frame has no malformed_value |
| `p8-inner-count-and-decode` | built and matches | a pack with member_count 2 whose compressed payload begins with the generated zstd header and is cut short by one byte, sealed and read with the segment key, and the same pack as a clear frame: each has damaged_frame with decode_failed alone, is opaque with damaged and has the payload state damaged; no malformed_value is given |
| `p8-inner-external-length` | derived from the text | a sealed frame whose inner frame is an external with decoded_length 1, a payload of one byte and a wrong payload_digest, read with the segment key: the field rule of the inner type decides at step 8, malformed_value with out_of_range; the frame is opaque with malformed, step 9 is not run and the frame has no damaged_frame |
| `sealed-inner-newer-value-terms` | derived from the text | a segment of registry version 3 read by a reader of version 2 that holds the key, a sealed frame whose inner terms frame has the order 9 and a clear blob frame after it: with one byte of the inner payload_digest changed the sealed frame has damaged_frame with payload_digest at step 9 and no unsupported_critical, and the blob frame is folded; without the change it gives unsupported_critical with the subject [2, 18], is opaque with unsupported_critical and counts in unopened_sealed, and the blob frame is opaque with stopped |

Totals: built and matches: 80; derived from the text: 4 (84 entries).
