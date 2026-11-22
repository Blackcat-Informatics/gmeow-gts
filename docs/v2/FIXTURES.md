<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Guide: Fixture catalogue of generation 2

This document is informative. It states no requirement of the format. The normative documents of `docs/v2/` govern, and where
this document and they differ, they are right. It lists inputs, the read made of each, and the typed result that the rules
give, so that a reader, a writer, a verifier or a converter can be tested against them.

## 1. How to read an entry

Each entry has an identifier `FX-nnn`, in one numbering for the whole catalogue. The entries are grouped by the part or
profile whose rule gives the result. An entry has these lines:

- Input: the bytes or records that are read or judged.
- Built: how the input is made. Where the line is absent, the writer of the corpus writes the input as described.
- Read: the capacity and the inputs of the read, where they differ from the default read below.
- Result: the typed result that the rules give.
- Application: the outcome of an acceptance contract of the application, where the entry has one. The format does not compute
  it. `GUIDE-applications.md` describes such contracts. The outcome is accepted, refused or incomplete.
- Measured: in a benchmark entry, what is measured; the rules give no value for it.
- Rule: the part and the section title of the rule that gives the result.
- Corpus: the entry of the draft corpus of this generation that is the case, or the nearest to it, by directory and entry name.
  That corpus is `vectors/v2/`; its `README.md` says what an entry records.

THE DEFAULT READ. Unless an entry says otherwise, the read has the access `whole`, every field of `capacity` that has a
minimum value at that value, `marks` `absent`, an empty `profiles`, the registry version that the file needs, and empty
`trusted_keys`, `held_keys` and `sealing_input`. Where an entry puts a key in `trusted_keys`, or calls a signer trusted,
`trusted_keys` holds that key, and `held_keys` holds the key of every signature of the file, unless the entry says otherwise.

WORDS. KEYLESS is a read with an empty `sealing_input`. A POSITION is the position of an item in its segment, the header at
`0`. An ORDINARY POSITION counts the ordinary frames of a segment from `0`. A STORAGE REWRITE changes `storage`, a stored payload
or a `signatures` list, and with them the `offsets` item and the locator, and no identifier. Terms are IRIs unless
the entry says otherwise. An entry that says "two readers" means two readers written independently.

WHAT AN ENTRY RECORDS. An entry of the corpus records the encoded `read_result`, or the outcome of the proof, the compaction,
the conversion or the writer request that it tests, together with the capacity, the access, the keys, the marks and the
profiles of the read.

## 2. `P1`: framing, identifiers and growth

### FX-001 A resumption whose `anchor` is the identifier of the frame that the region took

- Input: `H F1 F2 Q0 F3 ...`, with the first byte of `F2` damaged. The damaged region takes the ordinary frame `F2`. Reading
  resumes at the parity frame `Q0`, whose `anchor` is the identifier of `F2`.
- Built: the input of the corpus entry named below.
- Result: `F3` is `linked` and DETACHED, because the `anchor` differs from the effective identifier of `F1`, the nearest
  preceding ordinary frame. `anchored_frames` is `1`.
- Rule: `P1`, "The predecessor and transparent frames"; `P7`, "What fills each field" (`anchored_frames`).
- Corpus: `P7/read/resume` is the case. Its recorded `anchored_frames` is `1`, as the rule gives. Its input is not in the
  repository; `vectors/v2/ABSENT.md` lists it.

### FX-002 Files with equal heads and different signatures, storage or parity

- Input: a file `A` of one segment with a signed valid commit, and three files made from it. `B1`: a second signature added to
  the `signatures` list of that commit. `B2`: one payload stored `stored_compressed` where `A` stores it `stored_plain`. `B3`:
  parity frames added at their places, with the `offsets` item and the locator written again.
- Built: `B1` and `B2` are storage rewrites of `A`. `B3` adds transparent frames, each with an identifier of its own, and changes
  no identifier of `A`.
- Read: `trusted_keys` holds the first signer and `held_keys` holds both signers; a second read of `A` and of `B1` trusts the
  added key alone, with the same `held_keys`.
- Result: in all four files every frame of `A` has the same identifier, the head and the root of the commit are equal, and the
  content digests are equal. A file proof from `A` to each of them gives `proof_holds`; the `proof` of its entry is `absent`, and
  the proof holds by the comparison of the head. The bytes and their whole-byte digests
  differ. `B1` has one more `signature_result`, and when only the added key is trusted `A` is `unauthenticated` and `B1` is
  `authenticated`.
- Application: a synchronization that compares heads reports the four as equal. A synchronization of exact replicas compares
  whole bytes and reports three differences.
- Rule: `P1`, "What the identifier commits to"; `P5`, "What a proof proves, and what it does not".
- Corpus: `P1/equal-heads-different-bytes` is the four files and their reads; `P6/signature-added-other-key-trusted` is the reads
  of `A` and of `B1` with the added key alone trusted; `P5/proof-file-equal-heads` is the three file proofs.

### FX-003 A cursor in a file that holds one segment twice

- Input: a file `S` then `S`: two byte-equal segments with equal headers, and so equal `h0`. A transfer of the file is
  interrupted in the second segment.
- Result: every frame identifier of segment `0` stands again in segment `1`. A search for "the frame after the identifier `i`"
  has two places. The two `segment_result` records have the ordinals `0` and `1`.
- Application: a cursor of the whole-byte digest, the segment ordinal `1` and the ordinary position resumes at one place. A
  cursor of the identifier alone is refused as ambiguous.
- Rule: `P1`, "Every preimage, typed" (an identifier is a function of the fields of the frame and of its predecessor, and of
  no offset); `P7`, "The records" (`segment_result.ordinal`).
- Corpus: `P3/state-repeated-segment` is the file and its read. No entry records a cursor.

## 3. `P3`: content, the fold, suppression and the projection

### FX-004 A replayed segment with an annotation

- Input: a file `S` of one segment with one `annotations` row `(r, p, o)` in the default graph; and the file `S` then `S`.
- Result: `S` folds to one `annotation` line with the count `1`. The joined file has two segments, the information finding
  `lineage_begins` in the second, and one `annotation` line with the count `2`. The content digest of the joined file differs
  from that of `S`. The content digest of each of its segments equals that of `S`.
- Application: a synchronization layer drops the replay by the identity of the operation before it composes.
- Rule: `P3`, "The fold", "Definition" (item 1); `P1`, "Growing a file", CONCATENATION.
- Corpus: `P3/replayed-segment-annotation` is the case.

### FX-005 A replayed segment with a blank node

- Input: `S` with one `quads` row `(b, p, o)`, `b` a blank node of the label `x`; and the file `S` then `S`. A second pair
  with `b` replaced by an IRI.
- Result: the first joined file holds two `quad` lines over two nodes, `blank(0, x)` and `blank(1, x)`. The second joined file
  holds one line, and its content digest equals that of `S`.
- Application: as FX-004. An entity that two replicas speak of is named by an IRI.
- Rule: `P3`, "The term model" and "Resolution"; `P3`, "What the order of frames and the order of segments decide".
- Corpus: `P3/replayed-segment-blank-node` is the case.

### FX-006 Two deltas of which one depends on the other

- Input: `F`, a segment with the units and the `value` declaration of a value of the digest `d`. `G`, a segment whose `terms`
  frame holds a `literal_ref` term that names `d` and whose `quads` frame uses it; `G` holds no declaration of the value.
  Files: `F` then `G`; `G` then `F`; `G` alone.
- Result: in `F` then `G` the term is readable and the row is a line; the content digest of the file is `present`, and that of the
  segment `G`, in whose own fold the value does not stand, is `absent`. In `G` then `F`, and in `G` alone, the term is
  unreadable with `missing_dependency` and the reason `value`, the row is held with `opaque_term`, and the content digest of
  the file is `absent`.
- Application: the synchronization layer orders `F` before `G`.
- Rule: `P3`, "`literal_ref`: a lexical form held by a `value` frame", A VALUE SERVES ONLY WHAT STANDS AFTER IT; `P3`,
  "Unreadable terms" and "Held".
- Corpus: `P3/deltas-value-dependency` is the case, in its cases `value-then-term`, `term-then-value` and `term-alone`.

### FX-007 Every order of three independent ground segments

- Input: three segments `A`, `B`, `C`, each of `quad` lines over IRIs and literals stored as text, with some lines in common;
  the six files of their orders. A counter-case: `A` holds the line `empty_graph(g)` and `B` holds a `quad` line in the graph
  `g`.
- Result: the six files have equal folds and equal content digests of the file, and `rdf(F, whole)` is the union of the three
  projections. Every segment after the first has the information finding `lineage_begins`. In the counter-case `A` alone holds the
  `empty_graph` line; the joined file, in either order, does not.
- Rule: `P3`, "What the order of frames and the order of segments decide"; "What the digest is a function of"; "The fold",
  "Definition" (an `empty_graph` line whose name another line carries is not in the fold).
- Corpus: `P3/segment-orders-three-independent` is the case: the six orders, and the three files of the counter-case. It records
  no projection.

### FX-008 A suppression before its content, and a repeated suppression

- Input: a segment `X` with the terms and the entry `[2, (s, p, o, absent)]`; a segment `Y` with the `quad` line `(s, p, o)`.
  Files: `X`; `X` then `Y`; `Y` then `X`; `X` then `Y` then `Y`; `X` then `X` then `Y`. No frame is signed.
- Result: `X` alone reports `suppression_unmatched` with the reason `quad`. In every other file the line is in the fold and
  hidden, and no `suppression_unmatched` is reported. The suppression record holds one entry, also where `X` stands twice.
  `unauthenticated_suppress` is the number of folded `suppress` frames: `1`, and `2` in the last file.
- Rule: `P3`, "Suppression hides"; "The hiding rule"; "The record"; "The visible dataset and visibility".
- Corpus: `P3/suppress-before-content-repeated` is the case.

### FX-009 A revised belief, its audit link and a reaffirmation

- Input: an occurrence `a1`: `reifier_binding(a1, <<s p o>>)` and annotations for the actor and the time. An occurrence `a2`:
  `reifier_binding(a2, <<s p o2>>)` and the annotation `(a2, wasDerivedFrom, a1)`. Variant T adds the suppression entry `[1,
  a1]` (kind `term`). Variant R adds the entry `[3, a1]` (kind `reifier`). Then an occurrence `a3`: `reifier_binding(a3, <<s p
  o>>)` with a later claimed time. In a further variant the lines of `a1` are written again unchanged.
- Result: T hides the binding and the annotations of `a1`, and also the line `(a2, wasDerivedFrom, a1)`, in which `a1` occurs
  as the object; the visible dataset holds no link from `a2` to its predecessor. R hides the lines whose reifier is `a1` and
  leaves that link visible. In both, the lines of `a3` are visible, and the lines of `a1` written again stay hidden; the
  annotations written again add to their counts, so the content digest differs from that of the file without them. The lines
  are delivered in the order of the canonical file, in which `a3` need not be last.
- Application: recall orders by the annotation of the claimed time, with a stated rule for ties, and returns `a3` as current.
- Rule: `P3`, "The hiding rule" (`term`, `reifier`); "Suppression hides"; "The fold", THE ORDER OF DELIVERY.
- Corpus: `P3/revised-belief-audit-link` is the case. It records no order of delivery.

### FX-010 A suppression by a party that no reader trusts

- Input: segment `0`, by a trusted signer: a `quad` line `(s, p, o)` and a signed valid commit. Segment `1`, appended by a
  party with no trusted key: the terms and one `suppress` frame with the entry `[2, (s, p, o, absent)]`. Two variants: the
  frame of segment `1` covered by no signature; and covered by a signature of a key that is not trusted.
