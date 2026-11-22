<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Guide: Adoption of generation 2

## 0. Scope

This is an informative document of the specification of format generation `2`. It states no requirement of the format. The
sixteen normative documents govern, and where this document and they differ, they are right.

The document says what adoption of the generation needs beyond the text: the measurements to make, the register of the
evidence, the evidence that programs written apart from one another give, the review of the cryptography, the order of
implementation, and the decisions that the text keeps, each with its reason. It cites the normative documents by part and
section title. `GUIDE-applications.md` holds the contracts between an application and the format, and `GUIDE-migration.md`
the map from the first generation.

"A program" is a reader, a writer, a prover, a verifier or a converter of this generation. "The corpus" is the set of inputs
with their expected typed results that such programs are compared on. "The fixture catalogue" is the list of the inputs that
the corpus holds at least.

## 1. The capacity of a reader

### 1.1 What the minimum capacity is

The minimum values of `P7`, "Capacity", are limits on what a reader agrees to process:

| field of `capacity` | minimum value | what it bounds |
|---|---|---|
| `stored_bytes_per_frame` | `2^26` | the stored payload, the `content` element and the `signatures` element of one frame |
| `decoded_bytes_per_frame` | `2^26` | the decoded length of one frame |
| `items_per_read` | `2^26` | the items of one read |
| `labelled_lines` | `2^26` | the size of one labelled fold |
| `decoded_bytes_per_read` | `2^36` | the decoded bytes of one read |
| `rebuilt_bytes_per_read` | `2^37` | the rebuilt bytes of one read |
| `formed_bytes_per_read` | `2^37` | the bytes that the prefixes of the front-coded blocks copy in one read |
| `labelled_bytes_per_read` | `2^37` | the weight of the folds that one read labels |

The minimum values of `labelled_lines`, `rebuilt_bytes_per_read` and `labelled_bytes_per_read` are those of a read with the
access `whole`; the others hold for every access. The fields `access` and `labelling_units` have no minimum value.

These are limits of processing. They are not amounts of memory. A reader can decode a frame to a stream, keep a fold on disk
and label with an external sort: the specification fixes the result and the order of the refusals, not the storage.

A reader states any capacity. A reader whose every field is at least its minimum value claims the minimum capacity. A file
within the minimum values is read by every such reader without a refusal by one of those limits. A reader that states less is
complete in every rule and refuses, with a typed finding, what is beyond its limits; another reader of the same capacity gives
the same result for the same bytes. The claim is not a tier, a mode or a subset: every value, frame type, check and finding
stays mandatory, and so do the three accesses (`P7`, "Conformance").

### 1.2 What bounds the work of a reader

Three facts of `P7`, "Capacity", decide how a small budget is expressed.

- `labelling_units` bounds the walk of the labelling of one fold. A read keeps no total of it: a file of many segments runs
  one walk for each own fold and one for the fold of the file.
- `labelling_units` is never applied to an input for which the predicate `guaranteed` of `L1` holds. The work on such an
  input is bounded by a function of the input, so the limits on the size and the weight of the fold bound it.
- `labelled_lines` (the size of one fold) and `labelled_bytes_per_read` (the weight of all folds of the read) are applied to
  every input, `guaranteed` or not, before the labelling is called.

So a reader bounds its labelling work for all inputs with `labelled_lines` and `labelled_bytes_per_read`, and uses
`labelling_units` for the inputs outside the guaranteed class. Since `labelling_units` has no minimum value, a fold outside
the guaranteed class can be refused by the limit of the walk in a reader that claims the minimum capacity.

A budget in bytes or in items is stated as the `capacity`. A budget in time has no field. A stop by the clock, by the
exhaustion of memory or by a signal of the caller is not a refusal: a read so stopped has no result (`P7`, "Conformance").

## 2. Measurements

No measurement of this section has been made. The specification fixes results, not costs, and a claim that a design serves a
use well is a claim about workloads. Each measurement below is a duty before adoption: what is measured, on what input, and
the decision that it informs.

### 2.1 How a measurement is made and recorded

- Each measurement is made with at least two programs written apart from one another.
- Each input is built once, by the program that builds the corpus, and kept with its parameters.
- Each result is recorded in the evidence register (section 3) with the input, the capacity, the host, the program and its
  version. No result is written into a normative document.
