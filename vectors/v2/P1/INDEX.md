# Vectors of P1 section 10

Built by `tools/build_vectors.py` from the description of each entry; the reader's typed result is in the `.json` beside each `.bin`.

| vector | section | outcome | notes |
|---|---|---|---|
| `ser-uint-heads` | 10.1 | built and matches |  |
| `ser-int-heads` | 10.1 | built and matches |  |
| `ser-length-heads` | 10.1 | built and matches |  |
| `ser-bool` | 10.1 | built and matches | the frame is a parity frame (critical f4, transparent f5); its table and shards (P5) are not built |
| `ser-uint-long-form` | 10.1 | built and matches |  |
| `ser-uint-long-form-wide` | 10.1 | built and matches |  |
| `ser-length-long-form` | 10.1 | built and matches |  |
| `ser-list-long-form` | 10.1 | built and matches |  |
| `ser-indefinite-bytes` | 10.1 | built and matches |  |
| `ser-indefinite-text` | 10.1 | built and matches |  |
| `ser-indefinite-list` | 10.1 | built and matches |  |
| `ser-indefinite-map` | 10.1 | built and matches |  |
| `ser-map` | 10.1 | built and matches |  |
| `ser-tag` | 10.1 | built and matches |  |
| `ser-float` | 10.1 | built and matches |  |
| `ser-null` | 10.1 | built and matches |  |
| `ser-undefined` | 10.1 | built and matches |  |
| `ser-simple` | 10.1 | built and matches |  |
| `ser-negative-in-uint` | 10.1 | built and matches |  |
| `ser-text-bad-utf8` | 10.1 | built and matches |  |
| `ser-record-short` | 10.1 | built and matches |  |
| `ser-record-long` | 10.1 | built and matches |  |
| `ser-option-absent-with-value` | 10.1 | built and matches |  |
| `ser-option-present-without-value` | 10.1 | built and matches |  |
| `ser-option-tag-unregistered` | 10.1 | built and matches |  |
| `ser-enum-zero` | 10.1 | built and matches |  |
| `ser-enum-unregistered` | 10.1 | built and matches | a terms content whose order is 99; the frame is opaque with malformed |
| `ser-digest-short` | 10.1 | built and matches |  |
| `ser-digest-long` | 10.1 | built and matches |  |
| `ser-first-violation` | 10.1 | built and matches |  |
| `ser-counts-inconsistent` | 10.1 | built and matches |  |
| `ser-payload-trailing` | 10.1 | built and matches |  |
| `ser-allocation` | 10.1 | built and matches |  |
| `file-empty` | 10.2 | built and matches |  |
| `file-first-incomplete` | 10.2 | built and matches |  |
| `file-first-undelimitable` | 10.2 | built and matches |  |
| `file-not-this-format` | 10.2 | built and matches |  |
| `lineage-previous-unreadable` | 10.2 | built and matches |  |
| `lineage-previous-links-unavailable` | 10.2 | built and matches |  |
| `gen-record-two` | 10.2 | built and matches |  |
| `gen-record-three` | 10.2 | built and matches |  |
| `gen-record-one` | 10.2 | built and matches |  |
| `gen-record-zero` | 10.2 | built and matches |  |
| `gen-first-generation` | 10.2 | built and matches |  |
| `gen-first-generation-tagged` | 10.2 | built and matches |  |
| `gen-first-generation-other-version` | 10.2 | built and matches |  |
| `gen-first-generation-two` | 10.2 | built and matches |  |
| `gen-later-first-generation` | 10.2 | built and matches |  |
| `gen-later-header-three` | 10.2 | built and matches |  |
| `file-concat` | 10.2 | built and matches |  |
| `file-prefix` | 10.2 | built and matches |  |
| `header-minimal` | 10.2 | built and matches | h0 is the value of the vector header-h0-minimal: the `header_identifier` (and `head`) of the segment result of this file; the header is the 12 bytes of P1 10.2 |
| `header-lineage` | 10.2 | built and matches | the 48-byte header (P1 10.2) is the header of a second segment naming the head of the first; no lineage_mismatch (the only finding is canonical_violated header: a canonical claim with a lineage); h0 is the value of the vector header-h0-lineage: the `header_identifier` of the second segment result |
| `header-length-six` | 10.2 | built and matches |  |
| `header-length-eight` | 10.2 | built and matches |  |
| `header-registry-zero` | 10.2 | built and matches |  |
| `header-hash-unregistered` | 10.2 | built and matches |  |
| `header-hash-newer` | 10.2 | built and matches |  |
| `header-registry-newer` | 10.2 | built and matches |  |
| `header-long-form-field` | 10.2 | built and matches |  |
| `header-long-form-first-item` | 10.2 | built and matches |  |
| `header-element0-long-form-later` | 10.2 | built and matches |  |
| `value-zero-frame-type` | 10.2 | built and matches |  |
| `item-kind-newer` | 10.2 | built and matches |  |
| `frame-storage-tag-newer` | 10.2 | built and matches |  |
| `header-profile-unknown` | 10.2 | built and matches |  |
| `header-profile-zero` | 10.2 | built and matches |  |
| `gen-first-generation-second-only` | 10.2 | built and matches |  |
| `header-lineage-ordinal-zero` | 10.2 | built and matches |  |
| `header-lineage-digest-short` | 10.2 | built and matches |  |
| `lineage-matches` | 10.2 | built and matches |  |
| `lineage-matches-no-frame` | 10.2 | built and matches |  |
| `lineage-head-mismatch` | 10.2 | built and matches |  |
| `lineage-ordinal-mismatch` | 10.2 | built and matches |  |
| `lineage-first-present` | 10.2 | built and matches |  |
| `segment-offsets-then-frame` | 10.2 | built and matches |  |
| `segment-two-offsets` | 10.2 | built and matches | both |
| `segment-two-locators` | 10.2 | built and matches |  |
| `segment-locator-then-frame` | 10.2 | built and matches |  |
| `segment-locator-before-offsets` | 10.2 | built and matches | both |
| `segment-offsets-locator-header` | 10.2 | built and matches | a locator with no index frame before it is also index_claim_failed (index_locator); no finding for the order |
| `item-kind-unregistered` | 10.2 | built and matches |  |
| `item-not-array` | 10.2 | built and matches |  |
| `offsets-holds` | 10.3 | built and matches |  |
| `offsets-length-differs` | 10.3 | built and matches |  |
| `offsets-count-short` | 10.3 | built and matches |  |
| `offsets-count-long` | 10.3 | built and matches |  |
| `offsets-counts-opaque` | 10.3 | built and matches | the opaque item has its own malformed_value; the offsets claim holds (its length 1 is in item_lengths) |
| `locator-recognised` | 10.3 | built and matches | recognised(700) from the end; asserted in tests/test_vectors_p1.py |
| `locator-wrong-kind` | 10.3 | built and matches |  |
| `locator-short-form` | 10.3 | built and matches |  |
| `locator-zero-distance` | 10.3 | built and matches |  |
| `locator-beyond-start` | 10.3 | built and matches |  |
| `locator-item-overruns` | 10.3 | built and matches |  |
| `locator-short-input` | 10.3 | built and matches | 10 bytes: the first item is a header cut inside its last field, which ends in 82 03 1b |
| `locator-not-index-item` | 10.3 | built and matches |  |
| `locator-torn-end` | 10.3 | built and matches |  |
| `locator-lookalike-torn` | 10.3 | built and matches | the last 11 bytes are the inner locator and recognise_locator finds the inner index; the read from the start discards it |
| `frame-commit-minimal` | 10.4 | built and matches | the frame is 114 bytes; the preimage with predecessor <P> is 109 bytes (tests/test_p1_examples.py) |
| `frame-overhead` | 10.4 | built and matches | one frame per type of 3.2; length = 77 + content + payload head and bytes |
| `frame-overhead-compressed` | 10.4 | built and matches | a blob frame and a quads frame, stored_compressed; the storage element is 20 bytes against 2 |
| `frame-test-length-ten` | 10.4 | built and matches |  |
| `frame-test-length-twelve` | 10.4 | built and matches |  |
| `frame-test-element-long-form` | 10.4 | built and matches |  |
| `frame-test-tagged` | 10.4 | built and matches |  |
| `frame-flags-critical` | 10.4 | built and matches |  |
| `frame-flags-transparent` | 10.4 | built and matches |  |
| `frame-compressed-empty` | 10.4 | built and matches |  |
| `frame-plain-empty` | 10.4 | built and matches |  |
| `frame-content-shape` | 10.4 | built and matches |  |
| `frame-content-wrong-type` | 10.4 | built and matches |  |
| `identifier-ill-typed-identity-well-typed` | 10.4 | built and matches | Case F of 5.2 |
| `identifier-ill-typed-identity-ill-typed` | 10.4 | built and matches | Case G of 5.2; F1 and F2 broken, F3 and F4 linked |
| `frame-identifier-short` | 10.4 | built and matches |  |
| `frame-identifier-type` | 10.4 | built and matches |  |
| `frame-unknown-noncritical` | 10.4 | built and matches | built in a segment of registry_version 2 (reader 1): the text does not state the version (Q) |
| `frame-unknown-critical` | 10.4 | built and matches | registry_version 2, reader 1 |
| `frame-unknown-transparent` | 10.4 | built and matches |  |
| `frame-unknown-content-map` | 10.4 | built and matches | the unknown-type finding stands beside the malformed_value (both the field-rule step of P7 3, one position: ascending on the encoding, code 5 before code 7, P7 1.4). registry_version 2, reader 1: at registry_version 1 the type 99 is itself unregistered_value, which comes first |
| `frame-enum-unknown-in-content` | 10.4 | built and matches |  |
| `frame-enum-unknown-newer` | 10.4 | built and matches |  |
| `integrity-hash-unimplemented` | 10.4 | built and matches |  |
| `parity-signature` | 10.4 | built and matches |  |
| `parity-type-second-form` | 10.4 | built and matches |  |
| `parity-geometry-newer` | 10.4 | built and matches |  |
| `parity-critical-newer` | 10.4 | built and matches |  |
| `parity-codec-newer` | 10.4 | built and matches |  |
| `parity-shape-newer` | 10.4 | built and matches |  |
| `parity-capacity` | 10.4 | built and matches |  |
| `parity-capacity-and-flag` | 10.4 | built and matches |  |
| `parity-geometry-same` | 10.4 | built and matches |  |
| `parity-damaged-and-used` | 10.4 | built and matches | the findings of P5 about the use of the frame are not built in this slice |
| `frame-value-above-header` | 10.4 | could not be built | built in section P8 (tools/vec_p8.py, name frame-value-above-header): it needs a reader of registry version 2, which this builder of reader-version-1 cases cannot state |
| `pred-transparent-between` | 10.4 | built and matches |  |
| `pred-opaque-between` | 10.4 | built and matches |  |
| `pred-transparent-not-bool` | 10.4 | built and matches |  |
| `pred-transparent-report` | 10.4 | built and matches |  |
| `storage-rewrite` | 10.4 | built and matches | the frame recompressed at another level with a signature added has the same identifier as the original; its one signature is by a key the reader does not hold: signature_unverified with key_not_held |
| `zstd-ok` | 10.5 | built and matches | the reference encoder (libzstd 1.5.7, level 12, window log 26) on 300 bytes; its frame begins with H(300) = 28 b5 2f fd 60 2c 00 |
| `zstd-header-rows` | 10.5 | built and matches | each payload is the reference encoder's frame of D zero bytes and begins with H(D) (checked by tests/test_rev4.py) |
| `zstd-header-differs` | 10.5 | built and matches |  |
| `zstd-header-short` | 10.5 | built and matches |  |
| `zstd-two-frames` | 10.5 | built and matches |  |
| `zstd-trailing-byte` | 10.5 | built and matches |  |
| `zstd-reserved-block` | 10.5 | built and matches |  |
| `zstd-walk-overrun` | 10.5 | built and matches |  |
| `zstd-output-short` | 10.5 | built and matches | H(6) and one raw block of 5 bytes |
| `link-ok` | 10.6 | built and matches |  |
| `link-healed` | 10.6 | built and matches |  |
| `link-content-damage` | 10.6 | built and matches |  |
| `link-two-damaged` | 10.6 | built and matches |  |
| `link-last-damaged` | 10.6 | built and matches |  |
| `link-healed-effective` | 10.6 | built and matches | the range leaf of P5 is not built here; the effective identifier of the first frame is the computed one |
| `parity-damaged-link` | 10.6 | built and matches |  |
| `parity-flag-differs` | 10.6 | built and matches |  |
| `parity-damaged-field` | 10.6 | built and matches |  |
| `link-first-frame` | 10.6 | built and matches |  |
| `link-missing-frame` | 10.6 | built and matches |  |
| `link-inserted-frame` | 10.6 | built and matches |  |
| `link-transparent-damaged` | 10.6 | built and matches |  |
| `link-healed-before-transparent` | 10.6 | built and matches |  |
| `link-header-changed` | 10.6 | built and matches | the byte changed is registry_version 1 -> 2, so unsupported_registry_version is also reported |
| `link-successor-of-broken-binds-no-header` | 10.6 | built and matches | the two headers differ in their profile; the effective identifier of the second frame is the same in both files and the header identifiers differ |
| `link-broken-in-middle` | 10.6 | built and matches | the first two frames differ between the files; the third keeps one stored identifier in both; the fourth has one effective identifier in both |
| `link-unbroken-binds-header` | 10.6 | built and matches | the first pair of link-successor-of-broken-binds-no-header with the identifier of the first frame correct for each header: the effective identifier of the second frame differs between the two files |
| `link-malformed-frame` | 10.6 | built and matches |  |
| `stored-matches` | 10.6 | built and matches |  |
| `stored-mismatch` | 10.6 | built and matches |  |
| `stored-check-short` | 10.6 | built and matches |  |
| `stored-plain-none` | 10.6 | built and matches |  |
| `payload-verified` | 10.6 | built and matches |  |
| `payload-length` | 10.6 | built and matches |  |
| `payload-digest` | 10.6 | built and matches |  |
| `payload-decoder-error` | 10.6 | built and matches |  |
| `payload-output-longer` | 10.6 | built and matches | the payload begins with H(10), not H(6): its header differs (P1 3.5 step 1); the decoder is not run |
| `payload-after-stop` | 10.6 | built and matches |  |
| `payload-fields-ill-typed` | 10.6 | built and matches |  |
| `stored-above-capacity` | 10.6 | built and matches |  |
| `payload-codec-constraint-twice` | 10.6 | built and matches |  |
| `payload-exceeds-stored` | 10.6 | built and matches |  |
| `payload-exceeds-decoded` | 10.6 | built and matches |  |
| `payload-unknown-codec` | 10.6 | built and matches |  |
| `payload-before-link` | 10.6 | built and matches | the compressed payload claims 50000000 bytes and holds 5; it is decoded only after the link check, under the declared length |
| `torn-frame` | 10.7 | built and matches |  |
| `torn-after-frames` | 10.7 | built and matches |  |
| `torn-header` | 10.7 | built and matches |  |
| `undelimitable-reserved` | 10.7 | built and matches |  |
| `undelimitable-integer-indefinite` | 10.7 | built and matches |  |
| `undelimitable-break` | 10.7 | built and matches |  |
| `undelimitable-chunk` | 10.7 | built and matches |  |
| `undelimitable-simple` | 10.7 | built and matches |  |
| `undelimitable-nested` | 10.7 | built and matches |  |
| `undelimitable-odd-map` | 10.7 | built and matches |  |
| `torn-nested` | 10.7 | built and matches |  |
| `delimited-simple` | 10.7 | built and matches |  |
| `delimited-outside-grammar` | 10.7 | built and matches |  |
| `opaque-leaves-pred` | 10.7 | built and matches |  |
| `read-failed` | 10.7 | built and matches |  |
| `no-exception-prefixes` | 10.7 | built and matches | 67998 prefixes read, 0 exceptions |
| `no-exception-random` | 10.7 | built and matches | 3000 random inputs read, 0 exceptions |
| `boundary-claim-wrong` | 10.7 | built and matches |  |
| `boundary-hint-embedded` | 10.7 | built and matches |  |
| `hctx-header` | 10.8 | built and matches |  |
| `hctx-frame` | 10.8 | built and matches |  |
| `hctx-payload` | 10.8 | built and matches |  |
| `hctx-stored` | 10.8 | built and matches |  |
| `hctx-unit` | 10.8 | built and matches |  |
| `hctx-range_leaf` | 10.8 | built and matches |  |
| `hctx-range_node` | 10.8 | built and matches |  |
| `hctx-range_root` | 10.8 | built and matches |  |
| `hctx-shard` | 10.8 | built and matches |  |
| `hctx-parity_table` | 10.8 | built and matches |  |
| `hctx-signature_subject` | 10.8 | built and matches |  |
| `hctx-key_id` | 10.8 | built and matches |  |
| `hctx-gear_table` | 10.8 | built and matches |  |
| `hctx-source_bytes` | 10.8 | built and matches |  |
| `hctx-nested_term` | 10.8 | built and matches |  |
| `hctx-distinct` | 10.8 | built and matches |  |
| `hctx-empty-message` | 10.8 | built and matches |  |
| `preimage-identity` | 10.8 | built and matches |  |
| `preimage-stored` | 10.8 | built and matches |  |
| `ser-trailing-before-count` | 10.9 | built and matches | a quads frame with one extra byte and row_count 3 over 2 rows: the trailing byte is judged before the count |
| `gen-first-generation-v-twice` | 10.9 | built and matches | the pairs "v": "x" and "v": 1: the first decides and is not a uint, so step 5 does not hold; step 6, not_this_format |
| `header-two-violations-byte-order` | 10.9 | built and matches | a later header whose generation is 3 and whose lineage has a previous_head of 31 bytes: one finding, out_of_range, for generation |
| `header-outcome-unreadable-first` | 10.9 | built and matches | registry_version 2 (reader 1), hash_algorithm 9 and profile [2, 0]: unreadable (unregistered_value), not links_unavailable |
| `locator-last-index` | 10.9 | built and matches | two index frames and a locator: naming the first is index_locator, naming the second (the last) is no finding |
| `frame-signatures-unordered` | 10.9 | built and matches | two records in descending bytewise order of their encodings: unordered_or_repeated, opaque malformed |
| `frame-flags-order` | 10.9 | built and matches | critical true (equality with the registry column judged after element 2) and a text transparent: forbidden_combination first |
| `frame-unknown-content-bad-utf8` | 10.9 | built and matches | frame_type 99 with 62 c3 28 as content in a segment of registry_version 2: wrong_type (P1-10), beside the unknown type; both are the field-rule step at one position, so unknown_enumeration_value (code 5) stands before malformed_value (code 7) by the bytewise order of P7 1.4 |
| `payload-exceeds-total` | 10.9 | built and matches | frames of decoded_length 4, 4 and 1 with decoded_bytes_per_read 8: the third is refused, count 9 (the sum 8 plus its own length) |
| `payload-total-at-capacity` | 10.9 | built and matches | a total equal to the capacity is not refused |
| `payload-total-after-refusal` | 10.9 | built and matches | a frame of decoded_length 0 after the refusal is refused too, count 8: the total does not grow by a refused frame, and every later frame is refused |
| `payload-total-order` | 10.9 | built and matches | two segments: the frames of the first segment are judged first, the frame of the second brings the total above the capacity, no frame of the first is refused |
| `items-over-limit` | 10.9 | built and matches | H and three frames with items_per_read 3: the header and two frames are read and judged; the bytes of the third are named, count 4 |
| `items-at-limit` | 10.9 | built and matches | H and two frames with items_per_read 3: all read, no finding |
| `ser-deep-list` | 10.1 | built and matches | 1000000 heads 81 in the content of a frame of type 99 (critical false), registry_version 2, reader 1; the depth changes neither result |
| `gen-record-tagged` | 10.2 | built and matches |  |
| `locator-index-type-long-form` | 10.3 | built and matches | element 1 of the index frame is 18 0f: no value, so the frame has no frame type and the locator names no index frame |
| `frame-field-outside-grammar` | 10.4 | built and matches |  |
| `payload-ill-formed-head` | 10.6 | built and matches | the sixth: a text where a uint is expected, then 1c (the earlier violation is the type); the seventh: a text head of 5 announced bytes of which 2 are present |
| `torn-deep` | 10.7 | built and matches |  |
| `delimited-deep` | 10.7 | built and matches |  |
| `undelimitable-deep` | 10.7 | built and matches |  |
| `append-after-canonical` |  | built and matches | a canonical file with a segment appended |
| `append-after-recorded-cut` |  | built and matches | the writer recorded the length of H T Q and stopped after 10 bytes of the next segment; its holder cuts to the recorded length |
| `append-after-torn-naive` |  | built and matches | the first file of append-after-torn-refused with a segment of fewer than m bytes appended after its incomplete frame, which no conforming writer writes |
| `append-cut-boundary-prefix` |  | built and matches | H T Q cut exactly at the end of T |
| `append-cut-boundary-segment` |  | built and matches | the file of append-cut-boundary-prefix with H2 T Q appended; the file of append-torn-payload-segment |
| `append-cut-boundary` |  | built and matches | H T Q |
| `append-segment-lineage` |  | built and matches | the file H T Q and the same file with the segment H2 T Q appended |
| `append-segment-unreadable-head` |  | built and matches | the appended segment has lineage absent; a reader reports lineage_begins for it |
| `append-torn-header-prefix` |  | built and matches | the bytes before that offset: H T Q |
| `append-torn-header-segment` |  | built and matches | that prefix file with H2 T Q appended |
| `append-torn-header` |  | built and matches | H T Q then the first 10 bytes of H |
| `append-torn-payload-prefix` |  | built and matches | H T |
| `append-torn-payload-segment` |  | built and matches | H T with H2 T Q appended |
| `append-torn-payload` |  | built and matches | H T then the bytes of Q up to and including the first byte of the content of its payload |
| `append-two-writers` |  | built and matches | a segment and two frames appended after its last frame by two writers, each computed with that frame as predecessor |
| `concat-append-built` |  | built and matches | two files each built by appending a second segment with lineage to its first, concatenated |
| `concat-concat` |  | built and matches | (A0 A1 B0 B1) then C0 C1, and A0 A1 then (B0 B1 C0 C1): equal bytes and equal results |
| `concat-continuation` |  | built and matches | a file and a file that continues it, whose first segment names its head and its ordinal |
| `concat-resolves` |  | built and matches | F with a binding whose blob referent F does not declare, G declaring the blob with a suppress entry on the line of g and a line of graph h |
| `continue-close` |  | built and matches | a segment written by one writer in several requests of one continuous use, and the same segment written at once |
| `continue-segment` |  | built and matches | a segment written by one writer in several requests of one continuous use, and the same segment written at once |
| `append-after-torn-refused` |  | built and matches | (i) Q cut inside its payload with 100 bytes missing: appended_resync, filler of 102 bytes, the item at L + 102 with the torn_tail and filler marks; (ii) H T Q then 1c: appended_resync, no filler (P1-87) |
| `append-check-filler-after-damage` |  | built and matches | a checking writer, H T Q1 X Q2 Q3 with X failing its signatures grammar in mid-file and Q3 cut in its payload: the read ends in the incomplete Q3, so it appends the filler and a resync item reached by framing (P1-87) |
| `append-check-filler` |  | built and matches | a checking writer, H T Q with Q cut in its payload: a filler, then a resync item reached by framing (P1-87) |
| `append-check-recovery-resumed` |  | built and matches | H T Q1 D Par Q2, D the five bytes 1c, Par a parity frame at which recovery resumes, the file ending at the complete Q2: the walk with no recovery stops at D: appended_resync, no filler, the item alone (P1-87) |
| `append-damaged-length-head` |  | built and matches | a segment in which one payload length head is damaged so that the walk ends in an incomplete item: (i) 300 bytes beyond the end: a filler; (ii) 10000: no filler, the item alone, found by the scan (P1-87) |
| `append-empty-file` |  | built and matches | a file of zero bytes and the segment H T Q to append: appended; the file after it H T Q, H with lineage absent, one finding uncommitted_tail count 2 at position 1 (P1-87) |
| `append-foreign-incomplete` |  | built and matches | a file of the two bytes 83 07: refused_generation (P1-87) |
| `append-foreign-undelimitable` |  | built and matches | the bytes ff d8 ff: refused_generation (P1-87) |
| `append-generation` |  | built and matches | a first-generation header, a header [1, 3, ...] and a first-generation map followed by a frame: refused_generation in each (P1-87) |
| `append-header-after-torn` |  | built and matches | the first file of append-after-torn-refused with a segment of fewer than 100 bytes appended after its incomplete frame: torn_tail at its offset, the appended header swallowed and unread (P1-87) |
| `append-opaque-tail` |  | built and matches | H T Q then 00 00 00, and the segment H2 T Q to append: appended; the first segment's items at 3 to 5 opaque with malformed_value wrong_type (P1-87) |
| `append-read-failed` |  | built and matches | H T Q with a read that fails at offset 0 for 11 bytes, and at the offset after H for 11 bytes: refused_read, nothing written (P1-87) |
| `append-resync-after-torn` |  | built and matches | H T Q, the first 10 bytes of H2, then a resync item at that length from a writer that holds the head (P1-85) |
| `append-resync-fresh-after-pad` |  | built and matches | a first item that states a length that is not its landing offset, then a fresh item that states it: both reached by framing; after damage, the scan accepts the second and rejects the first at part 3 (P1-85) |
| `append-resync-length-only` |  | built and matches | a writer given only the length of the file writes the bytes of a writer that read the file, when that writer's read gives it no lineage, past or marks to write (P1-85) |
| `append-torn-first-header` |  | built and matches | the first 10 bytes of H: appended_resync, no filler, the resync item alone at offset 10; the one byte 1c: refused_generation (P1-87) |
| `lineage-ordinal-after-absent` |  | built and matches | a segment with lineage absent in the middle, then a segment naming its head with ordinal 2 and 7: no lineage_mismatch, the lineage checked in both (P1-88) |
| `link-heal-closing-items` |  | built and matches | H T Q1 X O Loc C1 C2, X a signed commit with one bit flipped in its stored identifier, C1 and C2 the commits of co-signers: X broken, C1 broken, C2 linked and failing ROOT, no signature authenticates; with the bit restored everything valid and authenticating, nothing scanned (5.2) |
| `link-heal-opaque-between` |  | built and matches | an ordinary frame whose identifier field is damaged, then `82 00 80`, then the frame its writer formed over the computed identifier: broken, opaque, broken, linked (5.2) |
| `link-heal-region-between` |  | built and matches | the file of link-heal-opaque-between with five bytes `1c` in place of `82 00 80` and a parity frame after them: the first frame broken with chain_break, the resumption at the parity frame as P5 gives it, no healed_link (5.2) |
| `marks-code-zero` |  | built and matches | a hostile mark whose strains is 0: ignored and counted (P1-96) |
| `marks-cut-inside-mark` |  | built and matches | two items longer than resync_max bytes, each with one mark that begins before byte 4096 of the item and holds 18 05 in place of a uint: at byte 4094, not_deterministic; at byte 4097, out_of_range (P1-96) |
| `marks-filler-apart` |  | built and matches | the file of resync-filler-capacity-spent with a writer whose read is complete: a torn_tail mark for Q2' by the ordinary grouping, the torn-tail mark over Q4', the filler mark: three marks, none merged; and the same writer fitting an item of 64 marks keeps both (P1-111) |
| `marks-filler-no-effect` |  | built and matches | the file of marks-torn-tail-recorded and the same file whose filler mark is removed, the frames re-made over the h0 of the item: equal results in every field that 2.4b names (P1-111) |
| `marks-filler-on-scope` |  | built and matches | a filler mark on the scope: ignored and counted (P1-96) |
| `marks-future-finding-code` |  | built and matches | a mark naming finding 50: readable, ignored and counted by a reader of version 1, applied by a reader of version 2, the same bytes (P1-96) |
| `marks-head-not-shortest` |  | built and matches | two items, each with one hostile mark on the scope: strains written 18 05 in place of 05, and the head of the mark's own array written 98 04 in place of 84: unreadable, malformed_value not_deterministic, past absent, marks_ignored 0 (P1-96) |
| `marks-hostile-displaced` |  | built and matches | the file F of 1000 bytes joined before G, whose item at 400 carries a hostile mark on [100, 150): a reader that honours marks refuses the fold of the first segment of G at [1100, 1150) and not that of F, as G read alone does (P7-82) |
| `marks-ignored-equal` |  | built and matches | two files that differ only in the content of marks of equal encoded length, the frames re-made over the h0 of each item: equal results in the fields the three properties name (P1-98) |
| `marks-no-fold-effect` |  | built and matches | the pairs of marks-ignored-equal: equal folds and content digests (P1-99) |
| `marks-none` |  | built and matches | a file of a header segment and two segments opened by resync items with an empty marks list, the second after a filler: no finding names a mark, opener.marks_ignored is 0 for both, and marks of the result is absent (P1-97) |
| `marks-observed-judged-by-nothing` |  | built and matches | two files that differ only in the counts of their observed_defect marks, the frames of each item re-made over its h0: equal results in every field that 2.4b names (P1-111) |
| `marks-observed-on-scope` |  | built and matches | an observed_defect mark on the scope: ignored and counted (P1-96) |
| `marks-order-at-one-byte` |  | built and matches | four items, each unreadable: a 65th mark that also sorts below the 64th: out_of_range; a second mark whose first element is a text and that sorts below the first: wrong_type; a second mark that sorts below the first and holds a head not in its shortest form deeper inside: unordered_or_repeated; a first mark that holds such a head and a second that sorts below it: not_deterministic (P1-96) |
| `marks-outside-grammar` |  | built and matches | an item whose one mark holds f6 in place of its strains: unreadable, malformed_value wrong_type; and an item whose one mark has an unknown tag and holds the array 82 01 02 after it: readable, the mark ignored and counted; and an item whose one mark is the empty array 80: unreadable, wrong_type (P1-96) |
| `marks-range-after-item` |  | built and matches | a byte target that ends after the item: ignored and counted (P1-96) |
| `marks-range-empty` |  | built and matches | a byte target of length 0: ignored and counted (P1-96) |
| `marks-registry-version-not-raised` |  | built and matches | an item of registry_version 1 whose mark names finding 50: a conforming file; no registry finding (P1-96) |
| `marks-torn-tail-recorded` |  | built and matches | a frame cut in its last signature value, the filler, then a resync item whose marks hold the torn_tail mark over [b, L) and the filler mark over [L, L + m): reached by framing, opener [2, walked, false, matches, 0] (P1-111) |
| `marks-unknown-tag` |  | built and matches | a mark of tag 4: readable, ignored and counted (P1-96) |
| `resync-after-damaged-length-head` |  | built and matches | a damaged length head in mid-file followed by a resync item that a writer wrote later: torn_tail for the frame (P1-102) |
| `resync-after-torn-header` |  | built and matches | a header cut, then a resync item: torn_tail, the item accepted (P1-102) |
| `resync-after-torn-staying` |  | built and matches | an incomplete item that stays incomplete over the appended bytes (a declared length beyond them): the item found by the scan, torn_tail before it (P1-102) |
| `resync-after-torn-swallowed-failing` |  | built and matches | a torn frame completed by the bytes of the item into an item that fails: torn_tail from its first byte, the item accepted inside it (P1-102) |
| `resync-after-torn-swallowed-passing` |  | built and matches | a torn locator completed by the item's bytes into an item that passes, the item after it failing: the completed item stays, no framing finding (P1-102) |
| `resync-after-torn-swallowed` |  | built and matches | a torn item that the appended bytes complete: into an item that fails (torn_tail from its first byte, the item accepted inside it) and into an item that passes with the item after it failing (the completed item stays) (P1-102) |
| `resync-after-undelimitable` |  | built and matches | bytes that cannot be delimited, then a resync item: undelimitable_input over them, the region one item of the previous segment (P1-102) |
| `resync-candidate-marks-unordered` |  | built and matches | an occurrence with a true offset and identifier whose two marks stand in descending order: part 5 fails, no candidate (P1-101) |
| `resync-capacity-decides-nothing` |  | built and matches | a file read with a stored_bytes_per_frame below and above the size of the frame: the same segments, opener and framing; the findings differ only by the capacity findings (P1-100) |
| `resync-chain` |  | built and matches | the first ordinary frame after a resync item links to its h0 (the computed identifier); a segment with no frame has it as its head (P1-94) |
| `resync-charge-before-read` |  | built and matches | a candidate with a true offset refused by the total: exceeds_capacity total_decoded, no byte after the offset read, no later candidate (P1-101) |
| `resync-charge-refused-in-region` |  | built and matches | a candidate refused by the total in a damaged region: the finding with the position the next item would have, in the previous segment (P1-101) |
| `resync-cosigned-not-scanned` |  | built and matches | `... I Par O Loc C`, C the commit of a co-signer, the payload of the parity frame holding a forged resync item with a true offset: malformed_value/unordered_or_repeated for O and Loc, nothing scanned (P1 6.7) |
| `resync-crash-after-filler` |  | built and matches | a crash after the filler and before the first byte of the item: the next writer's read ends at the boundary after F', it writes no filler, its item stands at that boundary (P1 THE CRASH AT EVERY POINT, THE GAP) |
| `resync-crash-hole` |  | built and matches | a hole of 40 bytes 00 in the writer's own bytes: at item boundaries 40 opaque items, R reached by framing; and inside the payload of Q: Q damaged (P1 THE CRASH AT EVERY POINT) |
| `resync-crash-in-filler` |  | built and matches | a crash after j of the m filler bytes: the second writer's filler is the remaining m - j, the file reads as with one whole filler (P1 THE CRASH AT EVERY POINT) |
| `resync-crash-in-item` |  | built and matches | the file ends in an incomplete resync item T; the next restart appends R' at the length of the file: framing is lost at T, the scan covers it, R' is accepted, the bytes before it torn_tail (P1 THE CRASH AT EVERY POINT) |
| `resync-crash-in-restart-item` |  | built and matches | T, then the partial R' of j bytes, then R'': the scan covers T and finds R'' (the partial R' has no identifier) (P1 THE CRASH AT EVERY POINT) |
| `resync-displaced-join` |  | built and matches | a resync item whose offset differs from its place, reached by framing: displaced true and no finding (P1-95) |
| `resync-field-differs` |  | built and matches | a byte of a mark changed: readable, no finding at the item, the first ordinary frame broken with chain_break, later links linked (P1-92) |
| `resync-field-head-not-shortest` |  | built and matches | a frame whose decoded_length is written `18 05`: it fails the framing test and is scanned with the item before it in a readable v1 segment and in a readable v2 segment whose hash algorithm v1 holds; in a links_unavailable segment it passes and nothing is scanned; a stored_plain frame whose payload holds `fb` passes in every segment (P1 6.7) |
| `resync-filler-cap` |  | built and matches | a frame cut with 4094 payload bytes missing: the filler is 4096 bytes; with 4095 missing: no filler, found by the scan (P1-108) |
| `resync-filler-capacity-spent` |  | built and matches | H T Q1 Q2' R1 T2 Q3 Q4': a writer with ample decoded_bytes_per_read appends the filler that completes Q4' and its item; a writer whose total is below the charge of R1 makes no complete read and appends its item alone, found by the scan (P1-108) |
| `resync-filler-evidence-commit` |  | built and matches | H(profile evidence) T Q C, C signed with mldsa65_ed25519, cut with 1000 bytes of its 3373-byte value missing: a filler of 1000 bytes; the scope R with profile evidence (P1-108) |
| `resync-filler-first-item-torn` |  | built and matches | the files `87 01` and `87 01 02`: no filler, the resync item alone at offset 2 or 3, found by the scan from offset 0; the bytes before it torn_tail (P1-108) |
| `resync-filler-head-at-limit` |  | built and matches | a frame cut directly after the payload head bytes `59 0f`: the filler is 3843 bytes; cut after `59 10`: 4099 needed, no filler (P1-108) |
| `resync-filler-header` |  | built and matches | a header cut after element 3: unreadable, an empty segment; the lineage of the next segment absent and lineage_begins (P1-108) |
| `resync-filler-identifier` |  | built and matches | a frame cut inside its identifier with k = 5 bytes missing: the filler is k + 1 bytes (P1-108) |
| `resync-filler-none-long-payload` |  | built and matches | a frame cut with 5000 payload bytes missing: no filler, the item alone, found by the scan (P1-108) |
| `resync-filler-offsets-entry-head` |  | built and matches | an offsets item cut directly after the head byte `19` of an entry: the filler `00 00` gives `19 00 00`, not_deterministic; it fails (P1-108) |
| `resync-filler-offsets-entry-leading-zero` |  | built and matches | an offsets entry `1a 00 01 11 70` cut after `1a 00`: the filler `00 00 00` gives `1a 00 00 00 00`, not_deterministic, the item fails; cut after `1a 00 01`: the grammar holds (P1-108) |
| `resync-filler-offsets-list-head` |  | built and matches | an offsets item of a segment of 30 items cut directly after the list head `98`: the filler `00` gives `98 00`, not_deterministic; it fails (P1-108) |
| `resync-filler-offsets` |  | built and matches | an offsets item cut at an entry boundary with entries still declared: it passes, its claim fails (P1-108) |
| `resync-filler-one-call` |  | built and matches | the filler and the item land in one call, verified at the first byte of the filler (P1-108, THE LANDING OFFSET) |
| `resync-filler-parity` |  | built and matches | a parity frame cut inside its payload: parity_damaged alone, fold state not_content, not counted by left_out; it fails, the resync item read by framing (P1-108) |
| `resync-filler-payload` |  | built and matches | a frame cut inside its payload with 200 bytes missing: the filler is 202 bytes (P1-108) |
| `resync-filler-resync-item` |  | built and matches | a resync item cut inside its identifier: readable, its stored identifier differs, an empty segment, then the item (P1-108) |
| `resync-filler-signed-commit-plain` |  | built and matches | H T Q C, C signed by K1, cut with m bytes of its last value missing, a filler and a resync item appended: C' valid, its signature invalid with K1 trusted and unverified without it; no uncommitted_tail in the first segment; before the append torn_tail, uncommitted_tail for Q, end torn (P1-108) |
| `resync-filler-table-frame` |  | built and matches | the item of kind frame cut at every byte: the filler that completes it, then the resync item reached by framing; the completed item passes or fails the framing test as its row says (P1-108) (215 files) |
| `resync-filler-table-header` |  | built and matches | the item of kind header cut at every byte: the filler that completes it, then the resync item reached by framing; the completed item passes or fails the framing test as its row says (P1-108) (48 files) |
| `resync-filler-table-locator` |  | built and matches | the item of kind locator cut at every byte: the filler that completes it, then the resync item reached by framing; the completed item passes or fails the framing test as its row says (P1-108) (10 files) |
| `resync-filler-table-offsets` |  | built and matches | the item of kind offsets cut at every byte: the filler that completes it, then the resync item reached by framing; the completed item passes or fails the framing test as its row says (P1-108) (66 files) |
| `resync-filler-table-other-kind-bytes` |  | built and matches | an item of another kind than the table's (bytes, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (33 files) |
| `resync-filler-table-other-kind-float` |  | built and matches | an item of another kind than the table's (float, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (8 files) |
| `resync-filler-table-other-kind-map` |  | built and matches | an item of another kind than the table's (map, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (4 files) |
| `resync-filler-table-other-kind-negint` |  | built and matches | an item of another kind than the table's (negint, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (4 files) |
| `resync-filler-table-other-kind-tag` |  | built and matches | an item of another kind than the table's (tag, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (3 files) |
| `resync-filler-table-other-kind-text` |  | built and matches | an item of another kind than the table's (text, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (33 files) |
| `resync-filler-table-other-kind-uint` |  | built and matches | an item of another kind than the table's (uint, not an array) cut at every byte: completed as the walk declares, judged as it is (P1 6.7 test 1: it fails), the resync item reached by framing under whole and forward (4 files) |
| `resync-filler-table-parity` |  | built and matches | the item of kind parity cut at every byte: the filler that completes it, then the resync item reached by framing; the completed item passes or fails the framing test as its row says (P1-108) (552 files) |
| `resync-filler-table-resync-item` |  | built and matches | the item of kind resync-item cut at every byte: the filler that completes it, then the resync item reached by framing; the completed item passes or fails the framing test as its row says (P1-108) (100 files) |
| `resync-filler-unreadable-segment` |  | built and matches | the input of the second limit with a filler: the item is reached by framing inside the unreadable segment and opens its own segment (P1-108) |
| `resync-filler-value-head` |  | built and matches | the same commit cut after `59 0d`: a filler of 3329 bytes, the value invalid with value_length (P1-108) |
| `resync-filler-value-not-last` |  | built and matches | a commit with two signature records cut inside the value of the first: the filler completes it and makes the second record `00`; malformed_value/wrong_type, link linked, it fails, the item reached by framing (P1-108) |
| `resync-first-byte-foreign` |  | built and matches | the bytes ff d8 then a resync item with a true offset: undelimitable_input from 0 up to the item, then the scope (P1-19, P1-105) |
| `resync-first-item-fails` |  | built and matches | a complete first item that fails (the map {x: bytes} holding a resync item with a true offset): the item accepted inside it, no not_this_format (P1-105) |
| `resync-first-item-torn` |  | built and matches | a torn first item then a resync item with a true offset: torn_tail from 0, the file continues at the item (P1-105) |
| `resync-first-item` |  | built and matches | a file that begins with a resync item (P1-105) |
| `resync-flood-false-offsets` |  | built and matches | one MiB of occurrences whose offsets are false: no charge, no finding (P1-104) |
| `resync-flood-true-offsets` |  | built and matches | occurrences with true offsets and false identifiers, with a decoded_bytes_per_read that refuses the (cap / 4096 + 1)-th: one exceeds_capacity total_decoded, no later candidate (P1-104) |
| `resync-full` |  | built and matches | an item with lineage, past and three marks (P1-91) |
| `resync-hash-unknown` |  | built and matches | an occurrence whose hash_algorithm is of a later version: no candidate for every reader (P1-101) |
| `resync-heal-at-item-limit` |  | built and matches | H T X Y, items_per_read 3, X the third item: reading stops, X broken, no candidate tested, Y not read, exceeds_capacity item_count (P1-100) |
| `resync-heal-next-item-only` |  | built and matches | X with a bit of its identifier flipped and a forged item in its payload; the item after it K = 82 00 80 or O1 O2 O3 = 82 04 80: X broken, the forged item accepted (P1-100) |
| `resync-heal-no-witness-past-region` |  | built and matches | H T X D P Y: X as in (i), D the bytes 1c x 5, P a parity frame after X: X broken, decided at D; the forged item accepted (P1-100) |
| `resync-heal-witness-removed` |  | built and matches | H T X W // R: X broken with chain_break, the forged..., R accepted in W with decoded_bytes_per_read at least t + d_X + 4096; below it R refused by total_decoded and X healed (P1-100) |
| `resync-healed-not-scanned` |  | built and matches | a frame whose stored identifier is damaged and whose payload holds a forged item, with a genuine successor: healed, not scanned (P1-100) |
| `resync-identifier-false` |  | built and matches | a damaged item after a torn tail: rejected; the torn bytes run to the end (P1-101) |
| `resync-identifier-field-differs-forgery` |  | built and matches | H T Q R T2 Q2 C2 with a forged item in Q; R has one flipped identifier bit: nothing scanned, two segments, no finding (P1-92) |
| `resync-identifier-field-differs` |  | built and matches | only the stored identifier changed: readable, no finding, every frame linked, the scope read whole, nothing scanned (P1-92) |
| `resync-isolation-bad-writer-run` |  | built and matches | P_bad holds a hole of bytes 00 (a writer's bad run): 40 opaque items; R accepted at o in both (P1-106) |
| `resync-isolation-broken-link` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind broken_link: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-capacity-spent` |  | built and matches | a prefix of one large frame against many empty frames of the same length: a total that the prefix spends refuses a frame of Y in one read and admits it in the other (P1-106) |
| `resync-isolation-checked-lineage-unavailable` |  | built and matches | R carries a lineage; the segment before it has an unreadable opener in P_bad: the lineage is not checked there and is in P_clean: the lineage findings are join fields (P1-106) |
| `resync-isolation-later-resync` |  | built and matches | a Y that holds a second resync item after bytes that cannot be delimited, accepted by a scan in both files (P1-106) |
| `resync-isolation-links-unavailable` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind links_unavailable: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-lost-header` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind lost_header: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-lost-keys` |  | built and matches | registry version 2: P_bad holds a sealed frame whose keys are lost (no keys frame): R accepted at o in both (P1-106) |
| `resync-isolation-refused-terms` |  | built and matches | P_bad holds a terms frame refused by formed_bytes_per_read; Y is read the same (P1-106) |
| `resync-isolation-stale-parity` |  | built and matches | P_bad holds a parity frame whose span no longer holds its bytes: R accepted at o in both (P1-106) |
| `resync-isolation-stopped-fold` |  | built and matches | P_bad holds an unsupported_critical frame that stops its fold: R accepted at o in both, the scope unaffected (P1-106) |
| `resync-isolation-torn-header` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind torn_header: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-torn-staying` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind torn_staying: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-torn-swallowed-failing` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind torn_swallowed_failing: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-torn-swallowed-passing` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind torn_swallowed_passing: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-undelimitable` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind undelimitable: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-unreadable-header` |  | built and matches | the file P_bad // Y and the file P_clean // Y for the defect kind unreadable_header: R accepted at o in both, equal but for the join fields (P1-106) |
| `resync-isolation-unreadable-ordinal` |  | built and matches | R, then an opener with registry_version 0, then a header naming ordinal 3, after prefixes of one and of three segments of the same length: no lineage finding for that header in either (P1-106) |
| `resync-isolation-value-served` |  | built and matches | Y = R, a terms frame defining id 0 as a literal_ref naming a value of digest d, a quads row whose object is id 0; P_bad holds a value frame of digest d, P_clean does not: the results differ exactly in the join fields of resolution; and with the value restated after R and before the terms frame: the row folded in both (P1-106) |
| `resync-item-before-g` |  | built and matches | a frame that passes and holds a forged item with a true offset, then an item that fails: accepted inside the frame; the frame stays an item (P1-100) |
| `resync-item-limit-accepted` |  | built and matches | the count and the decision point: a torn Q completed by the first bytes of a resync item; items_per_read 4, 5 and unlimited (P1-100) |
| `resync-item-limit-region` |  | built and matches | H T D with items_per_read 3, D bytes holding a resync item with a true offset: not tested, one segment, undelimitable_input, no item_count (P1-100) |
| `resync-limit-midfile-length-head` |  | built and matches | a damaged length head of a kind-6 item in a v2 segment swallows a resync item and ends at a second passing item: R is not accepted, no segment begins at it (P1 6.7 limit 4) |
| `resync-limit-torn-ends-at-measure` |  | built and matches | limit 4 (b): a frame of a segment of registry version 2 cut so that the completion ends at a measure `82 06 v` of a hostile mark of R: R is not accepted (P1-107) |
| `resync-limit-torn-ends-at-new-kind` |  | built and matches | limit 4 (a): a frame of a segment of registry_version 2 cut with 27 bytes of a signature value missing, then R(33030): the completion ends at R[27], the item `81 06` passes, `81 01` at R[29] is an unreadable header that opens a segment; R is not accepted (P1-107) |
| `resync-limit-torn-resync-lineage` |  | built and matches | limit 5 (a): a resync item torn inside its lineage, then R(L) alone: R[0..25) complete the digest, the torn item is readable with no finding and ends at the end of R; with a scope after R, R is found and accepted (P1-107) |
| `resync-limit-torn-resync-mark` |  | built and matches | limit 5 (b): a torn resync item whose last mark is cut after `84 01 81 01 03 8b 82 01 18`, then R(L) alone: `18 8b` is 139, the ten items from R[1] fill the measures and identifier is the own identifier of R: the torn item ends at the end of R (P1-107) |
| `resync-limit-torn-signature-at-end` |  | built and matches | limit 1 (a): a frame whose last signature is mldsa65_ed25519, cut with /R/ bytes of value missing, then R(L) alone: the frame completes at the end of the input, links and passes, nothing follows (P1-107) |
| `resync-limit-torn-signature-at-nested-header` |  | built and matches | limit 1 (c): an evidence commit cut with 180 bytes of its last value missing, then R(L) (66 bytes) and a stored_plain blob whose payload begins with bytes that are not a file of this format, as many as bring the nested file to L + 180: the nested header opens a segment found by framing, framing is lost after the nested file, the run does not reach L (P1-107) |
| `resync-limit-torn-signature-at-new-kind` |  | built and matches | limit 1 (d): the same commit in a segment of registry_version 2, cut with m bytes missing, the first frame of the scope a blob whose payload holds `81 06` at L + m: R is not accepted; in a segment of version 1 the `81 06` fails and R is found (P1-107) |
| `resync-limit-torn-signature-at-opener` |  | built and matches | limit 1 (b): the same frame cut with more than /R/ bytes missing, then R(L) and a scope that holds a second, genuine resync item at the boundary where the completion ends: that opener opens its own segment, R is not accepted (P1-107) |
| `resync-limit-torn-unreadable-segment` |  | built and matches | limit 2: a header with hash_algorithm 7, then a frame cut with 200 bytes missing; R(L) and a scope whose first frame is 134 bytes: the torn frame completes at the end of the first scope frame, the second scope frame passes classification: the scope frames are items of the unreadable segment (P1-107) |
| `resync-lineage-after-torn-signature` |  | built and matches | the same frame cut with 24 bytes missing, then R(L) with lineage present and no profile: the item [2, [digest, ordinal]] at R[24] fails by step 5 (P1 6.7 limit 4) |
| `resync-marker` |  | built and matches | the 32-byte output H_ctx(header, 81 05), M, the pattern and its border test (P1-93) |
| `resync-minimal` |  | built and matches | the 64-byte item with offset 0: bytes by field, the identifier a vector value (P1-91) |
| `resync-nested-intact` |  | built and matches | a blob that holds a file with a resync item whose offset is true for its place in the outer file, in a frame that passes, followed by an item that passes: not examined; equal to the file whose blob holds other bytes of the same length (P1 6.7) |
| `resync-new-kind-after-unreadable` |  | built and matches | mid-file, the unreadable opener `87 01 02 00 01 81 01 81 01 81 01` (registry_version 0), then `82 06 00`: the opener fails test 2; `82 06 00` does not fail since the opener is not readable; nothing else in the segment is judged (P1 6.7) |
| `resync-new-kind-fails-v1` |  | built and matches | the same input in a segment of registry_version 1 is no limit: `81 06` fails, the scan covers it and the torn frame, and R is accepted (P1 6.7) |
| `resync-newer-kind-not-scanned` |  | built and matches | H(registry_version 3) T Q1 K Q2, K of kind 6, Q1 a blob holding a forged item: nothing scanned, one segment, unknown_item_kind (P1-100) |
| `resync-offset-false` |  | built and matches | a nested file in a damaged region: rejected at part 3, no charge, no finding (P1-101) |
| `resync-offset-sizes` |  | built and matches | items with offset 23, 24, 255, 256, 65535, 65536, 2^32 - 1, 2^32: 64, 65, 65, 66, 66, 68, 68, 72 bytes (P1-91) |
| `resync-order-decision-point` |  | built and matches | H T F P W X: P broken and G, the scan decided at resync_max bytes after the end of P, Q accepted, torn_tail from P's first byte, W and X no items (P1-100) |
| `resync-order-held-after-g` |  | built and matches | G fails by its signatures grammar; its last record's value holds the forged item up to the head of a marks list of one element; W1 is the mark, W2 the identifier: G charged at its place, the candidate after G and before W1, W2, accepted under both accesses (P1-100) |
| `resync-other-generation-scanned` |  | built and matches | H T Q, Q a blob holding a forged item with a true offset, then a header of generation 3: the header unreadable, the run covers Q and the header, the forged item accepted (P1-100) |
| `resync-parity-first` |  | built and matches | both kinds in one region, the parity candidate first: reading resumes at it (P1-101) |
| `resync-parity-span` |  | built and matches | the parity span of a scope opened by a resync item begins at the first byte of the item (P1-94) |
| `resync-past-absent` |  | built and matches | no past: absent (P1-110) |
| `resync-past-differs` |  | built and matches | past different from the digest: differs (P1-110) |
| `resync-past-displaced-join` |  | built and matches | the item of a joined file: matches over the bytes of its own file under whole; not_checked under forward (P1-110) |
| `resync-past-matches` |  | built and matches | past equal to the unit digest of the offset bytes before the item: matches (P1-110) |
| `resync-past-not-checked-count` |  | built and matches | a decoded_bytes_per_read below the offset: the check is not made: not_checked (P1-110) |
| `resync-past-not-checked-cut` |  | built and matches | a file that begins with the item, offset above its place: not_checked (P1-110) |
| `resync-past-unit-equal` |  | built and matches | a resync item whose past is computed over the bytes B, and a blob that holds the bytes B: the past equals the unit digest of the blob (4.1) |
| `resync-position-zero` |  | built and matches | a resync item has position 0 and is no frame, leaf or predecessor (P1-94) |
| `resync-prefix-equal` |  | built and matches | each file of resync-after-* and its prefix [0, o): equal per-segment results but the stated differences (P1-103) |
| `resync-profile-after-torn-signature` |  | built and matches | a frame of a readable v1 segment whose only signature is ed25519, cut with 22 of the 64 value bytes missing, then R(L) with profile present: the completion ends at R[22], the item [2, 1] fails by step 5, R is accepted (P1 6.7 limit 4) |
| `resync-range-leaf0` |  | built and matches | the commit of a scope opened by a resync item has its root over h0, the computed identifier, as leaf 0 (P1-94) |
| `resync-restart-terms` |  | built and matches | a scope that names an id that its own terms frames do not reserve: forward_reference, and not the term of the earlier segment (P1-109) |
| `resync-resync-first` |  | built and matches | both kinds in one region, the resync candidate first: it decides (P1-101) |
| `resync-signature-subject` |  | built and matches | a commit of a scope opened by a resync item signed over the computed h0: authenticated (P1-94) |
| `resync-too-long` |  | built and matches | an item of exactly 4097 bytes with no other violation: unreadable with out_of_range, walked; by a scan, no candidate; and an item of exactly 4096 bytes: readable (P1-92) |
| `resync-unknown-type-content-passes` |  | built and matches | a readable v2 segment, two frames of a type of registry version 2, critical false: content an array of uints passes the test and nothing is scanned; content `fb` and eight bytes (a float) is not well typed, no identifier, link broken, fails the test by its link (P1 6.7) |
| `resync-unreadable` |  | built and matches | an item that is an array of 10 (P1-92): unreadable |
| `resync-writer-fits` |  | built and matches | the largest honest item: a fixed part of 152 bytes, 48 observed_defect marks of 37 bytes, the mark of the torn tail of 37, the filler mark of 22 and 14 hostile marks of 123: 3709 bytes, 64 marks, readable (P1-91) |
| `chain-break-withheld-codec-constraint` |  | built and matches | a `broken` frame whose only `malformed_value` is `codec_constraint` from the payload check has no `chain_break` (P1 5.2 step 4, P1-62) |
| `envelope-field-before-content-value` |  | built and matches | a `quads` frame of the reader's registry version, written as an item of 11 elements whose element 10, `signatures`, is the `uint` `2`, with its `order` changed by one bit from `1` to `0` so that its link fails: `malformed_value` with `wrong_type`, the finding of element 10 alone and no `unregistered_value`; the frame opaque with `malformed`, link `broken` and no `chain_break` (P1-36) |
| `first-item-five-alone` |  | built and matches | a first item `[5]`, an array whose element 0 is `5` and which has no element 1: no record grammar; scanned; `not_this_format`, no segment (P1 2.3 steps 4 and 6, P1-20) |
| `frame-enum-unknown-term-kind-newer` |  | built and matches | a `terms` frame with `critical` true in a segment of `registry_version` 2 whose `runs` hold a `term_kind` a reader of version 1 does not hold, its link broken: the content of a `broken` frame is not interpreted, so `chain_break` alone, no `unsupported_critical` and no `malformed_value` (P1 3.3, P3-74) |
| `frame-unknown-transparent-not-bool` |  | built and matches | a frame of a type the reader does not know whose `transparent` is not a `bool`: only its `malformed_value` `wrong_type`, no `unknown_enumeration_value`, no `unsupported_critical`, and the fold does not stop (P1 3.3) |
| `framing-test-content-value` |  | built and matches | `H T B` with `B` a `blob` frame whose `media_type` is the option tag `0` and whose payload holds a resync item at its true offset: the tag is a rule of the part, not of the shape the framing test judges, so `B` passes the test, framing is not lost and the occurrence is not scanned; `B` `linked` with `malformed_value` `unregistered_value` (the step CONTENT) and opaque with `malformed`; one segment (P1 6.7, P1-100) |
| `head-longer-map-or-tag` |  | built and matches | a map or a tag written with a longer head than its argument needs is `wrong_type`, not `not_deterministic`: in a frame's `identifier` and inside a mark of a resync item (P1 1.4) |
| `header-previous-head-unheld-algorithm` |  | built and matches | a second header of `registry_version` 2 and `hash_algorithm` 99, which the reader does not hold, whose `lineage` is `present` with a `previous_head` of 8 bytes and an `ordinal` of 0: for this reader `previous_head` is `bytes` of any length, so the first violation in the order of the bytes is the `ordinal`: `malformed_value` `out_of_range`, and the header is `unreadable` (P1 2.4 HEADER GRAMMAR CHECKS, P1-23) |
| `item-limit-region-candidate-untested` |  | built and matches | `items_per_read` 5: a first item that is an unreadable opener (0/234) whose run holds a resync item with a true offset at 206, then three items, then bytes that cannot be delimited from 271 to the end: the walk to the decision point counts the region as one item, so the count reaches the limit at the region and the candidate is not tested; two segments, `undelimitable_input` 271/198, no `torn_tail` and no `exceeds_capacity` with `item_count`, since no byte remains (P1 6.6, 6.7 THE ORDER, P1-80) |
| `item-limit-unreadable-first-opener` |  | built and matches | the first item, a complete opener that is unreadable and holds no candidate, counts one item toward `items_per_read` (P7 2, THE ONE COUNT; P1-80) |
| `lineage-begins-links-unavailable` |  | built and matches | a later segment whose header outcome is `links_unavailable` and whose `lineage` is `absent`: `unsupported_registry_version` and `lineage_begins` (P1 2.4, P1-24) |
| `lineage-head-broken-last-frame` |  | built and matches | the head of a segment whose last ordinary frame is `broken` with a damaged stored identifier is that stored identifier: a later `previous_head` written before the damage is `lineage_mismatch` with `head` (P1 3.4 HEAD, 5.2 step 5) |
| `links-unavailable-lineage-not-compared` |  | built and matches | two segments, the second opened by a header of `registry_version` `2` and a `hash_algorithm` the reader does not hold, whose `lineage` is `present` and names a head that is not the first segment's: the opener is `links_unavailable`, its lineage is compared with nothing: `unsupported_registry_version` alone, no `lineage_mismatch`, no `lineage_begins`, its lineage not checked (P1 2.4, P1-24) |
| `opener-option-tag-first` |  | built and matches | an opener whose `lineage` option tag names no alternative and whose later bytes break another rule (a head not in its shortest form; `null`): the tag is the first violation in the order of the bytes, `unregistered_value` (P1-9, P1-23) |
| `parity-later-version-after-stop` |  | built and matches | a segment of `registry_version` `2`, reader version `1`, `H T Q X P`: `X` a critical frame of a type the reader does not hold (the fold stops), `P` a parity frame whose `geometry` is of a later version, linked: the stop is no condition of the later-version case: `P` opaque with `stopped`, no finding of its value, no `parity_damaged`, not used, outside the tail (P1 3.1 THE EXCEPTION OF PARITY FRAMES clause (ii), P1-38, P7 1.3) |
| `parity-later-version-payload-damaged` |  | built and matches | a segment of `registry_version` `2`, reader version `1`, `H T Q P` with `P` a parity frame whose `geometry` is of a later version, linked, and whose stored payload differs from its `payload_digest` in one bit: the payload state is no condition of the later-version case: `P` has `damaged_frame` `payload_digest`, no `unknown_enumeration_value` (its payload has not verified), no `parity_damaged`, opaque with `damaged`, not used (P1 3.1 THE EXCEPTION OF PARITY FRAMES clause (ii), P1-38, P7 the step CONTENT) |
| `resync-candidate-hash-later-version` |  | built and matches | `H T B` with `B` a `blob` frame whose element 10 is the `uint` `2` (framing is lost at `B`) and whose payload holds, at its true offset, a resync item of `registry_version` `2` and `hash_algorithm` `2`, outside registry version `1`: the occurrence fails at part 2 of the candidate test whatever its `registry_version`, before its place and before any charge: no candidate, one segment, `B` `malformed_value` `wrong_type` (P1 6.7 THE CANDIDATE TEST, P1-101) |
| `resync-candidate-option-rule` |  | built and matches | an occurrence of the pattern with a true offset whose `profile` or `lineage` breaks its row of 2.4a fails part 2 of the candidate test: it is not charged and gives no finding; the opener's own occurrence included (P1-101) |
| `resync-candidate-untested-at-limit` |  | built and matches | the count reaches `items_per_read` at or before the decision point of a run whose candidate would be accepted: the candidate is not tested, one segment, no torn_tail, and no item_count when no byte remains (P1 6.7 step 4) |
| `resync-charge-kept-first-item-lost` |  | built and matches | a resync candidate accepted in a first item that is incomplete keeps its charge `resync_max` in the total decoded length, so a later candidate whose charge passes the total is refused (P1 6.7 part 4) |
| `resync-incomplete-item-not-counted` |  | built and matches | an incomplete item that holds a resync candidate is not counted toward `items_per_read`: the candidate is tested and accepted as the item that reaches the limit (P1 6.1, P1-80) |
| `resync-occurrence-tested-once` |  | built and matches | a first item that is an unreadable header of 389 bytes holding an occurrence with a true offset at 209 whose item fails part 5, then 475 one-byte items, and bytes that cannot be delimited from 899 holding a second occurrence at 900: the occurrence at 209 is charged by the run of the first item and is not tested again by the next run, which covers it; the second occurrence is refused, 4096 + 4096 > 4626, with the position of the region; one finding (P1 6.7 WHAT IS SCANNED, part 4, WHAT A HOSTILE FILE COSTS, P1-104) |
| `resync-previous-head-unheld-algorithm` |  | built and matches | `H T Q R`: `R` a resync item at its true offset, reached by framing, of `registry_version` 2 and `hash_algorithm` 99, which the reader does not hold, whose `lineage` is `present` with a `previous_head` of 8 bytes and an `ordinal` of 0: `lineage` is judged as the header's row, so for this reader `previous_head` is `bytes` of any length and no `wrong_length` is made; the first violation in the order of the bytes is the `ordinal`: `malformed_value` `out_of_range`, and the item is `unreadable` (P1 2.4a THE RULES OF THE FIELDS, P1-23) |
| `resync-refused-charge-once-two-runs` |  | built and matches | an occurrence with a true offset refused at part 4 in the run of the first item at which framing is lost, then covered again by the run of the next one (an incomplete item whose item before it holds the occurrence): it is not tested again and the finding `exceeds_capacity` with `total_decoded` is given once, in the findings of the segment (P1 6.7 part 4, WHAT A HOSTILE FILE COSTS, P1-104) |
| `resync-refused-charge-once` |  | built and matches | a charge that the total refuses gives one `exceeds_capacity` with `total_decoded`, at the position of the item in which the occurrence begins, when a later run holds the same occurrence again: in a readable segment and in an unreadable one (P1-104) |
| `resync-refused-in-first-opener-once` |  | built and matches | a first item that is an unreadable header of 716 bytes holding an occurrence with a true offset at 700, then an incomplete item: the total of 600 refuses the charge in the run of the first item; the file has a segment, so the one finding stands in the segment's findings with `position` 0 and not in the findings of the file, and the next run, which covers the occurrence again, gives none (P1 6.7 part 4, P2-82, P1-104) |
| `resync-refused-position-where-begins` |  | built and matches | an unreadable header, a frame at position 1 (11..716) holding an occurrence with a true offset at 700, then an incomplete item: the run of the incomplete item holds the occurrence, the total refuses its charge, and the finding has the `position` of the item in which the occurrence begins (1), not that of the item whose run is scanned (P1 6.7 part 4, P2-82) |
| `resync-total-refused-no-occurrence-tested` |  | built and matches | a header, a frame whose `decoded_length` 698 the total of 600 refuses and which fails the framing test, holding an occurrence with a true offset at 700, then an incomplete item whose run covers the occurrence again: once the total is refused no occurrence is tested and none gives a finding, in the run of the frame or in the next; the one `exceeds_capacity` is the frame's, `count` 698 (P1 6.7 part 4, P1-104) |
| `undelimitable-chunk-wrong-major-type-cut` |  | built and matches | an indefinite-length byte string whose chunk head is of another major type and whose argument bytes pass the end of the input is `undelimitable_input`, not `torn_tail` (P1 6.1, 6.3 item 4) |

Totals: built and matches: 466, could not be built: 1 (467 entries).