- Read: `trusted_keys` holds the signer of segment `0`; `held_keys` holds both keys.
- Result: the line is in the fold and is hidden. The visible dataset is empty. `visible` is `[1, 1]`: complete, with
  `unauthenticated_suppress` `1`. Segment `0` is `authenticated`, segment `1` `unauthenticated`. No finding is an error. The two
  variants differ only in the `signature_result` of the second and in the lengths that the signature adds.
- Application: a contract that requires `unauthenticated_suppress` `0`, or that the suppressing frame be authenticated by a
  key entitled to revise, rejects the revision and reads the claim from the fold. It never reports the claim as absent.
- Rule: `P3`, "The hiding rule" and "The visible dataset and visibility"; `P7`, "How each state value is decided".
- Corpus: `P3/suppress-by-untrusted-party` is the case, in its cases `unsigned` and `untrusted-key`.

### FX-011 Three assertions of one proposition and a decision

- Input: one file of four segments of registry version `1`, each appended by a writer that restarted, with its own signing key
  (`m1`, `m2`, `m3`, `h`), each ending in a signed valid commit. Segments `0` to `2` each hold one assertion occurrence of the
  same triple `(s, p, o)`: a `reifier_binding` line with a reifier of its own (`a1`, `a2`, `a3`) and annotations for the actor
  and for the evidence, the evidence differing. Segment `3` holds the decision occurrence `d`, with annotations that name
  `a1`, `a2`, `a3`, the outcome and the scope, and no `suppress` frame.
- Read: once with `trusted_keys` holding the four keys, once holding `h` alone; `held_keys` holds the four keys in both reads.
- Result: the fold holds three `reifier_binding` lines (three reifiers, one triple term), every annotation with the count `1`,
  and `d`. `visible` is `[1, 0]`. With four trusted keys each segment is `authenticated`. With `h` alone, segments `0` to `2`
  are `unauthenticated`, and nothing of the fold changes.
- Application: the three occurrences stay distinct. The decision is accepted only from a key that the contract entitles to
  decide, for the scope that its annotations state.
- Rule: `P3`, "The fold", "Definition" (lines are a set by value); `P6`, "Authentication"; `P7`, "How each state value is
  decided".
- Corpus: `P3/assertions-and-decision-keys` is the case.

### FX-012 A package with two releases

- Input: segment `0` holds a release `:r1` whose manifest names the module `:m1`, bound to the blob `N1`. An appended segment
  holds `:r2`, whose manifest names `:m2`, bound to `N2`. Both blobs are present.
- Result: the fold holds both bindings and both blobs. No finding.
- Application: for the release `:r2`, accepted with the module `N2`. For a request that names no release, refused: two terms
  have the role of the module.
- Rule: `P3`, "The fold", "Definition", and "The frame, the referents and what a reader does with a binding".
- Corpus: `P3/package-two-releases` is the case.

### FX-013 Two licence statements for one image

- Input: segment `0` binds `:master` to the blob `N` and states `:lic1` for it, signed by the curator. An appended segment
  states `:lic2` for the same term, and that `:lic2` replaces `:lic1`, signed by the curator. A release `:rel2` names `:lic2`.
- Read: `trusted_keys` holds the key of the curator.
- Result: both statements are in the fold and visible. No finding.
- Application: for `:rel2`, the licence is `:lic2`. A request for "the licence of the image" with no release is refused as
  ambiguous.
- Rule: `P3`, "The fold", "Definition": a later line withdraws no earlier line.
- Corpus: `P3/two-licence-statements` is the case.

### FX-014 An artwork with two revisions of its licence and catalogue record, and restricted notes

- Input: an append history of three segments. Segment `0`: the image as a blob with a binding from the IRI of the artwork, a
  licence occurrence `l1` and a catalogue occurrence `c1`, signed by the museum. Segment `1`: occurrences `l2` and `c2` that
  name `l1` and `c1` as superseded, and `reifier` suppression entries for `l1` and `c1`, signed by the museum. Segment `2`, of
  registry version `2`: conservation notes in sealed frames under a key of the conservators, over a dictionary of its own.
- Read: keyless; `trusted_keys` holds the key of the museum.
- Result: one binding of the artwork to one blob, declared by a verified `blob` frame, so the state of the blob is complete and
  the frame has no `rebuild_result`. The visible licence statements are those of `l2`, with
  its visible link to `l1`. `l1` and `c1` are in the fold and hidden. `visible` is `incomplete`, with `unopened_sealed` the
  number of sealed frames of segment `2`.
- Application: the public contract selects the state of the last commit of segment `1`, whose run holds no sealed frame, and
  accepts it with no sealing key. The image and the licence in force are one each. A contract that asked for `visible`
  `complete` of the whole file would refuse every public reader, which is why the state is named.
- Rule: `P3`, "The hiding rule" (`reifier`); `P3`, "Graph states"; `P4`, "Rebuilding a blob or a value".
- Corpus: `P3/artwork-revisions-sealed-notes` is the case.

### FX-015 A state selector, before and after compaction

- Input: segment `0` holds content, a manifest and a signed commit `c` with the effective identifier `I`. An appended segment
  with a checked lineage holds a binding of `:r1` to the `state` referent `[3, I]` and its own signed commit. Then the fold of
  the whole read is compacted and the result is read.
- Result: before compaction the referent resolves to the state of `c`, with no `missing_dependency`. A reader of the canonical
  file reports `missing_dependency` with the reason `state` for the referent: no commit of that file has the identifier `I`.
- Application: before, the release is selected by `I`. After, the selection by `I` is refused, and the release is selected by
  the commit of the canonical file.
- Rule: `P3`, "Graph states"; `P7`, "Compaction".
- Corpus: `P3/state-selector-compaction` is the case: `appended` is the file before compaction, `canonical` its compaction.

### FX-016 Two reads with equal folds, of which one holds a row

- Input: two files with equal folds. `A` holds one valid ground quad `q`. `B` holds `q` and a second row that names an
  unreadable term.
- Built: `B` is the input of `P7/read/content-order-held`; `A` holds its readable row alone.
- Result: the folds are equal. `A`: the content digest is `present`, and `compact` gives `[1, file]`. `B`: one held row,
  `opaque_term`, the content digest `absent`, and `compact` gives `[2, opaque_term, absent]`.
- Rule: `P3`, "The fold", THE FOLD AND THE READ; "Held"; "The content digest"; `P7`, "Compaction" (case 2).
- Corpus: `P7/read/equal-folds-held-row` is the case: `a` is the file of the ground row alone, `b` the file that also holds the
  held row, which is the input of `P7/read/content-order-held`. The outcome of each compaction stands in the key
  `compaction_result`. `P3/content-digest-absent-held` and `P7/canonical/compaction/held` each hold one held row and no other row.

### FX-017 Two reads with equal folds and different sizes

- Input: two files with equal folds. `A` holds one `quads` frame with the row `r`. `B` holds two `quads` frames that each hold
  `r`.
- Read: `labelled_lines` `1`; the reader does not claim the minimum capacity.
- Result: `A`: the size is `1`, the fold is labelled and the digest is `present`. `B`: the size is `2`, `labelling_refused`
  with `refusal_size`, the digest `absent`, and `compact` gives `[2, labelling_refused, present(refusal_size)]`.
- Rule: `P3`, "Refusals and the reader's limit", THE SIZE OF THE LABELLED SET (the size is counted over the folded frames
  before the union); `P7`, "Compaction" (case 3).
- Corpus: `P3/labelling-size-equal-folds` is the case; `a` and `b` are read with `labelled_lines` `1`.

### FX-018 A literal outside the lexical space of its datatype

- Input: a fold with the one line `quad(<urn:s>, <urn:p>, "not-an-integer"^^xsd:integer)`, the literal stored as text.
- Built: the projection is taken in both variants, `visible` and `whole`.
- Result: one quad, the text byte for byte, and no finding. The result holds no text that RDF does not admit, so it is an RDF
  dataset.
- Application: an adapter reports two outcomes apart. Its loss report has no entry for the literal. Its datatype report names
  one literal outside the lexical space of its datatype; the format does not judge that.
- Rule: `P3`, "Texts that RDF does not admit".
- Corpus: `P3/project/ill-typed-literal` is the case.

### FX-019 A literal of a datatype that no tool recognises

- Input: a fold with one line whose object is a literal with an absolute datatype IRI that no RDF tool recognises.
- Built: as FX-018.
- Result: as FX-018: a literal of RDF 1.2, and an RDF dataset.
- Rule: `P3`, "Texts that RDF does not admit".
- Corpus: `P3/project/unknown-datatype` is the case.

### FX-020 A relative or malformed IRI

- Input: a fold with a line whose subject is an `iri` term of a relative text; a second case with a text that is absolute in
  form and malformed.
- Built: the first case is the input of the corpus entry named below.
- Result: the quad is carried unchanged. Its text is one that RDF does not admit, and the result is NOT AN RDF DATASET. No
  finding is made and no quad is dropped.
- Rule: `P3`, "Texts that RDF does not admit".
- Corpus: `P3/project/not-rdf-texts` is the first case; `P3/project/malformed-iri` is the second.

### FX-021 A typed literal of the datatype text `rdf:langString`, and a language that is not a tag

- Input: a typed literal whose datatype text equals `rdf:langString`; a `lang_literal` whose language is not a tag.
- Built: the input of the corpus entry named below.
- Result: each is a text that RDF does not admit. The projection carries both terms unchanged, and the result is NOT AN RDF
  DATASET.
- Application: an adapter does not claim RDF output for this result, changes no text, and reports the term.
- Rule: `P3`, "Texts that RDF does not admit".
- Corpus: `P3/project/not-rdf-texts` is the case, holds both forms, and agrees.

### FX-022 A typed literal of the datatype text `rdf:dirLangString`, and an empty language

- Input: a typed literal whose datatype text equals `rdf:dirLangString`; a `lang_literal` with the empty language text.
- Result: each is a text that RDF does not admit; the result is NOT AN RDF DATASET.
- Rule: `P3`, "Texts that RDF does not admit" (it names both generated datatype texts).
- Corpus: `P3/project/dirlangstring-and-empty-language` is the case, in its cases `typed-literal-dirlangstring` and
  `empty-language`.

### FX-023 A rich fold exported as N-Quads and into a schema that keeps the fold

- Input: one file whose fold holds: a `quad` line; a `reifier_binding` line; an `annotation` line with the count `3` whose four
  terms equal those of a `quad` line; an `empty_graph` line; a binding to a blob; a suppression
  entry that hides one `quad` line; a literal stored as a typed value, so with a pair of VALUES; the literals `"x"@EN` and
  `"x"@en`; a blank node that occurs only in a binding; a blob declaration with its bytes.
- Result: `rdf(F, visible)` and `rdf(F, whole)` are as the function gives them. The reifier binding is a quad with
  `rdf:reifies`. The two lines of equal terms are one quad. The count is not carried. The name of the empty graph is in `G0`.
  The two language literals are two terms. The hidden quad is absent from `visible` and present in `whole`. The blank node
  that is in no line is absent.
- Application: the N-Quads export writes `Q`, states the variant, and lists in its loss report: `G0` (one name), the count,
  the two kinds of line, the pair of VALUES, the binding, the suppression entry, the blob declaration, and the node that is in
  no line. An export to a schema that keeps the fold holds a row for each of them, and its loss report is empty. The import of
  the N-Quads output has another content digest than the source.
- Rule: `P3`, "The function", "What each construct gives and what it loses", "Import and the round trip".
- Corpus: `P3/project/rich-fold` is the case.

