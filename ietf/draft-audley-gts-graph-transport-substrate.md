---
stand_alone: true
ipr: trust200902
cat: info
submissiontype: IETF
area: Applications and Real-Time Area

docname: draft-audley-gts-graph-transport-substrate-00
title: Graph Transport Substrate
abbrev: GTS
lang: en
date: 2026-06-23
kw: [CBOR Sequence, RDF, content-addressed, graph transport]
author: [{role: editor, ins: P. Audley, name: Patrick Audley, org: Blackcat Informatics Inc., email: paudley@blackcatinformatics.ca, country: Canada}]

normative:
  RFC6838:
  RFC8610:
  RFC8742:
  RFC8949:
  RFC9052:
  RFC9053:
  RFC9277:
  RFC5646:
  RDF12:
    target: https://www.w3.org/TR/2026/CR-rdf12-concepts-20260407/
    title: RDF 1.2 Concepts and Abstract Data Model
    author: [{org: W3C}]
    date: 2026-04-07
  BLAKE3:
    target: https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf
    title: BLAKE3
    author: [{name: "Jack O'Connor"}, {name: Jean-Philippe Aumasson}, {name: Samuel Neves}, {name: Zooko Wilcox-O'Hearn}]
    date: 2020

informative:
  RFC7049:
  RFC9111:
  RFC6906:
  GTS-SPEC:
    target: https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md
    title: GTS - Graph Transport Substrate - Specification
    author: [{name: Patrick Audley, org: Blackcat Informatics Inc.}]
    date: 2026-06-18
  GTS-CONFORMANCE:
    target: https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-CONFORMANCE.md
    title: GTS Conformance
    author: [{name: Patrick Audley, org: Blackcat Informatics Inc.}]
    date: 2026

--- abstract

GTS (Graph Transport Substrate) is an ontology-independent binary
container and transport format for RDF 1.2 datasets and
content-addressed binary payloads.  A GTS file is a CBOR Sequence of one
or more append-only segments.  Each segment consists of a deterministic
CBOR header followed by deterministic CBOR frames linked by BLAKE3
content identifiers.  The logical dataset is obtained by a deterministic
fold over the segment sequence.  GTS supports partial readability,
opaque encrypted or unknown-codec frames, append-only suppression,
optional signatures and encryption, and cross-language conformance
through a shared vector corpus.

--- middle

# Introduction

GTS encodes a graph as an append-only log of CBOR frames.  The logical
graph is the fold, or replay, of the log.  Growth is an append; deletion
is represented as additive suppression; optimization is a separate
lossy compaction step that rewrites the log into a snapshot.

At revision -00, this document is an individual Internet-Draft and work
in progress.  It is not an IETF standard, a Proposed Standard, an
adopted working-group item, or a CBOR working-group document.  The
primary goals of this draft, derived from the repository specification
{{GTS-SPEC}}, are to make the wire format reviewable in IETF form,
prepare media type registration for
`application/vnd.blackcat.gts+cbor-seq`, and invite review of the CBOR
Sequence framing, CDDL, deterministic-CBOR preimages, and media type
template.

Four properties define the format:

- CBOR all the way down.  A GTS file is a CBOR Sequence {{RFC8742}} of
  data items with no whole-file enclosing CBOR item.
- A durable transform catalog.  Payloads name stackable codecs from a
  file-local catalog whose entries use ecosystem-wide canonical names.
- Integrity by construction.  Headers and frames use deterministic CBOR
  {{RFC8949}} preimages and BLAKE3-256 content identifiers {{BLAKE3}}.
- Recursive composition.  A blob can itself carry a complete GTS file,
  including a signed or encrypted nested graph.

GTS does not define a query language, a mandatory random-access index, a
reasoner, or a mutation protocol.  Those are profile, projection, or
application-layer concerns.

# Terminology and Conventions

{::boilerplate bcp14-tagged}

This document uses these terms:

Log:
: The ordered sequence of frames in a GTS segment.

Segment:
: One self-contained GTS header followed by zero or more frames.

Frame:
: A CBOR data item after the segment header.

Fold:
: The deterministic replay of a segment or file into graph state.

Term:
: An RDF term, including an IRI, literal, blank node, or quoted triple,
  represented by a segment-local integer identifier while reading.

Reifier:
: A term that denotes a quoted triple and can carry statement-level
  metadata under the RDF 1.2 `rdf:reifies` model {{RDF12}}.

Capability:
: A library or key needed to reverse a payload transform.

Opaque node:
: The graph representation of a frame whose payload cannot be decoded
  by the reader.

Baseline Reader:
: A reader that parses the CBOR Sequence, verifies the header/frame
  chain, supports the baseline codecs, folds the core frame types, and
  surfaces undecodable frames as opaque nodes.

Full Reader:
: A reader that also verifies COSE signatures, decrypts frames for which
  it holds keys, recurses into nested GTS blobs when requested, and can
  use optional index data.

# File Structure and Wire Format

A GTS file is a CBOR Sequence, not one enclosing CBOR item.  A file is
one or more segments; each segment begins with a Header and then carries
zero or more Frame items.

~~~ text
GTS-file = segment *segment
segment  = [ self-describe-tag ] header *frame
~~~

The optional self-describe tag is CBOR tag 55799
(`0xd9 0xd9 0xf7`) from {{RFC8949}}.  If present, it tags the segment
Header item only.  It is not a separate log item, does not have an
`"id"`, and does not participate in the id/prev chain.

The first data item of every segment MUST be a Header.  Every subsequent
data item of the segment is a Frame until the next Header or end of
input.  A writer appends by concatenating another frame to the current
segment or by concatenating a complete later segment.  There is no
length prefix or item count that a writer must rewrite.

## Multi-Segment Composition

Byte-concatenation of valid GTS files yields a valid multi-segment GTS
file.  After a reader has consumed at least one frame, a later data item
that is a map containing the key `"gts"` and lacking the frame key `"t"`
MUST be treated as the Header of a new segment.  Any self-describe tag
on that item tags the new Header.

Each segment has its own genesis Header, id/prev chain, signatures, and
optional index.  A file's composite identity is the ordered list of
segment head identifiers.  Term identifiers are segment-scoped; the only
cross-segment identity is term value.  Blank-node labels MUST NOT be
merged across segments.

## Streaming Prefixes and Layout States

Every byte prefix of a valid GTS file that ends at a CBOR item boundary
is itself a valid prefix fold.  A reader MUST ignore trailing incomplete
CBOR bytes as a torn append and surface a diagnostic.

The default layout state is accretive, meaning append-optimized.  A
segment can instead claim a streamable layout with Header key
`"layout": "streamable"`.  A verifying reader MUST check that the
claimed covered region has an intact trailing index and that each inline
blob is preceded by graph metadata describing its digest.  Violations
are reported as `StreamableLayoutError`.

# CBOR Conventions

Any bytes that are hashed or signed MUST use deterministic CBOR encoding
under {{RFC8949}}, Section 4.2: shortest-form integers,
definite-length items, and map keys sorted bytewise on their encoded
form.  Implementations MUST NOT rely on older {{RFC7049}} canonical
ordering if that library sorts map keys length-first.

Maps use short text-string keys such as `"t"` and `"d"`.  BLAKE3 digests
are 32-byte byte strings.  The complete CDDL appendix is in
{{CDDL-appendix}} and uses the Concise Data Definition Language from
{{RFC8610}}.

# Header and Frame Grammar

The Header is the first data item in a segment and the chain genesis.
It is not a frame and has no `"prev"` key.

~~~ cddl
header = {
  "gts"  : "GTS1",
  "v"    : uint,
  "prof" : tstr,
  "cat"  : { * codec-id => codec },
  ? "layout": tstr,
  ? "dct": { * tstr => bstr },
  ? "meta": any,
  "id"   : content-id,
}

codec = {
  "name" : tstr,
  "cls"  : "encode" / "compress" / "encrypt",
  ? "dct": tstr,
  ? "p"  : any,
}
~~~

The Header `"id"` MUST equal `BLAKE3-256` over the deterministic CBOR of
the Header map excluding only the `"id"` key.  All other keys, including
unknown extension keys, participate.  The optional self-describe tag is
outside the Header map and outside the preimage.

The Header `"dct"` map is a segment-local dictionary registry keyed by
text identifiers.  Each value is an uninterpreted byte string until a
codec catalog entry references that key through its own `"dct"` field.
A codec `"dct"` value names the dictionary bytes used to reverse that
codec.  A codec `"p"` value carries codec-specific parameters as
deterministic CBOR.  A reader that does not understand required
parameters or cannot resolve a referenced dictionary folds the affected
frame as opaque.

All frames share one envelope:

~~~ cddl
frame = {
  "t"   : frame-type,
  ? "x" : [+ codec-id],
  ? "pub": any,
  ? "to": [+ recipient],
  ? "d" : bstr / any,
  "prev": content-id,
  "id"  : content-id,
  ? "sig": bstr,
}

frame-type = "terms" / "quads" / "reifies" / "annot" / "blob" /
             "suppress" / "snapshot" / "meta" / "index" / "opaque"

recipient = { "kid": tstr, ? "alg": tstr, * tstr => any }
~~~

Each frame's `"id"` MUST equal `BLAKE3-256` over the deterministic CBOR
of the frame map excluding `"id"` and `"sig"`.  Each frame's `"prev"`
MUST equal the previous data item's `"id"`; for the first frame in a
segment, `"prev"` is the Header `"id"`.  Because `"prev"` is inside the
hashed frame content, each frame id transitively commits to the prior
items in the segment.

## Payload Resolution

If `"x"` is absent, the payload is `"d"` directly.  If `"x"` is present,
`"d"` MUST be a byte string and each codec identifier MUST resolve
through the segment Header catalog.  The reader reverses the codec chain
last-to-first.  If a codec library or key is missing, the reader stops
and folds the frame as opaque.  After a transform chain is reversed, the
decoded bytes are interpreted as the frame-type-specific CBOR payload,
except that a `blob` payload is raw bytes.

## Index Frame

An `index` frame is optional and accelerates reading; it is not required
by a Baseline Reader.

~~~ cddl
index-payload = {
  "count"  : uint,
  "head"   : content-id,
  ? "off"  : [+ uint],
  ? "ti"   : { * frame-type => [+ uint] },
  ? "dict" : [+ uint],
  ? "mmr"  : content-id,
}
~~~

`"off"` records byte offsets, `"dict"` locates term dictionary frames,
`"head"` records the last covered frame id, and `"mmr"` records an
optional Merkle-Mountain-Range root over covered frame ids.  A later
index MAY cover an earlier index as an ordinary frame.  An index never
covers itself.

# Graph Data Model and Fold

A reader folds each segment into logical state containing terms, quads,
reifier bindings, annotations, blobs, blob metadata, segment metadata,
suppressions, opaque nodes, signatures, diagnostics, segment heads,
profile declarations, and layout state.  A file fold is the ordered
value-union of segment folds.  Segment-local term ids are resolved
inside their own segment before any cross-segment union.

GTS imports the RDF 1.2 Concepts and Abstract Data Model Candidate
Recommendation Snapshot dated 7 April 2026 for IRIs, blank nodes,
literals, RDF datasets, triple terms, version label `"1.2"`, and
`rdf:reifies` {{RDF12}}.  Core GTS does not require an RDF parser, query
language, entailment regime, canonicalization algorithm, or concrete RDF
syntax.

Value equality is exact at the transport layer:

- IRIs compare by exact Unicode string value after CBOR decoding.
- Literals compare by lexical string, datatype IRI, language tag when
  present, and RDF 1.2 base direction when present.  Datatype lexical
  canonicalization is not applied.
- Blank nodes compare only within their blank-node scope.  Blank nodes
  from different segments or nested GTS files are never equal.
- Quoted triple terms compare by their resolved subject, predicate, and
  object term values.
- Opaque nodes preserve occurrence identity by segment identity and
  frame content id.

## Terms

A `terms` payload is an ordered array.  Term ids are unsigned integers
assigned in append order, starting at zero for each segment.

~~~ cddl
terms-payload = [+ term]
term = {
  "k"   : 0 / 1 / 2 / 3,
  ? "v" : tstr,
  ? "dt": term-id,
  ? "l" : tstr,
  ? "dir": "ltr" / "rtl",
  ? "rf": term-id,
}
~~~

For literals, absent datatype is defaulted to `rdf:dirLangString` when
both `"l"` and `"dir"` are present, `rdf:langString` when only `"l"` is
present, and `xsd:string` otherwise.  Language tags use BCP 47 syntax
{{RFC5646}}.  A `"dir"` value has no meaning without `"l"`.

The term kind `"k"` identifies the row form: `0` is an IRI whose `"v"` is
the IRI string, `1` is a blank node whose `"v"` is the segment-local
blank-node label, `2` is a literal whose `"v"` is the lexical form, and
`3` is a quoted triple term.  A quoted triple term uses `"rf"` to name
the reifier term id whose binding appears in a `reifies` payload.  The
`"dt"`, `"l"`, and `"dir"` fields apply only to literals.

## Reifiers, Quads, and Annotations

RDF 1.2 permits a triple to be the subject or object of another triple.
GTS represents a quoted triple through a reifier term and a `reifies`
frame:

~~~ cddl
reifies-payload = { * term-id => [term-id, term-id, term-id] }
quads-payload = [+ [term-id, term-id, term-id, ? term-id]]
annot-payload = [+ [term-id, term-id, term-id]]
~~~

A `quads` row asserts a triple in the default graph or the named graph
identified by the fourth term id.  A `reifies` binding asserts
`R rdf:reifies <<( S P O )>>` in the default graph.  An `annot` row
asserts metadata about a reifier; its positions are `[reifier,
predicate, object]`.  Referencing a quoted triple does not assert the
base triple; the base triple is asserted only if it appears in a
`quads` frame.

Predicates in `quads`, `reifies`, and `annot` rows MUST be IRIs.  A quad
subject MUST be an IRI, blank node, or quoted triple.  A graph name, when
present, MUST be an IRI or blank node.

## Fold Algorithm

The fold is deterministic.  In outline:

~~~ text
result := empty file state
for segment in file order:
  verify the Header id and each frame id/prev link within the segment
  terms := [] ; graph := {} ; reifiers := {} ; annotations := []
  blobs := {} ; meta := {} ; suppressions := [] ; opaque := []
  for frame in segment order:
    payload := resolve frame payload
    if undecodable: append an opaque node and continue
    switch frame.t:
      "terms"    : append terms and assign segment-local ids
      "quads"    : add each resolved quad value tuple
      "reifies"  : bind each reifier, keeping the first non-conflict
      "annot"    : append annotation rows
      "blob"     : store inline bytes or register external digest
      "suppress" : append suppression directives
      "snapshot" : replace segment state
      "meta"     : shallow-merge segment metadata
      "opaque"   : append explicit opaque node
  union the segment fold into result by value, not raw term id
~~~

Duplicate quads collapse as set entries.  Annotation rows remain an
ordered multiset.  Conflicting reifier bindings produce a
`ConflictingReifier` diagnostic and keep the first binding.  Unknown
structural frame types preserve chain verification and are surfaced as
opaque nodes or diagnostics until a profile-aware reader handles them.

Suppressions are additive tombstones.  They never remove bytes from the
CBOR Sequence, never change frame ids, and never break chain
verification.  A default projection applies suppression directives by
value across the folded file state unless a profile defines a narrower
scope.  A `frame` target suppresses that frame's logical contribution
while leaving term definitions available for resolving later rows.  A
`blob` target suppresses the matching blob digest.  A `quad` target
suppresses matching resolved quads.  A `reifier` target suppresses the
reifier binding and annotation rows whose first position is that reifier.
A `term` target suppresses only direct exposure of that term-table entry;
it does not cascade to quads, quoted triples, reifiers, annotations, or
blobs that reference the term.

On full replay, a `snapshot` frame replaces the accumulated state for
the current segment at that frame position, and following frames apply on
top of the snapshot state.  A snapshot does not erase state already
folded from earlier segments; it can only affect earlier visible content
through explicit suppression directives carried in or after the
snapshot.

# Transform Catalog

Each codec catalog entry declares a class:

| Class | Examples | Capability needed to reverse |
|---|---|---|
| `encode` | `identity`, `base64url`, `base85` | None |
| `compress` | `gzip`, `zstd`, `lzma2` | Codec library |
| `encrypt` | `cose-encrypt0`, `cose-encrypt` | Recipient key |

The `"x"` transform array is applied in order on encode and reversed in
the opposite order on decode.  Decoding requires every named capability.
A missing library or key yields an opaque node rather than data loss.

A Baseline Reader MUST implement `identity`, `gzip`, and `zstd`.
Writers targeting maximum longevity SHOULD restrict payload transforms
to this core set.  Conformance claims are checked against the project
vector corpus and conformance policy {{GTS-CONFORMANCE}}.

# Integrity and Confidentiality

GTS separates four concerns:

- frame integrity, via each Header or frame `"id"`;
- history integrity, via the frame `"prev"` chain;
- origin or authorship, via optional COSE signatures; and
- freshness or non-truncation, via a head commitment such as a signature
  over the head id or an index `"head"`/`"mmr"` commitment.

The first two are mandatory and key-free.  The last two are optional and
profile-specific.

## Hash Chain

Each frame id is a hash of self-contained frame content.  With known
offsets, frame hashes can be verified in parallel followed by a simple
`"prev"` equality pass.  A corrupt frame fails its own id.  Recovery of
later frames is guaranteed only when later offsets are known from an
intact index, checkpoint, external framing, or storage layer.

The id/prev chain does not detect truncation by itself.  A verifier
needs a trusted head commitment.

## Signatures and Encryption

A frame MAY carry `"sig"`, a serialized `COSE_Sign1` {{RFC9052}} over
the frame id as a detached payload.  Because the frame id commits to the
public envelope, transformed payload bytes, and chain position, the
signature binds those fields to the signer.  The signature algorithm is
declared by COSE and uses algorithm identifiers such as those registered
for COSE algorithms {{RFC9053}}.

COSE is profile-specific in this draft.  GTS references COSE only for
optional signature and encryption behavior.  Review of signature and
encryption profiles, including any request to the COSE working group, is
a future-focused step after those profiles are stable enough for that
review.

An `encrypt`-class codec wraps the payload as `COSE_Encrypt0` or
`COSE_Encrypt`.  Recipients are listed in cleartext by key identifier
only.  A reader that lacks the needed key folds the frame as opaque.

## Opacity Invariant

Opacity hides content, not existence, provenance, or position.  For
every frame, `"id"`, `"prev"`, `"t"`, `"x"`, `"to"`, `"pub"`, and
`"sig"` remain in cleartext when present.  A reader without a key can
still carry the frame, verify its chain position, and report public
envelope metadata.

# Profiles and Nested GTS

A profile is a named set of conventions above the core wire format,
declared by the Header `"prof"` field.  Profiles can define vocabulary,
validation rules, trust policy, capability requirements, and publication
workflows.  Profiles MUST NOT change Header or frame grammar,
segment-boundary detection, id or signature preimages, transform-catalog
resolution, or the core fold.

Baseline readers parse and fold a file even when they do not implement a
named profile, subject to normal capability limitations.  Profile-aware
tools can apply stricter validation and diagnostics.

A blob whose media type is `application/vnd.blackcat.gts+cbor-seq` is
itself a complete GTS file.  A Full Reader MAY recurse into such a blob,
folding the inner file as an independent subgraph.  The nested GTS has
its own Header, id/prev chain, signatures, and blank-node scope.  A
reader that recurses MUST enforce maximum nesting depth and total
decoded-size budgets.

# Media Type and HTTP Serving Semantics

The media type for GTS is `application/vnd.blackcat.gts+cbor-seq`.  The
`+cbor-seq` structured syntax suffix {{RFC9277}} is used because a GTS
file is a CBOR Sequence, not a single enclosing CBOR data item.  The
file extension is `.gts`.

When identifying bytes without trusted metadata, a reader treats `.gts`
and `application/octet-stream` as hints only.  It then parses the first
CBOR item, optionally unwrapping self-describe tag 55799, and confirms a
Header map containing `"gts": "GTS1"` and lacking frame key `"t"`.
Complete validity still requires parsing the whole observed byte stream
and applying the selected conformance checks.

HTTP deployments that serve GTS SHOULD preserve bytes exactly, SHOULD
avoid transport-layer transformations that would break content hashes,
and SHOULD support byte ranges for partial consumption.  Versioned,
immutable URLs can use long-lived immutable caching.  Mutable `latest`
or content-negotiated aliases need conservative cache behavior and
appropriate `Vary` handling; see {{RFC9111}}.  Profile negotiation
through mechanisms such as the profile link relation {{RFC6906}} is a
possible future extension and is not required by this draft.

# Security Considerations {#Security}

The id/prev chain provides integrity, not confidentiality.  Use
`encrypt`-class codecs for confidentiality.

Truncation is undetectable from the chain alone.  Evidence or archival
profiles need a trusted head commitment, such as a signature over the
head id or an index `"head"`/`"mmr"` commitment.

Recovery after a damaged frame requires known offsets.  In a bare CBOR
Sequence, arbitrary corruption can desynchronize the decoder.  GTS does
not define parity or erasure coding.

Cleartext `"to"` key identifiers, `"pub"` envelopes, and metadata can
leak relationship or content metadata.  High-privacy profiles need
pseudonymous or pairwise key identifiers and must avoid secrets in
cleartext fields.

A valid signature attests that a signer signed the frame bytes.  It does
not assert that the signed graph claims are true.

Compaction rewrites ordering.  Snapshot compaction discards original
frame signatures.  Streamable compaction can preserve old frame
signatures only as detached provenance over old frame ids, while the new
ordering is attested by the compactor.

Readers MUST bound decompression of attacker-supplied frames.  Readers
MUST also bound nested-GTS recursion depth and total decoded size to
avoid resource-exhaustion attacks.

Segments are independently authentic, not mutually vouched.  Concatenating
a segment does not imply endorsement by earlier segment signers.  An
untrusted appended segment can suppress earlier content from default
presentation; high-assurance consumers need policy about which segment
signers may issue suppression directives.

# IANA Considerations

This document requests registration of one media type under the
procedures of {{RFC6838}}.  The registration is an interoperability step
and is not an IETF endorsement or standardization of GTS.

## Media Type Registration: application/vnd.blackcat.gts+cbor-seq

Type name:
: `application`

Subtype name:
: `vnd.blackcat.gts+cbor-seq`

Required parameters:
: None.

Optional parameters:
: None.

Encoding considerations:
: Binary.  A GTS file is a CBOR Sequence and is not restricted to
  7-bit or textual transports.  Transports that are not 8-bit clean
  need a content-transfer encoding.

Security considerations:
: See {{Security}}.  In summary, the content-id chain provides integrity
  but not confidentiality; truncation requires a head commitment;
  decompression and nested-GTS recursion must be bounded; signatures
  attest bytes, not claim truth; and cleartext envelope fields can leak
  metadata.

Interoperability considerations:
: The `+cbor-seq` suffix signals that the payload is a CBOR Sequence, so
  generic sequence tooling can inspect ordered data items before
  applying GTS-specific rules.  GTS requires deterministic CBOR for
  hashed and signed preimages.  The self-describe tag 55799 can tag a
  segment Header as a magic number.  Conformance is defined by the
  versioned vector corpus maintained with the specification.

Published specification:
: This Internet-Draft while in draft form; the resulting RFC if this
  draft is published as an RFC.

Applications that use this media type:
: Content-addressed RDF 1.2 graph transport and archival; signed
  agent-memory and provenance artifacts; package distribution where the
  payload bundles a graph and the binary content it references.

Fragment identifier considerations:
: None.  Fragment identifiers are not defined for this media type.

Additional information:
: Magic numbers: optional `0xd9 0xd9 0xf7`, the CBOR self-describe tag
  55799, when present at the start of a segment Header.  File extension:
  `.gts`.  Macintosh file type code: none.

Person and email address to contact for further information:
: Patrick Audley <paudley@blackcatinformatics.ca>

Intended usage:
: COMMON.

Restrictions on usage:
: None.

Author:
: Blackcat Informatics Inc.

Change controller:
: Blackcat Informatics Inc., unless changed through a later IETF
  process.

--- back

# CDDL Appendix {#CDDL-appendix}

This appendix is the copyable schema surface for implementers.  The
top-level file is a CBOR Sequence; CDDL describes the individual items
in that sequence.

~~~ cddl
gts-item = header-item / frame
header-item = header / self-described-header
self-described-header = #6.55799(header)

term-id = uint
frame-index = uint
codec-id = uint
digest = bstr .size 32
content-id = digest
blake3-uri = tstr
digest-ref = digest / blake3-uri
profile-name = tstr
layout-state = "streamable" / tstr
extension-key = tstr

header = {
  "gts": "GTS1",
  "v": 1,
  "prof": profile-name,
  "cat": { * codec-id => codec },
  ? "layout": layout-state,
  ? "dct": { * tstr => bstr },
  ? "meta": any,
  "id": content-id,
  * extension-key => any,
}

codec = {
  "name": tstr,
  "cls": "encode" / "compress" / "encrypt",
  ? "dct": tstr,
  ? "p": any,
  * extension-key => any,
}

frame = {
  "t": frame-type,
  ? "x": [+ codec-id],
  ? "pub": any,
  ? "to": [+ recipient],
  ? "d": frame-payload / bstr,
  "prev": content-id,
  "id": content-id,
  ? "sig": cose-sign1,
  * extension-key => any,
}

frame-type = "terms" / "quads" / "reifies" / "annot" / "blob" /
             "suppress" / "snapshot" / "meta" / "index" / "opaque"

recipient = {
  "kid": tstr,
  ? "alg": tstr,
  * extension-key => any,
}

cose-sign1 = bstr

frame-payload = terms-payload / quads-payload / reifies-payload /
                annot-payload / blob-payload / suppress-payload /
                snapshot-payload / meta-payload / index-payload /
                opaque-node

terms-payload = [+ term]
term = {
  "k": 0 / 1 / 2 / 3,
  ? "v": tstr,
  ? "dt": term-id,
  ? "l": tstr,
  ? "dir": "ltr" / "rtl",
  ? "rf": term-id,
  * extension-key => any,
}

triple-row = [term-id, term-id, term-id]
quad-row = [term-id, term-id, term-id] /
           [term-id, term-id, term-id, term-id]

quads-payload = [+ quad-row]
reifies-payload = { * term-id => triple-row }
annot-payload = [+ triple-row]

blob-payload = bstr
blob-pub = {
  ? "mt": tstr,
  ? "rep": tstr,
  ? "digest": digest-ref,
  * extension-key => any,
}

suppress-payload = {
  "targets": [+ suppress-target],
  ? "reason": tstr,
  ? "by": term-id,
  * extension-key => any,
}

suppress-target = suppress-frame / suppress-blob / suppress-term /
                  suppress-quad / suppress-reifier
suppress-frame = { "kind": "frame", "id": digest-ref,
                   * extension-key => any }
suppress-blob = { "kind": "blob", "digest": digest-ref,
                  * extension-key => any }
suppress-term = { "kind": "term", "id": term-id,
                  * extension-key => any }
suppress-quad = { "kind": "quad", "q": quad-row,
                  * extension-key => any }
suppress-reifier = { "kind": "reifier", "id": term-id,
                     * extension-key => any }

snapshot-payload = {
  "terms": terms-payload,
  ? "quads": quads-payload,
  ? "reifies": reifies-payload,
  ? "annot": annot-payload,
  ? "blobs": { * digest-ref => bstr },
  ? "meta": any,
  * extension-key => any,
}

meta-payload = any

index-payload = {
  "count": uint,
  "head": content-id,
  ? "off": [+ uint],
  ? "ti": { * frame-type => [+ frame-index] },
  ? "dict": [+ frame-index],
  ? "mmr": content-id,
  * extension-key => any,
}

opaque-node = {
  "id": content-id,
  "type": frame-type,
  ? "pub": any,
  ? "to": [+ recipient],
  ? "sigstat": sig-status,
  "reason": opaque-reason,
  * extension-key => any,
}

sig-status = "none" / "valid" / "invalid" / "unverified"
opaque-reason = "unknown-codec" / "missing-key" / "damaged" /
                "unknown-frame-type"

diagnostic = {
  "code": diagnostic-code,
  "detail": tstr,
  ? "frame_index": frame-index,
  * extension-key => any,
}

diagnostic-code = "EmptyFile" / "TornAppendError" / "DamagedFrame" /
                  "BrokenChain" / "TruncatedLog" / "UnknownCodec" /
                  "MissingKey" / "KeyWrapFailed" /
                  "ConflictingReifier" / "IllTypedLiteral" /
                  "RecursionLimit" / "StreamableLayoutError" /
                  "PositionConstraint" / "ForwardReference" /
                  "SegmentBoundary" / "IndexMmrError" /
                  "UnknownFrameType" / tstr

profile-status = "core-required" / "optional-standard" /
                 "experimental" / "domain-specific"
profile-registration = {
  "name": profile-name,
  "status": profile-status,
  ? "owner": tstr,
  ? "spec": tstr,
  ? "namespace": [+ tstr],
  ? "requires": any,
  ? "validation": any,
  ? "security": any,
  * extension-key => any,
}
~~~

When `"x"` is present and non-empty, the frame `"d"` value is a byte
string carrying the encoded, compressed, or encrypted payload.  After
reversing the transform chain, those bytes decode to the
frame-type-specific payload above, except for `blob`, whose decoded
payload is raw bytes.

# Hash and Signature Preimages

All preimages use deterministic CBOR.  Unless a row explicitly excludes
a field, every key/value pair in the map participates, including unknown
extension keys.

| Subject | Bytes hashed or signed | Excluded fields |
|---|---|---|
| Header `"id"` | `BLAKE3-256(deterministic-CBOR(header without "id"))` | `"id"` only |
| Frame `"id"` | `BLAKE3-256(deterministic-CBOR(frame without "id" and "sig"))` | `"id"` and `"sig"` only |
| Frame `"prev"` link | The `"prev"` value participates in the frame id preimage | None beyond frame id exclusions |
| COSE frame signature | Detached `COSE_Sign1` over the frame id bytes | `"sig"` is excluded from frame id |
| Inline blob digest | `BLAKE3-256(decoded blob bytes)` | Frame envelope fields |
| External blob digest | `pub.digest` names external bytes | External bytes are absent from the frame |
| Index `"head"` | Content id of the last covered frame | Not applicable |
| Index `"mmr"` | MMR root over covered frame ids | The index frame itself unless covered by a later index |

Readers MUST include unknown extension keys when recomputing Header and
frame preimages.  Readers MUST NOT reject an otherwise valid map solely
because it contains an unknown extension key.  A re-authoring tool that
cannot preserve unknown extension keys MUST treat the operation as lossy
and MUST NOT claim old frame signatures remain attached to rewritten
frames.
