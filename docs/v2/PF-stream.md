<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Profile PF-stream: Live event stream

Title of this document, as the profile table of `P2` names it: `Profile PF-stream: Live event stream`.

## 0. Scope and dependencies

This profile defines a LIVE EVENT STREAM: a file that a writer appends to while readers read it, in which commits stand at a
fixed cadence and parity protects every span the writer has passed, so that one damaged region costs no more than parity can
repair. It is the value `3` `stream` of the profile table, has two checks of its own and three constants; it adds no vocabulary;
it adds no value to any enumeration of the core and uses no frame type of its own.

It depends on these parts, cited by part and section title:

- `P1` (Model and wire): "The header", "The frame envelope", "The predecessor and transparent frames";
- `P2` (Registry): "Identity, labelling, geometry, severity and profile enumerations" (the profile table), "The finding table"
  and "The fields of a finding that are present" (the findings `profile_violation` and `profile_not_checked`);
- `P5` (Durability): "The checks of a commit", "The committed prefix", "Parity: one geometry" ("The stream and the spans",
  "Shards, stripes and parity shards", "Checking a parity frame", "The unprotected tail", "What parity guarantees, and what it
  does not"), "The one recovery procedure";
- `P7` (Canonical form and reading): "The typed result of reading", "The reader procedure", "The fold of the whole file", "The
  closing phase of a segment, and visibility", "Reading as a stream", "The canonical form", "Where each finding stands, and in
  what order".

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`, "The typed model and its one serialization"). A byte string is written in hexadecimal, two digits a byte, a
space between bytes. A requirement
has an identifier `PF-stream-n` .

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST and
MUST NOT, and no other.

## 1. The profile value and its checks

The value of this profile in the profile table of `P2` is `3` `stream`. A profile is not a registry version: a header that
declares this profile has the `registry_version` that the core content of its segment needs and no more, and the profile never
raises it. A reader states the profiles it implements. A reader that does not implement this profile reads a segment that
declares it in full, runs none of its checks and reports the one information finding `profile_not_checked` with subject
`[4, 3, absent]` (`P2`, "The finding table").

The checks of this profile, numbered by this document from `1`:

| check | label |
|---|---|
| 1 | `stream_commit_interval` |
| 2 | `stream_parity_missing` |

A failed check is the finding `profile_violation` with `reason` `absent` and `subject` `[4, 3, present(c)]`, `c` the number of
the check. The profile adds no value to `reason` or to any other enumeration of the core.

**PF-stream-1.** A reader MUST report the checks of this profile by the numbers and labels of the table above; a check is never
renumbered, and
a later revision of this document appends a check with the next number.

**PF-stream-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. Constants

- COMMIT CADENCE `Ns`; value: `200` ordinary frames; reason: a design bound. The cadence bounds what a reader that stops at
  the last valid commit leaves unread to `200` frames, at a small cost in commit frames.
- PARITY GEOMETRY; value: `geometry_1` of `P5` (`k` `64`, `m` `4`, shards of `4096` bytes for a span of `262144` bytes or more,
  spans of `4194304` bytes); reason: without parity damaged bytes are lost; with this geometry
  recovery restores what `P5`, "What parity guarantees, and what it does not", states, and a span of fewer than `262144` bytes
  restores at most `4 * z` bytes, `z` the shard length of `P5`, "Shards, stripes and parity shards".
- SPAN LENGTH; value: `4194304` bytes, the span length of `geometry_1` (`P5`, "The stream and the spans"); the profile adds no
  second value.

A different cadence is a different profile, with a value of its own in the profile table, never a parameter.

## 3. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the constants of section 2 and the conditions of section 5. It turns the parity
that `P5` "The stream and the spans" lets a live writer omit into a requirement of this profile.

It changes none of these: the header grammar, the frame envelope and every content record, framing, identifiers, every hash and
signature preimage, the codec rules, the fold, the labelled set, the content digest, the parity geometry, its placement rule and
its checks, the recovery procedure, the canonical form, and the registry and the meaning of any of its values.

**PF-stream-3.** A reader or writer that implements this profile MUST give, for every input, the result that the parts it depends
on give, with the findings of this document added and nothing else changed; every rule of this
document is a check of section 5 or a numbered requirement on a writer, a reader or a consumer of a result.

## 4. Writer rules

DEFINITIONS. A VALID COMMIT is a `commit` frame that `P5` "The checks of a commit" calls valid. A COMMIT INTERVAL of a segment is
a maximal
sequence of consecutive ordinary frames of the segment, in item order, none of which is a valid commit; transparent and ordinary
frames are those of `P1` "The predecessor and transparent frames", and transparent frames neither belong to a commit interval nor
end one, an item that is not a frame neither belongs to one nor ends one, and a frame with no effective identifier, which is no
ordinary frame for a count (`P1`, "The predecessor and transparent frames"), is not counted.
`S`, a SPAN and its `first` and `end` are those of `P5` "The stream and the spans"; a span is FULL as that section defines it, and
a full span is REACHED when its `end` is at most the length of `S`. The item that REACHES a span is the first non-transparent item
whose end in `S` is at least the span's `end`. A reached span is DUE when the item that reaches it is followed, in the segment, by
a further ordinary frame.

**PF-stream-4.** A writer MUST write a valid commit after at most `Ns` consecutive ordinary frames that are not valid commits.

**PF-stream-5.** A writer MUST write the parity frame of every full span immediately after the item that reaches it, as `P5`
"The stream and the spans" places it, and, when it closes a segment with an `index` frame, the parity frame of the final span
after that frame.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form"). A
canonical file has one commit, so the canonical file of a fold of more than `Ns` ordinary frames before that commit fails
`stream_commit_interval`; the snapshot of a long stream is
distributed without this profile.

**PF-stream-6.** A writer of the canonical file of a fold under this profile MUST write the canonical file of the fold with the
header's
`profile` `present(stream)`: the profile changes no step of its construction.

## 5. Checks

A reader that implements this profile runs these checks at the step `PROFILE CHECKS` of `P7` "The closing phase of a segment,
and visibility", the step that runs the checks of a declared profile, for every segment whose header outcome is `readable` and
whose header `profile` is `present(stream)`. Each failing check is the finding `profile_violation` with `reason` `absent` and
`subject` `[4, 3, present(c)]`, `c` the number of the check (section 1). A reader that does not implement the profile reports
`profile_not_checked` instead (section 1).

The conditions, one for each check:

1. `stream_commit_interval`: a commit interval of the segment holds more than `Ns` frames. One finding for each such commit
   interval, at the frame of
   the commit interval that is `Ns + 1`-th in it, with `count` the number of frames of the commit interval.
2. `stream_parity_missing`: a due span has no parity frame in the segment whose `first` and `end` are those of the span, which
   stands at the place that `P5` "The stream and the spans" gives it (PF-stream-9), and which `P5` "Checking a
   parity frame" does not report `parity_damaged`. One finding for each item that reaches at least one such span, at that item,
   with `count` the number of such spans it reaches.

`offset` and `length` are absent in both findings (`P2`, "The fields of a finding that are present"). Each stands in the list that
its `position` gives it (`P7`, "Where each finding stands, and in what order").

**PF-stream-7.** A reader that implements this profile MUST run the two checks on every segment of section 5 and MUST report, once
for each commit interval that fails check `1` and once for each item that check `2` names, one `profile_violation` with `reason`
`absent`, `subject` `[4, 3, present(c)]`, and the `position` and `count` that
the condition states.

**PF-stream-8.** A reader MUST NOT judge a span whose reaching item is the last ordinary frame of the segment, and MUST NOT
judge a short final span, under `stream_parity_missing`.

**PF-stream-9.** A reader MUST count a parity frame for a due span under `stream_parity_missing` only when it stands at the place
that `P5` "The stream and the spans" gives it, and MUST report a due span whose parity frame stands elsewhere as missing. The
place is the position of the item that reaches the span, plus one, plus the number of spans that the same item reaches before it.

CLAIM (a prefix in which every commit is valid conforms). Let a segment pass both checks, and let `F'` be a prefix of the file
that ends at an item boundary inside the segment and in which every `commit` frame of the segment that `F'` holds is valid in
`F'` (`P5`, "The checks of a commit", read on `F'` as a file). Then `F'` reports no finding of this profile.

A READ AS A STREAM. A reader with the access `forward` (`P7`, "Reading as a stream") reads the segment at each item boundary as
the file that is the prefix up to it, so the claim above holds for the stream read of a conforming writer at every item
boundary: both checks run on the frames that have arrived, a due span being judged when the ordinary frame after its reaching
item has arrived.

**PF-stream-10.** A consumer of the result of reading a prefix of a segment MUST NOT take the findings of this profile in it, or
their absence, as the outcome of the complete segment when a `commit` frame of the prefix is not valid in the prefix.

What is not judged. The bytes after the largest `end` of a used parity frame are the unprotected tail of `P5`, "The unprotected
tail", which that section reports for a segment that holds a parity frame; the final span is protected when the writer closes the
segment and writes its parity (`PF-stream-5`).

Overlaps. Each check reports its own finding. A damaged parity frame gives `parity_damaged` and, for a due span, also
`stream_parity_missing`. A commit that fails its count or root is not valid and gives `commit_count_mismatch` or
`commit_root_mismatch`; the commit interval it would have ended continues and can give `stream_commit_interval`. A long commit
interval at the end of
the segment gives `uncommitted_tail` and `stream_commit_interval`.

## 6. Worked example

A segment of this profile whose item at position `40` reaches the end of the second span, followed by an ordinary frame and with
no parity frame for that span between them. The finding is the typed value
`[profile_violation, absent, present([4, 3, present(2)]), present(40), absent, absent, present(1)]`: subject profile `stream`,
check `2` `stream_parity_missing`. Its bytes, from the finding record and the subject form `[4, profile, check: option(uint)]`
of `P2`: `87 18 2e 81 01 82 02 83 04 03 82 02 02 82 02 18 28 81 01 81 01 82 02 01` (24 bytes). The fields: `87`
(array of seven), code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the
check (eight bytes for a profile and a check below `24`), then `position`, `offset`, `length` and `count`, each `81 01` when
absent or `82 02` and the value.
