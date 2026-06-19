# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""GTS — Graph Transport Substrate — the format engine (reference implementation).

A small, dependency-light reader/writer for the GTS wire format defined in
``docs/GTS-SPEC.md``: the CBOR append-only log, the RDF 1.2 folded state,
multi-segment ``cat``-append composition (§3.1), the ``identity``/``gzip``/
``zstd`` codecs, opaque/damaged degradation, torn-append detection, the
``gts → nquads`` transform, and COSE signing (§9.2).

Producers and database transforms (``RDF → GTS``, ``gts → {sqlite,duckdb}``)
live with their heavyweight dependencies in consuming packages — this package
owns the format, nothing else.
"""

from __future__ import annotations

from gts.crypto import InMemoryKeys, KeyProvider, Signer
from gts.from_nquads import from_nquads
from gts.model import (
    Diagnostic,
    Graph,
    OpaqueNode,
    Signature,
    Suppression,
    Term,
    TermKind,
)
from gts.nested import GTS_MEDIA_TYPE, NestedReadResult, read_nested
from gts.nquads import to_nquads
from gts.policy import (
    ProfileFinding,
    SignatureTrust,
    TrustPolicy,
    evaluate_profile_policy,
    signature_trust,
)
from gts.rdf import (
    RDF12UnsupportedError,
    from_rdflib,
    from_rdflib_dataset,
    to_rdflib,
    to_rdflib_dataset,
)
from gts.reader import read, read_segments
from gts.trig import from_trig, to_trig
from gts.writer import Writer

__all__ = [
    "Diagnostic",
    "Graph",
    "GTS_MEDIA_TYPE",
    "InMemoryKeys",
    "KeyProvider",
    "NestedReadResult",
    "OpaqueNode",
    "ProfileFinding",
    "RDF12UnsupportedError",
    "Signature",
    "SignatureTrust",
    "Signer",
    "Suppression",
    "Term",
    "TermKind",
    "Writer",
    "TrustPolicy",
    "evaluate_profile_policy",
    "from_nquads",
    "from_trig",
    "from_rdflib",
    "from_rdflib_dataset",
    "read",
    "read_nested",
    "read_segments",
    "signature_trust",
    "to_nquads",
    "to_trig",
    "to_rdflib",
    "to_rdflib_dataset",
]