- The measurements `M1` to `M7`, `M9` and `M11` are run on three hosts: a service (many cores, memory in the tens of
  gigabytes), a desktop, and a constrained host (one core, memory below one gigabyte). Each is run with a reader that claims
  the minimum capacity and with a reader at one sixteenth of every minimum value. For each run the record holds the wall
  time, the peak resident memory, the peak temporary storage, and the limit that refuses first, if one does.

### 2.2 The capacity of a reader

`M1`. The byte limits of a frame and of a read.

- Measured: the cost of reading at the per-frame and per-read byte minimums.
- Input: one segment whose frames each have `2^26` decoded bytes, up to a total of `2^36`.
- Informs: whether `2^26` and `2^36` are reachable on the constrained host.

`M2`. The item limit.

- Measured: the cost of reading at the item minimum, with the access `whole` and with the access `forward`.
- Input: `2^26` items of the smallest frame.
- Informs: the minimum value of `items_per_read`.

`M3`. Many labellings in one read.

- Measured: the cost of one labelling for each own fold and one for the file, where the read keeps no total of limited
  units.
- Input: a file of `2^10`, of `2^14` and of `2^16` segments, each with a small own fold.
- Informs: whether a bound on the labelling work of a whole read is needed beside `labelled_bytes_per_read`.

`M4`. Nested reads.

- Measured: the time of the nested reads; the totals that they use against what remains of the capacity; the depth and the
  number of nested files at which the shared allowance is used up.
