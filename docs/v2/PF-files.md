<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Profile PF-files: File tree

Title of this document, as the profile table of `P2` names it: `Profile PF-files: File tree`.

## 0. Scope and dependencies

This profile defines a FILE TREE carried as RDF statements and blobs: one entry for each file, directory, link or special file;
a path that is a list of components, never a string that is split; the bytes of a regular file reached by a binding of the core;
and the kinds and metadata that a portable archive of a POSIX tree records (permissions, times, owners, device numbers, link
targets, extended attributes and the extended header records of the POSIX archive format). It is the value `5` `files` of the
profile table, has eleven checks of its own and an RDF vocabulary; it adds no value to any enumeration of the core and uses no
frame type of its own.

It depends on these parts, cited by part and section title:

- `P1` (Model and wire): "The typed model and its one serialization";
- `P2` (Registry): "Identity, labelling, geometry, severity and profile enumerations" (the profile table), "The finding table"
  and "The fields of a finding that are present" (the findings `profile_violation` and `profile_not_checked`), "Allocation, the
  record, withdrawal and retirement";
- `P3` (RDF content): "The term model", "Typed lines", "Typed values", "The fold", "Typed references: the `bindings` frame",
  "Suppression hides", "The hiding rule", "The visible dataset and
  visibility", "Metadata is RDF";
- `P4` (Blobs): "The blob set";
- `P7` (Canonical form and reading): "The typed result of reading", "The reader procedure", "The fold of the whole file", "The
  closing phase of a segment, and visibility", "The canonical form", "Where each finding stands, and in what order".

External documents cited, by identifier, title and date:

- W3C XML Schema Definition Language (XSD) 1.1 Part 2: Datatypes; W3C Recommendation, 5 April 2012; for the names of the
  datatypes `string`, `hexBinary`, `nonNegativeInteger` and `dateTime`; no lexical space of them is judged;
- RDF 1.2 Schema (W3C Working Draft, 12 November 2026), "RDF Collections", for the collection vocabulary `rdf:first`,
  `rdf:rest`, `rdf:nil`;
- RDF 1.2 Concepts and Abstract Data Model (W3C Candidate Recommendation Draft, 15 November 2026), for the names of the term
  kinds that the ranges of section 2.4 name; it defines no rule of this profile;
- RFC 2119 and RFC 8174 (BCP 14), March 1997 and May 2017.

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`, "The typed model and its one serialization"). A byte string is written in hexadecimal, two digits a byte, a
space between bytes. `rdf:` stands for
`http://www.w3.org/1999/02/22-rdf-syntax-ns#` and `xsd:` for `http://www.w3.org/2001/XMLSchema#` in this text only; no rule
reads a prefix, and every IRI is compared whole. `f:` stands for `https://w3id.org/gts/pf/files#` in the same way. A requirement
has an identifier `PF-files-n`
.

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST and
MUST NOT, and no other.

## 1. The profile value and its checks

The value of this profile in the profile table of `P2` is `5` `files`. A profile is not a registry version: a header that
declares this profile has the `registry_version` that the core content of its segment needs and no more, and the profile never
raises it. A reader states the profiles it implements. A reader that does not implement this profile reads a segment that
declares it in full, runs none of its checks and reports the one information finding `profile_not_checked` with subject
`[4, 5, absent]` (`P2`, "The finding table").

The checks of this profile, numbered by this document from `1`:

| check | label |
|---|---|
| 1 | `files_entry_path` |
| 2 | `files_component` |
| 3 | `files_duplicate_path` |
| 4 | `files_missing_parent` |
| 5 | `files_kind` |
| 6 | `files_content_binding` |
| 7 | `files_property_cardinality` |
| 8 | `files_property_domain` |
| 9 | `files_value` |
| 10 | `files_hard_link` |
| 11 | `files_record` |

A failed check is the finding `profile_violation` with `reason` `absent` and `subject` `[4, 5, present(c)]`, `c` the number of
the check. The profile adds no value to `reason` or to any other enumeration of the core.

**PF-files-1.** A reader MUST report the checks of this profile by the numbers and labels of the table above; a check is never
renumbered, and a
later revision of this document appends a check with the next number.

**PF-files-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. The vocabulary

Every IRI of the vocabulary begins with `https://w3id.org/gts/pf/files#`; the IRIs are listed whole in this section, and the
vocabulary holds no other.

