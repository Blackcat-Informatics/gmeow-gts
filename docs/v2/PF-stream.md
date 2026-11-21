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
  "Checking a parity frame", "The unprotected tail"), "The one recovery procedure";
- `P7` (Canonical form and reading): "The typed result of reading", "The reader procedure", "The fold of the whole file", "The
  closing phase of a segment, and visibility", "The canonical form".

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`). A byte string is written in hexadecimal, two digits a byte, a space between bytes. The words MUST, MUST
NOT and MAY have their meaning in BCP 14 (RFC 2119, March 1997; RFC 8174, May 2017) and carry it only in capitals. A requirement
has an identifier `PF-stream-n` and states one thing.

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

**PF-stream-1.** The checks of this profile MUST be the numbers and labels of the table above; a check is never renumbered, and
a later revision of this document appends a check with the next number.

**PF-stream-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. Constants

- COMMIT CADENCE `Ns`; value: `200` ordinary frames; reason: measurement `FE4-12`, experiment 5. On a stream of events of two
  frames each (one `terms`, one `quads` frame of `50` quads), a commit after every `100` events, that is every `200` frames,
  costs `0.1` per cent of the file, and a commit after every event costs `12.1` per cent; commits every `100` events with parity
  are a non-dominated configuration of that measurement. The cadence also bounds what a reader that stops at the last valid
  commit leaves unread to `200` frames.
- PARITY GEOMETRY; value: `geometry_1` of `P5` (`k` `64`, `m` `4`, shards of `4096` bytes for a span of `262144` bytes or more,
  spans of `4194304` bytes); reason: measurement `FE4-12`, experiment 5, which measures spans of shards of `4096` bytes; a span
  of fewer than `262144` bytes restores at most `4 * z` bytes (`P5`, "What parity guarantees, and what it does not"). Without parity, `4096` damaged bytes in the middle of a file of small frames lost `49950`
  to `50300` of `100000` quads in every one of `9` files, because the fold stops at the damage; with this geometry recovery
  restored every byte after `4096` and after `65536` bytes overwritten with zero bytes, with `ff` bytes and with pseudo-random
  bytes, at a cost of `6.9` to `7.4` per cent of the file (`P5`, "Cost").
- SPAN LENGTH; value: `4194304` bytes, the span length of `geometry_1` (`P5`, "The stream and the spans"); the profile adds no
  second value.

A different cadence is a different profile, with a value of its own in the profile table, never a parameter.

## 3. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the constants of section 2 and the conditions of section 5. It turns the parity
that `P5` "The stream and the spans" lets a live writer omit into a requirement of this profile.

It changes none of these: the header grammar, the frame envelope and every content record, framing, identifiers, every hash and
signature preimage, the codec rules, the fold, the labelled set, the content digest, the parity geometry, its placement rule and
its checks, the recovery procedure, the canonical form, and the registry and the meaning of any of its values.

**PF-stream-3.** An implementation of this profile MUST NOT change any rule of the parts it depends on; every rule of this
document is a check of section 5 or a requirement on a writer.

## 4. Writer rules

DEFINITIONS. A VALID COMMIT is a `commit` frame that `P5` "The checks of a commit" calls valid. A COMMIT INTERVAL of a segment is a maximal
sequence of consecutive ordinary frames of the segment, in item order, none of which is a valid commit; transparent and ordinary
frames are those of `P1` "The predecessor and transparent frames", and transparent frames neither belong to a commit interval nor end one.
`S`, a SPAN and its `first` and `end` are those of `P5` "The stream and the spans"; a span is FULL when its `end` is at most the
length of `S`. The item that REACHES a span is the first non-transparent item whose end in `S` is at least the span's `end`. A
full span is DUE when the item that reaches it is followed, in the segment, by a further ordinary frame.

**PF-stream-4.** A writer MUST write a valid commit after at most `Ns` consecutive ordinary frames that are not valid commits.

**PF-stream-5.** A writer MUST write the parity frame of every full span immediately after the item that reaches it, as `P5`
"The stream and the spans" places it, and, when it closes a segment with an `index` frame, the parity frame of the final span
after that frame.

How a live writer meets the rules. It counts the ordinary frames since its last commit and writes a commit before the count
would pass `Ns`; it holds the bytes of the open span, at most `4194304` bytes,
since the open span may be the final span, whose depth and shard length are known only when the closing index is written (`P5`,
"Cost"), and writes the
span's parity frame as soon as an item reaches the span's end, before it writes the next ordinary frame. A span that ends before the closing index holds
`4194304` bytes, so every parity frame of a live segment before its closing `index` frame is the frame of a span of shards of
`4096` bytes; the parity frame of the final span, written after the closing index from the bytes held, takes the shard length that `P5`,
"Shards, stripes and parity shards", gives its length.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form"). A
canonical file has one commit, so the canonical file of a fold of more than `Ns` ordinary frames before that commit fails `stream_commit_interval`; the snapshot of a long stream is
distributed without this profile.

**PF-stream-6.** The canonical file of a fold under this profile MUST be the canonical file of the fold with the header's
`profile` `present(stream)`: the profile changes no step of its construction.

## 5. Checks

A reader that implements this profile runs these checks at the step `PROFILE CHECKS` of `P7` "The closing phase of a segment,
and visibility", the step that runs the checks of a declared profile, for every segment whose header outcome is `readable` and
whose header `profile` is `present(stream)`. Each failing check is the finding `profile_violation` with `reason` `absent` and
`subject` `[4, 3, present(c)]`, `c` the number of the check (section 1). A reader that does not implement the profile reports
`profile_not_checked` instead (section 1).

The conditions, one for each check:

1. `stream_commit_interval`: a commit interval of the segment holds more than `Ns` frames. One finding for each such commit interval, at the frame of
   the commit interval that is `Ns + 1`-th in it, with `count` the number of frames of the commit interval.
2. `stream_parity_missing`: a due span has no parity frame in the segment whose `first` and `end` are those of the span, which
   stands at the place that `P5` "The stream and the spans" gives it (immediately after the item that reaches the span and after
   the parity frames of the spans that the same item reaches before it, in the order of the spans), and which `P5` "Checking a
   parity frame" does not report `parity_damaged`. One finding for each item that reaches at least one such span, at that item,
   with `count` the number of such spans it reaches.

`offset` and `length` are absent in both findings (`P2`, "The fields of a finding that are present"). Both stand in the findings
of the frame they name (`P7`, "Where each finding stands, and in what order").

**PF-stream-7.** A reader that implements this profile MUST run the two checks on every segment of section 5 and MUST report
each failure as `profile_violation` with `reason` `absent`, `subject` `[4, 3, present(c)]`, and the `position` and `count` that
the condition states.

**PF-stream-8.** A reader MUST NOT judge a span whose reaching item is the last ordinary frame of the segment, and MUST NOT
judge a short final span, under `stream_parity_missing`.

**PF-stream-9.** A reader MUST count a parity frame for a due span under `stream_parity_missing` only when it stands at the place
that `P5` "The stream and the spans" gives it, and MUST report a due span whose parity frame stands elsewhere as missing. The
place is the position of the item that reaches the span, plus one, plus the number of spans that the same item reaches before it.

CLAIM (a prefix in which every commit is valid conforms). Let a segment pass both checks, and let `F'` be a prefix of the file
that ends at an item boundary inside the segment and in which every `commit` frame of the segment that `F'` holds is valid in
`F'` (`P5`, "The checks of a commit", read on `F'` as a file). Then `F'` reports no finding of this profile. Proof. A commit that
is valid in `F'` is valid in the file: its link state is decided from the frames up to the frame after it, which `F'` holds, and
its `count` and `root` from the frames before it, and these are the same in the file. So the valid commits of `F'` are the commit
frames that `F'` holds, and no commit that stands in `F'` is valid in the file and not in `F'`. Every commit interval of `F'` is
therefore a prefix of a commit interval of the file, so it holds at most `Ns` frames. A span due in `F'` is reached by an item
followed in `F'` by an ordinary frame, so it is due in the file and has a parity frame, which stands immediately after the
reaching item and its other parity frames (`P5`, "The stream and the spans") and therefore before that ordinary frame, inside
`F'`. End of proof.

