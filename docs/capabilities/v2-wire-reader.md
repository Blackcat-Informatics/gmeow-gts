---
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: CC-BY-4.0
type: Capability
id: cap:v2-wire-reader
title: "Version 2 wire reader"
description: "Reads the wire observations of a version 2 file: generation dispatch and the framing of its tails."
state: delivered
bindings:
  code:
    - gmeow_gts::v2::reader::read_wire
    - gmeow_gts::v2::reader::Dispatch
  tests:
    - gmeow_gts::test_crate::v2_wire_reader::dispatches_by_the_typed_generation_field
    - gmeow_gts::test_crate::v2_wire_reader::framing_distinguishes_torn_and_undelimitable_tails
  examples:
    - gmeow_gts::v2::reader::read_wire
depends: []
---

# Version 2 wire reader

## Purpose

A caller that holds the bytes of a file needs to know which format generation wrote them and where the items of the file end, without trusting the file. The wire reader returns those observations and refuses to guess at a generation it does not implement.

## Contract

**Dispatch by generation.** The reader decides the generation from the typed generation field of the opening item. A first-generation file and a header of an unsupported generation each give a dispatch result and no segments, with no read stop, so the caller can refuse the file by name instead of misreading it.
<!-- raf:tested-by gmeow_gts::test_crate::v2_wire_reader::dispatches_by_the_typed_generation_field -->

## Usage

Pass the whole input and a read capacity to `gmeow_gts::v2::reader::read_wire`. Read the dispatch result first, then the segments and the stop it reports; a result with a stop describes how far the read got and why it ended.

## Examples

A first-generation header gives the first-generation dispatch and an empty segment list. A tail of one torn byte after a complete segment gives a framing stop at the end of the last complete item, while a tail the framing cannot delimit gives the undelimitable reason at the same offset.

## Constraints

- A torn tail and an undelimitable tail are different framing reasons. The reader reports the offset and length of the tail in both cases and keeps every complete item before it, so a caller can repair or discard the tail knowingly.
<!-- raf:tested-by gmeow_gts::test_crate::v2_wire_reader::framing_distinguishes_torn_and_undelimitable_tails -->

## Explanation

Dispatching on the typed generation field, and not on the shape of the bytes, keeps an old or newer file from being read as the current generation. Reporting the tail as data, and not as a failure of the whole read, preserves every item that was fully written.

## Verification

Two tests cover the contract. One reads published vectors of the first generation and of an unsupported generation and a hand-built second-generation opener, and checks the dispatch result. The other appends each kind of bad tail to a valid file and checks the stop and the last complete item.

<!-- raf:generated:verification-matrix -->
| Item | Kind | Section |
| --- | --- | --- |
| `gmeow_gts::test_crate::v2_wire_reader::dispatches_by_the_typed_generation_field` | test | [Contract](#contract) |
| `gmeow_gts::test_crate::v2_wire_reader::framing_distinguishes_torn_and_undelimitable_tails` | test | [Constraints](#constraints) |
| `gmeow_gts::v2::reader::read_wire` | doctest | [Usage](#usage) |
<!-- raf:generated:end -->
