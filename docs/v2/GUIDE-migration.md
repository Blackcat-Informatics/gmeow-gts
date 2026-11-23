<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Guide: Migration from generation 1 to generation 2

This document is informative. It states no requirement of the format. The normative documents govern, and where this document
and they differ, they are right. It maps the features, interfaces, examples and schemas of generation 1 to generation 2, says
what changes for each, and gives the order of a migration. It cites the normative documents by part and section title.

## 1. Scope

Generation 1 is the format of the first-generation specification that `P9` cites. Generation 2 is the format of the
documents in this directory. The two are different formats: a file of one is not a file of the other, and a feature of
generation 1 has a counterpart in generation 2 only where the map below names one.

`P9` defines the conversion of a file; this document does not. It says where each feature stands afterwards and what a
user of the feature does.

## 2. A file of generation 1 in a reader of generation 2

A reader of generation 2 decides by the first item of a file (`P1`, "The generation"). `P9`, "The reader obligation", gives
the two claims a reader makes.

| reader claim | a file whose first item is a generation-1 map with `"v"` `1` |
|---|---|
| `second_only` | the finding `unsupported_generation`; nothing after the first item is read, translated or guessed |
| `first_and_second` | the generation-1 reader gets the input; its result returns unchanged; no rule of generation 2 applies |

A file holds the segments of one generation only. A generation-1 header after the first item of a generation-2 file is an
opaque item. The bytes of a generation-1 file followed by the bytes of a generation-2 file are a generation-1 file with
trailing items (`P9`, "The reader obligation").

A user who has to keep reading old files without converting them needs a reader that claims `first_and_second`. A user who
has to read the content in generation 2 converts the file (section 10).

## 3. The map at a glance

| generation 1 | generation 2 | kind of change |
|---|---|---|
| header map `{gts, v, prof, cat, layout, dct, meta, id}` | header record | replaced; each key is structure or content |
| profile named by a text | one value of the profile table | replaced |
| transform chain `x` and catalogue `cat` | `storage` of the envelope | replaced; one codec |
| COSE signatures `sig` | signature record | replaced; not carried by conversion |
| COSE Encrypt0, recipients `to` | `keys` and `sealed` frames | replaced; not carried by conversion |
| opaque node | opaque frame with a reason | typed |
| payload that is a file | nested file under `bundle` | typed |
| header and frame `meta` | RDF content | content |
| `fold` and `from-nq` | the RDF projection and import | not inverses |
| `verify`, `extract-key` | typed inputs of a verifier; none in the core | replaced; none |
| `prove`, `verify-proof` | typed proof records | replaced |
| `heads`, `segments`, `missing` | heads, results, proofs | replaced |
| `resume --after` | a cursor of ordinal and position | replaced |
| `extract` | whole read and blob set | replaced |
| `cat` | concatenation | same act, other fold |
| `compact` | `compact(S, profile)` | one parameter |
| `pack`, `unpack`, `diff`, `ls` | the profile `files` | tree defined; tools outside |
| streaming reader | the access `forward` | other result |
| `info` | the `read_result` | replaced |
| agent-memory helpers | content patterns | see section 6 |
| path helpers of a C interface | components of `files` | see section 7 |
| Markdown round trip, dump directory, tar, relational exports, RDF syntaxes | adapters over a fold | see section 8 |

## 4. Features of the format

### The header map

Generation 1 writes the header as a map. Generation 2 writes the record `[item_kind, generation, registry_version,
hash_algorithm, profile, canonical, lineage]` (`P1`, "The header").

Nothing is carried key by key. `P9`, "Every key of the first generation, in exactly one list", puts each key in one of two
lists. A structure key serves the framing, chaining, storage or protection of the source and is not carried: the magic
identifier, `"v"`, `"cat"` with the codec keys, `"dct"`, `"id"`, and for a frame `"id"`, `"prev"`, `"x"` and `"sig"`. A
content key is mapped to statements about its header or frame: `"prof"`, `"layout"`, `"meta"`, `"pub"`, `"to"` and the
extension keys.

A user decides, for each key the application relies on, whether it is structure or content. Content is read again from
the statements of the converted file.

### The profile

