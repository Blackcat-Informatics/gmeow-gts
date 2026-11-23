<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Guide: Applications of generation 2

This document is an informative document of the generation-2 specification. It states no requirement of the format. The
normative documents govern, and where this document and they differ, they are right. It uses no requirement key word of its
own. It says what an application checks before it relies on a file, how the parts of the format combine in the uses that
the format serves, and how an interface names what a result establishes. It cites the normative documents by part and
section title.

## 1. Terms

"The application" is the program or person that decides to rely on a file. "The result" is the `read_result` of `P7`,
"The typed result of reading", and what a profile delivers beside it. "A caller" is the code that asks a reader for a read.
"A consumer" is an application that judges a result.

The format carries content, identities, signatures and sealed frames. It judges no vocabulary (`P3`, "Metadata is RDF"). Who
may do what, which release applies, what a licence permits and when a statement is current are decisions of the application.
Each section below says which facts of the result such a decision rests on and which facts the result does not hold.

## 2. What a verifiable package establishes

A file that reads without an error finding has established little by that alone. An application that relies on a package
answers eight questions, each from its own evidence.

| question | evidence | not enough |
|---|---|---|
| Are these the expected bytes? | each required blob has the state COMPLETE (`P4`, "Rebuilding a blob or a value"): a verified `blob` declaration or a verified rebuild, to the name and length that the selected manifest states | a binding that resolves: it resolves when a declaration exists, an `external` one included |
| Which release or decision is judged? | a release or event named by the caller, with a selected state (section 3) | the last line delivered, the last name met, or the fold of the whole file by default |
| Who endorsed it? | the keys the caller expects, among the signatures whose `authenticated` is `true`, on frames that cover the selected content (`P6`, "Coverage and authentication") | that some signature is `valid`, or that some trusted key authenticates some frames |
| Was that signer allowed to do this? | the application's table of keys and roles (publisher, reviewer, curator, custodian) | a key or a name stated inside the file; that a sealed frame opened |
| Are all required parts present? | a manifest whose required blobs are COMPLETE and whose required states and nested files resolve, each nested file by its own contract | canonical form; `presence` `present`; a verified distribution |
| Is the view complete for this audience? | a stated scope every frame of which the reader folded, with every suppression that can act on it folded (section 4.2) | treating unopened sealed frames as empty |
| Is it current? | a checkpoint accepted from outside the file, or an explicit statement that the version is taken as given | the last commit of a valid file (`P5`, "What a commit does not show") |
| Are the earlier records kept exactly? | the original append log, or the earlier signed files carried as blobs | a compacted file: it has a new commit and none of the earlier signatures |

Two cases show the distance between a result of the format and an acceptance.

A verified distribution can lack a required blob. A distribution file can declare the blob of a required module external and bind
a term to it.
The binding resolves, nothing is rebuilt and nothing is reported missing. The file is a verified distribution and is not a
complete offline package. The predicate of `PF-dist`, "Verified distribution", says nothing about the bytes of a blob whose
`presence` is `external`; the application applies its own rule to the blob set.

An outer bundle can pass while a nested file fails. The checks of `PF-bundle` judge the binding of a nested-file term, the
size, the presence of the bytes, that the bytes are a file of generation 2 with a readable header, and the depth. They do not
judge the integrity, the signatures or the visibility of the nested file. The nested reads stand beside the outer result
(`PF-bundle`, "What a reader reports about nested content"). The application reads the nested result of every child it
requires and applies that child's contract to it.

### 2.1 The application result

An application that accepts or refuses a package records, beside the result of reading and never inside it:

- the release or event selected, and the state selected for it (the effective identifier of a commit);
- the audience for which the acceptance holds, and the scope that audience is expected to read in full;
- the roles required and, for each, the key that authenticated;
- for each required blob its state, and for each required nested file the outcome of its own contract;
- the contract applied, with its revision;
- the basis of freshness: an outside checkpoint, or "taken as given".

The outcome is one of three: accepted; refused, because a fact of the file contradicts the contract; incomplete, because the
reader could not finish (a capacity, a key not held, a part not fetched). An incomplete outcome is not an acceptance with
warnings. The core result of the whole file stays as the reader gave it; a narrower acceptance does not rewrite it.

## 3. Selected release and audience

### 3.1 The release root