### FX-024 A digest written as a literal

- Input: a literal whose text is the hexadecimal digest of a payload that the file does not hold; no binding names it.
- Result: the literal is a term of a line. The blob set has no entry for it. No finding is made, and no reader reports a
  missing blob.
- Rule: `P3`, "Typed references: the `bindings` frame" (a reference is never recognised by a spelling).
- Corpus: `P3/project/digest-as-literal` is the case.

## 4. `P4`: blobs

### FX-025 A large file changed by one chunk, in the same segment and in an appended one

- Input: a file of one segment with a blob of 1 GiB written as a `recipe` over chunks; in the form of a file tree, one regular
  file under the profile `files`. One chunk changes; a second case changes one byte. (a) The writer that still holds the file
  continues the segment: the changed `chunk` frame, a new `recipe`, a new entry, the hiding of the old entry, a commit. (b) A
  writer that restarted appends a segment with the same change.
- Result: in (a) the new recipe resolves the unchanged units among the earlier frames of its segment; the appended bytes are
  of the order of one chunk and one entry list. In (b) the recipe finds its units in its own segment only. The appended
  segment then holds every unit of the file again, and the appended bytes are of the order of the file; or the recipe is
  incomplete with `missing_dependency` and the reason `chunk`.
- Measured: the bytes appended and transferred in (a) and in (b), and their ratio to the bytes changed. This entry is the
  input of the measurement of incremental revision in `GUIDE-adoption.md`.
- Rule: `P4`, "The unit index and the unit used"; `P1`, "Growing a file", THE FORMS OF APPENDING.

## 5. `P5`: commits and proofs

In FX-026 and FX-027, `a`, `a'` and `b` are the identifiers of the frames `A`, `A'` and `B` of FX-034.

### FX-026 A file proof against a later file with a detached frame

- Input: the file proof `[[0, 2, b, absent]]` of `F1 = H A B` (no commit), verified against `F2`, the file of FX-034 without
  its signature.
- Built: the prover on `F1`; `F2` as FX-034.
- Result: `proof_failed` with `consistency_diverged`: `B` is one of the first `c_0 = 2` ordinary frames of `F2` and is
  detached (step 4).
- Rule: `P5`, "File".
- Corpus: `P5/proof-file-detached-frame` is the case: `f1` and `f2` are the reads of the two files, `verify` the outcome of the
  proof.

### FX-027 A file proof of a source with a broken frame

- Input: `F1 = H A' B`, with `B` unchanged from `H A B`, so that `B` is `broken` and the head of `F1` is the stored identifier
  `b`. The proof `[[0, 2, b, absent]]`. `F2 = H A B`.
- Built: `A` is replaced by `A'` in `H A B`, and `B` is left as it is.
- Result: a verifier that reads `F1` gives `proof_failed` with `consistency_old_root` (step 3). A verifier that keeps `c_k`
  and `g_k` for later verifications keeps none from this read of `F1`.
- Rule: `P5`, "File".
- Corpus: `P5/proof-file-source-broken` is the case.

### FX-028 A consistency proof that is one entry short

- Input: two consistent roots of one segment, and a consistency proof whose `path` is one entry short.
- Built: the case `short-path` of the corpus entry named below.
- Result: `proof_malformed` with `length`. The outcome shows only that the proof given does not show the extension. No
  statement about the signer follows.
- Rule: `P5`, "Consistency"; "What a proof proves, and what it does not", AUTHENTICATION.
- Corpus: `P5/proof-consistency-short-path` is the case: `short-path` is the proof that is one entry short, `full-path` the
  neighbour that holds. `P5/proof-consistency-negative`, case `short-path`, records the same outcome.

### FX-029 A consistency proof of the right shape with a wrong hash

- Input: two consistent roots, each on a commit signed by one trusted key, and a consistency proof of the right length in
  which one hash of the `path` is replaced: once among the peaks of `old_count`, once among the later entries.
- Built: the prover on an honest segment of two signed commits; then one hash of each kind is replaced.
- Result: `proof_failed` with `consistency_old_root`, and `proof_failed` with `consistency_new_root`. Both signatures
  authenticate their roots. Neither outcome shows that the key signed two histories.
- Rule: `P5`, "Consistency"; "What a proof proves, and what it does not", AUTHENTICATION.
- Corpus: `P5/proof-consistency-signed-roots` is the case, in its cases `old-peak-replaced` and `later-entry-replaced`.

### FX-030 A file proof from an empty source to an empty file

- Input: `F1` of zero bytes, `F2` of zero bytes, and the proof `80`, the empty list.
- Built: by hand.
- Result: `proof_holds` with `consistency`.
- Rule: `P5`, "File" (steps 1 to 5 run over no entry; step 6 runs only when `p` is not `0`).
- Corpus: `P5/proof-file-empty-source-empty-target` is the case.

### FX-031 A file proof from an empty source to a file of one segment

- Input: `F1` of zero bytes, `F2` any file of one segment, and the proof `80`.
- Built: by hand.
- Result: `proof_holds` with `consistency`. The segment of `F2` is not covered by the proof.
- Rule: `P5`, "File" (a segment of `F2` with an ordinal of `p` or more is not covered).
- Corpus: `P5/proof-file-empty-source-one-segment` is the case.

### FX-032 A file proof from a file of one segment to an empty file

- Input: `F1` of one segment, `F2` of zero bytes, and the proof of `F1`: once with the `proof` of its entry `absent` (`F1 = H
  A B`, no valid commit), once with it `present`.
- Built: the prover on `H A B`; for `present`, the same entry with an inclusion proof written by hand, which step 2 never
  verifies.
- Result: `absent`: `proof_failed` with `consistency_diverged` (step 3: `F2` has fewer than `p` segments). `present`:
  `proof_malformed` with `count` (step 2, which stands before step 3).
- Rule: `P5`, "File".
- Corpus: `P5/proof-file-empty-target` is the case, in its cases `absent` and `present`.

### FX-033 A file proof of one entry for an empty source

- Input: `F1` of zero bytes and a proof of one entry.
- Built: by hand.
- Result: `proof_malformed` with `length` (step 1).
- Rule: `P5`, "File".
- Corpus: `P5/proof-file-empty-source-one-entry` is the case.

## 6. `P6`: signatures, coverage and authentication

In FX-034 to FX-039 the positions in a coverage are ordinary positions. `a`, `a'` and `b` are the identifiers of the frames
`A`, `A'` and `B`.

### FX-034 A valid signature on a detached frame

- Input: `H A' ff P B`. `H` is the header, with the identifier `h0`. `A'` is a blob frame linked from `h0`. `ff` is one byte.
  `P` is a parity frame whose `anchor` is `a`, the identifier of another blob frame `A` linked from `h0`, and whose span is
  the bytes of `H`, `A'` and `ff`. `B` is the blob frame linked from `a`, with one signature.
- Built: write `H A B` and sign `B`. Write `A'` with another payload. Compute the shards and the table of `P` over the bytes
  `H A' ff` under `h0`, and its identifier with `a` as its predecessor. Copy `B` with its signature unchanged.
- Read: `trusted_keys` and `held_keys` hold the key of the signature.
- Result: `undelimitable_input` over `ff`. `A'`, `P` and `B` are each `linked`. `B` is DETACHED. The signature on `B` is
  `valid`, with the coverage `[1, 2)`. `A'` is not an authenticated frame. `authenticated_frames` is `1`, `authenticity`
  `partly_authenticated`, `anchored_frames` `1`, `integrity` `damaged`.
- Rule: `P1`, "The predecessor and transparent frames" and "What the identifier commits to" (Claim 2); `P6`, "Coverage" and
  "Authentication"; `P7`, "What fills each field" (`anchored_frames`).
- Corpus: `P6/sig-detached-frame` is the case.

### FX-035 The neighbour of FX-034 whose `anchor` is true

- Input: `H A ff P B`: as FX-034 with the frame `A` in the place of `A'`, so that the `anchor` of `P` is the effective
  identifier of the last ordinary frame before the region.
- Built: as FX-034.
- Read: as FX-034.
- Result: `B` is not detached. The signature on `B` is `valid`, with the coverage `[0, 2)`. `A` and `B` are authenticated
  frames. `authenticated_frames` is `2` and `anchored_frames` `2`. `integrity` is `damaged`, by the region.
- Rule: `P1`, "The predecessor and transparent frames" (step 4); `P5`, "The one recovery procedure", RESUMPTION.
- Corpus: `P6/sig-resumed-frame-anchor-true` is the case.

### FX-036 A signature after a broken link

- Input: four blob frames, the second with a broken link, the fourth signed by a trusted key.
- Built: the input of the corpus entry named below.
- Result: the coverage is `[2, 4)`: the run begins after the broken frame. `anchored_frames` is `1`, `authenticated_frames`
  `2`, `authenticity` `partly_authenticated`.
- Rule: `P1`, "What the identifier commits to" (Claim 2); `P6`, "Coverage".
- Corpus: `P6/cov-break-before` is the case and agrees. Its recorded signature is `[valid, absent, 1, key, 2, 4, true]`, and
  its `anchored_frames` is `1`. `P7/read/anchored` and `P6/auth-broken-middle` hold the same rule and agree.

### FX-037 A signed valid commit over an undamaged segment

- Input: a blob frame and a valid commit over it, the commit signed by a trusted key; no damaged region.
- Built: the input of the corpus entry named below.
- Result: the commit is valid. The coverage is `[0, 2)`. Both frames are authenticated through the root. `authenticity` is
  `authenticated`.
- Rule: `P6`, "Authentication".
- Corpus: `P6/auth-commit-valid` is the case and agrees. It records `committed_frames` `2` and `authenticated_frames` `2`.

### FX-038 A signed valid commit after a detached frame

- Input: `H A' ff P B C`: the file of FX-034 without the signature on `B`, then a commit `C` linked from `b`, whose `count` is `2`
  and whose root is over `h0` and the effective identifiers `a'` and `b`, signed by a trusted key.
- Built: as FX-034; the root of `C` is computed from the frames present.
- Result: `C` is a valid commit. `A'`, `B` and `C` are authenticated frames, the detached frame `B` and the frame before it
  included, because the root holds each effective identifier as a leaf. The coverage of the signature on `C` is `[1, 3)`,
  `authenticity` is `authenticated`, and `anchored_frames` is `1`.
- Rule: `P6`, "Authentication" (a signed valid commit); `P5`, "The committed prefix".
- Corpus: `P6/sig-commit-after-detached-frame` is the case.

### FX-039 Two signatures after a detached frame

- Input: `H A' ff P B D`: the file of FX-034 with one more blob frame `D` linked from `b`; `B` and `D` are each signed by one
  trusted key.
- Built: as FX-034.
- Result: the signature on `B` covers `[1, 2)`, and the signature on `D` covers `[1, 3)`. `B` and `D` are authenticated
  frames; `A'` is not. `authenticated_frames` is `2`, `authenticity` `partly_authenticated`, `anchored_frames` `1`.
- Rule: `P6`, "Coverage" (the run begins at the detached frame).
- Corpus: `P6/sig-two-after-detached-frame` is the case.

### FX-040 Deterministic signing

- Input: one fixed composite key and one fixed `ed25519` key, and one subject: a frame identifier, with the header identifier of
  its segment, as the worked example "The subject and its bytes" of `P6` forms one. Each of two writers makes the signature twice
  with each key.
- Result: four equal signature values for each key. The ML-DSA half is the output of the deterministic variant of FIPS 204,
  with `rnd` of 32 zero bytes. A writer that is given the signature a second time for the same frame leaves its `signatures`
  list unchanged.
