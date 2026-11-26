<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# GIP: Certified, range-queryable snapshots of a graph state

**Decision status:** `proposed`

**Change class:** core. The proposal adds reader requirements, a new conformance claim and a new typed result
([governance §2](../GTS-GOVERNANCE.md#2-gts-improvement-proposals)). It changes no byte of the wire format of
[P1](../v2/P1.md), no registry table of [P2](../v2/P2.md) and no existing outcome.

**Vectors:** [`vectors/v2/proposals/queryable-snapshots/`](../../vectors/v2/proposals/queryable-snapshots/INDEX.md),
written by [`python/scripts/gen_snapshot_vectors.py`](../../python/scripts/gen_snapshot_vectors.py).

## 1. Summary

A SNAPSHOT is an immutable byte string that holds the RDF projection of one graph state, sorted and cut into
content-addressed blocks under a directory of fences. A reader that holds the identity of a snapshot can answer a quad
pattern by fetching a few byte ranges of it from any holder, verify every block it uses, and state the strongest result the
evidence supports: an exact set of rows, a verified absence, or an incomplete result that names each block it could not
obtain.

This proposal specifies the first shared case: one public state, one order of the quad components (`spog`), delivery as a
separate object, and a caller that pins the snapshot identity. It defines the object layout, the term and key order, the
deterministic construction, certification against the source, the deciding-block rule, the query procedure and its typed
result. Section 11 lists the requirements that later proposals add.

## 2. Affected sections

The proposal adds a normative part. It cites the following sections and changes none of them.

| part | section | use |
|---|---|---|
| P1 | 1, "The typed model and its one serialization" | deterministic encoding `E(x)`, records, variants, `option` |
| P1 | 2.6, "The locator" | precedent for the fixed-length trailer; the locator is unchanged |
| P1 | 4.1, "The one construction and the hash context table" | `H_ctx(unit, bytes)`, the unit digest, names every block |
| P1 | 7, "Identification outside the file" | no GTS identifier is an HTTP validator (P1-75, P1-76) |
| P3 | 7.2, "Graph states" | the state name, its resolution and claims ST1 to ST3 |
| P3 | 8.1, "Definition" (content digest) | the dataset digest |
| P3 | 9.4, "The visible dataset and visibility" | the variant `visible` |
| P3 | 10a, "The RDF projection of a fold" | the content of a snapshot |
| P3 | 11.1, "The canonical dictionary" | the order of terms and the label `E(position)` of a blank node |
| P5 | 4.3, "Checking an index" | unchanged: an `index` frame still answers no absence |
| P7 | 3.9, "Reading by range" | unchanged: its `answer` keeps its meaning beside the new result |
| P8 | 10.2 and 11 | unchanged: a public snapshot discloses only a public state |

## 3. Motivation and non-goals

A consumer often needs one small question answered over a large published state: the statements about one subject, or
proof that there are none. Today it can read the file whole, or read one segment by range under
[P7 3.9](../v2/P7.md#39-reading-by-range). The answer of a read by range has no field for an absence, a count or a digest of
the file, and an `index` frame answers no absence either ([P5 4.3](../v2/P5.md)). An absence is the decisive case: a reader
may report one only when its evidence covers every place a match could stand, and an unreadable or withheld block must give
a different outcome.

Non-goals of this proposal:

- query parsing, planning and joins, which remain engine choices;
- carriage of a snapshot inside a GTS file, and any registry assignment;
- recipient audiences and sealed derived data;
- a portable attestation format, discovery from a GTS file, and freshness;
- term dictionaries, compression and orders other than `spog`.

## 4. The normative change

The requirements below take the label `QS-n`. On adoption they move into the new part with its own requirement labels.

### 4.1 Terms of this proposal

- SNAPSHOT OBJECT: the byte string of section 4.4. Offsets are counted from its first byte.
- BLOCK: one item of a snapshot object, the deterministic encoding `E(b)` of a `block` value (4.3).
- IDENTITY of a block: its unit digest `H_ctx(unit, bytes)` ([P1 4.1](../v2/P1.md#41-the-one-construction-and-the-hash-context-table)),
  32 bytes for `blake3_256`. A block is therefore also a blob that a GTS file can declare by the same name.
- SNAPSHOT IDENTITY: the identity of the manifest block.
- SOURCE STATE: the state that a commit names ([P3 7.2](../v2/P3.md#72-graph-states)). Its NAME is the effective
  identifier of that commit, the digest that a `state` referent holds.
- PROJECTED DATASET: `(Q, G0) = rdf(S, v)` of [P3 10a.1](../v2/P3.md#10a1-the-function), for the source state `S` and the
  variant `v`.
- DATASET DIGEST: the content digest ([P3 8.1](../v2/P3.md#81-definition)) of `IMPORT(rdf(S, v))`
  ([P3 10a.5](../v2/P3.md#10a5-import-and-the-round-trip)).

Three identities stay distinct. The source-state name identifies what the snapshot was built from. The dataset digest
identifies the projected dataset, whatever the physical layout. The snapshot identity identifies one physical
representation: two snapshots of one projected dataset with different layout parameters have one dataset digest and two
snapshot identities.

### 4.2 Snapshot terms and the key order

```text
snapshot_term = [1, iri: text]
              / [2, lexical: text, form: enum(literal_form), datatype_or_language: text]
              / [3, label: bytes]
              / [4, subject: snapshot_term, predicate: snapshot_term, object: snapshot_term]
graph         = option(snapshot_term)          ; [1] the default graph, [2, t] a named graph
quad          = [subject: snapshot_term, predicate: snapshot_term, object: snapshot_term, graph: graph]
```

A term of the projected dataset is written as follows. An `iri` is tag `1` with its text. A typed literal is tag `2` with
the form `typed` (`1` of `literal_form`) and the text of its datatype IRI. A language literal is tag `2` with the form
`language`, `language_ltr` or `language_rtl` by its direction and its language text. A triple is tag `4` with its three
components. Every text is carried byte for byte ([P3-94](../v2/P3.md#10a5-import-and-the-round-trip)). A blank node is
tag `3` with the label `E(position)`, where `position` is the final position of the node in the labelling of
`IMPORT(rdf(S, v))` ([P3 11.1](../v2/P3.md#111-the-canonical-dictionary), the label of a blank node). The same projected
dataset therefore gives every blank node the same label in every snapshot, and two occurrences of one node carry one label.

THE ORDER OF TERMS. Terms are ordered by tag first, `1` before `2` before `3` before `4`, and within a tag by its key,
component by component, each text and each label compared bytewise as [P1](../v2/P1.md) defines it:

- tag `1`: the text;
- tag `2`: `(lexical, form, datatype_or_language)`, the form by its code;
- tag `3`: the label;
- tag `4`: the depth, then the subject, the predicate and the object under this order. The depth of a triple whose
  components hold no triple is `1`, and otherwise one more than the deepest component.

This is the order of P3 11.1 with every literal taken as the kind `literal`, so the storage form of a literal never moves a
term. A graph orders the default graph before every named graph and named graphs by their terms.

THE KEY of a quad in the order `spog` is `(subject, predicate, object, graph)`, compared component by component. Keys are
compared as values and never as encodings. A PREFIX of length `k` of a key is its first `k` components.

**QS-1.** A writer and a reader MUST order terms, graphs and keys as 4.2 states and MUST NOT compare their encodings.

### 4.3 Blocks

```text
block      = manifest / directory / page
manifest   = [1, snapshot_version: uint, source: source, projection: projection, audience: audience,
              dataset: dataset, empty_graphs: list(snapshot_term), layout: layout, orders: list(order_root)]
source     = [hash_algorithm: enum(hash_algorithm), state: digest]
projection = [law: uint, variant: uint]         ; law 1: P3 10a; variant 1 visible, 2 whole
audience   = [1]                                ; public
dataset    = [quad_count: uint, dataset_digest: digest]
layout     = [page_rows: uint, fanout: uint]
order_root = [order: uint, height: uint, root: located]   ; order 1: spog
located    = [identity: digest, offset: uint, length: uint]
directory  = [2, level: uint, entries: list(entry)]
entry      = [lower: option(quad), child: located, rows: uint]
page       = [3, rows: list(quad)]
```

`snapshot_version` is `1`. `state` is the name of the source state, in the hash algorithm that `hash_algorithm` names.
`empty_graphs` is `G0`, ascending. `quad_count` is the number of quads of `Q`. `orders` holds one `order_root`, of order
`1` (`spog`). A page is at level `0`; a directory of level `n` has children at level `n - 1`; the root is at level
`height`, at least `1`.

These enumerations belong to the snapshot object. They are not registry tables of P2, and none of their values is a frame
type, an item kind or a finding.

THE INTERVAL OF A NODE. The root covers every key. A directory whose interval is `[lo, hi)` gives its child `i` the interval
`[lower_i, lower_(i+1))`, where `lower_i` is the `lower` of entry `i`, and the last child the upper bound `hi`. An absent
`lower` is the lower end of the key space and an absent upper bound its upper end. The `lower` of entry `0` equals `lo`, so
it is absent exactly when `lo` is. The intervals of the children of a node therefore partition its interval, and the
intervals of the pages partition the key space. A FENCE is the `lower` of an entry: it claims that no quad of `Q` with a key
in the interval of a child stands anywhere but in that child.

**QS-2.** A writer MUST write each block as the deterministic encoding of its value under P1 section 1.

### 4.4 The snapshot object and its trailer

The object is its blocks, each once, followed by a TRAILER of 33 bytes:

```text
trailer = [tag: text, manifest_offset: uint, manifest_length: uint]   ; tag "gts2 snapshot"
```

The two `uint` fields are written with the eight-byte argument (head `1b`), as the locator of P1 2.6 writes its distance,
so the trailer begins `83 6d` followed by the 13 bytes of the tag, and its length is fixed. The trailer is outside every
identity. A reader reads it with one request for the last 33 bytes of the object.

**QS-3.** A writer MUST write the trailer as 4.4 states, with `manifest_offset` and `manifest_length` the offset and length of
the manifest block, and with the manifest ending at the first byte of the trailer.

### 4.5 The construction

The construction is a function of the source state `S`, the variant `v` and the layout `[page_rows, fanout]`, with
`page_rows >= 1` and `fanout >= 2`. It writes one object and no other.

1. PROJECT. Compute `(Q, G0) = rdf(S, v)` and the dataset digest. When the labelling of `IMPORT(rdf(S, v))` is refused
   ([P3 6.5](../v2/P3.md#65-refusals-and-the-readers-limit)) the dataset digest is absent and the construction writes no
   snapshot.
2. SORT. Sort the quads of `Q` by key. `Q` is a set, so the keys are distinct.
3. PAGES. Cut the sorted quads into consecutive pages of `page_rows` quads, the last page holding the rest. When `Q` is
   empty the snapshot has one page with no quad. The `lower` of the first page is absent; that of every other page is the
   key of its first quad.
4. DIRECTORIES. Level `1` groups the pages, in order, into consecutive runs of `fanout`, the last run holding the rest, and
   writes one directory for each run. Each level above groups the directories of the level below the same way, until a level
   holds one directory, the root. The root exists even when there is one page. The `lower` of an entry is the `lower` of its
   child; the `lower` of a directory is that of its first entry; `rows` is the number of quads in the child's subtree.
5. LAYOUT. Write the pages in key order, then the directories of level `1`, then each higher level, each level in key
   order, then the manifest, then the trailer. Every `located` names the offset and length of its block in this layout.

**QS-4.** A writer MUST write a snapshot by the construction of 4.5, and MUST NOT write one whose state is not a public state
(4.6).

### 4.6 Binding to the source, the projection and the audience

The manifest binds the snapshot to one source state, one projection and one audience.

- SOURCE. `state` is a state name. By claims ST1 to ST3 of P3 7.2 a name names one value to every reader to which it names a
  state, so a snapshot keeps answering for its state after any append. A state name does not survive compaction
  ([P7 4.5](../v2/P7.md#45-compaction)), which writes new commit identifiers; the dataset digest does.
- PROJECTION. Law `1` is `rdf(S, v)` of P3 10a with its losses ([P3 10a.2](../v2/P3.md#10a2-what-each-construct-gives-and-what-it-loses)):
  a snapshot holds no binding, VALUES pair, suppression entry, blob declaration or annotation count, and a quoted triple is
  not asserted. Every exact result is a statement about `rdf(S, v)` of the named variant and of nothing else.
- AUDIENCE. The audience `public` admits a state that a commit names to a reader with an empty sealing input. A sealed frame
  that such a reader does not open makes every later commit of its run unavailable (P3 7.2, clause (3)), so a public state
  holds no content of a sealed frame, and its snapshot discloses nothing that P8 seals.

**QS-5.** A writer MUST give `source`, `projection`, `audience` and `dataset` the values of the state, the variant and the
projected dataset from which the construction wrote the blocks.

### 4.7 Certification

A snapshot object is CERTIFIED for a source state, a variant, the audience `public` and a layout when the construction of 4.5
applied to them writes an object with the same snapshot identity. Under the collision resistance of `blake3_256` an equal
snapshot identity means equal blocks: the manifest names the root by identity, offset and length, and each directory names
its children the same way.

A CERTIFIER reads a file that holds the source state with the access `whole` and an empty sealing input, resolves the state
name as a `state` referent ([P3-65](../v2/P3.md#72-graph-states)), runs the construction and compares the snapshot identity.
Certification therefore checks every fence, every count and every row against an independently rebuilt projection, and a
snapshot whose blocks are authentic but whose fence, row or count is false fails it.

A reader relies on derivation in one of two ways:

- PINNED: the caller supplies the snapshot identity and accepts it as certified for the binding in its manifest. How the
  caller came to accept it (a signed release, its own earlier certification) is the caller's trust decision; this proposal
  defines no attestation record.
- SELF-CERTIFIED: the reader certifies the snapshot itself before it queries it.

The query procedure is the same in both. Integrity comes from the identity chain from the pinned manifest. Exactness needs
derivation, which a pin or a certification provides, and coverage, which the deciding-block rule provides.

**QS-6.** A certifier MUST report a snapshot as certified only when the construction applied to the named state, variant,
audience and layout writes the same snapshot identity.

### 4.8 The query and the deciding-block rule

```text
snapshot_query = [order: uint, pattern: list(slot)]       ; four slots, in the order of the components
slot           = [1] / [2, value]                         ; a variable / a bound term (a graph for the fourth slot)
```

A pattern is SERVED by an order when its bound slots are a prefix of the components in that order. The order `spog` serves
the patterns that bind the subject, the subject and predicate, the subject, predicate and object, or all four, and the
pattern of no bound slot. The QUERY INTERVAL of a served pattern whose first `k` slots are bound to `b` is the set of keys
whose prefix of length `k` equals `b`; the pattern of no bound slot has the whole key space.

THE DECIDING-BLOCK RULE. A node whose interval is `[lo, hi)` DECIDES a pattern with bound prefix `b` exactly when

- `lo` is absent or the prefix of length `k` of `lo` is at most `b`, and
- `hi` is absent or the prefix of length `k` of `hi` is at least `b`.

The DECIDING BLOCKS of a query are the manifest, the root and every child of a deciding directory that itself decides. The
rule is conservative: it includes the left neighbour of a page whose first key has the prefix `b`, because that neighbour
can hold keys with the prefix `b` below the fence. A run of equal prefixes that continues across a page boundary, or across
a directory boundary, is therefore covered whole. Every deciding block is necessary: this proposal defines no other block
set that covers the interval of a deciding block, so a reader that lacks one cannot replace it.

A SHARD or PARTITION is a child subtree of a directory. Its interval comes from the fences of the certifying directory
that names it, exactly as the interval of any other child does, so a run of equal prefixes that crosses a shard boundary is
covered by the same rule: the neighbouring shard decides, and its absence makes the result `incomplete`.

THE EVIDENCE of a query is the list of the identities of the deciding blocks that the reader obtained and verified, in the
order of the procedure (4.9). Evidence is defined by block identity, whatever the transport.

**QS-7.** A reader MUST select the deciding blocks by the rule of 4.8, MUST obtain and verify every deciding block before it
reports an exact result or an absence, and MUST NOT treat a block outside the rule as evidence.

### 4.9 The procedure

The inputs of a query are the snapshot identity (the pin), the query, the access to the bytes of the object and the capacity
`[bytes_per_block, blocks_per_query]`. The steps run in this order, and the first verification failure ends the procedure
with the outcome `failed`.

1. TRAILER. Read the last 33 bytes. When the reader cannot obtain them, the outcome is `incomplete` with no row, the pin
   as the one missing block with the reason `withheld`, and no evidence. When they are not a trailer of 4.4, or the manifest range does not end at the first byte
   of the trailer, the outcome is `failed` with the reason `trailer` and no block.
2. MANIFEST. Obtain the manifest range. When the reader cannot obtain it, the outcome is `incomplete` with no row, the pin
   as the one missing block with the reason `withheld`, and no evidence. Verify its `identity` (step 4), then check only
   that it is an array `[1, snapshot_version: uint, ...]`; a `snapshot_version` other than `1` gives `unsupported` with the
   reason `version`, whatever the rest of its shape. Apply the full `structure` check to a manifest of version `1` only.
   Then a law, variant or audience the reader does not implement, or a query of an order the manifest does not hold gives `unsupported` with the
   reason `version`, `projection`, `audience` or `order`; a pattern that no order of the manifest serves gives `unsupported`
   with the reason `pattern`.
3. DESCENT. Visit the root, then each level below it. At each level, visit the deciding children of every verified directory
   of the level above, in key order. A block that the reader cannot obtain is MISSING: its identity is recorded with the
   reason `withheld`, and no block below it is visited. A block whose `length` exceeds `bytes_per_block`, or that would make
   the number of blocks obtained exceed `blocks_per_query`, is missing with the reason `capacity` and is not requested.
4. VERIFICATION of each obtained block, in this order:
   - `identity`: the bytes have the `length` of their `located` and their unit digest is its `identity`; for the manifest,
     the pin;
   - `structure`: the bytes are one deterministic encoding (P1 1.4) of a block of the kind and level that the step expects;
   - `fence`: the `lower` of entry `0` equals the lower bound of the node; the entries have strictly ascending `lower`, each
     inside the interval of the node; the quads of a page are strictly ascending and inside the interval of the page; a
     page whose lower bound is present begins with the quad of that key; a page holds a quad unless it is the only page;
   - `count`: the number of quads of a page, and the sum of the `rows` of a directory, equal the `rows` of the entry that
     names it; for the root, the `quad_count` of the manifest.
5. RESULT. When no block is missing and some quad of a deciding page lies in the query interval, the outcome is `exact`
   with those quads, ascending by key. When no block is missing and none does, the outcome is `absent`. When a block is
   missing, the outcome is `incomplete` with the quads of the verified deciding pages that lie in the query interval and the
   list of missing blocks.

An `incomplete` result can hold rows. They are matches the reader verified, and they are not every match: a consumer does
not count them, negate them or treat them as exact. A consumer can rely on a count or a negation only from an `exact` or
`absent` result. Rows are a set: a quad appears once, as in `Q`.

**QS-8.** A reader MUST run the steps of 4.9 in their order and MUST report `failed` at the first verification failure.

**QS-9.** A reader MUST report `incomplete` whenever a deciding block is missing, MUST name every missing block by its
identity, and MUST NOT report `exact` or `absent` for that query, whatever rows the other blocks give.

### 4.10 The typed result

```text
snapshot_result = [1, rows: list(quad), evidence: list(digest)]                         ; exact
                / [2, evidence: list(digest)]                                           ; absent
                / [3, rows: list(quad), missing: list(missing), evidence: list(digest)] ; incomplete
                / [4, reason: uint]                                                     ; unsupported
                / [5, block: option(digest), reason: uint]                              ; failed
missing         = [block: digest, reason: uint]       ; 1 withheld, 2 capacity
```

The reasons of `unsupported` are `version` (`1`), `projection` (`2`), `audience` (`3`), `order` (`4`) and `pattern` (`5`). The
reasons of `failed` are `trailer` (`1`), `identity` (`2`), `structure` (`3`), `fence` (`4`) and `count` (`5`); `block` names
the block whose check failed and is absent for `trailer`. The record is a new result type. It is not the `answer` of a read
by range ([P7 3.9](../v2/P7.md#39-reading-by-range)), whose meaning stays unchanged: an `answer` with no items is still no
absence.

**QS-10.** A reader MUST report the outcome of a query as one `snapshot_result` of 4.10 and MUST NOT report it as an `answer`
of P7 3.9.

### 4.11 A byte-range transcript

The procedure reads byte ranges and names nothing by its transport. Over HTTP each read is one range request (RFC 9110
section 14): the trailer as a suffix range of 33 bytes, and each block as the range `offset` to `offset + length - 1`.
A transcript is checked against the evidence: each served range is the trailer or the `located` range of a block of the
evidence, and each block of the evidence has a served range whose bytes have its identity. As P1-75 and P1-76 require, no
identity of this proposal is presented as an HTTP validator, and no media type or entity tag decides an outcome.

### 4.12 Claims

CLAIM QS-C1 (exactness). For a snapshot certified for `(S, v, public, layout)` and a served pattern, an `exact` result holds
exactly the quads of `rdf(S, v)` that match the pattern, each once, and `absent` means that no quad matches. The intervals of
the pages partition the key space (4.3); the construction places every quad of `Q` in the page whose interval holds its key
(4.5); the deciding pages include every page whose interval meets the query interval (4.8); and the identity chain from the
pinned manifest makes each verified block the certified one.

CLAIM QS-C2 (no exact status from partial evidence). When a deciding block is missing, no reader reports `exact` or `absent`
for the query, whatever rows the other blocks give (QS-9).

## 5. Compatibility impact

- WIRE. None. No item kind, frame type, registry version or hash context is added. A snapshot object is not a GTS file and
  is never read as one.
- CORPUS. No existing expectation changes. The vectors of this proposal stand under `vectors/v2/proposals/`, outside the
  corpus that `SHA256SUMS` lists.
- PACKAGES. No public API changes until an engine implements the proposal.
- PROFILES. None. The feature needs no profile; a profile could not carry the binary layout ([P7 6](../v2/P7.md#6-the-profile-mechanism)).
- OLDER READERS. Unaffected: they meet no new item in any GTS file.

## 6. Conformance vectors

The entry `snapshot/spog-public` holds one canonical source file of nine quads and one empty named graph, the snapshot of the
state its last commit names (`visible`, `public`, `page_rows` `3`, `fanout` `16`), and four cases:

| case | query | result |
|---|---|---|
| `certify` | the construction applied to the source | the snapshot identity, the dataset digest, the state name and every block with its identity, offset and length |
| `hit` | subject `b`, whose four rows continue from the first page into the second | `exact` with four rows; evidence: the manifest, the root and both pages |
| `absent-neighbour` | subject `c`, between `b` and `d` | `absent`; evidence: the manifest, the root and the second page |
| `absent-withheld` | subject `c`, the second page withheld | `incomplete` with no row, naming the identity of the second page |

Each query case records the pin, the withheld blocks, the encoded query, the encoded `snapshot_result`, the rows in plain
form, the evidence by kind and identity, and an HTTP transcript of the ranges in the order of the procedure. The generator
writes the source with the v2 engine of the Python package, reads it back with no finding and compacts it to its canonical
file; it then builds the snapshot by 4.5 and runs the procedure of 4.9. Running it twice gives identical bytes.

The vectors cover no blank node, no triple term, no verification failure, no capacity outcome and no hierarchy of more than
one directory level. Section 11 lists them as required before adoption.

## 7. Affected engines and implementation plan

1. Adopt the format of section 4 and the entry `snapshot/spog-public`.
2. Implement the construction, certification and the query procedure in the Rust and Python engines first, each checked
   against the vectors and against the other.
3. Add the negative vectors of section 11 and the remaining engines.
4. On adoption, move the text into a new normative part and the vectors into its corpus directory (section 9).

## 8. Security and privacy considerations

This change is security-sensitive under [governance §6](../GTS-GOVERNANCE.md#6-parser-and-cryptographic-security-governance):
it adds a CBOR layout that a reader decodes from an untrusted holder.

- THREAT MODEL. The holder is untrusted: it can withhold, truncate, reorder or replace any range. A replaced block fails
  `identity`; a withheld or unobtainable block makes the result `incomplete`; neither yields `exact` or `absent`. The
  writer is trusted only through the pin or a certification: step 4 detects a fence, row or count that contradicts the
  blocks a reader holds, and certification (4.7) detects one that is consistent with them and false against the source.
  How a caller comes to trust a pin is a non-goal of this proposal.
- RESOURCES. A reader reads a fixed trailer, then at most `blocks_per_query` blocks of at most `bytes_per_block` bytes each.
  The deciding pages of a served pattern are contiguous: the pages that hold a match and at most one page before them,
  at most `ceil(m / page_rows) + 2` pages for `m` matches, and exactly one page when nothing matches. The deciding
  directories are the ancestors of those pages.
- REQUIRED OUTCOMES. An unknown version, law, variant, audience or order is `unsupported`; nothing is guessed.
- DIAGNOSTICS. Every failure names its check and, but for the trailer, its block identity.
- DISCLOSURE. A public snapshot holds the projection of a public state and nothing else. Its directory discloses the
  quad count and the keys at the fences, both of which are part of that public projection.
- HOSTILE INPUTS. Before adoption the vectors add a replaced block, a non-deterministic block, a false fence and a false
  count (section 11).

## 9. Migration plan and release milestone

Nothing migrates: the proposal adds a capability and removes none. On adoption the change adds the new normative part, a row
in the table of [GTS-SPEC.md](../GTS-SPEC.md) and its translations under `docs/i18n/`, the standards-review entry in `ietf/`,
the fixture entries of `docs/v2/FIXTURES.md`, the claim in P7 section 5, and the vectors as a part directory of the corpus,
listed in its `SHA256SUMS`. The value of `spec_document_version` in `docs/documentation-roster.json` changes with the
specification text. At this proposal stage none of these files changes. The milestone is the specification revision after
Implementer Draft 1.

## 10. Alternatives considered

- A NEW FRAME TYPE IN A GTS SEGMENT. Every new frame type opens registry version 3, and a reader of version `n` must
  implement every frame type of versions `1` to `n` (P2-8, P2-9), so optional index families cannot be registry-coded. The
  existing locator names only an `index` frame (P1 2.6) and cannot name a snapshot root. A separate object needs none of
  this for the first case, and its blocks are blobs by name, so a later proposal can carry them in a segment.
- REUSING THE `index` FRAME. P2-6 forbids changing the shape of its content record, and its covered fold is one segment's
  own (P5 4.1), not a state over a run of segments.
- A PROFILE. A profile adds no frame and reads inside no `bytes` (P7 6, P2-75, P7-42), so it cannot carry the layout.
- FENCES AS SEPARATE UPPER AND LOWER KEYS PER PAGE. Two keys per page can leave a gap between pages that no block covers.
  The partition of 4.3 leaves none, and one key per entry is smaller.
- PLACEMENT OUTSIDE THE DIRECTORY. Locating blocks through a separate table keeps the directory independent of the
  layout, but a cold reader must then read a table that grows with the dataset. The `located` record keeps each step to one
  range, and the dataset digest still identifies the content across layouts.

## 11. Open requirements

The following remain requirements of the capability and are not settled by this proposal.

- DISCOVERY AND FRESHNESS. Discovery of a snapshot from a GTS file: a later commit can bind a term to the manifest by a
  `bindings` referent of kind `blob`, which keeps the construction acyclic; the term and its vocabulary are open. Resolving
  several snapshots, a mismatched selector, an external checkpoint for freshness, and selection of a snapshot after
  compaction by dataset digest.
- EMBEDDED DELIVERY. Carrying the blocks as blobs of a GTS segment, the logical byte ranges of inline blobs, recipes, packs
  and external objects, and the physical bytes to fetch or decrypt.
- THE NATIVE LAYOUT. A term dictionary with lookup in both directions, index-local identifiers and widths, compression,
  further orders (`posg`, `ospg`, `gspo` and the like) with their iterator guarantees, objects and predicates as bound
  keys, graph enumeration with a variable graph name, and graph inventory blocks for a `G0` larger than a manifest.
- ATTESTATION. A portable attestation binding the certification rules, the source selector, the projection identity, the
  snapshot identity and the scope, and its validation under caller-supplied trust.
- IMMUTABLE SHARD SETS. Identification of a set of snapshots as shards of one dataset, snapshot pinning of the set and
  coverage of its partitions beyond the rule of 4.8.
- TRANSPORT OUTCOMES. Changed validators during a read, truncated ranges, a stale cached block and mixed representations,
  beyond the `identity` failure and the `withheld` record that this proposal defines.
- RECIPIENT AUDIENCES. Sealed snapshots: bootstrap and range access for recipients, a sealed inner type for derived data,
  one index per key index, and the clear metadata that P8 10.2 and P8-53 now forbid. Each needs an explicit amendment of P8.
- RICHER VIEWS AND METADATA. Annotation counts, typed values, blob and state bindings, suppression history, a dataset
  description, label and full-text lookup, statistics and summaries. An approximate summary may guide planning but may
  exclude a candidate from an exact answer only under certified conservative bounds.
- VECTORS BEFORE ADOPTION. Blank nodes and nested triple terms, two layouts of one dataset, a high-degree key across several
  pages and directories, a false fence, an omitted row and a false count in authentic blocks, a replaced and a
  non-deterministic block, the capacity outcomes, and each `unsupported` reason.

## 12. Review findings this proposal settles

| finding | settlement |
|---|---|
| the locator names only an `index` frame | the snapshot is a separate object with its own trailer; no GTS item changes |
| "index root" collides with the `index` frame and `range_root` | the terms are snapshot, manifest, root directory and page |
| a certified absence needs a path that P5 4.3 lacks | certification (4.7) and the deciding-block rule (4.8); P5 4.3 is unchanged |
| new frames need registry version 3; no experimental range | no registry value is assigned; the enumerations belong to the object |
| registry-coded optional families become mandatory | none is registry-coded; later families come by proposal |
| a profile cannot carry the binary layout | no profile is used |
| clear metadata over sealed content | the audience is public only; a public state holds no sealed content |
| the source selector already exists | the state name of P3 7.2 is reused |
| the projected-dataset digest is undefined | the content digest of `IMPORT(rdf(S, v))` |
| P7 3.9 has no absence | a distinct `snapshot_result`; `answer` is unchanged |
| a derived frame in a canonical segment breaks its claim | the snapshot stands outside the source file |