A READ AS A STREAM. A reader with the access `forward` (`P7`, "Reading as a stream") reads the segment at each item boundary as
the file that is the prefix up to it, so the claim above holds for the stream read of a conforming writer at every item
boundary: both checks run on the frames that have arrived, a due span being judged when the ordinary frame after its reaching
item has arrived.

A writer that keeps the rules and writes undamaged bytes meets the premise at every item boundary: each commit it writes has an
intact identifier, so its link holds without a later frame, and its `count` and `root` agree. A reader that reads such a file live,
at any item boundary, reports no finding of this profile. For a prefix that holds a commit that is not valid in it, the claim
says nothing: a later item can make such a commit valid in a longer prefix without changing any earlier byte, so the findings
of that prefix, `stream_commit_interval` and `uncommitted_tail` among them, are the findings of that prefix read as a file and
not the outcome of the segment.

**PF-stream-10.** A statement that a stream conforms, drawn from the reading of a prefix, MUST be made only for a prefix in which
every `commit` frame is valid in the prefix; for another prefix the findings of this profile are the findings of that prefix as
a file, and a reader or a tool MUST NOT take them as the outcome of the complete segment.

What is not judged. The bytes after the end of the last due span are protected only when the writer closes the segment and
writes the final span's parity (`PF-stream-5`); until then they are the unprotected tail of `P5`, reported as `unprotected_tail`
(information).

