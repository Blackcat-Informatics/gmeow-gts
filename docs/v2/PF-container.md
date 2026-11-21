<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Profile PF-container: Container archive

Title of this document, as the profile table of `P2` names it: `Profile PF-container: Container archive`.

## 0. Scope and dependencies

This profile defines a CONTAINER ARCHIVE: the record of a tree of web resources retrieved from a server, export after export,
each resource with its representation (a graph of RDF statements or the bytes of a blob), its containment and its metadata as
served. An EXPORT is a graph state of the core, named by the effective identifier of the commit that closes it. Removal and
change are recorded as content (tombstones and versioned graphs), never by suppression. It is the value `6` `container` of the
profile table, has seventeen checks of its own and an RDF vocabulary; it adds no value to any enumeration of the core and uses no
frame type of its own.

It depends on these parts, cited by part and section title:

- `P2` (Registry): "Identity, labelling, geometry, severity and profile enumerations" (the profile table), "The finding table"
  and "The fields of a finding that are present" (the findings `profile_violation` and `profile_not_checked`);
- `P3` (RDF content): "The term model", "Typed lines", "The fold", "Typed references: the `bindings` frame", "Graph states",
  "Suppression hides", "The visible dataset and visibility", "Metadata is RDF";
- `P4` (Blobs): "The blob set";
- `P5` (Durability): "The checks of a commit", "The committed prefix";
- `P7` (Canonical form and reading): "The typed result of reading", "The reader procedure", "The fold of the whole file", "The
  closing phase of a segment, and visibility", "The canonical form".

External documents cited, by identifier, title and date:

- W3C XML Schema Definition Language (XSD) 1.1 Part 2: Datatypes; W3C Recommendation, 5 April 2012; for the names of the
  datatypes `string`, `hexBinary`, `boolean`, `nonNegativeInteger` and `dateTime`; no lexical space of them is judged;
- RDF 1.2 Concepts and Abstract Data Model (W3C Candidate Recommendation Draft, 15 November 2026), for IRIs, blank nodes and
  graph names;
- RFC 9110, HTTP Semantics, June 2022: section 8.8.3 (entity tag), section 15 (status codes);
- RFC 2119 and RFC 8174 (BCP 14), March 1997 and May 2017.

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`). A byte string is written in hexadecimal, two digits a byte, a space between bytes; `<S>` stands for the
`32` bytes of a digest that the example does not state. `rdf:` stands for `http://www.w3.org/1999/02/22-rdf-syntax-ns#`, `xsd:`
for
`http://www.w3.org/2001/XMLSchema#` and `c:` for `https://w3id.org/gts/pf/container#` in this text only; no rule reads a prefix,
and every IRI is compared whole. A
requirement has an identifier `PF-container-n` and states one thing.

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST and
MUST NOT, and no other.

## 1. The profile value and its checks

The value of this profile in the profile table of `P2` is `6` `container`. A profile is not a registry version: a header that
declares this profile has the `registry_version` that the core content of its segment needs and no more, and the profile never
raises it. A reader states the profiles it implements. A reader that does not implement this profile reads a segment that
declares it in full, runs none of its checks and reports the one information finding `profile_not_checked` with subject
`[4, 6, absent]` (`P2`, "The finding table").

The checks of this profile, numbered by this document from `1`:

| check | label |
|---|---|
| 1 | `container_suppress_frame` |
| 2 | `container_export_state` |
| 3 | `container_export_chain` |
| 4 | `container_export_properties` |
| 5 | `container_observation_properties` |
| 6 | `container_observation_duplicate` |
| 7 | `container_kind_content` |
| 8 | `container_graph_shared` |
| 9 | `container_graph_orphan` |
| 10 | `container_blank_shared` |
| 11 | `container_dangling_auxiliary` |
| 12 | `container_dangling_containment` |
| 13 | `container_outside_root` |
| 14 | `container_canonical_claim` |
| 15 | `container_observation_late` |
| 16 | `container_graph_late` |
| 17 | `container_blob_late` |