- Input: files of the profile `bundle` with `1` to `10^3` nested files at the depths `1` to `Dmax`, some of them near `Smax`.
- Informs: the five fields that a nested read shares with its outer read (`PF-bundle`, "What a reader reports about nested
  content"); the size of the bundles that a reader of the minimum capacity verifies in one read.

`M5`. The depth of nested terms.

- Measured: the time and the stack use of decoding, lowering and labelling.
- Input: terms nested to the depths `2^4`, `2^10` and `2^16` (a `triple` term inside a `triple` term).
- Informs: whether depth needs a limit that the capacity does not have.

`M6`. The labelling inside the guaranteed class.

- Measured: the time and the memory of the canonical labelling; the units of the search-free run against the allowance of the
  input.
- Input: folds of `10^3` to `2^26` lines for which `guaranteed` holds: with no blank node, with many twin blank nodes, and
  with an incidence graph that is a forest.
- Informs: the cost of the inputs that no `labelling_units` refuses; whether the minimum values of `labelled_lines` and
  `labelled_bytes_per_read` are kept.

`M7`. The labelling at the edge of the guaranteed class and outside it.

- Measured: the units of the walk against `labelling_units`; the size at which the walk passes one second, one minute and
  one hour; the inputs at which the labelling refuses.
- Input: the last input of the guaranteed class and folds just outside it: one first failure, then growing symmetric
  components.
- Informs: the value of `labelling_units` that a program states for a deployed capacity, and whether a minimum value for it is
  needed.

### 2.3 Reading by range

`M8`. The requests of a query.

- Measured: for one query, cold and then warm (the dictionary kept from an earlier answer): the bytes transferred, the
  requests made, the bytes decoded and the time; once with each range asked alone, once with ranges joined and with
  read-ahead.
- Input: canonical files of `10^4`, `10^6` and `10^7` lines with `1`, `100` and `10^4` named graphs; a query for a graph of
  `10` rows and one for a graph of `10^5` rows; a store that takes `50` milliseconds for a request.
- Informs: whether the access `sparse` serves remote queries as it stands; which plan of requests a program uses. The plan is
  free: the answer and its charges are those of the procedure (`P7`, "Reading by range").

`M9`. The dictionary against the graph asked for.

- Measured: the bytes of the covered `terms` frames against the bytes of the frames of the target graph; the bytes read and
  the time against the bytes of the rows returned.
- Input: the files of `M8`; a canonical segment with `2^4`, `2^10` and `2^14` `terms` frames, queried for a graph of one row; a
  file whose dictionary is large against its rows (long IRIs, few rows for each term).
- Informs: whether the read of every covered `terms` frame makes a read of a small graph useless; whether a writer splits a
  publication into segments by groups of graphs; whether a directory or locator of the dictionary is proposed for a later
  registry version.

`M10`. Packs, and blobs of several units.

- Measured: the bytes fetched and decoded for one unit in a pack, against the length of the unit; the cost of finding a blob
  of several units by a whole read, against finding it with a catalogue made from a whole read.
- Input: canonical files with units of 4 KiB to 64 MiB; blobs of `1`, `10^2` and `10^4` units.
- Informs: the pack sizes of a file written for reads by range; whether a catalogue held by the application is enough, or a
  locator for recipes and values is proposed.

### 2.4 Distribution

`M11`. The canonical file of the intended datasets.

- Measured: for each frame of the list of `P7`, "The canonical frames that can exceed the minimum capacity", its size in the
  canonical file; the largest `decoded_length` of a frame; whether the fold has a distribution file; the time and the peak
  memory of the labelling and of the canonical check.
- Input: the datasets that are to be published: the largest ontology release with every attachment; a large file archive; a
  graph whose dictionary is large against its rows; a dataset with one very large literal.
- Informs: how many intended datasets have no distribution file; whether `dist` is the release path for every package or a
  second path is defined for a fold without a distribution file; whether another ceiling, which is another profile value
  (`PF-dist`, "Constants"), is needed; whether the minimum capacity suits real releases.

### 2.5 Streams

`M12`. The memory of a forward reader.

- Measured: the resident memory of a reader with the access `forward`, and the size of its keyed stores on disk.
- Input: streams of `10^6` and `10^8` items; one long segment against many short ones; values named by reference.
- Informs: whether a constrained device relays or consumes a stream; how often a long stream opens a new segment.

### 2.6 Sealing

`M13`. Keys, recipients and dictionary layouts.

- Measured: the bytes of the `keys` frames (key entries and recipient entries) as a share of the file, for each audience; the
  time to open; the result of a reader without keys for each dictionary layout; the time from opening the file to a state
  that a commit names.
- Input: a review package of a public part, two restricted analyses and a private deliberation, scaled to `10`, `1,000` and
  `100,000` sealed frames; `1` to `10^3` recipients for a key (among them `2`, `8` and `32`); `1` to `10^2` audiences; one key
  for each audience against one key for each sealed frame; the three layouts of a dictionary: a clear dictionary first, a
  private dictionary first, one segment for each audience.
- Informs: the grouping of frames under keys by audience and period; which dictionary layout the examples of sealing use.
- Known without measuring: a recipient entry is 1,243 bytes for each recipient of each key (`P8`, "A recipient entry and the
  wrap's key schedule"). With `n` sealed frames and `r` recipients, one key entry takes `1,243 * r` bytes of recipient
  entries, and a key entry for each sealed frame takes `1,243 * r * n`.

### 2.7 Histories and archives

`M14`. Compaction after each event.

- Measured: the time and the bytes of writing the canonical file of the whole history after each appended event, against
  appending alone.
- Input: a log of a document review over a document of 10 MiB, grown by `1`, `100` and `10,000` events of a few lines each.
- Informs: whether such a workflow compacts after each event, at checkpoints, or never.

`M15`. A changed file and the units of an earlier segment.

- Measured: the bytes appended and the bytes transferred for each byte changed; the share of the units that already stand
  in an earlier segment; how often a writer restarts; the time and the bytes of the compaction that follows.
- Input: a tree of 10,000 files and a regular file of 1 GiB and of 64 GiB, revised by an edit of one byte, of one chunk, of
  1 MiB and of 10 per cent; each revision written once by a writer that continues its segment and once by a restarted writer
  that appends a segment.
- Informs: whether the rule that a recipe finds its units in its own segment is kept as it is for incremental backup
  (section 7.2).
- Known without measuring: the appended segment holds every unit of the new recipe, since an entry whose unit stands in
  another segment has no unit and the declaration is incomplete with `missing_dependency` and `chunk`; the continued segment
  adds the changed units and one recipe (`P4`, "The unit index and the unit used").

`M16`. Extraction in one pass.

- Measured: the time to the first extracted file, the peak memory and the peak temporary disk of an extraction that reads
  the file once.
- Input: the tree of `M15` as a canonical file, and as a file that is not canonical with the entries and the bindings before
  the packs.
- Informs: which layout a producer of archives for extraction during arrival writes; whether `GUIDE-applications.md` names a
  layout.
- Known without measuring: in the canonical file the packs stand before the terms and the bindings, so no path is known
  before the last pack is read (`P7`, "The construction, from a fold to the item sequence").

`M17`. The creation of an archive.

- Measured: the time, the peak memory and the size of the output of creating an archive from a file system.
- Input: the tree of `M15` and a tree of 1,000,000 small files.
- Informs: the cost that the archive use claims; how the constants of the chunker and of the pack serve small files.
- Known without measuring: nothing follows from the rules alone.

### 2.8 The decision on the minimum values

One decision is made on the numbers of `M1` to `M7` before a public `implementation_claim` is fixed: the eight minimum values stay
as they are, or a proposal changes them. Until then the minimum values are those of the text, a reader of a
smaller stated capacity conforms, and a program publishes its `capacity` in its claim.

## 3. The evidence register

The sixteen documents hold rules, definitions, registries and worked examples. They hold no proof, rationale or measurement.
The evidence for their claims is kept beside them, with its version, and each record is traceable.

A record of the register has:

- the claim it supports: the document, the section title and, where one exists, the identifier of the requirement or claim;
- its kind: proof, machine-checked proof, measurement, test of a program, review;
- the commit of the text it was made against;
- where it is: a file under the directory of the evidence, with its digest;
- for a measurement: the input (an identifier of the corpus, or one of `M1` to `M17`), the capacity, the host, the program
  and its version;
- its state: current, or made against an earlier text and not yet repeated.

A change of a document marks as not repeated every record whose claim stands in a changed section.

Adoption needs a current record for each claim that is new to this format:

- the labelling of `L1`: its lemmas, the guarantee and the bound of the search-free run;
- the canonical form and its uniqueness (`P7`, "Definition and the two claims");
- the equality that a conversion proof shows (`P9`, "The conversion proof");
- the bounds of the sealing (`P8`, "Security considerations");
- the recovery procedure (`P5`, "The one recovery procedure").

## 4. Evidence from programs written apart

Adoption rests on programs that agree, not on one program that passes its own tests.

### 4.1 The executable specification

Before adoption there is an executable form of the text, written to be read against the text and not to be fast. It has one
function for each step of the reader procedure (`R1` to `R24` of `P7`, "The reader procedure"), for each step of opening a
sealed frame (`P8`, "The procedure"), for the walk of `L1` with its accounting, and for the construction of the canonical file
(`P7`, "The construction, from a fold to the item sequence"). Each function names the identifiers of the requirements that
it carries out, so the executable form is indexed by requirement.

It is the program that builds the corpus and gives the expected result of every entry.

### 4.2 Two readers and one writer

- Write two readers, each from the text of the sixteen documents alone. Neither author reads the source of the other
  program or of the program that builds the corpus.
- Write one canonical writer (the construction and the compaction of `P7`, "The canonical form") from the text alone, by an
  author who did not write the program that builds the corpus.
- Compare the readers on the same input and report every difference. An input is the bytes, the capacity (all ten fields,
  the access among them), the input `marks`, the held keys and the trusted keys, the sealing input, the registry version of the
  reader, and the profiles with the number of checks of each; with `forward` also the value of `end`, and with `sparse` the query.
  The comparison is of the whole encoded `read_result`, byte for byte, and of the stream points
  and the answers by range where the access gives them. A field is left out of the comparison only where the text says that
  two conforming readers can differ in it, and the report names each such field.
- Compare the writers on the same fold and the same `profile` value: the items of the head claim are equal byte for byte;
  the stored bytes are equal when both hold the reference encoder (`P7`, "The reference encoder and the bytes claim").
- Each reader carries out every part that it claims: signatures, sealing, profiles, proofs, conversion, the canonical check,
  and the accesses `forward` and `sparse` are compared as the access `whole` is. A part that only one program carries out is
  evidence of one program and is reported as such.

What it informs: whether two authors read each rule alike. A difference between two programs is a defect of the text until
one program is shown wrong by a sentence of the text.

### 4.3 What the corpus states

- Every entry states the expected typed result of its input: the encoded `read_result`, or the outcome of the proof, the
  compaction, the conversion or the request to a writer that it tests. An entry that states only decoded content, or only
  that a round trip succeeds, does not test a rule about findings, states or refusals.
- Every entry states its inputs in full: the capacity, the access, the keys, the marks and the profiles with their numbers
  of checks. An expected result without them is the result of an unstated reader.
- For every rule that separates two outcomes, the corpus holds the smallest input on the failing side and the nearest input
  on the other side. The second input shows that the rule does not refuse more than it says.
- The corpus holds inputs that cross parts, because a rule that holds in each part alone can fail where two parts meet. The
  cases to hold at least:
  - a valid signature on a frame that is detached from the frames before it;
  - a canonical file read with fewer profile checks than its profile numbers;
  - a first-generation source that uses one blank-node label in a snapshot and outside it;
  - a frame that is malformed and also above the capacity;
  - a parity resumption after a region that took an ordinary frame;
  - two reads with equal folds of which one holds a row;
  - nested files that share one total;
  - one input read with each of the three accesses.
- An entry whose expected result changes when a rule changes is built again, and the change of its result is recorded with
  the rule.
- The corpus and the requirements are compared (section 6): a requirement with no entry is a gap of the corpus, and an entry
  with no requirement is a gap of the text.

### 4.4 Counterexamples as files

A counterexample that is traced through the text and never read by a program is a claim about the text. Build each one as a
file, read it with both readers, and record the result. The first to build: a file with a parity frame whose `anchor` is not
the identifier before a damaged region, with a signed frame after it. It is read for coverage and authentication, and it is
used as the later file of a file proof.

What it informs: whether the rules for a detached frame (`P1`, "The predecessor and transparent frames"), as two programs
read them, give the result that the text states.

### 4.5 The labelling

The canonical labelling is new. That no wrong line sequence of it is known is not a proof.

- Compare the labelling with an oracle on every small input. For every labelled set up to a stated number of blank nodes and
  lines, the oracle tries every assignment of positions with no pruning and takes the least line sequence. The labelling
  gives the same sequence or the same refusal. State the bound that was reached.
- Test properties on generated inputs above that bound: renaming the blank nodes changes no line sequence; the order of the
  lines of the input changes none; a nested term and its lowered form give the positions that the text states; the canonical
  file of a fold, read and compacted again, gives the same bytes; an input with an automorphism gives one line sequence
  whichever of the symmetric nodes is met first.
- Include the inputs at the edge of each stated behaviour: the last input of the guaranteed class and the first outside it;
  an input at each refusal (size, weight, width, limit, a collision of the digests of nested terms); an input for which the
  walk reaches its limit at the last unit.
- Have the invariants and the bounds of the work of the labelling read by a person who is not its author. That person
  examines the argument for each invariant, each bound and each claim of complexity, not the stability of a node map: a node
  map that changes by an automorphism is correct.

What it informs: whether the content digest, on which every comparison of content rests, is one value for one content.

### 4.6 Framing and durability

- Cut representative files at every byte and read each prefix with `whole` and with `forward`, with the end declared and
  not. The files cover each kind of item, a segment opened by a resync item, a sealed segment, and a file with parity.
- Damage one field at a time in each class of field: the head of an item, an envelope field, the content record, the
  payload, the stored identifier, the signature list, a parity table and a parity shard, an `offsets` item and a locator. For
  each, record the findings, the states and what is left out.
- Vary where parity stands: a full span, a short span, a span that ends at an index, a parity frame that is stale, repeated,
  misplaced or of another segment, and a parity frame whose `anchor` is and is not the identifier before the region.
- Stop a writer at every byte of an append and start it again: with and without reading the file first, after a torn tail,
  after a damaged region, and after a recovery that resumed.

What it informs: whether every input has one result, and whether a file that a writer can produce after a failure is one
that every reader reads alike.

### 4.7 The three accesses

Read each input of section 4.6 with `whole`, `forward` and `sparse`, and compare the three results field by field with the
table of `P7`, "Access: the steps by access". Each access states the facts that the text gives it and no other: a `forward`
read reports no fact that needs a step it does not run, and an answer by range claims no membership in a chain, no
authentication and no absence. A field that is equal under two accesses where the table says it can differ is recorded as
well; it is not an error.

What it informs: whether a consumer can rely on the name of an access for what a result establishes.

### 4.8 The values of the worked examples

Compute again every value that a worked example prints, with a program that shares no code with the program that produced
it: every digest and identifier (BLAKE3 under each context); every byte string of `P1` to `P9` and of the profiles; the
signatures of `P6`; the conversion of `P9` with its proof; the key schedule, the ciphertext and the padding of `P8`; and one
complete encapsulation and decapsulation of the key wrap from a recipient seed. Record for each example which program
computed it again, and list the examples that only one program has produced.

What it informs: whether an author of a program can use an example as a test.

### 4.9 A complete reading of every part

Have every one of the sixteen documents read in full by a person who did not write it, with the other parts at hand. A
reading by search finds the passages that a question names; it does not find a second statement of one rule in another part.
The reading records, for each part, that it was read whole and by whom, and each pair of passages that state one rule in two
ways.

### 4.10 Five complete examples

Before adoption, five examples are built end to end. Each is read by at least two programs written apart, with equal
results. Each has its application acceptance contract (`GUIDE-applications.md`) and the entries of the fixture catalogue that
are named for it.

A document reviewed by several models and a person.

- Uses: document review, agent memory, selective disclosure.
- Fixtures: three models that assert one proposition and the decision of a person; a suppression by an actor that is not
  trusted, of a claim that is authenticated; public contributions beside contributions private to each of two audiences;
  private terms before later clear terms.

An artwork release.

- Uses: an artwork with its licence and provenance, the distribution of a dataset.
- Fixtures: an artwork with two revisions of its licence and of its catalogue record and with restricted conservation notes;
  a source of mixed audiences, opened in full and compacted for publication.

A tool capsule in two versions.

- Uses: a tool capsule, distribution, archives.
- Fixtures: a capsule with two module versions and with schema and permission versions that do not match; a distribution
  capsule whose required module is external; a valid outer bundle with a damaged, an untrusted or an unopened required
  child; an archive without entries and with metadata meant for its root, hard links, names that are not portable, and a
  path replaced by two writers; a large file changed by one chunk after a restart of the writer.

A custody chain.

- Uses: evidence and custody, replication.
- Fixtures: objects with equal heads and different signatures or storage; a cursor in a file that holds one segment twice;
  the cases of a signed frame after a parity resumption, of a consistency proof between two signed roots, and of a file
  proof of a source with no segment.

A memory and a synchronization that are safe under replay.

- Uses: synchronization, agent memory, interchange with a database.
- Fixtures: a replay, dependent deltas in another order, a repeated annotation, and blank nodes with one label; a revised
  belief with the link of its successor and a later reaffirmation; a rich graph exported to N-Quads and to a schema that
  keeps the fold; a suppression before the content it names, and a repeated suppression; the orders of independent ground
  segments; literals that are ill-typed and texts that RDF does not admit, through an adapter.

Together they exercise every use named here with one core format and no wire format for a use.

### 4.11 Before a use is relied on

- Evidence and custody rest on what a signature covers and on file proofs. The use is not relied on until the cases of
  coverage and of a file proof across a damaged region and a resumption pass in two programs written apart.
- Distribution rests on the predicate of a verified distribution (`PF-dist`, "Verified distribution"). The use is not relied
  on until the cases of a reader that runs fewer checks than the profile numbers pass in two programs written apart.
- The entries of the fixture catalogue that belong to a use are part of the corpus that a program passes before it claims
  that use.

## 5. Review of the cryptography

The limits that `P6` and `P8` state are stated limits. What adoption has to establish is whether the composition and its
bounds hold, and whether every program keeps the checks and the limits. Commission a review of the composition, not of the
primitives alone:

- what the signed subject binds, and what a signature covers and authenticates across a broken link, a detached frame and a
  commit;
- recovery beside signatures: that no step of healing or recovery takes a key, and that no result of recovery widens a
  coverage;
- the commitment and its verification, the key wrap inside the sealing construction, and the bounds that the text states for
  them, with their proofs;
- the limits for one read against the limits for the life of a key;
- what a party without a key can observe: outcomes, findings, lengths and timing.

The review states, for each claim of `P6` and `P8` and for each assumption that they name, whether the argument is complete.
Its record enters the evidence register.

What it informs: whether the assurances that the text states are those that the construction gives.

## 6. The order of implementation

The roadmap is derived from the identifiers of the requirements. It is a table with one row for each numbered requirement
of the sixteen documents. A row holds:

- the requirement: its identifier;
- the actor that the requirement names (a reader, a writer, a verifier, a consumer, an implementation and the others);
- the typed outcome: the finding and reason, the state, or the field of the result that the requirement fixes;
- the vectors: the identifiers of the entries of the corpus that pass only when the requirement is met, at least one of them
  a counterexample, that is an input that a plausible wrong program gets wrong;
- the stage: one of the stages below;
- the state: not started, implemented, passing in one program, passing in two.

The stages follow the dependencies of the parts. A stage is finished when every row of it is "passing in two".

1. The typed model and its encoding, the items, the header and framing (`P1`, "The typed model and its one serialization",
   "File, segment, items, header, offsets and locator" and "Framing: torn, damaged and undelimitable input").
2. The envelope, the identifiers, the links, the stored check and the payload check (`P1`, "The frame envelope",
   "Identifiers, hashing and every preimage" and "The checks of every frame"), and the registry (`P2`).
3. The result record, the capacity and the order of judgement, with a reader of a low stated capacity from the start (`P7`,
   "The typed result of reading" and "Capacity").
4. Terms, rows and the fold without labelling (`P3`), and blobs (`P4`).
5. The labelling (`L1`) and the content digest (`P3`, "The content digest").
6. Commits, proofs, the index, parity and recovery (`P5`).
7. Signatures (`P6`).
8. The reader procedure in full for `whole`, then `forward`, then `sparse` (`P7`, "The reader procedure").
9. The canonical form, the canonical check and compaction (`P7`, "The canonical form").
10. Sealing (`P8`).
11. The profiles, each after the parts it depends on.
12. Conversion from the first generation (`P9`).

A requirement with no vector is a gap of the corpus, and a vector with no requirement is a gap of the text. Both are listed
and closed before the stage ends.

## 7. Decisions kept, and their reasons

### 7.1 Deterministic signing

The format has one signing rule: a writer signs deterministically (`P6`, "Signing"). A verifier cannot tell a deterministic
signature from a hedged one, and no result of reading depends on the rule.

What the rule gives:

- a key that signs the same frame again makes the same record. A `signatures` list holds no record twice, and a writer adds
  no signature equal to one already present (`P6`, "The signature record" and "Co-signing and pinning"), so signing again in
  a storage rewrite or after a restore does not grow the list. Under hedged signing each act of signing gives another record
  of the same key;
- a signature can be reproduced from the key and the file, so a signer, an auditor that holds the key and a test vector
  agree byte for byte;
- signing needs no random source, so a weak or repeated one cannot leak the key;
- one rule serves production and test vectors, so the format has one signing procedure and not a second one that exists
  only for vectors.

What it costs: FIPS 204, section 3.4, states that fault attacks and side channels are harder to mitigate without randomness
(`P6`, "What a signature binds and authenticates"). Identity does not need the rule: no identifier, root or content digest
holds a signature.

What a deployment does:

- it relies on the verification before writing that the format requires of a writer (`P6`, "Signing"): a signature that a
  fault makes invalid is not written;
- a signer that an adversary can reach physically, or whose power or timing the adversary can observe, runs in hardware that
  has its own countermeasures (a security module with masked or redundant arithmetic) and that is validated for the
  deterministic variant;
- a deployment that cannot meet that, and has to sign hedged, is outside the signing rule and says so where it states its
  claim. Its signatures verify, and its files differ from those of a conforming writer only in the signature bytes.

A change to a hedged rule is a change of the one signing rule, by proposal, in a later registry version.

### 7.2 Recipe units stay in their segment

A recipe finds its units in its own segment only (`P4`, "The unit index and the unit used"). A changed large file that a
restarted writer appends in a new segment therefore carries every unit of its recipe again; a writer that continues its
segment reuses the units that stand before the recipe.

The rule stays. A resolution of units across segments is a design of its own, for sealed content above all, and its cost is
not known. The measurement `M15` is the figure that can open the question; it is not opened without that figure.

### 7.3 Reading by range carries no proof

An answer of a read by range is verified frame by frame. It is local: it is bound to no commitment of the file, and `P7`,
"Reading by range", states in full what it establishes and what it does not. This generation has no answer that a holder who
is not trusted cannot falsify.

The specification is kept as it is. An answer that carries a proof needs a new record, a commitment over the index and the
places of the frames, and vectors of its own; it is a contract of its own and not a change of the procedure of a read by
range. A caller that needs an authenticated answer reads the file with the access `whole` and applies its acceptance rule,
as `GUIDE-applications.md` describes. Interfaces name the answer of a read by range as local.

### 7.4 What a later proposal for an authenticated query binds

Adoption decides whether authenticated answers from a holder that is not trusted are a goal of deployment. The decision is
made before adoption, with the measurements `M8` to `M10`.

If they are not a goal, nothing follows but the interface rules of `GUIDE-applications.md`.

If they are, the work is a proposal under the proposal process of the format, as a registry version of its own and with
vectors of its own, for a query contract in which the answer carries a proof. The proof binds to one commitment that a
trusted key signed:

- the index used, so that it is the index of the committed segment and the last one;
- the `offsets` item, and the place of every frame read, so that each frame stands in the chain of that segment and not in a
  payload or in another segment;
- the dictionary used to resolve the rows, the `terms` frames included;
- a statement of completeness: that no other frame of the committed range holds a row, a unit or a suppression entry for
  the query;
- the suppression entries that apply, those of other segments included, or the statement that the answer is before
  suppression.

The proposal defines what "complete" means for a graph and for a blob. The measurement `M9` is evidence for it, since a
proof that names every `terms` frame costs at least what reading them costs.

### 7.5 Design decisions to examine

Each of these is a choice of the text with another possible choice. An adopter examines the choice. A program or a corpus
made to the other choice differs in the value named.

- A frame after a parity resumption is detached when the `anchor` is not the identifier before the damaged region (`P1`, "The
  predecessor and transparent frames"). An unbroken run, the coverage of a signature on a frame that is not a commit, and
  `anchored_frames` stop there. The other choice makes every resumption a boundary, whatever the `anchor`. Value that
  differs: `anchored_frames`, the coverage and `authenticated_frames` after such a resumption.
- A file proof is refused for a source in which an ordinary frame is `broken` or detached, with the reason
  `consistency_old_root` (`P5`, "File"). The other choice proves the prefix before the first such frame.
- Of the outcomes of a consistency proof between two roots that one trusted key signed, only `consistency_diverged` shows
  two histories (`P5`, "What a proof proves, and what it does not").
- A file proof of a source with no segment holds for every later file (`P5`, "File").
- A result is a verified distribution only when the reader states at least the number of checks that the profile numbers
  (`PF-dist`, "Verified distribution"). A reader that states fewer, and finds no refuting fact, gives a result that is
  neither verified nor refuted. The other choice fixes a least number of checks for a reader that claims the profile.
- Conversion keeps the scope of the blank nodes of a first-generation snapshot (`P9`, "The conversion reading"). Value that
  differs: the content digest of the conversion of a source that uses one label in a snapshot and outside it, or in two
  snapshots.
- A literal whose lexical form is outside the lexical space of its datatype is a literal of RDF; the projection that holds
  it is an RDF dataset (`P3`, "Texts that RDF does not admit").
- An unreadable observation of the container profile has no status when no response was received (`PF-container`, "Properties of
  an observation").
- The fold is the content alone. Whether a digest is present, whether the labelling is refused and whether compaction writes
  are facts of the fold as read (`P3`, "The fold"). Compaction of a read that left content out writes a file, and the writer
  keeps the source (`P7`, "Compaction").
- `convert` takes the capacity of the converter. Whether it stops depends on the source, the capacity and, in one case, the stored
  bytes that the encoder gives; the proof and the
  head claim of a result depend on the source alone; the stored bytes depend on the encoder (`P9`, "The conversion
  function").
