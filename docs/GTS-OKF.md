<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS OKF Profile

The Rust `okf` feature maps an OKF Markdown bundle to a verifiable GTS package
and projects an OKF-profile folded graph back to a bundle directory.

```bash
cargo run --features okf --bin gts -- from-okf okf-bundle/ -o bundle.gts
cargo run --features okf --bin gts -- to-okf bundle.gts --directory restored-okf/
```

The GTS segment profile is `okf`. The header metadata carries an OKF manifest
with schema `gts-okf-v1`, the base IRI used for minted subjects, document count,
and source paths.

## Vocabulary

The v1 vocabulary namespace is:

```text
https://blackcatinformatics.ca/projects/gts/okf#
```

The default document base IRI is:

```text
https://blackcatinformatics.ca/projects/gts/okf/doc/
```

## Mapping

| OKF construct | GTS representation |
|---|---|
| `foo/bar.md` | one RDF subject node |
| subject IRI | `resource:` when present, otherwise `base-iri + percent-encoded relative path` |
| bundle path | `okf:path` string literal |
| `type:` | required `okf:type` string literal |
| `title:` | `okf:title` string literal |
| `description:` | `okf:description` string literal |
| `resource:` | `okf:resource` IRI |
| `tags:` | repeated `okf:tag` string literals, re-emitted sorted |
| `timestamp:` | `okf:timestamp` `xsd:dateTime` literal |
| producer extension scalar | `okf:<key>` string, integer, decimal, or boolean literal |
| producer extension object/array/null | `okf:<key>` JSON literal with datatype `okf:json` |
| Markdown body | `okf:body` literal carrying a `blake3:<hex>` digest plus one inline blob with media type `text/markdown` |
| inline body variant | `okf:body` string literal, accepted by export only with `--inline-body` |
| `[text](target.md)` | `okf:links` edge to the target subject, reified with `okf:linkText` and `okf:linkOccurrence` |

The body blob is authoritative for re-serialization. Link triples are query
surfaces derived from the body; `to-okf` does not rewrite Markdown from them.

## Directory Export

`to-okf` refuses an existing destination directory. On success it writes:

```text
out/
├── .gts-okf/
│   └── manifest.json
├── concept-a.md
├── nested/
│   └── concept-b.md
└── _unmapped.nq
```

`_unmapped.nq` is present only when the graph contains triples outside the OKF
profile, named graphs, or non-OKF reifier/annotation state. These triples are
reported on stderr and preserved in the sidecar instead of being silently
dropped.

## Manifest

`.gts-okf/manifest.json` uses schema `gts-okf-v1`:

```json
{
  "schema": "gts-okf-v1",
  "base_iri": "https://blackcatinformatics.ca/projects/gts/okf/doc/",
  "doc_count": 2,
  "source_paths": ["concept-a.md", "nested/concept-b.md"],
  "unmapped_triples": 0
}
```

The GTS header metadata carries the same schema name, base IRI, document count,
and source path list for verifiable provenance inside the package.

## Round-Trip Laws

Forward OKF round trip:

```text
okf-dir -> from-okf -> package.gts -> to-okf -> okf-dir'
```

The restored bundle is content-equal modulo sorted frontmatter keys, sorted
tags, and YAML canonicalization. Markdown body bytes are byte-identical.

Reverse GTS round trip:

```text
package.gts -> to-okf -> okf-dir -> from-okf -> package.gts'
```

For OKF-profile graphs, the folded graph projection is equal after the round
trip. Content IDs may differ because the importer authors a deterministic new
segment rather than replaying source bytes.

## Rejections

`from-okf` rejects:

- non-directory bundle roots;
- symlinks in the bundle;
- Markdown files without YAML frontmatter;
- frontmatter that is not a mapping;
- documents missing required `type:`;
- unsafe relative paths;
- dangling Markdown links when `--strict-links` is passed.

`to-okf` rejects:

- existing output directories;
- OKF subjects without `okf:path`;
- OKF documents without `okf:type`;
- missing or undecodable body blobs;
- inline `okf:body` literals unless `--inline-body` is passed.

## Relationship To Other Directory Surfaces

`gts dump --directory` writes an inspection tree for arbitrary GTS archives.
`gts to-okf --directory` writes an OKF authoring surface for graphs that use the
OKF profile vocabulary. They are intentionally separate directory contracts:
`gts-dump-v1` is for archive examination, while `gts-okf-v1` is for Markdown
bundle interchange.