- Rule: `P6`, "Signing"; "Co-signing and pinning" (no signature equal to one already present).
- Corpus: none is the case. The nearest are `P6/sign-ed25519-reproduce` and `P6/sign-composite-reproduce`, which record the
  signature value for a subject of their own.

### FX-041 A hedged signature verifies

- Input: the key and the subject of FX-040, with the ML-DSA half made with a random `rnd`. A second input: a frame whose
  `signatures` list holds the deterministic and the hedged record of that key.
- Built: by the generator of test inputs. A writer writes neither input: it signs deterministically, and it writes no two
  signatures of one `algorithm` and `key_id` in one list.
- Read: `trusted_keys` and `held_keys` hold the key.
- Result: the status is `valid`, and `authenticated` is as for the deterministic signature. No finding is made. The second
  input has two `signature_result` records of one `key_id`, each reported by itself.
- Rule: `P6`, "Signing" (no outcome of reading depends on the rule); "The signature record"; "Co-signing and pinning".
- Corpus: `P6/composite-hedged` is the case: `hedged` for the first input, `both` for the second.

### FX-042 A receipt whose signature is removed

- Input: a file whose last commit carries two signatures, of the custodian that gives and of the custodian that receives. A
  copy from which the second signature was removed. A third file that declares `bundle` and carries the first file whole as a
  blob, bound to a nested-file term, the term of the custody event.
- Built: the copy is a storage rewrite of the first file.
- Result: the copy reads as the first file does, with one `signature_result` fewer at that commit and no finding for the
  removal. Every identifier is equal. In the third file the blob is COMPLETE, and its bytes hold both signatures.
- Application: for the copy, the transfer is refused: the role of the receiving custodian is not among the authenticating
  signatures. For the third file, the receipt is accepted from the read of the nested file.
- Rule: `P6`, "The signature record" and "What a signature binds and authenticates"; `P4`, "The rebuild"; `PF-bundle`, "What a
  reader reports about nested content".
- Corpus: `P6/cosign-remove-signature` is the copy, in its cases `two-signatures` and `second-removed`;
  `PF-bundle/bundle-signed-file-whole` is the third file, with its `nested_results`.

## 7. `P7`: capacity

### FX-043 A reader of a low stated capacity, and one frame beyond it

- Input: a file of one segment with a `terms` frame and one `quads` frame, stored plain, whose `decoded_length` is `1048577`.
- Read: `decoded_bytes_per_frame` `1048576`, every other field at its minimum value. A second read with
  `decoded_bytes_per_frame` `67108864`.
- Result: the `capacity` of the result holds `1048576`. The frame has the payload state `unchecked` and the finding
  `exceeds_capacity` with the reason `decoded_size`. Its fold state is `opaque` with `exceeds_capacity`. It is never
  `damaged`. `left_out` counts it. The reader does not claim the minimum capacity. In the second read the frame is `verified`
  and folded.
- Rule: `P7`, "Capacity", MINIMUM CAPACITY and ORDER OF JUDGEMENT; `P1`, "The payload check" (its capacity step).
- Corpus: `P7/read/capacity/low-stated` is the case: `decoded-1048576`, then `decoded-67108864`.

### FX-044 Two readers of one low capacity agree at every limit

- Input: one file for each limit, each one unit above one sixteenth of the minimum value. The limits: the stored size, the decoded
  size, the total decoded length, the total formed length, the total rebuilt length, the size of a fold, the total labelled
  weight. The number of items is not among them: a read that stops at that limit judges every item before the stop, and its result
  is too large to record.
- Read: two readers with the same `capacity`, each field at one sixteenth of its minimum value.
- Result: for each file the two `read_result` values are equal byte for byte, and each holds the refusal of the limit that the
  file was built for and of no other: `exceeds_capacity` with `stored_size`, `decoded_size`, `total_decoded`, `total_formed` or
  `total_rebuilt`, once; `labelling_refused` with `refusal_size` in the list of the segment and in the list of the file;
  `labelling_refused` with `refusal_weight` in the list of the file only, the own fold of the segment being admitted.
- Rule: `P7`, "Capacity" (one result for one capacity), ORDER OF JUDGEMENT.
- Corpus: `P7/read/capacity/sixteenth` is the case, one case for each of the seven limits. It records the result of one reader.

### FX-045 The claim of the minimum capacity, and its boundary

- Input: three `implementation_claim` records. The first has every field at its minimum value. The second has
  `decoded_bytes_per_read` `2^30` and the other fields at their minimum values. The third has every field above its minimum
  value. For the first, a file with one frame whose `decoded_length` is exactly `67108864`.
- Built: the records by hand; the file by the writer of the corpus.
- Result: the first and the third claim the minimum capacity. The second does not, and it conforms. The reader of the first
  claim reads the frame with the payload state `verified` and no `exceeds_capacity`.
- Rule: `P7`, "Capacity", MINIMUM CAPACITY.
- Corpus: none is the case for the claim records. `P7/read/capacity/minimum` is the read of a frame whose `decoded_length` is
  `67108864`; `P7/conformance/claim` is the first claim record.

### FX-046 A `guaranteed` input and the labelling limits of a small reader

- Input: a fold of `1000` lines, for which the predicate `guaranteed` of `L1` holds.
- Read: `labelling_units` `0` and `labelled_lines` `1000`; then the same with `labelled_lines` `999`.
- Result: first read: the fold is labelled, and the content digest is `present`. Second read: `labelling_refused` with
  `refusal_size`, in the list of the segment and in the list of the file, made before `L1` is called; the content digest is
  `absent`; every frame is folded and `left_out` does not
  move.
- Rule: `P7`, "Capacity" (`labelling_units` is never applied to a `guaranteed` input; `labelled_lines` is applied to every
  input); `P7`, "Honouring `hostile`", WHAT THE CONTENT DIGEST AND THE COUNT SHOW.
- Corpus: `P7/read/labelling/guaranteed-size-boundary` is the case: `lines-1000`, then `lines-999`.

### FX-047 A stopped read, and a refusal by the labelling limit

- Input: a file whose fold is outside the guaranteed class and whose walk is long.
- Read: run one: `labelling_units` `2^64 - 1`, and the test harness sends the reader the cancellation signal of its interface
  during the labelling. Run two: the same, stopped by a time limit of the host. Run three: `labelling_units` below the units
  of the walk, and no stop.
- Result: runs one and two: no `read_result`, no partial content, and no finding `labelling_refused` or `exceeds_capacity`
  anywhere; the interface reports that the read did not complete. Run three: a `read_result` with `labelling_refused` and the
  reason `refusal_limit`, equal for every reader with that capacity.
- Rule: `P7`, "Conformance"; `L1`, "The walk and the outcome of a limit", THE FREEDOM OF AN ENGINE.
- Corpus: none is the case for runs one and two, which give no result. `P7/read/labelling/limit` is a run of the kind of run
  three.

### FX-048 A frame that is malformed and also above capacity

- Input: frames to which several opaque reasons apply, among them a frame above capacity that is also malformed.
- Result: each frame carries the first reason in the order of precedence of the opaque reasons: `malformed` stands before
  `exceeds_capacity`.
- Rule: `P2`, "Enumerations of the typed result" (`opaque_reason`); `P7`, "Where each finding stands, and in what order".
- Corpus: `P7/read/fold-precedence-each` is the case: one file for each pair of opaque reasons, and in its case
  `malformed-exceeds-capacity` a frame that is `malformed` and above capacity. `P7/read/capacity/order`,
  `P5/parity-above-capacity-and-damaged` and `P9/convert/order-earlier-item-wins` state the order of the capacity limits of a
  read, of the findings of a parity frame and of the stops of a conversion.

## 8. `P7`: the accesses

### FX-049 One stream read forward and whole

- Input: one segment that declares `stream`: rows; a commit; a `suppress` frame that hides an earlier row; a `value`
  declaration, a term that names it by reference and a row that uses the term; a blob declared by a `recipe`; a final signed
  commit; one changed byte in a payload inside a span that has its parity frame.
- Read: once with the access `forward` and `end` true; once with the access `whole`.
- Result: `forward`: every row of a frame that folds is delivered at its frame, the hidden row before the entry that hides it. The
  row by reference
  is delivered by reference. `content_digest` is `absent`. Every rebuild has the state `not_run`. The damaged frame is `damaged`,
  its parity frame gives `parity_mismatch` with `data`, and nothing is repaired. `integrity` is that of the items read. `whole`:
  the frame is `damaged` as well, since the result reports the input as it is; the same `parity_mismatch` stands, the finding
  `repaired` is added and the repaired copy holds the restored bytes; the fold is
  formed, the row is hidden in the visible dataset, the value and the blob rebuild, and the content digest
  is `present`.
- Rule: `P7`, "Access: the steps by access", STEPS BY ACCESS and FIELDS BY ACCESS; "Reading as a stream".
- Corpus: `P7/stream-forward-and-whole` is the case: `forward`, then `whole`.

### FX-050 One input under the three accesses

- Input: one segment with a damaged region that takes an ordinary frame, the parity frame of the span after it, frames after
  that, a closing index, an `offsets` item and a locator.
- Built: the damage of `P5/recover-c`, in a file small enough for all three reads.
- Read: with `whole`; with `forward`, the end declared; and by a graph query with `sparse`.
- Result: `whole`: reading resumes at the parity frame, the later frames are `linked`, the first of them is detached, and
  `anchored_frames` is the count before the region. `forward`: no resumption; the region runs to the end of the input, and the
  later frames are not read. `sparse`: the rows of the frames that the index places, each frame verified locally, with no
  chain membership, no authentication and no `anchored_frames`.
- Rule: `P7`, "Access: the steps by access" and "Reading by range"; `P1`, "The predecessor and transparent frames".
- Corpus: `P7/read/three-accesses-after-region` reads one input with all three: `whole`, `forward`, `sparse`. `P5/recover-c` and
  `P7/read/resume` are the `whole` read of a larger input, and `P7/idx-local-after-parity` the `sparse` read of another; they
  agree.

## 9. `P7`: reading by range

### FX-051 An answer over a false `offsets` item

- Input: a file of two segments `S1` and `S2`, whose dictionaries have the same number of terms with different texts. The
  `offsets` item of `S2` is written again so that the places of its header and of its covered `terms` frames are those of the
  header and the `terms` frames of `S1`, one length covering the bytes between them and the next frame of `S2`. The index of `S2`
  is left as it is; the locator is written again at its new place, with the distance to the same index.
- Built: the `offsets` item is under no identifier, so the change alters no identifier.
- Read: the access `sparse`, with a query for a graph of `S2`; then the access `whole`.
- Result: `sparse`: every frame read is verified locally. The answer is `answered`, and its rows are the rows of `S2` resolved
  over the dictionary of `S1`: rows that no whole read gives. `whole`: the rows of `S2` over its own dictionary; the `offsets`
  item is checked where the reader meets it, gives `index_claim_failed` with `index_offsets`, and changes no state.
- Rule: `P7`, "Reading by range", WHAT AN ANSWER ESTABLISHES, AND WHAT IT DOES NOT; `P5`, "The `offsets` item and the locator".
- Corpus: `P7/idx-false-offsets-whole` is the case: `sparse`, then `whole`. Its input is that of `P7/idx-mixed-chains`.

### FX-052 The same query on the true file

- Input: the file of FX-051 before the `offsets` item is written again.
- Read: the access `sparse`, with the query of FX-051.
- Result: `answered`, with the rows that the whole read gives for that graph, a term by reference where the dictionary holds
  it by reference.
