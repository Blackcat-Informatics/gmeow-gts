<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Profile PF-evidence: Signed custody chain

Title of this document, as the profile table of `P2` names it: `Profile PF-evidence: Signed custody chain`.

## 0. Scope and dependencies

This profile defines a CUSTODY CHAIN: a file that is only appended to, in which every commit carries an authenticating signature
of one pinned algorithm, commits stand at a fixed cadence, and every later segment is bound to the one before it by its lineage.
It is the value `2` `evidence` of the profile table, has eight checks of its own and two constants; it adds no vocabulary; it
adds no value to any enumeration of the core and uses no frame type of its own.

It depends on these parts, cited by part and section title:

- `P1` (Model and wire): "The header" (lineage), "The frame envelope", "The predecessor and transparent frames";
- `P2` (Registry): "Identity, labelling, geometry, severity and profile enumerations" (the profile table), "The finding table"
  and "The fields of a finding that are present" (the findings `profile_violation` and `profile_not_checked`);
- `P3` (RDF content): "Suppression hides";
- `P5` (Durability): "The checks of a commit", "The committed prefix";
- `P6` (Signatures): "The typed inputs of a verifier", "Verification: the procedure and its typed outcomes", "Coverage and
  authentication", "Co-signing and pinning";
- `P8` (Sealing): "The inner frame";
- `P7` (Canonical form and reading): "The typed result of reading", "The reader procedure", "The fold of the whole file", "The
  closing phase of a segment, and visibility", "The canonical form".

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`). A byte string is written in hexadecimal, two digits a byte, a space between bytes. A requirement
has an identifier `PF-evidence-n` .

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST and
MUST NOT, and no other.

## 1. The profile value and its checks

The value of this profile in the profile table of `P2` is `2` `evidence`. A profile is not a registry version: a header that
declares this profile has the `registry_version` that the core content of its segment needs and no more, and the profile never
raises it. A reader states the profiles it implements. A reader that does not implement this profile reads a segment that
declares it in full, runs none of its checks and reports the one information finding `profile_not_checked` with subject
`[4, 2, absent]` (`P2`, "The finding table").

The checks of this profile, numbered by this document from `1`:

| check | label |
|---|---|
| 1 | `evidence_lineage` |
| 2 | `evidence_commit_interval` |
| 3 | `evidence_uncommitted_tail` |
| 4 | `evidence_commit_unsigned` |
| 5 | `evidence_commit_unauthenticated` |
| 6 | `evidence_signature_algorithm` |
| 7 | `evidence_suppress_unauthenticated` |
| 8 | `evidence_no_commit` |

A failed check is the finding `profile_violation` with `reason` `absent` and `subject` `[4, 2, present(c)]`, `c` the number of
the check. The profile adds no value to `reason` or to any other enumeration of the core.

**PF-evidence-1.** A reader MUST report the checks of this profile by the numbers and labels of the table above; a check is never
renumbered,
and a later revision of this document appends a check with the next number.

**PF-evidence-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. Constants

- COMMIT CADENCE `Ne`; value: `1024` ordinary frames; reason: a design bound. A commit frame with no signature is `116` bytes for
  a `count` from `256` to `65535` (the `114` bytes of `P7`, "A minimal valid file: one header and one commit", and two more bytes
  of `count`), and one composite signature record adds `3412` bytes (a value of `3373` bytes, a key identifier of `32` bytes, the
  algorithm and the heads), so a signed commit is about `3530` bytes. `1024` keeps the cost of the signed commits small beside
  the frames they cover and bounds the frames that an interruption leaves unsigned.
- PINNED ALGORITHM; value: `mldsa65_ed25519` (code `2` of `signature_algorithm`); reason: custody evidence is kept for longer
  than a classical scheme can be assumed to hold, and the composite holds while either of its halves holds (`P2`, "The mandatory
  algorithm set"). The cost of its size is bounded by the cadence above.

Both constants rest on stated design reasons and are bounds a writer has to meet.

A different cadence or algorithm is a different profile, with a value of its own in the profile table, never a parameter.

## 3. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the constants of section 2 and the conditions of section 5. It requires of the
result of reading what `P6` "Co-signing and pinning" allows a profile to require: conditions over the fields of
`signature_result` (`algorithm`, `authenticated`), reported as `profile_violation`.

It changes none of these: the header grammar, the frame envelope and every content record, framing, identifiers, every hash and
signature preimage (the signed subject is that of `P6`), the codec rules, the fold, the labelled set, the content digest, the
canonical form, the meaning of `authenticated` and `authenticity` (`P6` "Coverage and authentication"), and the registry and the
meaning of any of its values. No file names a trusted key: trust is the input `trusted_keys` of the reader (`P6`, "The typed
inputs of a verifier").

**PF-evidence-3.** A reader or writer that implements this profile MUST give, for every input, the result that the parts it
depends on give, with the findings of this document added and nothing else changed; every rule of this
document is a check of section 5 or a numbered requirement on a writer, a reader or a consumer of a result.

## 4. Writer rules

DEFINITIONS. A VALID COMMIT is a `commit` frame that `P5` "The checks of a commit" calls valid. A COMMIT
INTERVAL of a segment is a maximal sequence of consecutive ordinary frames of the segment, in item order, none of which is a valid
commit; transparent and ordinary frames are those of `P1` "The predecessor and transparent frames"; transparent frames neither
belong to a commit interval nor end one, an item that is not a frame neither belongs to one nor ends one, and a frame with no
effective identifier, which is no ordinary frame for a count (`P1`, "The predecessor and transparent frames"), is not counted. The
UNCOMMITTED TAIL is that of `P5` "The committed prefix".

**PF-evidence-4.** A writer MUST write a valid commit after at most `Ne` consecutive ordinary frames that are not valid commits.

**PF-evidence-5.** A writer MUST end every segment with a valid commit, followed by no ordinary frame other than an `index`
frame (transparent frames, such as the `parity` frames that follow an index, are not counted), and by nothing else but the
`offsets` item and the locator, before it stops writing the segment (`P1`, "Growing a file", CLOSING A SEGMENT).

**PF-evidence-6.** A writer MUST sign every `commit` frame it writes with at least one signature of `mldsa65_ed25519` and MUST NOT
write a signature of any other algorithm in a segment of this profile.

**PF-evidence-7.** A writer that appends a segment to a file whose last segment has an opener that is `readable` for it MUST write
its `lineage` `present`, naming the previous segment's
head and, as `ordinal`, `1` when that segment has no lineage and that segment's `ordinal` plus `1` when it has one (`P1`,
"The header").

**PF-evidence-13.** A writer that appends a filler (`P1`, "Growing a file", THE FILLER) and whose resync item states the profile
`evidence` in its `profile` field MUST write, in that resync item, the `observed_defect` mark of the torn item (finding
`torn_tail`, `count` `1`, the byte target of the torn
item's bytes) and the `filler` mark naming exactly the filler's bytes. This is a requirement of a writer; no check of a reader
judges it, since a reader cannot tell a filler that no mark names.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form"). A
canonical file has one commit, so the canonical file of a fold of more than `Ne` ordinary frames before that commit fails
`evidence_commit_interval` and is no custody chain.

**PF-evidence-8.** A writer of the canonical file of a fold under this profile MUST write the canonical file of the fold with the
header's
`profile` `present(evidence)`: the profile changes no step of its construction.

## 5. Checks

A reader that implements this profile runs checks 1 to 6 and 8 at the step `PROFILE CHECKS` of `P7` "The closing phase of a
segment, and visibility", the step that runs the checks of a declared profile, for every segment whose header outcome is
`readable` and whose header `profile` is `present(evidence)`. It runs check 7, a check over the whole file, once, after the
closing phase of every segment of the file, and reports it as a finding of the step `PROFILE CHECKS` of the last segment whose
header outcome is `readable` and whose header `profile` is `present(evidence)`. The findings of a list stand in the order of `P7`
("Where each finding stands, and in what order"). Each failing check is the finding `profile_violation` with `reason` `absent` and
`subject` `[4, 2, present(c)]`, `c` the number of the check (section 1). A reader that does not implement the profile reports
`profile_not_checked` instead (section 1). The checks read the results of the core: link states, the validity of commits, the
uncommitted tail, the `lineage` outcome and each `signature_result`, computed with the reader's inputs.

The conditions, one for each check:

1. `evidence_lineage`: the segment is not the first of the file, and its lineage is not checked (`P1`, "The header"). The check
   applies to a segment that a resync item opens: a restart that writes `lineage` `absent` is reported by it.
2. `evidence_commit_interval`: a commit interval of the segment holds more than `Ne` frames. One finding for each such commit
   interval.
3. `evidence_uncommitted_tail`: the uncommitted tail of the segment holds at least one frame.
4. `evidence_commit_unsigned`: a frame of type `commit` has no `signature_result` whose `algorithm` is `mldsa65_ed25519`. One
   finding for each such frame.
5. `evidence_commit_unauthenticated`: a frame of type `commit` has at least one `signature_result` whose `algorithm` is
   `mldsa65_ed25519`, and none of them has `authenticated` `true`. One finding for each such frame.
6. `evidence_signature_algorithm`: an ordinary frame of the segment has a `signature_result` whose `algorithm` is not
   `mldsa65_ed25519`. One finding for each such frame.
7. `evidence_suppress_unauthenticated`: a folded `suppress` frame of the FILE, in any segment, clear or opened from a sealed
   frame, that is not authenticated: a clear `suppress` frame is authenticated when it is an authenticated frame of its segment
   (`P6`, "Coverage and authentication"), and an inner `suppress` frame opened from a sealed frame (`P8`, "The inner frame")
   exactly when the sealed frame that carries it is an authenticated frame. It is a check over the whole file: it runs once, in
   the last segment of the file whose header outcome is `readable` and whose header `profile` is `present(evidence)`, and gives
   one finding there, with `position` absent and `count` the number of such frames. The reason: a suppression entry hides
   content of every segment (`P3`, "Suppression hides"), and what an entry hides does not depend on the segment that holds it, and
   depends on
   the order of frames and of segments only through the holder of a frame, which decides the node that a pair names
   (`P3`, "Naming a blank node in a binding or an entry"), and the values that serve a term (`P3`, "`literal_ref`: a lexical form
   held by a `value` frame"), so an entry in any segment, whatever
   that segment declares, that no trusted signer authenticated could hide authenticated content of the chain from the visible
   dataset, which defeats a custody chain.

8. `evidence_no_commit`: the segment holds no valid commit. A segment of a header alone, or of a header and frames that no
   valid commit closes, fails it: the other checks judge commits that exist and frames that stand after them, and none of
   them requires a commit to exist. One finding for the segment.

A frame of type `commit` whose `signatures` field is malformed (`P6`, "The signature record") has `malformed_value` and no
signature result, and checks 4 and 5 do not judge it.

The present fields. `offset` and `length` are absent in every finding of this profile (`P2`, "The fields of a finding that are
present").

| check | `position` | `count` |
|---|---|---|
| `evidence_lineage` | absent | absent |
| `evidence_commit_interval` | the frame of the commit interval that is `Ne + 1`-th in it | the number of frames of the commit interval |
| `evidence_uncommitted_tail` | the first frame of the tail | the number of frames of the tail |
| `evidence_commit_unsigned` | the commit | absent |
| `evidence_commit_unauthenticated` | the commit | absent |
| `evidence_signature_algorithm` | the frame | the number of its signatures of another algorithm |
| `evidence_suppress_unauthenticated` | absent | the number of folded `suppress` frames of the file, clear or inner, that are not authenticated |
| `evidence_no_commit` | absent | absent |

**PF-evidence-9.** A reader that implements this profile MUST run checks 1 to 6 and 8 on every segment of section 5 and check 7
once for the file, and MUST report each failure as `profile_violation` with `reason` `absent`, `subject` `[4, 2, present(c)]`, and
exactly the `position` and
`count` of the table.

**PF-evidence-10.** A reader MUST judge checks 4, 5 and 6 from the `signature_result` records of `P6` alone, computed with the
reader's inputs, and MUST NOT verify any signature a second time or by another rule.

**PF-evidence-11.** A reader MUST report `evidence_lineage` and `evidence_uncommitted_tail` at most once for a segment, and
`evidence_suppress_unauthenticated` at most once for the file, in the last segment that declares the profile; it MUST report
`evidence_no_commit` at most once for a segment, and each other check once for each commit interval or frame that fails it.

**PF-evidence-12.** A writer MUST write every `suppress` frame of a segment of this profile before a `commit` frame that it
signs as `PF-evidence-6` requires, so that the frame is authenticated wherever its signer is trusted.

Overlaps. Each check reports its own finding. A `suppress` frame in the uncommitted tail of an evidence segment gives
`evidence_suppress_unauthenticated` and `evidence_uncommitted_tail`. `evidence_uncommitted_tail` stands beside the core's
`uncommitted_tail`, and beside `evidence_commit_interval` when the tail holds more than `Ne` frames. `evidence_lineage` stands
beside `lineage_begins` or `lineage_mismatch`, and stands alone for a segment whose previous segment has a header outcome
`unreadable`
or `links_unavailable`. A commit whose pinned signature is `invalid` or `unverified` gives
`signature_invalid` or `signature_unverified` and `evidence_commit_unauthenticated`. A commit that fails its count or root is
not valid: its signature does not authenticate (`P6`, "Coverage and authentication"), so it gives
`evidence_commit_unauthenticated`, and the commit interval it would have ended continues.

What the profile proves, stated with its assumption. When no check fails, the segment holds a valid commit, every frame of type
`commit` of the segment carries a
composite signature that authenticates under a key of `trusted_keys`, and the segment ends at a valid commit; by `P6` "Coverage
and authentication" every ordinary frame other than an `index` frame whose link holds is then an authenticated frame, and every
folded `suppress` frame of
the file, in whatever segment it stands, clear or opened from a sealed frame, is authenticated (a clear one as a frame of its
segment, an inner one through the sealed frame that carries it), so every entry that hides something in the visible dataset
stands in a frame that a key of `trusted_keys` authenticates. The claim holds while the composite signature is unforgeable for
the keys the reader trusts, which needs one of its two halves to hold (`P2`, "The mandatory algorithm set"). Rollback to an
earlier valid commit and loss of a later suffix cannot be detected from the file alone (`P5`, "What a commit does not show").

## 6. Worked example

A segment of this profile has a commit at position `7` with one composite signature whose key the reader holds and does not
trust: the status is `valid` and `authenticated` is `false`. The finding is the typed value
`[profile_violation, absent, present([4, 2, present(5)]), present(7), absent, absent, absent]`: subject profile `evidence`,
check `5` `evidence_commit_unauthenticated`; position `7`. A segment whose first commit interval begins at position `1` and holds
`1030` frames gives check `2` at position `1025`, the `1025`-th frame of the commit interval, with `count` `1030`:
`[profile_violation, absent, present([4, 2, present(2)]), present(1025), absent, absent, present(1030)]`. Their bytes, from the
finding record and the subject form `[4, profile, check: option(uint)]` of `P2`:
`87 18 2e 81 01 82 02 83 04 02 82 02 05 82 02 07 81 01 81 01 81 01` (22 bytes) and
`87 18 2e 81 01 82 02 83 04 02 82 02 02 82 02 19 04 01 81 01 81 01 82 02 19 04 06` (27 bytes). Check, once by fields: `87`
(array of seven), code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the
check (eight bytes for a profile and a check below `24`), then `position`, `offset` `81 01`, `length` `81 01` and `count`, each
`81 01` when absent or `82 02` and the value; once by counting the bytes listed.