Generation 1 names a profile by a text. Generation 2 has a table of six values (`P2`, "Identity, labelling, geometry,
severity and profile enumerations"): `dist`, `evidence`, `stream`, `bundle`, `files`, `container`. A segment declares one
profile or none. A reader that meets a value outside its table reads the segment in full and reports `profile_not_checked`
(`P7`, "The profile mechanism").

Conversion writes `profile` `absent` and states the source text as content (`P9`, "The profile declaration"). A text such as
`ai-package` has no value of the table and stays content. A writer of generation 2 declares the one profile whose checks it
wants run, or none. An application that used a profile text to select its own contract carries that contract as data: a
manifest in RDF and the application's own acceptance rule.

### The transform chain and the catalogue

Generation 1 stores a payload through a chain of transforms named in a catalogue: `identity`, `gzip`, `zstd`,
`zstd-rsyncable`, `cose-encrypt`. Generation 2 has no chain and no catalogue. The `storage` of the envelope is
`stored_plain`, or `stored_compressed` with the one codec `zstd` (`P2`, "Items, options, storage, codecs and algorithms";
`P1`, "The zstd constraint"). Encryption is sealing (below).

A writer stores payloads again. No identifier depends on the stored form, so a change of storage changes no identifier.

### Signatures

Generation 1 signs with COSE structures over the identifier of a frame. Generation 2 signs with the signature record of `P6`
over the one subject of `P6`, with the algorithms `ed25519` and `mldsa65_ed25519`. A key identifier is a digest of the
algorithm and the public key (`P6`, "The key identifier").

Conversion counts the signed frames of the source and carries no signature, no key and no statement about one (`P9`,
"Signatures"). A holder who needs the source signatures keeps the source file, which the proof names by its digest. The
signer signs the content again in generation 2.

### Encryption

Generation 1 has COSE Encrypt0 helpers, recipients in `to` and the transform `cose-encrypt`. Generation 2 has the `keys`
and `sealed` frames of registry version 2 (`P8`): one cipher, one key-wrap suite, and recipients wrapped for each key entry
(`P8`, "The key table: the `keys` frame" and "The key wrap").

Conversion takes no key. A source frame under an `encrypt` stage stops the conversion with `source_not_decodable`, and
the result never carries the encryption of the source (`P9`, "What cannot be converted, and the loud failure"). A writer of
generation 2 seals the content again. A holder who needs the encrypted source keeps it.

An application that shipped keys by the means of generation 1 ships them as content. Generation 2 has no counterpart of
that means in the core.

### The unreadable frame

Generation 1 turns a frame it cannot read into an opaque node. In generation 2 the frame is `opaque` with one opaque reason,
and its fold state and its findings stand in the result (`P7`, "How each state value is decided"). A user reads the typed
fields and matches no word or phrase.

Conversion carries no opaque node of the source. A frame that the generation-1 reader cannot open stops the
conversion with `source_not_decodable`. An `opaque` frame of the source stops it with `unrepresentable_frame` (`P9`,
"Every key of the first generation, in exactly one list").

### A payload that is a file

Generation 1 allows a payload that is a complete file. Generation 2 has the nested file: a blob that the class of the
profile `bundle` declares to be a file, read by its nested read (`PF-bundle`, "How a blob is declared to be a nested
file"). A user declares `bundle`, or reads the blob as a file by the application's own rule.

### Metadata

Generation 1 has `meta` on the header and on frames. Generation 2 holds metadata as RDF content (`P3`, "Metadata is RDF").
Conversion states it under the conversion vocabulary (`P9`, "The statements about a segment"). An application that reads the
statements chooses a vocabulary of its own for new files.

## 5. Commands and interfaces

### `fold`

`fold` writes N-Quads. Generation 2 has the projection `rdf(F, v)` of a fold `F` in the variant `v`, `visible` or `whole`,
with the set `G0` of the empty named graphs beside the quads (`P3`, "The RDF projection of a fold").

A tool names the variant and gives `G0` beside the quads. A plain N-Quads stream cannot write `G0`, so a tool that writes N-Quads
gives `G0` beside the stream. A tool does not report a projection as preserving bindings, the
suppression record, annotation counts or blob declarations (`P3`, "What each construct gives and what it loses"). Export
formats remain a matter of the tool.

### `from-nq`

Generation 1 offers `from-nq` as the inverse of `fold`. Its counterpart is import (`P3`, "Import and the round trip"). It is not
an
inverse: the import of a projection holds quads and empty-graph lines only, and its content digest differs from that of the
fold unless the fold held nothing else. A user who needs the original fold keeps the original file.

### `verify --key`

Generation 2 gives a verifier two typed inputs from the caller, `trusted_keys` and `held_keys` (`P6`, "The typed inputs of a
verifier"). No file names a key as trusted. A user who passed a key to a command passes it as a held key, and adds its
identifier to the trusted keys to rely on it. Removing a key from `trusted_keys` is the whole of expiry and revocation.

### `extract-key`

Generation 1 embeds an OpenPGP transport key. Generation 2 has no counterpart in the core. Recipients are entries of the
key table (`P8`, "The key table: the `keys` frame"). An application that ships a key ships it as content.

### `prove` and `verify-proof`

Generation 1 prints proofs as JSON. Generation 2 has the typed proof records `inclusion_proof`, `consistency_proof`,
`range_proof` and `file_proof`, each in the deterministic encoding of a record (`P5`, "Proofs"). A tool can print them in any
form. What is exchanged and verified is the record.

A failed proof is not by itself evidence of two histories. Only a diverged consistency proof between two roots that one
trusted key signed shows that the key signed two histories. The other failures show only that the proof given does not show
the extension (`P5`, "What a proof proves, and what it does not").

### `heads`, `segments`, `missing --from-head`

In generation 2 the counterparts are:

- the head of a segment (`P1`, "The predecessor and transparent frames") and the `header_identifier`, `committed_frames` and
  `frames` of each `segment_result` (`P7`, "The typed result of reading");
- the `offsets` item and the locator (`P1`, "The `offsets` item" and "The locator");
- a consistency proof against a root that the caller recorded (`P5`, "Consistency").

A head does not cover signatures or storage: a signature added in place, or a storage rewrite, changes no identifier. A
synchronization by heads alone misses both. The section of `GUIDE-applications.md` on replication gives the identities to
compare.

### `resume --after <frame-id>`

A frame identifier does not name a place. A segment that occurs twice in a file holds the same frame identifiers twice (`P1`,
"Growing a file", CONCATENATION; `P3`, "What the order of frames and the order of segments decide"). A cursor names the snapshot
of the source, the segment by its ordinal in the file and the frame by
its ordinary position (`P7`, "The typed result of reading"; `P1`, "The predecessor and transparent frames").

### `extract <file> <digest>`

Generation 2 has a whole read and its blob set (`P4`, "The blob set"). A read by range reaches a blob only where one `blob`
frame or member holds the blob whole; it reaches no blob by the digest of its recipe (`P7`, "Reading by range"). A user who
needs a blob by name from a remote file reads the file whole, or keeps a catalogue made from a verified full read; the
section of `GUIDE-applications.md` on reading by range from a remote object describes both.

`extract --include-suppressed` has no counterpart switch. A hidden declaration stays in the blob set, and a suppression entry
of kind `blob` hides it from the visible dataset only (`P4`, "The blob set").

### `cat`

Concatenation is a defined act (`P1`, "Growing a file"). The fold of the joined file is the fold of all its segments, and it
is not in general the union of the two folds. A blank node is scoped to its segment, so the same label in two segments is
two nodes. A user names things that two files share by IRI.

### `compact --streamable --seal-original --timestamp`

Generation 2 has one function, `compact(S, profile)`, which writes the canonical file of a fold (`P7`, "Compaction"). It has
no other parameter. No option seals the source or stamps a time. A canonical file holds no `keys` frame and no `sealed`
frame.

Compaction keeps hidden content, writes opened sealed content in the clear, and writes a commit with an empty signature list.
The file that results holds a new commit: a binding whose referent is a state of the source names no commit of it. The
signatures of the source do not carry over.

### `pack`, `unpack`, `diff`, `ls`

The profile `files` defines the tree: entries, paths as lists of components, kinds, metadata, the order of every listing and
the checks (`PF-files`). Packing, unpacking and comparing are operations of a tool over it. What extraction does with a name
is the extracting system's matter (section 7).

### The streaming reader

The counterpart is the access `forward` (`P7`, "Reading as a stream"). Its result is not that of a whole read: it runs no
labelling, so the content digest is `absent`; it resolves nothing and repairs no parity; and every profile but `stream` gets
`profile_not_checked` (`P7`, "Access: the steps by access").

An interface that promised results equal to a full read does not keep the name "streaming reader" for the access `forward`.
It offers a whole read at the end of the stream. The section of `GUIDE-applications.md` on streaming and interrupted writes gives
the
pattern.

### `info`

The per-segment ledger is the `read_result` and its `segment_result` records (`P7`, "The typed result of reading").

## 6. Examples of agent memory

The five patterns below are those of the memory examples of generation 1. They describe what a program that moves to
generation 2 changes. The helpers of generation 1 are correct for generation 1.

### Superseding an assertion

Generation 1 adds the `wasDerivedFrom` annotation of the successor, then suppresses the `term` of the old assertion.

In generation 2 a `term` entry hides every line in which the term occurs, at any depth, and its bindings (`P3`, "The hiding
rule"). The successor's annotation has the old reifier as its object, so it is hidden too, and the chain of revisions is cut
in the visible dataset.

A user writes a `reifier` entry for the old assertion instead: it hides the `reifier_binding` and `annotation` lines whose
reifier is the old one and leaves the successor's link visible. Or the user keeps the revision as an event and hides nothing
(the section of `GUIDE-applications.md` on agent memory and revision of beliefs).

### Finding the most recent claim

Generation 1 reverses the claims in the order delivered. In generation 2 a whole read delivers every listing in the order of
the canonical file, which sorts rows by their keys (`P3`, "The fold", THE ORDER OF DELIVERY; "The canonical rows"). The order
carries no time. A read with the access `forward` delivers in arrival order.

A user queries the claimed-time and cause annotations, with a stated tie-break.

### A reason for a suppression

Generation 1 passes a reason to the suppression helper. In generation 2 an entry has a kind and its terms, and no reason,
actor or time (`P3`, "The wire record"). A user writes them as annotations of the signed revision occurrence.

### The profile spelling `ai-package`

See "The profile" in section 4. The header holds no text. A user declares one of the six profiles when its checks are
wanted, or none, and carries the application contract as data: a manifest in RDF and an acceptance contract of the
application.

### A large tool payload replaced by its digest

Generation 1 replaces the payload by its digest in a literal. In generation 2 a literal is a literal: no reference is
recognised by a spelling, nothing enters the blob set and no byte is recovered (`P3`, "Typed references: the `bindings`
frame"). A user who needs the run to be reproducible stores the blob and binds the term to it. Otherwise the user declares an
`external` blob and binds to it, so that the result says `external` (`P4`, "The blob set").

## 7. Paths of a C interface

An interface that takes a path as a NUL-terminated UTF-8 string, such as the path helpers of the C interface of generation 1,
cannot hold every component of `PF-files`. A component is any byte string that is not empty, `.` or `..`. It can hold a
zero byte, a `/` or bytes that are not UTF-8 (`PF-files`, "Paths").

A binding of generation 2 to such an interface states one of two behaviours for a component outside the domain of the
interface, and states the same one everywhere:

- it refuses the entry and reports it;
- it maps the name by a stated, reversible encoding and reports that it did.

It does not truncate a name at a zero byte, replace bytes or drop the entry silently. It delivers the components themselves
through an interface that carries a length, so that every name the archive holds can be listed whatever the host can create.

## 8. Adapters

The Markdown round trip, the inspection dump directory, the tar interface, the relational and columnar exports and the RDF text
syntaxes (N-Triples, Turtle, TriG, RDF/XML) are adapters over a fold. Generation 2 can carry what
each needs. None of them follows from a reader of the core.

Each adapter has a schema, mapping laws and safety behaviour of its own: the refusal of dangerous paths on extraction, the
escaping of a syntax. Each is specified again for generation 2, with fixtures of its own. An adapter keeps its version apart
from the registry version of the format.

An adapter that writes RDF text applies the condition on RDF output to every term it writes, including the name of an empty
graph. An adapter that writes only quads loses the empty named graphs (`P3`, "The RDF projection of a fold").

## 9. Versions of tools and schemas

A tool of generation 2 versions its command line and its proof files apart from the registry version. It states the
generation and the schema version of what it reads and writes, and it refuses a proof or a key file of the other generation by
name. The capabilities of a generation-1 tool move to generation 2 by the plan of section 10, one capability at a time, and
not by reusing the name of a generation-1 command for a different result.

## 10. The order of a migration

1. List the features in use and place each by sections 3 to 8: carried by the format, carried as content by the
   application, or a matter of a tool or an adapter.
2. Keep every source file of generation 1. The proof of a conversion names the source by its digest, and the signatures and
   the encryption of the source are not carried.
3. Where files are only read, use a reader that claims `first_and_second`. Do not concatenate across generations
   (section 2).
4. Convert the files whose content generation 2 has to read (`P9`, "The conversion function"). The result is the canonical
   file of the mapped fold, unsigned, with `profile` `absent`.
5. Check each conversion with the proof (`P9`, "What the holder of a proof and both files can check"): hash the source, count
   its segments and signed frames, read the result and compare the content digest, and recompute the content digest from the
   source fold.
6. Read what the proof does not show (`P9`, "What the proof proves, and what it does not"). It does not show that the mapping is
   the right one, that the generation-1 reader read the source correctly, anything outside the fold of the source (the
   identifiers, links, chains, catalogues and signatures of the source), that a signature of the source verified, the
   authenticity of the result, the order of the source frames, or the bytes of an `external` blob.
7. Handle a stop as a stop. A conversion that stops returns no result: its reason, one of seven, and its offset name what to fix
   or what capacity to supply (`P9`, "The reasons and their offsets").
8. Sign the result. Seal again what was sealed. Declare the profile whose checks the application wants.
9. Rework each interface of sections 5 and 6, one capability at a time, and version each tool (section 9).
10. Keep a source file until the converted file has been read and judged by the application's own acceptance rule.