- Rule: `P7`, "Reading by range", THE ANSWER.
- Corpus: `P7/idx-true-offsets` is the case.

### FX-053 A query by range for a blob of several units

- Input: a canonical segment whose blob pack holds a blob `N` declared by a `recipe` member over three `chunk` members `u1`, `u2`,
  `u3`, and a blob `K` held by one `blob` member.
- Read: the access `sparse`, with three queries of the target `unit`: the digest of `N`; the digest of `u2`; the digest of
  `K`. Then the access `whole`.
- Result: the first query is `answered` with no item. The second is `answered` with the unit `u2`. The third is `answered`
  with the bytes of `K`. The whole read gives `N` in the blob set with its rebuild verified.
- Rule: `P7`, "Reading by range", THE QUERY and the step THE FRAMES OF THE TARGET; `P5`, "The `index` frame" (`pack_members`).
- Corpus: `P7/idx-unit-recipe-and-blob` is the case: `blob-of-three-units`, `second-unit`, `blob-of-one-unit`, then `whole`.

### FX-054 Joined requests, read-ahead and a read-ahead that fails

- Input: one file of five `terms` frames in the order `arrival`, one `quads` frame, a commit and an index, whose header makes no
  canonical claim, and one graph query.
- Read: three readers of equal capacity, with the access `sparse`. One asks for each range of the procedure alone. One asks
  for the last 64 KiB of the file in one request and joins adjacent ranges. One also holds the covered `terms` frames from an
  earlier answer over the same bytes. Variants: `decoded_bytes_per_read` set so that the procedure is refused at the third
  `terms` frame; a store that fails every request that overlaps a byte the procedure does not reach; a store that fails at an
  offset that it does reach.
- Result: the three answers are equal in every field. With the lowered capacity each is `unknown` with the reason `capacity`,
  at the same range. With the first failing store each is `answered`. With the second each is `unknown` with the reason
  `read`.
- Rule: `P7`, "Reading by range", CHARGES and THE PROCEDURE (a range is read when the procedure takes its bytes at its step).
- Corpus: `P7/idx-joined-readahead` is the case. It records the answer of the reader that asks for each range alone, and the
  variants; the three readers differ only in their requests.

## 10. `P7`: the canonical form, compaction and the profile mechanism

### FX-055 The compaction of a source that was read with every key

- Input: (a) the fold of the read of the file `X'` with every key, as in FX-069 (section 11), then `compact` with `profile`
  `absent`. (b) A file
  built as that of FX-073, but with the sealed frames of each audience holding a `terms` frame and a `quads` frame, an unsigned
  commit, and a clear `suppress` frame that hides one private row, read with both keys, so that `visible` is `complete`;
  then `compact` with `profile` `absent`.
- Result: `[1, file]` in both. The file is one clear segment with `canonical` `present`. It holds no `keys` and no `sealed`
  frame, and its commit has an empty `signatures` list. In (a) it holds the five lines of the four segments, among them the row
  `q`, and a `suppress` frame with the one entry that hides `q`; a keyless reader of it folds everything and reports `visible`
  `[1, 1]`. In
  (b) it holds the public rows, the rows of both private audiences and the hidden row.
- Application: the publication of this file is refused: no export scope was selected. The publishing workflow selects the
  scope before it compacts. For (a) the public release is the compaction of the file that holds the bytes of segment `0`
  alone, signed after compaction. The workflow does not sign a canonical file made from a read that opened sealed frames
  outside the scope, and it reports the scope of the source and the scope of the output apart.
- Rule: `P7`, "The construction, from a fold to the item sequence" and "Compaction"; `P8`, "The inner frame".
- Corpus: `P7/canonical/compaction/keyed-source` is (a) in its case `all-segments` and (b) in its case `two-audiences`.

### FX-056 The compaction of a read that refused a `terms` frame

- Input: a read that refuses a `terms` frame by the total formed length; then the compaction of its fold.
- Built: the input of the corpus entry named below.
- Result: the frame is `opaque` with `exceeds_capacity` and is counted in `left_out`. `compact` gives `[2, exceeds_capacity,
  present(total_formed)]`.
- Rule: `P7`, "Compaction" (case 1).
- Corpus: `P7/canonical/compaction/formed-refused` is the case and agrees; it records `left_out` `1`.
  `P9/convert/compaction-stop-formed` is the same stop in a conversion, with `content_mismatch`, and agrees.

### FX-057 One-pass extraction from two layouts (benchmark)

- Input: one file tree as a canonical file, and as a file that is not canonical, with the entries and the bindings before the
  `blob` frames.
- Result: what the rules give without measuring: in the canonical file no path is known before the last pack is read.
- Measured: the time to the first extracted file, the peak memory and the peak temporary storage. This entry is the input of
  the measurement of one-pass extraction in `GUIDE-adoption.md`.
- Rule: `P7`, "The construction, from a fold to the item sequence".
- Corpus: `PF-files/files-two-layouts` is the case; its case `canonical` is the input of `PF-files/files-pass`.

### FX-058 Content of one profile in a segment that declares another, and two profiles in one file

- Input: (a) one canonical segment that declares `dist` and whose fold holds a file tree in the vocabulary of `PF-files`, with
  two entries of equal paths. (b) A file of a `dist` segment followed by a segment that declares `evidence`. (c) A canonical
  segment that declares no profile and is within both ceilings of `PF-dist`.
- Result: (a) the checks of `dist` run, and none of `files`. The result is a VERIFIED DISTRIBUTION and holds no
  `files_duplicate_path`. (b) `dist_not_alone` in the first segment; the result is REFUTED. (c) no profile check runs; the
  result is REFUTED, since the `profile` of its segment is `absent`.
- Application: a contract that wants the path rules in (a), or the ceilings in (c), runs them as its own checks and reports
  them under its own name.
- Rule: `P7`, "The profile mechanism"; `PF-dist`, "Checks" and "Verified distribution".
- Corpus: `PF-dist/dist-other-profile-content` is (a) and (b); `PF-dist/dist-refuted-not-declared` is (c).

### FX-059 A profile value outside the profile table

- Input: a header whose `profile` is `present` with a value that the profile table does not hold.
- Result: the segment is read in full. No profile check runs. The one finding is `profile_not_checked`, of the severity
  `information`, with the subject `[4, profile, absent]`.
- Rule: `P7`, "The profile mechanism" (item 3).
- Corpus: `P7/profile/value-outside-table` is the case.

## 11. `P8`: sealing

FX-060 to FX-064 are built from the sealed frame of the worked example "A sealed frame, opened" of `P8`: an inner `pack` of
one member with the content `[1, 3]`. After each change a holder of the key seals the inner record again and the identifiers
are computed again. The read holds the segment key.

### FX-060 One fault of the inner type

- Input: `member_count` written `2`.
- Result: steps 1 to 9 of the opening pass. Step 10 gives `malformed_value` with `inconsistent_counts`. The frame is opaque
  with `malformed`, and its payload state is `verified`. It is not folded, it is counted in `unopened_sealed`, and no failed
  opening is counted.
- Rule: `P8`, "The procedure" (step 10) and "Mutations of that frame".
- Corpus: `P8/p8-inner-count` is the case.

### FX-061 Two faults: the count and the digest

- Input: `member_count` written `2`, and one byte of the inner `payload_digest` changed.
- Result: `damaged_frame` with `payload_digest` at step 9. The frame is opaque with `damaged`, and its payload state is
  `damaged`. The count is not judged, and the frame has no `malformed_value`.
- Rule: `P8`, "The procedure" (steps 9 and 10) and "Mutations of that frame".
- Corpus: `P8/p8-inner-count-and-digest` is the case.

### FX-062 Two faults: the count and a compressed payload that does not decode

- Input: `member_count` written `2`; the inner `storage` `[2, zstd]`; the inner `payload` a zstd frame that passes the zstd
  constraint and is cut short by one byte.
- Built: the cut payload is chosen so that the step of the payload check that it fails is the decoding, and not the zstd
  constraint.
- Result: `damaged_frame` with `decode_failed` at step 9. The frame is opaque with `damaged`, and its payload state is
  `damaged`. It has no `malformed_value`. A clear `pack` frame with the same two faults has the same finding and reason.
- Rule: `P8`, "The procedure" (steps 9 and 10); `P1`, "The payload check".
- Corpus: `P8/p8-inner-count-and-decode` is the case, in its cases `sealed` and `clear`.

### FX-063 A field rule of the inner type decides before the payload

- Input: an inner frame of the type `external` with `decoded_length` `1`, a payload of one byte and a wrong `payload_digest`.
- Result: `malformed_value` with `out_of_range` at step 8. The frame is opaque with `malformed`. Step 9 is not run, so the
  frame has no `damaged_frame`.
- Rule: `P8`, "The procedure" (step 8); `P4`, "The frames" (the `decoded_length` of an `external` frame).
- Corpus: `P8/p8-inner-external-length` is the case.

### FX-064 An unknown value and a failing payload, in a segment of a later registry version

- Input: a segment whose `registry_version` is above that of the reader; an inner `terms` content with an enumeration value
  that the reader does not hold; one byte of the inner `payload_digest` changed. Then the same without the changed byte.
- Result: first: `damaged_frame` with `payload_digest` at step 9, and no `unsupported_critical`. Second:
  `unsupported_critical` with the subject `[2, 18]`; the fold of the segment stops, every later frame is `stopped`, and the
  sealed frame counts in `unopened_sealed`.
- Rule: `P8`, "The sealed inner record" and "The procedure" (steps 9 and 10).
- Corpus: `P8/sealed-inner-newer-value-terms` is the case; `P8/sealed-inner-newer-value` is the same stop with an inner `quads`
  frame.

THE FILES `X` AND `X'`. FX-065 to FX-069 read these files. `P`, `A`, `B` and `H` are the signing keys of a publisher, of two
model services and of a human reviewer; `rA`, `rB` and `rH` are their recipient keys. No transparent frame is written.

The file `X` has four segments, each opened by a header with `lineage` `absent`. Segments `1` to `3` have `registry_version`
`2`.

- Segment `0`, public: `terms` (position `1`); `quads` (`2`), among its rows the row `q` = `(:claim1, :status, :accepted)`;
  `commit` (`3`), signed by `P`.
- Segment `1`, analysis A: `keys` (`1`) with one key entry and recipient entries for `rA` and `rH`; `sealed` (`2`), inner
  `terms`; `sealed` (`3`), inner `quads`; `commit` (`4`), signed by `A`.
- Segment `2`, analysis B: as segment `1`, with `rB` and `rH`, signed by `B`.
- Segment `3`, deliberation: `keys` (`1`) with one key entry and a recipient entry for `rH`; `sealed` (`2`), inner `terms`;
  `sealed` (`3`), inner `quads`; `commit` (`4`), signed by `H`.

The file `X'` is `X` with one more sealed frame in segment `3` before its commit: `sealed` (`4`), inner `suppress`, with one
entry of the kind `quad` whose four fields are the terms of `q`, defined in the dictionary of segment `3`. The commit is then
at the position `5`.

### FX-065 A public reader

- Input: `X`.
- Read: keyless; `trusted_keys` and `held_keys` hold `P`.
- Result: segment `0` is folded, its commit is valid and names a state, and its `authenticity` is `authenticated`. In each of
  segments `1` to `3`: one `segment_key_not_held` at the position `2` with the `count` `2`; both sealed frames opaque with
  `missing_key`; and the commit names no state. `visible` is `[2, 0, 6, 0]`. The fold of the file holds the lines of segment
  `0` and nothing else.
- Application: the scope "the state of the commit of segment `0`, signed by the publisher" is accepted. The scope "the file"
  is incomplete.