Overlaps. Each check reports its own finding. A damaged parity frame gives `parity_damaged` and, for a due span, also
`stream_parity_missing`. A commit that fails its count or root is not valid and gives `commit_count_mismatch` or
`commit_root_mismatch`; the commit interval it would have ended continues and may give `stream_commit_interval`. A long commit interval at the end of
the segment gives `uncommitted_tail` and `stream_commit_interval`.

## 6. Worked example

A segment of this profile whose item at position `40` reaches the end of the second span, followed by an ordinary frame and with
no parity frame for that span between them. The finding is the typed value
`[profile_violation, absent, present([4, 3, present(2)]), present(40), absent, absent, present(1)]`: subject profile `stream`,
check `2` `stream_parity_missing`. Its bytes, from the finding record and the subject form `[4, profile, check: option(uint)]`
of `P2`: `87 18 2e 81 01 82 02 83 04 03 82 02 02 82 02 18 28 81 01 81 01 82 02 01` (24 bytes). Check, once by fields: `87`
(array of seven), code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the
check (eight bytes for a profile and a check below `24`), then `position`, `offset` `81 01`, `length` `81 01` and `count`, each
`81 01` when absent or `82 02` and the value; once by counting the bytes listed.

## 7. Vectors

Every vector is a file read by a reader of registry version `1` that implements this profile, with the minimum capacity (`P7`
"Capacity") and empty inputs; the expected result is a `read_result` (`P7`). Files are built by the reference engine with the
real constants. An EVENT FRAME of a vector is a `blob` frame valid by `P4`, `stored_plain`, which gives no finding of its own.

