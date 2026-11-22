# Vectors of P3

| vector | outcome | notes |
|---|---|---|
| `terms-six-terms` | built and matches |  |
| `terms-forms-and-values` | built and matches | 12.1, the second payload: the languages en, en, fr, fr; the directions absent, ltr, absent, absent; two pairs of VALUES |
| `terms-restart-group-one` | built and matches |  |
| `terms-prefix-splits-character` | built and matches | é is c3 a9 and è is c3 a8: the common prefix of 10 bytes ends inside the second character |
| `terms-text-not-utf8` | built and matches |  |
| `terms-prefix-not-maximal` | built and matches |  |
| `terms-prefix-at-restart` | built and matches |  |
| `terms-prefix-too-long` | built and matches |  |
| `terms-suffix-total` | built and matches |  |
| `terms-id-base-mismatch` | built and matches | the third frame's id_base counts the id the opaque second frame reserved |
| `terms-id-base-after-opaque` | built and matches | the first frame is damaged (payload_digest, P1); its runs are well formed and reserve its ids |
| `terms-runs-empty` | built and matches |  |
| `terms-run-count-zero` | built and matches |  |
| `terms-runs-adjacent-same-kind` | built and matches |  |
| `terms-run-kind-unregistered` | built and matches |  |
| `terms-order-unregistered` | built and matches |  |
| `terms-form-unregistered` | built and matches |  |
| `terms-value-kind-unregistered` | built and matches |  |
| `terms-block-tag-unregistered` | built and matches |  |
| `terms-id-above-uint` | built and matches | P3 2.1 AN ID IS A uint and P3-5: the first frame's run of 2^64 - 1 reserves the ids 0 to 2^64 - 2 (its payload cannot hold them: its own finding is inconsistent_counts, and it reserves them all the same, P3-6); the second, whose last id would be 2^64, has runs not well formed (out_of_range) and reserves no id; the third is judged at the expected base 2^64 - 1 and is folded; no count of the result is above 2^64 - 1 |
| `terms-language-case-distinct` | built and matches | P3 1, WHAT THE FORMAT DOES NOT JUDGE: two language texts that differ in the case of a letter are two languages; two terms, two lines, no finding, and the content digest of those two lines |
| `terms-restart-group-zero` | built and matches |  |
| `terms-blocks-count` | built and matches |  |
| `terms-block-list-length` | built and matches |  |
| `terms-block-tag-kind` | built and matches |  |
| `terms-digest-length` | built and matches |  |
| `terms-form-list-counts` | built and matches | the last case: forms holds one element for a run of two, judged before the (empty) datatypes |
| `terms-language-table` | built and matches |  |
| `terms-language-table-repeat` | built and matches |  |
| `terms-language-table-unused` | built and matches |  |
| `terms-language-table-order` | built and matches |  |
| `terms-language-index-range` | built and matches |  |
| `terms-language-table-both-blocks` | built and matches |  |
| `terms-language-case` | built and matches |  |
| `terms-language-not-a-tag` | built and matches |  |
| `terms-language-empty` | built and matches |  |
| `terms-iri-not-an-iri` | built and matches |  |
| `terms-literal-forms` | built and matches |  |
| `terms-datatype-text-equals-generated` | built and matches | the typed literal of the datatype text rdf:langString and the language literal x@en are two terms |
| `terms-typed-datatype-xsd-string` | built and matches |  |
| `terms-lang-literal-uses-no-iri` | built and matches |  |
| `terms-datatype-forward` | built and matches |  |
| `terms-triple-forward` | built and matches |  |
| `terms-datatype-not-iri` | built and matches |  |
| `terms-triple-predicate-not-iri` | built and matches |  |
| `terms-triple-subject-literal` | built and matches |  |
| `terms-triple-component-unreadable` | built and matches | the triple is unreadable through its component: no second finding (2.8 item 4) |
| `terms-literal-ref-text` | built and matches |  |
| `terms-literal-value-ref` | built and matches | the three storage forms give one term, the typed literal FF0041 of hexBinary, six characters |
| `terms-value-each-kind` | built and matches |  |
| `terms-value-time-range` | built and matches |  |
| `terms-value-equals-text` | built and matches |  |
| `terms-value-not-read-from-text` | built and matches | the text 07 is a term of its own; no value is read from it |
| `terms-literal-ref-not-utf8` | built and matches |  |
| `terms-literal-ref-value-missing` | built and matches | rows name both terms to show them unreadable: held, opaque_term once (4.3) |
| `terms-literal-ref-rebuild-fails` | built and matches |  |
| `terms-literal-ref-length-differs` | built and matches |  |
| `terms-duplicate-arrival` | built and matches |  |
| `terms-duplicate-canonical` | built and matches |  |
| `terms-duplicate-canonical-kinds` | built and matches |  |
| `terms-canonical-kind-order` | built and matches |  |
| `terms-canonical-iri-order` | built and matches |  |
| `terms-canonical-literal-order` | built and matches | the datatypes stand in an arrival frame with ids against their text order (d2 is id 0, d1 is id 1): a datatype is compared by its IRI text |
| `terms-canonical-literal-ref-order` | built and matches | two values; the order is digest, then length, then form (typed before language) |
| `terms-canonical-literal-value-order` | built and matches |  |
| `terms-canonical-literal-value-ref-order` | built and matches |  |
| `terms-canonical-blank-order` | built and matches |  |
| `terms-canonical-triple-depth` | built and matches | ids 3 and 4 have depth 1, id 5 (object id 3) depth 2; the swapped case exchanges the two of depth 1 |
| `terms-canonical-order-ties` | built and matches |  |
| `terms-canonical-value-order` | built and matches |  |
| `terms-canonical-across-frames` | built and matches |  |
| `terms-unreadable-first-condition` | built and matches | term 0: a lexical form not valid UTF-8 and a forward datatype: ill_formed_term only; term 1: forward_reference |
| `terms-canonical-follows-opaque` | built and matches | the third frame's nearest earlier terms frame is not decoded: no earlier term to follow |
| `terms-literal-ref-first-candidate` | built and matches | P3 2.6: a candidate that fails keeps the blobs part's finding at its own position in either order of the frames, and the term is readable with the octets of the one that rebuilds |
| `terms-literal-ref-all-fail` | built and matches |  |
| `terms-order-record-before-payload` | built and matches | restart_group 0 (step 1) before a prefix_lengths of one element for two terms (step 3) |
| `terms-order-type-before-count` | built and matches | a list too short in the first block (step 3) and a block tag 9 in the second (step 2) |
| `terms-order-count-sequence` | built and matches | the suffix lengths sum to 2 for 3 bytes, and term 1 is a restart (g 1) with prefix 1 |
| `quads-four-rows` | built and matches |  |
| `rows-row-count-zero` | built and matches |  |
| `rows-graph-run-zero` | built and matches |  |
| `rows-run-count-zero` | built and matches |  |
| `rows-value-negative` | built and matches |  |
| `rows-value-above-range` | built and matches | the subjects decode to 2^64 - 1 and then 2^64 |
| `annotations-count-zero` | built and matches |  |
| `rows-graph-runs-sum` | built and matches |  |
| `rows-column-length` | built and matches |  |
| `rows-run-column-sum` | built and matches |  |
| `rows-graph-runs-adjacent-equal` | built and matches |  |
| `rows-run-column-adjacent-equal` | built and matches |  |
| `rows-payload-wrong-record` | built and matches |  |
| `rows-order-unregistered` | built and matches |  |
| `rows-forward-reference` | built and matches |  |
| `rows-opaque-term-held` | built and matches | two rows name the unreadable term (id 1): opaque_term once |
| `rows-position-subject-literal` | built and matches |  |
| `rows-position-predicate-blank` | built and matches |  |
| `rows-position-graph-literal` | built and matches |  |
| `rows-position-reifier-literal` | built and matches |  |
| `reifier-bindings-triple-not-triple` | built and matches |  |
| `rows-repeat-collapse` | built and matches |  |
| `rows-repeat-after-resolution` | built and matches |  |
| `annotations-repeat` | built and matches |  |
| `annotations-repeat-after-resolution` | built and matches | ids 4 and 9 are one IRI: equal after resolution (judged on the wire identity) |
| `annotations-counts-add` | built and matches |  |
| `annotations-count-overflow` | built and matches | both frames are in one segment, so the segment's own fold and the file's fold each report the refusal |
| `empty-graphs-column` | built and matches |  |
| `empty-graph-left-by-quad` | built and matches |  |
| `empty-graph-left-by-reifier-binding` | built and matches |  |
| `empty-graph-left-by-annotation` | built and matches |  |
| `empty-graph-left-later` | built and matches |  |
| `empty-graph-left-hidden-carrier` | built and matches | the carrying quad is hidden; the empty_graph line stays out of the fold |
| `empty-graph-stays-on-mention` | built and matches |  |
| `rows-order-canonical-ok` | built and matches |  |
| `rows-order-canonical-violated` | built and matches | each frame still folds: its lines are in the fold |
| `rows-order-canonical-two-runs` | built and matches | a canonical frame with one run for each of two graphs, the default graph first |
| `rows-order-canonical-across-frames` | built and matches |  |
| `rows-order-unordered` | built and matches |  |
| `fold-two-segments` | built and matches |  |
| `fold-segment-order` | built and matches | in the order S2, S1 the nodes (0, b) and (1, b) exchange segments: the folds are isomorphic and the lines equal as sets; a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `fold-blank-scope` | built and matches |  |
| `fold-equal-headers-two-scopes` | built and matches | a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `fold-unfolded-frame` | built and matches |  |
| `labelling-source-identifier` | built and matches | E([0, b]) is before E([1, b]) bytewise |
| `labelling-no-blank` | built and matches |  |
| `labelling-refusal-width` | built and matches |  |
| `record-binding-payload` | built and matches |  |
| `record-suppression-payload-each-kind` | built and matches | the fold holds a quad in g with a triple object, a reifier binding, an annotation, the blob of 'abc' and a binding on s |
| `record-blob-declaration-payload` | built and matches |  |
| `record-optional-slot-absent` | built and matches |  |
| `record-optional-slot-present` | built and matches |  |
| `record-unnamed-slot` | built and matches |  |
| `record-triple-slot` | built and matches | the subject pair names S1's node (b) and the object pair names no node: the triple leaf is unnamed as a whole (6.3) |
| `record-equal-collapse` | built and matches | S1 names its node by id 8, S2 by the pair (head of S1, b): one entry, one record |
| `record-pair-two-segments` | built and matches | the two byte-identical segments share their head: the pair has ONE resolution, the node of the first of them, one record line (P3 6.3); a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `record-pair-no-segment` | built and matches |  |
| `record-pair-no-label` | built and matches | a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `record-node-only-in-records` | built and matches |  |
| `record-graph-only-in-record` | built and matches |  |
| `record-hidden-in-set` | built and matches |  |
| `bindings-basic` | built and matches |  |
| `bindings-count-mismatch` | built and matches |  |
| `bindings-referent-tag-unregistered` | built and matches |  |
| `bindings-leaf-tag-unregistered` | built and matches |  |
| `bindings-algorithm-unregistered` | built and matches |  |
| `bindings-digest-length` | built and matches |  |
| `bindings-triple-leaf-no-pair` | built and matches |  |
| `bindings-forward-reference` | built and matches |  |
| `bindings-opaque-term` | built and matches |  |
| `bindings-duplicate` | built and matches |  |
| `bindings-unnamed-pair` | built and matches |  |
| `bindings-blank-in-no-line` | built and matches |  |
| `bindings-other-segment-pair` | built and matches |  |
| `bindings-referent-blob-resolved` | built and matches |  |
| `bindings-referent-blob-unresolved` | built and matches | two bindings with one referent: one finding |
| `bindings-referent-frame-resolved` | built and matches | the referent is the effective identifier of the terms frame |
| `bindings-referent-frame-unresolved` | built and matches |  |
| `bindings-referent-state-resolved` | built and matches |  |
| `bindings-referent-state-unresolved` | built and matches |  |
| `bindings-referent-state-failing-commit` | built and matches | the commit's root is false (P5 commit_root_mismatch): it stands in the file and names no state |
| `bindings-referent-other-segment` | built and matches | a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives; the later segment has a header without lineage, so that its identifiers are known before the earlier one is written |
| `state-prefix-fold` | built and matches | the state named by the second commit is the fold of the prefix that ends with it (7.2): it holds the line of the first segment |
| `content-digest-empty-fold` | built and matches |  |
| `content-digest-lines-only` | built and matches |  |
| `content-digest-with-bindings` | built and matches |  |
| `content-digest-with-suppression` | built and matches |  |
| `content-digest-with-blobs` | built and matches |  |
| `content-digest-hidden-counted` | built and matches |  |
| `content-digest-order-free` | built and matches | the second file swaps the segments and the two row frames of the first |
| `content-digest-rename-no-binding` | built and matches |  |
| `content-digest-rename-twin-pair-binding` | built and matches |  |
| `content-digest-rename-twin-pair-binding-renamed` | built and matches |  |
| `content-digest-rename-twin-pair-entry` | built and matches |  |
| `content-digest-rename-ordinals` | built and matches | a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `content-digest-absent-held` | built and matches |  |
| `content-digest-absent-refused` | built and matches |  |
| `content-digest-blob-lengths-order` | built and matches |  |
| `content-digest-own-fold-value-elsewhere` | built and matches |  |
| `suppress-term` | built and matches |  |
| `suppress-quad` | built and matches |  |
| `suppress-reifier` | built and matches |  |
| `suppress-graph` | built and matches |  |
| `suppress-blob` | built and matches |  |
| `suppress-reifier-binding` | built and matches |  |
| `suppress-annotation` | built and matches |  |
| `suppress-binding` | built and matches |  |
| `suppress-unmatched-term` | built and matches |  |
| `suppress-unmatched-quad` | built and matches |  |
| `suppress-unmatched-reifier` | built and matches |  |
| `suppress-unmatched-graph` | built and matches |  |
| `suppress-unmatched-blob` | built and matches |  |
| `suppress-unmatched-reifier-binding` | built and matches |  |
| `suppress-unmatched-annotation` | built and matches |  |
| `suppress-unmatched-binding` | built and matches |  |
| `suppress-unmatched-once` | built and matches |  |
| `suppress-unnamed-leaf-matches-nothing` | built and matches |  |
| `suppress-12-4` | built and matches |  |
| `suppress-term-hides-binding` | built and matches |  |
| `suppress-term-hides-triple-component` | built and matches |  |
| `suppress-term-datatype-unmatched` | built and matches |  |
| `suppress-term-hides-graph-name-lines` | built and matches |  |
| `suppress-graph-not-mention` | built and matches |  |
| `suppress-annotation-any-count` | built and matches |  |
| `suppress-reifier-all-graphs` | built and matches |  |
| `suppress-record-line-not-hidden` | built and matches |  |
| `suppress-blob-hides-declaration-keeps-set` | built and matches |  |
| `suppress-blank-other-segment` | built and matches | the node is in a later segment; a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-blank-pair-earlier-segment` | built and matches |  |
| `suppress-triple-leaf` | built and matches |  |
| `suppress-triple-leaf-no-blank` | built and matches |  |
| `suppress-entry-count-mismatch` | built and matches |  |
| `suppress-tag-unregistered` | built and matches |  |
| `suppress-digest-length` | built and matches |  |
| `suppress-forward-reference` | built and matches |  |
| `suppress-opaque-term` | built and matches |  |
| `suppress-order-free` | built and matches |  |
| `suppress-no-revive` | built and matches |  |
| `suppress-monotone` | built and matches |  |
| `suppress-unauthenticated` | built and matches | no frame is authenticated without the trust inputs of P6: every folded suppress frame counts |
| `suppress-not-folded` | built and matches |  |
| `suppress-unknown-codec` | built and matches | codec 9 under a header of a later registry version is an unknown codec (P1 3.3), not a violation: the frame is not folded; the later version also gives unsupported_registry_version (P1) |
| `suppress-visible-named-graphs` | built and matches |  |
| `record-value-payload` | built and matches | five pairs of VALUES, one of each kind: payloads 82 04 82 0k 81 01, a term list of the one literal |
| `fold-values-union` | built and matches | a typed value in two segments and the same literal as text in a third: one pair of VALUES, equal digests in every order; a second segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `fold-values-unused-term` | built and matches | the literal of [2, 7] is named by no row: its pair and its record are in the labelled set, the literal in the canonical dictionary |
| `labelling-size-values` | built and matches | size 3 = one row + a literal_value run of 2, at labelled_lines 2: refused only through the run (P3 6.5, the fourth addend) |
| `content-digest-value-is-content` | built and matches | the same line; the first holds the literal as [2, 7] (one value record more), the second as text only |
| `suppress-term-keeps-value` | built and matches | a term entry naming the literal of [2, 7]: the line hidden; the pair of VALUES and its record kept |
| `record-pair-own-segment` | built and matches | the entry names, by a frame of its own segment and a label that segment holds, a node of its own segment: the segment is excluded as holder (6.3), the pair is unnamed |
| `record-pair-header-identifier` | built and matches | the header identifier is the identifier of no frame (6.3) |
| `record-pair-transparent-frame` | built and matches | a parity frame is transparent: held by no segment (6.3) |
| `record-pair-broken-link` | built and matches | both the stored identifier of a frame whose link is broken and the identifier it should have are unnamed (6.3) |
| `record-pair-healed-link` | built and matches | the first pair names the computed identifier of the healed frame and is named (its node hidden); the second names the damaged stored identifier and is unnamed |
| `record-pair-shared-frame-first-holder` | built and matches | the first holder (segment 0) lacks the label; the second (segment 1) holds the frame and the label and is not searched; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `record-pair-label-in-holder-only` | built and matches | the label is defined in the first holder: named, its node is hidden; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-pair-order-differing-segments` | built and matches | two segments share their first frame, differ afterwards and both define b; an entry in a third naming the shared frame: in each order the node of the first segment of that order is hidden, and the content digests differ; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-expansion-family` | built and matches | two byte-identical base segments then a segment whose one entry nests the pair to depth d in the objects of triple terms: ONE record for the entry at every depth (not 2^(d+1)) and, from depth 2, d - 1 records of its nested terms (P3 6.2a); a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `labelling-size-family` | built and matches | the family at depth 6 has size 8 (two quads rows, one entry and five nested candidates): refused by size at labelled_lines 7, labelled at 8 with two lines, the record of the entry and five nested_term records (P3 6.5) |
| `labelling-size-guaranteed` | built and matches | three rows, no blank node: guaranteed; refused by size at labelled_lines 2 in the segment's own fold and in the file's |
| `labelling-size-own-fold` | built and matches | the segment's own fold has the size of the file's fold: both lists report it and both digests are absent |
| `suppress-append-terms` | built and matches | an entry whose label no terms frame of its holder defines, then the terms frame appended to the holder: unmatched before, hiding after; the digest differs; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-append-segment` | built and matches | an entry whose frame no segment holds, then a segment that holds it appended: unmatched before, named and hiding after; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-append-duplicate-segment` | built and matches | an entry that names a node of a segment, then a byte-identical copy appended: the first holder stays the holder; the hidden set and the record are unchanged; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-append-heal` | built and matches | the last frame's identifier field is damaged: it is broken (nothing follows) and the pair naming its computed identifier is unnamed; a frame appended heals it and the pair is named; the pair naming the stored identifier is unnamed in both; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `suppress-append-commit` | built and matches | two files (before of 587 bytes, after of 701 bytes, the first as a byte prefix of the second): the node hidden in both, no suppression_unmatched, equal content digests |
| `state-chained-lineage` | built and matches | the second commit's state holds the whole first segment: the lineage is checked |
| `state-no-lineage-local` | built and matches | the second segment's lineage is absent: its state is that segment alone; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `state-lineage-mismatch` | built and matches | a second segment with a lineage whose previous_head differs: its commit names no state, a referent to it is missing_dependency with state_unavailable |
| `state-lineage-ordinal` | built and matches |  |
| `state-lineage-previous-unreadable` | built and matches | the lineage of a segment after an unreadable header is not checked (P1-78) and its commit names no state |
| `state-lineage-chain-broken-middle` | built and matches | the third segment's lineage is checked but the second has no run: the third has no run |
| `state-failing-commit` | built and matches |  |
| `state-same-name-first-holder` | built and matches | a commit identifier that two segments with lineage absent share: the referent resolves to the first segment; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `state-same-name-first-holder-three` | built and matches | three segments, the second and third byte-identical with lineage absent: a referent to their commit identifier resolves to the second segment, whose ordinal the blank node carries; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `state-same-name-first-naming` | built and matches | the first segment's broken frame keeps its stored identifier, so both commits have one identifier; the commit of the first names no state, the referent resolves to the state the commit of the second names; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `state-broken-link-names-nothing` | built and matches | a frame whose link is broken before a valid commit: the commit names no state (7.2) |
| `state-payload-damaged` | built and matches | an intact link and a payload whose digest differs before a valid commit: the commit names no state |
| `state-payload-unchecked-capacity` | built and matches | a frame above decoded_bytes_per_frame before a valid commit: unchecked at the smaller capacity, no state; verified at the larger, a state; the two results differ in the capacity record |
| `state-earlier-segment-frame-broken` | built and matches | a frame of the first segment with a broken link after the first segment's last commit: the commit of the second segment names no state, the first commit does |
| `state-ordinals-kept` | built and matches | a state over two segments in a file whose first segment has another segment before it: the blank node is node (1, b), the node of the file's fold; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `state-same-name-different-prefix` | built and matches | two files of 463 bytes each (state_a, state_b), last segments of 125 bytes byte-identical, lineage absent: one effective identifier for the last commit, the state is the last segment alone and equal in the two files, the whole files differ in content digest |
| `state-repeated-segment` | built and matches | a file of 676 bytes: one segment twice, lineage absent: both commits have one effective identifier and name equal states; a referent resolves to the first segment |
| `state-chain-broken-names-nothing` | built and matches | two files of 338 bytes (good, altered), one byte differing in the first frame: the final commit has one identifier and is valid in both; it names a state in the first file and none in the second |
| `binding-undefined-and-unreadable` | built and matches | a term leaf of one id not defined at the frame and one id of an unreadable term: dropped with forward_reference, no opaque_term, content digest present |
| `suppress-undefined-and-unreadable` | built and matches | an entry with one id not defined and one id of an unreadable term: dropped with forward_reference, no opaque_term |
| `bindings-order-type-before-count` | built and matches | a referent tag outside the registry and an entry_count that differs: the first in the order of the bytes decides (1.4) |
| `binding-frame-referent-first` | built and matches | a frame referent whose digest is the identifier of an ordinary frame in two byte-identical segments resolves to the first; a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `content-digest-frame-order` | built and matches | the frames of a segment written in another order that 4.2 admits: equal digests |
| `content-digest-segment-order` | built and matches | the segments of a file in another order, no pair naming a frame that two different segments hold: equal digests (the ordinals are renamed); a segment whose header names no lineage begins a lineage (P5 lineage_begins), a finding of P5 that the input gives |
| `rows-order-record-before-payload` | built and matches | the content record (order 3) is judged before the payload (a null, not a record): the one malformed_value is the record's (P3 1.4 step 1) |
| `rows-order-type-before-count` | built and matches | a graph run of 0 rows (step 2, type) and a column of another length than row_count (step 3, count): the type violation decides |
| `rows-order-trailing-before-count` | built and matches | a byte after the one item (step 2) and a column of another length than row_count (step 3): the remaining byte decides |
| `rows-canonical-follows-opaque` | built and matches | a canonical frame follows an opaque frame of its type: it has no earlier row to follow, so its first row (smaller than the row of the first frame) breaks no declared order |
| `fold-delivery-order` | built and matches | quads of the default graph and of two graphs written in descending order, two bindings and three entries: delivered in the order of the canonical file (P7-39) |
| `fold-delivery-order-refused` | built and matches | the same kind of fold read with labelled_lines below its size: the blank nodes are compared bytewise on E([ordinal, label]) in the place of their positions |
| `canonical-dictionary-membership` | built and matches | terms of a hidden line, a datatype and a record-only blank node, each once |
| `canonical-dictionary-unnamed-pair` | built and matches |  |
| `canonical-blank-label` | built and matches | 300 blank nodes: the heads of E(position) at 23, 24, 255 and 256 |
| `canonical-literal-value-form` | built and matches |  |
| `canonical-lang-literal-datatype-absent` | built and matches |  |
| `canonical-lang-literal-digest-generated` | built and matches |  |
| `canonical-lang-literal-direction-digest` | built and matches |  |
| `canonical-rows-sort` | built and matches | 12.5 |
| `canonical-annotations-sum` | built and matches |  |
| `canonical-bindings-sorted` | built and matches |  |
| `canonical-suppress-frame` | built and matches | 12.5 |
| `canonical-suppress-no-finding` | built and matches |  |
| `canonical-pair-first-holder` | built and matches | the pair names a frame that two byte-identical segments hold: one resolution, one entry (P3 6.3) |
| `canonical-twin-positions` | built and matches | LEMMA I |
| `labelling-limit-guaranteed` | built and matches | labelling_units 0 on a guaranteed input |
| `labelling-limit-not-guaranteed` | built and matches | an input that is not guaranteed and whose walk has more limited units than the allowance, read with labelling_units 0 (refused) and with labelling_units equal to the limited units of its walk (labelled) |
| `canonical-pair-unnamed-stays` | built and matches | a pair that names a frame no segment holds, compacted: kept as written, still unnamed, equal content digests |
| `labelling-size-at-capacity` | built and matches | the family at depth 6 (size 8) with labelled_lines 8: labelled, with two lines, the record of the entry and five nested_term records |
| `labelling-width-and-limit` | built and matches | a fold that meets the refusal width and the refusal limit for a not guaranteed input: one labelling_refused, refusal_width |
| `suppress-entry-position` | built and matches | the entry before the line, after it, in the same segment and in another segment before or after: equal visible datasets |
| `terms-runs-order` | built and matches | P3-5: the empty list first; the runs in list order; in one run the kind against the registry, then against the run before it, then the count |
| `terms-front-term-zero-prefix` | built and matches | P3-12: term 0 has no term before it and is a restart; a prefix length other than 0 there is forbidden_combination and out_of_range is not judged for it |
| `terms-block-wrong-tag-counts` | built and matches | P3-12 last sentence, P3-8: the lists of a block are counted as the lists of the block's own tag against the count of its run, before P3-9 judges the tag |
| `terms-literal-ref-first-reaching-frame` | built and matches | P3 2.6: the finding stands at the first frame that names the digest among the terms that reach item 5 of 2.8; a term made unreadable by an earlier item has that finding alone |
| `terms-canonical-skipped-term` | built and matches | P3 2.9: a term whose key cannot be formed (a datatype id not smaller than its own id, or naming a non-iri) is compared with no term; the next is compared with the last term not skipped, in the frame or at the junction. The frames also hold the forward_reference or position_constraint of 2.8, which each case lists |
| `annotations-triple-components-written-alike` | built and matches | P3 3.3: two triple terms whose three components are written alike, component by component, are written alike although the ids differ: the two annotation rows differ only in the count of ids, so they are equal in every field other than the count |
| `labelling-held-fold` | built and matches | P3 8.1: a fold with a held item is labelled on the labelled set of what it holds; a refusal of that labelling is reported with its finding; the digest is absent in both cases |
| `bindings-missing-dependency-fold-only` | built and matches | P3 7.1: only the bindings of the fold count for missing_dependency of a referent: a dropped binding (forward_reference), a held one (opaque_term) and the referent inside a binding entry of a suppress frame give none; the finding of a referent stands at the first frame with a binding of the fold that has it |
| `state-damaged-region-before-commit` | derived from the text | P3 7.2: a damaged region takes an ordinary frame before a commit: reading resumes at the parity frame, the commit links against its anchor and is detached, so anchored_frames is 1, and the commit has commit_count_mismatch and commit_root_mismatch and names no state |
| `terms-record-field-order` | built and matches | P3 1.4 step 1: the content record of a terms frame is judged in the order id_base (P3-6), runs (P3-5), order (P3-7), restart_group (P3-6): the first violation is the one finding; an id_base that differs from the expected base wins over every violation of a field after it |
| `terms-record-field-order-newer-version` | built and matches | P3 1.4 step 1: the fields of a terms content record are judged in the same order in a segment of every registry version: an order of a newer version stands at its place after id_base and runs, so a violation of either is malformed_value and the frame is not unsupported_critical; an order of a newer version before a restart_group of 0 gives the outcome of P2 (unsupported_critical) |
| `rows-record-field-order` | built and matches | P3 1.4 step 1: the content record of quads, reifier_bindings and annotations is judged in the order of its fields, order (P3-7, P3-26) before row_count (P3-23): an order outside term_order with a row_count of 0 is unregistered_value; the control with a registered order and the same row_count is out_of_range |
| `state-signature-clean` | built and matches | P3 7.2: the control of the next three: one blob frame of 20 bytes and a valid commit; the commit names a state of one blob entry of length 20 |
| `state-signature-duplicate` | built and matches | P3 7.2 clause (3): the signatures field of the blob frame as two equal records makes it opaque; the valid commit names no state, the referent is state_unavailable |
| `state-signature-noncanonical-head` | built and matches | P3 7.2 clause (3): the signatures field as the bytes 98 00 makes the blob frame opaque; the commit names no state |
| `state-signature-value-invalid` | built and matches | P3, what the name does not bind: a signature of the wrong value length and one that does not verify are signature outcomes; the state is that of the control |
| `state-same-name-order-free` | built and matches | P3 7.2, ST2: the control and the changed segment in either order: the referent resolves to the state of the control |
| `state-opaque-noncontent-before-commit` | built and matches | P3 7.2 clause (3): an opaque commit (signatures grammar) before a valid commit: the valid commit names no state |
| `state-commit-own-opaque` | built and matches | P3 7.2 clause (1): a valid commit whose own fold state is opaque names no state |
| `state-parity-ordinary-broken-link` | built and matches | P3 7.2 clause (3): an ordinary parity frame with a broken link is not_content and the commit after it names no state |
| `state-rebuild-refused-capacity` | built and matches | P3 7.2 clause (4): a term unreadable because the rebuild allowance refused its value: the commit names no state below the allowance, a state above it; the refusal is produced by the total rebuilt length of P4 6.3 |
| `state-rebuild-refused-mixed` | built and matches | P3 7.2 clause (4): two value frames of one value, equal total_length N, the first with a missing unit, the second complete: with allowance N the first is incomplete, the second refused, the term unreadable and the commit names no state; with 2N the term is readable and the commit names a state that holds the row |
| `state-readers-agree` | built and matches | P3 ST3: every reader that finds a state finds one value; the keyless reader of the sealed file finds none |
| `terms-literal-ref-two-lengths` | built and matches | one value frame of (D, 3) and two terms naming D of lengths 3 and 4: the first readable, one missing_dependency with value for (D, 4) at its terms frame (P3-92) |
| `terms-literal-ref-pairs-coalesce` | built and matches | four terms in two terms frames of two segments naming (D, 4), (D, 5), (D, 4), (D, 5) in file order and one value frame of (D, 3): exactly two missing_dependency with value, one for each pair, each at the frame that first names it; the chunk of the value is named by the value (no allowance needed) |
| `terms-literal-ref-wrong-length-candidate` | built and matches | a value frame of (D, 4) whose entries sum to 4 and do not hash to D, and a term of (D, 4): damaged_frame with blob_digest at the value frame, its rebuild state incomplete with blob_digest, the term unreadable, no missing_dependency |
| `terms-literal-ref-length-cross-segment` | built and matches | a value frame of (D, 3) in segment 0, terms of (D, 3) and (D, 4) in segment 1: the first readable in the file's fold, missing_dependency with value for (D, 4), and no finding for either in the own fold of segment 1 |
| `terms-literal-ref-failed-before-ok-lengths` | built and matches | two value frames of (D, 3), one failing its rebuild and one verified, and terms of (D, 3) and (D, 4): the first term read from the verified frame, the failing frame's finding once at its position, one missing_dependency with value for (D, 4); the same with the frames exchanged |
| `terms-formed-witness` | built and matches | 4096 IRIs, term 0 of 65536 bytes, every later term prefix 65535 and one suffix byte (decoded payload 86033, charge 268365825): refused with 268365824 (count 268365825, frame opaque, no ill_formed_term), folded with 268365825 (P3-101, P3-102) |
| `terms-formed-copy` | built and matches | 1024 IRIs, term 0 of 65536 bytes, every later term prefix 65536 and suffix 0 (charge 67043328): refused with 67043327, folded with 67043328; the charge does not depend on whether prefixes are copied or shared (P3-103) |
| `terms-formed-then-small` | built and matches | three terms frames of charges 100, 50 and 10, read with 120: the first folded; the second refused with count 150; the third refused with count 110 although it would fit (P3-102) |
| `terms-formed-order` | built and matches | two segments, each with one terms frame of charge T, read with T: the first segment's frame folded and the second's refused (P3-101) |
| `terms-formed-malformed-first` | built and matches | a terms frame whose prefix_lengths holds one element too many and whose elements sum above the capacity, then a terms frame of charge 5, read with 5: the first malformed_value with inconsistent_counts and not charged; the second folded (P3-101) |
| `terms-formed-suffix-free` | built and matches | a frame of 1024 restarts (restart_group 1) whose suffixes hold 1048576 bytes, read with 0: folded, the charge 0 (P3-101) |
| `terms-formed-ids-reserved` | built and matches | a refused frame of 3 terms, then a terms frame with id_base 3 of charge 0, and a quads row naming ids 3, 3, 3: the second frame refused as well, with the count equal to the total kept (P3-102); no inconsistent_counts, because its id_base follows the ids that the first frame reserves; the row held, opaque_term at the row frame (P3-103) |
| `terms-formed-row-held` | built and matches | a refused frame whose first term a quads row names: the row held, opaque_term at the row frame, the content digest absent, integrity intact_unchecked (P3-103) |
| `labelling-weight-example` | built and matches | the segment of the worked example of 6.5 (weight 12448 for each fold): with 24896 both folds labelled, with 24895 refusal_weight in the file's list only, with 12447 in both lists (P3-104, P3-105) |
| `labelling-weight-witness` | built and matches | one IRI of 1024 bytes, a chain of 500 triple terms and 500 quads rows (decoded payloads 4289 bytes, 499 nested terms, weight of each fold 3127936): with 6255872 both folds labelled, with 6255871 the fold of the file refused; at the minimum both labelled (P3-105) |
| `labelling-weight-segments` | built and matches | two segments, each a fold of weight W, and a file fold of weight 2W: with 4W all labelled; with 4W - 1 the file's fold refused; with 2W - 1 the second segment's own fold and the file's fold refused (P3-105) |
| `labelling-weight-after-size` | built and matches | a segment whose own fold exceeds labelled_lines and a second segment whose own fold weighs W, read with W: the first own fold and the file's fold refused by size, the second own fold labelled; no refusal_weight (P3-104) |
| `labelling-weight-blank` | built and matches | the one line quad(_:b, <http://e/p>, <http://e/o>), weight 1 + 9 + 19 + 19 + 1 = 49, read with 98 and with 97: with 98 both folds labelled; with 97 the file's fold refused (P3-104) |
| `labelling-weight-records` | built and matches | one binding of a blank node and one blob declaration, and no line: a weight equal to the sum of the weights of the two record lines of 6.2, and a refusal one byte below twice that sum (P3-104) |
| `labelling-weight-guaranteed` | built and matches | a guaranteed labelled set above labelled_bytes_per_read: refused with refusal_weight (P3-105) |
| `project/quad-lines` | built and matches | P3 13.8: two quad lines in the default graph and a named graph: Q of two quads, G0 empty, both variants |
| `project/reifier-annotation` | built and matches | P3 13.8: a reifier_binding line and an annotation line of count 3: the quads (r, rdf:reifies, <<s p o>>, g) and (r, q, v, g), no count, no quad for (s, p, o) |
| `project/annotation-count-lost` | built and matches | P3 13.8: folds that differ only in the count of one annotation line: equal projections, different content digests |
| `project/annotation-equals-quad` | built and matches | P3 13.8: an annotation line and a quad line with the same four terms: one quad in Q, two lines in the fold |
| `project/binding-quad-collision` | built and matches | P3 13.8: a reifier_binding line and a quad line (r, rdf:reifies, <<s p o>>, g): one quad, a content digest different from the fold with one of them |
| `project/language-case` | built and matches | P3 13.8: x@EN and x@en: two quads in Q, the language texts byte for byte (that an RDF tool reads them as one is not a result of the projection) |
| `project/value-pair` | built and matches | P3 13.8: a pair of VALUES for a literal and none: equal projections, different content digests |
| `project/empty-graph` | built and matches | P3 13.8: an empty_graph line for g: Q empty, G0 = {g}; with a line that carries g: G0 empty |
| `project/empty-graph-hidden` | built and matches | P3 13.8: visible: the graph whose only line is hidden is no name in G0, the other name is; whole: the hidden line is in Q |
| `project/hidden-lines` | built and matches | P3 13.8: three lines, one hidden by a quad entry: visible two quads, whole three |
| `project/bindings-suppression-blobs` | built and matches | P3 13.8: a binding, an entry, a blob declaration and a pair of VALUES: Q holds only the quads of its lines |
| `project/blank-node-only-in-record` | built and matches | P3 13.8: a blank node named only by a binding: no blank node in the result |
| `project/not-rdf-texts` | built and matches | P3 13.8: a relative IRI, a language that is no tag, a typed literal of datatype rdf:langString: three quads, texts unchanged, no finding, no stop |
| `project/triple-term-nested` | built and matches | P3 13.8: a triple term whose object is a triple term, as an object of a quad: the same nested term |
| `project/round-trip-rdf` | built and matches | P3 13.8: a dataset of quads in two graphs, a nested triple term and an empty named graph: rdf(IMPORT(D), v) = D for both variants (R1) |
| `project/round-trip-idempotent` | built and matches | P3 13.8: rdf(IMPORT(rdf(F, v)), v) = rdf(F, v) (R2) and the digest of the import differs from that of F |
| `project/digest-not-rdf` | built and matches | P3 13.8: x@EN and x@en: the imports have different content digests |
| `project/order` | built and matches | P3 13.8: a fold read with labelled_lines below its size: the listing of Q is in the order of 5.1 after a refusal; the set is that of a labelled reading |
| `lowering-nested-term` | built and matches | P3 12.8: x = <<b q c>> nested in t = <<a p x>>, one row quad(a, p, t): one term node and one record (the preimage of d(x) is the 46 bytes of 12.8, size 2, weight 182, no blank term in the canonical file) (P3-107, P3-108, P3-109) |
| `lowering-shared-nested-term` | built and matches | x = <<b q c>>, <<a p x>> and <<c p x>> and the lines quad(a, p, <<a p x>>), quad(c, p, <<c p x>>): one term node and one record for x, shared by two lines; size 4 (P3-107, P3-108) |
| `lowering-two-depths` | built and matches | x = <<b q c>> and the lines quad(a, p, x), quad(a, p, <<a p x>>): the labelled set quad(a, p, <<b q c>>), quad(a, p, <<a p n_x>>) and record(P, [n_x, b, q, c]); the first line keeps x at depth one (P3-108) |
| `lowering-none` | built and matches | triple terms of depth one only: quad(a, p, <<b q c>>), reifier_binding(_:r, <<b q c>>), annotation(1, _:r, q, c): no term node and no nested_term record; the labelled set is the three lines as written; both nested-candidate numbers of the size 0; content digest <Dnone> (6.2a, THREE CASES) |
| `lowering-deep` | built and matches | the chain t(1) = <<a a a>>, t(k + 1) = <<a a t(k)>> of 1000 terms in one triple run and 1000 rows quad(a, a, t(k)): 999 term nodes and 999 nested_term records, size 1999, weight 172 n - 64 = 171936, no triple term of depth above one formed (6.2a, LOWERING); and the same file read with labelled_bytes_per_read 171935: both folds refused with refusal_weight, no record built (P3-105, P3-106) |
| `lowering-doubling` | built and matches | t(1) = <<a p a>>, t(k + 1) = <<t(k) p t(k)>> for k from 1 to 31 and the row quad(a, p, t(32)): t(2) to t(32) unreadable (position_constraint once), the row held (opaque_term), the content digest absent, no term node (P3-19) |
| `lowering-doubling-leaf` | built and matches | the leaf form of the shape of lowering-doubling: a binding whose leaf is [3, [L, [1, p], L]] with L a leaf of tag 3 that holds a leaf of tag 2: an unreadable leaf (6.3, POSITIONS IN A LEAF), the binding held, opaque_term, the content digest absent (P3-112) |
| `lowering-blank-inside` | built and matches | a segment S with the blank term u (label 75), x = <<_:u q c>>, <<a p x>> and the line quad(a, p, <<a p x>>); and a file of two segments, each S: h(_:u), K(_:u), the preimage of d(x) of 69 bytes, the source identifier; two nested terms and two term nodes in the fold of the file, one in the own fold of each segment (P3-109) |
| `lowering-twins` | built and matches | one segment with the blank terms u (75) and w (77) and the lines quad(a, p, <<a p <<_:u q c>>>>), quad(a, p, <<a p <<_:w q c>>>>): the lowered set, guaranteed, the final positions of u, w, n_u, n_w and the content digest as the vector gives them (6.4) |
| `bindings-leaf-deep` | built and matches | two segments, the first holding the blank term b, the second a binding whose leaf is of kind triple nested 100000 deep in its object, the innermost leaf the subject of kind blank naming b (read with labelled_bytes_per_read 1000, so that the labelling is refused by weight after the lowering has weighed the fold); and the same with the bottom leaf of kind term: the first read with no malformed_value, its term, in the fold of the file, the triple term of that depth, 99999 nested terms, size 1 + 99999; the second malformed_value/forbidden_combination |
| `lowering-literal-source` | built and matches | a segment with quad(a, p, <<a p <<b q "x"@EN-us--ltr>>>>) and quad(a, p, <<a p <<b q "7"^^xsd:nonNegativeInteger>>>>), the second literal stored as the typed value [2, 7]: the sources K and the preimages of 6.2a (44 and 93 bytes), the digests <Dlang> and <Dtyped> and the content digest <Dlit> (P3-109) |
| `lowering-twins-one-component` | built and matches | one segment with the blank terms u (75) and w (77) and the lines quad(a, p, <<a p <<\_:u q c>>>>), quad(a, p, <<a p <<\_:w q c>>>>), quad(\_:u, r, \_:w), quad(\_:w, r, \_:u): the value of guaranteed <Gtwin1>, the final positions <Ptwin1> and the content digest <Dtwin1>, as the vector gives them (6.4) |
| `suppress-value-segment-order` | built and matches | a segment with a value frame of the pair (D, 3) and a segment with the term, the row and a suppress entry that names that line, in both orders |
| `terms-literal-ref-value-after` | built and matches | a terms frame with a literal_ref term and a row that names it, then a value frame that provides the pair |
| `broken-link-commit-damaged-id` | built and matches | the stored identifier of the broken frame replaced and its content damaged too: no healing, the commit fails its root, the rows stay held (P3-117) |
| `broken-link-commit-doubt-stays` | built and matches | a valid commit ends the rule after a broken link for frames below it, never the doubt that K begins (P3-117) |
| `broken-link-commit-ends` | built and matches | rows after a broken frame and below a valid commit are not held under whole, held without the commit and under forward (P3-117) |
| `broken-link-commit-id-text` | built and matches | the identifier element of the broken frame a text, the rest intact: the commit is valid, the rows below it are not held (P3-117) |
| `broken-link-doubt-resync` | built and matches | the joined file in a segment opened by a resync item; and the file of resync-field-differs with later terms frames (P3-117) |
| `broken-link-doubt` | built and matches | a torn frame followed by the header and the frames of a joined file: the rows of the joined frames held (P3-117) |
| `broken-link-forward-held` | built and matches | the file of broken-link-commit-ends under the access forward: rows held, the commit releases none (P3-117) |
| `broken-link-transparent` | built and matches | a broken transparent frame starts no rule: the rows after it are not held (P3-117) |
| `dictionary-doubt-decides` | built and matches | an opaque item that is no frame, then a broken frame, rows, a valid commit, rows: the doubt stays after the commit (P3-116, P3-117) |
| `doubt-lost-header-recovery` | derived from the text | the header of the second segment is lost: the bytes from it to the parity frame of the second segment are a damaged region of the first segment, reading resumes at that parity frame, whose table holds under the equal h0, and the quads frame after it is detached and in doubt, so anchored_frames is 2 and its row is held with opaque_term (P3-116, P5 7.2A) |
| `doubt-lost-header` | built and matches | P3 2.8 THE DICTIONARY IN DOUBT: a lost second header, three files (P3-116) |
| `doubt-units-only` | built and matches | an opaque item `01` puts the dictionary in doubt; a blob frame after it is unaffected (P3-116) |
| `resync-doubt-ends` | built and matches | a lost header and then a resync item: the dictionary in doubt ends at the item, the rows after it resolve against the terms frames after it (P3-116) |
| `resync-forged-ahead` | built and matches | a forged resync item with a true offset inside a failing frame, ahead of genuine frames, and a later genuine item (P3-117) |
| `resync-state-local` | built and matches | a resync item with a checked lineage after a segment with an opaque frame: the run of the segment it opens is that segment alone, its commit names its own state (7.2 THE RUN) |
| `state-checked-copy` | built and matches | the file F F with F = S0 S1, S1 with lineage [head(S0), 1] and a commit: both commits of S1 have one identifier and name equal states (ST2) |
| `terms-literal-ref-length-pair` | built and matches | header, value frame of 61 62 63, terms with a literal_ref term, quads, commit; length 3 and 4 (P3-92) |
| `terms-literal-value-ref-length-pair` | built and matches | the same two files with a literal_value_ref term (P3-92) |
| `broken-link-content-value-not-judged` | built and matches | the file `H T Q` (registry version 1, `T` a `terms` frame of three terms, `Q` a `quads` frame of one row, each linked as its writer wrote it), and the same file with the `order` of `Q` changed by one bit from `2` to `18`, a value of no registry version, so that the link of `Q` fails: `Q` `broken` with `chain_break` alone, no `malformed_value`, no `unregistered_value`; its payload `verified`; opaque with `chain_break`; no finding of the file other than those that the lack of a commit gives (P3-74) |
| `broken-link-newer-value-not-interpreted` | built and matches | a segment of `registry_version` `2`, reader version `1`, `H T B`: `T` a critical `terms` frame, the first of its segment, whose `order` is `9`, a value the reader does not hold, `B` a `blob` frame linked as its writer wrote it, and the `id_base` of `T` changed by one bit from `0` to `16` so that the link of `T` fails: `unsupported_registry_version` once; `T` `broken` with `chain_break` alone, no `inconsistent_counts`, no `unsupported_critical`, no `unknown_enumeration_value`; `T` opaque with `chain_break`; the fold of the segment does not stop: `B` `linked` and `folded` (P3-74) |
| `broken-link-no-effective-identifier` | built and matches | an ordinary frame that is `broken` and has no effective identifier (its `transparent` not a `bool`, its `identifier` not `bytes`) starts THE DICTIONARY AFTER A BROKEN LINK: a later row naming ids of an earlier `terms` frame is held, `opaque_term`, the content digest `absent` (P3 2.8, P3-117) |
| `doubt-misplaced-locator-none` | built and matches | a misplaced `locator` item after the header, then a `terms` frame and a `bindings` frame: the dictionary is not in doubt, the binding is in the fold and its `state` referent names a commit that names no state: `missing_dependency` `state_unavailable`, no `opaque_term` (P1-17, P3 2.8) |
| `doubt-terms-conditions-judged` | built and matches | `H T X T2`: `X` an opaque item that is not a frame (the dictionary is in doubt from it), `T2` a `terms` frame whose first term is an `iri` whose text is not valid UTF-8: items 1 to 6 of 2.8 are still judged for a `terms` frame in doubt, before item 7: `T2` has `ill_formed_term`, and its terms are unreadable (P3 2.8, P3-116) |
| `terms-id-base-ill-typed-reserves` | built and matches | a `terms` frame whose `id_base` is ill typed and whose `runs` are well formed is opaque and still reserves its ids: a later row naming them is held, `opaque_term`, not `forward_reference` (P3 2.1, P3-6) |
| `state-selector-compaction` | built and matches | a state referent of a signed commit of an appended file resolves; in the canonical file written from its fold the same referent is missing_dependency with state |
| `labelling-size-equal-folds` | built and matches | two files of equal folds read with labelled_lines 1: the file of one quads frame is labelled, the file of two quads frames of the same row has size 2 and is refused by size, and compaction stops with labelling_refused |
| `project/ill-typed-literal` | built and matches | P3 10a.3: a typed literal whose lexical form is outside the lexical space of its datatype: one quad, the text unchanged, no finding, and an RDF dataset |
| `project/unknown-datatype` | built and matches | P3 10a.3: a typed literal whose datatype IRI no RDF tool recognises: one quad, the text unchanged, no finding, and an RDF dataset |
| `project/malformed-iri` | built and matches | P3 10a.3: an iri of a text that is absolute in form and malformed: one quad, the text unchanged, no finding, and the result is not an RDF dataset |
| `project/dirlangstring-and-empty-language` | built and matches | P3 10a.3: a typed literal of the datatype text rdf:dirLangString and a lang_literal of the empty language: each is carried unchanged with no finding, and the result is not an RDF dataset |
| `project/digest-as-literal` | built and matches | P3 7: a literal whose text is the hexadecimal digest of a payload that the file does not hold and that no binding names: a term of one quad, no finding, no missing blob |
| `project/rich-fold` | built and matches | P3 10a: one fold of every construct of the projection (quad, reifier binding, annotation of count 3, annotation equal to a quad, empty graph, binding, suppression entry, typed value, two language cases, blank node only in a binding, blob declaration): the two variants differ only by the hidden quad |
| `replayed-segment-annotation` | built and matches | a segment with one annotation row, alone and joined to itself: the annotation line takes the count 2 and the second segment begins a lineage |
| `replayed-segment-blank-node` | built and matches | a segment with one row over a blank node, and a second with the same row over an IRI, each alone and joined to itself: two nodes in the first joined file, one line in the second |
| `deltas-value-dependency` | built and matches | a segment that declares a value and a segment whose term names it, in both orders and the second alone: the value serves only what stands after it, and the segment's own fold never holds it |
| `segment-orders-three-independent` | built and matches | three independent segments of ground quad lines with lines in common in all six orders, and an empty graph named by a quad of another segment: equal folds and digests, and the empty graph kept only where no segment names it |
| `suppress-before-content-repeated` | built and matches | a segment with a suppression entry for a quad that stands in no segment, alone and joined to a segment with the quad in four ways: the entry is unmatched alone and hides the line in every joined file, with one entry in the record |
| `revised-belief-audit-link` | built and matches | occurrences of a belief and its revision, with the occurrence hidden by a term entry and by a reifier entry: the term entry also hides the audit link of the revision, the reifier entry leaves it, and lines written again stay hidden |
| `suppress-by-untrusted-party` | built and matches | a segment by a trusted signer and an appended segment of a party that no reader trusts that suppresses its line, with the frame covered by no signature and by a signature of an untrusted key: the line is hidden, the state says that the suppressing frame is unauthenticated |
| `assertions-and-decision-keys` | built and matches | three assertions of one triple by three signers and a decision by a fourth, read with all four keys trusted and with the key of the decision alone: the fold holds the three occurrences in both reads and only the authentication of the first three segments changes |
| `package-two-releases` | built and matches | a release with its module bound to a blob and an appended release with another module bound to another blob: the fold holds both bindings and both blobs and reports no finding |
| `two-licence-statements` | built and matches | a licence statement for a bound image, and an appended statement that replaces it, both signed by the curator: both statements stand in the fold and are visible, and no finding is reported |
| `artwork-revisions-sealed-notes` | built and matches | an artwork with two revisions of its licence and catalogue record hidden by reifier entries and conservation notes in sealed frames of a segment of registry version 2, read without a sealing key: the earlier occurrences are hidden and their audit links visible, and the state is incomplete by the unopened sealed frames |

Totals: built and matches: 410; derived from the text: 2 (412 entries).
