<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Profile PF-dist: Distribution file

Title of this document, as the profile table of `P2` names it: `Profile PF-dist: Distribution file`.

## 0. Scope and dependencies

This profile defines a DISTRIBUTION FILE: one canonical segment, standing alone, each of whose frames is within the per-frame
minimum capacity of a reader. That is its promise and no more: whether one reader verified one file is a property of that
reader's result, which section 6 defines as a VERIFIED DISTRIBUTION. It is the value `1` `dist` of the profile table, has five
checks of its own and two constants; it adds no vocabulary; it adds no value to any enumeration of the core and uses no frame
type of its own.

It depends on these parts, cited by part and section title:

- `P1` (Model and wire): "The header", "The frame envelope";
- `P2` (Registry): "Identity, labelling, geometry, severity and profile enumerations" (the profile table), "The finding table"
  and "The fields of a finding that are present" (the findings `profile_violation` and `profile_not_checked`);
- `P5` (Durability): "The committed prefix";
- `P7` (Canonical form and reading): "The typed result of reading", "Capacity", "The reader procedure", "The fold of the whole
  file", "The closing phase of a segment, and visibility", "The canonical form", "The canonical check", "The canonical frames
  that can exceed the minimum capacity";
- `P8` (Sealing): "The values of registry version `2`", for the frame types `keys` and `sealed`.

Notation. Enumeration values are written by label; codes are those of `P2` and of section 1. `E(x)` is the deterministic CBOR
encoding of `x` (`P1`). A byte string is written in hexadecimal, two digits a byte, a space between bytes. A requirement
has an identifier `PF-dist-n` .