THE STATEMENTS JUDGED. The profile reads only `quad` lines of the DEFAULT GRAPH of the VISIBLE DATASET of the fold of the file
(`P3`, "The visible dataset and visibility"), called STATEMENTS below, and the visible bindings. A statement in a named graph, a
hidden statement and an `annotation` or `reifier_binding` line are not read. An ENTRY is a term `T` for which the statement
`(T, rdf:type, f:Entry)` exists; `T` is an IRI or a blank node. An entry that a statement of another segment names (a hard link
appended later) is an IRI, because a blank node is scoped to its segment (`P3`, "The term model").

### 2.1 Classes and individuals

- `https://w3id.org/gts/pf/files#Entry`: the class of entries; one entry for each file, directory, link or special file.
- The KINDS, each an IRI of the vocabulary: `https://w3id.org/gts/pf/files#RegularFile`,
  `https://w3id.org/gts/pf/files#Directory`, `https://w3id.org/gts/pf/files#SymbolicLink`,
  `https://w3id.org/gts/pf/files#HardLink`, `https://w3id.org/gts/pf/files#Fifo`,
  `https://w3id.org/gts/pf/files#CharacterDevice`, `https://w3id.org/gts/pf/files#BlockDevice`,
  `https://w3id.org/gts/pf/files#Socket`.
- The PERMISSIONS, each an IRI of the vocabulary, one for each permission of a POSIX file mode:
  `https://w3id.org/gts/pf/files#OwnerRead`, `https://w3id.org/gts/pf/files#OwnerWrite`,
  `https://w3id.org/gts/pf/files#OwnerExecute`, `https://w3id.org/gts/pf/files#GroupRead`,
  `https://w3id.org/gts/pf/files#GroupWrite`, `https://w3id.org/gts/pf/files#GroupExecute`,
  `https://w3id.org/gts/pf/files#OtherRead`, `https://w3id.org/gts/pf/files#OtherWrite`,
  `https://w3id.org/gts/pf/files#OtherExecute`, `https://w3id.org/gts/pf/files#SetUserId`,
  `https://w3id.org/gts/pf/files#SetGroupId`, `https://w3id.org/gts/pf/files#Sticky`.

The mode of an entry is the SET of its `f:permission` statements: an entry with none has a mode with no permission. No number
holds a mode and no bit of a number carries a permission.

### 2.2 Literal forms

- A TEXT LITERAL is a typed literal (`P3`, "The term model") of datatype `xsd:string`, the datatype compared whole.
- An OCTETS LITERAL of a byte string `b` is the typed literal that VALUES of the fold (`P3`, "The fold") holds with the typed
  value of kind `octets` whose bytes are `b`, for every byte string `b`, whether or not it is text. Its datatype is
  `xsd:hexBinary` and its lexical form is generated by `P3`, "Typed values": two upper-case hexadecimal digits for each byte,
  so that the octets `ff 41` give `FF41`. The octets of an octets literal are `b`. A text literal is not an octets literal, and
  neither is a literal of datatype `xsd:hexBinary` that VALUES does not hold. No test of the bytes chooses between two forms of a
  value: a property of this vocabulary that takes a byte string takes the octets literal of it, for every byte string, and the
  property alone decides that.
- A COUNT LITERAL is a typed literal that VALUES holds with a typed value of kind `uint`.
- A TIME LITERAL is a typed literal that VALUES holds with a typed value of kind `time`.

Each form is a fact of the presence and the kind of a pair of VALUES, or of the whole equality of a datatype with `xsd:string`;
no form is judged on a lexical form. A count or a time written as text with its datatype, with no pair of VALUES, is of no form.
The forms are one encoding for each value: a byte string has one octets literal, a count one count literal and an instant one
time literal, because the lexical form of a typed value is generated (`P3`, "Typed values"). A consumer that reads an octets
literal receives its octets; the text that the literal generates is upper-case hexadecimal and is not the bytes of a name.

### 2.3 Paths

A PATH is an RDF collection: the object of `f:path` is the head of a WELL-FORMED LIST. A list whose head is `h` is well formed
when

1. `h` is a blank node, and has exactly one statement with predicate `rdf:first` and exactly one with predicate `rdf:rest`;
2. the object of its `rdf:rest` is `rdf:nil` or a node that meets 1 and 2 in turn, and no node occurs twice in the chain;
3. every node of the chain is the object of exactly one statement: the head of the `f:path` statement, each other node of the
   `rdf:rest` statement of the node before it.