- Rule: `P8`, "The procedure" (step 3) and "Resolving the segment key of a key entry"; `P3`, "Graph states" and "The visible
  dataset and visibility"; `P6`, "Coverage and authentication".
- Corpus: `P8/p8-audiences` is the case, in its case `public`.

### FX-066 One private audience alone

- Input: `X`.
- Read: `sealing_input` holds the seed of `rA`, and `trusted_keys` holds `P` and `A`. A second read with `rB` and `B`.
- Result: segments `0` and `1` are folded, and both commits name states: the run of segment `1` is that segment alone.
  Segments `2` and `3` are as in FX-065. `visible` is `[2, 0, 4, 0]`. The second read gives the same with segments `1` and `2`
  exchanged: the inner `terms` frame of segment `2` has the expected base `0`, because the dictionary is that of its segment.
- Application: the scopes "the public state" and "analysis A, the state of the commit of segment `1`, signed by A" are
  accepted. "The file" is incomplete.
- Rule: as FX-065; `P3`, "The content record and the ids"; `P3`, "Graph states", THE RUN.
- Corpus: `P8/p8-audiences` is the case, in its cases `audience-a` and `audience-b`.

### FX-067 The reader with every key

- Input: `X`.
- Read: `sealing_input` holds the seed of `rH`; `trusted_keys` holds `P`, `A`, `B` and `H`.
- Result: every sealed frame opens, and its inner frame is folded. No `segment_key_not_held`. Every commit names a state.
  `visible` is `[1, 0]`. Every segment is `authenticated`.
- Application: the scope "the file" is accepted when the four roles are those required.
- Rule: `P8`, "The procedure" and "The inner frame".
- Corpus: `P8/p8-audiences` is the case, in its case `every-key`.

### FX-068 A keyless mirror

- Input: `X`. The mirror writes the bytes that it received to its output.
- Read: keyless; `held_keys` holds the public keys of `P`, `A`, `B` and `H`; `trusted_keys` is empty.
- Result: the output equals the input byte for byte. Every link holds, and every commit is valid. Every signature has the
  status `valid` and `authenticated` `false`. Every sealed frame passes steps 1 and 2 of the opening and is opaque with
  `missing_key`. `visible` is `[2, 0, 6, 0]`.
- Application: "the history of the ciphertext is intact and signed by these four keys" is accepted. No scope of plaintext is
  accepted or refused.
- Rule: `P8`, "What it can do" and "The procedure"; `P6`, "The typed inputs of a verifier" and "Coverage and authentication".
- Corpus: `P8/p8-audiences` is the case, in its case `mirror`.

### FX-069 A private suppression that the public cannot open

- Input: `X'`.
- Read: once as FX-065, once as FX-067.
- Result: keyless: the row `q` is in the visible dataset, and `visible` is `[2, 0, 7, 0]`. With every key: the row `q` is in
  the fold and hidden; the suppression record holds one entry; `visible` is `[1, 0]`.
- Application: keyless: the scope "the state of the commit of segment `0`" is accepted as that state. A claim that the fold is
  the current public view is refused, because `visible` is `incomplete`.
- Rule: `P3`, "Suppression hides" (an entry acts whatever segment holds it) and "The visible dataset and visibility"; `P8`,
  "The inner frame".
- Corpus: `P8/p8-audiences-suppress` is the case.

### FX-070 A private dictionary before public terms

- Input: (a) one segment of `registry_version` `2`: `keys` (position `1`); `sealed` (`2`), inner `terms` of three terms with
  `id_base` `0`; a clear `terms` (`3`) of three terms with `id_base` `3`; a clear `quads` (`4`) with one row that names the ids
  `3`, `4` and `5`; and a variant of (a) whose `quads` frame holds a second row that names the ids `0`, `1` and `2`. (b) The same
  content in another order: the clear `terms` frame first, with `id_base` `0`, and the
  clear rows; then the `keys` frame; then the sealed inner `terms` frame with `id_base` `3`; then sealed rows.
- Read: each input with the key and keyless.
- Result: (a) with the key: the expected base of the clear `terms` frame is `3`; every frame is folded, and one line is
  delivered. (a) keyless: frame `2` is opaque with `missing_key`. Frame `3` has the expected base `0`: it is `malformed_value`
  with `inconsistent_counts`, opaque with `malformed` and not folded, and the three ids `0` to `2` that it reserves are
  unreadable terms. The row of frame `4` names ids that are not defined at its frame and is dropped with `forward_reference`.
  No line is delivered, and `visible` is `incomplete`. In the variant, read keyless, the second row names ids that the refused
  frame reserves and is held with `opaque_term`, and the content digest is `absent`; read with the key, both rows are lines. (b)
  keyless: the clear `terms` frame is folded, every row that names only its ids is a line, and the sealed
  frames are not opened. (b) with the key: all rows are lines.
- Application: the contract prescribes layout (b), or a clear dictionary shared by all audiences, or a segment for each
  audience. It refuses layout (a) at writing.
- Rule: `P3`, "The content record and the ids", "Unreadable terms" and "What happens to a row"; `P8`, "The inner frame"; `P8`,
  "What is not claimed" (the item on term ids after an unopened inner `terms` frame).
- Corpus: `P8/p8-private-dictionary` is the case: `first-key` and `first-keyless` are (a), `first-held-key` and
  `first-held-keyless` its variant, `last-keyless` and `last-key` are (b).

### FX-071 A rotated audience

- Input: one segment: a clear `terms` (position `1`); `keys` (`2`) with the key entry of index `0` and recipient entries for
  `r1` and `r2`; `sealed` (`3`) under the key index `0`, inner `quads`; `keys` (`4`) with the key entry of index `1` and
  recipient entries for `r1` and `r3`; `sealed` (`5`) under the key index `1`, inner `quads`. A variant: the `keys` frame at
  the position `4` holds in addition a recipient entry for `r3` whose target is the key entry of index `0`.
- Read: with the seed of `r1`, of `r2` and of `r3`.
- Result: `r1` opens both sealed frames. `r2` opens frame `3` and has `segment_key_not_held` at frame `5` with the `count`
  `1`. `r3` has the reverse, at frame `3`. In the variant the added entry is `malformed_value` with `unordered_or_repeated`
  and is not tried; the rest of the frame is applied; `r3` still does not open frame `3`.
- Rule: `P8`, "The key table of a segment" and "Rules of recipient entries".
- Corpus: `P8/p8-rotated-audience` is the case.

### FX-072 Two recipient entries that wrap different keys

- Input: one segment: a clear `terms` (position `1`); `keys` (`2`) with one key entry and two recipient entries, for `r1`
  wrapping the key `K` and for `r2` wrapping another 32-byte key `K2`; `sealed` (`3`), sealed under `K`.
- Read: with both seeds; with the seed of `r1` alone; with the seed of `r2` alone.
- Result: with both seeds: `key_conflict` at frame `3`; the frame is opaque with `missing_key`; no opening is counted. With
  `r1` alone: the frame opens. With `r2` alone: `seal_failed` with `aead`; the frame is opaque with `damaged`, its payload
  state is `damaged`, and one failed opening is counted.
- Rule: `P8`, "Resolving the segment key of a key entry"; "The procedure" (step 6); "Commitment".
- Corpus: `P8/p8-recipient-conflict` is the case.

### FX-073 Public content and two audiences that do not read each other

- Input: one segment of registry version `2`: a clear `terms` frame and clear rows, the public content; a `keys` frame with
  the key entry of the audience `A`, immediately before the first sealed frame under it; sealed frames of rows under that key;
  a `keys` frame with the key entry of the audience `B`, immediately before the first sealed frame under it; sealed frames of
  rows under that key; a signed commit. The sealed rows name only terms of the clear dictionary.
- Read: four times: keyless, with the segment key of `A`, with the segment key of `B`, with both segment keys.
- Result: keyless: the fold holds the public rows, both groups of sealed frames are not opened, and `visible` is `incomplete`,
  with `unopened_sealed` the number of sealed frames. With the key of `A`: the fold holds the public rows and those of `A`,
  and `visible` is `incomplete` with the count of the frames of `B`. With both keys: `visible` is `complete`. The commit after
  the sealed frames names a state only for the read with both keys.
- Application: each audience is told the scope that it was promised, and that its view is not the whole. No reader reports a
  complete view while `unopened_sealed` is above `0`.
- Rule: `P3`, "The visible dataset and visibility" and "Graph states"; `P8`, "The key table of a segment" and "Opening a
  sealed frame"; `P8`, "What is not claimed" (AGREEMENT BETWEEN RECIPIENTS).
- Corpus: `P8/p8-two-audiences` is the case.

### FX-074 Key grouping (benchmark)

- Input: the layout of FX-071 with `n` sealed frames and `r` recipients, once with one key entry for all the frames and once
  with a key entry for each sealed frame.
- Result: what the rules give without measuring: a recipient entry takes `1,243` bytes, once for each recipient of each key.
  The recipient entries take `1,243 * r` bytes in the first layout and `1,243 * r * n` in the second.
- Measured: the bytes of the `keys` frames as a share of the file, and the time to open. This entry is the input of the
  measurement of key grouping in `GUIDE-adoption.md`.
- Rule: `P8`, "A recipient entry and the wrap's key schedule".

## 12. `P9`: conversion from the first generation

The sources of FX-075 to FX-080 are files of the first generation, written by the first-generation writer of the corpus.

### FX-075 One blank-node label in a snapshot and outside it

- Input: a source with the `terms` `a`, `p` and the blank `b`, and the `quads` row `(a, p, _:b)`; and a `snapshot` with the
  `terms` `c`, `p` and the blank `b`, and the `quads` row `(c, p, _:b)`.
- Built: the input file of the corpus entry named below (521 bytes).
- Result: two blank nodes. The lines are `quad(a, p, _:1)` and `quad(c, p, _:2)`. The conversion proof holds. The content
  digest is that of two nodes.
- Rule: `P9`, "The conversion reading" (SNAPSHOT SCOPE AND IDS) and "Blank nodes".
- Corpus: `P9/convert/snapshot-label-one-scope` is the case.

### FX-076 One label in two snapshots

- Input: a source with two `snapshot` frames, each with a blank term of the label `b` and one row that names it. A second input in
  which the payloads of the two snapshots are equal.
- Result: two nodes in each input.
- Rule: `P9`, "The conversion reading" (the scope of a blank-node term entry of a snapshot is that `snapshot` frame).
- Corpus: `P9/convert/snapshot-label-two-snapshots` is the case, in its cases `different` and `identical`.

### FX-077 One label twice in one snapshot

- Input: a source with one `snapshot` whose `terms` hold two entries of the blank label `b`, each the subject of one row, the two
  rows differing in their object.
- Result: one node and two lines.
- Rule: `P9`, "Blank nodes" (two entries of one scope with one label that is not empty are one node).
- Corpus: `P9/convert/snapshot-blank-same-label-twice` is the case.

### FX-078 Anonymous blank terms in a segment and in two snapshots

- Input: a source with an anonymous blank term, once with `"v"` absent and once with `"v"` empty, in a frame of the segment,
  in one snapshot and in a second snapshot, each named by one row.
- Result: one node for each term entry: six nodes.
- Rule: `P9`, "The conversion reading" and "Blank nodes".
- Corpus: `P9/convert/anonymous-blank-snapshots` is the case.

### FX-079 A triple term over a blank node, in a segment and in a snapshot

- Input: a source whose segment and whose snapshot each hold a blank term `x` and a term of kind `3`, `<<( _:x p o )>>`, over
  it, each used as the object of one row.
