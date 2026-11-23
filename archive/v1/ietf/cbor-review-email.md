<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

Subject: [CBOR] Review request: draft-audley-gts-graph-transport-substrate-00

Hello CBOR WG,

I would appreciate review of
`draft-audley-gts-graph-transport-substrate-00`, an individual
Internet-Draft for Graph Transport Substrate (GTS).  GTS is a
content-addressed graph transport format encoded as a CBOR Sequence of
segment headers and frames.  The draft is intended as an Informational
individual submission and is not a request for immediate working-group
adoption.

The most useful near-term review would be on these points:

- Whether the CBOR Sequence framing is clear and appropriate, including
  that a GTS file is a sequence of data items rather than one enclosing
  CBOR item.
- Whether the CDDL appendix accurately describes the per-item grammar
  without implying a whole-file enclosing CBOR item.
- Whether the deterministic-CBOR and content-id preimage rules are
  reviewable and unambiguous.
- Whether the `+cbor-seq` media type suffix use is correct for
  `application/vnd.blackcat.gts+cbor-seq`.
- Whether the IANA media type registration template has enough
  interoperability and security detail for useful review.

COSE appears in the draft only for optional signature and encryption
behavior.  I am not asking the CBOR WG to review those profile details
as part of this initial request, and any focused COSE review would be a
separate later step.

The goal for this first pass is review and discussion of the CBOR/CDDL
and media-type surface.  Working-group adoption, if it ever becomes
appropriate, would require the normal formal call and rough-consensus
process after review.

Thank you,

Patrick Audley
Blackcat Informatics Inc.