- `stream-pass`; holds: a header `[1, 2, 1, 1, [2, 3], [1], [1]]` (`12` bytes), then `60` commit intervals, each of `200` event frames
  stored as items of exactly `1000` bytes and a commit with no signature (the first commit, `count` `200`, is `115` bytes; every
  later one, `count` at least `401`, is `116` bytes: the `114` bytes of a commit with `count` `0` in `P7` "A minimal valid file:
  one header and one commit", plus one or two bytes of the longer head of `count`). `S` (`P5`, "The stream and the spans") is
  then `12 + 200115 + 59 * 200116 = 12006971` bytes and holds two full spans of `4194304` bytes. The end of the first span,
  `4194304`, is reached by event frame `4192` (frame `192` of commit interval `21`, which runs from `4193331` to `4194331` in `S`), at
  position `4212`, so the first parity frame stands at position `4213`; the end of the second span, `8388608`, is reached by
  event frame `8384` (frame `184` of commit interval `42`, which runs from `8387767` to `8388767`), at position `8426`, so the second parity
  frame stands at position `8427`. Expected: no `profile_violation`; `unprotected_tail` for the `3618363` bytes of `S` after the
  second span.
- `stream-pass-prefixes`; holds: every prefix of `stream-pass` at an item boundary; expected: no `profile_violation` in any of
  them.
- `stream-interval-boundary`; holds: a commit interval of exactly `200` frames, then a commit; expected: no `profile_violation`.
- `stream-interval`; holds: a commit interval of `201` frames; expected: `stream_commit_interval` at frame `201` of the commit interval, `count` `201`.
- `stream-interval-bad-commit`; holds: `stream-pass` as its writer writes it with the first commit's `count` WRITTEN as `199`
  and every other field computed from the frames that are there: its `root` is the root over leaf count `201`, `h0` and the
  `200` event frames before it, which is the root a reader computes over the ordinary frames it observed whatever `count` states
  (`P5`, "The checks of a commit"), and its link holds; the file is not edited after writing. Expected: `commit_count_mismatch`
  at the commit (position `201`) with `count` `200`, and no `commit_root_mismatch`; the commit is therefore not valid, its commit interval
  joins the next, and `stream_commit_interval` stands at frame `201` of the joined commit interval (the commit itself, position `201`) with
  `count` `401`. Every size and position is that of `stream-pass`, since `199` is written in two bytes like `200`, so the file
  still holds both used parity frames and the reading also gives the `unprotected_tail` of `stream-pass` (`P5`, "The unprotected
  tail"): `length` `3618363`, the bytes of `S` after the second span, and `offset` the input offset of byte `8388608` of `S`.
  These three are every finding of the reading: the last commit is valid, so there is no `uncommitted_tail`.
- `stream-parity-missing`; holds: `stream-pass` with the parity frame of the first span removed; expected:
  `stream_parity_missing` at the item that reaches the first span (position `4212`), `count` `1`.
- `stream-parity-damaged`; holds: `stream-pass`, its two parity frames at positions `4213` and `8427` and its `60` commits as
  there, as written by a writer that puts in the content of the first parity frame a `table_digest` that is not the digest of
  that frame's table: that ONE field is altered, and every other field is computed as written (its `identifier` over the content
  as it stands, so its link holds; its payload and the shard digests of its table those of the first span). The first parity
  frame is then damaged by its table digest (`P5`, "Checking a parity frame") and NOT USED; the second is used, and its shards
  hold. Expected, every finding of the whole reading: `parity_damaged` at position `4213`; `stream_parity_missing` at the item
  that reaches the first span (position `4212`), `count` `1`; and `unprotected_tail` as in `stream-pass` (`P5`, "The unprotected
  tail"), `length` `3618363` from byte `8388608` of `S`, because the largest end of a used span that is not stale is still
  `8388608`. The first span is then covered by no used frame, yet it is not in the tail, which runs from the largest such end to
  the end of `S`; `stream_parity_missing` is the finding that reports it.
- `stream-parity-misplaced`; holds: `stream-pass` with the parity frame of the first span moved two frames later, its fields
  unchanged; expected: `stream_parity_missing` at the item that reaches the first span (position `4212`), `count` `1`, beside
  the `parity_damaged` that `P5` "Checking a parity frame" gives it at stage 1, condition 3, since its `anchor` then differs from
  its predecessor; it is damaged, so it never reaches stage 2 and gets no `parity_stale` (PF-stream-9).
- `stream-parity-last-item`; holds: a segment of event frames of about `1` KiB, with commits every `200` frames, whose last
  ordinary frame is the first to reach the end of the first span (`4194304` bytes of `S`) and has no parity frame after it;
  expected: no `profile_violation`.
- `stream-no-parity`; holds: `stream-pass` with every parity frame removed; expected: `stream_parity_missing` at each of the two
  reaching items (positions `4212` and `8425`, there being no parity frame between them), `count` `1` each; no
  `unprotected_tail` (no parity claim).
- `stream-prefix-commit-boundary`; holds: `stream-pass` cut immediately after the commit that ends commit interval `1`, after
  the one that ends interval `21` and after the last; expected: in each, every commit is valid in the prefix, no
  `profile_violation` and no `uncommitted_tail`; the clean-writer guarantee of the claim
- `stream-heal-pending-full`; holds: the header of `stream-pass`, `200` event frames, a commit whose stored `identifier` is the
  32 bytes `00` and whose other fields are computed, and one more event frame linked to the computed identifier of the commit;
  expected: the commit `healed` (`healed_link` at position `201`) and valid; `committed_frames` `201`; `uncommitted_tail` with
  `count` `1` at position `202`; no `profile_violation`; `integrity` `intact`
- `stream-heal-pending-prefix`; holds: the file of `stream-heal-pending-full` cut immediately after the commit (position `201`,
  the last item); expected: the commit `broken` with `chain_break` at position `201`, not valid; `committed_frames` `0`;
  `uncommitted_tail` with `count` `201` at position `1`; `stream_commit_interval` at position `201` with `count` `201`;
  `integrity` `damaged`. The premise of the claim does not hold, so the claim says nothing of this prefix, and the file of
  `stream-heal-pending-full` conforms
- `stream-stream-read`; holds: the file of `stream-pass` read with the access `forward` at every item boundary; expected: no
  `profile_violation` at any item boundary, the claim of this document stated on the stream read (PF-stream-7, PF-stream-10)
- `stream-concat-two`; holds: the byte concatenation of two files of `stream-pass`; expected: no `profile_violation` (both
  checks are checks of a segment and each segment keeps its own commits, its own `S` and its own spans), the second segment
  has `lineage_begins`, and each segment has an `unprotected_tail` of the `length` of that of `stream-pass`, at its own offset (PF-stream-7)
- `stream-heal-pending-by-parity`; holds: a commit that reaches the end of the first span, whose stored `identifier` is
  damaged, followed by the parity frame of that span, which heals it (`P1`, vector `link-healed-before-transparent`), and then an
  ordinary frame; and the same file cut immediately after the commit; expected: the full file has the commit `healed` and no
  `profile_violation`; the cut has the commit `broken`, not valid, and its commit interval continues