- Result: two `triple` terms over two nodes, and two lines that differ in their object.
- Rule: `P9`, "Terms" and "Blank nodes".
- Corpus: `P9/convert/snapshot-triple-over-blank` is the case.

### FX-080 A suppression of the node of the segment

- Input: the source of FX-075 with the rows `(_:b, p, a)` in the segment and `(_:b, p, c)` in the snapshot, and a `suppress`
  frame after the snapshot with a `term` target on the id of the entry of `b` in the segment.
- Result: one `term` entry on the node of the segment. The line of that node is hidden. The line of the node of the snapshot
  is visible. The visibility check passes, and the proof holds.
- Rule: `P9`, "The conversion reading" (an id names the node of its entry) and "Suppression"; `P3`, "The hiding rule".
- Corpus: `P9/convert/suppress-term-of-segment-node` is the case.

### FX-081 One source converted with and without the reference encoder

- Input: one source, converted by a converter that holds the reference encoder and by one that does not.
- Built: the input of the corpus entry named below.
- Result: the same proof and the same head. The stored bytes differ. A reader of either result reports the storage level
  `verified`, `differs` or `unchecked`, with no finding.
- Rule: `P9`, "The conversion function"; `P7`, "The reference encoder and the bytes claim".
- Corpus: `P9/convert/storage-two-converters` is the case. `P9/convert/total-no-option` converts the same source and agrees.
  `P9/convert/storage-differs` records such a conversion and no read.

## 13. `PF-bundle`: nested files

### FX-082 A valid outer bundle with a damaged, an untrusted and an unopened child

- Input: an outer file that declares `bundle`, intact and signed by a trusted key, with three nested-file terms bound to blobs
  that are present and rebuild. Child `x`: a file of generation 2 of one segment, in which one byte of the stored payload of
  the `quads` frame was changed after the file was written; the outer blob is declared over the changed bytes. Child `y`:
  intact, signed by a key outside the trusted set. Child `z`: of registry version `2`, with content sealed under a key that
  the reader does not hold.
- Result: the outer result has no `profile_violation`: each term passes checks `1` to `5`. Its `integrity` is `intact` and its
  `authenticity` `authenticated`. `nested_results` holds three records of the depth `1`. For `x`: a result with
  `damaged_frame` at the `quads` frame and `integrity` `damaged`. For `y`: `authenticity` `unauthenticated`. For `z`:
  `visible` `incomplete`, with `unopened_sealed` above `0`.
- Application: a contract that requires the three children accepts none of them on the outer result. It refuses `x`. It
  refuses `y` unless its signer is entitled. It reports `z` as carried and not opened.
- Rule: `PF-bundle`, "What a reader reports about nested content" and "Checks".

### FX-083 Two nested files that share one total

- Input: two nested files of the depth `1` that each need a charge `T` of the total rebuilt length.
- Built: the input of the corpus entry named below.
- Read: `rebuilt_bytes_per_read` equal to what the outer read uses plus `T`.
- Result: the first nested read, in ascending `E(name)`, is made with `T`. The second is made with `0` and is limited by its
  capacity. Each nested result states its capacity.
- Rule: `PF-bundle`, "What a reader reports about nested content" (THE SHARED ALLOWANCE and THE ORDER OF THE NESTED READS).
- Corpus: `PF-bundle/bundle-shared-allowance` is the case and agrees. `PF-bundle/bundle-shared-items`,
  `PF-bundle/bundle-shared-formed`, `PF-bundle/bundle-shared-labelled` and `PF-bundle/bundle-read-order` hold the other totals
  and the order; all agree.

## 14. `PF-container`: container archive

### FX-084 An unreadable observation with a status

- Input: an export with an observation of the kind `c:Unreadable` whose `c:status` is a count literal: once `200`, once `503`.
- Built: each case is the input of a corpus entry named below.
- Result: no finding of the profile.
- Rule: `PF-container`, "Properties of an observation" and "What each kind holds" (`Unreadable`: at most one `status`).
- Corpus: `PF-container/container-observation-status-typed` is the case with the status `200` and
  `PF-container/container-observation-status-503` the case with `503`.

### FX-085 An unreadable observation with no status

- Input: the export of FX-084 with an `Unreadable` observation that has no `c:status`. A retrieval that timed out before the
  headers and a connection that failed give this one record.
- Built: the status statement is omitted.
- Result: no finding of the profile. The record differs from that of FX-084 and from a tombstone.
- Rule: `PF-container`, "What each kind holds" (`Unreadable`: at most one `status`) and "Properties of an observation" (no status
  when no response was received).
- Corpus: `PF-container/container-observation-status-absent` is the case.

### FX-086 A resource observed as gone

- Input: an export in which a resource is observed as gone: the kind `c:Absent`, no `status` and no graph.
- Built: the input of the corpus entry named below.
- Result: the current observation of the resource is the tombstone. No finding.
- Rule: `PF-container`, "What each kind holds" (`Absent`).
- Corpus: `PF-container/container-current-latest` is the case and agrees. `PF-container/container-kind-absent-graph` is the
  tombstone that breaks its kind and agrees.

## 15. `PF-dist`: distribution file

### FX-087 A frame above the decoded ceiling, read with each number of checks

- Input: one canonical segment that declares `dist`, whose `terms` frame has a `decoded_length` of `67108865`, one byte above
  the decoded ceiling. A fold with one IRI whose text is long enough gives such a file.
- Built: the input file of `PF-dist/dist-decoded-size` (3873 bytes).
- Read: five times, with `decoded_bytes_per_frame` at least `67108865` and `profiles` `[[1, k]]` for `k` from `1` to `5`.
- Result: `k` = `1` and `2`: checks `3` to `5` are not run; no `profile_violation`; the result is not a VERIFIED DISTRIBUTION
  and is INCONCLUSIVE. `k` = `3`, `4` and `5`: one `profile_violation` with the subject `[4, 1, present(3)]` at the `terms`
  frame, with the `count` `67108865`; the canonical result has structure and labelling `verified`; the result is REFUTED.
- Rule: `PF-dist`, "Checks" and "Verified distribution".
- Corpus: `PF-dist/dist-decoded-size-checks-prefix` is the case; its case `checks-k` is the read with `profiles` `[[1, k]]`.
  `PF-dist/dist-decoded-size` is the same file read at the minimum capacity with `profiles` `[[1, 5]]` (FX-089).

### FX-088 A valid distribution file read with fewer checks than the profile numbers

- Input: a valid distribution file within every ceiling.
- Built: the input file of `PF-dist/dist-pass`.
- Read: four times, with `profiles` `[[1, k]]` for `k` from `1` to `4`.
- Result: no finding of the profile. `canonical` is verified. The result is not a VERIFIED DISTRIBUTION, because `checks` is
  below the number of the checks. It shows no refuting fact, so it is INCONCLUSIVE.
- Rule: `PF-dist`, "Verified distribution" (item 2, and the causes of INCONCLUSIVE).
- Corpus: `PF-dist/dist-inconclusive-checks-prefix` is the case; its case `checks-k` is the read with `profiles` `[[1, k]]`.
  `PF-dist/dist-verified` is the same file with `[[1, 5]]`, a VERIFIED DISTRIBUTION. The note of
  `P2/registry/profile/checks-prefix` states the rule of the prefix; that entry records the control alone.

### FX-089 The frame above the ceiling, read at the minimum capacity with all five checks

- Input: the file of FX-087.
- Built: the input of the corpus entry named below.
- Read: the default read, with `profiles` `[[1, 5]]`.
- Result: `exceeds_capacity` with `decoded_size`, and beside it `profile_violation` with the subject `[4, 1, present(3)]` and
  the `count` `67108865`. The result is REFUTED.
- Rule: `PF-dist`, "Checks" (check `3`, and Overlaps).
- Corpus: `PF-dist/dist-decoded-size` is the case and agrees.

### FX-090 A reader that does not implement the profile

- Input: a valid distribution file; in a second case, the file of FX-087.
- Built: each case is the input of a corpus entry named below.
- Read: `profiles` is empty.
- Result: `profile_not_checked` with the subject `[4, 1, absent]`. `profiles` holds no pair of the profile `1`. The result is
  INCONCLUSIVE in both cases. In the second case, read at the minimum capacity, the `terms` frame also has `exceeds_capacity` with
  `decoded_size`, the `quads` frame `opaque_term`, and both content digests are `absent`.
- Rule: `P7`, "The profile mechanism"; `PF-dist`, "Verified distribution".
- Corpus: `PF-dist/dist-inconclusive-not-implemented` is the first case and
  `PF-dist/dist-inconclusive-not-implemented-decoded-size` the second.

### FX-091 A result that states more checks than the profile numbers

- Input: the `read_result` of a valid distribution file with `profiles` `[[1, 6]]`, judged by the predicate. A second half,
  for a revision of the profile that appends a sixth check: a result with `[[1, 5]]`, judged by the predicate of that
  revision.
- Built: the result record of `PF-dist/dist-verified` is encoded with `checks` `6`. The second half is built when a revision
  appends a check.
- Result: first: a VERIFIED DISTRIBUTION, since `6` is at least the number of the checks: the predicate reads the field `checks`
  and nothing else of it. Second:
  INCONCLUSIVE.
- Rule: `PF-dist`, "Verified distribution" (item 2 names the number of the checks of the profile, and not `5`).
- Corpus: no entry judges a result whose `checks` differs from `5`. The nearest is `PF-dist/dist-verified`.

### FX-092 A writer asked for the distribution file of a fold that has none

- Input: the fold of FX-087, and a request to a writer for its distribution file.
- Result: the writer writes no distribution file: the fold has none. It does not hand the canonical file that compaction returns
  to its caller as one.
- Rule: `PF-dist`, "Constants" (what follows from the ceilings) and "Writer rules".

### FX-093 A frame exactly at the ceiling

- Input: as FX-087 and FX-092, with a `terms` frame whose `decoded_length` is exactly `67108864`.
- Result: the writer writes the file. A read gives no `profile_violation`, and the result of a reader that claims the minimum
  capacity is a VERIFIED DISTRIBUTION.
- Rule: `PF-dist`, "Constants" (`Cd`) and "Verified distribution"; `P7`, "Capacity", MINIMUM CAPACITY.
- Corpus: `PF-dist/dist-decoded-size-pass` is the read. No entry tests the writer.

### FX-094 A verified distribution whose required blob is external

- Input: one canonical segment that declares `dist`. Its fold holds one `external` declaration of a blob named `N` (name,
  length, media type), whose bytes are not in the file, and one binding of the term `:module` of the manifest to the `blob`
  referent `[1, N]`.
- Result: the blob set holds `N` with `presence` `external`, and the state of the blob is EXTERNAL. The binding resolves to
  it, with no `missing_dependency`. `integrity` is `intact`. The result is a VERIFIED DISTRIBUTION.
- Application: a capsule contract that requires the module offline asks for `presence` `present` and a verified rebuild of
  every blob that the manifest requires. Its outcome is incomplete: the required blob is not COMPLETE. The two outcomes
  differ, and both are reported.
- Rule: `PF-dist`, "Verified distribution"; `P4`, "The frames", "The blob set" and "The rebuild"; `P3`, "Typed references: the
  `bindings` frame".
- Corpus: `PF-dist/dist-verified-external-blob` is the case. The blob set is content and stands in no `read_result`.

### FX-095 A capsule with two module versions and mixed manifests

- Input: one canonical segment that declares `dist`: two modules as the blobs `w1` and `w2`; two schema documents `t1` and
  `t2` and two permission documents `q1` and `q2` as blobs; a manifest in RDF whose release names three part terms, bound to `w2`,
  `t1` and `q2`; a second manifest whose release names one part term, bound to `w1`. Variants with exactly one manifest whose
  three parts agree.
