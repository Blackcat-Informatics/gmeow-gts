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
has an identifier `PF-evidence-n` and states one thing.

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST,
MUST NOT and MAY, and no other.

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

**PF-evidence-1.** The checks of this profile MUST be the numbers and labels of the table above; a check is never renumbered,
and a later revision of this document appends a check with the next number.

**PF-evidence-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. Constants

- COMMIT CADENCE `Ne`; value: `1024` ordinary frames; reason: a design bound, computed from the figures of measurement `FE4-12`
  (experiment 5, the event stream of frames of about `956` bytes for each event of two frames) and the sizes of `P2` "The
  mandatory algorithm set". A commit frame with no signature is `116` bytes for a `count` from `256` to `65535` (the `114` bytes
  of `P7`, "A minimal valid file: one header and one commit", and two more bytes of `count`), and one composite signature record
  adds `3412` bytes (a value of `3373` bytes, a key identifier of `32` bytes, the algorithm and the heads), so a signed commit
  is about `3530` bytes (`3528` measured at a `count` of `1024`). After every `1024` frames of that stream (about `489600`
  bytes) it costs about `0.72` per cent; after every `200` frames it would cost about `3.7` per cent. `1024` keeps the
  signed commit below one per cent of the measured stream and bounds the frames that an interruption leaves unsigned.
- PINNED ALGORITHM; value: `mldsa65_ed25519` (code `2` of `signature_algorithm`); reason: custody evidence is kept for longer
  than a classical scheme can be assumed to hold, and the composite holds while either of its halves holds (`P2`, "The mandatory
  algorithm set"). The cost of its size is bounded by the cadence above.

Both constants rest on stated design reasons and are bounds a writer must meet, not measured optima; the cadence uses the
figures of `FE4-12` and of `P2` for its arithmetic; that measurement wrote one signed commit, for its size, and no stream of
signed commits.

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

**PF-evidence-3.** An implementation of this profile MUST NOT change any rule of the parts it depends on; every rule of this
document is a check of section 5 or a requirement on a writer.

## 4. Writer rules

DEFINITIONS. A VALID COMMIT is a `commit` frame that `P5` "The checks of a commit" calls valid. A COMMIT
INTERVAL of a segment is a maximal sequence of consecutive ordinary frames of the segment, in item order, none of which is a valid
commit; transparent and ordinary frames are those of `P1` "The predecessor and transparent frames"; transparent frames neither
belong to a commit interval nor end one. The
UNCOMMITTED TAIL is that of `P5` "The committed prefix".

**PF-evidence-4.** A writer MUST write a valid commit after at most `Ne` consecutive ordinary frames that are not valid commits.

**PF-evidence-5.** A writer MUST end every segment with a valid commit, followed by no ordinary frame other than an `index`
frame (transparent frames, such as the `parity` frames that follow an index, are not counted), and by nothing else but the
`offsets` item and the locator, before it hands the file over.

**PF-evidence-6.** A writer MUST sign every `commit` frame it writes with at least one signature of `mldsa65_ed25519` under a
key that the intended verifiers trust, and MUST NOT write a signature of any other algorithm in a segment of this profile.

**PF-evidence-7.** A writer that appends a segment to a file MUST write its `lineage` `present`, naming the previous segment's
head and, as `ordinal`, `1` when that segment has no lineage and that segment's `ordinal` plus `1` when it has one (`P1`,
"The header").

