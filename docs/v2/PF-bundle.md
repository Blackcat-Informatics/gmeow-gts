<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Profile PF-bundle: Nested files

Title of this document, as the profile table of `P2` names it: `Profile PF-bundle: Nested files`.

## 0. Scope and dependencies

This profile defines a BUNDLE: a file some of whose blobs are themselves files of this generation. It gives one typed way to say
that a blob is a nested file, bounds the depth of nesting and the size of a nested file by constants, and states what a reader
reports about nested content. It is the value `4` `bundle` of the profile table, has five checks of its own, one class IRI, two
constants and one record; it adds no value to any enumeration of the core and uses no frame type of its own.

It depends on these parts, cited by part and section title:

- `P1` (Model and wire): "The header", "The generation";
- `P2` (Registry): "Identity, labelling, geometry, severity and profile enumerations" (the profile table), "The finding table"
  and "The fields of a finding that are present" (the findings `profile_violation` and `profile_not_checked`), "Allocation, the
  record, withdrawal and retirement";
- `P3` (RDF content): "Typed lines", "Typed references: the `bindings` frame", "The visible dataset and visibility", "Metadata
  is RDF";
- `P4` (Blobs): "The blob set", "Rebuilding a blob or a value";
- `P7` (Canonical form and reading): "The typed result of reading", "Where each finding stands, and in what order", "Capacity",
  "The reader procedure", "The fold of the whole file", "The
  closing phase of a segment, and visibility", "The canonical form".

External documents cited, by identifier, title and date:

- RDF 1.2 Concepts and Abstract Data Model (W3C Candidate Recommendation Draft, 15 November 2026), for the subjects of a
  statement;
- RFC 2119 and RFC 8174 (BCP 14), March 1997 and May 2017.

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`, "The typed model and its one serialization"). A byte string is written in hexadecimal, two digits a byte, a
space between bytes; `<D>` stands for the
`32` bytes of a digest that the example does not state. `rdf:type` is the IRI `http://www.w3.org/1999/02/22-rdf-syntax-ns#type`. A
requirement has an identifier `PF-bundle-n` .

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST and
MUST NOT, and no other.

## 1. The profile value and its checks

The value of this profile in the profile table of `P2` is `4` `bundle`. A profile is not a registry version: a header that
declares this profile has the `registry_version` that the core content of its segment needs and no more, and the profile never
raises it. A reader states the profiles it implements. A reader that does not implement this profile reads a segment that
declares it in full, runs none of its checks and reports the one information finding `profile_not_checked` with subject
`[4, 4, absent]` (`P2`, "The finding table").

The checks of this profile, numbered by this document from `1`:

| check | label |
|---|---|
| 1 | `bundle_binding` |
| 2 | `bundle_size` |
| 3 | `bundle_bytes_unavailable` |
| 4 | `bundle_not_a_file` |
| 5 | `bundle_depth` |

A failed check is the finding `profile_violation` with `reason` `absent` and `subject` `[4, 4, present(c)]`, `c` the number of
the check. The profile adds no value to `reason` or to any other enumeration of the core.

**PF-bundle-1.** A reader MUST report the checks of this profile by the numbers and labels of the table above; a check is never
renumbered, and
a later revision of this document appends a check with the next number.

**PF-bundle-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. How a blob is declared to be a nested file

