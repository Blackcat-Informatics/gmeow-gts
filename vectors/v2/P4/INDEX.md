# Vectors of P4

| vector | outcome | notes |
|---|---|---|
| `unit-digest` | built and matches | the 2^20-byte string is the first 2^20 bytes of BLAKE3 extended output over 'P4 unit-digest' (the .bin file) |
| `blob-name` | built and matches |  |
| `chunker-table` | built and matches | the first 64 bytes equal the 64 bytes of hctx-empty-message of P1 (the same extended output stream) |
| `chunker-cut-strict` | built and matches |  |
| `chunker-cut-min` | built and matches | 16382 pseudo-random bytes, then two bytes chosen so that strict holds at the first test, then 40000 further bytes: the lengths are 16384 and 40000 |
| `chunker-cut-loose` | built and matches |  |
| `chunker-cut-max` | built and matches | 300000 bytes of the byte 0: after 64 updates the register is -G[0] mod 2^64, which no mask accepts |
| `chunker-golden` | built and matches | the cut lengths of the chunker on the empty input, on the first 63, 64, 65, 16384 and 16385 bytes of the stream, on 1 MiB of zero bytes (max-length chunks) and on the stream (BLAKE3 extended output over 'gts2 P4 chunker golden stream', the .bin file, listed in ABSENT.md), also with one byte inserted; the result also records the lengths for a text file that is not stored |
| `chunker-end` | built and matches | the register at length 16384 fails strict, so the 16385-byte string ends at the end of input |
| `chunker-restart` | built and matches | two strings of 150000 bytes, BLAKE3 extended output over 'P4 restart a' and over 'P4 restart b', and their concatenation: chunked as one string, a chunk spans the boundary, so the lengths of the concatenation are not those of a followed by those of b, and a writer that chunks each blob on its own restarts the cut at offset 0 of b (P4-12) |
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
| `writer-one-unit-blob` | built and matches | the blob of the three bytes abc with media type text/plain, of one unit: written as one blob frame and no chunk or recipe frame (case plain, P4-19), and in its canonical file as one blob member of one pack and no recipe member (case canonical, P4-44, verified) |
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
| `canon-regions-shared` | built and matches | the canonical file of a fold cut from 400000 bytes of BLAKE3 extended output over 'P4 canon shared', whose chunking gives the units c1 (48824 bytes), c2 (73555) and c3 (80312): the multi-unit blobs b1 = c1 c2 (media type x/1) and b2 = c2 c3 (none), the one-unit blob b3 = c3 (a/a and b/b), an external name 7e7e...7e of length 9, and a literal_value_ref v of kind octets whose 597843 octets are c2 then 524288 bytes over 'P4 canon shared value'. Ascending E(name) is b3, b2, b1. The value region is one pack: chunk c2, 8 chunk members of the further units of v, value v; the blob region is one pack: blob c3, chunk c2, recipe b2, chunk c1, recipe b1: c2 is stored in both regions, each unit once in its region, b3 is a blob member with least(M) a/a, and the external frames are 7e.. with no media type, then b3 with b/b; the canonical check verifies it |
| `canon-pack-cut` | built and matches | the packing of the blob region by P = 16777216: 64 one-unit blobs of 262144 bytes each of one constant byte, the first 64 byte values c in ascending order for which (-G[c] mod 2^64) >= 2^50, so that no mask cuts (case 64-blobs: one pack of 64 blob members, unit_length 16777216, equal to P and so not cut); the same with a blob of the one byte 01 added, which stands at index 27 of the 65 names in ascending E(name) (case 65-blobs: the 65th member would bring the sum to 16777217, above P, so the first pack closes with 64 members and the last blob of 262144 bytes is alone in a second pack); the canonical check verifies both |
| `canon-unit-shared-regions` | built and matches | the canonical file of a fold that holds a literal_value_ref of kind octets whose 524288 octets are BLAKE3 extended output over 'P4 canon unit shared' (8 units) and a one-unit blob whose bytes are the first unit of that value (113355 bytes): the unit is stored in both regions, as the first chunk member of the value pack and as the one blob member of the blob pack (P4-45); the canonical check verifies it |
| `canon-one-unit-blob-first-emitted` | built and matches | the canonical file of a fold of two present blobs: a multi-unit blob of 150000 bytes (the first 150000 bytes of BLAKE3 extended output over 'P4 first emitted'), first in E(name), and the one-unit blob of its first unit (105745 bytes) with media type m/t: the first unit is emitted once, as a blob member with m/t, among the units of the multi-unit blob and before its recipe member, and nothing more is emitted for the one-unit blob; the canonical check verifies it |
| `canon-empty-blob` | built and matches | the canonical file of a fold whose one present blob is the empty blob: one pack in the blob region with one member of kind blob, digest H_ctx(unit, empty) and body length 0, member_count 1 and unit_length 0; the canonical check verifies it |
| `canon-hidden` | built and matches | a fold whose one present blob, the eleven bytes 'hidden blob' in a blob frame, is hidden by a suppression entry of kind blob: the declaration stays in the blob set (P4-33), and the canonical file of the fold writes it once, as one blob member of the one blob pack, beside the suppress frame; the canonical check verifies it (P4-48) |
| `canon-media-types` | built and matches | the canonical file of a fold whose one present blob, the seventeen bytes 'three media types', has the media types a/b, text/plain and zz/top-level: the blob member carries least(M), a/b (E(a/b) is 63612f62, smallest bytewise), and two external frames of length 17 follow the pack, text/plain then zz/top-level, ascending by E(media_type); the canonical check verifies it |
| `canon-value-region-typed-octets` | built and matches | a fold of three literals: the typed value of kind octets of 1048576 bytes v, the typed literal of datatype xsd:string whose lexical form is the same 1048576 bytes, and the typed value of kind octets of 524287 bytes w: the value region of the canonical file holds one value, the 1048576 bytes v, named by the literal_value_ref and by the literal_ref term; the octets value below the value threshold is a literal_value term and has no value; the canonical check verifies the canonical file |
| `canon-external-order` | built and matches | a fold of two external names and no present blob, 0303...03 of length 20 and 0505...05 of length 10, no media type: in its canonical file (case canonical, verified) the external frame of 03.. stands at position 1 and that of 05.. at 2, ascending by E(name); the file whose two external frames are exchanged (case swapped) is failed at the structure level with the first difference blob_region at position 1 and one canonical_violated there (P4-47, P4-50) |
| `canon-frame-outside-pack` | built and matches | three files that claim the canonical form and hold one frame outside a pack, each otherwise the canonical file of its fold: a chunk frame of abc (the unit of a present blob) before the one blob pack (case chunk, position 1); a value frame of the value of 1048576 bytes x after the one value pack, where the canonical file holds the terms frame (case value, position 2); a chunk frame of abc after the terms frame, where the canonical file holds the quads frame (case chunk-at-rows): each is failed at the structure level with the first difference value_region for the value frame and blob_region for a chunk frame at the position of that frame, never frame_sequence, and one canonical_violated there (P4-47, P4-50) |
| `canon-blob-set-preserved` | built and matches | the fold of the worked example of P4 5 (a blob of the five bytes 'hello' declared by a blob frame with media type a/b and by an external frame of length 5 with c/d, and a second external frame of c/d that states length 6, one conflicting_declaration at position 3) and its canonical file, one blob member with a/b and one external frame of length 5 with c/d: the blob set [1, [1, D], 5, [a/b, c/d]], D = H_ctx(unit, hello), and its one blob_declaration record line are the same in both files (P4-52) |
| `canon-value-region-literals` | built and matches | a fold of two typed literals of datatype xsd:string, one whose lexical form is 1048576 bytes x (stored in the source as a literal_ref with its value frame) and one of 1048575 bytes y, the source also holding a value frame that no term names: the value region of the canonical file holds one value, the 1048576 bytes x, for the literal at the literal threshold, which is a literal_ref term; the literal one byte below it is a literal term and the unnamed value is not written; the canonical check verifies the canonical file |
| `canon-claim-refused` | built and matches | a fold whose one present blob (AAAAAABBBBB, a recipe frame of two entries) has no declaration with every unit, the chunk BBBBB being held by no frame: compaction writes no canonical file and stops with [2, missing_dependency, present(chunk)] (P4-51; P7, "Compaction", case 4) |
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
| `rebuild-unit-after-recipe` | built and matches | a recipe frame then the chunk frames of its entries; the units first; the recipe and the units as members of one pack, in both orders |
| `rebuild-unit-after-value` | built and matches | a value frame, then the chunk frames of its entries, and a literal_ref term of its pair standing after all of them; and the units first |
| `value-opaque-declares-nothing` | built and matches | a `value` frame that is `broken` (opaque) declares nothing: the `literal_ref` term of a later `terms` frame of its digest has no candidate, `missing_dependency` with `value` at the `terms` frame (P4 5, P4-54, P3-92) |

Totals: built and matches: 96 (96 entries).