REQUIREMENT LANGUAGE. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT",
"RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14
[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here. This document uses the key words MUST,
MUST NOT, and no other.

## 1. The profile value and its checks

The value of this profile in the profile table of `P2` is `1` `dist`. A profile is not a registry version: a header that
declares this profile has the `registry_version` that the core content of its segment needs and no more, and the profile never
raises it. A reader states the profiles it implements. A reader that does not implement this profile reads a segment that
declares it in full, runs none of its checks and reports the one information finding `profile_not_checked` with subject
`[4, 1, absent]` (`P2`, "The finding table").

The checks of this profile, numbered by this document from `1`:

| check | label |
|---|---|
| 1 | `dist_not_canonical` |
| 2 | `dist_not_alone` |
| 3 | `dist_decoded_size` |
| 4 | `dist_stored_size` |
| 5 | `dist_sealed_frame` |

A failed check is the finding `profile_violation` with `reason` `absent` and `subject` `[4, 1, present(c)]`, `c` the number of
the check. The profile adds no value to `reason` or to any other enumeration of the core.

**PF-dist-1.** A reader MUST report the checks of this profile by the numbers and labels of the table above; a check is never
renumbered, and a
later revision of this document appends a check with the next number.

**PF-dist-2.** A writer MUST NOT raise the `registry_version` of a header because the segment declares this profile.

## 2. Constants

- DECODED CEILING `Cd`; value: `67108864` bytes (`2^26`); reason: the minimum `decoded_bytes_per_frame` of a conforming reader
  (`P7` "Capacity"). With every `decoded_length` at most `Cd`, no conforming reader reports
  `exceeds_capacity` with `decoded_size` for a frame of a distribution file, so every conforming reader checks and folds every
  frame of a distribution file whose total decoded length is at most `2^36` bytes, whose items are at most `2^26`, whose total
  rebuilt length is at most `2^37` bytes and whose total formed length is at most `2^37` bytes (`P7-34`, `P7-35`, `P7-45`,
  `P7-47`). The ceilings bound each frame and not these totals, nor the size of the fold (`P7-33`), its total labelled
  weight (`P7-48`) or the work of the labelling (`P7`, "Capacity"); a read that meets one of those limits does not
  verify the file (section 6).
- STORED CEILING `Cs`; value: `67108864` bytes (`2^26`); reason: the minimum `stored_bytes_per_frame` of a conforming reader
  (`P7` "Capacity"), by the same argument for `stored_size`.

Both ceilings rest on a stated design reason, the minimum capacity of a reader, and are bounds a writer has to meet.

A different ceiling is a different profile, with a value of its own in the profile table, never a parameter.

What follows from the ceilings. The part on canonical form and reading lists, once, every frame of a canonical file that can
exceed `2^26` bytes and so the minimum capacity (`P7`, "The canonical frames that can exceed the minimum capacity"). A fold
whose canonical file holds a frame of that list above a ceiling has no distribution file.

## 3. What the profile adds, and what it never changes

The profile consists of its checks (section 1), the constants of section 2 and the conditions of section 5. It requires a
canonical claim and a segment that stands alone, and it forbids sealed frames.

It changes none of these: the header grammar, the frame envelope and every content record, framing, identifiers, every hash and
signature preimage, the codec rules, the fold, the labelled set, the content digest, the canonical form and its constants, and
the registry and the meaning of any of its values.

**PF-dist-3.** A reader or writer that implements this profile MUST give, for every input, the result that the parts it depends on
give, with the findings of this document added and nothing else changed; every rule of this document
is a check of section 5 or a numbered requirement on a writer, a reader or a consumer of a result.

## 4. Writer rules

A writer of a distribution file writes the canonical file of the fold it distributes (`P7`, "The canonical form") with the
header field `profile` `present(dist)` and the `registry_version` that `P7`, "The constants", gives it, as one
segment and the whole of the file.

**PF-dist-4.** A writer MUST write a distribution file as exactly one segment that is the canonical file of its fold under
`canonical_form_v1`, with `canonical` `present(canonical_form_v1)`.

**PF-dist-5.** A writer MUST NOT write a distribution file whose canonical file holds a frame with `decoded_length` above `Cd`
or a `payload` longer than `Cs`: the fold has no distribution file, and the writer MUST NOT hand the canonical file that
compaction returns for it (`P7`, "Compaction") to its caller as one.

**PF-dist-6.** A writer MUST NOT write a frame of type `keys` or `sealed` in a distribution file. The value stated: a
distribution is read in full by every reader, with no key. A reader of registry version `1` meets a sealed frame as a critical
frame of an unknown type, and the fold of its segment stops there (`P7`, "A reader of an older registry version"); a reader of
registry version `2` that does not hold the key continues the fold, the sealed frame opaque and `visible` `incomplete` with the
count of sealed frames not opened (`P8`). Neither reads the whole content.

Canonical file. A profile places no frame. The canonical file of a fold has the same frames in the same order whatever profile
its header declares; the `profile` value changes the header and what is computed from it (`P7`, "The canonical form").

**PF-dist-7.** A writer of the canonical file of a fold under this profile MUST write the canonical file of the fold with the
header's
`profile` `present(dist)`: the profile changes no step of its construction.

## 5. Checks

A reader that implements this profile runs these checks at the step `PROFILE CHECKS` of `P7` "The closing phase of a segment,
and visibility", the step that runs the checks of a declared profile, for every segment whose header outcome is `readable` and
whose header `profile` is `present(dist)`. Each failing check is the finding `profile_violation` with `reason` `absent` and
`subject` `[4, 1, present(c)]`, `c` the number of the check (section 1). A reader that does not implement the profile reports
`profile_not_checked` instead (section 1).

| check | condition (the segment fails when) | `position` | `count` | one finding for |
|---|---|---|---|---|
| `dist_not_canonical` | the canonical result of the segment has `claimed` `false` (`P7`, "The canonical check"), or the canonical check gives its structure or its labelling level the state `failed` | absent | absent | the segment |
| `dist_not_alone` | the file holds a segment other than this one | absent | the number of segments of the file | the segment |
| `dist_decoded_size` | a frame has `decoded_length` above `Cd` | the frame | the `decoded_length` | each such frame |
| `dist_stored_size` | a frame has a `payload` of more than `Cs` bytes | the frame | the length of `payload` | each such frame |
| `dist_sealed_frame` | a frame has as its `frame_type` the `uint` `17` or `18` (`keys`, `sealed`), compared as read whatever the reader's registry version | the frame | absent | each such frame |

`offset` and `length` are absent in every finding of this profile (`P2`, "The fields of a finding that are present"). A finding
with a `position` stands in the findings of that frame, every other in the findings of the segment (`P7`, "Where each finding
stands, and in what order").

Check `1` judges the HEAD claim and nothing else: the header's `canonical` field and the two levels of the canonical check that
can violate it, structure and labelling (`P7`, "The canonical check"). The storage level only reports and never enters the
check, so a canonical file whose stored bytes are another encoder's passes it; a level that is `unchecked` (for example with
`exceeds_capacity`) is not `failed` and does not enter it either. A check that does not fail verifies nothing: the absence of
`profile_violation` is not acceptance, and section 6 states what is. Check `2` reads the segments of the file, and checks `3` to
`5` read the fields of the envelope,
so they are decided without decoding a payload: `decoded_length`, the length of `payload` and `frame_type` of each item that
passes the envelope test (`P1`, "The frame envelope"). The checks read the frame type of an item by `P1`, "The locator", A FRAME
ITEM HAS A TYPE ONLY BY A VALUE: an item of no frame type is of no type and gives none of these checks, and its
`malformed_value` finding stands as for any file. A transparent frame is judged as any frame. The size checks `3` and `4`
are this profile's own and report `profile_violation`; the canonical check reports nothing for a frame's size.

**PF-dist-8.** A reader that implements this profile MUST run the five checks of the table on every segment of section 5, and
MUST report each failure as `profile_violation` with `reason` `absent`, `subject` `[4, 1, present(c)]`, and exactly the
`position` and `count` of the table.

**PF-dist-9.** A reader MUST report `dist_decoded_size`, `dist_stored_size` and `dist_sealed_frame` once for each frame that
fails, and `dist_not_canonical` and `dist_not_alone` at most once for the segment.

Overlaps. Each check reports its own finding (`P2`, "The finding table"). A frame above `Cd` read by a reader whose
`decoded_bytes_per_frame` is `Cd` gives `exceeds_capacity` with `decoded_size` and `dist_decoded_size`; the same holds for `Cs`
and `stored_size`. A file of two distribution files gives `dist_not_alone` in each segment and `lineage_begins` in the second:
`dist_not_alone` is a check of the segment, not a check over the content of the file, so every segment that declares the profile
reports it.

## 6. Verified distribution

A consumer that accepts a file as a distribution applies this definition to the `read_result` of one read of it (`P7`, "The
typed result of reading") and to nothing else. It is a predicate over the fields of that result, not a check of this profile: it
produces no finding, and the result does not hold it.

A `read_result` is a VERIFIED DISTRIBUTION exactly when all five of these hold.

1. ONE CLAIMED SEGMENT. `file` has the tag `second_generation` and exactly one segment; that segment's `header_identifier` is
   `present`, its `profile` is `present` with `1` (`dist`), and its `canonical` has `claimed` `true`, structure `verified` and
   labelling `verified`, whatever its storage level.
2. PROFILE CHECKED, NO VIOLATION. No finding of the segment or of its frames has the code `profile_not_checked`, and none has the
   code `profile_violation` with the subject profile `1`.
3. CONTENT DIGEST. The file's `content_digest` is `present` and equal to the segment's `content_digest`.
4. WHOLE INPUT READ. The file's findings hold none of `torn_tail`, `undelimitable_input`, `read_failed` and `exceeds_capacity`;
   the segment's `offset` is `0`; its `integrity` is `intact` (every payload verified, no rebuild and no `terms` frame refused, no
   damage); and no
   finding of the segment or of its frames has the code `exceeds_capacity`.
5. VISIBILITY COMPLETE. `visible` has the tag `complete`.

A result that is not a verified distribution is INCONCLUSIVE, whatever else it shows, when the file's findings hold
`read_failed`, or `exceeds_capacity` with the reason `item_count`: the reader did not read the whole input, a failed read says
nothing of the bytes that were not read, and what the items read show of a segment that the read cut short (no segment, a
canonical level `failed`, a `profile_violation`) is not a fact of the file.

Every other result that is not a verified distribution is REFUTED when it shows that the bytes read are not a distribution file:
a `file` that is not of the tag `second_generation` or that has no segment; a first segment whose `header_identifier` is `present`
and
whose `profile` is `absent` or is not `1`; in any segment, a finding `profile_violation` with the subject profile `1`, a canonical
level `failed` or `integrity` `damaged`; or a file finding `torn_tail` or `undelimitable_input`. It is INCONCLUSIVE otherwise: the
reader did not finish the verification, because a limit of its capacity was met (a level or a payload `unchecked` with
`exceeds_capacity`, a finding
`exceeds_capacity`, a rebuild `refused`, a `labelling_refused` by a limit of the reader), because its labelling is refused by
`refusal_width`, which no capacity changes, or by `refusal_term_node`, the refusal of a fold in which two different nested terms
have one digest, which no capacity changes either, because it does not implement this profile or its access is `forward`
(`profile_not_checked`) or the registry version, a codec or the hash algorithm of the segment, or because its fold is not
complete for a reason that is not damage. Such a result that shows both a refuting fact and an unfinished verification is
REFUTED. The three outcomes are
disjoint and every result has one. An inconclusive result is accepted neither as verified nor as refuted; a reader of a larger
capacity or a fuller implementation can verify the same file.

The predicate says nothing about who wrote the file: `authenticity` is a separate field, and a consumer that needs it applies its
own rule to it (`P6`, "Coverage and authentication").

**PF-dist-10.** A consumer that accepts a file as a distribution MUST accept it only when one `read_result` of it is a VERIFIED
DISTRIBUTION by section 6, and MUST NOT take the absence of `profile_violation` as acceptance.

**PF-dist-11.** A consumer MUST treat a result that section 6 makes INCONCLUSIVE as neither verified nor refuted, and a result
that it makes REFUTED as not a distribution.

## 7. Worked example

A distribution file whose header is `[1, 2, 1, 1, [2, 1], [2, 1], [1]]`: item kind `header`, generation `2`, registry version
`1`, `blake3_256`, profile `present(dist)`, canonical `present(canonical_form_v1)`, lineage `absent`. Its bytes:

```text
87            array of 7
01            item_kind header
02            generation 2
01            registry_version 1
01            hash_algorithm blake3_256
82 02 01      profile present(1), dist
82 02 01      canonical present(1), canonical_form_v1
81 01         lineage absent
```

Thirteen bytes. Check by counting: `1 + 1 + 1 + 1 + 1 + 3 + 3 + 2 = 13`.

The item at position `5` of the segment is a `pack` frame whose `decoded_length` is `67108865`, one byte above `Cd`. The finding
is the typed value `[profile_violation, absent, present([4, 1, present(3)]), present(5), absent, absent, present(67108865)]`: no
reason, subject `[4, 1, present(3)]` (profile `dist`, check `3` `dist_decoded_size`), position `5`, count the decoded length.
Its bytes, from the finding record and the subject form `[4, profile, check: option(uint)]` of `P2`:
`87 18 2e 81 01 82 02 83 04 01 82 02 03 82 02 05 81 01 81 01 82 02 1a 04 00 00 01` (27 bytes). Check, once by fields: `87`
(array of seven), code `46` as `18 2e`, `reason` absent `81 01`, subject `82 02` then `83 04`, the profile, `82 02` and the
check (eight bytes for a profile and a check below `24`), then `position`, `offset` `81 01`, `length` `81 01` and `count`, each
`81 01` when absent or `82 02` and the value; once by counting the bytes listed.