THE CLASS. The vocabulary of this profile is one IRI, `https://w3id.org/gts/pf/bundle#NestedFile`, called `NestedFile` below. It
is a class: a term `T` is a NESTED-FILE TERM of a fold when the visible dataset of the fold (`P3`, "The visible dataset and
visibility") holds the `quad` line `(T, rdf:type, NestedFile)` in the default graph. `T` is an IRI or a blank node, the subjects
that RDF 1.2 Concepts and Abstract Data Model (W3C Candidate Recommendation Draft, 15 November 2026) admits. A statement in a
named graph declares nothing.

THE BLOB. The bytes of a nested file are a blob of the core, reached by a BINDING (`P3`, "Typed references: the `bindings`
frame"): the visible dataset holds exactly one binding whose term value is `T`, and its referent is a `blob` referent
`[1, name]`. The blob named `name` is the NESTED FILE of `T`.

 No media type is compared and no blob byte is inspected to find a nested file: the class statement says it.  Other statements
 about `T` (a name, a role, a media
type of the consumer's vocabulary) are the consumer's and are not read by this profile.

**PF-bundle-3.** A writer that declares a blob to be a nested file MUST write the statement `(T, rdf:type, NestedFile)` in the
default graph and exactly one binding of `T` to a `blob` referent naming that blob, and MUST write no other binding of `T`.

**PF-bundle-4.** A writer MUST declare a nested file only for a blob whose bytes it writes in the same file (a `blob` frame, a
`recipe` frame or a `pack` member that carries or lists them) and whose bytes are a file of generation `2`.

## 3. Constants

- DEPTH `Dmax`; value: `4`.
- SIZE `Smax`; value: `4294967296` bytes (`2^32`).

A different depth or size is a different profile, with a value of its own in the profile table, never a parameter.

## 4. What a reader reports about nested content

THE NESTED READ. For a nested file whose bytes the reader can rebuild, the reader reads those bytes by the procedure of `P7`
"The reader procedure", with the registry version, the `inputs`, the `marks` and the sealing input of the outer read, the access
`whole`, the capacity of THE SHARED ALLOWANCE (below), and with one input that only a nested read has: the DEPTH `d` of that file.
The findings of its checks `4` and `5` are those that the depths of THE TREE (below) give once every nested read of the outer read
has been made. A nested read is the read of one file: it runs the
checks of this profile on that file with depth `d` and reads no other file; which nested files are read, and at which depth, THE
ORDER OF THE NESTED READS (below) decides for the whole tree. The content of a nested read is not delivered. That read is
separate: its result is not part
of the outer `read_result`,
and the outer fold, the outer labelled set, the outer content digest, `integrity` and `authenticity` hold the nested file as a
blob and nothing else. A nested file's frames are never frames of the outer segment and its lines are never lines of the outer
fold.

THE TREE. The outer file has depth `0`. A nested file of a file `G` of depth `d` has depth `d + 1` when `G` is the outer file,
or when a segment of `G` declares `bundle` and the nested read of `G` judges it under this profile. A nested file that is not read
(bytes unavailable or above `Smax`), and one whose nested read fails the condition of check `4`, has no nested files: no nested
read is made for it and it adds no depth. A blob bound at several depths has the smallest of
them, and is read once, at that depth; that one read judges checks `4` and `5` for every nested-file term bound to the blob,
wherever the term stands. The TREE BELOW a file `G` holds the nested files of `G` and the tree below each of them, at whatever
depth each is read.

THE ORDER OF THE NESTED READS. Depths are assigned in ascending order: the nested files of depth `d + 1` are found by the reads
of depth `d`, and all the reads of one depth are made before any read of the next. Within one depth the nested files are read in
ascending bytewise order of `E(name)` of the blob name (the bytes compared as unsigned integers from the first byte, a proper
prefix before the longer string), each distinct blob once. A blob is read at the smallest depth at which any file that is read
binds it; a blob already read at a smaller depth, or earlier at the same depth in this order, is not read again. The reader
delivers beside the outer `read_result` the list `nested_results` of records `nested_result = [name: qualified_name, depth: uint,
result: read_result]`, one for each nested file that is read, in this order, `name` the blob name, `depth` the depth at which it
was read and `result` the result of its nested read; the list is empty when no nested file is read. The findings of the step
`PROFILE CHECKS` of a nested result are those that its checks give once every nested read of the outer read has been made.

THE SHARED ALLOWANCE. An outer read and all its nested reads draw on ONE allowance of each of the five totals of a read of `P7`
"Capacity": the items delimited (`items_per_read`), the total decoded length (`decoded_bytes_per_read`), the total rebuilt
length (`rebuilt_bytes_per_read`), the total formed length (`formed_bytes_per_read`) and the total labelled weight
(`labelled_bytes_per_read`). The outer read is made with the reader's capacity. Each nested read, in the ORDER OF THE NESTED
READS, is made with the reader's capacity except those five fields, each of which is the reader's value less what the outer
read and every nested read before it in that order have used: the items they delimited, the sums of their total decoded
lengths, of their total rebuilt lengths, of their total formed lengths and of their total labelled weights. The `capacity` of
the result of a nested read states the values it was made
with, so every refusal in it is explained by its own result (`P7`, "Capacity"). These values can lie below the minimum capacity
of a conforming reader: the minimums of `P7` bind the reader's own capacity, and a nested read uses what remains of it. The bytes
of a nested file are not charged again: they are the output of a rebuild that the outer read admitted, or the decoded payload of
a `blob` frame or member, which a total already counts.

A CAPACITY-LIMITED nested read is one whose result holds a finding `exceeds_capacity` of any reason. A term whose bound blob has
no verified declaration and some declaration with the rebuild state `refused` or `not_run` is judged by checks `1` and `2` only; a
term whose
nested read, or a nested read in the tree below it, is capacity-limited is judged by checks `1` to `3` only. Neither counts for a
later check.

THE BOUND ON THE WORK. The outer read and all its nested reads together delimit at most `items_per_read` items, decode at most
`decoded_bytes_per_read` bytes, rebuild at most `rebuilt_bytes_per_read` bytes, form at most `formed_bytes_per_read` bytes of
front-coded text and weigh at most `labelled_bytes_per_read` bytes of labelled lines, within the reader's capacity. In
addition, each
nested read takes at most `Smax` bytes of input, each distinct blob is read once, and no file of a depth above `Dmax` is read. Let
`E_1` be the number of distinct nested files of the outer file, and `E_(d+1)` the number of distinct nested files of the files
read at depth `d`; the nested reads of one outer read take at most `Smax * (E_1 + ... + E_Dmax)` bytes of input. The limited units
of the labellings are bounded for each fold and not for the read (`P7`, "Capacity"). The bound is a function of the reader's
capacity, the outer file and the two constants.

What the outer result carries. Exactly the findings of section 6, which judge the nested-file terms of the outer fold and the
depth of the tree below them. A finding of a nested read (a damaged frame inside a nested file, its own `profile_violation`) is
in the result of that read and is never copied into the outer result.

**PF-bundle-5.** A reader MUST NOT fold, label or digest the content of a nested file into the outer file, MUST NOT copy a finding
of a nested read into the outer result, and MUST NOT deliver the content of a nested read.

**PF-bundle-6.** A reader MUST read each distinct nested file (by blob name) at most once for one outer read, at the smallest
depth at which it occurs, MUST count every depth from the outer file, and MUST NOT read a nested file of depth above `Dmax`.

**PF-bundle-12.** A reader MUST make the nested reads by ascending depth, all reads of one depth before any of the next, and
within one depth in ascending bytewise order of `E(name)` of the blob name, and MUST deliver `nested_results` beside the outer
`read_result`, with one `nested_result` for each nested file that it read, in that order, and MUST give, among the findings of the
step `PROFILE CHECKS` of each nested result, as the findings of the checks of this profile those that they give for the terms of
that file once every nested read of the outer read has been made.

**PF-bundle-14.** A reader MUST make each nested read with the reader's capacity except `items_per_read`,
`decoded_bytes_per_read`,
`rebuilt_bytes_per_read`, `formed_bytes_per_read` and `labelled_bytes_per_read`, each of which MUST be the reader's value
less what the outer read and the nested reads before it, in the order of the nested reads, have used, and MUST state those
values in the `capacity` of the nested result.

**PF-bundle-15.** A reader MUST NOT judge check `3` for a term whose bound blob has no verified declaration and some refused
one, and MUST NOT judge checks `4` and `5` for a term whose nested read, or a nested read in the tree below it, is
capacity-limited.

## 5. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the class of section 2, the constants of section 3, the record `nested_result` of
section 4 and the conditions of section 6.

It changes none of these: the header grammar, the frame envelope and every content record, the grammar of the `bindings` frame
and its referent kinds, framing, identifiers, every hash and signature preimage, the codec rules, the fold, the labelled set,
the content digest, the canonical form, and the registry and the meaning of any of its values.

**PF-bundle-7.** A reader or writer that implements this profile MUST give, for every input, the result that the parts it depends
on give, with the findings of this document added, the list `nested_results` of section 4 delivered beside the `read_result`, and
the `capacity` of a nested result as section 4 states it, whether or not it lies below the minimums of `P7`, and nothing else
changed; every rule of this
document is a check of section 6 or a requirement on a writer or a reader.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form"). The
canonical status of a nested file is its own.

**PF-bundle-8.** A writer of the canonical file of a fold under this profile MUST write the canonical file of the fold with the
header's
`profile` `present(bundle)`: the profile changes no step of its construction.

## 6. Checks

THE REPORTING SEGMENT. The checks judge the fold of the whole file (`P7`, "The fold of the whole file"), so they run once for a
file: once, after the closing phase of every segment of the file, and reported as a finding of the step `PROFILE CHECKS` of `P7`
"The closing phase of a segment, and visibility", the step that runs the checks of a declared profile, of the last segment of the
file whose header outcome is `readable` and whose header `profile` is `present(bundle)`. Each failing check is one finding
`profile_violation` with `reason` `absent`, `subject` `[4, 4, present(c)]` (`c` the number of the check, section 1), `position`,
`offset` and `length` absent, and `count` the number of nested-file terms that fail it; it stands in the findings of the
reporting segment, in the order of `P7` "Where each finding stands, and in what order". A reader that does not implement the
profile reports `profile_not_checked` instead (section 1).

Each nested-file term `T` of the outer fold is judged by the checks below in the order of the checks; the first that fails is the
one finding for `T`, and the
later ones do not judge it. Each term is judged alone and a finding counts terms. The checks:

1. `bundle_binding`: the visible dataset does not hold exactly one binding of `T`, or that binding's referent is not a `blob`
   referent.
2. `bundle_size`: the blob set of the fold holds an entry of the bound name whose `length` is above `Smax`.
3. `bundle_bytes_unavailable`: the blob set of the fold holds no entry of the bound name with `presence` `present`, or the
   blob's state is INCOMPLETE (`P4`, "Rebuilding a blob or a value": no declaration is verified and none is refused or not run). A
   blob
   whose state is UNCHECKED is not judged by this check (section 4). The check reads the blob set, in which a hidden declaration
   stays: hiding the declaration of the bound blob does not fail it.
4. `bundle_not_a_file`: the nested read of its bytes gives a `file_result` that is not of tag `second_generation`, or one with
   no segment, or one whose file findings hold `torn_tail`, `undelimitable_input` or `read_failed`, or one with a segment whose
   `header_identifier` is `absent`. The check is not judged for a nested-file term in a file of depth `Dmax` whose blob has no
   depth of at most `Dmax`, for which no nested read is made: check 5 judges that term (PF-bundle-10).
5. `bundle_depth`: the nested file of `T` has a depth above `Dmax`, or the tree below it holds a nested file that has.

**PF-bundle-9.** A reader that implements this profile MUST run the checks `1` to the number it states in `profiles` (`P2`,
"Allocation, the record, withdrawal and retirement") on every nested-file term of the outer fold, in
the order of section 6, and MUST report each check that fails for at least one term as one `profile_violation` with `subject`
`[4, 4, present(c)]` and `count` the number of terms for which it is the first failing check.

**PF-bundle-10.** A reader MUST decide `bundle_depth` by reading nested files to depth `Dmax` and no further: a nested-file term
in a file of depth `Dmax` that is judged under this profile is, when it passes check `1`, a nested file of depth `Dmax + 1`,
unless
its binding names a blob that has a depth of at most `Dmax` by the smallest-depth rule of section 4; for a term that is a nested
file of depth `Dmax + 1` the reader counts the term and reads nothing.

**PF-bundle-11.** A reader MUST judge each nested-file term of the outer fold by the checks in the order of the checks.

**PF-bundle-13.** A reader MUST run the checks of this profile once for a file, after the closing phase of every segment, and
report each failing check as a finding of the step `PROFILE CHECKS` in the findings of the last segment of the file whose header
outcome is `readable` and whose header `profile` is `present(bundle)`, the findings of that list standing in the order of `P7`
"Where each finding stands, and in what order".

Overlaps. Each check reports its own finding for the terms for which it is the first to fail. A bound blob that no declaration
names gives `missing_dependency` with `blob` and
`bundle_bytes_unavailable`; a missing unit gives `missing_dependency` with `chunk` and `bundle_bytes_unavailable`; a binding of
`T` to a `frame` or `state` referent gives `bundle_binding`, and `missing_dependency` when that referent does not resolve.

## 7. Worked example

A segment declares one nested file. Its dictionary holds the blank term `_:n` with term id `3`; its `quads` frame holds the row
`(_:n, rdf:type, NestedFile)` in the default graph; its `bindings` frame holds the binding `[[1, 3], [1, [1, D]]]`: the leaf of
tag `1` naming term id `3`, and the `blob` referent naming the blob `[blake3_256, D]`. The binding's bytes:

```text
82                      binding: array of 2
  82 01 03              leaf [1, 3]
  82 01                 referent: array of 2, tag 1 (blob)
    82 01 58 20 <D>     qualified_name [1, D]
```

Check. `1 + 3 + 2 + 36 = 42` bytes; the qualified name is `1 + 1 + 2 + 32 = 36` bytes. Counted again from the values: the
binding record `82` (one byte), the leaf `82 01 03` (three), the referent head and tag `82 01` (two), the name `82 01`, `58 20`
and the digest (thirty-six).

The blob `D` is present and holds a bundle file of depth `1` that declares a nested file of its own, which declares another,
down to a nested file of depth `5`. The outer reader reads to depth `4`, finds a nested-file term at depth `4`, and reports one
finding, the typed value `[profile_violation, absent, present([4, 4, present(5)]), absent, absent, absent, present(1)]` (check
`5` `bundle_depth`). Its bytes, from the finding record and the subject form `[4, profile, check: option(uint)]` of `P2`:
`87 18 2e 81 01 82 02 83 04 04 82 02 05 81 01 81 01 81 01 82 02 01` (22 bytes). The fields: `87` (array of seven),
code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the check (eight bytes for
a profile and a check below `24`), then `position`, `offset`, `length` and `count`, each `81 01` when absent or `82 02` and the
value.
