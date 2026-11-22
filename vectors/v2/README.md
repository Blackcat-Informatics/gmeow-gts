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

A case states every further input of its read or judgement:

- `read`: the inputs of the read that are not the default: `profiles`, `trusted_keys`, `held_keys`, `sealing_input`, `marks`,
  `registry_version`, `readers` (the generations that the reader claims to read), and `holds_reference_encoder`
  (whether the reader holds the reference encoder of the storage level).
  A recorded `read_result` itself holds the `capacity`, the `inputs` and the `profiles` of its read; where a case has no
  `read` key, those fields of the record state them.
- a key that ends in `_file`: a second file of the case, such as the source of a conversion, the later file of a proof, a
  proof, or a result.
- `projection`, `compaction_result`, `verify`: the projection of the fold in plain values, the encoded outcome of a
  compaction, and the outcome of a proof.
- `input_cut`, in place of `input_file`: an input that is a prefix of a corpus file and is not stored, stated as the
  `file`, the `length` of the prefix and its `sha256`.
- `input_edit`, in place of `input_file`: an input that is a corpus file changed by an edit and is not stored, stated as
  the `file`, the `edit` in words, and the `length` and `sha256` of the result.
- `output_read_result`: the read of the file that a compaction of the case's input writes; `later_read_result`: the read
  of the later file of a file proof.
- An entry that reads no file states its known answer in plain values, under keys that name them (`key`, `nonce`,
  `plaintext`, `result` and the like).
- `compaction`: the `profile` input of a compaction of the case's input and, where it was written, its outcome.
- `dataset`: in an entry of IMPORT, the dataset that the file was made from, in plain values.
- `nested_results`: in an entry of the bundle profile, the hexadecimal of the list of the records `[name, depth,
  read_result]` of the nested reads; `nested` and `children` state the same reads and the nested files in plain values.
- `signing_rnd`: the random input with which a signature of the input was made, where it is not the deterministic one.

`SHA256SUMS` lists the SHA-256 of every file of the corpus in its expanded form: 6,239 files.

## Large inputs

A file of at most 1 MiB stands here as it is: 6,165 files.

A larger input that compresses to at most 1 MiB stands here as `<name>.bin.zst`, a Zstandard frame of the input: 47 files,
listed in `PACKED.md` with their expanded size and SHA-256. Expand one with `zstd -d` before use.

A larger input that does not compress to that size is not in this directory: 27 files, 565,827,205 bytes, listed in `ABSENT.md`
with their size and SHA-256. Their descriptions are here. The files are in the release archive that `ABSENT.md` names;
extract it at the root of a checkout and compare each SHA-256 before use.

## How the recorded values were obtained

An entry whose `status` is `built and matches` holds the value that the program that built the corpus gave, and that value
agrees with the rules of `docs/v2/`. An entry whose `status` is `derived from the text` holds a value that was derived from
those rules by hand, because that program was written against an earlier text and gave another value. Where an entry and
the text differ, the text governs.

Two entries, `P2/registry/retired/signature-trust` and `P2/registry/retired/hash-trust`, do not state which registry table
their recorded value was made with.

## What the corpus does not cover

Each entry of `docs/v2/FIXTURES.md` names the corpus entry that holds its files, or says why none does: the entry states a
duty of a writer or a contract of an application that no file holds, or it is a measurement.

Inputs that are not in this directory. The inputs listed in `ABSENT.md` are recorded by length and SHA-256 with their
expected results, and a case whose input is a prefix or an edit of such a file states it by `input_cut` or `input_edit`. A
reader of this corpus checks those cases only with the release archive that `ABSENT.md` names.

Values that no second computation confirmed. Every entry was checked against the text by a reader independent of the program
that wrote it, but some values were taken from that program without being recomputed:

- the content digests of folds whose canonical labelling needs the search of `docs/v2/L1.md`, and the unit counts of the
  entries that the labelling limit refuses;
- the bytes of the storage level, which depend on the reference encoder;
- the parity bytes of the spans whose shards were checked only against their digests, and the solving of a stripe outside
  `P5/gf-toy`, `P5/recover-no-signature` and `P5/parity-placement`;
- the wrapped keys of the sealing entries other than the known answers of `P8`.

Behaviour that no entry holds. A member index missing after a full-size sealed pack (`P8/p8-members-layout-missing-small`
holds the rule at a small size), and a file before and after an append that keeps its parity frames (`P5-27`).
