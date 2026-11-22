# Vectors of P4

| vector | outcome | notes |
|---|---|---|
| `unit-digest` | built and matches | the 2^20-byte string is the first 2^20 bytes of BLAKE3 extended output over 'P4 unit-digest' (the .bin file) |
| `blob-name` | built and matches |  |
| `chunker-table` | built and matches | the first 64 bytes equal the 64 bytes of hctx-empty-message of P1 (the same extended output stream) |
| `chunker-cut-strict` | built and matches |  |
| `chunker-cut-min` | built and matches | 16382 pseudo-random bytes, then two bytes chosen so that strict holds at the first test |
| `chunker-cut-loose` | built and matches |  |
| `chunker-cut-max` | built and matches | 300000 bytes of the byte 0: after 64 updates the register is -G[0] mod 2^64, which no mask accepts |
| `chunker-golden` | built and matches | stream: BLAKE3 extended output over 'gts2 P4 chunker golden stream' (the .bin file); text file: GTS-2.0-text/P1.md as it stood; a second implementation is a window-sum formulation run on first16385 and the text (the rolled register is the engine's); the 1 MiB of zero bytes give max-length chunks |
| `chunker-end` | built and matches | the register at length 16384 fails strict, so the 16385-byte string ends at the end of input |
| `chunker-restart` | built and matches |  |
| `chunker-two-byte-step` | built and matches | the two-byte register tests only after each pair of bytes; its lengths differ, and the expected lengths are those of the one-byte rule |
| `chunk-ok` | built and matches | a chunk named by a value, so that P4-41 gives no unreferenced_unit |
| `chunk-empty` | built and matches |  |
| `chunk-unit-digest` | built and matches | a lone chunk is folded (P7 1.3: a unit damage does not change the fold state) and named by nothing, so P4-41 also gives unreferenced_unit |
| `chunk-payload-digest` | built and matches | one damaged_frame at the payload check; the frame is opaque, so it is not in the unit index and has no second finding |
| `blob-ok` | built and matches |  |
| `blob-empty` | built and matches |  |
| `recipe-ok` | built and matches |  |
| `recipe-one-entry` | built and matches |  |
| `recipe-list-length` | built and matches |  |
| `recipe-zero-entry-length` | built and matches |  |
| `recipe-sum-overflow` | built and matches |  |
| `recipe-sum` | built and matches |  |
| `recipe-digest-length` | built and matches |  |
| `recipe-order` | built and matches | a list of three entries under entry_count 2, one of length 0: the list-length check comes first |
| `value-ok` | built and matches |  |
| `value-zero-entries` | built and matches |  |
| `value-sum` | built and matches |  |
| `value-not-in-set` | built and matches | the blob frame of the same 11 bytes is the only entry; without it (case 'value-only') the set is empty |
| `external-ok` | built and matches |  |
| `external-payload` | built and matches | an external frame whose decoded_length is not 0, stored plain or compressed, is malformed_value with out_of_range, a field rule judged at ITEMS, and a payload that is not empty under decoded_length 0 is damaged_frame with decoded_length alone (P4-18) |
| `writer-one-unit-blob` | built and matches |  |
| `pack-ok` | built and matches | the pack of section 4 alone: its chunk member is named by nothing, so P4-41 gives unreferenced_unit at the pack |
| `pack-zero-members` | built and matches |  |
| `pack-member-count` | built and matches |  |
| `pack-bodies-length` | built and matches |  |
| `pack-unit-length` | built and matches |  |
| `pack-chunk-empty-body` | built and matches |  |
| `pack-member-tag` | built and matches | member_kind has no external value; the member holds the content of an external frame under the tag 5, and under the frame code 12 |
| `pack-recipe-member-sum` | built and matches |  |
| `pack-recipe-trailing` | built and matches |  |
| `pack-recipe-trailing-count` | built and matches | a recipe member whose body has a byte after the list and whose list has two entries under entry_count 3: wrong_length, the trailing-bytes test coming first (P4-21 to P4-23) |
| `pack-two-bad-members` | built and matches | member one (a chunk with an empty body: out_of_range) and member two (a recipe with a byte after its list: wrong_length) both violate; the members are judged in directory order and the one malformed_value is the reason of member one (P4-22) |
| `pack-member-digest` | built and matches | the failing chunk member's stated digest is named by no recipe or value of the segment: P4-41 also reports it (unreferenced_unit at the pack); the blob member that fails in the second case still declares its blob |
| `pack-recipe-declares` | built and matches | an external of the same name states 10: the entry takes 10 and the recipe member (11) conflicts, at the pack's position |
| `pack-compressed` | built and matches | the one-frame pack decodes to the payload of section 4 (its chunk member is unreferenced, P4-41) |
| `blobset-entry` | built and matches | the 48 bytes of section 5 |
| `blobset-record-lines` | built and matches |  |
| `blobset-one-entry-per-name` | built and matches |  |
| `blobset-length` | built and matches | a blob frame (5), externals of 9 and 3: the entry takes 3 in every order; the two others conflict |
| `blobset-conflict` | built and matches |  |
| `blobset-order-free` | built and matches |  |
| `rebuild-ok` | built and matches |  |
| `rebuild-first-verified` | built and matches |  |
| `rebuild-value-first-verified` | built and matches | two value declarations of one digest in two segments, one failing its rebuild (a unit missing) and one verified: the term is readable with the octets of the verified one, the failure is reported at its own position (the blobs part's missing_dependency with chunk) and there is no missing_dependency with value; the same with the segments swapped gives the same term and the same findings (P4-54) |
| `rebuild-value-none-verified` | built and matches | two value declarations of one digest, both failing their rebuilds: the term is unreadable, one finding at each declaration (missing_dependency with chunk), the row held, and no missing_dependency with value (P4-54) |
| `rebuild-missing` | built and matches |  |
| `rebuild-missing-damaged` | built and matches |  |
| `rebuild-blob-length` | built and matches |  |
| `rebuild-blob-digest` | built and matches |  |
| `rebuild-other-segment` | built and matches | the two chunks of segment 0 are named by no recipe of their own segment, so P4-41 also reports them |
| `rebuild-value-other-segment` | built and matches | the value frame stands in segment 0 and the term that names it in segment 1; in the second case the value's unit stands in segment 1, beside the term: the value is not rebuilt (P4-36), the term is unreadable and its row held; the chunk of segment 1 is named by no recipe of its own segment, so P4-41 also reports it |
| `rebuild-capacity` | built and matches | a reader of stated capacity 150 bytes per frame (under the minimum of P7-8, for the test) |
| `rebuild-unaffected` | built and matches |  |
| `rebuild-incomplete-no-output` | built and matches |  |
| `unit-unreferenced` | built and matches |  |
| `blob-no-media-type-use` | built and matches | the blob's bytes are a file of this format under its media type (P1); the fold holds the one record line of the blob |
| `canon-regions-shared` | built and matches | the value is given to the region as its octets: which literals are values of the canonical file is P7 (threshold) |
| `canon-pack-cut` | built and matches | 64 one-unit blobs of 262144 bytes each of a constant byte that no mask cuts (16777216 bytes of bodies), then one more blob of one byte; the cuts are the chunker's (one unit each), passed directly to save time |
| `canon-unit-shared-regions` | built and matches |  |
| `canon-one-unit-blob-first-emitted` | built and matches |  |
| `canon-empty-blob` | built and matches |  |
| `canon-hidden` | built and matches | a blob declaration hidden by a suppression entry of kind blob stays in the blob set and the regions of 7 write it once |
| `canon-media-types` | built and matches | 'a/b' (4 bytes encoded) is smallest by E, then 'text/plain', then 'zz/top-level' |
| `canon-value-region-typed-octets` | built and matches |  |
| `canon-value-order` | built and matches |  |
| `canon-blob-order` | built and matches |  |
| `canon-external-order` | built and matches |  |
| `canon-frame-outside-pack` | built and matches |  |
| `canon-blob-set-preserved` | built and matches |  |
| `canon-value-region-literals` | built and matches |  |
| `canon-claim-refused` | built and matches |  |
| `rebuild-value-length` | built and matches | a verified value of (D, 3) and a term of (D, 4): the term unreadable, missing_dependency with value, the value read for no term and still verified (P4-54) |
| `rebuild-value-length-shared` | built and matches | one verified value of (D, 3) and two terms of (D, 3) and (D, 4) in one terms frame: the first read, one missing_dependency for the second, the rebuild state reported once (P4-54) |
| `rebuild-charge-witness` | built and matches | a chunk of 16384 bytes and a recipe of 1024 entries naming it, total_length 16777216 and a wrong blob_digest: with 16777215 one exceeds_capacity with total_rebuilt, count 16777216, the rebuild state refused, no damaged_frame, the blob UNCHECKED, intact_unchecked; with 16777216 and at the minimum damaged_frame with blob_digest (P4-55, P4-57, P4-58) |
| `rebuild-charge-declared` | built and matches | one chunk of 67108864 zero bytes and a recipe of 1000000 entries naming it, total_length 67108864000000, read at the minimum: one exceeds_capacity with total_rebuilt and that count, the rebuild state refused, no other finding of the recipe (P4-55, P4-57) |
| `rebuild-charge-unused` | built and matches | a recipe whose blob no binding names, read with a capacity one below its charge: refused, as a recipe that a binding names would be (P4-55) |
| `rebuild-charge-missing-unit` | built and matches | a recipe whose second unit is absent, charge T: with T - 1 refused and no missing_dependency; with T, missing_dependency with chunk and the rebuild state incomplete with chunk (P4-55, P4-58) |
| `rebuild-charge-repeated` | built and matches | two recipe frames of one blob name with equal entries, each of charge T: with 2T - 1 the first verified, the second refused with count 2T, the blob COMPLETE; with 2T both verified (P4-59) |
| `rebuild-charge-then-small` | built and matches | three recipe frames of charges 100, 50 and 10 read with 120: the first admitted, the second refused with count 150, the third refused with count 110 although it would fit (P4-57) |
| `rebuild-charge-order` | built and matches | a pack whose directory holds a recipe member of charge 60 then a value member of charge 60, and after the pack a recipe frame of charge 10, read with 100: the recipe member admitted, the value member refused with count 120, the recipe frame refused with count 70; the same members in the other directory order: the value member admitted (P4-56) |
| `rebuild-charge-segments` | built and matches | two segments, each with one recipe of charge T, read with T: the first segment's recipe admitted and the second's refused (P4-56) |
| `rebuild-charge-not-charged` | built and matches | a blob frame of 1048576 bytes, a pack with one blob member and an external frame, read with rebuilt_bytes_per_read 0: no total_rebuilt finding, every rebuilds list empty (P4-55) |
| `rebuild-charge-value-term` | built and matches | a literal_ref term whose one candidate is refused: the candidate's exceeds_capacity with total_rebuilt, its rebuild state refused, opaque_term at the frame of each row that names the term, no missing_dependency with value, the content digest absent (P4-58) |
| `rebuild-charge-two-candidates` | built and matches | two candidates of one term in two segments: a capacity that admits the first and refuses the second reads the term from the first and reports the second refused; a capacity that admits neither: the term unreadable, two refused outcomes, no missing_dependency (P4-54, P4-58) |
| `rebuild-charge-spooled` | could not be built | the reference engine has one path for holding the output of a rebuild (in memory) and no switch that spools it or streams it to a caller, so there are not two engines to compare; the engine's charge depends on total_length alone (blobs.collect) and a second read of the same input gives an equal result, which rebuild-charge-repeated and the equal results of rebuild-charge-witness show without a switch (P4-39, P4-59) |
| `rebuild-unit-after-recipe` | built and matches | a recipe frame then the chunk frames of its entries; the units first; the recipe and the units as members of one pack, in both orders |
| `rebuild-unit-after-value` | built and matches | a value frame, then the chunk frames of its entries, and a literal_ref term of its pair standing after all of them; and the units first |
| `value-opaque-declares-nothing` | built and matches | a `value` frame that is `broken` (opaque) declares nothing: the `literal_ref` term of a later `terms` frame of its digest has no candidate, `missing_dependency` with `value` at the `terms` frame (P4 5, P4-54, P3-92) |

Totals: built and matches: 98, could not be built: 1 (99 entries).