An append adds content and removes none: a second module, schema or licence stands in the fold beside the first. An application
that takes "the binding of the module" from the fold can take the code of one release and the permissions of another. A
package therefore names each release by a term (an IRI), and a manifest of that release names each required part by role, one
part for each role, each bound to its bytes by a binding with a `blob` referent (`P3`, "Typed references: the `bindings`
frame"). The application selects the release it was asked for and reads only what that manifest names. A request that names
no release is refused as ambiguous when two terms hold one role.

### 3.2 The order of construction

The name of a state is the effective identifier of its commit, and the commit covers the frames before it (`P3`, "Graph
states"). A statement that holds the identifier of the commit that will cover it cannot be written. The order is: write the
content and the manifest; write the commit and sign it; give the identifier of that commit to the caller as the selector.
Where RDF has to bind the release term to that state, the binding (a `state` referent) stands in a later frame, covered by a
later signed commit, or in an outer package.

### 3.3 What a selector survives

A `state` referent resolves in a file that holds a commit of that identifier which names a state to the reader. Compaction writes
a new commit (`P7`, "Compaction"), so a selector made before compaction names no commit of the compacted file and is
reported `missing_dependency` with the reason `state`. A package that relies on selectors keeps the append log, or carries the
earlier file as a blob. A compacted release is selected by its own commit.

### 3.4 The audience

The audience for which an acceptance holds is part of the acceptance. A public recipient accepts a selected state that the
public can read in full, not "the file". Section 4.2 says which states a reader without keys can name.

## 4. Selective disclosure with sealed frames

### 4.1 The audience layout

A segment can hold several key entries. Each sealed frame is under one of them, and each entry is wrapped for its own
recipients, so the frames of one segment can be readable by different and overlapping groups (`P8`, "The key table of a
segment"). The layout of a document reviewed by several models and a person shows the pattern.

| material | readers | representation |
|---|---|---|
| the published document and the public decision | everyone | clear frames with a signed commit of their own |
| the restricted analysis of model A | the service of model A, the human reviewer | frames under key A |
| the restricted analysis of model B | the service of model B, the human reviewer | frames under key B |
| the private deliberation | the review team | frames under a third key |
| the history of the transport | mirrors and auditors without keys | identifiers, commits and signatures, which are clear |

A reader that holds a key opens every frame under it. Two permissions need two keys and two sets of frames. A seal covers a
whole frame, not chosen rows of it. Blobs are sealed as whole packs, with the directory of the pack inside the seal, so every
member of a sealed pack has the audience of the pack (`P8`, "The unit of sealing for blobs is the pack").

A party without keys carries the file verbatim, and checks every identifier, link, commit and signature and the ciphertext
of every sealed frame (`P8`, "What it can do"). It cannot open a frame, fold the segment or compute a content digest. The
application that is such a party reports "the ciphertext history is intact and is signed by these keys". It accepts or
refuses no scope of plaintext.

### 4.2 Scope and completeness

A reader without a key verifies identifiers, links, commits, signatures and the digests of the ciphertext, and can pass the
file on unchanged. It does not have the hidden content, and it does not have the complete state of the file. A recipient
entitled to one private analysis verifies and uses that analysis without the keys of the others, where the layout of
section 4.3 allows it.

An application does not ask for every key. It asks for completeness within the scope it accepts, and it defines that scope so
that the reader can know it is complete. The result of reading keeps its whole-file view as it is.

- The whole-file view is complete only when `visible` is `complete`. An unopened sealed frame can hold a suppression that acts
  on clear content, so a reader with any unopened sealed frame reports `incomplete`, and the fold it has is not "the current
  public view" (`P3`, "The visible dataset and visibility").
- A state is a scope a reader can know to be complete. A commit names a state to a reader only when every ordinary frame
  before it in its run is linked or healed and is folded or is not content (`P3`, "Graph states"). A public state is therefore a
  commit with no sealed frame before it
  in its run, or with none that the public cannot open.
- A segment with `lineage` `absent` has a run of its own. A commit in it names a state to every reader that can read that
  segment, whatever stands in the other segments of the file.
- A segment appended with a checked lineage extends the run of the segment before it. A commit in a public segment that
  follows a private one names no state to the public.
- An earlier signed public commit stays a public state when private content follows it. A later commit whose run includes
  unopened sealed frames is no public state.

The accepted scope is then "the state named by this commit, signed by this role", and the application reports that scope, not
"the file". The reporting keeps unopened sealed frames apart from empty ones: an unopened frame is `missing_key`, not damage,
and a sealed frame that is not opened does not stop the fold (`P8`, "Opening a sealed frame").

### 4.3 The dictionary layout

Term ids are counted over the `terms` frames of a segment, inner frames included (`P3`, "The content record and the ids"). A
reader that cannot open a sealed frame does not count the terms of an inner `terms` frame, and every later `terms` frame of that
segment is then out of place for it and is not folded (`P8`, "What is not claimed", the item on term ids after an unopened
inner `terms` frame). The rows that name the ids of such a frame are dropped or held. A private `terms` frame placed before a
clear one therefore makes the clear one unreadable without the key.

Layouts that avoid this:

- one clear dictionary first, then the row frames of each audience under its key, when the terms themselves can be shown to
  everyone;
- the public dictionary and the public rows first, the private additions to the dictionary after them, when one private
  audience follows the public content;
- a segment for each audience, each with its own dictionary: dictionary state starts again in every segment. Two private
  audiences that each need a private dictionary need this layout, since in one segment the dictionary of the second stands
  after that of the first and cannot be counted by a reader that holds the key of the second alone;
- for public and private views that are each complete by themselves, files of their own carried as nested files, or a public
  commit whose whole run the public can read.

Statements in one segment about things of another name them by IRI: a blank node is scoped to its segment.

### 4.4 The key lifecycle

The `keys` frame of an entry stands immediately before the first sealed frame under it and holds that one entry with every
recipient entry of that key (`P8`, "The key table of a segment"). The recipients of a key are final at its first use: a later
recipient entry for a used key is refused (`P8`, "Rules of recipient entries"). Adding a reader is a new key entry for the
frames that follow, or giving that reader the existing key outside the file.

A key for one audience in one period of publication spreads the cost of wrapping over many frames: a recipient entry is 1,243
bytes for each recipient of each key (`P8`, "A recipient entry and the wrap's key schedule"). A key for every small frame
multiplies that cost and adds no boundary of access. One key over material for different audiences removes the boundary. The
break-even is a measurement to be made (`GUIDE-adoption.md`, the measurement of keys, recipients and dictionary layouts). A key is
not shared between unrelated writers
or periods to save bytes, because every holder of a key can seal under it and the per-key limits count the use of all holders
(`P8`, "What is not claimed", the items on every holder and on the total use of a key).

A reader given an old key opens every frame ever sealed under it. A reader removed from an audience keeps what it holds. After
a change of audience the writer takes a new key for what follows. The format has no forward secrecy and no recovery after a key
is compromised. The identifiers of recipient keys, the grouping of frames by key and the padded lengths are visible to
everyone (`P8`, "What leaks, by design").

### 4.5 Confidential publication and compaction

Suppression keeps content: a hidden line stays in the fold and in the file. Sealing gives no erasure. A workflow that has to
forget bytes physically keeps them in a store of its own, with a retention rule for the bytes and for the keys, and writes into
the file only what may stay.

The canonical file of a fold is clear. A reader that holds every key and compacts a file with public and private content gets
one clear segment that holds all of it, the suppressed content included, with a new commit that has no signature (`P7`,
"Compaction": it keeps hidden content and writes in clear frames the content of every inner frame that the read opened). The
application selects the export scope before it constructs and signs:

- to publish the public part, it compacts a file that holds the public segments alone, or the bytes up to a public commit,
  never the fold of its own full read;
- private content that the release keeps is written sealed again, in a segment that is not canonical, or travels as a nested
  file;
- it reports the scope of the source and the scope of the output separately, and it does not sign a canonical file made from a
  read that opened sealed frames outside the scope.

### 4.6 Two shapes for a release with private content

1. One file with clear and sealed frames, and an application contract that names the scope of each audience (section 4.2).
   The file is not canonical and is not a distribution file.
2. A clear outer file, complete for the public, that carries each private part as a blob whose bytes are a file of its own,
   with its own sealed frames and signatures. The outer file can be canonical and a distribution file. Verifying the outer
   bytes shows that the child's bytes are the ones named; it does not show that the child was opened or accepted. The child
   is read as a nested file and judged by its own contract. A segment declares one profile, so an outer segment that is to be
   a distribution and to have its nested files read under `bundle` cannot declare both. The application either reads the
   children itself from their blobs, or declares `bundle` and applies the conditions of a distribution as its own rule.

For a `dist` release the second shape is the only one, because `PF-dist` forbids `keys` and `sealed` frames.

## 5. The use cases

### 5.1 A document reviewed by several models and a person

Names. Claims, review events, participants and the document are named by IRIs, because they are named again from other
segments. Five things stay apart: the proposition asserted; the event of asserting it; the model and version that asserted it;
the confidence given; the decision of the person. A triple shared by two participants says neither who asserted it nor about
which version of the document.

Evidence. The document is a blob, and a term bound to it by a `blob` referent stands for those exact bytes. A page or passage
selector is stated relative to that term, with the convention of the selector named. The format carries a selector; it does
not make a page number mean the same in two revisions of a document.

Disagreement and decisions. Each assertion of each participant has its own identity and provenance. Acceptance, rejection and
qualification are statements of their own, with their reasons. Suppression changes what is visible and keeps the content; it
explains nothing, so a decision is never expressed by a suppression alone.

Who may change the accepted view. A folded suppression hides content whether or not it is authenticated; the count
`unauthenticated_suppress` says how many are not (`P3`, "The visible dataset and visibility"). A suppression authenticated by a
trusted model key is authenticated, and that does not make the model entitled to hide the reviewer's decision. `PF-evidence`
checks authentication. The application holds the table of which role may issue which kind of statement and which suppression,
and computes its accepted view from the fold and the suppression record with that table. The fold keeps hidden content for that
purpose.

The signed decision. Authentication and coverage are per segment and do not extend through a lineage (`P6`, "Coverage and
authentication"). The reviewer signs a decision that names the document reviewed and the states or contributions selected, in a
segment of the reviewer's own, and the application has a rule for reading that statement. The authorship of each contribution
rests on the signature of its participant; the reviewer's later statement is another fact.

Audiences. Public claims and the private analyses of the models follow section 4. The public outcome is a state that the
public can read in full, and it does not require the working material of any model.

The path that serves best. Assertion and review events that are never rewritten; one projection, aware of roles, that gives
the current decision; state selectors for a review that has to be reproduced later. The append log is kept. A smaller signed
package of the current review is published when it is useful. Writing the canonical file of the whole growing history after
each small event is avoided unless the measurement of that cost in `GUIDE-adoption.md` shows it affordable.

### 5.2 An artwork release

The release manifest. A release names: the master image; each derivative and its relation to the master; the attribution text
required; the licence statement that applies; the revision of the catalogue record; the restricted records that exist, as
optional parts. Each image is bound to its bytes, so a licence statement names a digest and not "the image". The application
maps the key of a curator to the role of curator.

The issuer. A trusted key shows that its holder signed. That the holder is the curator or the rights holder is the
application's table of keys and roles, and the signature covers the frames of the release selected (`P6`, "Coverage and
authentication").

Corrections and what applies. Later licence and catalogue statements stand beside the earlier ones; the fold does not choose.
Each revision is a record of its own, a correction names what it corrects, and a release names the revision that applies to it.
A request for "the licence of the image" with no release is ambiguous. Where "the current licence" matters, the release is
compared with a checkpoint published outside the file. A revision that retires an earlier occurrence uses a `reifier`
suppression entry when the link from the successor is to stay visible (section 5.8).

The package. The signed preservation history is kept. A public release is closed: it holds every image and every statement it
requires. Where the history travels with the release it is carried as a nested file, exact. Private conservation notes are a
separately scoped sealed child or sealed frames by section 4, so that a public recipient verifies the publication with no
conservation key. The format decides no copyright question and ranks no licence.

### 5.3 A tool capsule

The release. The host loads one release, named by the caller (section 3.1). A capsule to which a second module was appended
holds two modules.

The acceptance, in order:

1. Select the release asked for and its manifest.
2. Require exactly one of each: module, interface and schema set, selected knowledge scope, binary-interface declaration,
   permission declaration, test set.
3. Require every part needed offline to be COMPLETE, to the name and length that the manifest states.
4. Require the publisher and reviewer roles among the authenticating signatures over the selected release, and decide what is
   required of each segment that contributes to it.
5. Read each required nested file and apply its contract; record optional restricted children apart.
6. Check the stated provenance of the selected knowledge and record what was selected. A content digest fixes what was exported;
   it does not show that the query that selected it left nothing out.
7. Give the host one outcome, accepted, refused or incomplete, before any policy of execution is applied.

The host keeps execution, the grant of permissions, the sandbox and the running of the tests.

Distribution. A capsule of one selected release for execution; earlier releases exact, as nested files or as the append
history beside it. Proprietary modules or confidential knowledge can be sealed for recipients, and the scope that is to be
executed is complete for the host expected to run it. A host entitled only to the public metadata reads that metadata, and its
outcome for "runnable capsule" is incomplete, not accepted.

### 5.4 Datasets, ontologies and packages

Required assets and dependencies. A verified distribution shows one canonical segment that declares `dist`, whose frames are
within the per-frame minimum capacity, with every check of the profile run and no violation (`PF-dist`, "Verified
distribution"). A package contract adds: the list of required blobs, each COMPLETE; the releases it depends on, each pinned by
a digest or a state; the publisher role. An `external` declaration is legitimate content and does not meet a promise that
everything needed travels in one file.

What the carrier does not know. The format reads no vocabulary. A validation profile of a vocabulary, the revision of an
imported ontology and a rule of interpretation are named and versioned by the package contract; none of them holds because the
carrier is valid. The dependency runs one way: an ontology package, such as a GMEOW package, is a file of this format, and a
reader of this format needs nothing of
the vocabulary of the package.

The route. Canonical signed releases for stable publication; manifests that pin required blobs and imports by content; nested
packages for large or differently protected dependencies; documented projections for consumers that need RDF alone, with what
the projection loses (`P3`, "The RDF projection of a fold"). Reasoning, domain validation and the policy for fetching imports
stay above the format. Not every fold has a distribution file (section 7.3), and the cost of labelling and of large canonical
frames is measured on the largest real release first (`GUIDE-adoption.md`, the measurement of the canonical file of the intended
datasets).

### 5.5 Archives and file trees

The root. A tree under `files` can have a root entry: the entry whose path is the empty path (`rdf:nil`), of kind `Directory`,
that states the permissions, the owner, the time of last modification, the attributes and the records of the root of the tree
(`PF-files`,
"Paths"). A tree need not have one, and an archiver that has to keep the metadata of the archived directory writes it. Two
entries of one path, a root entry included, are reported by check `files_duplicate_path`.

Revisions and two writers. A change hides the old entry, its record nodes and the hard links that name it, and writes a new
entry under a term that nothing hides (`PF-files`, "Writer rules"). Two replicas that each replace one path leave two visible
entries with that path, and the profile reports `files_duplicate_path`. That is a conflict found, not a version chosen. The
application keeps the relation between revisions as statements and resolves the conflict by a decision: a further segment that
hides one of the two.

Extraction. The profile defines the tree. Writing it to a file system is a projection with a result of its own, under a
contract that has a version. A component is a name of bytes and can hold `/`, a zero byte or `../`; the profile judges none of
that (`PF-files`, "Writer rules", A COMPONENT IS A NAME, NOT A PATH). An extractor never joins components into a path by
concatenating bytes.
By default it refuses, and for each entry it does not write as stored it reports which of these applies: a name the host cannot
hold; two names that the host treats as one; a special file that the policy does not create; a link that would lead outside
the destination; hard-link metadata that disagrees with its target; an attribute the host does not support. It does not write
an entry under another name silently. Where a binding to a system interface cannot carry every byte string, it refuses the
entry and reports it, or maps the name by a stated, reversible encoding and reports that it did, and it lists every name
through an interface that carries a length.

Layout. In a canonical file the packs stand before the terms, the paths and the bindings (`P7`, "The construction, from a fold
to the item sequence"). An extractor that reads once holds or spools the units before it knows a path; a content-addressed
temporary store bounds memory and costs disk and time to the first file. For extraction while a file arrives, a file that is
not canonical, with entries and bindings before the packs, can serve better. A recipe resolves its units in its own segment
only (`P4`, "The unit index and the unit used"), so a changed large file appended in a new segment carries all its units
again, and a writer that continues its segment does not. The costs of both layouts and of the appended segment are measurements to
be made (`GUIDE-adoption.md`, the measurements of histories and archives).

The route. A static-tree profile, a versioned extraction contract, a revision model and the measured costs of creating an
archive from a file system.

### 5.6 Evidence and custody

Custody events. A custody chain is more than commits. Each transfer is an event, stated in RDF, that names: the evidence (a
term bound to its bytes); the custodian who gives and the one who receives; the event or checkpoint accepted before it; the act
and the time claimed; the signatures or receipts expected for it. The format has no timestamp; a time is a claim of the signer.

What a signature shows. The profile `evidence` shows that every commit of a segment carries a signature of the pinned algorithm
by some key that the reader trusts (`PF-evidence`, "Checks", WHAT THE CHECKS ESTABLISH). A signature that passes shows that the
holder of a trusted key signed a commit over those frames. It does not show which key signed a commit when it was written: a
signature list is outside every identifier, so the holder of one trusted key can add its signature to every earlier commit and
remove the others, and no check fails. It does not show that the intended recipient accepted custody, that an object changed
hands, or that the commit shown is the latest: a rollback to an earlier valid commit is not detected from the file
(`P5`, "What a commit does not show"). The application requires the role of each party of an event among the authenticating
signatures, and compares the head with a checkpoint held outside the file where "latest" matters. In both contracts below the
consumer also keeps the last `root` it accepted, and asks for a consistency proof against it at the next read (`P5`,
"Consistency").

A custody transfer that a consumer can verify. An application for which the sequence of custodians is itself the evidence uses
one of two contracts, or both.

THE TRANSFER IS CONTENT. Each transfer is a set of lines in a frame that a later commit covers. It names:

- the actor that hands over, and the actor that receives (terms of the application's vocabulary);
- the state handed over: the effective identifier of the commit that closes it (a graph state, `P3`, "Graph states"), or its
  `root` and `count`;
- the receipt before it: the same name of the previous transfer, so that the transfers form a chain inside the content;
- what else the application needs (a time claimed by the actor, a place, a reason).

The actor that hands over signs the commit that covers these lines, and the actor that receives signs the next commit, which
covers the first by the chain (`P6`, "Co-signing and pinning"). Removing a signature later does not remove the lines: they are
under the content digest and under the root of every later commit. A consumer checks the lines against the keys: for each
transfer, the commit that covers it has an authenticating signature by the key of the actor that the lines name. The mapping
from an actor to a key is the consumer's own input.

THE SIGNERS ARE PINNED OUTSIDE THE FILE. The consumer holds a list, obtained apart from the file, of which key signs the commits
of which positions or of which segment (by `lineage` ordinal). After the read it compares, for each commit, the `key_id` of the
authenticating signatures with the list, using the `signature_result` records alone: the decision about a set of signers is the
caller's (`P6`, "Co-signing and pinning"). A commit whose listed key is not among its authenticating signatures fails the
consumer's rule, whatever the profile check gave.

Receipts. A signature list can be replaced: a signature removed leaves a file that no reader can tell from one never signed by
that key. Where the history of endorsements matters, the receipt is kept exact: the earlier signed file as a blob, or its digest
in a statement covered by a later signature. A statement about an earlier receipt carries the receipt's bytes or a binding to
their digest. The nested file is read by its own contract (`PF-bundle`, "What a reader reports about nested content").

Sealed evidence: four facts. They are reported apart, never as one "verified":

- the continuity of the ciphertext is authenticated (commits and signatures over the sealed frames);
- the plaintext that this custodian opened and verified;
- the completeness of the selected evidence scope for this custodian (section 4.2);
- the chronology or freshness established outside the file, where it is required.

Sealing names no sender. Every holder of a segment key can seal under it, and a recipient entry can be written by anyone who
knows the recipient's public key (`P8`, "What is not claimed", the items on origin and on every holder). The application
requires the signing roles over the `keys` frames and the sealed frames of the scope, by an authenticating signature on a
commit after them.

The route. An append-only log under `evidence`, with signed custody events and receipts kept apart; release or checkpoint
artifacts made from it for distribution. The canonical file of a large fold has one commit and can exceed the cadence of the
profile (`PF-evidence`, "Writer rules", "Canonical file"), so compaction does not replace the log.

### 5.7 Synchronization by concatenation

What the format gives. The concatenation of two files of this generation is a file, and its fold is the fold of all its
segments (`P1`, "Growing a file", CONCATENATION). That fold is not in general the union of the two folds. Five facts decide
what a synchronization layer adds:

1. A segment occurrence is a segment. A file that holds the same segment bytes twice has two segments; each blank node carries
   the ordinal of its segment, so the same label in the two copies is two nodes (`P3`, "The term model"; "What the order of
   frames and the order of segments decide").
2. Annotation lines are counted. Two rows with equal reifier, predicate, object and graph are one line whose count is the sum
   (`P3`, "The fold", "Definition"). A replayed segment that holds an annotation raises its count.
3. Quads, reifier bindings and empty-graph lines are sets. A replayed segment that holds only such lines over terms without
   blank nodes adds nothing, and the content digest of the file is unchanged (`P3`, "What the digest is a function of").
4. A value serves only what stands after it. A term that names a value by reference is readable only when a declaration of the
   value stands before its `terms` frame, in its segment or an earlier one (`P3`, "`literal_ref`: a lexical form held by a
   `value` frame"). A recipe finds its units in its own segment only (`P4`, "The unit index and the unit used").
5. Suppression is monotone and independent of order. An entry hides what it names whether it stands before or after it, in
   any segment; no later line and no repeated entry restores a hidden line (`P3`, "Suppression hides").

The contract of a synchronization layer. A layer that claims that its replicas converge states each of these:

| element | what it states | why the format needs it |
|---|---|---|
| operation identity | every exchanged segment (or delta file) has an immutable identity: the effective identifier of its last valid commit together with its header identifier, or the digest of its bytes | facts 1 and 2: the format does not tell a replay from a second operation |
| duplicate filtering | a replica drops an operation whose identity it holds, before it composes | facts 1 and 2 |
| dependencies | an operation is self-contained (it defines every term, value and unit it uses) or declares the operations it depends on | fact 4 |
| causal order | where operations depend on one another, the composed file orders a dependency before what depends on it, by a rule that every replica computes alike | fact 4; the order of segments also decides the ordinal of each blank node and the holder of a frame |
| shared entities | an entity that two replicas speak of is named by an IRI, never by a blank node | fact 1 |
| suppression authority | which signer may suppress what, judged by the application over `authenticity` of the frames and the suppression record | fact 5; the format hides by an entry whoever wrote it and reports `unauthenticated_suppress` |
| concurrent revisions | a rule for two replacements of one thing, and for a re-addition after a suppression: each revision is an event with an identity of its own (section 5.8), and the rule names the winner or keeps both | fact 5: an identical re-addition stays hidden, so a revived statement needs a new term or a new reifier |

The restricted model under which union holds. For a set of operations filtered for duplicates, each a self-contained segment of
`quad` lines over IRIs and literals stored as text, with no annotation, no blank node, no reference to a value, no binding, no
suppression and no `empty_graph` line, the fold of every order of composition is the same set, the union of the operations, and
the content digest of the file is the same. A layer that claims convergence for more than this model states which of the seven
elements it relies on. The format is not a mutation protocol and decides no conflict.

One writer. A file has one writer at a time (`P1`, "Growing a file", ONE WRITER). Producers that work at once each write a
segment or a file of their own; the layer concatenates them, or one owner serializes the appends.

### 5.8 Agent memory and revision of beliefs

The model. A belief is an ASSERTION OCCURRENCE: a reifier term of its own (an IRI minted for the occurrence) with a
`reifier_binding` line to the triple term it asserts, and annotation lines on that reifier for the actor, the reason, the
evidence, the cause (the occurrence it supersedes or derives from) and the claimed time. Two occurrences of one proposition are
two reifiers. A revision is a new occurrence that names the earlier one, signed by its author; the earlier occurrence is hidden
only where the application wants it out of the visible dataset.

Which suppression entry. The kinds differ in what they hide (`P3`, "The hiding rule"):

| entry | hides | effect on a revision chain |
|---|---|---|
| `term` of the old reifier | every line in which the term occurs, at any depth, and its bindings | hides the successor's own link to the old occurrence too, because its object is the old reifier; the audit chain is cut in the visible dataset |
| `reifier` of the old reifier | every `reifier_binding` and `annotation` line whose reifier is the old one | retires the old occurrence and leaves the successor's link visible |
| `annotation` or `reifier_binding` with all fields | that one line | retires one statement of the occurrence |

A memory that retires a predecessor and keeps the audit link uses the `reifier` entry. A memory that needs the whole history
readable reads the `whole` variant of the projection (`P3`, "The RDF projection of a fold"), or keeps revisions as events and
hides nothing.

What the entry does not hold. A suppression entry is a kind and its terms; it has no reason, actor or time (`P3`, "The wire
record"). They are annotation lines of the revision occurrence, in a frame that the revising party signs.

Recall order. A whole read delivers every listing in the order of the canonical file, not in the order of writing (`P3`, "The
fold", THE ORDER OF DELIVERY). "Most recent" is a query over the claimed-time and cause annotations with a stated tie-break (for
example claimed time, then the reifier IRI bytewise); it is never the last row delivered. A reaffirmation of a retired belief
is a new occurrence with a new reifier: lines identical to hidden ones stay hidden.

Latency. The application keeps an index of its own beside the file, appends complete events (the occurrence, its annotations
and, where signed, its commit) in one continuous use of the file (`P1`, "Growing a file"), and publishes a derived checkpoint (a
canonical file of a selected state) at intervals while it keeps the signed log. Compaction writes a new commit without a
signature, and a `state` referent made for the log names no commit of the canonical file (`P7`, "Compaction").

Confidential memories. Each audience has a segment key of its own (section 4.4). Hiding removes nothing from the file, and
sealing has no ratchet and no erasure: whoever obtains a segment key opens every frame ever sealed under it. Section 4.5 states
what a workflow does that has to forget.

### 5.9 Bounded export of knowledge

An export of a part of a memory is a new artifact. The exporter:

1. selects a state that is available to it: a graph state named by a valid commit, or the fold of a whole read whose `visible`
   is `complete` and which left nothing out;
2. selects the assertions that the recipient is authorized to hold;
3. applies its revision policy (section 5.8) and says which: visible only, or with retired occurrences;
4. enforces the size limit asked for, and says what was cut;
5. says whether the evidence and the payload bytes are included, or named as external blobs;
6. writes the selected occurrences with their provenance as a new file, with a statement of the source (the effective identifier
   of the commit of a graph state, or the content digest of a whole read) as RDF of the export.

The content digest of the export is the digest of the export. The digest of the source is not the digest of the part, and a
proof that a frame stands in the source proves nothing about the completeness of the selection (`P5`, "What a proof proves, and
what it does not"). The completeness of the selection is a claim of the exporter, which it signs.

### 5.10 Interchange with a database

The projection. `P3`, "The RDF projection of a fold", gives the quads `Q` and the set `G0` of empty named graphs, for the variant
`visible` or `whole`. It loses the annotation counts, the kind of a line, the pairs of VALUES, the bindings, the suppression
record, the blob declarations and the bytes. A stream of N-Quads carries `Q` and cannot carry `G0`. An import of a projection
gives a fold of `quad` and `empty_graph` lines, which in general has another content digest than the source, and no signature
of the source applies to it (`P3`, "Import and the round trip").

A schema that keeps the fold. An adapter to a relational or columnar store that is to keep more than the projection has a
schema with a version of its own and these tables:

| table | columns | keeps |
|---|---|---|
| `source` | schema version; the whole-byte digest of the artifact; its content digest or that it is absent; the state selected (a commit identifier, or the whole fold); the variant; `visible` and `integrity` of the read | the selected source state and the identity of the original artifact |
| `term` | a key; the kind (`iri`, `literal`, `lang_literal`, `blank`, `triple`); the texts byte for byte; for a `blank` the segment ordinal and the label; for a `triple` the keys of its three components | typed terms and the scope of blank nodes |
| `line` | the kind (`quad`, `reifier_binding`, `annotation`, `empty_graph`); the term keys of its slots; the graph; the count of an annotation; hidden or not | the kind of a line, the graph, the count, the empty named graphs |
| `binding` | the term key; the referent kind and its name or digest; hidden or not | bindings |
| `suppression` | the kind of the entry and the term keys or the blob name of its fields | the suppression record |
| `value` | the term key of the literal; the kind of the typed value; its octets | the pairs of VALUES |
| `blob` | the qualified name; the length; `present` or `external`; the media types; the rebuild state; the bytes or their absence | the names of the assets and their availability and verification |

What an adapter states. The term forms that its target represents; that it writes every text byte for byte; and a LOSS REPORT,
a machine-readable record with one entry for each construct of the source that the target did not receive (the construct of
`P3`, "What each construct gives and what it loses", and the number of instances). A loss of the target beyond the projection (a
column type that folds the case of a language tag, a length limit) is a separate entry. An adapter reports two outcomes
separately and never as one: whether every text of the result is admitted by RDF (`P3`, "Texts that RDF does not admit"), and
whether each typed literal is in the lexical space of its datatype, which the format does not judge and an adapter can.

The original bytes. Rows of a database do not rebuild the frames, the commits or the signatures. Where a signature has to be
verified again, or the artifact examined, the artifact itself is kept, and the `source` table names it by its whole-byte digest.

## 6. Transport

### 6.1 Streaming and interrupted writes

What a forward read is. A read with the access `forward` delivers items as they arrive and holds keyed stores (`P7`, "Reading
as a stream"). It forms no fold of the file, resolves no referent across segments, labels nothing, rebuilds no blob or value,
repairs nothing by parity, gives no content digest, and runs the checks of the profile `stream` and of no other (`P7`, "Access:
the steps by access"). A consumer that shows content as it arrives therefore holds a PROVISIONAL VIEW:

- a row delivered now can be hidden by a `suppress` frame that arrives later; the view applies each entry when it arrives, to
  the rows already shown and to those that follow;
- a row that names a value by reference is delivered by reference, and whether the value rebuilds is not known;
- rows after a broken link are delivered held, and no later commit releases them;
- a frame is authenticated only when a signed commit that covers it has arrived.

The pattern. A forward sink with persistent keyed stores (the dictionary, the value table, the lengths); a view of the consumer
built from the arrival items, which it can rebuild; and, where a result equal to a whole read is needed, a read with the access
`whole` over the bytes kept, once the end is declared. Two stages are distinct: the forward read finds a resync item after lost
framing and reads on (`P1`, "Finding a resync item where framing is lost"); the repair of damaged bytes by parity is a step of
the whole read (`P5`, "The one recovery procedure"), and the reading of a repaired copy is a new read of other bytes.

Commits. The cadence of `PF-stream` counts frames: at most `200` ordinary frames stand between two valid commits. It bounds no
time and no number of bytes. A writer commits earlier wherever its application flushes or publishes. A valid commit is a
logical commitment; it is no evidence that the storage kept the bytes (`P5`, "What a commit does not show"). A writer that needs
that evidence obtains it from its storage and records it outside the format.

Interrupted writes. A file whose last item is incomplete reads with the finding `torn_tail` once the end is declared; frames
after the last valid commit are the uncommitted tail (`P5`, "The committed prefix"). A writer that was interrupted does not
continue the segment it wrote earlier. A writer appends frames only to a segment that it wrote in one continuous use of the
file, and otherwise appends a new segment (`P1`, "Growing a file"). A co-signer appends a commit only at the end of a file that
ends at an item boundary and writes nothing otherwise (`P6`, "Co-signing and pinning").

Writers. One writer holds a file at a time. Producers that work at once publish segments or files of their own (section 5.7).

### 6.2 A read that did not complete

A read ends in one of two ways.

A RESULT. The reader returns the typed result of `P7`. Everything that a limit of the reader refused is in it: a frame beyond a
limit is `unchecked` with `exceeds_capacity`, a fold beyond one has `labelling_refused`, and the `capacity` that the result
records explains each. P7-10 defines the comparison between readers. A source that fails to
deliver bytes at an offset is an input of the read and gives a result with the finding `read_failed` (`P1`, "A failed read is not
the end of the input").

NO RESULT. The process was stopped by something that is not a rule of the format: a time limit, the exhaustion of memory, a
cancellation by the caller, the loss of the host. The reader then returns no result of any kind, no answer, no further stream
point, and no part of one (`P7`, "Conformance").

An application keeps the two apart:

- it reports a stopped read in words of its own ("the read did not complete") and never with a finding code, a refusal reason
  or a state of the format;
- it does not keep, show or pass on the content that the reader had delivered before the stop as if it were the content of
  the file;
- it does not retry with a "smaller" read in the hope of a partial result. To bound the work of a read, it selects a smaller
  `capacity` before the read. The refusals that follow are typed and repeatable;
- it does not treat a stopped read as evidence about the file: the file is neither verified nor refuted.

An application that has a budget in bytes or in items states it as the `capacity` of the read. A budget in time has no field:
it is met by choosing `labelled_lines`, `labelled_bytes_per_read`, `labelling_units` and the byte totals so that the largest
admitted read fits the time (`GUIDE-adoption.md`, the measurements of the capacity of a reader are to give the relation), and a
read that still
overruns is a stopped read.

### 6.3 Reading by range from a remote object

What an answer is. A read with the access `sparse` returns an `answer`. An `answered` answer shows that the frames it read were
each verified on their own and that its rows are the rows those frames give over the dictionary read. It does not show that the
frames belong to the chain of one segment, that the rows are all the rows, that a row is absent, that a row is not hidden by a
`suppress` frame elsewhere, or that any frame is signed (`P7`, "Reading by range", WHAT AN ANSWER ESTABLISHES, AND WHAT IT DOES
NOT). A holder that builds a false `offsets` item can make a reader return rows that no read of the whole file gives. A read
by range opens no sealed frame. A caller that needs more reads the file whole.

Interface naming. An interface keeps this boundary visible:

- the operation and its result type carry the word `sparse`, `range` or `unverified` in their names (for example
  `read_range_unverified`, `SparseAnswer`), and the result type is not the type of the content of a whole read and does not
  convert to it without a call whose name says so;
- the result type has no accessor for a count of the graph, for an absence, for a content digest or for an authentication
  state, since the answer holds none;
- the documentation of the operation states, in its first paragraph, the sentence "This answer is not a query result of the
  file";
- an `unknown` answer is shown with its reason and is not retried against the same holder as if it were a transient fault.

One byte string. Every range of an answer, and every byte kept from an earlier answer, is of one byte string. Over HTTP the
caller pins it with the validator of the object (an entity tag sent with `If-Match`, or a version identifier of the object
store) on every request, and discards what it kept when the validator changes. An object that changes between two requests
gives no answer that the format defines.

Requests. The procedure states which ranges are read, in which order, and what each is charged; how the bytes are fetched is
free (`P7`, "Reading by range", the requirement on ranges and requests). A reader joins adjacent ranges, reads ahead (the tail
of the file holds the locator, the `offsets` item and the index), and keeps the covered `terms` frames of a segment for later
queries of the same byte string. The answer and its charges are those of the procedure. A kept dictionary is charged again in
the next answer. A read-ahead that fails beyond the point at which the procedure stops changes nothing.

Layout. A graph query reads every covered `terms` frame of the segment, and a unit in a pack costs the whole pack frame. A
writer that serves range reads and does not need the canonical file chooses its layout for them: one segment for each group of
graphs that is asked together, so that its dictionary is small; packs sized to the units that are asked together. The canonical
file has one fixed construction and offers no such choice.

A blob by its digest. The target `unit` finds a unit. A blob that one `blob` frame or member holds is found by its digest; a
blob declared by a `recipe` is not, and neither is a value (`P7`, "Reading by range", THE QUERY). A holder that serves a blob by
digest from a remote object keeps a CATALOGUE: for each blob of the blob set of one whole read of the byte string, its name, its
length, and the item number and member index of its recipe and of each unit of the recipe, with the whole-byte digest of the byte
string it was made from. The catalogue is trusted as that whole read is trusted, and no further; each unit fetched through it
is still verified by its digest, and the bytes assembled are verified against the name of the blob.

Private content. A segment whose index names a `keys` or a `sealed` frame gives `unknown` with the reason `sealed`. Sealed
content is read with the access `whole` or `forward` by a party that holds the key.

An authenticated answer from a holder that is not trusted. No read by range gives one in this generation. An application that
needs it has two courses today:

1. read the whole file with the access `whole`, apply its acceptance rule to the `read_result` (the signer of the commits, the
   predicate of the profile), and query the fold itself;
2. obtain, from a party it trusts, the head of the segment and the content digest; fetch the segment; read it with the access
   `whole`; compare. The answer by range is then a preview that this read confirms or replaces.

### 6.4 Replication: three identities, inventory and resume

Three identities answer three questions. An API that offers them names each one and does not offer one in the place of another.

| identity | compare it to decide | it does not cover |
|---|---|---|
| content digest (`P3`, "The content digest") | whether two files hold the same labelled content | the layout, the frames, the commits, the signatures, the history of appends |
| head, and the root of a commit (`P1`, "What the identifier commits to"; `P5`, "What the root binds") | whether two segments have the same frames in the same order, or one extends the other (a consistency proof) | `storage`, the stored payload bytes, the `signatures` lists, transparent frames (parity), the `offsets` item and the locator |
| digest of the whole bytes, or an immutable object version | whether two replicas are the same bytes | who wrote them, and whether they are current |

A protocol that compares heads alone treats as equal two files that differ by a signature added to a commit, by the storage of
a payload or by parity. A replica that has to hold the co-signatures of its source compares whole bytes, or the application
makes such a change visible in the chain (a co-signer appends a commit, `P6`, "Co-signing and pinning").

Inventory. A replica states what it holds as one entry for each segment, in order: the ordinal, the header identifier, the head
and the number of ordinary frames. These are the values from which a prover builds a file proof (`P5`, "File"). A file proof
shows that a later file extends the earlier one at the level of identifiers, and not that the bytes are equal.

Proofs. A proof is a statement about identifiers. An inclusion proof shows that a frame stands in the prefix that a root
closes. A consistency proof shows that a later root extends an earlier one. Against a root that an authenticating signature
covers, they show a fact about the signer's history. Against any other root they show agreement with that root only (`P5`,
"What a proof proves, and what it does not"). A failed proof is not by itself evidence of two histories.

Cursors. A frame identifier is not a place in a file: a segment that stands twice in a file holds every identifier twice. A
cursor from which a transfer resumes is the triple of the source snapshot (the digest of the whole bytes or the object
version), the ordinal of the segment, and the ordinary position of the frame (or the byte offset of an item boundary). A
receiver checks the frame at that place against the identifier it holds before it resumes.

## 7. Profile composition and the shapes of an artifact

### 7.1 One profile for each segment

A header declares at most one profile, and a reader runs the checks of that profile on that segment (`P7`, "The profile
mechanism"). A reader that implements six profiles runs one of them on a segment. A file of a `dist` segment and a segment of
another profile is no distribution file (`dist_not_alone`).

### 7.2 The application acceptance contract

A use that needs the conditions of several profiles at once (a signed, offline-complete tool capsule with a file tree) is an
APPLICATION ACCEPTANCE CONTRACT: a document with a name and a version that states

1. the access and the capacity of the one read it judges (`whole`), the `inputs` (the trusted and the held keys) and the sealing
   input;
2. the conditions on the fields of the `read_result` that it requires (for example: `integrity` `intact`, `visible`
   `complete` with `unauthenticated_suppress` `0`, `authenticity` `authenticated`, nothing left out, no blob of presence
   `external` among those the manifest requires);
3. the conditions on the content (exactly one manifest, the versions that belong together);
4. for each nested file it requires, the conditions on its `nested_result`;
5. what it reports, under its own name.

A contract that borrows the ceilings of `PF-dist` or the path rules of `PF-files` for a segment that does not declare that
profile reports the outcome of its own check. It does not report a verified distribution: that predicate holds only for a
result whose one segment declares `dist` and whose reader ran its checks (`PF-dist`, "Verified distribution").

### 7.3 Writing a distribution file

Not every fold has a distribution file. `PF-dist` fixes two ceilings of `2^26` bytes, and the canonical file of a fold can hold
a frame above them (`P7`, "The canonical frames that can exceed the minimum capacity": a large index, a long recipe or value
list in a pack, a long pack directory, one very long term, long labels, a long media type, a long signature list). No repacking
changes this, because the canonical file of a fold is fixed by the fold.

A writer interface makes this a result of its own:

- the operation that writes a distribution file returns one of two outcomes, a file or `no distribution file for this fold`,
  and the second names the frame type and the length that exceeds a ceiling. It is not an error of the input and not a failure
  of the writer;
- the operation never returns the canonical file with the profile `dist` in that case (`PF-dist`, "Writer rules");
- the documentation of the operation states the condition before the description of success;
- an application that has to deliver such a fold delivers the core file (canonical or not) without the profile, or splits the
  content into several folds by a rule of its own. Both are choices of the application: the profile offers neither.

`dist` is the form for a dataset that fits the ceilings and is to be verifiable by every reader that claims the minimum
capacity (`P7`, "Capacity"). It is not the interchange form of every fold that the core can express.

### 7.4 The three artifact shapes

| shape | use | accepted on |
|---|---|---|
| an append history with explicit events and checkpoints | collaboration, custody, memory, observations that accumulate | continuity (lineage checked, consistency of the roots a party recorded), the signers that contributed, the authority of each revision, the state selected |
| an immutable selected release | the execution of a tool, the publication of an ontology, the reproduction of an artwork | one manifest, every required asset present and rebuilt, the endorsements intended, an explicit version |
| an outer carrier with child files of their own scope | exact earlier releases, private audiences, nested evidence | the binding of the outer bytes to each child (a blob name), and for each required child its own acceptance and what of it was opened |

The shapes coexist in one delivered file as nested files, and each keeps its own guarantee: the outer result says that the bytes
of a child are the bytes named, and the child's `nested_result` says what the child is. An outer result without a violation is
no acceptance of a child. A consumer judges a nested file by the `result` of its `nested_result` and by nothing else, and treats
a nested file that has no `nested_result` as not read: its bytes were unavailable, above the size ceiling or beyond the depth
(`PF-bundle`, "What a reader reports about nested content"). What a consumer asks of the child's result (signers, visibility, a
profile of the child) is the contract of section 7.2.
