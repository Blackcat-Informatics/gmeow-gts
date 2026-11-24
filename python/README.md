<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# gts — Graph Transport Substrate

The [GTS v2 specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) defines the current format. The released package predates v2.

## Install

```bash
pip install gmeow-gts
```

The installed package name is `gmeow-gts`; the import name and CLI binary both
remain `gts`, and GTS files keep the `.gts` extension. Optional extras:
`pip install 'gmeow-gts[rdf]'` for `rdflib` interop and
`pip install 'gmeow-gts[db]'` for the DuckDB/Parquet relational exports.

## Verifying the build

Wheels and sdists for `gmeow-gts` are built in GitHub Actions and signed with GitHub
artifact attestations. After downloading a package from PyPI, verify it with:

```bash
gh attestation verify <path-to-wheel-or-sdist> --repo Blackcat-Informatics/gmeow-gts
```

An SPDX SBOM is also generated for each release and attached as a workflow artifact.

## Format and conformance

In the source tree, `gts.v2.decode_header(data, offset=0)` decodes one P1 header
and returns its fields, the next byte offset and the header identifier when
the hash algorithm is supported. `gts.v2.encode_header(header)` writes the
registry version 1 header record for a segment writer.
Use `gts.v2.item_end(data, offset=0)` to walk a CBOR item at a known boundary and
`gts.v2.generation_field(data)` to identify the first item's generation prefix.
Use `gts.v2.dispatch_file(data, budget)` to decide the first opener, recover
a resync at the file start, or obtain the initial framing finding. The
result gives the offset at which the caller can continue after an opener.
`first_and_second=True` also enables selection of the legacy reader.
`gts.v2.decode_frame(data, offset=0)` reads a frame envelope, preserves its exact
identity fields and exposes separate checks for stored and decoded payloads.
`gts.v2.encode_plain_frame(predecessor, frame_type, content, payload)` builds an
unsigned, uncompressed envelope from a byte payload and a content record.
For frame payloads, use one `gts.v2.PayloadBudget` per read. Check frames in
reader order after checking their links. The budget guards zstd decoding and
reports the stored check independently of payload acceptance.

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

## License

See the repository’s [licensing terms](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md) for the package and its documentation.
