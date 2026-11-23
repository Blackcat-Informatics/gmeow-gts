# Vectors of P2

| vector | outcome | notes |
|---|---|---|
| `registry/tables` | built and matches | the registry written as data (the input file, 28306 bytes): every enumeration of section 4 (64 tables), the severity of the 48 codes, the 92 reasons, the 15 contexts with the digest of 61 62 63, the reasons each code allows, the fields of 5.2 and the assignment log of 9a; the result records the number of rows compared with the tables of P2 and the values of registry version 2 listed apart |
| `finding/empty_input` | built and matches |  |
| `finding/not_this_format` | built and matches |  |
| `finding/unsupported_generation` | built and matches |  |
| `finding/unsupported_registry_version` | built and matches |  |
| `finding/unknown_enumeration_value/unknown_codec` | built and matches | a frame with codec 2 (the value in the subject), in a segment of registry version 2 read by a reader of version 1; P2-56 gives the one unsupported_registry_version beside it |
| `finding/unknown_enumeration_value/unknown_frame_type` | built and matches | a non-critical frame of type 99 in a segment of version 2, reader version 1 (P2-58) |
| `finding/unsupported_critical` | built and matches | a critical frame of type 99 in a segment of version 2, then two frames (opaque, stopped; P2-59) |
| `finding/malformed_value/not_deterministic` | built and matches | the uint 5 written as 18 05 in the decoded_length of a frame |
| `finding/malformed_value/wrong_type` | built and matches | a text string where the uint decoded_length stands |
| `finding/malformed_value/wrong_length` | built and matches | a blob frame whose content record holds three fields where it has two |
| `finding/malformed_value/out_of_range` | built and matches | a quads frame whose object column is the single delta -1: it decodes below 0 (input of the P3 vector rows-value-negative) |
| `finding/malformed_value/unregistered_value` | built and matches | a terms frame whose only block has the tag 8, in a segment of version 1 read by a reader of version 1 |
| `finding/malformed_value/unordered_or_repeated` | built and matches | two annotations rows equal but for count (input of the P3 vector annotations-repeat) |
| `finding/malformed_value/unordered_or_repeated/signatures` | built and matches | two inputs, each one finding: a frame whose signatures list holds two records in descending bytewise order of their encodings, and a frame whose list holds one record twice |
| `finding/malformed_value/inconsistent_counts` | built and matches | a recipe whose entry_count 2 differs from its list of 3 entries (input of the P4 vector recipe-list-length) |
| `finding/malformed_value/forbidden_combination` | built and matches | a terms frame written with critical false (the column of 4.2 is yes) |
| `finding/malformed_value/codec_constraint` | built and matches | a zstd payload whose frame header has the checksum flag set (the header differs from the one that decoded_length generates, P2-43) |
| `finding/undelimitable_input` | built and matches | bytes ff ff after the complete header |
| `finding/torn_tail` | built and matches | a final item cut after its first byte (8b, the head of an array of 11) |
| `finding/read_failed` | built and matches | a source whose read of 16 bytes at offset 10 fails (the 10 bytes before it were read) |
| `finding/chain_break` | built and matches | the first frame's envelope altered after its identifier was computed (its content), the next frame unchanged: the identifier is neither the computed one nor explained by the next link |
| `finding/healed_link` | built and matches | the first frame's identifier field altered, the next frame linking to the identifier that the first frame's envelope computes |
| `finding/damaged_frame/payload_digest` | built and matches | one payload byte altered |
| `finding/damaged_frame/decoded_length` | built and matches | decoded_length 4 where the payload has 3 bytes |
| `finding/damaged_frame/blob_length` | built and matches | a recipe whose entry of 7 bytes names a chunk of 6: the rebuilt blob has another length (input of the P4 vector rebuild-blob-length) |
| `finding/damaged_frame/blob_digest` | built and matches | a recipe whose rebuilt blob has another digest than its name (input of the P4 vector rebuild-blob-digest) |
| `finding/damaged_frame/decode_failed` | built and matches | a zstd payload with the generated header whose blocks do not decode |
| `finding/stored_check_mismatch` | built and matches | one stored byte of a compressed frame altered (a literal byte of a raw block): the stored check disagrees, and the decoded payload then fails its digest (P1: the two checks are independent) |
| `finding/commit_count_mismatch` | built and matches | a commit with count 3 after two ordinary frames; the file then has no committed prefix, so P5 2.3 also reports the uncommitted tail |
| `finding/commit_root_mismatch` | built and matches | a commit with an altered root |
| `finding/uncommitted_tail` | built and matches | three ordinary frames after the last commit |
| `finding/index_claim_failed/index_head` | built and matches | an index whose head is altered |
| `finding/index_claim_failed/index_root` | built and matches | an index whose range_root is altered |
| `finding/index_claim_failed/index_offsets` | built and matches | an offsets item with the length of the first frame one too high |
| `finding/index_claim_failed/index_digest` | built and matches | an index whose content_digest is altered |
| `finding/index_claim_failed/index_members` | built and matches | an index with one pack_members entry where the segment has no pack |
| `finding/index_claim_failed/index_directory` | built and matches | an index whose graph directory has the last number of its last range one too high |
| `finding/index_claim_failed/index_locator` | built and matches | a locator whose distance leads to the first byte of a frame that is not the index |
| `finding/missing_dependency/chunk` | built and matches | a recipe naming a unit that no frame holds (input of the P4 vector rebuild-missing) |
| `finding/missing_dependency/value` | built and matches | two inputs, each one finding at the terms frame: a literal_ref term with no value frame, and a literal_value_ref term with none (P3 2.8 item 5) |
| `finding/missing_dependency/blob` | built and matches | a binding to a blob that nothing declares (input of the P3 vector bindings-basic, case undeclared) |
| `finding/missing_dependency/frame` | built and matches | a binding to a frame identifier that no frame of the file has |
| `finding/missing_dependency/state` | built and matches | a binding to a commit identifier that no commit of the file has |
| `finding/exceeds_capacity/decoded_size` | built and matches | a frame of decoded_length 20 read with a capacity of 10 decoded bytes for a frame (a capacity under the minimum of P7-8, as a test device) |
| `finding/exceeds_capacity/stored_size` | derived from the text | a frame whose stored payload of 20 bytes and a commit frame whose content element of 36 bytes each exceed a capacity of 10 stored bytes for a frame |
| `finding/exceeds_capacity/total_decoded` | built and matches | frames of decoded length 4, 4 and 1 with decoded_bytes_per_read 8: the third brings the total to 9; no commit (a commit after the refusal is refused too, its length 0 on a total of 8 judged after the refusal) |
| `finding/exceeds_capacity/item_count` | built and matches | the header and three frames read with items_per_read 3: a byte remains after the second frame (the third item is the last allowed); the two frames read have no commit |
| `finding/declared_order_violated` | built and matches | a quads frame declaring canonical with two rows swapped (input as the P3 vector rows-order-canonical) |
| `finding/forward_reference` | built and matches | a row naming the term id 99 that no frame has yet defined (input of the P3 vector rows-forward-reference) |
| `finding/position_constraint` | built and matches | a quads row whose predicate is a literal; the row is dropped |
| `finding/ill_formed_term` | built and matches | a front-coded IRI whose suffix bytes ff are not valid UTF-8 |
| `finding/opaque_term` | built and matches | a quads row naming a term of a terms frame whose payload is damaged: the frame is opaque, its terms are unreadable (P3 2.8) |
| `finding/labelling_refused/refusal_width` | built and matches | annotation counts that sum above 2^64 - 1 (input of the P3 vector labelling-refusal-width); P3-39 allows one labelling_refused for each fold, and the segment's fold and the file's fold are two folds: one stands in the segment's list, one in the file's |
| `finding/labelling_refused/refusal_limit` | built and matches | the Paley graph on 13 blank nodes (not guaranteed) read with labelling_units 0, the default limit; one in the segment's list and one in the file's, as for refusal_width |
| `finding/labelling_refused/refusal_size` | built and matches | three rows read with labelled_lines 2 (input of the P3 vector labelling-size-guaranteed); one in the segment's list and one in the file's |
| `finding/canonical_violated/header` | built and matches | a claimed canonical file whose header declares registry_version 2 though it uses nothing of version 2, read by a reader of version 2 (P2-15); the header differs from the canonical header |
| `finding/canonical_violated/frame_sequence` | built and matches | a claimed canonical file with its last item removed |
| `finding/canonical_violated/terms` | built and matches | a claimed canonical file whose terms frame declares the order arrival (2) in place of canonical (1), with the same terms in the same order: the content record of the frame differs from the canonical frame |
| `finding/canonical_violated/rows` | built and matches | a claimed canonical file whose quads frame declares the row order unordered (2) in place of canonical (1), with the same rows: the content record differs |
| `finding/canonical_violated/value_region` | built and matches | a value region in the wrong order |
| `finding/canonical_violated/blob_region` | built and matches | the members of the one pack in the reverse order |
| `finding/canonical_violated/bindings` | built and matches | a bindings frame with its two entries swapped |
| `finding/canonical_violated/suppression` | built and matches | a suppress frame with its two entries swapped |
| `finding/canonical_violated/commit` | built and matches | a commit with count one too high: it differs from the canonical commit; as a commit it is invalid (P5 2.2), and the frames before it are an uncommitted tail (P5 2.3) |
| `finding/canonical_violated/index` | built and matches | an index whose content_digest is absent: the payload differs from the canonical index; the segment's own digest is present, so the claim `absent` is false (P5 4.3) |
| `finding/canonical_violated/labelling` | built and matches | blank labels that name no labelling |
| `finding/canonical_violated/content_digest` | built and matches | an index whose content_digest is 32 zero bytes: it differs from the canonical digest, and as a claim it is false (P5 4.3) |
| `finding/canonical_violated/tie` | built and matches | two differences at one item position: the index frame differs from the canonical index (check index) and its content_digest, absent, differs from the digest of the canonical line sequence (check content_digest); the reason of the smaller check code, index |
| `finding/suppression_unmatched/term` | built and matches | an entry of kind term that matches nothing (input of the P3 vector suppress-unmatched-term) |
| `finding/suppression_unmatched/quad` | built and matches | an entry of kind quad that matches nothing (input of the P3 vector suppress-unmatched-quad) |
| `finding/suppression_unmatched/reifier` | built and matches | an entry of kind reifier that matches nothing (input of the P3 vector suppress-unmatched-reifier) |
| `finding/suppression_unmatched/graph` | built and matches | an entry of kind graph that matches nothing (input of the P3 vector suppress-unmatched-graph) |
| `finding/suppression_unmatched/blob` | built and matches | an entry of kind blob that matches nothing (input of the P3 vector suppress-unmatched-blob) |
| `finding/suppression_unmatched/reifier_binding` | built and matches | an entry of kind reifier_binding that matches nothing (input of the P3 vector suppress-unmatched-reifier-binding) |
| `finding/suppression_unmatched/annotation` | built and matches | an entry of kind annotation that matches nothing (input of the P3 vector suppress-unmatched-annotation) |
| `finding/suppression_unmatched/binding` | built and matches | an entry of kind binding that matches nothing (input of the P3 vector suppress-unmatched-binding) |
| `finding/binding_unanchored` | built and matches | a binding whose leaf is a pair that names no node; the binding stays in the fold (input of the P3 vector bindings-unnamed-pair) |
| `finding/parity_damaged` | built and matches | a parity frame with one byte of the table_digest field of its content altered: parity_damaged and no other finding of P1 |
| `finding/parity_mismatch/data` | built and matches | one data shard (shard 2, inside the blob frame) inverted, its digest in the parity frame unchanged: the frame fails its payload digest, the shard is an erasure that the recovery solves (P5 7.2) |
| `finding/parity_mismatch/parity` | built and matches | one parity shard altered inside a parity frame whose payload digest is consistent with the altered bytes, its digest in the table unchanged |
| `finding/repaired` | built and matches | one shard zeroed in a stripe: the repair completes and is accepted |
| `finding/repair_failed/too_many_erasures` | built and matches | five shards of the one stripe zeroed: more erasures than the four parity shards of the stripe |
| `finding/repair_failed/verify_failed` | built and matches | the parity table covers a frame whose payload digest was already wrong; one byte of it is changed: the solved shard restores that frame and it still fails its check |
| `finding/parity_stale` | built and matches | the first frame rewritten in storage at equal length (its first four bytes as one RLE block), the parity frame not rebuilt: the one failing shard lies in items that pass (P5 6.6, P5-32) |
| `finding/parity_unexplained` | built and matches | in one stripe, one shard of a damaged compressed frame (explained) and one shard of a frame rewritten at equal length (every item it overlaps passes): the second is unexplained, solved as an unknown and never written |
| `finding/unprotected_tail` | built and matches | a blob frame, its parity frame, then a second blob frame and a commit that no parity span covers |
| `finding/proof_malformed/count` | built and matches | an inclusion proof for a file of 5 frames checked against a root of 8 |
| `finding/proof_malformed/length` | built and matches | an inclusion proof whose path has two digests where the shape of 5 frames needs three |
| `finding/proof_malformed/order` | built and matches | a range proof with first 4 above end 3 |
| `finding/proof_failed/inclusion` | built and matches | an inclusion proof whose two first path digests are swapped, against the right root |
| `finding/proof_failed/consistency_old_root` | built and matches | a consistency proof checked against an old root that differs |
| `finding/proof_failed/consistency_new_root` | built and matches | a consistency proof checked against a new root that differs |
| `finding/proof_failed/consistency_diverged` | built and matches | two roots of the same count 5 that differ, with the proof of equality |
| `finding/proof_failed/range` | built and matches | a range proof for the frames 2 and 3 given with an altered identifier for the second |
| `finding/proof_holds/inclusion` | built and matches | a correct inclusion proof |
| `finding/proof_holds/consistency` | built and matches | a correct consistency proof from 5 to 8 frames |
| `finding/proof_holds/range` | built and matches | a correct range proof |
| `finding/signature_invalid/key_mismatch` | built and matches | a commit with one signature, checked by a verifier that holds the key: a key_id of an ed25519 key written under the algorithm 2: it is not the digest of the key held for it |
| `finding/signature_invalid/value_length` | built and matches | a commit with one signature, checked by a verifier that holds the key: an ed25519 value of 63 bytes |
| `finding/signature_invalid/non_canonical_scalar` | built and matches | a commit with one signature, checked by a verifier that holds the key: an ed25519 value whose scalar S equals the group order L |
| `finding/signature_invalid/small_order_key` | built and matches | a commit with one signature, checked by a verifier that holds the key: a held key of small order |
| `finding/signature_invalid/small_order_nonce` | built and matches | a commit with one signature, checked by a verifier that holds the key: a nonce point R of small order |
| `finding/signature_invalid/equation_failed` | built and matches | a commit with one signature, checked by a verifier that holds the key: an ed25519 value with S = L - 1, which fails the equation |
| `finding/signature_invalid/composite_classical_failed` | built and matches | a commit with one signature, checked by a verifier that holds the key: a composite value whose ed25519 half is altered in R |
| `finding/signature_invalid/composite_post_quantum_failed` | built and matches | a commit with one signature, checked by a verifier that holds the key: a composite value whose ML-DSA half is altered |
| `finding/signature_unverified/unknown_signature_algorithm` | built and matches | a signature of algorithm 3 in a segment of registry version 2, read by a reader of version 1 (P2-61) |
| `finding/signature_unverified/key_not_held` | built and matches | a valid signature whose key the verifier does not hold |
| `finding/lineage_begins` | built and matches | a second segment whose header has lineage absent |
| `finding/lineage_mismatch/head` | built and matches | a second segment whose lineage names a previous_head that is not the head of the first |
| `finding/lineage_mismatch/ordinal` | built and matches | a third segment, after a second whose lineage names the first and the ordinal 1, whose lineage names the head of the second and the ordinal 7 in place of 2 |
| `finding/lineage_mismatch/ordinal/first` | built and matches | a first segment whose lineage is present (P2-42) |
| `finding/unreferenced_unit` | built and matches | a chunk that no recipe, blob or value names |
| `finding/conflicting_declaration` | built and matches | the name of a blob declared by a blob frame of length 5 and by an external frame of length 9: one finding, at the frame of length 9 |
| `finding/profile_not_checked` | built and matches | a header with profile present(99): the segment is read in full, with no profile check |
| `finding/undelimitable_input/one_byte` | built and matches | an input of the one byte 1c |
| `finding/torn_tail/no_item` | built and matches | an input of the one byte 82 (the head of an array of 2) |
| `finding/conversion_stopped/source_damaged` | built and matches | a source whose index frame has a wrong mmr: an item failing its own first-generation check (input of the P9 vector convert/index-mmr-wrong) |
| `finding/conversion_stopped/source_not_decodable` | built and matches | a source frame whose codec is outside the baseline of the first-generation format (input of the P9 vector convert/unknown-codec) |
| `finding/conversion_stopped/unrepresentable_term` | built and matches | a source quads row whose subject is a literal: a generalized term (input of the P9 vector convert/literal-subject) |
| `finding/conversion_stopped/unrepresentable_frame` | built and matches | a source frame of type zzz with no image in this generation (input of the P9 vector convert/unknown-frame-type) |
| `finding/conversion_stopped/source_not_deterministic` | built and matches | a source presented as one file on the first read and as another on the second (the converter reads twice, P9) |
| `finding/conversion_stopped/source_exceeds_capacity` | built and matches | a source whose fold has the size of its labelled set above the converter's labelled_lines (a capacity under the minimum, as a test device; input of the P9 vector convert/capacity-size) |
| `finding/conversion_stopped/content_mismatch` | built and matches | a first-generation source of 1023 bytes and the finding conversion_stopped with the reason content_mismatch: the second digest of step 5 differs from the first |
| `finding/unprotected_tail/canonical` | built and matches | negative vector (P2-37): a canonical file whose offsets item and locator stand outside every parity span gives no unprotected_tail |
| `finding/suppression_unmatched/canonical` | built and matches | negative vector: a canonical file whose suppression entries all match gives no finding |
| `registry/zero/item_kind` | built and matches | the code 0 in the item_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/option_tag` | built and matches | the code 0 in the option_tag field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/storage_kind` | built and matches | the code 0 in the storage_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/codec` | built and matches | the code 0 in the codec field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/hash_algorithm` | built and matches | the code 0 in the hash_algorithm field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/signature_algorithm` | built and matches | the code 0 in the signature_algorithm field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/frame_type` | built and matches | the code 0 in the frame_type field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/canonical_form` | built and matches | the code 0 in the canonical_form field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/profile` | built and matches | the code 0 in the profile field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/parity_geometry` | built and matches | the code 0 in the parity_geometry field: parity_damaged (damage of a parity frame, P1 3.1 and P2-67); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/term_order` | built and matches | the code 0 in the term_order field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/row_order` | built and matches | the code 0 in the row_order field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/term_kind` | built and matches | the code 0 in the term_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/literal_form` | built and matches | the code 0 in the literal_form field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/value_kind` | built and matches | the code 0 in the value_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/graph_name_kind` | built and matches | the code 0 in the graph_name_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/referent_kind` | built and matches | the code 0 in the referent_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/member_kind` | built and matches | the code 0 in the member_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/leaf_kind` | built and matches | the code 0 in the leaf_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/suppression_kind` | built and matches | the code 0 in the suppression_kind field: malformed_value, unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/option` | built and matches | the variant option with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/storage_form` | built and matches | the variant storage_form with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/block` | built and matches | the variant block with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/typed_value` | built and matches | the variant typed_value with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/graph_name` | built and matches | the variant graph_name with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/referent` | built and matches | the variant referent with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/member` | built and matches | the variant member with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/suppression_entry` | built and matches | the variant suppression_entry with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/zero/tag/leaf` | built and matches | the variant leaf with the tag 0: unregistered_value, whatever h and r (P2-63); two inputs, h = r = 1 and h = 2, r = 1 |
| `registry/unknown/item_kind` | built and matches | the code 6, one above the highest of item_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/option_tag` | built and matches | the code 3, one above the highest of option_tag, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/storage_kind` | built and matches | the code 3, one above the highest of storage_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/codec` | built and matches | the code 2, one above the highest of codec, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/hash_algorithm` | built and matches | the code 2, one above the highest of hash_algorithm, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/signature_algorithm` | built and matches | the code 3, one above the highest of signature_algorithm, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/frame_type` | built and matches | the code 17, one above the highest of frame_type, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/canonical_form` | built and matches | the code 2, one above the highest of canonical_form, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/parity_geometry` | built and matches | the code 2, one above the highest of parity_geometry, with h = r = 1: parity_damaged (P1 2.7, P2-67) |
| `registry/unknown/term_order` | built and matches | the code 3, one above the highest of term_order, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/row_order` | built and matches | the code 3, one above the highest of row_order, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/term_kind` | built and matches | the code 8, one above the highest of term_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/literal_form` | built and matches | the code 5, one above the highest of literal_form, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/value_kind` | built and matches | the code 6, one above the highest of value_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/graph_name_kind` | built and matches | the code 3, one above the highest of graph_name_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/referent_kind` | built and matches | the code 4, one above the highest of referent_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/member_kind` | built and matches | the code 5, one above the highest of member_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/leaf_kind` | built and matches | the code 4, one above the highest of leaf_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/suppression_kind` | built and matches | the code 9, one above the highest of suppression_kind, with h = r = 1: unregistered_value (P2-57) |
| `registry/unknown/profile` | built and matches | the profile code 7, one above the highest: not unregistered_value, because P2-57 excepts the header's profile and P2-21 judges it: profile_not_checked with subject [4, 7, absent] and the segment read in full (question Q158) |
| `registry/term_kind/values` | built and matches | P2 4.3: a terms frame with one run of each term_kind 1 to 7: no finding; the result records the registered kinds and the code 8 of an unregistered block tag (malformed_value, unregistered_value) |
| `registry/literal_form/values` | built and matches | P2 4.3: a literal block whose forms are 1, 2, 3, 4 (typed, language, language_ltr, language_rtl): no finding; a forms element 5: malformed_value, unregistered_value |
| `registry/value_kind/values` | built and matches | P2 4.3: a terms frame with one literal_value run of five values, one of each value_kind 1 to 5 (bool, uint, int, time, octets): no finding; the result records the code 6 of an unregistered typed_value tag (malformed_value, unregistered_value) |
| `registry/version/zero` | built and matches | a header with registry_version 0: malformed_value, out_of_range (P2-5) |
| `registry/version/above` | built and matches | P2 7: h 3, r 1, a frame of a type above 16 (17): unsupported_registry_version once, then P2-56: the link is checked, the frame is opaque with unknown_frame_type and reported unknown_enumeration_value |
| `registry/version/hash_equal` | built and matches | h 1, r 1, hash_algorithm 2: unregistered_value; the header is unreadable (P2-57) |
| `registry/version/hash_above` | built and matches | P2 7: h 3, r 1, hash_algorithm 2: unsupported_registry_version, no link checked and nothing folded |
| `registry/profile/not_checked` | built and matches | a header with profile present(5) read by a reader that implements no profile: the segment is read in full and profile_not_checked has the subject [4, 5, absent] |
| `registry/version/smallest` | built and matches | r 2, a canonical file that declares registry_version 2 and uses nothing of version 2: canonical_violated, header (P2-15) |
| `registry/version/forbidden` | built and matches | P2 7: h 1, r 2, a frame of type 17, a type above 16 that r holds: malformed_value, forbidden_combination; the frame is opaque with malformed |
| `registry/frame_type/columns` | built and matches | P2 4.2: the result records, for each type of 4.2 (1 to 16), the findings of a frame with the critical and transparent columns of the registry (no forbidden_combination) and of a frame with its critical flag flipped (forbidden_combination, and for parity (16) parity_damaged and no other finding of P1); the input file is a header and one frame of type 1 |
| `registry/frame_type/critical_unknown` | built and matches | h 2, r 1, a critical frame of type 99, then three frames: unsupported_critical, the fold of the segment stops |
| `registry/frame_type/ancillary_unknown` | built and matches | P2 7: h 2, r 1, a non-critical frame of type 99, then three: the frame is opaque with unknown_frame_type and the three after it fold |
| `registry/frame_type/unknown_equal` | built and matches | P2 7: h 1, r 1, a frame of type 99: malformed_value, unregistered_value; the frame is opaque with malformed |
| `registry/codec/unknown` | built and matches | P2 7: h 2, r 1, a frame with codec 2: payload unchecked with unknown_codec; the frame is opaque with unknown_codec |
| `registry/storage/unknown_tag` | built and matches | P2 7: h 2, r 1, a storage tag 3: as an unknown codec, with the subject [1, 3] |
| `registry/item_kind/unknown` | built and matches | P2 7: h 2, r 1, an item of kind 9 between two frames: opaque, counted (the frame after it is at position 3), unknown_item_kind with the subject [5, 9] |
| `registry/signature/unknown` | built and matches | h 2, r 1, a signature of algorithm 3: unverified (status unverified, reason unknown_signature_algorithm), signature_unverified, and no unknown_enumeration_value |
| `registry/content/unknown_value` | built and matches | P2 7: h 2, r 1, a terms frame whose order is 3: the frame is a frame of unknown type: unsupported_critical (the type is critical), the fold stops |
| `registry/profile/unknown_code` | built and matches | a header with profile present(99): profile_not_checked with the subject [4, 99, absent] and no error |
| `registry/parity/signed` | built and matches | P2 9: a parity frame with one signature: parity_damaged and no other finding of P1; no signature result |
| `registry/integrity/not_checked` | built and matches | P2 9: h 3, r 1, hash_algorithm 2: integrity not_checked, not damaged (P2-78) |
| `registry/authenticity/index` | built and matches | P2 9: a canonical file whose commit carries a valid trusted signature: authenticated, the closing index not counted (P2-77) |
| `registry/parity/damaged` | built and matches | P2 7: a parity frame with one byte of the table_digest altered: parity_damaged and no other finding of P1; the integrity is that of the file without the damage |
| `registry/position/transparent` | built and matches | P2 5.2: a parity frame between two frames: the later positions count it, and the ordinary_position of the index's pack_members does not |
| `registry/opaque/precedence` | built and matches | P2 4.7: a frame that is malformed and stands after a stop: the opaque reason is malformed (P2-30) |
| `registry/algorithms/lengths` | built and matches | signatures of both algorithms one byte short and one byte long (63 and 65 for ed25519, 3372 and 3374 for the composite): signature_invalid, value_length |
| `registry/fields/all` | built and matches | P2 5.2: the result records, for each of the 48 finding codes, the fields that its seven-field record has present (exactly those that the table of 5.2 marks); the input file is a header, two frames and a commit |
| `registry/item_kind/resync` | built and matches | P2 4.1: an item of kind 5 of the generation 2, read by a reader of registry version 1: a resync item, an opener that opens a segment |
| `registry/reader_advisory_mark/values` | built and matches | P2 4.5: a resync item with one mark of each tag 1 to 3: all applicable; a mark of tag 4: not applicable, counted in marks_ignored, no finding |
| `registry/mark_target_kind/values` | built and matches | P2 4.5: a mark with a target of tag 1 and one of tag 2: applicable; a target of tag 3: the mark is not applicable, no finding |
| `registry/capacity_field/values` | built and matches | P2 4.5: a resync item with one hostile mark whose strains is 10, which is no capacity_field: not applicable, no finding |
| `registry/opener_kind/values` | built and matches | P2 4.7: a file of one header segment and one segment that a resync item opens: opener [1] and [2, reached, displaced, past, marks_ignored] |
| `registry/resync_reach/values` | built and matches | P2 4.7: a resync item reached by framing: walked; one found by the scan after a damaged region: scanned |
| `registry/past_state/values` | built and matches | P2 4.7: a resync item with past absent, equal to the digest of the bytes before it, different from it, and a read that does not check it: absent, matches, differs, not_checked |
| `registry/file_end/values` | built and matches | P2 4.7: a header, a blob frame, a commit frame and the byte 1c: an undelimitable_input finding runs to the end of the input, file_end undelimitable (6) |
| `registry/example/1` | built and matches | P2 section 10, example 1: uncommitted_tail at position 5, count 3; the input is three frames, a commit at position 4 and three frames |
| `registry/example/2` | built and matches | P2 section 10, example 2: unknown_enumeration_value, reason unknown_codec, subject [1, 2], position 7 |
| `registry/example/3` | built and matches | P2 section 10, example 3: unsupported_registry_version, the six absent fields |
| `registry/example/4` | built and matches | P2 section 10, example 4: signature_unverified, key_not_held, subject [3, 1], position 3 |
| `registry/example/5` | built and matches | P2 section 10, example 5: a blob name declared at position 4 (length 5, a blob frame) and at position 9 (length 9, an external frame): one finding at position 9 |
| `registry/example/6` | built and matches | P2 section 10, example 6: the reasons that the example names, each with its code and the finding code it qualifies: codes 1 and 2 qualify 5, codes 3 to 11 qualify 7, codes 12 and 16 qualify 13, and code 53 qualifies 41 |
| `registry/example/7` | built and matches | P2 section 10, example 7: visibility [2, 1, 0, 0] of a read in which one suppress frame is not folded (input of the P3 vector suppress-not-folded) |
| `registry/example/8` | built and matches | P2 section 10, example 8: suppression_unmatched, reason quad (34), at the suppress frame at position 12 |
| `finding/missing_dependency/state_unavailable` | built and matches | a binding to a commit identifier whose every commit in the file names no state to the reader (the commit's root is false) |
| `finding/exceeds_capacity/total_rebuilt` | built and matches | a recipe of total_length 11 read with rebuilt_bytes_per_read 10 (a capacity under the minimum of P7-45, as a test device): the total rebuilt length of the read up to and including the declaration is 11 |
| `finding/exceeds_capacity/total_formed` | built and matches | a terms frame of charge 9 read with formed_bytes_per_read 8 (a capacity under the minimum of P7-47, as a test device): the total kept so far plus the charge is 9 |
| `finding/labelling_refused/refusal_weight` | built and matches | the worked example of P3 6.5 read with labelled_bytes_per_read 24895 (under the minimum of P7-48, as a test device): the fold of the file weighs 12448 and the total labelled weight kept so far is 12448 |
| `finding/unknown_enumeration_value/unknown_item_kind` | built and matches | an item of kind 9 between two frames in a segment of version 2, reader version 1: opaque, counted; P2-56 gives the one unsupported_registry_version beside it |
| `registry/profile/checks-reported` | built and matches | a reader that implements bundle at 5 checks reads bundle-pass: profiles [[4, 5]]; no profile_violation |
| `registry/profile/checks-prefix` | built and matches | a reader that runs 3 of 5 checks reads a file that fails check 5 only: profiles [[4, 3]]; no profile_violation (check 5 did not run); the control with 5 checks reports it |
| `registry/profile/not-in-list` | built and matches | a reader with profiles [] reads a segment declaring bundle: profile_not_checked, subject [4, 4, absent]; profiles [] |
| `registry/profile/order` | built and matches | a reader implementing files and bundle: profiles [[4, 5], [5, 11]], ascending by profile, whatever the order the reader lists them in |
| `registry/withdrawn/read` | built and matches | a withdrawn value of a published version in a file (no value is withdrawn in a published version: a test log withdraws zstd): decoded with its meaning, no finding (P2-8) |
| `registry/retired/signature-trust` | built and matches | read by a reader of registry version 2, whose table (P2 section 8) retires no algorithm: a valid ed25519 signature under a trusted key authenticates: status valid, coverage [0, 2), authenticated true, segment authenticity authenticated; a registry version that retires an algorithm is not defined, so no entry reads one |
| `registry/retired/signature-invalid` | built and matches | the same table, a signature that fails the equation: status invalid, reason equation_failed, as for an algorithm not retired |
| `registry/retired/hash-trust` | built and matches | read by a reader of registry version 2, whose table (P2 section 8) retires no hash algorithm: both valid signatures under trusted keys authenticate, coverage [0, 2), segment authenticity authenticated; a registry version that retires a hash algorithm is not defined, so no entry reads one |
| `registry/retired/reader-below` | built and matches | the file of registry/retired/signature-trust read by a reader of registry version 1, in whose table the algorithm is not retired: authenticated true; the two results differ in registry_version |

Totals: built and matches: 232; derived from the text: 1 (233 entries).
