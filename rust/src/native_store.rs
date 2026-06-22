// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optional native in-memory RDF store.
//!
//! This module is compiled only with `--features native-store`. The store is a
//! small deterministic wrapper around the crate's native RDF dataset model; it
//! is not a SPARQL engine or persistent database.

use std::fmt;

use ciborium::value::Value;

use crate::model::{
    BlobEntry, Diagnostic, Graph, OpaqueNode, Signature, StreamableInfo, Suppression,
};
use crate::rdf::{
    to_rdf_quads, writer_from_rdf_dataset_with_profile, Dataset, RdfAdapterError, RdfQuad,
};
use crate::writer::Writer;

/// Error raised by the optional native store adapter.
#[derive(Debug)]
pub enum NativeStoreError {
    /// RDF model conversion failed before reaching the store.
    Rdf(RdfAdapterError),
}

impl fmt::Display for NativeStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rdf(err) => write!(f, "RDF adapter error: {err}"),
        }
    }
}

impl std::error::Error for NativeStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rdf(err) => Some(err),
        }
    }
}

impl From<RdfAdapterError> for NativeStoreError {
    fn from(value: RdfAdapterError) -> Self {
        Self::Rdf(value)
    }
}

/// GTS-specific state carried next to a pure RDF store projection.
///
/// The native store receives only RDF quads. Suppressions, signatures, blobs,
/// diagnostics, segment heads, profiles, and metadata stay in this sidecar so
/// callers can inspect them without encoding GTS implementation details into
/// the RDF graph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GtsSidecar {
    pub blob_meta: Vec<(String, Value)>,
    pub blobs: Vec<(String, BlobEntry)>,
    pub meta: Vec<(String, Value)>,
    pub suppressions: Vec<Suppression>,
    pub opaque: Vec<OpaqueNode>,
    pub signatures: Vec<Signature>,
    pub diagnostics: Vec<Diagnostic>,
    pub segment_heads: Vec<Vec<u8>>,
    pub segment_profiles: Vec<String>,
    pub segment_meta: Vec<Vec<(String, Value)>>,
    pub segment_streamable: Vec<StreamableInfo>,
}

impl GtsSidecar {
    /// Clone the GTS-only state from a folded graph.
    pub fn from_graph(graph: &Graph) -> Self {
        Self {
            blob_meta: graph.blob_meta.clone(),
            blobs: graph.blobs.clone(),
            meta: graph.meta.clone(),
            suppressions: graph.suppressions.clone(),
            opaque: graph.opaque.clone(),
            signatures: graph.signatures.clone(),
            diagnostics: graph.diagnostics.clone(),
            segment_heads: graph.segment_heads.clone(),
            segment_profiles: graph.segment_profiles.clone(),
            segment_meta: graph.segment_meta.clone(),
            segment_streamable: graph.segment_streamable.clone(),
        }
    }
}

/// Deterministic in-memory RDF store backed by native RDF quads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeStore {
    dataset: Dataset,
}

impl NativeStore {
    /// Create an empty native store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap an existing native RDF dataset.
    pub fn from_dataset(dataset: Dataset) -> Self {
        Self { dataset }
    }

    /// Consume the store and return its native RDF dataset.
    pub fn into_dataset(self) -> Dataset {
        self.dataset
    }

    /// Borrow the underlying native RDF dataset.
    pub fn as_dataset(&self) -> &Dataset {
        &self.dataset
    }

    /// Insert a quad. Returns `true` if the quad was not already present.
    pub fn insert(&mut self, quad: RdfQuad) -> bool {
        self.dataset.insert(quad)
    }

    /// Iterate over quads in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &RdfQuad> {
        self.dataset.iter()
    }

    /// Number of unique quads.
    pub fn len(&self) -> usize {
        self.dataset.len()
    }

    /// Whether the store contains no quads.
    pub fn is_empty(&self) -> bool {
        self.dataset.is_empty()
    }
}

impl<'a> IntoIterator for &'a NativeStore {
    type Item = &'a RdfQuad;
    type IntoIter = <&'a Dataset as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.dataset).into_iter()
    }
}

impl Extend<RdfQuad> for NativeStore {
    fn extend<T: IntoIterator<Item = RdfQuad>>(&mut self, iter: T) {
        for quad in iter {
            self.insert(quad);
        }
    }
}

/// Native store plus the GTS-only sidecar state split out of the source graph.
#[derive(Clone, Debug, PartialEq)]
pub struct StoreWithSidecar {
    pub store: NativeStore,
    pub sidecar: GtsSidecar,
}

/// Owning iterator over native RDF quads projected from a GTS graph.
pub struct IntoQuads {
    inner: std::vec::IntoIter<RdfQuad>,
}

impl Iterator for IntoQuads {
    type Item = RdfQuad;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.len()
    }
}

impl ExactSizeIterator for IntoQuads {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// Project a folded graph into native RDF quads without materializing N-Quads text.
pub fn graph_into_quads(graph: Graph) -> Result<IntoQuads, NativeStoreError> {
    let quads = to_rdf_quads(&graph)?;
    Ok(IntoQuads {
        inner: quads.into_iter(),
    })
}

/// Project a folded graph into native RDF quads and return GTS-only sidecar state.
pub fn graph_into_quads_with_sidecar(
    graph: Graph,
) -> Result<(IntoQuads, GtsSidecar), NativeStoreError> {
    let sidecar = GtsSidecar::from_graph(&graph);
    Ok((graph_into_quads(graph)?, sidecar))
}

/// Project a folded graph into a new in-memory native store.
pub fn graph_to_store(graph: &Graph) -> Result<NativeStore, NativeStoreError> {
    let mut store = NativeStore::new();
    for quad in to_rdf_quads(graph)? {
        store.insert(quad);
    }
    Ok(store)
}

/// Project a folded graph into a native store and split GTS-only state aside.
pub fn graph_to_store_with_sidecar(graph: Graph) -> Result<StoreWithSidecar, NativeStoreError> {
    let sidecar = GtsSidecar::from_graph(&graph);
    Ok(StoreWithSidecar {
        store: graph_to_store(&graph)?,
        sidecar,
    })
}

/// Build a deterministic GTS writer from a native RDF store.
///
/// GTS-only sidecar state is intentionally not folded back into the pure RDF
/// writer; callers that need signatures, blobs, suppressions, or diagnostics
/// should keep the [`GtsSidecar`] next to the store.
pub fn store_to_writer(store: &NativeStore, profile: &str) -> Result<Writer, NativeStoreError> {
    Ok(writer_from_rdf_dataset_with_profile(
        store.as_dataset(),
        profile,
    )?)
}

/// Build GTS bytes from a native RDF store.
pub fn store_to_gts_bytes(store: &NativeStore, profile: &str) -> Result<Vec<u8>, NativeStoreError> {
    Ok(store_to_writer(store, profile)?.to_bytes())
}
