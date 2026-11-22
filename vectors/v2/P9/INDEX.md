# Vectors of P9

| vector | outcome | notes |
|---|---|---|
| `convert/example-9-1` | built and matches | 9.1: 50 statements, the three entries, the one binding, the blob-set entry bytes, source_signatures 0 |
| `convert/minimal` | built and matches | a header, one terms, one quads of one row, written by the repository's own writer |
| `convert/compaction-equal` | built and matches | the result of convert/example-9-1 compacted again by P7: the same bytes |
| `convert/total-no-option` | built and matches | two runs equal; an engine without the encoder: the same head and proof |
| `convert/two-segments-blank` | built and matches | two segments each with the blank node label b and a line: two nodes |
| `convert/anonymous-blank` | built and matches | two terms {k 2} and {k 2, v ""} in one segment: two nodes |
| `convert/blank-same-label-twice` | built and matches | two term entries {k 2, v b} in one segment: one node |
| `convert/datatype-default` | built and matches | a literal with datatype xsd:string, a lang_literal with direction absent, a lang_literal with direction ltr or rtl |
| `convert/language-case` | built and matches | x@EN and x@en with no suppression: two language literals, languages EN and en, two lines, the proof holds |
| `convert/language-case-suppressed` | built and matches | a term target on one of two terms that differ in the case of their tag: converted, the line of the other stays visible, the proof holds |
| `convert/language-datatype-generated` | built and matches | l en with dt naming rdf:langString; l en and dir ltr with dt naming rdf:dirLangString: a lang_literal each |
| `convert/language-datatype-other` | built and matches | l en with dt naming xsd:string; l en with dt naming rdf:dirLangString; l en and dir ltr with dt naming rdf:langString: unrepresentable_term at the start of the frame |
| `convert/language-not-a-tag` | built and matches | l 'not a tag': a lang_literal with that language, unchanged |
| `convert/typed-literal-langstring-iri` | built and matches | v x with dt naming the IRI rdf:langString and no l: a literal with that datatype, distinct from the language literal x@en |
| `convert/typed-fields-values` | built and matches | a header meta holding 1, -2, true and h'ff', a frame pub holding 0 and a blob target: pairs of VALUES for each and for the ordinals |
| `convert/text-literal-no-value` | built and matches | a source term 5 of xsd:integer used by a row and no integer 5 elsewhere: a literal with no pair of VALUES, stored as text |
| `convert/text-literal-meets-value` | built and matches | the source term 1 of xsd:integer used by a row, in a source whose meta holds the integer 1: one term with a pair of VALUES from the meta value |
| `convert/direction-without-language` | built and matches | {k 1, v x, dir ltr}: unrepresentable_term |
| `convert/annotation-count` | built and matches | an annot frame with two equal rows and a second frame with one more: one line, count 3 |
| `convert/reifies-multi` | built and matches | two reifies rows with one reifier and different triples: two reifier_binding lines and two triple terms |
| `convert/triple-term-with-rf` | built and matches | a k 3 term with tt and rf: one triple term; rf by a TermNote |
| `convert/legacy-triple-term` | built and matches | a k 3 term with rf only; with neither: unrepresentable_term |
| `convert/triple-subject` | built and matches | a quads row whose subject is a k 3 term: unrepresentable_term at the quads frame |
| `convert/literal-subject` | built and matches | a quads row whose subject is a literal: unrepresentable_term |
| `convert/snapshot` | built and matches | a snapshot frame with terms, quads, blobs and meta: the images of its sections in one scope with its segment |
| `convert/blob-inline-multi-unit` | built and matches | a blob of 300000 bytes (above the chunker's max 262144) named [blake3_256, H_ctx(unit, bytes)] |
| `convert/blob-pub-digest-mismatch` | built and matches | an inline blob whose pub.digest is not its digest: source_damaged |
| `convert/blob-external` | built and matches | an external blob with mt and rep: a gtsc:ExternalBlob node, no binding, no blob-set entry |
| `convert/blob-merged-pub` | built and matches | two blob frames of one digest, overlapping pub maps: one blob, one node, the later value of each key; held and external is held |
| `convert/blob-digest-forms` | built and matches | a suppress blob target and a pub.digest of a held blob, each as a byte string and as the generated digest text: the same 32 bytes in each statement |
| `convert/blob-digest-text-other` | built and matches | a held blob whose pub.digest is the digest text with upper-case digits, and one whose text has 63 digits: source_damaged each (the text equals no generated digest text) |
| `convert/blob-digest-text-external` | built and matches | an external blob and a suppress blob target with a digest text that equals the digest text of no held digest: no gtsc:digest, gtsc:digestText holds the text whole, the target names the external node |
| `convert/suppress-target-member-types` | built and matches | suppress targets of each kind with a member of a type outside the lists of 5.8, and a frame target whose id is a byte string of 31 bytes: stop unrepresentable_frame each |
| `convert/blob-external-no-digest` | built and matches | an external blob with no digest: unrepresentable_frame |
| `convert/digest-literal` | built and matches | a literal blake3:<hex> in a quad: one ordinary literal, no binding |
| `convert/suppress-term-quad-reifier` | built and matches | one suppress frame with a target of each of the three kinds: three entries |
| `convert/suppress-frame-earlier` | built and matches | a frame target on an earlier quads frame of an earlier segment: one quad entry for each distinct row |
| `convert/suppress-frame-later` | built and matches | a frame target that names a later frame (of a later segment): the same entries |
| `convert/suppress-frame-unresolved` | built and matches | a frame target that names no frame: no entry; the proof holds |
| `convert/suppress-blob-external` | built and matches | a blob target that names an external blob only: one term entry on its node |
| `convert/suppress-reason-by` | built and matches | a suppress frame with reason and by and targets of the five kinds: the node Y, a node for each target, no binding naming a frame, the same result after compaction |
| `convert/suppress-unknown-kind` | built and matches | a target of kind graph: unrepresentable_frame |
| `convert/header-extension` | built and matches | a header with an extension key and layout: gtsc:headerExtension and gtsc:layout statements |
| `convert/meta-value-types` | built and matches | a meta frame holding a float, a byte string, null, undefined, a tagged value, a bignum, a nested map and array: the term and statements of V for each |
| `convert/pub-frames` | built and matches | frames of types terms, quads, suppress, index and meta each with a pub (one nested, with a float and a tag): a gtsc:Frame node for each |
| `convert/pub-blob-not-map` | built and matches | a blob frame whose pub is an array: unrepresentable_frame |
| `convert/key-structure` | built and matches | frames that hold id, prev, x, sig and a header cat and dct: none in the result; source_signatures counts the sig |
| `convert/key-extension` | built and matches | extension keys in the header, a codec, a frame, a snapshot payload, a suppress payload and a target, and a term map: one statement group for each |
| `convert/term-ignored-keys` | built and matches | a triple term with tt and rf, an IRI with l: a gtsc:TermNote for each; the terms unchanged |
| `convert/to-recipients` | built and matches | an unencrypted frame with to: F gtsc:to V(value) |
| `convert/encrypted-frame` | built and matches | a frame whose chain has an encrypt stage and no key: source_not_decodable |
| `convert/profile-declared` | built and matches | headers with prof evidence, files, stream, opaque and music-package: result profile absent; gtsc:profile states the text; the files# rows unchanged |
| `convert/meta-not-map` | built and matches | a meta frame whose payload is an array: unrepresentable_frame |
| `convert/signatures` | built and matches | a source whose frames carry two sig: source_signatures 2; no signature in the result |
| `convert/index-frame` | built and matches | a source with an index frame: no frame for it; the proof holds |
| `convert/index-mmr-wrong` | built and matches | an index frame with a wrong mmr: source_damaged at the frame |
| `convert/torn-tail` | built and matches | a source that ends inside an item: source_damaged at the incomplete item |
| `convert/unknown-codec` | built and matches | a frame with a codec outside the baseline of S1 8.4: source_not_decodable |
| `convert/opaque-frame` | built and matches | an explicit opaque frame: unrepresentable_frame |
| `convert/unknown-frame-type` | built and matches | a frame with t zzz: unrepresentable_frame |
| `convert/forward-reference` | built and matches | a quads row naming an id defined later: source_damaged |
| `convert/not-first-generation` | built and matches | a file of this generation as the source; zero bytes: source_damaged, offset 0 |
| `convert/capacity` | built and matches | a source whose fold the converter's labelling refuses (the Paley graph on 13 vertices, labelling_units 0): source_exceeds_capacity, offset 0 |
| `convert/capacity-size` | built and matches | a source whose fold has a size above the converter's labelled_lines: source_exceeds_capacity, offset 0; a size equal to it is labelled (sub-minimum capacities as a test device) |
| `convert/capacity-items` | built and matches | a source with one item more than the converter's items_per_read: source_exceeds_capacity, offset the start of the item after the last that fits |
| `convert/capacity-total` | built and matches | a source whose items' decoded payloads sum to above the converter's decoded_bytes_per_read: source_exceeds_capacity, offset the start of the item that brings the sum above it |
| `convert/order-reads-first` | built and matches | a source with a damaged item whose second read returns one item fewer: source_not_deterministic, offset 0 (section 6 step 1, 8.3 item 1 first) |
| `convert/order-earlier-item-wins` | built and matches | a prev that breaks at the third item and an item above decoded_bytes_per_frame at the fifth; the two swapped; one item both: the stop of the earlier item, and the capacity test first for one item |
| `convert/language-case-suppressed-two` | built and matches | two pairs x@EN, x@en and y@EN, y@en in two frames, term targets on one of each: converted (a tag is copied, 5.2 item 4), the other lines stay visible |
| `convert/suppress-frame-twin-segments` | built and matches | two byte-identical segments and a frame target on their frame identifier: the union of the entries of the two frames, one entry for each distinct row |
| `convert/map-key-order` | built and matches | a meta map whose keys differ in the length of their encodings and in a shared prefix: entries created in ascending bytewise order of the encodings; the same result for another listing order |
| `convert/content-mismatch` | built and matches | a faulty engine that gives step 5 a Dr that differs from Ds: content_mismatch, offset 0 |
| `convert/result-frame-above-capacity` | built and matches | a source of small items whose M(SF) has 300 named graphs: the canonical index frame is above a converter capacity set below it (a sub-minimum capacity, as the P7 capacity vectors use): source_exceeds_capacity, offset 0 |
| `convert/zstd-dictionary` | built and matches | a frame under zstd with a dct entry in the header's catalogue (a trained dictionary named by the frame): converted; proof holds (Q: the package refuses such a frame) |
| `convert/term-target-datatype` | built and matches | a term target on a datatype IRI in use: converted; no line of those literals hidden (C1) |
| `convert/quad-target-literal-subject` | built and matches | a suppress frame with a reason and a quad target whose subject is a literal: unrepresentable_term at the frame |
| `convert/blank-label-not-utf8` | built and matches | a blank node label 61 ff 62 used in two rows through two term entries: converted, one node |
| `convert/text-not-utf8-term` | built and matches | a terms frame holding {k 0, v 61 ff 62}: unrepresentable_term at the frame |
| `convert/text-not-utf8-meta` | built and matches | a meta frame holding a value that is the text string 61 ff 62: unrepresentable_term at the frame |
| `convert/storage-differs` | built and matches | a conversion by an engine without the reference encoder, then a read by a reader that holds it: storage differs, no finding; without it: unchecked with encoder_not_held, no finding |
| `convert/later-header-v2` | built and matches | a second segment whose header has v 2: source_damaged at that header's offset |
| `convert/compaction-stop-held` | built and matches | a faulty engine whose fold holds a held row (compaction stops with opaque_term): content_mismatch, offset 0 |
| `convert/compaction-stop-labelling` | built and matches | a source whose labelled set the converter's labelling refuses (the cube graph, labelling_units 0): source_exceeds_capacity, offset 0 |
| `convert/compaction-stop-blob` | built and matches | a faulty engine whose blob does not rebuild to its name (compaction stops with damaged_frame): content_mismatch, offset 0 |
| `convert/prof-not-text` | built and matches | a header with prof 1: unrepresentable_frame, offset 0 |
| `convert/nondeterministic` | built and matches | two sources presented on two reads: source_not_deterministic, offset 0 |
| `proof/check-holds` | built and matches | source, result and proof of convert/example-9-1: each of the four checks of 7.2 holds |
| `proof/check-tampered-result` | built and matches | the result of a source with one literal changed, against the proof of convert/example-9-1: check 3 of 7.2 fails |
| `proof/check-tampered-source` | built and matches | the source with one byte changed: check 1 of 7.2 fails |
| `reader/second-only` | built and matches | a first-generation file read by a second_only reader: unsupported_generation, no segment |
| `reader/first-and-second` | built and matches | the same file read by a first_and_second reader: file_result variant first_generation, the first-generation reader's result unchanged |
| `reader/mixed` | built and matches | a first-generation file followed by a file of this generation: the first-generation reader's result; the trailing items are its own diagnostics |
| `reader/header-late` | built and matches | a file of this generation with a first-generation header after its first segment: an opaque item with wrong_type; no segment begins |
| `reader/version-two-map` | built and matches | a first item {v 2}: unsupported_generation from either claim |
| `convert/deep-value` | built and matches | a first-generation source whose meta value nests arrays deep: the value that V gives at depth 1 for the same shape, carried whole, no stop (3.3, case 9). The whole conversion (reading, M(SF), labelling, canonical file, proof and the four checks) runs at the depth of the entry, 1000 |
| `convert/capacity-rebuilt` | built and matches | a source whose held blobs share units, so that the recipes of the result have a total rebuilt length T, converted with rebuilt_bytes_per_read T - 1 and with T: stop source_exceeds_capacity, offset 0; with T, a result and a proof (P9-42) |
| `convert/compaction-stop-formed` | built and matches | a faulty engine whose fold comes from a read that refused a terms frame by the total formed length: stop content_mismatch, offset 0 |
| `convert/text-null-iri` | built and matches | {k 0, v null} used by a row: unrepresentable_term at the offset of the frame (3.3 case 1: a null is not text) |
| `convert/text-null-blank` | built and matches | {k 2, v null}: unrepresentable_term, not an anonymous node (3.3 case 1) |
| `convert/text-null-language` | built and matches | {k 1, v x, l null}: unrepresentable_term (3.3 case 1) |
| `convert/text-null-direction` | built and matches | {k 1, v x, l en, dir null}: unrepresentable_term (3.3 case 1) |
| `convert/text-null-datatype` | built and matches | {k 1, v x, dt null}: unrepresentable_term (3.3 case 1) |
| `convert/text-absent-kind` | built and matches | a term without k: unrepresentable_term (3.3 case 1) |
| `convert/text-empty-iri` | built and matches | {k 0, v empty} used by a row: converted, an iri of empty text (3.3 case 1) |
| `convert/text-empty-language` | built and matches | {k 1, v empty, l empty}: a lang_literal with empty lexical form and empty language (3.3 case 1) |
| `convert/prof-absent` | built and matches | a header with no prof: converted; S gtsc:profile generic; the result's profile absent (3.3 case 6) |
| `convert/prof-empty` | built and matches | a header with prof empty: converted; S gtsc:profile with the empty text (3.3 case 1) |
| `convert/prof-null` | built and matches | a header with prof null: unrepresentable_frame (3.3 case 1) |
| `convert/dt-beside-language-text` | built and matches | l en with dt naming the IRI that is the generated datatype by text only: a lang_literal; any other dt beside l is unrepresentable_term (3.3 case 2) |
| `convert/snapshot-ids-after` | built and matches | a snapshot after three terms, then a quads frame naming the ids of snapshot terms shifted by three: the rows resolve to the snapshot terms; dt, tt and the snapshot's rows are shifted by three (3.3 case 3) |
| `convert/snapshot-label-one-scope` | derived from the text | the label b in a snapshot and in a frame of its segment: two nodes, two lines (3.3 case 3) |
| `convert/meta-order-snapshot` | built and matches | a meta frame {a 1}, a snapshot whose meta is {a 2, b 3}, a meta frame {b 4}: the segment meta {a 2, b 4} (3.3 case 4) |
| `convert/meta-key-not-text` | built and matches | a meta map with an integer key: unrepresentable_frame (3.3 case 4) |
| `convert/meta-replace-whole` | built and matches | {m {x 1}} then {m {y 2}}: m holds {y 2} only, no map is merged into a map (3.3 case 4) |
| `convert/float-hash-finite` | built and matches | a frame with pub holding 1.5 and -0.0 (written in half width), its id over the 64-bit forms: converted (3.3 case 5(a)) |
| `convert/float-hash-shortest` | built and matches | the same frame with its id over the shortest forms (f9 3e00): converted; M(SF) equal to that of convert/float-hash-finite (3.3 case 5(a)) |
| `convert/float-hash-mixed` | built and matches | a frame with pub holding 1.5 and 2.5, its id over 1.5 in 64 bits and 2.5 in half width: source_damaged, rule 3 (3.3 case 5(a)) |
| `convert/float-hash-special` | built and matches | a frame holding NaN with a payload, +inf and -inf, its id over f9 7e00, f9 7c00, f9 fc00: converted (3.3 case 5(a)) |
| `convert/float-nan-two-images` | built and matches | two sources equal but for one meta value, the half-width NaN f9 7e00 and f9 fe01, each id over f9 7e00: both converted; the literals 7FF8000000000000 and FFF8040000000000; different content digests (3.3 case 5(b)) |
| `convert/stop-layout-header-offset` | built and matches | a segment claiming layout streamable and breaking it, and in the same segment a later terms frame with a k outside 0 to 3: source_damaged at the header offset (3.3 case 7) |
| `convert/stop-raise-order` | built and matches | one source read through two first-generation readers that raise the diagnostics of an item in opposite orders (a test double that reverses the raise order only): the same stop, reason and offset (3.3 case 7) |
| `convert/stop-lowest-rule` | built and matches | one frame holding a row of a wrong length (rule 13) and a k 3 term without tt (rule 9): unrepresentable_term; the rule of the lowest number that holds anywhere in the item (3.3 case 7) |
| `convert/snapshot-label-two-snapshots` | derived from the text | the label b in two snapshots, in two different lines and in two byte-identical snapshots: two nodes each (3.3 case 3) |
| `convert/snapshot-blank-same-label-twice` | built and matches | two term entries {k 2, v b} in one snapshot, named by two rows: one node, two lines |
| `convert/anonymous-blank-snapshots` | built and matches | the terms {k 2} and {k 2, v ""} in a frame of the segment and in each of two snapshots: six nodes, six lines |
| `convert/snapshot-triple-over-blank` | derived from the text | a blank term x and a triple term over it in a frame of the segment and in a snapshot: two triple terms over two nodes, two lines |
| `convert/suppress-term-of-segment-node` | derived from the text | a term target on the entry of b in the segment, and b in a snapshot: the line of the segment node is hidden, the line of the snapshot node is visible |
| `convert/storage-two-converters` | built and matches | one source converted with and without the reference encoder: the same proof and head, the stored bytes differ; a holder reads storage verified or differs, a reader without it unchecked, no finding |

Totals: built and matches: 124; derived from the text: 4 (128 entries).