A failed check is the finding `profile_violation` with `reason` `absent` and `subject` `[4, 6, present(c)]`, `c` the number of
the check. The profile adds no value to `reason` or to any other enumeration of the core.

**PF-container-1.** A reader MUST report the checks of this profile by the numbers and labels of the table above; a check is never
renumbered,
and a later revision of this document appends a check with the next number.

**PF-container-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. The model

THE STATEMENTS JUDGED. The profile reads the visible dataset of the fold of the file (`P3`, "The visible dataset and
visibility"): its `quad` lines, called STATEMENTS, and its bindings. Archive metadata stands in the DEFAULT GRAPH; the content
of an RDF resource stands in a NAMED GRAPH of its own. A term `X` is an EXPORT when the default graph holds
`(X, rdf:type, c:Export)`, and an OBSERVATION when it holds `(X, rdf:type, c:Observation)`. An export is an IRI, because a later
export in another segment names it and a blank node is scoped to its segment (`P3`, "The term model"); an observation is an IRI
or a blank node.

AN EXPORT IS A GRAPH STATE. An exporter retrieves the resources under a root, writes what it found, and closes the export with a
`commit` frame. The export is the graph state that this commit closes (`P3`, "Graph states"): the fold of the frames of the run
of its segment up to and including the commit, named by the commit's effective identifier. The state is local to the segment of
the commit unless the checked lineage of a segment opened by a header joins earlier segments to it (a segment opened by a resync
item is joined to none), so an export whose content stands in an earlier segment needs that lineage. The commit closes an
export only when it names a state to the reader, which needs every ordinary frame before it
in its run to be linked or healed and folded or not content for that reader (`P3`); to a reader for which one is opaque the
export is unavailable. The export node `X` names the state by exactly one BINDING of `X` to a `state` referent (`P3`, "Typed
references: the `bindings` frame"). Because the identifier of the commit depends on every frame before it, the export node and
its binding are written after the commit, and the writer then commits again. THE STATE OF AN EXPORT, written `S(X)`, is the
state that the `state` referent of `X` names, when it resolves.

AN OBSERVATION is one retrieval of one resource in one export: what the server returned, when, and how the resource stood in the
tree. It is never rewritten and never suppressed; the observations are the history.

THE CONTENT OF AN OBSERVATION `O` of an export `X` (a term `O` with the visible statement `(O, c:export, X)`) is, in the visible
dataset of the file:

- (a) its STATEMENTS: every `quad` line of the default graph whose subject is `O`; every `reifier_binding` line of the default
  graph whose `triple` has the subject `O`; and every `annotation` line of the default graph whose reifier is the reifier of such
  a `reifier_binding` line;
- (b) its GRAPH, when `(O, c:graph, G)` is a statement: every line, of any kind, that carries the name `G` (`P3`, "Typed lines"),
  and the `empty_graph` line of `G`;
- (c) its BLOB, when a binding has the term `O` and a `blob` referent of name `N`: that binding.

The content of every observation of `X` lies in the fold of `S(X)`: this is what an export is, a graph state that holds the
resources it records. Later additions to an observation's content are therefore defects, and checks `15`, `16` and `17`
judge them.

THE EXPORT CHAIN. Exports are ordered by `c:previousExport`: the FIRST export has none, and each other export names the export
before it. The LATEST export is the one that no export names. The order survives compaction: it is a statement of the fold,
while the commits that name the states of earlier exports do not survive in a canonical file.

THE CURRENT STATE. When check `3` holds, the exports stand in CHAIN ORDER: the first export at place `0`, and each other export
one place after the export it names. The OBSERVATIONS OF `R`, for a resource IRI `R`, are the observations whose `c:resource` is
`R`. The current observation is DEFINED only when checks `3`, `4`, `5` and `6` all hold; then every observation has exactly one
`c:export`, an export of the chain, and exactly one `c:resource`, and no two observations of one resource stand in one export.
The CURRENT OBSERVATION of `R` is then the observation of `R` whose export has the greatest place in chain order, and `R` is LIVE
when the kind of that observation is not `c:Absent`. When check `3`, `4`, `5` or `6` fails, no resource has a current observation
and checks `11` to `13` are not judged.

THE LISTING ORDER. Wherever a reader or a tool lists them, the observations of one export stand in ascending bytewise order of
their `c:resource` IRI (`P1`, Notation), two observations of one resource (a file that fails check `6`) in ascending order of
the place of the observation terms (`P7`, THE PLACE OF A TERM); the objects of `c:contains` and of `c:managedPredicate` of one
observation stand in ascending bytewise order of the IRI; the exports stand in chain order.

REMOVAL AND CHANGE. A resource that disappears gets an observation of kind `c:Absent` (a tombstone) and no content. A resource
that changes gets a new observation whose `c:graph` names a NEW graph (an RDF resource) or whose binding names the new blob (a
non-RDF resource); the earlier graph and blob stay as history. A resource that comes back gets a new observation and a new
graph. Nothing is suppressed: suppression is monotone and would not let a resource deleted at one IRI come back at it.

## 3. The vocabulary

Every IRI of the vocabulary begins with `https://w3id.org/gts/pf/container#`; the IRIs are listed whole here, and the vocabulary
holds no other.

### 3.1 Classes and individuals

- `https://w3id.org/gts/pf/container#Export`: the class of exports.
- `https://w3id.org/gts/pf/container#Observation`: the class of observations.
- The KINDS of an observation, each an IRI: `https://w3id.org/gts/pf/container#RdfResource` (a resource whose representation is
  RDF statements), `https://w3id.org/gts/pf/container#NonRdfResource` (a resource whose representation is bytes),
  `https://w3id.org/gts/pf/container#ContainerResource` (a container, with its listing),
  `https://w3id.org/gts/pf/container#Unreadable` (a retrieval that failed), `https://w3id.org/gts/pf/container#Absent` (a
  resource that is gone: a tombstone).

### 3.2 Literal forms

- A TEXT LITERAL is a typed literal (`P3`, "The term model") of datatype `xsd:string`, the datatype compared whole.
- An OCTETS LITERAL of a byte string `b` is the typed literal that VALUES of the fold (`P3`, "The fold") holds with the typed
  value of kind `octets` whose bytes are `b`, for every byte string `b`, whether or not it is text. Its datatype is
  `xsd:hexBinary` and its lexical form is generated by `P3`, "Typed values": two upper-case hexadecimal digits for each byte.
  The octets of an octets literal are `b`. A text literal is not an octets literal, and neither is a literal of datatype
  `xsd:hexBinary` that VALUES does not hold. No test of the bytes chooses between two forms of a value: a property of this
  vocabulary that takes bytes from a server takes the octets literal of them, for every byte string, and the property alone
  decides that.
- A COUNT LITERAL is a typed literal that VALUES of the fold (`P3`, "The fold") holds with a typed value of kind `uint`.
- A TIME LITERAL is a typed literal that VALUES holds with a typed value of kind `time`.
- A TRUTH LITERAL is a typed literal that VALUES holds with a typed value of kind `bool`.

Each form is a fact of the presence and the kind of a pair of VALUES, or of the whole equality of a datatype with `xsd:string`;
no form is judged on a lexical form, and a count, a time or a truth value written as text with its datatype, with no pair of
VALUES, is of no form. Each form is one encoding for each value, because the lexical form of a typed value is generated (`P3`,
"Typed values"). An octets literal that the vocabulary takes from a server (a media type, an entity tag, a link relation type)
holds the value exactly as the server sent it, compared bytewise and never parsed: the format does not judge the inside of
these values. A consumer that reads such a literal receives its octets; the text that the literal generates is upper-case
hexadecimal and is not the value the server sent.

### 3.3 Properties of an export

- `https://w3id.org/gts/pf/container#previousExport`; object: an export (an IRI); at most one; absent exactly on the first
  export.
- `https://w3id.org/gts/pf/container#root`; object: an IRI, the root container the exporter started from; exactly one.
- `https://w3id.org/gts/pf/container#retrievedBy`; object: an IRI, the agent that made the export; exactly one.
- `https://w3id.org/gts/pf/container#exportedAt`; object: a time literal; exactly one.
- `https://w3id.org/gts/pf/container#complete`; object: a truth literal, `true` when the exporter followed every page of every
  listing it followed and retrieved every resource listed, `false` otherwise; exactly one.
- The binding of the export to a `state` referent; exactly one, and no other binding.

### 3.4 Properties of an observation

- `https://w3id.org/gts/pf/container#export`; object: an export (an IRI); exactly one.
- `https://w3id.org/gts/pf/container#resource`; object: an IRI, the resource as the server names it; exactly one.
- `https://w3id.org/gts/pf/container#retrievedIri`; object: an IRI, the IRI actually retrieved after redirects; at most one.
- `https://w3id.org/gts/pf/container#kind`; object: one of the five kind IRIs; exactly one.
- `https://w3id.org/gts/pf/container#retrievedAt`; object: a time literal; exactly one.
- `https://w3id.org/gts/pf/container#lastModified`; object: a time literal, the modification time of the resource as the writer
  states it; at most one.
- `https://w3id.org/gts/pf/container#mediaType`; object: an octets literal whose octets are not empty, the media type
  label as the server sent it; the format does not judge the inside of the value, which is compared whole and bytewise; at most
  one.
- `https://w3id.org/gts/pf/container#entityTag`; object: an octets literal whose octets are not empty, the entity tag as sent
  (RFC 9110 section 8.8.3), its quotes and weak marker included; at most one.
- `https://w3id.org/gts/pf/container#graph`; object: an IRI or a blank node, the name of the graph that holds this observed
  state of the resource's statements; at most one.
- `https://w3id.org/gts/pf/container#status`; object: a count literal, the status code the server returned (RFC 9110 section
  15); at most one.
- `https://w3id.org/gts/pf/container#auxiliaryFor`; object: an IRI, the subject resource of which this resource is an auxiliary
  resource; at most one.
- `https://w3id.org/gts/pf/container#relationType`; object: an octets literal whose octets are not empty, one link relation type,
  as the exporter
  states it; the format does not judge the inside of the value, which is compared whole and bytewise; exactly one when
  `auxiliaryFor` is present, none otherwise.
- `https://w3id.org/gts/pf/container#contains`; object: an IRI, a member the container listed; any number.
- `https://w3id.org/gts/pf/container#parent`; object: an IRI, the container the server named as this resource's parent; at most
  one.
- `https://w3id.org/gts/pf/container#managedPredicate`; object: an IRI, a predicate that the server manages in this resource's
  representation; any number.
- The binding of the observation to a `blob` referent: the bytes of a non-RDF resource; their length and media types are those
  of the blob set (`P4`, "The blob set"). No literal spells a digest or a size.

### 3.5 What each kind holds

| kind | `graph` | blob binding | `status` | `contains` | other |
|---|---|---|---|---|---|
| `RdfResource` | exactly one | none | none | none | |
| `NonRdfResource` | none | exactly one | none | none | `mediaType` present |
| `ContainerResource` | at most one | none | none | any number | |
| `Unreadable` | none | none | exactly one | none | |
| `Absent` | none | none | none | none | no `mediaType`, `entityTag`, `lastModified` |

An observation has no binding other than the one blob binding that its kind allows.

## 4. Writer rules

**PF-container-3.** A writer MUST write the observations of an export, the named graphs they name and the blob declarations and
bindings they hold, each with all of its content (section 2), before the `commit` frame that closes the export and in the segment
of that commit or in a segment of its run, and MUST write the export node and its one `state` binding after that commit, and then
a second `commit`. The sequence is: the observations in the listing order of section 2, each followed by its named graph and its
blob declaration and binding; the commit that closes the export; the statements of the export node and its one `state` binding;
the second commit.

**PF-container-12.** A writer that puts the content of an export in more than one segment MUST write the `lineage` of every
segment of the run after the first so that it is checked (`P1`, "The header"). Each segment of the run after the first is
opened by
a header; a segment opened by a resync item is never joined, so a writer that restarts by a resync item writes the content of an
export again in full in the new scope.

**PF-container-14.** A reader or tool that lists the exports, the observations of an export, or the objects of `c:contains` or
`c:managedPredicate` of an observation MUST list them in the listing order of section 2.

**PF-container-4.** A writer MUST write every observation with the properties of 3.4 in the numbers 3.4 states and the content
3.5 states for its kind, and every export with the properties of 3.3.

**PF-container-15.** A writer MUST write every time of the vocabulary as a typed value of kind `time`, every count as a typed
value of kind `uint`, every truth value as a typed value of kind `bool` and every media type, entity tag and link relation type as
the octets literal of its bytes, a typed value of kind `octets`, for every byte string whether or not it is valid UTF-8 (`P3`,
"Typed values").

**PF-container-5.** A writer MUST give every observed state of an RDF resource a graph name that no other observation names, and
MUST NOT let a blank node that occurs in a line of a resource graph occur anywhere outside that graph, with one exception: a
blank node that is the NAME of a resource graph occurs, beside the lines of that graph, as the object of the one default-graph
line `(O, c:graph, b)` that names it.

**PF-container-6.** A writer MUST NOT write a `suppress` frame in a file any segment of which declares this profile; it records
removal by an observation
of kind `c:Absent` and change by a new observation.

**PF-container-7.** A writer that appends an export MUST name the latest export of the file as its `c:previousExport`.

APPEND-ONLY. An archive is kept append-only: every export appends to the file, and nothing of an earlier export is rewritten. A
graph state is a property of the append history: compaction writes a new file whose commits are new, so the `state` referent of
every earlier export is unresolved in the canonical file of the fold (`missing_dependency` with `state`). A compacted copy is
therefore not an archive of this profile, and a segment that declares the profile makes no canonical claim (check `14`). The
canonical file of a fold otherwise has the same frames in the same order whatever profile its header declares (`P7`, "The
canonical form").

**PF-container-8.** A writer MUST NOT write `canonical` `present` in the header of a segment that declares this profile.

What a tool does is not this document's matter: restoring into a server, the access semantics of auxiliary resources and the
writing of managed predicates are the tool's behaviour and outside the specification.

## 5. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the vocabulary of section 3 and the conditions of section 6. It changes none of
these: the header grammar, the frame envelope and every content record, framing, identifiers, every hash and signature preimage,
the codec rules, the fold, the labelled set, the content digest, graph states and their names, suppression, the canonical form,
and the registry and the meaning of any of its values. No rule of the core reads the vocabulary (`P3` "Metadata is RDF").

**PF-container-9.** A reader or writer that implements this profile MUST give, for every input, the result that the parts it
depends on give, with the findings of this document added and nothing else changed; every rule of this
document is a check of section 6 or a numbered requirement on a writer or a reader.

## 6. Checks

THE REPORTING SEGMENT. The checks judge the file and the visible dataset of its fold (`P7`, "The fold of the whole file"), so
they run once for a file, after the closing phase of every segment of the file, and each is reported as a finding of the step
`PROFILE CHECKS` of `P7` ("The closing phase of a segment, and visibility", the step that runs the checks of a declared profile)
of the last segment whose header outcome is `readable` and whose header `profile` is `present(container)`. Each failing check is
ONE finding `profile_violation` with `reason` `absent`, `subject`
`[4, 6, present(c)]` (`c` the number of the check, section 1), `position`, `offset` and `length` absent, and `count` the number
of items its condition names; it stands in the findings of the reporting segment, in the order of `P7` ("Where each finding
stands, and in what order"). A
reader that does not implement the profile
reports `profile_not_checked` instead (section 1).

The conditions:

1. `container_suppress_frame`: the file holds an item that passes the envelope test with `frame_type` `suppress`, or an inner
   frame of that type opened from a sealed frame (`P8`, "The inner frame"). Counts such items and inner frames.
2. `container_export_state`: an export that has not exactly one visible binding, or whose binding is not a `state` referent.
   Counts exports.
3. `container_export_chain`: the exports do not form one chain: no export or more than one export lacks `c:previousExport`, an
   export is the `c:previousExport` of more than one export, or following `c:previousExport` from some export does not reach the
   first export. Counts the exports of the file, all of them, or `1` when there is no export: a set that is not one chain has
   no one first or latest export from which a part of it could be counted.
4. `container_export_properties`: an export that is not an IRI, or an export with one of the five properties of 3.3 in another
   number than 3.3 states, or with an object outside its range. The binding of an export is judged by check `2` alone. Counts
   exports.
5. `container_observation_properties`: an observation with a property of 3.4 in another number than 3.4 states (`relationType`
   included), or with an object outside its range, or a property of 3.3 or 3.4 on a term that is not of its class. Counts
   subjects.
6. `container_observation_duplicate`: two observations with equal `c:resource` and equal `c:export`. Counts observations.
7. `container_kind_content`: an observation with a valid kind whose `graph`, bindings, `status`, `contains`, `mediaType`,
   `entityTag` or `lastModified` differ from the row of its kind in 3.5. Counts observations.
8. `container_graph_shared`: a graph name that is the `c:graph` object of more than one observation. Counts observations.
9. `container_graph_orphan`: a named graph of the visible dataset (`P3`, "The visible dataset and visibility") that is the
   `c:graph` of no observation. Counts graphs.
10. `container_blank_shared`: a blank node `b` that occurs in a line of a graph `G` named by an observation (in any slot, at any
    depth of a triple term, the graph slot included) and also occurs in a line that is not of `G`, the default graph included,
    other than the one default-graph line `(O, c:graph, b)` by which an observation names `G` when `b` is the name of `G`.
    Counts blank nodes.
11. `container_dangling_auxiliary`: a current observation of a live resource whose `c:auxiliaryFor` names a resource that is not
    live. Counts observations.
12. `container_dangling_containment`: a current observation of a live resource whose `c:contains` names an IRI that is the
    resource of no observation in the current state. The check reads the LISTED members against the current observations of any
    kind: a member whose current observation is a tombstone (`c:Absent`) or `c:Unreadable` has an observation and is not
    dangling; whether a member is live is judged by check `11` only for `c:auxiliaryFor`. Counts observations.
13. `container_outside_root`: a current observation of a live resource that is not REACHABLE. The REACHABLE resources are the
    smallest set of resource IRIs that holds the object of every `c:root` statement of the latest export (none, when it has
    none) and is closed under this rule: a live resource is
    in the set when the current observation of a resource of the set lists it with `c:contains`, or its own current observation
    names a resource of the set with `c:parent` or with `c:auxiliaryFor`. So two resources that name each other, and are named
    by no resource of the set, are not reachable. Counts observations.
14. `container_canonical_claim`: a segment whose header declares this profile has `canonical` `present`. One finding for each
    such segment, in the findings of that segment, with `position`, `offset`, `length` and `count` absent; it is the one check
    not run in the reporting segment alone.
15. `container_observation_late`: an export `X` whose `state` referent resolves, and an observation `O` of `X` with a statement
    (section 2, (a)) that is not a line of the fold of `S(X)`. The statements `(O, rdf:type, c:Observation)` and
    `(O, c:export, X)` are among them. Counts observations.
16. `container_graph_late`: an export `X` whose `state` referent resolves, and an observation `O` of `X` with a statement
    `(O, c:graph, G)` such that a line that carries the name `G`, or the `empty_graph` line of `G`, is in the visible dataset and
    is not a line of the fold of `S(X)`. Counts observations.
17. `container_blob_late`: an export `X` whose `state` referent resolves, and an observation `O` of `X` with a visible binding of
    the term `O` to a `blob` referent of name `N` such that: the binding is not a binding of the fold of `S(X)`; or the blob set
    of the fold of `S(X)` holds no entry of the name `N`; or the entry of `N` in the fold of `S(X)` differs in any field from the
    entry of `N` in the fold of the file; or that entry is `present` and the blob `N` is INCOMPLETE in the fold of `S(X)` (`P4`,
    "Rebuilding a blob or a value": no declaration of it is verified from the units of the state and none is refused). A blob
    that is UNCHECKED there (none verified and one refused by the total rebuilt length of the read) is not judged by this
    condition: the reader did not finish the rebuild, and its finding `exceeds_capacity` with `total_rebuilt` says so. A blob
    whose entry is `external` is judged by its declaration alone: its bytes are not in the file and are not needed. Counts
    observations.

Checks 11 to 13 need the current state, so they are not judged when check `3`, `5` or `6` fails (section 2, THE CURRENT STATE);
checks 15 to 17 judge an
export only when its `state` referent resolves, and an export whose `state` referent does not resolve gives the core's
`missing_dependency` and none of them: with `state` when no commit of the file has that identifier, and with
`state_unavailable` when every commit that has it names no state to this reader. Checks 15 to 17 are then not judged, which is
not a pass: the reason `state_unavailable` says that the state was not available to this reader, and the fold states of the
frames before the commit say why (an opaque frame, a sealed frame not opened, a capacity).
In checks 15 to 17, "a line of the fold of `S(X)`" means any line of that fold, hidden or visible: hidden content is content,
and these checks do not use the visible dataset of the state.

**PF-container-10.** A reader that implements this profile MUST run those of the checks `1` to `13` and `15` to `17` that are at
or below the number it states in `profiles` (`P2`, "Allocation, the record, withdrawal and retirement") once for the file, after
the closing phase of every segment, and MUST report each of them that fails as one `profile_violation` with `subject`
`[4, 6, present(c)]` and the `count` its condition names, in the findings of the reporting segment; and MUST run check `14`, when
that number reaches it, for every segment that declares the profile and report each failure in the findings of that segment.

**PF-container-13.** A reader MUST judge checks `15`, `16` and `17` for an export only when its `state` referent resolves, and
MUST judge them against the fold of the state that the referent names, comparing lines and bindings as typed values with the
blank nodes of the state taken as the nodes of the same pair of segment ordinal and label in the fold of the file.

**PF-container-11.** A reader MUST compare every IRI and literal of the vocabulary as a typed value (`P3`, "The term model"),
MUST judge the forms of 3.2 by the pairs of VALUES of the fold and the whole equality of a datatype, MUST NOT choose a form of a
value by a test of its bytes, and MUST NOT parse a media
type, an entity tag, a link relation type, a status code, an IRI or any lexical form.

Overlaps. A failure of check `3`, `5` or `6` withholds checks `11` to `13`. Each check reports its own finding. A `suppress` frame
gives `container_suppress_frame`, and its unmatched entries
give `suppression_unmatched` from the core. An export bound to a commit that the file does not hold gives `missing_dependency`
with `state` from the core, one bound to a commit that names no state to the reader gives it with `state_unavailable`, and
neither gives `container_export_state`. An observation with two kinds gives
`container_observation_properties` and is not judged by `container_kind_content`. An observation of kind `RdfResource` with no
`c:graph` whose resource graph exists gives `container_kind_content` and `container_graph_orphan`. An observation whose properties
and graph lines stand after the commit that closes its export gives `container_observation_late` and `container_graph_late`, each
once for the observation.

## 7. Worked example

The first export of a server with root container `<https://pod.example/>`, which lists one RDF document
`<https://pod.example/d>`. Statements of the default graph (written as triples of typed terms; `_:` marks blank nodes of the
segment):

```text
_:o1 rdf:type c:Observation .  _:o1 c:export <https://agent.example/x/1> .  _:o1 c:resource <https://pod.example/> .
_:o1 c:kind c:ContainerResource .  _:o1 c:contains <https://pod.example/d> .
_:o1 c:retrievedAt "2026-11-15T10:00:00Z"^^xsd:dateTime .
_:o2 rdf:type c:Observation .  _:o2 c:export <https://agent.example/x/1> .  _:o2 c:resource <https://pod.example/d> .
_:o2 c:kind c:RdfResource .  _:o2 c:graph _:g1 .  _:o2 c:parent <https://pod.example/> .
_:o2 c:retrievedAt "2026-11-15T10:00:01Z"^^xsd:dateTime .  _:o2 c:mediaType "746578742F747572746C65"^^xsd:hexBinary .
```

and the statements of the document in the graph `_:g1`. Then the commit `C1`, whose effective identifier is `S`, closes the
export. After it the writer writes the export node:

```text
<https://agent.example/x/1> rdf:type c:Export .  <https://agent.example/x/1> c:root <https://pod.example/> .
<https://agent.example/x/1> c:retrievedBy <https://agent.example/a> .
<https://agent.example/x/1> c:complete "true"^^xsd:boolean .
<https://agent.example/x/1> c:exportedAt "2026-11-15T10:00:02Z"^^xsd:dateTime .
```

Every time literal of the example is written as a typed value of kind `time` (`2026-11-15T10:00:00Z` is `[4, 1794736800, 0]`),
`"true"^^xsd:boolean` as `[1, true]`, and the media type, the bytes `74 65 78 74 2f 74 75 72 74 6c 65` (`text/turtle`), as the
typed value `[5, h'74 65 78 74 2f 74 75 72 74 6c 65']`, whose generated literal is `"746578742F747572746C65"^^xsd:hexBinary`
(shown above in that form), so that VALUES of the fold holds each (3.2).

The writer then writes the binding of the export (term id `9`) to the state `S`, `[[1, 9], [3, S]]`, and a second commit. The
binding's bytes:

```text
82                  binding: array of 2
  82 01 09          leaf [1, 9]
  82 03             referent: array of 2, tag 3 (state)
    58 20 <S>       digest of 32 bytes
```

Check. `1 + 3 + 2 + 34 = 40` bytes. Counted again: array head one byte, leaf three, referent head and tag two, digest head two
and thirty-two bytes.

A second export `<https://agent.example/x/2>` with `c:previousExport <https://agent.example/x/1>` records that
`<https://pod.example/d>` is gone: an observation of kind `c:Absent` and no graph. The current observation of `d` is then that
tombstone; `d` is not live, and its graph `_:g1` stays as history. If instead the second export also held an observation of kind
`c:Unreadable`, with its `c:status`, of a new resource that named `<https://pod.example/d>` with `c:auxiliaryFor` (and its
`c:relationType`) and the root with `c:parent <https://pod.example/>` (so that it is reachable and check `13` holds), that
resource would be dangling, and the finding is the typed value
`[profile_violation, absent, present([4, 6, present(11)]), absent, absent, absent, present(1)]` (check `11`
`container_dangling_auxiliary`). Its bytes, from the finding record and the subject form `[4, profile, check: option(uint)]` of
`P2`: `87 18 2e 81 01 82 02 83 04 06 82 02 0b 81 01 81 01 81 01 82 02 01` (22 bytes). Check, once by fields: `87` (array of
seven), code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the check (eight
bytes for a profile and a check below `24`), then `position`, `offset` `81 01`, `length` `81 01` and `count`, each `81 01` when
absent or `82 02` and the value; once by counting the bytes listed.
