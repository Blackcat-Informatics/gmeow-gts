<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Internet-Draft Package

This directory contains the GTS individual Internet-Draft source and
generated RFCXML submission artifact.

- Canonical source:
  `draft-audley-gts-graph-transport-substrate.md`
- Generated RFCXML v3:
  `draft-audley-gts-graph-transport-substrate-00.xml`
- CBOR WG review request draft:
  `cbor-review-email.md`

The draft is an individual Internet-Draft with intended status
Informational.  It is not an IETF standard, a Proposed Standard, an
adopted working-group item, or a CBOR working-group document.

## Regenerate RFCXML

The local development environment used for the initial -00 package did
not have Ruby installed directly, so the reproducible command uses the
official Ruby container image and installs `kramdown-rfc` inside the
throwaway container.  The kramdown output is then passed through
`xml2rfc --preptool` so the committed XML is explicit RFCXML v3 with the
Internet-Draft `seriesInfo` populated:

```sh
tmp_xml="$(mktemp /tmp/gts-draft-unprepped.XXXXXX)"
docker run --rm \
  -v "$PWD:/work:ro" \
  -w /tmp \
  ruby:3.3-alpine \
  sh -lc 'gem install --no-document kramdown-rfc >/tmp/kramdown-rfc.log && KRAMDOWN_NO_SOURCE=1 /usr/local/bundle/bin/kramdown-rfc2629 -3 /work/ietf/draft-audley-gts-graph-transport-substrate.md' \
  > "$tmp_xml"
uvx --from xml2rfc xml2rfc \
  "$tmp_xml" \
  --preptool \
  --out ietf/draft-audley-gts-graph-transport-substrate-00.xml
rm -f "$tmp_xml"
```

If Ruby and `kramdown-rfc` are installed locally, the equivalent local
commands are:

```sh
tmp_xml="$(mktemp /tmp/gts-draft-unprepped.XXXXXX)"
KRAMDOWN_NO_SOURCE=1 kramdown-rfc2629 -3 \
  ietf/draft-audley-gts-graph-transport-substrate.md \
  > "$tmp_xml"
uvx --from xml2rfc xml2rfc \
  "$tmp_xml" \
  --preptool \
  --out ietf/draft-audley-gts-graph-transport-substrate-00.xml
rm -f "$tmp_xml"
```

## Render and Check

Render text and HTML with `xml2rfc`:

```sh
uvx --from xml2rfc xml2rfc \
  ietf/draft-audley-gts-graph-transport-substrate-00.xml \
  --text --html
```

Run `idnits` when available:

```sh
idnits ietf/draft-audley-gts-graph-transport-substrate-00.txt
```

If `idnits` is not installed locally, use the Datatracker submission
precheck or document that `idnits` was unavailable and attach the
`xml2rfc --text --html` result.

## IANA Media Type Check

Before submission, check that the exact subtype is not already present
in the IANA application media type registry:

```sh
curl -fsSL https://www.iana.org/assignments/media-types/application.csv \
  | rg -i 'vnd\.blackcat\.gts\+cbor-seq'
```

The registry should also be checked for nearby `+cbor-seq` registrations
when reviewing the structured-suffix rationale:

```sh
curl -fsSL https://www.iana.org/assignments/media-types/application.csv \
  | rg -i 'cbor-seq'
```

The registration template lives in the draft IANA Considerations
section.