- Result: every variant is a VERIFIED DISTRIBUTION by one read. The profile judges the segment and its frames and reads no
  manifest.
- Application: the contract of the host requires exactly one manifest, and that the versions stated in the module, the schema
  and the permissions agree. It refuses the input with two manifests as ambiguous and the manifest of `w2`, `t1` and `q2` as
  mixed, and it accepts the consistent variant.
- Rule: `PF-dist`, "Verified distribution" (a predicate over the fields of the result).
- Corpus: `PF-dist/dist-verified-capsule-manifests` is the case: `two-manifests` is the input with two manifests, and
  `one-manifest-version-1` and `one-manifest-version-2` are the consistent variants.

### FX-096 The largest release as a distribution (benchmark)

- Input: the largest ontology release that is to be published, with its attachments, compacted with `profile` `present(dist)`.
- Result: what the rules give without measuring: the fold has no distribution file when a frame of its canonical file is above
  `2^26` bytes.
- Measured: the time and the peak memory of the labelling and of the canonical check, and the largest `decoded_length` of a
  frame. This entry is the input of the measurement of the largest release in `GUIDE-adoption.md`.
- Rule: `PF-dist`, "Constants"; `P7`, "The canonical frames that can exceed the minimum capacity".

## 16. `PF-evidence`: signed custody chain

### FX-097 Every signature replaced by that of a third trusted key

- Input: a segment that declares `evidence`, with three commits, the first two signed by the key `K1` and the third by the key
  `K2`. Then the same segment with every `signatures` list replaced by one signature of a third key `K3` over the same commit,
  and the `offsets` item and the locator written again.
- Built: the signatures are of `mldsa65_ed25519`, under three fixed test keys. The second file is a storage rewrite of the
  first.
- Read: `trusted_keys` holds `K1`, `K2` and `K3`, and `held_keys` their public keys.
- Result: both files: no `profile_violation` of checks `4`, `5` or `6`; the same header identifier, effective identifiers,
  roots and content digest; every commit authenticated. The two results differ only in the `key_id` of the `signature_result`
  records. Nothing in the second result shows that `K1` or `K2` signed.
- Rule: `PF-evidence`, "Checks" (checks `4` and `5`, and WHAT THE CHECKS ESTABLISH); `P6`, "The signature record".

### FX-098 A signer list of the consumer finds the replacement

- Input: the two results of FX-097, and a signer list that the consumer holds outside the file: commits one and two by `K1`,
  commit three by `K2`.
- Result: no result of the format changes.
- Application: the first file passes the rule of the consumer. The second fails it at each commit, though no check of the
  profile fails. This entry tests a contract of the application, and no reader.
- Rule: `P6`, "Co-signing and pinning" (whether a set of signers suffices is a rule of the caller).

### FX-099 A canonical file under `evidence` above the cadence

- Input: the canonical file, with `profile` `present(evidence)`, of a fold whose canonical file has 1,030 ordinary frames
  before its commit. The commit is then signed with `mldsa65_ed25519` by a trusted key.
- Result: `profile_violation` for check `2`, `evidence_commit_interval`, at the position of the frame that is the 1,025th of
  the commit interval, with the `count` `1030`.
- Application: the file is not accepted as a custody chain. The append log is.
- Rule: `PF-evidence`, "Writer rules" (Canonical file) and "Checks".

### FX-100 A suppression by a key that authenticates and is not entitled

- Input: a file of the profile `evidence`, of three segments with checked lineage. Segment `0`: the claims, signed by a model
  key `A`. Segment `1`: the decision of the reviewer, with the row `q`, signed by `H`. Segment `2`: a `suppress` frame with a
  `quad` entry that names `q`, before a commit signed by `A`. A variant: the `suppress` frame stands after the last commit of
  its segment and is covered by no signature.
- Read: `trusted_keys` holds `A` and `H`.
- Result: `q` is hidden. `visible` is `[1, 0]`. No `profile_violation`. In the variant: `q` is hidden, `visible` is `[1, 1]`,
  and `profile_violation` is given for checks `3` and `7`.
- Application: under a table of roles in which the reviewer alone is entitled to hide a decision, the suppression is not
  applied to the accepted view, `q` stands in it, and the event is reported.
- Rule: `P3`, "The visible dataset and visibility"; `PF-evidence`, "Checks".

## 17. `PF-files`: file tree

### FX-101 The root of a tree

- Input: four files that declare `files`. (a) No entry. (b) The tree of the worked example of `PF-files`, with one more entry
  `_:r` of the kind `Directory` whose `f:path` object is `rdf:nil`, the empty path, with an owner and a time. (c) As (b), with
  the kind `RegularFile` on `_:r`. (d) No entry, and a statement of an owner and a time on a term that is no entry.
- Result: (a) no finding of the profile: the empty tree. (b) `_:r` is the root entry; check `1`, `files_entry_path`, and check
  `5`, `files_kind`, do not fail for it, and it states the owner and the time of the root of the tree. (c) `profile_violation`
  for check `5` with the `count` `1`. (d) `profile_violation` for check `8`, `files_property_domain`.
- Application: an archiver that is asked to keep the metadata of the archived directory writes the root entry. An extractor
  gives a tree with no root entry the defaults of the extractor for its root.
- Rule: `PF-files`, "Paths" (the empty path and the root entry) and "Checks".

### FX-102 A hard link whose target is hidden

- Input: a `RegularFile` entry, a `HardLink` entry whose `f:hardLinkOf` names it, then a segment that hides the regular file
  by a `term` entry and leaves the hard link.
- Result: before the hiding, `files_hard_link` does not fail. After it, the hard link has no visible target and fails check
  `7`, `files_property_cardinality`.
- Rule: `PF-files`, "Writer rules" (Removal and change) and "Checks".

### FX-103 Two writers replace one path

- Input: a base segment with an entry `:e0` of the kind `RegularFile` at the path of the one component `a.txt`. Replica 1
  appends a segment that hides `:e0` by a `term` entry and writes `:e1` at the same path. Replica 2 does the same with `:e2`.
  The file is the base followed by both segments. Then a further segment that hides `:e2`.
- Result: `:e1` and `:e2` are visible, and `:e0` is hidden. `profile_violation` for check `3`, `files_duplicate_path`, with
  the `count` `2`. A listing gives both, ordered by the place of their terms. After the further segment, check `3` passes.
- Application: a conflict to be decided, and not a version chosen. The contract refuses the archive, or picks by a stated
  rule, or a further segment hides one of the two.
- Rule: `PF-files`, "Writer rules" (Removal and change), "Paths" (THE ENTRY ORDER) and "Checks"; `P3`, "Suppression hides".

### FX-104 Names that a file system cannot take as they are

- Input: a tree with regular files in one directory whose last components are the octets `61 2f 62`, `61 00 62` and `2e 2e 2f
  78`; two names that differ only in the case of a letter; and a name whose octets are not UTF-8.
- Result: every check of `files` passes. Each component is valid: a component is refused only when it is empty, `2e` or `2e 2e`.
- Application: under an extraction contract each entry that the host cannot hold as stored is refused, or mapped by a stated
  rule, and reported: a name that the host cannot hold, or two names that the host treats as one. Nothing is written outside
  the destination, and nothing under another name silently.
- Rule: `PF-files`, "Paths" and "Writer rules" (A COMPONENT IS A NAME, NOT A PATH).

### FX-105 A name outside the domain of an interface of NUL-terminated UTF-8 paths

- Input: the tree of the last paragraph of the worked example of `PF-files`: a file whose one component is the octets `ff 41`.
- Result: the component is valid; its generated literal is `"FF41"^^xsd:hexBinary`.
- Application: through a binding whose paths are NUL-terminated UTF-8 strings, the entry is refused and reported, or mapped by
  a stated reversible encoding and reported, the same on every call. The listing by octets gives `ff 41`. `GUIDE-migration.md`
  describes the two behaviours.
- Rule: `PF-files`, "Literal forms", "Paths" and "Worked example".

## 18. Inputs whose result no rule of the format gives

### FX-106 Adapters, one family for each

- Input: (a) OKF Markdown: a bundle, the file made from it, and the bundle made from that file, with the mapping laws of the
  adapter's own document. (b) An inspection dump: a file with every frame type of registry version `1`, and the directory that
  it gives, with the hidden content shown and not shown. (c) Tar: a tree with a symbolic link that leaves the tree, an
  absolute name and a name with `..`, extracted with the refusals of the adapter. (d) Each RDF text syntax: the fold of FX-023
  written and read again, with the loss report of that syntax.
- Result: for each, the output that the specification of the adapter for this generation gives, and its loss report. None is
  derived from the core alone.
- Rule: the adapter's own specification; `P3`, "The RDF projection of a fold", for what an adapter can receive from a fold.

### FX-107 Cold creation of an archive (benchmark)

- Input: a tree of 10,000 files with a file of 1 GiB, and a tree of 1,000,000 small files.
- Result: nothing follows from the rules alone.
- Measured: the time, the peak memory and the output size of creating an archive from a file system. This entry is the input
  of the measurement of archive creation in `GUIDE-adoption.md`.

## 19. The predicate of a distribution, by number of checks

The outcome of the predicate of `PF-dist`, "Verified distribution", for a read with the access `whole` and a capacity that refuses
nothing of the file. The columns are the `profiles` of the read.

| file | `[1, 1]` | `[1, 2]` | `[1, 3]` | `[1, 4]` | `[1, 5]` | no pair of profile `1` |
|---|---|---|---|---|---|---|
| canonical, one frame above `Cd` | INCONCLUSIVE | INCONCLUSIVE | REFUTED | REFUTED | REFUTED | INCONCLUSIVE |
| canonical, within every ceiling | INCONCLUSIVE | INCONCLUSIVE | INCONCLUSIVE | INCONCLUSIVE | VERIFIED | INCONCLUSIVE |

In the first row the columns `[1, 1]` and `[1, 2]` show no violation, and the three REFUTED cells show the violation of check
`3`. In the last column both results hold `profile_not_checked`.

| cells | entry |
|---|---|
| first row, the five numbered columns | FX-087 |
| first row, last column | FX-090, second case |
| second row, `[1, 1]` to `[1, 4]` | FX-088 |
| second row, `[1, 5]` | `PF-dist/dist-verified` of the corpus |
| second row, last column | FX-090, first case |

## 20. Witness and neighbour

For a rule that separates two outcomes, the corpus holds the smallest input on the failing side and the nearest input on the
other side. The second shows that the rule refuses no more than it says.

| rule | smallest witness | nearest neighbour on the other side |
|---|---|---|
| `P1`, `P6`: a frame after a resumption with a false `anchor` is detached | FX-034 | FX-035 |
| `PF-dist`: verified only with every check run | FX-088 at `[[1, 4]]` | `PF-dist/dist-verified` |
| `P9`: a snapshot is a scope of blank nodes | FX-075 | FX-077; `P9/convert/blank-same-label-twice` |
| `P5`: only `consistency_diverged` shows two histories | FX-029 | `P5/proof-consistency-negative`, case `two-roots-one-count` |
| `P5`: a file proof of a source with no segment holds | FX-030 | `P5/proof-file` |
| `P3`: an ill-typed literal is a literal of RDF | FX-018 | `P3/project/not-rdf-texts` |
| `PF-container`: no status when no response was received | FX-085 | `PF-container/container-observation-status-typed` |
| `P3`, `P7`: equal folds can differ in what the read holds | FX-016 | `P3/content-digest-lines-only` |