The MEMBERS of the list are the objects of the `rdf:first` statements of its nodes, in chain order; a well-formed path has at
least one member. Each member is a COMPONENT: the name of one directory level, or of the entry itself for the last member.

A component is VALID when it is an octets literal whose octets are not empty, are not `2e` (one dot) and are not `2e 2e` (two
dots). These are the names that name the directory itself or its parent, or nothing. Nothing else of a name is judged: the format
does not judge the inside of a name, which is one value compared whole and bytewise, a name is never split, and no separator
exists
in a path. Which bytes a name can hold on a system that extracts the tree is that system's matter.

Two paths are EQUAL when they have the same number of members and their members are equal terms in order (`P3`, "The term
model"); since the text of an octets literal is generated from its octets, this is equality of the octets. A path `p` is a PROPER
PREFIX of a path `q` when
`p` has fewer members than `q` and they are equal member by member.

THE ENTRY ORDER. The entries that have a VALID PATH (a well-formed list, every component valid) are ordered by their paths:
compare the members in turn by their octets, bytewise, the bytes as unsigned integers from the first byte, a shorter octet
string that is a prefix of a longer one first; a path that is a proper prefix of another comes first. Every directory therefore
precedes what it holds. Entries with equal paths (check `3` reports them) are ordered among themselves by the place of the entry
term (`P7`, THE PLACE OF A TERM), ascending. An entry without a valid path has no
place in this order and is not listed. The order is a function of the paths and of the terms; no field or statement stores an
order, so no check judges one, and every listing of entries is in this order (PF-files-12).

THE ORDER OF THE SETS OF AN ENTRY. A listing that gives the parts of an entry that the vocabulary holds as sets gives them in
one order each. The permissions of an entry are listed in the order in which 2.1 lists them (`OwnerRead` first, `Sticky` last).
The attribute records are listed in ascending bytewise order of the octets of their `f:attributeName`, and the archive records
in ascending bytewise order of the UTF-8 of their `f:recordKey`, the bytes as unsigned integers from the first byte, a shorter
string that is a prefix of a longer one first; two records of one entry with equal names or keys (check `11` reports them) are
ordered by the place of the record term (`P7`, THE PLACE OF A TERM), ascending. A record that has no name that is an octets
literal, or no key that is a text literal,
stands before every record that has one, and a record with several is ordered by the smallest of them;
ties are broken by the place of the record term.

### 2.4 Properties

For each property: its full IRI, its subject (the domain), its object (the range, with the term kinds of the term model that it
allows), and
how many statements of it one subject has. "Functional" means at most one.

- `https://w3id.org/gts/pf/files#path`; subject: an entry; object: a blank node, the head of a well-formed list; exactly one.
- `https://w3id.org/gts/pf/files#kind`; subject: an entry; object: one of the eight kind IRIs; exactly one.
- `https://w3id.org/gts/pf/files#permission`; subject: an entry; object: one of the twelve permission IRIs; any number.
- `https://w3id.org/gts/pf/files#modified`; subject: an entry; object: a time literal, the last modification; functional.
- `https://w3id.org/gts/pf/files#ownerId`; subject: an entry; object: a count literal, the numeric user id; functional.
- `https://w3id.org/gts/pf/files#groupId`; subject: an entry; object: a count literal, the numeric group id; functional.
- `https://w3id.org/gts/pf/files#ownerName`; subject: an entry; object: an octets literal, not empty; functional.
- `https://w3id.org/gts/pf/files#groupName`; subject: an entry; object: an octets literal, not empty; functional.
- `https://w3id.org/gts/pf/files#deviceMajor`; subject: an entry of kind `CharacterDevice` or `BlockDevice`; object: a count
  literal; exactly one on such an entry.
- `https://w3id.org/gts/pf/files#deviceMinor`; subject and count as `deviceMajor`; object: a count literal.
- `https://w3id.org/gts/pf/files#linkTarget`; subject: an entry of kind `SymbolicLink`; object: an octets literal, not empty, the
  link's content exactly as the link holds it; exactly one on such an entry. It is carried and never split or resolved.
- `https://w3id.org/gts/pf/files#hardLinkOf`; subject: an entry of kind `HardLink`; object: an IRI or a blank node that is an
  entry of kind `RegularFile`; exactly one on such an entry.
- `https://w3id.org/gts/pf/files#extendedAttribute`; subject: an entry; object: an IRI or a blank node, an ATTRIBUTE RECORD; any
  number.
- `https://w3id.org/gts/pf/files#attributeName`; subject: an attribute record; object: an octets literal, not empty; exactly one.
- `https://w3id.org/gts/pf/files#attributeValue`; subject: an attribute record; object: an octets literal; exactly one.
- `https://w3id.org/gts/pf/files#archiveRecord`; subject: an entry; object: an IRI or a blank node, an ARCHIVE RECORD, one
  extended header record of the POSIX archive format; any number.
- `https://w3id.org/gts/pf/files#recordKey`; subject: an archive record; object: a text literal, not empty; exactly one.
- `https://w3id.org/gts/pf/files#recordValue`; subject: an archive record; object: an octets literal; exactly one.

THE INSIDE OF A VALUE IS NOT JUDGED. The format does not judge the inside of the object of `f:linkTarget`, `f:attributeName`,
`f:attributeValue`, `f:recordKey`, `f:recordValue`, `f:ownerName` or `f:groupName`. Each is one value, compared whole and
bytewise: no path, separator, namespace prefix, keyword, number or time in it is read. A path that is a list of components is
`f:path` alone.

An attribute record is the object of exactly one `f:extendedAttribute` statement, and an archive record of exactly one
`f:archiveRecord` statement. The attribute records of one entry have pairwise different `f:attributeName` objects, and its
archive records pairwise different `f:recordKey` objects: a name or a key holds one value for an entry.

THE BYTES OF A REGULAR FILE. An entry of kind `RegularFile` has exactly one visible BINDING (`P3`, "Typed references: the
`bindings` frame") whose term value is the entry and whose referent is a `blob` referent; the named blob holds the file's bytes.
Its length and its media types are those of the blob set (`P4`, "The blob set"); the vocabulary states neither. An entry of any
other kind has no binding. No literal spells a digest.

An access control list is carried, where a source has one,
as the extended attribute or the archive record that holds it.

## 3. Writer rules

**PF-files-3.** A writer MUST write each entry with exactly one `f:path`, whose object is a well-formed list of valid
components, and exactly one `f:kind`.

**PF-files-4.** A writer MUST write a component, and every other octets value of the vocabulary, as the octets literal of its
bytes (2.2), a typed value of kind `octets`, for every byte string whether or not it is valid UTF-8.

**PF-files-14.** A writer MUST write every count of the vocabulary as a typed value of kind `uint` and every time as a typed value
of kind `time` (`P3`, "Typed values").

**PF-files-5.** A writer MUST write an entry of kind `Directory` for every proper prefix of the path of every entry, and MUST
NOT write two entries with equal paths.

**PF-files-6.** A writer MUST bind every entry of kind `RegularFile`, and only such an entry, to the blob of its bytes by
exactly one binding with a `blob` referent.

**PF-files-7.** A writer MUST write the properties of 2.4 only on the subjects, with the objects and in the numbers that 2.4
states.

**PF-files-12.** An implementation that lists the entries of a fold under this profile MUST list the entries that have a valid
path in the entry order of 2.3, and MUST NOT list an entry
without one.

**PF-files-13.** An implementation that lists the permissions, the attribute records or the archive records of an entry MUST
list them in the order of 2.3, THE ORDER OF THE SETS OF AN ENTRY.

Removal and change. An archive is the set of its visible entries. A later segment that removes an entry hides it by a `term`
suppression entry of the core (`P3`, "Suppression hides"), and one that changes an entry hides the old entry and writes a new
one; the history stays content. Hiding a term hides every line in which it occurs (`P3`, "The hiding rule"): the entry's own
statements, its binding, and every statement that names it as an object, the `f:hardLinkOf` statement of a hard link to it
included. The statements of its attribute and archive records and of its path list name the record and list nodes, not the
entry, and stay visible unless those nodes are hidden too. A hard link does not survive the hiding of its target: a hard link
without a visible target fails `files_property_cardinality`.

**PF-files-15.** A writer that removes or changes an entry MUST also hide its record nodes, MUST hide or rewrite every hard link
that names it, and MUST give an entry that replaces a hidden one a term that no suppression entry of the file hides.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form").

**PF-files-8.** A writer of the canonical file of a fold under this profile MUST write the canonical file of the fold with the
header's
`profile` `present(files)`: the profile changes no step of its construction.

Whether a tool that extracts a tree follows a symbolic link, creates a device,
sets an owner or writes outside a destination is the tool's behaviour and outside the specification; this document defines the
archive, the checks on it and the order of every listing of it (PF-files-12, PF-files-13).

A COMPONENT IS A NAME, NOT A PATH. A file that passes every check of this profile can hold a component whose octets hold the
byte `2f`, the byte `00`, or the bytes `2e 2e 2f`: the profile judges a component whole (2.3) and refuses only the empty name,
`2e` and `2e 2e`. A tool that writes a component into a path of its system by joining bytes therefore can name a place that the
tree does not hold, outside its destination included. The tree itself is unambiguous: its structure is the list of `f:path`, and
each member is one name of one level.

## 4. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the vocabulary of section 2 and the conditions of section 5. It changes none of
these: the header grammar, the frame envelope and every content record, framing, identifiers, every hash and signature preimage,
the codec rules, the fold, the labelled set, the content digest, suppression, the canonical form, and the registry and the
meaning of any of its values. No rule of the core reads the vocabulary (`P3` "Metadata is RDF"); only the checks of this profile
do.

**PF-files-9.** A reader or writer that implements this profile MUST give, for every input, the result that the parts it depends
on give, with the findings of this document added and nothing else changed; every rule of this
document is a check of section 5 or a numbered requirement on a writer, a reader or an implementation that lists.

## 5. Checks

THE REPORTING SEGMENT. The checks judge the visible dataset of the fold of the whole file (`P7`, "The fold of the whole file"),
so they run once for a file: once, after the closing phase of every segment of the file, and reported as a finding of the step
`PROFILE CHECKS` of `P7` "The closing phase of a segment, and visibility", the step that runs the checks of a declared profile,
of the last segment of the file whose header outcome is `readable` and whose header `profile` is `present(files)`. Each failing
check is ONE finding `profile_violation` with `reason` `absent`, `subject` `[4, 5, present(c)]` (`c` the number of the check,
section 1), `position`, `offset` and `length` absent, and `count` the number of subjects that fail it; it stands in the findings
of the reporting segment, in the order of `P7` "Where each finding stands, and in what order". A reader that does not implement
the profile reports `profile_not_checked` instead (section 1).

The conditions. Each names the subjects it counts. An entry has a VALID KIND when it passes check `5`. A check that needs a valid
path or a valid kind does not judge an entry that
lacks one; that entry is counted by `files_entry_path`, `files_component` or `files_kind`.

1. `files_entry_path`: an entry that has not exactly one `f:path` statement, or whose path object is not the head of a
   well-formed list (2.3). Counts entries.
2. `files_component`: an entry with a well-formed path one of whose members is not a valid component. Counts entries.
3. `files_duplicate_path`: an entry with a valid path (well formed, every component valid) that equals the path of another
   entry. Counts entries.
4. `files_missing_parent`: an entry with a valid path some proper prefix of which is the path of no entry of kind `Directory`.
   Counts entries.
5. `files_kind`: an entry that has not exactly one `f:kind` statement, or whose `f:kind` object is not one of the eight kind
   IRIs. Counts entries.
6. `files_content_binding`: an entry of kind `RegularFile` that has not exactly one visible binding, or whose binding's referent
   is not a `blob` referent; or an entry of another kind that has a visible binding. Counts entries.
7. `files_property_cardinality`: a subject, of the property's domain or not, with more than one statement of a functional
   property or of a property of which 2.4 allows exactly one, `f:path` and `f:kind` excepted (checks `1` and `5` judge their
   number); or an entry with a valid kind that lacks a property 2.4 requires of that kind; or an attribute record or
   an archive record that lacks its name or key or its value. Counts subjects.
8. `files_property_domain`: a statement whose predicate is a property of 2.4 and whose subject is not of the property's domain:
   an entry property on a term that is not an entry; `deviceMajor`, `deviceMinor`, `linkTarget` or `hardLinkOf` on an entry that
   has no valid kind or another kind; a record property on a term that is not a record of its kind; or a term that is the object
   of more than one `f:extendedAttribute` statement or of more than one `f:archiveRecord` statement. Counts subjects.
9. `files_value`: a statement whose predicate is a property of 2.4 and whose object is not of its range: a term of another kind,
   a literal of another datatype or form (2.2), an empty value where 2.4 forbids one, or an IRI outside the set the property
   names. For `f:path` the check judges the kind of the object alone, a
   blank node; whether the list is well formed is check `1`. For `f:hardLinkOf` it judges the kind of the object alone, an IRI
   or a blank node; whether that term is an entry of kind `RegularFile` is check `10`. Counts subjects.
10. `files_hard_link`: an entry of kind `HardLink` whose `f:hardLinkOf` object is not an entry of kind `RegularFile`. Counts
    entries.
11. `files_record`: an entry two of whose attribute records have equal `f:attributeName` objects, or two of whose archive
    records have equal `f:recordKey` objects. Counts entries.

**PF-files-10.** A reader that implements this profile MUST run the checks `1` to the number it states in `profiles` (`P2`,
"Allocation, the record, withdrawal and retirement") on the visible dataset of the fold of the file, and MUST report each check
that fails for at least one subject as one `profile_violation` with `subject`
`[4, 5, present(c)]` and the `count` its condition names, in the reporting segment, once for the file, after the closing phase of
every segment.

**PF-files-11.** A reader MUST judge a component, a path and every value by the typed value of its term (`P3`, "The term model")
and by the forms of 2.2, that is by the pairs of VALUES of the fold and the whole equality of a datatype, and MUST NOT read,
split, trim, normalise or resolve the lexical form of any literal, but for the length and the whole bytewise comparison of the
lexical form of a text literal.

Overlaps. Each check reports its own finding. An entry whose only path is a literal gives `files_entry_path` and `files_value`
(the range of `f:path`). An entry bound to a blob that the file does not declare gives `missing_dependency` with `blob` from the
core and no finding of this profile; a `RegularFile` bound twice gives `files_content_binding`. A `HardLink` whose target is not
an entry gives `files_hard_link`, and `files_value` as well when the target is a literal.

## 6. Worked example

A tree of one directory `docs` and one regular file `docs/a.txt` with mode `rw-r--r--`, as statements of the default graph
(written as triples of typed terms; `_:` marks blank nodes):

```text
_:d  rdf:type f:Entry .   _:d f:kind f:Directory .   _:d f:path _:l1 .
_:l1 rdf:first [5, h'64 6f 63 73'] .   _:l1 rdf:rest rdf:nil .
_:a  rdf:type f:Entry .   _:a f:kind f:RegularFile .  _:a f:path _:l2 .
_:l2 rdf:first [5, h'64 6f 63 73'] .   _:l2 rdf:rest _:l3 .
_:l3 rdf:first [5, h'61 2e 74 78 74'] .  _:l3 rdf:rest rdf:nil .
_:a  f:permission f:OwnerRead .  _:a f:permission f:OwnerWrite .
_:a  f:permission f:GroupRead .  _:a f:permission f:OtherRead .
_:a  f:modified "2026-06-10T20:00:00Z"^^xsd:dateTime .
```

with one binding of `_:a` to the blob `[blake3_256, D]` that holds the file's bytes. The
literal of `f:modified` is written as the typed value `[4, 1781121600, 0]`, which VALUES of the fold holds; its lexical form
`2026-06-10T20:00:00Z` is generated. Every component is an octets literal, a typed value
of kind `octets` (`[5, h'...']` above): `docs` is the bytes `64 6f 63 73` with the generated lexical form `646F6373`, and `a.txt`
is the bytes `61 2e 74 78 74` with `612E747874`. The entry order puts `_:d` (path `["docs"]`) before `_:a` (path `["docs",
"a.txt"]`).

If the directory entry `_:d` is removed, `_:a` fails check `4` `files_missing_parent`, and the finding is the typed value
`[profile_violation, absent, present([4, 5, present(4)]), absent, absent, absent, present(1)]`. Its bytes, from the finding
record and the subject form `[4, profile, check: option(uint)]` of `P2`:
`87 18 2e 81 01 82 02 83 04 05 82 02 04 81 01 81 01 81 01 82 02 01` (22 bytes). The fields: `87` (array of seven),
code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the check (eight bytes for
a profile and a check below `24`), then `position`, `offset`, `length` and `count`, each `81 01` when absent or `82 02` and the
value.

A file name of the two bytes `ff 41` is the component written as the typed value `[5, h'ff 41']`, whose generated literal is
`"FF41"^^xsd:hexBinary`; the name `A1` is the component `[5, h'41 31']`, whose generated literal is `"4131"^^xsd:hexBinary`. Both
are valid components, and the form of the second does not depend on its bytes being text. The text literal `"A1"` is not an
octets literal, and neither is the literal `"FF41"^^xsd:hexBinary` written as text, which VALUES does not hold: each fails
`files_component`.
