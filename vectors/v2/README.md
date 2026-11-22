# Conformance corpus of format generation 2

This directory holds the conformance corpus of the draft of format generation 2 (`docs/v2/`). The generation is not in force;
the corpus of the generation in force is the rest of `vectors/`.

## What it holds

One directory for each part that has vectors (`P1` to `P9`) and one for each of the six profile documents. The vectors of the
labelling text `L1` stand in `P3`. Each directory has an `INDEX.md` that lists its entries.

An entry is a description, `<name>.json`, and its inputs, `<name>*.bin`. A `/` in the name of an entry is written `__` in
its file names: the entry `finding/exceeds_capacity/stored_size` of `P2` is `P2/finding__exceeds_capacity__stored_size.json`.
A description has these keys:

| key | holds |
|---|---|
| `name` | the name of the entry |
| `status` | how the recorded values were obtained |
| `note` | what the entry tests |
| `section` | the part and section whose rule the entry tests, where stated |
| `cases` | the cases of the entry |

A case names its input (`input_file`), states the `capacity` of the read, and records the expected typed value as the
hexadecimal of its deterministic encoding (`read_result`, or the record that the case names). The typed records are those of
`docs/v2/P7.md`, "The typed result of reading", and of the part that the entry belongs to.

`SHA256SUMS` lists the SHA-256 of every file of the corpus in its expanded form: 5,873 files.

## Large inputs

A file of at most 1 MiB stands here as it is: 5,812 files.

A larger input that compresses to at most 1 MiB stands here as `<name>.bin.zst`, a Zstandard frame of the input: 31 files,
listed in `PACKED.md` with their expanded size and SHA-256. Expand one with `zstd -d` before use.

A larger input that does not compress to that size is not in this directory: 30 files, 647,496,888 bytes, listed in `ABSENT.md`
with their size and SHA-256. Their descriptions are here. An entry whose input is absent cannot be run until that input is
obtained and its SHA-256 compared.

## Entries whose recorded value differs from the text

The corpus was recorded against an earlier text of the draft. An entry whose `status` is `derived from the text` holds a
value that was derived again from the rules of `docs/v2/` as they stand. Every other recorded value agrees with them, but for
one entry:

| entry | what differs | rule |
|---|---|---|
| `P9/convert/snapshot-label-one-scope` | a label in a snapshot and in its segment is two nodes; the result and the proof differ | `P9`, "The conversion reading" |

Until that entry is recorded again, the text governs and the entry is not a test.

Two entries, `P2/registry/retired/signature-trust` and `P2/registry/retired/hash-trust`, do not state which registry table
their recorded value was made with.

## What the corpus does not cover

`docs/v2/FIXTURES.md` lists further inputs with their expected typed results, among them the smallest witness and the nearest
neighbour for each boundary that the text draws between an authenticated run and a detached frame, a verified and an
inconclusive distribution, and two blank-node scopes. An entry of that catalogue that names no entry of this corpus has no
files here.
