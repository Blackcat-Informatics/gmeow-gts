# Vectors of P7

| vector | outcome | notes |
|---|---|---|
| `canonical/construction` | built and matches | 7.5: the bytes of the pack, terms content, quads, bindings and header payloads, the index fields, positions 7 to 9 and the read-back; the input is the canonical file |
| `read/minimal` | built and matches | 7.1: the 233 bytes of the result; 7.2: the 250 bytes of the torn tail (read/torn-tail) |
| `canonical/constants/terms-budget` | built and matches | the cut by Bt on the real constant, a term alone above Bt, a blank by label length |
| `canonical/constants/rows` | built and matches | Nr + 1 quads: two frames, the second of one row |
| `canonical/constants/threshold` | built and matches | lexical forms of 1048575 and 1048576 bytes, typed and language; octets values of 524287 and 524288 |
| `canonical/constants/value-form` | built and matches | the literal 7 of nonNegativeInteger with and without its pair of VALUES |
| `canonical/constants/storage-choice` | built and matches | 63 and 64 bytes and a frame that saves exactly 16 bytes |
| `canonical/check/verified-and-no-encoder` | built and matches | 7.4, the first two results |
| `canonical/check/first-difference/header` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/frame_sequence` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/terms` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/rows` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/blob_region` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/bindings` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/suppression` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/commit` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/index` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/labelling` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/first-difference/content_digest` | built and matches | the canonical result with [check, position] and one canonical_violated with that reason |
| `canonical/check/two-differences` | built and matches | the smaller position decides; two differences at one position (index and content_digest of the index frame): the smaller check code, index |
| `canonical/check/not-claimed` | built and matches | P7-25 |
| `canonical/check/capacity` | built and matches | all three levels unchecked(exceeds_capacity), no difference |
| `canonical/check/index-above-capacity` | built and matches | labelling and structure unchecked(exceeds_capacity) |
| `canonical/check/unchecked-order` | built and matches | not_claimed before exceeds_capacity before encoder_not_held |
| `canonical/check/parity-state` | built and matches | a parity frame with an altered table_digest, and with the parity frame removed |
| `canonical/check/refused` | built and matches | labelling unchecked(labelling_refused) |
| `canonical/check/storage-differs` | built and matches | a canonical file recompressed at level 3 |
| `canonical/check/signature-added` | built and matches | a signature added to the commit; parity, offsets and locator rewritten; no P6 verification (P6 is not built) |
| `canonical/check/lineage` | built and matches | check header at position 0 of the second segment |
| `canonical/hidden-kept` | built and matches | the canonical file holds the hidden line and the entry |
| `canonical/compaction/missing-unit` | built and matches | [2, missing_dependency, present(chunk)] |
| `canonical/compaction/damaged` | built and matches | [2, damaged_frame, present(blob_digest)] and blob_length |
| `canonical/compaction/two-declarations` | built and matches | a blob declared twice: one complete is written, both incomplete stops |
| `canonical/compaction/held-and-refused` | built and matches | [2, opaque_term, absent]; [2, labelling_refused, present(refusal_width)] |
| `canonical/compaction/state-referent` | built and matches | the file is written; its read reports missing_dependency with state |
| `profile/canonical-same` | built and matches | the same frames, equal content and payloads but commit root and index; headers, h0 and identifiers differ |
| `profile/not-checked` | built and matches | a profile not implemented, and a value outside the table |
| `conformance/claim` | built and matches | the implementation_claim record |
| `read/labelling/guaranteed-limit-zero` | built and matches | a guaranteed labelled set read with labelling_units 0: labelled |
| `read/labelling/limit` | built and matches | Paley 13 with labelling_units 0: labelling_refused with refusal_limit in the file's list |
| `canonical/check/first-difference/value_region` | built and matches | the value region out of order |
| `canonical/storage/1` | built and matches | the pack frame at position 1 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/2` | built and matches | the pack frame at position 2 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/3` | built and matches | the external frame at position 3 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/4` | built and matches | the terms frame at position 4 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/5` | built and matches | the quads frame at position 5 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/6` | built and matches | the reifier_bindings frame at position 6 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/7` | built and matches | the annotations frame at position 7 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/8` | built and matches | the empty_graphs frame at position 8 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/9` | built and matches | the bindings frame at position 9 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/10` | built and matches | the suppress frame at position 10 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/11` | built and matches | the commit frame at position 11 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/12` | built and matches | the index frame at position 12 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/13` | built and matches | the parity frame at position 13 of the canonical file of the storage fold: the decoded payload (`decoded`) and the stored bytes (`stored`) |
| `canonical/storage/2^26-repeated` | built and matches | a frame of decoded length 2^26 (an IRI of the byte 'a' repeated); the stored bytes are kept in the file when they are at most 20 MB |
| `canonical/storage/2^26-letters` | built and matches | a frame of decoded length 2^26 (an IRI of the letters A to Z drawn by random.Random(11).randbytes, each byte b a letter chr(65 + b % 26)); the stored bytes are kept in the file when they are at most 20 MB |
| `read/order/findings` | built and matches | a compressed frame with chain_break, stored_check_mismatch, damaged_frame (steps 5, 6, 7); a segment whose header is of registry version 2 and four items that are not frames: each list in the order of 1.4, checked against an oracle written from the table of P7 section 3 |
| `read/order/independent` | built and matches | the composite input read twice, and every list with several findings recomputed from every order of its members by the oracle of the order: equal lists (one engine; the second computation is the oracle's) |
| `read/placement` | built and matches | one finding of each class of 1.4: a frame's, a non-frame item's, a segment's (unsupported_registry_version) and the file's (torn_tail) |
| `read/fold-precedence` | built and matches | frames to which several opaque reasons apply carry the first of P2-30: malformed, chain_break, stopped, damaged, unsupported_critical, unknown_codec, exceeds_capacity, unknown_frame_type (ten files) |
| `read/content-digest/segment-alone` | built and matches | a segment whose entry names a node of an earlier segment by a pair: the segment digest is that of the segment alone (the pair unnamed there); the file digest names the node |
| `read/content-digest/value-elsewhere` | built and matches | a literal_ref term of segment 1 whose value is in segment 0: segment digest absent, no finding for the own fold, file digest present |
| `read/authenticity/index-excluded` | built and matches | P7 7.5 with a signed commit: authenticated, authenticated_frames 5 |
| `read/empty` | built and matches |  |
| `read/not-this-format` | built and matches | a map, an array, a uint |
| `read/generation-three` | built and matches |  |
| `read/first-generation/first-and-second` | built and matches | tag first_generation |
| `read/first-generation/second-only` | built and matches | unsupported_generation |
| `read/damaged-and-healed` | built and matches | 7.3: the segment result with its offsets and lengths |
| `read/frame-type-ill-typed` | built and matches |  |
| `read/integrity/each` | built and matches | intact, intact_unchecked, damaged (payload, opaque item, damaged region, unreadable header), not_checked |
| `read/anchored` | built and matches | a broken link at ordinary position 2 of five frames |
| `read/visibility/incomplete` | built and matches | a damaged suppress frame and one of an unknown codec |
| `read/visibility/unauthenticated` | built and matches | unsigned suppress frame [1, 1]; under an authenticating signature [1, 0] |
| `read/visibility/unreadable-header` | built and matches | a suppress frame in a segment with an unreadable header |
| `read/content-digest/two` | built and matches | two segments whose digests differ from the file's |
| `read/own-fold-silent` | built and matches |  |
| `read/value-missing` | built and matches |  |
| `read/value-rebuild-failed` | built and matches |  |
| `read/integrity/rebuild` | built and matches |  |
| `read/rebuild` | built and matches |  |
| `read/index/earlier` | built and matches |  |
| `read/read-failed` | built and matches | offset 12, length 64 |
| `read/older-registry` | built and matches | h 2, r 1, a frame type of version 2: unsupported_critical, later frames stopped |
| `read/capacity/minimum` | built and matches | a frame of decoded length 2^26 at the minimum capacity |
| `read/capacity/above` | built and matches | the same frame with decoded capacity 2^26 - 1 |
| `read/integrity/parity-ignored` | built and matches | the first input of canonical/check/parity-state: parity_damaged at position 7, unprotected_tail, integrity intact |
| `canonical/check/verified` | built and matches | 7.4, the first result: a canonical file read by a reader that holds the reference encoder (10 bytes) |
| `canonical/check/no-encoder` | built and matches | 7.4, the second result: the same file, no reference encoder held (11 bytes) |
| `canonical/check/where-it-stands` | built and matches | P7 4.6 WHERE A DIFFERENCE STANDS, P7-41: seven files, each with the position and the check the text gives and its one canonical_violated |
| `read/torn-tail` | built and matches | 7.2: the file of 7.1 followed by 8b 02 0e; the 256 bytes of the result |
| `canonical/compaction/held` | built and matches | [2, opaque_term, absent]: a fold with an unreadable term in a row |
| `canonical/compaction/refused` | built and matches | [2, labelling_refused, present(refusal_width)] |
| `read/signatures/order` | built and matches | P7-37, P7-40: three signatures ascending (three results in that order); another order, descending, one written twice: malformed_value unordered_or_repeated, opaque malformed, no result |
| `canonical/signatures-order` | built and matches | 7.5 with two signatures added to the commit in either order: the same bytes, ascending E(signature) (P7-37) |
| `profile/whole-file-segment` | built and matches | three segments (container, unreadable header, container), a suppress frame in the first and the third: one profile_violation, check 1, in the third segment, count 2 |
| `read/authenticity/partly` | built and matches | a tail after the last signed frame: the signed commit covers the run, the blob after it is not authenticated |
| `read/authenticity/none` | built and matches | no signature |
| `read/capacity/items` | built and matches | a header and three frames with items_per_read 3: item_count at the end of the second frame, count 4; nothing folded |
| `read/capacity/items-at-limit` | built and matches | a header and two frames with items_per_read 3: all read, no finding |
| `read/capacity/items-damaged-region` | built and matches | a damaged region ended by a resumption counts as one item |
| `read/capacity/order` | built and matches | items_per_read, total and labelled_lines all exceeded: only item_count |
| `read/capacity/total` | built and matches | frames of decoded length 4, 4, 1 with decoded_bytes_per_read 8: the third total_decoded, count 9 |
| `read/capacity/total-at-capacity` | built and matches | the first two frames with decoded_bytes_per_read 8: no finding |
| `read/capacity/total-before-decode` | built and matches | a hostile compressed frame whose decoded_length crosses the total: unchecked, nothing decoded |
| `read/capacity/total-segments` | built and matches | the total counted across two segments in file order |
| `read/capacity/total-then-small` | built and matches | a fourth frame of decoded length 0 after the refusal: total_decoded, count 8 |
| `read/labelling/size` | built and matches | a fold of size 3 read with labelled_lines 2: refusal_size in the file's and the segment's list; labelled_lines 3 labels it |
| `read/labelling/size-guaranteed` | built and matches | a guaranteed set above labelled_lines: refused by size |
| `read/inputs/sets` | built and matches | the same keys in three orders, a key repeated in two: one read_result, the lists as sets (P7-38) |
| `read/order/phases` | built and matches | three segments, a payload of the second above decoded_bytes_per_read: the segments judged in file order (P7-11) |
| `read/order/directions` | built and matches | findings of three steps, of two positions (absent first) and of one position with two encodings: ascending on step, position, bytewise (1.4) |
| `read/resume` | derived from the text | P5 7.4 Case C: the first byte of F2 is damaged: undelimitable_input over the region, the region is one item, F3 at position 4 links against the anchor of the parity frame at which reading resumes and is detached, and anchored_frames is 1 |
| `read/value-two-declarations` | built and matches | a literal_ref term whose digest two value declarations provide, in two segments, the first failing its rebuild (P4-54) |
| `read/capacity/total-sealed` | built and matches | a sealed frame of decoded_length N with an inner frame of 4: the inner payload crosses a total of one below keys + N + m (the text's 6 and 8 cannot be a sealed frame) |
| `read/capacity/total-sealed-at-capacity` | built and matches | the same file with the total exactly keys + N + m: judged, no exceeds_capacity |
| `read/value-length/three` | built and matches | the file of P3 terms-literal-ref-length-pair with length 3: integrity intact, no finding, both content digests present; the value frame's rebuilds [[absent, verified]] |
| `read/value-length/four` | built and matches | the same file with length 4: missing_dependency with value at the terms frame, opaque_term at the quads frame, both content digests absent; the value frame's rebuilds [[absent, verified]] |
| `read/value-length/pairs` | built and matches | the input of P3 terms-literal-ref-pairs-coalesce: two missing_dependency with value, one for each pair, in the order of 1.4 |
| `read/capacity/rebuilt` | built and matches | the input of P4 rebuild-charge-witness read with rebuilt_bytes_per_read 16777215: exceeds_capacity with total_rebuilt, count 16777216, at position 2; the frame result has fold folded and rebuilds [[absent, refused]]; no damaged_frame; integrity intact_unchecked; the content digest present; the result carries the capacity |
| `read/capacity/rebuilt-at-capacity` | built and matches | the same input read with 16777216: rebuilds [[absent, incomplete(blob_digest)]], damaged_frame with blob_digest, integrity damaged |
| `read/capacity/rebuilt-declared` | built and matches | the input of P4 rebuild-charge-declared at the minimum capacity: refused, count 67108864000000, no byte rebuilt |
| `read/capacity/rebuilt-after-decoded` | built and matches | a file whose recipe frame is refused by decoded_bytes_per_read (and read with rebuilt_bytes_per_read 0, so that a charge would refuse it): the recipe is opaque with exceeds_capacity, declares nothing and is not charged; no total_rebuilt finding |
| `read/capacity/rebuilt-value` | built and matches | a literal_ref term whose one candidate is refused: opaque_term, no missing_dependency with value, both content digests absent, rebuilds of the candidate's frame [[absent, refused]] |
| `read/capacity/rebuilt-state` | built and matches | a segment whose candidate of the value of a term is refused, then a commit, and a binding to that commit's state: the commit names no state to this reader (missing_dependency with state_unavailable); with a capacity that admits the candidate it names one |
| `read/capacity/rebuilt-sealed` | built and matches | read by a reader of registry version 2 with the segment key held: a sealed pack whose directory holds a recipe member and a value member, read with a capacity that admits the first only: the finding at the sealed frame, rebuilds of the sealed frame [[present(0), verified], [present(1), refused]]; the same file read without the key: no charge and an empty rebuilds |
| `canonical/check/rebuilt` | built and matches | a canonical file with one multi-unit blob and one value, read with rebuilt_bytes_per_read below the charge of the value member, and the same file with a row frame out of order: structure and labelling unchecked(exceeds_capacity), storage as the encoder gives, no canonical_violated; read with a capacity that admits the value member and refuses the recipe member: structure unchecked(exceeds_capacity), labelling verified |
| `canonical/compaction/rebuild-refused` | built and matches | the fold of a read whose only declaration of a present blob was refused by the total rebuilt length: [2, exceeds_capacity, present(total_rebuilt)]; the same blob also declared by a verified blob frame: the file is written |
| `read/capacity/formed` | built and matches | the input of terms-formed-witness read with formed_bytes_per_read 268365824: the frame exceeds_capacity with total_formed and count 268365825, its fold state opaque with exceeds_capacity, its payload state verified, integrity intact_unchecked; with 268365825 folded, intact |
| `read/capacity/formed-canonical` | built and matches | a canonical file with one terms frame of charge T, read with T - 1: structure and labelling unchecked with exceeds_capacity, no canonical_violated |
| `read/capacity/order-formed` | built and matches | an input that exceeds the total decoded length at a frame and the total formed length at a later terms frame: the later frame is refused by the total decoded length, not charged to the total formed length |
| `read/labelling/weight` | built and matches | the segment of the worked example of P3 6.5 (weight 12448 for each fold) read with labelled_bytes_per_read 24895: the own fold labelled, labelling_refused with refusal_weight in the file's list, the file's content_digest absent, the segment's present, the labelling level of an unclaimed segment not_claimed; with 12447 the finding in both lists and both digests absent |
| `read/labelling/weight-guaranteed` | built and matches | a guaranteed set, size within labelled_lines, weight above labelled_bytes_per_read: refused by weight |
| `canonical/compaction/formed-refused` | built and matches | the fold of a read that refused a terms frame by the total formed length: [2, exceeds_capacity, present(total_formed)]; the same fold with a held row also: the same value, since the formed stop is judged first |
| `canonical/compaction/refused-weight` | built and matches | a fold whose labelling is refused by weight: [2, labelling_refused, present(refusal_weight)] |
| `read/content-order-refused-limit` | built and matches | a fold that is not guaranteed, read with labelling_units below what it needs: refusal_limit; the same order as above |
| `read/content-order-refused-width` | built and matches | two annotation rows whose counts sum above 2^64 - 1: refusal_width, which says that the count is not exact, and the annotation line delivered with the count 2^64 - 1; the lines in the order after a refusal |
| `read/content-order-held` | built and matches | a fold with one held row (a row naming a term of a damaged terms frame) and a labelling that succeeds: content_digest absent, no labelling_refused, delivery in the canonical order |
| `read/content-order-segments` | built and matches | two segments, each with one row whose subject is a blank node of the label b, with one predicate and one object, the labelling refused: the line of segment 0 before the line of segment 1 (E([0, b]) before E([1, b])) |
| `read/content-order-term-places` | built and matches | a fold, the labelling refused, with an IRI and two blank nodes of the labels b and a written in that order: the place of each term is its id in the dictionary built with blank nodes compared on E([ordinal, label]), the node a before the node b |
| `read/content-order-hidden` | built and matches | a fold, the labelling refused, with a quad suppression entry that hides its second line: the visible dataset is the delivered lines without that line, its named graphs in ascending place of the graph term |
| `read/capacity/recovery` | built and matches | the input of recover-scan-charge: the total decoded length counts the candidates that reach their charge, in ascending offset at the place of the region; a frame of a later segment is refused by the total when the candidates have used it |
| `read/content-order` | built and matches | a fold of quads and an empty graph with a suppression entry written in two orders and as its canonical file: the delivered lines, bindings, entries, blob set, visible lines and visible graphs are in the order of the canonical file in all three (P7-39) |
| `read/content-order-refused-size` | built and matches | the fold of read/content-order, in both of its file orders, read with labelled_lines below its size: labelling_refused with refusal_size in the file's findings; the lists in the order after a refusal, the same for both file orders; content_digest absent (P7-46) |
| `read/content-order-sign` | built and matches | one file read with and without enough capacity: labelling_refused in the file's findings only in the first reading; the first reading's lists in the order after a refusal, the second's in the canonical order; the two hold the same sets (P7-46) |
| `read/content-order-renamed` | built and matches | two files whose folds differ only in the labels of two blank nodes, renamed so that their bytewise order is reversed, both read with the labelling refused: the lists differ in order; read with enough capacity, the two files give lists that are equal up to the renaming |
| `canonical/labels-automorphic` | built and matches | the canonical file of the fold of the two-segment file of lowering-blank-inside (two blank nodes in two nested terms; the second run on its canonical file returns positions that differ from the stored labels, so tau is not the identity), read alone: the stored labels are accepted, labelling verified; and the file that the construction of 4.3 forms from quad(_:v, p, a), quad(_:w, q, a) with the positions of the two blank nodes exchanged throughout: structure verified, labelling failed, one canonical_violated with check labelling at the terms frame that holds the first blank term that tau moves |
| `read/capacity/count-saturates` | built and matches | a reader whose every capacity is 2^64 - 1, and a segment of two frames: a frame of decoded_length 1, then a stored_plain frame whose envelope declares decoded_length 2^64 - 1 (its payload is short): the second frame unchecked, exceeds_capacity with total_decoded and the count 2^64 - 1, the sum 2^64 not being a uint (P7-49) |
| `canonical-no-resync` | built and matches | the compaction of a file with a resync segment: header segments only, and its canonical claim holds (P7-81) |
| `canonical-resync-claim` | built and matches | a segment opened by a resync item carries no canonical field, so it makes no canonical claim (P7-81) |
| `canonical/check/repeated-row-frame` | built and matches | a terms frame, a quads frame A (canonical, fewer rows than its cut allows) and a second frame B: B unordered: equal to A [frame_sequence, 3], one row of A and one other [rows, 2], annotations [rows, 2]; B canonical: [rows, 3]; a repeated empty_graphs frame: [frame_sequence, p] |
| `canonical/check/resolution-repeat` | built and matches | evidence for two entries that resolve to one value written as two (the same entry twice): first_difference [suppression, p]; the entry once is the canonical form |
| `canonical/check/triple-depth-order` | built and matches | a claimed segment whose dictionary stores the triple terms t1 = <<a p b>>, t2 = <<b p t1>> (t1 is the OBJECT of t2: see the text defect on <<t1 p b>>) and t3 = <<a p c>> in the order t1, t2, t3: structure failed, first_difference [terms, p], p the terms frame that holds t3, one canonical_violated with reason 64 + terms |
| `canonical/check/unit-cuts` | built and matches | a claimed segment with a blob B that the chunker cuts into u1 and u2, whose recipe names three units: u1 divided in two at a place that is no cut of the chunker, and u2; every unit verified, the blob rebuilt: first_difference [blob_region, p], p the pack holding the first unit that is no unit of the chunker |
| `canonical/check/unused-component` | built and matches | a claimed segment whose dictionary holds the iri term x in its first terms frame and the triple term <<x x x>> in a later terms frame, no line, binding or entry naming either: first_difference [terms, p1], p1 the frame that holds x |
| `file-end-at-authenticated-commit` | built and matches | end at_authenticated_commit, left_out 0 |
| `file-end-at-valid-commit` | built and matches | end at_valid_commit, left_out 0 |
| `file-end-no-commit` | built and matches | end no_commit, left_out 0 |
| `file-end-not-read` | built and matches | a read that fails: end not_read; an items_per_read exceeded: end not_read |
| `file-end-opaque-after-commit` | built and matches | end after_commit, left_out 3 |
| `file-end-region-after-commit` | built and matches | end after_commit, left_out 1 |
| `file-end-torn` | built and matches | end torn, left_out 1 |
| `file-end-undelimitable` | built and matches | end undelimitable, left_out 0 |
| `left-out-refused` | built and matches | a frame refused by capacity: counted |
| `left-out-region` | built and matches | a damaged region: left_out 1 |
| `left-out-sealed` | built and matches | registry version 2, a sealed frame the reader cannot open: integrity intact, left_out 1 |
| `left-out-stopped` | built and matches | an unsupported_critical frame and two frames after it: integrity intact, left_out 3 |
| `left-out-torn-end` | built and matches | a file that ends in an incomplete item: left_out 1, end torn |
| `left-out-torn-mid-file` | built and matches | H T Q1 Q2' (Q2' cut 40 bytes into its payload), then a resync item and T Q3 C appended by a writer that does not read: left_out 1; the same writer's file without the tear: 0 |
| `left-out-unknown-type` | built and matches | a frame of a type the reader does not know, not critical, in a segment of registry version 2 read by a reader of version 1: integrity intact, left_out 1 |
| `left-out-unreadable-segment` | built and matches | the items after an unreadable opener: left_out the count of those items |
| `left-out-zero` | built and matches | an intact file: left_out 0 |
| `marks-hostile-forged-earlier` | built and matches | the honest file H T Q C, then a resync item with a hostile mark on [0, n), strains stored_bytes_per_frame, and a scope: with marks absent the honest result; present with limits.labelling_units below what the fold needs: labelling_refused for the first segment's fold and the file's, Q still folded, left_out 0 (P7-82) |
| `marks-hostile-labelling` | built and matches | a file whose scope mark refuses the labelling: the fold of the file refused, the digest absent, the own digests of the other segments present (P7-82) |
| `marks-hostile-range-fold-only` | built and matches | a hostile mark with a byte target: only the folds that the range covers are judged, no frame (P7-82) |
| `marks-hostile-scope` | built and matches | a frame within capacity and above limits, marks present: unchecked with exceeds_capacity, exceeds_capacity with decoded_size, opaque, link and signatures kept (P7-82) |
| `marks-refused-row-frame` | built and matches | a quads frame refused by limits: the content digest present, the frame opaque, left_out 1 (P7-82) |
| `marks-same-input-same-result` | built and matches | one file read twice with one value of marks: equal results; with marks absent and present: the differences of 2.1 alone (P7 8.1) |
| `marks-total-unmoved` | built and matches | a hostile scope mark whose limits refuse a large frame, and a later damaged region whose candidate the total refuses: the same candidate refused and the same items accepted with marks absent and present (P7-82) |
| `read/value-after-term` | built and matches | a literal_ref term whose value stands after its terms frame, in its segment and in a later one: the term is unreadable, missing_dependency with value at the terms frame, whatever names it held with opaque_term (P3-114) |
| `resync-stream-end-declared` | built and matches | a journal restarted after a torn frame below stored_bytes_per_frame: the scope delivered at the declared end |
| `resync-stream-prefix-bound` | built and matches | the same with a torn frame longer than stored_bytes_per_frame + resync_max bytes as completed: found under whole, not under forward |
| `resync-stream-undelimitable` | built and matches | bytes that cannot be delimited, then a resync item with a true offset and its scope: the item accepted as the bytes arrive, the bytes before it undelimitable_input or torn_tail as the whole read gives them |
| `stream-before-first-item` | built and matches | a file whose first item is cut: end false: no segment and no finding; end true: torn_tail alone; a first-generation file: an empty listing at every point, end false the result for b 0, end true the result of whole but for capacity.access |
| `stream-chunking` | built and matches | one file delivered in chunks of 1 byte, 7 bytes, 4096 bytes and whole: equal stream points at every common boundary |
| `stream-committed-view` | built and matches | the commit-valid segment (H F1 T C F3) read as a stream: at the commit committed_frames covers the items of F1 and T and not those of F3 |
| `stream-delivery-order` | built and matches | registry version 2: a header, a keys frame whose item reaches the end of span 0, the parity frame of that span and the sealed frame under the keys frame: after the parity frame no item of the keys frame or of the parity frame; after the sealed frame the frame item of the keys frame, then the items of the sealed frame; and the same file with the stored identifier of the keys frame replaced, which the parity frame heals: the same listings, the keys frame's item with its computed identifier |
| `stream-empty-graph-after-line` | built and matches | a line of graph g, then an empty_graphs row for g: the row delivered folded, open empty_graph, absent from the fold of PR-3 |
| `stream-end-declared` | built and matches | a file whose last item is incomplete: no torn_tail at any point with end false, torn_tail with end true |
| `stream-every-byte` | built and matches | one file at every byte: end false: the point of the last boundary; end true: the forward read of the bytes as a file, torn_tail where a cut falls inside an item |
| `stream-every-prefix` | built and matches | for each file, the stream point at every item boundary against the forward read of that prefix as a file |
| `stream-field-authentication` | built and matches | a signed commit, then an unsigned frame: authenticated, then partly_authenticated |
| `stream-field-link` | built and matches | a pending commit: broken at the commit, healed and valid at the next frame |
| `stream-field-offsets` | built and matches | an offsets item with a wrong length then a frame: index_claim_failed index_offsets, then gone and malformed_value unordered_or_repeated at the offsets item; an offsets item whose claim holds then the opaque item 01: no finding at the third boundary, malformed_value unordered_or_repeated at the offsets item, malformed_value wrong_type at the opaque item, integrity damaged at the fourth |
| `stream-field-tail` | built and matches | a parity frame of span 0, then two frames: the unprotected_tail length grows with each |
| `stream-field-visible` | built and matches | read with stored_bytes_per_frame and decoded_bytes_per_frame 4096: a suppress frame of more than 4096 bytes after a complete prefix: complete, then incomplete |
| `stream-frame-items` | built and matches | a quads frame after a frame that stops the fold, a checked index frame, a frame whose content is damaged and that a frame follows, and a parity frame: one frame item for each of the first three, the stored identifier for the third, the index_claim items of the index, no other item of the first and the third, none of the parity frame |
| `stream-heal-hold` | built and matches | the shape of stream-heal-pending-full/-prefix (a small segment: H T Q, a commit whose stored identifier is 32 bytes 00, one more frame linked to its computed identifier) read as a stream: at the commit the commit broken and no state handed over; at the next frame healed |
| `stream-heal-opaque-between` | built and matches | an opaque item between the pending frame and the next frame: the frame broken with chain_break and settled at the boundary after the opaque item, no item of it in any listing |
| `stream-heal-parity` | built and matches | a pending last ordinary frame whose identifier field is damaged, followed by a parity frame of its span: at the boundary after the parity frame the frame healed and its items delivered there, before any item of the parity frame, which gives none |
| `stream-items-limit` | built and matches | a file of 11 items read with items_per_read 8: from the first byte after item 8, end false, the result of the first 8 items with exceeds_capacity item_count (offset the end of item 8, length the bytes received after it); end true: the length of the bytes after it, as whole |
| `stream-keys-pending` | built and matches | registry version 2: a keys frame is decided when the item after it arrives (P8-61): pending with malformed_value unordered_or_repeated and no item until the sealed frame comes; stored_compressed: forbidden_combination; two signatures in descending order: decided at its arrival |
| `stream-no-resume` | built and matches | a frame whose payload head runs past the end of the file, a true parity frame and two frames: as a stream with the end declared torn_tail at the frame and nothing after it read; with whole the region ends at the parity frame and the two frames are read |
| `stream-not-run` | built and matches | a file with a suppression entry that matches nothing, a recipe whose units stand before it, a chunk that no entry names, a damaged frame with a good parity span and a closing index with a false content_digest and a false graph_directory: none of suppression_unmatched, unreferenced_unit, repaired, index_digest, index_directory; damaged_frame and parity_mismatch present; the recipe's rebuild state not_run and integrity damaged; the five claims as items with status unverified |
| `stream-open-blob_entry` | built and matches | a blob declaration (open blob_entry and hiding), then a declaration of the same name with another media type |
| `stream-open-count` | built and matches | an annotation row (open count and hiding) and a later annotation row with the same reifier, predicate, object and graph |
| `stream-open-empty_graph` | built and matches | an empty_graphs row (open empty_graph and hiding), then a line that carries the graph's name |
| `stream-open-hiding` | built and matches | a file whose prefix holds a folded row (open hiding) and whose continuation holds the suppress entry that hides it; the listing at both points, the earlier a prefix of the later |
| `stream-open-name` | built and matches | an entry that holds a leaf [frame, label] naming no node (open name), then a segment that holds the frame and defines the label |
| `stream-open-referent` | built and matches | a binding whose referent names a blob not declared yet (open referent and hiding), then the blob declaration |
| `stream-order` | built and matches | a file whose canonical order and arrival order differ: the listing in arrival order; the same file read with whole, the content in the order of THE ORDER OF THE CONTENT |
| `stream-parity-long-item` | built and matches | an opaque item of 17 * 4194304 bytes (71303168), above the minimum stored_bytes_per_frame, then the parity frames of the spans it crosses: no parity_mismatch for them as a stream, and the read with whole judging them |
| `stream-parity` | built and matches | the P5 vectors parity-damaged-shard, parity-stale-geometry, parity-unexplained, parity-stale-equal and parity-span-short read as a stream: the same parity_* findings and no repaired |
| `stream-pending-at-end` | built and matches | a complete file whose last item is a frame with damaged content and a stored identifier of the digest length: at the end of the input, end true, the frame broken in the result and no item of it in any listing |
| `stream-pending-header` | built and matches | a pending frame followed by an opener: broken and settled at the opener |
| `stream-pipe` | built and matches | the file delivered through an interface that fails on a seek or a second read; the result equals the result delivered from a buffer |
| `stream-prefix-guarantee` | built and matches | for the files and every ordered pair of item boundaries j <= k: PR-1 (the listing at j is a prefix of the listing at k), PR-2 (each item of G_k not in G_j belongs to a frame decided at a boundary above j and at or below k), PR-3 as the checker below states it (199 ordered pairs checked, PR-3 at 21 boundaries) |
| `stream-profile-not-checked` | built and matches | a segment that declares a profile other than stream that the reader implements, read with forward: profile_not_checked, no profile_violation |
| `stream-read-failed` | built and matches | a header and a frame, then a read that fails at the offset after the frame for 64 bytes: read_failed with that offset and length, no later input point; a first-generation file with a failure inside it: the result of whole for those bytes and that failed read but for capacity.access, an empty listing |
| `stream-row-after-entry` | built and matches | a suppress entry, then a row that it hides: the row delivered folded, open hiding |
| `stream-row-values` | built and matches | a quads frame with one folded row, one row held with opaque_term and one row dropped with forward_reference: the value of the folded row its typed line; of each of the other two the graph id (absent for the default graph) and the column values in column order (predicate, object, subject) |
| `stream-two-sittings` | built and matches | a file read in two sittings, the first ending at an item boundary inside a segment that holds a value declaration, a terms frame and a commit before the boundary and a terms frame with a literal_ref term of that value, a parity frame and the next commit after it: the stream points at every item boundary of the second sitting equal those of one sitting |
| `stream-undelimitable` | built and matches | a complete frame, then bytes that cannot be delimited: undelimitable_input from the input point at which they cannot be delimited, end false, its length growing with the bytes received |
| `stream-value-before` | built and matches | a value declaration before a terms frame with a literal_ref term and a row that names it: by_reference, settled for the term; the declaration after: unreadable, missing_dependency value, the row held; the unit missing: by_reference in the stream, unreadable and held with opaque_term in whole; two terms frames and no declaration: one missing_dependency at the first; the forward_reference case |
| `idx-after-index` | built and matches | an index in mid-segment, quads frames, the last index named by the locator, one more quads frame: the frame after it is not returned |
| `idx-annotations-repeated` | built and matches | two annotation rows equal after resolution but for the count: the frame is malformed |
| `idx-capacity` | built and matches | the query of idx-graph-default with a decoded_bytes_per_read that refuses the last frame of the target |
| `idx-charges-once` | built and matches | each range charged once; the payload of the parity frame that the walk passes neither read nor charged; stored_compressed besides; a unit read only as the item before |
| `idx-chunk-empty` | built and matches | a chunk frame whose decoded_length is 0, whose payload is empty and whose unit_digest is the digest of the empty string |
| `idx-dictionary` | built and matches | rows whose ids fall in the first and the last terms frame of a segment of many: every covered terms frame is read, in item order |
| `idx-directory-claims` | built and matches | four files with a true offsets item and frames that verify locally: equal ranges, a row_count of 0, a range of the frame type terms, a first_row at the number of rows |
| `idx-directory-contradiction` | built and matches | a graph_directory entry that names a frame whose own graph_runs give the graph other rows |
| `idx-false-locator` | built and matches | a locator whose distance is above its own offset; a locator whose distance leads into the payload of a frame |
| `idx-false-offsets` | built and matches | an offsets item whose length of the index is wrong; one whose length of the first frame is too small by 1 |
| `idx-frame-type` | built and matches | a covered frame of the target whose element 1 is 200; an index whose type_runs names the type 200 |
| `idx-graph-default` | built and matches | a canonical-style file of one segment: the rows of the default graph, the scope, the typed lines equal to those of a read with `whole` |
| `idx-graph-named` | built and matches | the rows of a named graph of the file of idx-graph-default, the graph named by the text of its IRI |
| `idx-id-base` | built and matches | a terms frame whose id_base differs from its expected base |
| `idx-identifier` | built and matches | a frame of the target whose predecessor's stored identifier is damaged |
| `idx-literal-ref` | built and matches | a row whose object is a literal_ref term: the term by reference, by_reference true, no value frame read |
| `idx-local-after-header` | built and matches | the first frame of a segment: verified with h0 |
| `idx-local-after-parity` | built and matches | the file of idx-placement: the frame of the target whose item before it is the parity frame of a full span is verified with the anchor of that frame |
| `idx-local-opaque-before` | built and matches | the item before a frame of the target is a covered frame made opaque (first byte 8a); and one of 5 + 2^28 bytes whose first head is 5a 10 00 00 00 |
| `idx-locator-earlier-index` | built and matches | the same segment with a locator whose distance names the index in mid-segment |
| `idx-mixed-chains` | built and matches | two segments A and B; the offsets item of B replaced so that the header and the first terms frame are A's: the rows of B resolved over the terms of A |
| `idx-nested` | built and matches | an offsets item and a locator appended after a blob frame whose payload holds a complete segment: the nested rows, the scope names the nested header |
| `idx-not-envelope` | built and matches | a frame of the target, the item after it another frame of the target, whose first byte 8b is changed to 40 and, in a second file, to 8a |
| `idx-offsets-zero-length` | built and matches | a true file whose offsets item has one extra element 0 before the length of the index |
| `idx-pack-members-type` | built and matches | a true file whose index has one false pack_members entry for the digest asked, naming the ordinary position of a quads frame |
| `idx-payload-digest` | built and matches | a frame of the target whose payload has one byte changed |
| `idx-payload-not-bytes` | built and matches | an envelope whose element 8 is 81 18 ff: as the item before a frame, and as a frame after the index |
| `idx-payload-past-item` | built and matches | the head of a payload of 300 bytes rewritten from 59 01 2c to 59 ea 60: unknown, payload, nothing of the payload read or charged |
| `idx-placement-passed` | built and matches | an opaque item `01` among the frames of a short segment: three items taken as covered frames against frame_count 2 |
| `idx-placement-short` | built and matches | an index, the parity frame of the short span that ends at it, more frames and a closing index |
| `idx-placement` | built and matches | a segment of more than 4194304 bytes of S with a parity frame at the span end before the closing index: the rows of the frame after the first parity frame |
| `idx-query` | built and matches | a target digest of 20 bytes in a segment whose digest has 32 |
| `idx-read-failed` | built and matches | a read of the range of the index that fails |
| `idx-read-order` | built and matches | a frame of the target whose head is of another type than type_runs names, after the opaque item |
| `idx-reads` | built and matches | one file and one query: the ranges read, in order, are those of the steps |
| `idx-reason-order` | built and matches | the first failing check decides: offsets before a damaged frame; index_claims before a head that is no envelope; offsets before capacity |
| `idx-resync-opener` | built and matches | a closed segment that a resync item opens, after a header segment: a graph query of the second segment is answered, the scope names the offset of the resync item |
| `idx-row-dropped` | built and matches | a row whose subject names a literal is dropped; a row of a graph named by an id that a later terms frame defines is dropped as a forward reference |
| `idx-scope-unknown` | built and matches | the segment of idx-sealed read with segment_end the length of the file: unknown, sealed, the scope names the header of the segment and the frame_count of the index |
| `idx-sealed` | built and matches | registry version 2: a segment with a keys frame, a sealed frame and a clear quads frame of the target, and a closing index that names them |
| `idx-suppress-by-reference` | built and matches | a suppress entry naming a term by reference: unknown, value with suppression true; the row with no visibility with false; the file of idx-literal-ref with true: visible |
| `idx-suppress` | built and matches | a covered suppress entry that hides a row of the answer; a suppress entry of a later segment hides nothing in the answer for the earlier one |
| `idx-term-self` | built and matches | a triple term whose subject id is its own id: the row naming it held, the other rows answered |
| `idx-term-unreadable` | built and matches | an iri term whose text is not valid UTF-8: the first row held and not returned, the second returned |
| `idx-two-segments` | built and matches | a file of two segments each holding rows of the default graph: the last segment, then the first by the offset of the second header |
| `idx-unit-chunk` | built and matches | a unit held by a `chunk` frame outside any pack, in a file whose pack_members do not name it: found by its head |
| `idx-unit-digest` | built and matches | a chunk frame whose content names the digest d asked for and whose payload does not hash to it |
| `idx-unit-pack` | built and matches | a unit that is a member of a `pack` frame: one item with that `member` |
| `idx-wrong-type` | built and matches | an index whose type_runs names the first two covered frames, a terms frame and a quads frame, in the other order |
| `end-frame-no-effective-identifier` | built and matches | a frame with no effective identifier after the last valid commit is an ordinary frame other than an `index` frame: `end` is `after_commit` (P7 1.2, P1 3.4) |
| `end-locator-own-grammar` | built and matches | an item of kind `locator` that breaks its own grammar after the last valid commit does not move the end: `end` is `at_valid_commit` (P7 1.2) |
| `findings-order-by-step` | built and matches | the refusal of the charge of a scan candidate is a finding of the step PAYLOAD CHECK: in the file's list after `not_this_format` (of the step GENERATION), for a first item that is a map with no key `"v"`; in a segment's list after `lineage_begins` (of the step OPENER) (P7 1.4 ORDER, P7 3) |

Totals: built and matches: 277 (277 entries).