**PF-evidence-13.** A writer that appends a filler (`P1`, "Growing a file", THE FILLER) and whose resync item states the profile
`evidence` in its `profile` field MUST write, in that resync item, the `observed_defect` mark of the torn item (finding `torn_tail`, `count` `1`, the byte target of the torn
item's bytes) and the `filler` mark naming exactly the filler's bytes. This is a requirement of a writer; no check of a reader
judges it, since a reader cannot tell a filler that no mark names.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form"). A
canonical file has one commit, so the canonical file of a fold of more than `Ne` ordinary frames before that commit fails `evidence_commit_interval` and is no custody chain; a writer
of evidence keeps the rounds it appended.

**PF-evidence-8.** The canonical file of a fold under this profile MUST be the canonical file of the fold with the header's
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
   content of every segment (`P3`, "Suppression hides"), and what an entry hides does not depend on the segment that holds it, and depends on
   the order of frames and of segments only through the holder of a frame, which decides the node that a pair names
   (`P3`, "Naming a blank node in a binding or an entry"), and the values that serve a term (`P3`, "`literal_ref`: a lexical form
   held by a `value` frame"), so an entry in any segment, whatever
   that segment declares, that no trusted signer authenticated could hide authenticated content of the chain from the visible
   dataset, which defeats a custody chain.

8. `evidence_no_commit`: the segment holds no valid commit. A segment of a header alone, or of a header and frames that no
   valid commit closes, fails it: the other checks judge commits that exist and frames that stand after them, and none of
   them requires a commit to exist. One finding for the segment.

A frame of type `commit` is an item that passes the envelope test with `frame_type` `commit` and whose `signatures` field is in
its grammar (`P6`, "The signature record"); one whose `signatures` field is malformed has `malformed_value` and no signature
result, and checks 4 and 5 do not judge it.

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
once for the file, and MUST report each failure as `profile_violation` with `reason` `absent`, `subject` `[4, 2, present(c)]`, and exactly the `position` and
`count` of the table.

**PF-evidence-10.** A reader MUST judge checks 4, 5 and 6 from the `signature_result` records of `P6` alone, computed with the
reader's inputs, and MUST NOT verify any signature a second time or by another rule.

**PF-evidence-11.** A reader MUST report `evidence_lineage` and `evidence_uncommitted_tail` at most once for a segment, and
`evidence_suppress_unauthenticated` at most once for the file, in the last segment that declares the profile.

**PF-evidence-12.** A writer MUST write every `suppress` frame of a segment of this profile before a `commit` frame that it
signs as `PF-evidence-6` requires, so that the frame is authenticated wherever its signer is trusted.

Overlaps. Each check reports its own finding. A `suppress` frame in the uncommitted tail of an evidence segment gives
`evidence_suppress_unauthenticated` and `evidence_uncommitted_tail`. `evidence_uncommitted_tail` stands beside the core's
`uncommitted_tail`, and beside `evidence_commit_interval` when the tail holds more than `Ne` frames. `evidence_lineage` stands
beside `lineage_begins` or `lineage_mismatch`, and stands alone for a segment whose previous segment has a header outcome `unreadable`
or `links_unavailable`. A commit whose pinned signature is `invalid` or `unverified` gives
`signature_invalid` or `signature_unverified` and `evidence_commit_unauthenticated`. A commit that fails its count or root is
not valid: its signature does not authenticate (`P6`, "Coverage and authentication"), so it gives
`evidence_commit_unauthenticated`, and the commit interval it would have ended continues.

What the profile proves, stated with its assumption. When no check fails, the segment holds a valid commit, every frame of type `commit` of the segment carries a
composite signature that authenticates under a key of `trusted_keys`, and the segment ends at a valid commit; by `P6` "Coverage
and authentication" every ordinary frame whose link holds is then an authenticated frame, and every folded `suppress` frame of
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

## 7. Vectors

Every vector is a file read by a reader of registry version `1` that implements this profile, with the minimum capacity (`P7`
"Capacity"), unless the vector states another reader; the inputs are stated. The expected result is a `read_result` (`P7`). Files are built by the reference engine with
the real constants; `K` is a composite key and `J` an `ed25519` key.

- `evidence-pass`; holds: one segment of `10` ordinary frames and a final commit signed with `K`; inputs: `K` held and trusted;
  expected: no `profile_violation`; `authenticity` `authenticated`.
- `evidence-pass-two-segments`; holds: `evidence-pass` and an appended segment with a correct `lineage`, ending at a commit
  signed with `K`; inputs as above; expected: no `profile_violation`.
- `evidence-lineage`; holds: as above with the second segment's `lineage` `absent`; expected: `evidence_lineage` and
  `lineage_begins` in the second segment.
- `evidence-lineage-resync`; holds: a segment opened by a resync item with `lineage` `absent`, after a segment that declares the
  profile; expected: `evidence_lineage` for that segment.
- `evidence-writer-filler-marks`; holds: the file before is `H` (profile `evidence`) `T Q C`, with `C` cut so that `100` bytes of
  its last `value` are missing; the bytes appended are a filler of `100` bytes `00`, then a resync item with `profile` `evidence`
  whose marks hold the `torn_tail` mark over the bytes of `C` and the `filler` mark over `[L, L + 100)`, `L` the length of the
  file before, then the scope; expected: the marks are those of PF-evidence-13 (a vector of the writer; no finding of a reader
  depends on it). The vector `resync-filler-evidence-commit` of `P1`, "Growing a file", shows the segment after a filler with its
  lineage checked and no `evidence_lineage`.
- `evidence-lineage-mismatch`; holds: the second segment's `lineage` names another head; expected: `evidence_lineage` and
  `lineage_mismatch` with `head`.
- `evidence-lineage-previous-unreadable`; holds: the second segment's `lineage` is `present` and the header of the first segment
  is `unreadable`; expected: `evidence_lineage` for the second segment (its lineage is not checked), no `lineage_mismatch`.
- `evidence-interval-pass`; holds: commit intervals of exactly `1024` frames each followed by a signed commit; expected: no
  `profile_violation`.
- `evidence-interval`; holds: a commit interval of `1025` frames before the first commit; expected: `evidence_commit_interval` at
  the `1025`-th frame of the commit interval with `count` `1025`.
- `evidence-interval-invalid-commit`; holds: `1024` frames, a commit with a wrong root, and `1` more frame before the final signed
  commit; expected: `commit_root_mismatch`, `evidence_commit_unauthenticated` at the wrong commit, and one
  `evidence_commit_interval` at the `1025`-th frame, which is the wrong commit itself, with `count` `1026` (the invalid commit does
  not end the commit interval)
- `evidence-check-seven-segment`; holds: two segments declaring the profile, an unauthenticated `suppress` frame in the first and
  none in the second; expected: `evidence_suppress_unauthenticated` once, in the findings of the second segment, which is the
  last that declares the profile
- `evidence-tail`; holds: `evidence-pass` with one `blob` frame appended after the commit; expected:
  `evidence_uncommitted_tail` with `count` `1` and `uncommitted_tail` with `count` `1`.
- `evidence-unsigned`; holds: the final commit carries no signature; expected: `evidence_commit_unsigned` at the commit, and
  `authenticity` `unauthenticated`.
- `evidence-unauthenticated-untrusted`; holds: `evidence-pass`; inputs: `K` held, not trusted; expected:
  `evidence_commit_unauthenticated` at the commit.
- `evidence-unauthenticated-not-held`; holds: `evidence-pass`; inputs: empty; expected: `signature_unverified` with
  `key_not_held` and `evidence_commit_unauthenticated` at the commit.
- `evidence-unauthenticated-bad-root`; holds: the final commit's `root` altered and its signature made over the altered frame;
  inputs: `K` held and trusted; expected: `commit_root_mismatch`, `evidence_commit_unauthenticated`, `evidence_commit_interval`
  not reported (`11` frames), `evidence_uncommitted_tail` with `count` `11`, and `evidence_no_commit` (no valid commit remains).
- `evidence-algorithm`; holds: `evidence-pass` with a second signature by `J` on the commit; inputs: `K` and `J` held and
  trusted; expected: `evidence_signature_algorithm` at the commit with `count` `1`, and no `evidence_commit_unauthenticated`.
- `evidence-algorithm-only`; holds: the final commit signed by `J` only; inputs: `J` held and trusted; expected:
  `evidence_commit_unsigned` and `evidence_signature_algorithm` at the commit.
- `evidence-suppress-pass`; holds: one segment whose `suppress` frame hides one `quad` line of an earlier frame and stands
  before the final commit signed with `K`; inputs: `K` held and trusted; expected: no `profile_violation`.
- `evidence-suppress-tail`; holds: `evidence-pass` with a `suppress` frame appended after the commit, hiding one `quad` line;
  inputs: `K` held and trusted; expected: `evidence_suppress_unauthenticated` with `count` `1` in the segment's findings,
  `evidence_uncommitted_tail` with `count` `1` and `uncommitted_tail` with `count` `1`.
- `evidence-suppress-sealed`; read by a reader of registry version `2` that implements this profile (a reader of version `1`
  never opens a sealed frame); holds: `evidence-pass`, then a second segment of registry version `2` whose header declares no
  profile, with a correct `lineage`, holding a key table and one unsigned `sealed` frame whose inner frame is a `suppress` frame
  that hides one `quad` line of the first segment, then a commit with no signature; inputs: `K` held and trusted, and the
  segment key held; expected: `evidence_suppress_unauthenticated` with `count` `1` in the findings of the first segment.
- `evidence-suppress-other-segment`; holds: `evidence-pass`, then a second segment whose header declares no profile, with a
  correct `lineage`, holding one unsigned `suppress` frame that hides one `quad` line of the first segment, then a commit with
  no signature; inputs: `K` held and trusted; expected: `evidence_suppress_unauthenticated` with `count` `1`, in the findings of
  the first segment (the last that declares the profile), and no other `profile_violation`.
