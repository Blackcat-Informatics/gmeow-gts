<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# What changes in GTS v2?

Suppose a museum sends its catalogue, the scans it describes and a signed record of corrections to another archive.
GTS v1 already lets all three travel in one appendable file. With v2, the receiving archivist can ask more precise
questions. Did repacking change the catalogue? Which part of a damaged shipment can I still trust? What evidence
should I keep when I convert the original?

## Compare the contents of two copies

Two archives can use different local labels for the unnamed people or objects in a catalogue. They can also compress
their files differently. V2 defines a content digest that can agree across those differences. It covers details a
plain list of RDF statements would lose, including empty named graphs, attachment declarations and the record of
what was hidden. A matching digest compares the content each reader recovered. To establish who signed it or
whether every image arrived intact, the archivist checks the signature and attachment results separately.
[RDF content](docs/v2/P3.md) · [Attachments](docs/v2/P4.md)

## Know what a download has actually shown you

V1 supports streaming and range requests. V2 spells out what each kind of read delivers. A catalogue browser can ask
for one named graph and display the rows returned, with the checks on the fetched frames. An archivist deciding whether
the shipment is complete needs a whole-file read. A service following a live feed gets a third view, in arrival order,
with an indication of which results can still change. Each application can tell its users what it saw.
[Reading a file, a stream or a range](docs/v2/P7.md)

V2 also defines how to use parity data to repair damage within its recovery limits. If a reader can resume beyond a
damaged region, its report distinguishes the material it recovered from the chain of records a trusted signature
covers. That distinction matters when the missing record could be the correction that changed an attribution.
[Recovery and proofs](docs/v2/P5.md) · [Signature coverage](docs/v2/P6.md)

## Choose what travels and who can open it

V1 puts compression and encryption in a chain of transforms. V2 stores a payload plain or compressed with Zstandard
and gives encryption its own structure. The museum can publish the catalogue and seal a private condition report
for named recipients in the same file. Anyone preparing a public copy needs to choose what it contains deliberately.
Compacting a file after opening a sealed report writes the recovered content in the clear.
[Sealing](docs/v2/P8.md) · [Selective disclosure in practice](docs/v2/GUIDE-applications.md)

V1 also introduced profiles for jobs such as distribution and file archives. V2 defines six profiles and the checks
associated with each. For example, `files` describes paths and entries in a portable directory tree. The new
`container` profile records successive exports of a tree of web resources, including what changed or disappeared.
That gives a museum a way to preserve the history of its public collection site as well as the files behind it.
[File trees](docs/v2/PF-files.md) · [Web resource archives](docs/v2/PF-container.md)

## Bring a v1 archive forward

The two generations have different wire formats. A reader that supports both can open an existing v1 file directly.
For a migration, v2 defines a converter that rereads the output and compares it with the content mapped from v1.
On success it returns a separate proof tying that comparison to the exact source file. An encrypted source frame stops
conversion. Original signatures stay with the v1 file, so keep that file for anyone who needs to verify them.
[Conversion and its proof](docs/v2/P9.md) · [Feature-by-feature migration guide](docs/v2/GUIDE-migration.md)

The repository's reference engines implement v1. For teams building v2, the specification and test cases are available
for review and implementation. [V2 specification](docs/v2/README.md) · [Conformance corpus](vectors/v2/README.md)
