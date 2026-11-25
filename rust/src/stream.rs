// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The spec-owned `stream` vocabulary (GTS-SPEC §13.3) — mirror of
//! `packages/gts/src/gts/stream.py`.
//!
//! Constants only — the streaming-index terms a streamable segment leads with
//! (§3.3) and the compaction-provenance terms a streamable rewrite records
//! (§10.1). Like the `files` profile vocabulary (§13.2), the terms are
//! authored in the spec and carried as literal IRIs; no external ontology is
//! required.

/// The namespace prefix of the streaming and compaction vocabulary.
pub const STREAM_NS: &str = "https://w3id.org/gts/stream#";

// Streaming-index terms (§3.3): one Manifestation per promised blob.
/// The class of a manifestation, one for each promised blob.
pub const MANIFESTATION: &str = "https://w3id.org/gts/stream#Manifestation";
/// The predicate that gives the digest of a promised blob.
pub const DIGEST: &str = "https://w3id.org/gts/stream#digest";
/// The predicate that gives the media type of a promised blob.
pub const MEDIA_TYPE: &str = "https://w3id.org/gts/stream#mediaType";
/// The predicate that gives the byte size of a promised blob.
pub const SIZE: &str = "https://w3id.org/gts/stream#size";
/// The predicate that gives the role of a promised blob.
pub const ROLE: &str = "https://w3id.org/gts/stream#role";
/// The predicate that gives the order of a promised blob.
pub const ORDER: &str = "https://w3id.org/gts/stream#order";

// Compaction-provenance terms (§10.1).
/// The class of a compaction provenance record.
pub const COMPACTION: &str = "https://w3id.org/gts/stream#Compaction";
/// The predicate that names the agent that performed a compaction.
pub const AGENT: &str = "https://w3id.org/gts/stream#agent";
/// The predicate that gives the time of a compaction.
pub const TIMESTAMP: &str = "https://w3id.org/gts/stream#timestamp";
/// The predicate that names the source head that a compaction read.
pub const SOURCE_HEAD: &str = "https://w3id.org/gts/stream#sourceHead";
/// The predicate that names the sealed source of a compaction.
pub const SEALED_SOURCE: &str = "https://w3id.org/gts/stream#sealedSource";
/// The class of a detached signature record.
pub const DETACHED_SIGNATURE: &str = "https://w3id.org/gts/stream#DetachedSignature";
/// The predicate that names the frame a record refers to.
pub const SOURCE_FRAME: &str = "https://w3id.org/gts/stream#sourceFrame";
/// The predicate that carries the COSE signature bytes.
pub const COSE: &str = "https://w3id.org/gts/stream#cose";

/// The fixed compactor identity recorded as `stream:agent` — a constant so
/// the rewrite is byte-reproducible across engines (§14.1 determinism).
pub const COMPACT_AGENT: &str = "gts-compact";
