---
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: CC-BY-4.0
type: Debt
id: debt:default
findings:
  - kind: coverage-item-unbound
    subject: gmeow_gts::PolicyConfigError
  - kind: coverage-item-unbound
    subject: gmeow_gts::ProfileFinding
  - kind: coverage-item-unbound
    subject: gmeow_gts::Severity
  - kind: coverage-item-unbound
    subject: gmeow_gts::Severity::as_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::SignatureTrust
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::from_json_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::from_path
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::from_yaml_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::is_pseudonymous_recipient
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::is_trusted
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::to_json_string
  - kind: coverage-item-unbound
    subject: gmeow_gts::TrustPolicy::to_yaml_string
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::Codec
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::CodecError
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::Decryptor
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::EncodeOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::check_payload_structure
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::decode_chain
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::decode_chain_with_decrypt
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::encode_chain
  - kind: coverage-item-unbound
    subject: gmeow_gts::codec::encode_chain_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::compact
  - kind: coverage-item-unbound
    subject: gmeow_gts::compact::CompactRefusedError
  - kind: coverage-item-unbound
    subject: gmeow_gts::compact::compact_streamable
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::Encrypt0Error
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::SigStatus
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::decrypt0
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::encrypt0
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::encrypt0_with_iv
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::recipient_kid
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::sign_id
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::signature_kid
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::unsupported_algorithm
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::verify_sig
  - kind: coverage-item-unbound
    subject: gmeow_gts::cose::verify_signatures
  - kind: coverage-item-unbound
    subject: gmeow_gts::db
  - kind: coverage-item-unbound
    subject: gmeow_gts::db::DbExportError
  - kind: coverage-item-unbound
    subject: gmeow_gts::db::to_duckdb
  - kind: coverage-item-unbound
    subject: gmeow_gts::db::to_parquet
  - kind: coverage-item-unbound
    subject: gmeow_gts::db::to_sqlite
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::DumpError
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::DumpError::kind
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::DumpErrorKind
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::DumpOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::DumpReport
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::dump_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::dumpdir::dump_path
  - kind: coverage-item-unbound
    subject: gmeow_gts::evaluate_profile_policy
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Claim
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::claims
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::path
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::recall
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::record_tool_call
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::revise
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::store
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::tool_calls
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Memory::verify
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::MemoryError
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::RecallOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::Result
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::RevisionOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::StoreOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::ToolCallOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::examples::agent_memory::ToolCallRecord
  - kind: coverage-item-unbound
    subject: gmeow_gts::files
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FileBlobSource
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FileEntry
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FileEntryKind
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FileEntryKind::as_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FileEntryKind::parse
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FilePaxRecord
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::FileXattr
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::UnpackOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::diff
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::pack
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::pack_entries_v2
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::pack_entries_v2_with_blob_paths
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::pack_to_writer
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::read_entries
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::read_file_entries
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::unpack
  - kind: coverage-item-unbound
    subject: gmeow_gts::files::unpack_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_nquads
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_nquads::NQuadsParseError
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_nquads::from_nquads
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_nquads::from_ntriples
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_okf
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_okf::FromOkfOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_okf::OkfParseError
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_okf::from_okf
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_okf::from_okf_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_tar
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_tar::FromTarOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_tar::from_tar
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_tar::from_tar_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_tar::from_tar_to_writer
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_trig
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_trig::TriGParseError
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_trig::from_trig
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_trig::from_turtle
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_yamlld
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_yamlld::YamlLdParseError
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_yamlld::from_json_ld
  - kind: coverage-item-unbound
    subject: gmeow_gts::from_yamlld::from_yaml_ld
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::MmrPeak
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::PROOF_SCHEMA
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::Proof
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::Proof::from_json
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::Proof::to_json
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::ProofSide
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::ProofStep
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::parse_hex_32
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::prove
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::prove_file
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::root
  - kind: coverage-item-unbound
    subject: gmeow_gts::mmr::verify_proof
  - kind: coverage-item-unbound
    subject: gmeow_gts::model
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::AnnotationRow
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::cached_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::decode
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::decoded_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::decoded_len
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::decoded_vec
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::is_lazy
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::BlobEntry::lazy
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Diagnostic
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::blob_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::blob_bytes_cloned
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::blob_entry
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::datatype_iri
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::decoded_blobs
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::into_quads
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::quad_terms
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::reifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::reifier_triples
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::set_blob
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::set_blob_entry
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::set_blob_meta
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::set_lazy_blob
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::set_meta
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::set_reifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::snapshot_payload
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Graph::triple_of
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::OpaqueNode
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Quad
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::QuadTerms
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::QuadTermsIter
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::RDF_DIR_LANG_STRING
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::RDF_LANG_STRING
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::ReifierRow
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Signature
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::StreamableInfo
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Suppression
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Term
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::TermKind
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::TermKind::from_wire
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::Triple3
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::XSD_STRING
  - kind: coverage-item-unbound
    subject: gmeow_gts::model::is_literal_direction
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::GtsSidecar
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::GtsSidecar::from_graph
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::IntoQuads
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::as_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::from_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::insert
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::into_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::is_empty
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::iter
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::len
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStore::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::NativeStoreError
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::StoreWithSidecar
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::graph_into_quads
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::graph_into_quads_with_sidecar
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::graph_to_store
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::graph_to_store_with_sidecar
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::store_to_gts_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::native_store::store_to_writer
  - kind: coverage-item-unbound
    subject: gmeow_gts::nested
  - kind: coverage-item-unbound
    subject: gmeow_gts::nested::GTS_MEDIA_TYPE
  - kind: coverage-item-unbound
    subject: gmeow_gts::nested::NestedReadResult
  - kind: coverage-item-unbound
    subject: gmeow_gts::nested::NestedReadResult::subgraph
  - kind: coverage-item-unbound
    subject: gmeow_gts::nested::read_nested
  - kind: coverage-item-unbound
    subject: gmeow_gts::nquads
  - kind: coverage-item-unbound
    subject: gmeow_gts::nquads::to_nquads
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::DEFAULT_BASE_IRI
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_BODY
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_DESCRIPTION
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_JSON
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_LINKS
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_LINK_OCCURRENCE
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_LINK_TEXT
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_NS
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_PATH
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_RESOURCE
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_TAG
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_TIMESTAMP
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_TITLE
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OKF_TYPE
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OkfExportError
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OkfExportOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::OkfExportReport
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::RDF_REIFIES
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::XSD_BOOLEAN
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::XSD_DATETIME
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::XSD_DECIMAL
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::XSD_INTEGER
  - kind: coverage-item-unbound
    subject: gmeow_gts::okf::to_okf
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpError
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpSigningKey
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpSigningKey::fingerprint
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpSigningKey::into_parts
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpSigningKey::kid
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpSigningKey::raw_public
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::OpenPgpSigningKey::signing_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::TransportKey
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::parse_secret_signing_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::openpgp::parse_transport_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::policy
  - kind: coverage-item-unbound
    subject: gmeow_gts::policy::DEFAULT_PSEUDONYMOUS_KID_PATTERN
  - kind: coverage-item-unbound
    subject: gmeow_gts::policy::FILES_NS
  - kind: coverage-item-unbound
    subject: gmeow_gts::policy::signature_trust
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::BaseDirection
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::BaseDirection::as_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::BlankNode
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::BlankNode::as_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::BlankNode::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Dataset::insert
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Dataset::is_empty
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Dataset::iter
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Dataset::len
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Dataset::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::ExportOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::GraphName
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::GraphName::is_default_graph
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Iri
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Iri::as_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Iri::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Literal
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Literal::new_directional_language_tagged_literal
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Literal::new_language_tagged_literal
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Literal::new_simple_literal
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::Literal::new_typed_literal
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::NamedOrBlankNode
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfAdapterError
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfAdapterError::detail
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfQuad
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfQuad::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfTerm
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfTriple
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::RdfTriple::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::from_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::from_rdf_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::from_rdf_dataset_with_profile
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::to_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::to_rdf_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::to_rdf_dataset_lossy
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::to_rdf_dataset_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::to_rdf_quads
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::to_rdf_quads_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::writer_from_rdf_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf::writer_from_rdf_dataset_with_profile
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::EventGraphSink
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::EventGraphSink::into_graph
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::RdfCodecError
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::RdfCodecError::detail
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::from_ntriples
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::from_rdf_xml
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::from_rdf_xml_with_base_iri
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::from_trig
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::from_turtle
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::graph_from_erased_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::graph_from_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_ntriples
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_ntriples_from_erased_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_ntriples_from_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_rdf_xml
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_rdf_xml_from_erased_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_rdf_xml_from_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_trig
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_trig_from_erased_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_trig_from_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_turtle
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_turtle_from_erased_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_codecs::to_turtle_from_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventAnnotation
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventDiagnostic
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError::detail
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError::invalid_source
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError::kind
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError::sink
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventError::triple_nesting_limit
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventErrorKind
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventLiteralDirection
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventLocation
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventQuad
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventReifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventScopeId
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventTerm
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventTermId
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventTermKind
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::EventTriple
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::GraphRdfEventSource
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::GraphRdfEventSource::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::GraphRdfEventSource::with_scope
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfDatasetVisitor
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfDatasetVisitor::annotation
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfDatasetVisitor::diagnostic
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfDatasetVisitor::quad
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfDatasetVisitor::reifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfDatasetVisitor::term
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::annotation
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::declares_before_reference
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::diagnostic
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::end_scope
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::finish
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::quad
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::reifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::start_scope
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::term
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSink::triple_term_nesting_limit
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSource
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSource::drive
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::RdfEventSource::drive_erased
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::ReaderRdfEventSource
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::ReaderRdfEventSource::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::ReaderRdfEventSource::with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::ReaderRdfEventSource::with_scope
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::read_to_rdf_events
  - kind: coverage-item-unbound
    subject: gmeow_gts::rdf_events::visit_dataset
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::ContentKeyResolver
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::FileSegments
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::ReadOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::ReadOptions::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::ReadOptions::with_content_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingReadResult
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::annotation
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::blob
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::diagnostic
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::opaque
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::quad
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::reifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::segment_head
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::signature
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::streamable_layout
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::suppression
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::StreamingSink::term
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::read
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::read_file_segments
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::read_to_sink
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::read_to_sink_from_reader
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::read_to_sink_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::reader::read_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::ByteRange
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::FrameInventory
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::Inventory
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::Inventory::has_problems
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::MissingResult
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::MissingStatus
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::SegmentInventory
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::heads_json
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::inventory
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::missing
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::missing_json
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::resume_after
  - kind: coverage-item-unbound
    subject: gmeow_gts::replication::segments_json
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::AGENT
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::COMPACTION
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::COMPACT_AGENT
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::COSE
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::DETACHED_SIGNATURE
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::DIGEST
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::MANIFESTATION
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::MEDIA_TYPE
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::ORDER
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::ROLE
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::SEALED_SOURCE
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::SIZE
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::SOURCE_FRAME
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::SOURCE_HEAD
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::STREAM_NS
  - kind: coverage-item-unbound
    subject: gmeow_gts::stream::TIMESTAMP
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar::TarCompression
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar::TarError
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar::TarError::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar::ToTarOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar::to_tar
  - kind: coverage-item-unbound
    subject: gmeow_gts::tar::to_tar_vec
  - kind: coverage-item-unbound
    subject: gmeow_gts::trig
  - kind: coverage-item-unbound
    subject: gmeow_gts::trig::to_trig
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::DeterministicUlidGenerator
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::DeterministicUlidGenerator::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::DeterministicUlidGenerator::next_ulid
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::DeterministicUlidGenerator::with_counter
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid::MAX_TIMESTAMP_MS
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid::as_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid::from_counter
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid::from_parts
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid::randomness
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::Ulid::timestamp_ms
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::UlidError
  - kind: coverage-item-unbound
    subject: gmeow_gts::ulid::UlidError::detail
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::DecodedHeader
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::Header
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::HeaderError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::HeaderWriteError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::ImportError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::Lineage
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::MalformedReason
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::decode_header
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::encode_header
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::Frame
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::Frame::check_decoded_payload
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::Frame::check_storage
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::Frame::computed_identifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::FrameError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::Signature
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::Storage
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::decode_frame
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::frame::encode_plain_frame
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::from_nquads
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::Direction
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::FrameIssue
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::NQuadsExportError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::ProjectionError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::RdfProjection
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::RdfProjection::empty_graphs
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::RdfProjection::quads
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::RdfProjection::terms
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::RdfProjection::to_nquads
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::Term
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::rdf::ValueIssue
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::ClosingFinding
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::CommitCheck
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::CommitReadError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::CommitReport
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::DecodedResync
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::FrameLink
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::FramingFailure
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::FramingReason
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::IndexCheck
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::IndexOutcome
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::IndexWireClaims
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::ItemKind
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::LinkState
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::Opener
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::PayloadCheck
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::PayloadDamage
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::PayloadRefusal
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::PayloadState
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::ReadCapacity
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::ReadStop
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::ResyncRefusal
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::UncommittedTail
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::WireFrame
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::WireItem
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::WireRead
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::WireRead::whole_rdf_projection
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::WireRead::whole_rdf_projection_with_value_limit
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::WireSegment
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::reader::read_wire_with_keys
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::HeldKey
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::HeldKey::identifier
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::HeldKeys
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::HeldKeys::as_slice
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::HeldKeys::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::SignatureReason
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::SignatureResult
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::signature::SignatureStatus
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::AttachmentWriteError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::Blob
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::IndexFold
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::IndexWriteError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentFinishError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriteError
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::append
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::append_attachment_regions
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::append_index
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::as_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::commit
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::finish_indexed
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::head
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::into_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::v2::writer::SegmentWriter::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::EmbeddedTransportKey
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::VerificationResult
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::VerifyOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::VerifyOptions::require_signatures
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::VerifyOptions::strict
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::VerifyOptions::trust_policy
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::VerifyOptions::with_armored_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::extract_transport_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::format_fingerprint
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::verify_file
  - kind: coverage-item-unbound
    subject: gmeow_gts::verify::verify_file_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::MAGIC
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::SELF_DESCRIBE_TAG
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::VERSION
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::blake3_256
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::canonical
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::content_id
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::deterministic
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::digest_str
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::encode
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::header_id
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::hex
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::iter_items
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::map_get
  - kind: coverage-item-unbound
    subject: gmeow_gts::wire::unwrap_header
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::BlobRow
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::DEFAULT_RSYNCABLE_THRESHOLD
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Encrypt0Options
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::FrameOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::SnapshotOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::SnapshotSigner
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::TermRemap
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_annot
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_blob
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_frame
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_frame_with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_index
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_index_with_mmr
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_meta
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_quads
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_reifies
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_suppress
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::add_terms
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::deterministic
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::from_store
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::head
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::new
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::sign_with
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::sign_with_openpgp_secret_key
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::to_bytes
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::with_layout
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::Writer::with_options
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::WriterError
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::WriterOptions
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::choose_snapshot_transform
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::deterministic_term_remap
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::digest_string
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::snapshot_from_graph
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::snapshot_payload
  - kind: coverage-item-unbound
    subject: gmeow_gts::writer::term_to_wire
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::ILL_TYPED_LITERAL_CODE
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::ILL_TYPED_LITERAL_META_KEY
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::IllTypedLiteral
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::IllTypedLiteral::diagnostic
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::XSD_NS
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::XsdLexicalStatus
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::XsdLexicalStatus::canonical
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::XsdLexicalStatus::is_invalid
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::XsdLexicalStatus::is_valid
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::XsdLexicalStatus::reason
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::annotate_ill_typed_literals
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::ill_typed_literals
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::ill_typed_literals_metadata
  - kind: coverage-item-unbound
    subject: gmeow_gts::xsd::validate_lexical
  - kind: coverage-item-unbound
    subject: gmeow_gts::yamlld
  - kind: coverage-item-unbound
    subject: gmeow_gts::yamlld::to_json_ld
  - kind: coverage-item-unbound
    subject: gmeow_gts::yamlld::to_json_ld_string
  - kind: coverage-item-unbound
    subject: gmeow_gts::yamlld::to_yaml_ld
  - kind: coverage-test-unbound
    subject: gmeow_gts::codec::tests::encoded_core_codecs_round_trip
  - kind: coverage-test-unbound
    subject: gmeow_gts::codec::tests::gzip_encoding_is_deterministic
  - kind: coverage-test-unbound
    subject: gmeow_gts::codec::tests::zstd_decode_accepts_payloads_over_former_safety_bound
  - kind: coverage-test-unbound
    subject: gmeow_gts::codec::tests::zstd_level_is_per_encode_chain
  - kind: coverage-test-unbound
    subject: gmeow_gts::codec::tests::zstd_rsyncable_decodes_concatenated_frames
  - kind: coverage-test-unbound
    subject: gmeow_gts::db::tests::load_script_decode_failure_stops_before_commit
  - kind: coverage-test-unbound
    subject: gmeow_gts::db::tests::load_script_decodes_lazy_blob_without_caching_it
  - kind: coverage-test-unbound
    subject: gmeow_gts::mmr::tests::proof_round_trip_and_tamper
  - kind: coverage-test-unbound
    subject: gmeow_gts::mmr::tests::roots_change_with_order
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::parses_frozen_vector
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::parses_secret_signing_key_with_default_fingerprint_kid
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::parses_secret_signing_key_with_kid_override
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::rejects_non_pgp
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_mpi_to_seed_left_pads_short_mpis
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_signing_key_rejects_bad_checksum
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_signing_key_rejects_encrypted_secret_packets
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_signing_key_rejects_public_armor
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_signing_key_rejects_public_secret_mismatch
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_signing_key_rejects_unsupported_algorithms_and_versions
  - kind: coverage-test-unbound
    subject: gmeow_gts::openpgp::tests::secret_signing_key_rejects_unsupported_trailer_structure
  - kind: coverage-test-unbound
    subject: gmeow_gts::replication::tests::missing_returns_exact_tail_range
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::agent_memory::rust_agent_memory_flow_is_append_only_and_verifyable
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::agent_memory::rust_agent_memory_validates_inputs_and_empty_packages
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::census::census_self_test_passes_every_detector_case
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::census::debt_record_is_canonical_and_the_sources_add_nothing_to_it
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::census::ledger_is_canonical_and_declares_no_helper_or_provider
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::census::production_clippy_receipt_covers_this_snapshot_and_passed
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::census::workspace_census_is_clean_with_the_representation_rules_required
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::cat_composes_clean_inputs_as_raw_concatenation
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::cat_refuses_a_composition_whose_suppressions_hide_everything
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::cat_refuses_a_damaged_input
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::cat_refuses_fewer_than_two_inputs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::cat_refuses_suppress_everything_composition
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::compact_is_reproducible_with_fixed_timestamp
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::compact_refuses_evidence_without_seal_then_seals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::compact_requires_streamable_flag
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::compact_verify_info_round_trip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::diff_refuses_symlink_entry
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::diff_reports_changes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::dump_files_profile_materializes_payload_once_by_default
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::dump_indexes_suppressed_payload_without_materializing_by_default
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::dump_metadata_only_skips_payload_materialization
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::dump_refuses_existing_destination_unless_forced
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::dump_writes_best_effort_output_for_damaged_archive
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::dump_writes_exploration_directory_for_minimal_archive
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::extract_key_matches_frozen_stdout
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::extract_key_missing_exits_1
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::extract_verifies_and_asserts_media_type
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::fold_emits_nquads
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::fold_exits_nonzero_on_diagnostics
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::info_reports_accretive_tail
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::info_reports_damage_healed_by_resync_as_not_clean
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::info_reports_v2_damage_and_refuses_unknown_generations
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::info_reports_v2_wire_frames_and_legacy_graphs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::localized_help_and_unknown_command
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::ls_lists_digest_size_and_media_type
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::okf_commands_report_feature_gate_when_disabled
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::pack_deduplicates_identical_content
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::pack_refuses_symlink_entry
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::pack_unpack_round_trip_bit_for_bit
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::prove_and_verify_proof_round_trip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::rdf_codecs_commands_report_feature_gate_when_disabled
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::replication_verbs_handle_streamable_unknown_and_torn_inputs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::replication_verbs_report_ranges_and_resume_bytes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::tar_commands_report_feature_gate_when_disabled
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::unpack_refuses_destination_symlink_escape
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::unpack_refuses_leaf_symlink_redirect
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::unpack_refuses_traversal
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::unpack_refuses_windows_style_paths
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_declared_files_profile_object_only_is_not_unused
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_flags_damage_with_exit_1
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_flags_undeclared_files_profile
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_flags_undeclared_files_profile_object_only
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_policy_file_trusts_did_style_signer
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_refuses_streamable_lie
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cli::verify_warns_on_stream_vocab_without_claim
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::conformance::compact_reproduces_the_frozen_25b_bytes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::conformance::corpus_matches_frozen_expectations
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::conformance::prefix_fold_streaming_property
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::cose::cose_sign1_vectors_round_trip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::duckdb_commands_require_feature
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_duckdb_exports_schema_and_rows
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_parquet_cleans_staging_dir_when_replace_fails
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_parquet_writes_non_empty_tables
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_sqlite_exports_schema_and_rows
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_sqlite_preserves_existing_output_when_blob_decode_fails
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_sqlite_preserves_existing_output_when_tool_fails
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::db::to_sqlite_refuses_damaged_input_before_writing
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::deterministic_writer_matches_frozen_vector
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::deterministic_writer_orders_nested_triple_terms_by_depth
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::deterministic_writer_reorders_equivalent_graphs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::graph_snapshot_payload_folds_like_deterministic_writer
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::graph_snapshot_payload_orders_quads_by_graph_then_spo
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::snapshot_blob_sort_uses_media_type_as_final_tie_breaker
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::snapshot_from_graph_emits_sorted_content_addressed_blobs_before_snapshot
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::snapshot_from_graph_signs_transport_key_and_snapshot_frame
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::deterministic_writer::snapshot_transform_threshold_only_rewrites_default_zstd
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::encrypt0::encrypt0_random_iv_round_trip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::encrypt0::encrypt0_vector_seals_and_opens
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::default_unpack_refuses_v2_links_and_special_files
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::dumpdir_surfaces_v2_fields_in_entries_jsonl
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::opt_in_unpack_materializes_in_destination_symlinks
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::symlink_opt_in_still_refuses_destination_escape
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::unpack_strips_setid_bits_by_default
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::v1_archive_reads_as_default_file_entries
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::v2_emission_is_deterministic_after_sorting
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::v2_entries_round_trip_all_metadata
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::v2_reader_accepts_duplicate_iri_term_ids
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::files_v2::v2_unpack_materializes_directories_and_regular_files
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::cli_from_nq_inverts_fold
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::cli_from_nq_writes_stdout_when_no_out_path
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::compact_blank_node_and_language_tag_delimiters_roundtrip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::directional_language_literals_roundtrip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::fold_roundtrips_through_from_nquads_for_graph_vectors
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::literals_lang_and_datatype_roundtrip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::named_graph_reifier_and_annotation_roundtrip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::nquads_import_preserves_multiple_reifiers_for_the_same_statement
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::parses_backspace_and_formfeed_escapes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::quoted_triple_adjacent_delimiters_roundtrip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::rejects_empty_blank_node_label_and_language_tag
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::rejects_forbidden_iri_characters
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::rejects_invalid_rdf_term_positions
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::rejects_malformed_nquads
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::rejects_malformed_unicode_escape_without_panicking
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::rejects_unknown_literal_escape
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_nquads::writer_allows_multiple_reifiers_for_the_same_statement
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::cli_from_trig_inverts_to_trig
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::fold_roundtrips_through_trig_for_graph_vectors
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::parses_graph_keyword_and_a_predicate
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::parses_predicate_object_shorthand_and_lists
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::parses_prefixes_graph_blocks_reifiers_and_literals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::pn_local_esc_keeps_escaped_delimiters_in_local_name
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::prefixed_names_stop_before_quoted_triple_close
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::rejects_forbidden_raw_iri_characters
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::rejects_malformed_or_unsupported_trig
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::resolves_base_iri_relative_references
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::to_trig_groups_named_graphs_and_keeps_reifiers
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::from_trig::trig_preserves_directional_language_literals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::graph_iter::into_quads_yields_existing_quad_rows
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::graph_iter::quad_terms_resolves_term_references_lazily
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::blob_access_decompresses_and_caches
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::blob_contract_preserves_inline_bytes_and_leaves_external_bytes_out_of_band
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::blob_deferred_until_access
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::blob_payload_of_concatenated_zstd_frames_decodes_to_the_end
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::blob_payload_that_is_not_whole_zstd_frames_is_one_opaque_node
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::damage_inside_a_block_with_valid_structure_fails_cleanly_on_first_read
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_decode_failure_raises_on_access
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_external_records_layout_iou
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_identity_codec_is_direct
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_lookup_and_iteration_helpers
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_meta_is_eager
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_multi_segment_union_does_not_decode
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_no_pub_digest_falls_back_to_eager_decode
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::lazy_blob_unknown_codec_degrades
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::zstd_frame_declaring_a_window_the_decoder_refuses_is_one_opaque_node
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::lazy_blobs::zstd_frame_naming_a_dictionary_is_one_opaque_node
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::mmr::detached_proof_rejects_tampered_path
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::mmr::detached_proof_verifies_without_source_file
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::mmr::negative_proof_fixture_fails
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::mmr::positive_proof_fixture_verifies
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::mmr::reader_diagnoses_bad_index_mmr
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::native_store::graph_into_quads_returns_gts_sidecar
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::native_store::graph_to_store_preserves_named_graphs_and_sidecar
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::native_store::sidecar_clones_all_gts_only_fields
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::native_store::store_to_writer_roundtrips_native_quads
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::nested::nested_recursion_security_vector_descriptor_is_still_present
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::nested::read_nested_charges_duplicate_nested_digest_once
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::nested::read_nested_exposes_subgraph_by_blob_digest
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::nested::read_nested_records_damaged_nested_payload
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::nested::read_nested_stops_at_decoded_size_budget
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::nested::read_nested_stops_at_recursion_limit
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::cli_from_okf_inverts_to_okf
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::from_okf_maps_frontmatter_body_and_links
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::frontmatterless_index_navigation_pages_are_ignored
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::frontmatterless_non_index_markdown_is_rejected
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::inline_body_profile_variant_exports_only_when_requested
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::link_extraction_handles_utf8_link_text
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::malformed_bundle_missing_type_is_rejected
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::okf_forward_roundtrip_restores_body_bytes_and_normalizes_tags
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::okf_reverse_roundtrip_preserves_folded_profile_graph
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf::to_okf_writes_unmapped_sidecar_instead_of_dropping_rdf
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf_corpus::okf_corpus_matches_folded_expectations
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf_corpus::okf_corpus_round_trips_through_export
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::okf_corpus::okf_corpus_unmapped_sidecar_is_pinned
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::evidence_profile_requires_signatures_and_head_commitment
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::evidence_sealed_source_satisfies_unsigned_source_commitment_policy
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::invalid_and_unverified_signature_statuses_are_policy_errors
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::opaque_profile_requires_pseudonymous_recipient_kids
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::profile_and_stream_vocabulary_findings_match_policy_codes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::profile_policy_security_vector_descriptor_is_still_present
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::trust_policy_loads_from_json
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::trust_policy_loads_from_yaml
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::trusted_signer_policy_rejects_valid_but_unauthorized_signer
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::policy::valid_signature_does_not_imply_trusted_signer_or_true_claim
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::generated_blank_node_labels_do_not_collide_with_explicit_labels
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::gts_reifier_projection_uses_native_rdf12_triple_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::native_rdf12_triple_terms_import_as_gts_reifiers
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::native_rdf_dataset_roundtrips_directional_literals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::native_rdf_dataset_roundtrips_through_gts
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::native_rdf_import_keeps_every_reifier_binding
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::native_rdf_lexical_validators_reject_unsafe_values
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::strict_export_refuses_unrepresentable_quoted_triple_positions
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf::strict_export_rejects_quoted_triple_graph_names
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::cli_turtle_and_feature_trig_imports_roundtrip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::event_graph_materialization_surfaces_ill_typed_xsd_diagnostics
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::native_codecs_roundtrip_long_private_use_language_subtags
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::ntriples_import_preserves_and_flags_ill_typed_xsd_literals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::ntriples_isomorphism_helper_accepts_renamed_blank_nodes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::ntriples_parser_accepts_w3c_rdf12_positive_syntax_suite
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::ntriples_parser_accepts_w3c_syntax_cases_and_rdf12_triple_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::ntriples_parser_rejects_w3c_rdf12_negative_syntax_suite
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::ntriples_serialization_roundtrips_through_event_source
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_accepts_core_w3c_rdf_xml_shapes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_accepts_w3c_rdf12_triple_terms_and_annotations
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_passes_w3c_suite_when_fixture_root_is_set
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_preserves_empty_property_attributes_on_resource_objects
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_rejects_w3c_rdf12_bad_triple_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_resolves_rdf_id_against_base_without_fragment
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_resolves_text_entity_references
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_serialization_roundtrips_unicode_predicate_local_names
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::rdf_xml_serialization_uses_event_source_and_rejects_named_graphs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::trig_parser_accepts_named_graphs_and_rdf12_triple_forms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::trig_parser_allows_comment_only_empty_structures_inside_rdf12_triples
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::trig_parser_rejects_non_empty_structures_inside_rdf12_triples_after_comments
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::trig_serialization_roundtrips_through_event_source
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::turtle_parser_accepts_bare_numeric_boolean_and_long_string_literals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::turtle_parser_accepts_prefixed_names_with_internal_dots
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::turtle_parser_accepts_shared_turtle_grammar
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::turtle_parser_expands_rdf12_reifying_triples_and_annotations
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_codecs::turtle_serialization_uses_event_source_and_rejects_named_graphs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_events::dataset_visitor_is_infallible_folded_graph_visitor
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_events::graph_source_drives_generic_and_erased_sinks
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_events::lenient_sink_accepts_forward_reifier_order
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_events::reader_source_drives_events_from_gts_bytes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_events::source_rejects_out_of_range_quad_reference
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_events::strict_sink_gets_declarations_before_references
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_feature_gate::default_features::optional_adapters_are_not_enabled_by_default
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_feature_gate::native_store_feature_enables_adapter_module
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_feature_gate::rdf_codecs_feature_enables_rdf_text_codec_modules
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_feature_gate::rdf_feature_enables_adapter_module
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::rdf_feature_gate::yaml_ld_feature_enables_codec_modules
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::conflicting_reifier_is_only_the_legacy_indirect_term
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::forward_referencing_tt_is_diagnosed_and_dropped
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::identical_reifier_rows_still_collapse
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::indirect_self_reaching_quoted_triple_reifier_degrades
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::multi_segment_union_preserves_literal_datatype_mapping
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::public_reader_reports_malformed_input_diagnostics_without_panicking
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::reader_rejects_quoted_triple_in_reifier_position
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::reifies_frame_is_multi_valued
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::self_describing_triple_terms_sharing_a_reifier_stay_distinct
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::self_reaching_quoted_triple_reifier_degrades_and_keeps_its_row
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::tt_with_a_literal_predicate_is_a_position_constraint
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::union_keeps_distinct_triple_terms_that_share_a_reifier
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::unreified_triple_term_survives_a_round_trip
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::reader::wire_constructed_self_reaching_reifier_stays_total
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::signed::signature_algorithm_dispatch_vectors
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::signed::signed_file_vector
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::signed::writer_signs_with_openpgp_secret_key_default_fingerprint_kid
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::signed::writer_signs_with_openpgp_secret_key_override_kid
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::streaming::streaming_sink_events_match_segment_folds_for_corpus
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::streaming::streaming_sink_expected_head_mismatch_matches_full_reader
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::streaming::streaming_sink_final_state_matches_full_reader_for_corpus
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::streaming::streaming_sink_from_reader_matches_slice_api_for_corpus
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::streaming::streaming_sink_pre_segment_mode_matches_full_reader
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::allow_special_preserves_special_entry_metadata
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::cli_round_trips_tar_with_owner_pax_links_and_dedup
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::compression_variants_round_trip_through_the_cli
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::from_tar_and_to_tar_are_pipe_friendly
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::from_tar_refuses_links_special_files_and_traversal_by_default
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::from_tar_to_writer_streams_large_payload_writes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::pack_to_writer_streams_large_file_payloads
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::tar_verb_creates_and_extracts_compressed_tar_archives
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::tar_verb_creates_lists_extracts_and_diffs_gts_archives
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::tar_verb_enforces_symlink_opt_in_on_extract
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::to_tar_refuses_device_numbers_outside_tar_range
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar::to_tar_refuses_link_entries_without_targets
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::tar_corpus::tar_corpus_matches_pinned_expectations
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::ulid::deterministic_ulids_order_by_timestamp_then_counter
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::ulid::ulid_parse_roundtrips_and_rejects_invalid_input
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::ulid::ulid_renders_canonical_crockford_base32
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_attachments::blob_members_promote_shared_units_and_preserve_empty_blobs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_attachments::invalid_attachment_inputs_leave_the_committed_prefix_unchanged
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_attachments::packs_cut_on_body_length_at_the_published_boundary
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_attachments::value_and_blob_regions_store_shared_units_separately
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_close_segment::closing_a_buffered_indexed_segment_reproduces_the_corpus
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_close_segment::finishing_requires_a_closing_index_and_refuses_existing_parity
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_close_segment::reached_full_spans_follow_the_frame_that_crosses_them
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::frame_identifiers_and_plain_payloads_match_the_published_chain
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::later_registry_keeps_unknown_content_and_storage
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::plain_writer_preserves_payload_length_boundaries
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::plain_writer_requires_one_deterministic_content_record
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::report_the_first_envelope_violation
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::segment_writer_reproduces_commit_prefixes_in_the_corpus
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::stored_checks_judge_compressed_bytes_independently
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::truncated_frame_is_bounded_by_the_input
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_frame::unknown_content_judges_the_head_before_the_type
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_from_nquads::deep_triple_objects_use_earlier_ids_without_recursive_parsing
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_from_nquads::imports_rdf_projection_as_v2_quads_with_typed_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_from_nquads::keeps_dictionary_ids_across_frames_and_resets_front_coding_at_restarts
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_from_nquads::preserves_carriage_return_separated_statements_and_escaped_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_from_nquads::refuses_triple_subjects_outside_the_v2_position_rules
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_header::header_outcomes_and_identifiers_match_p1
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_header::truncated_prefixes_produce_header_errors
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_header::writer_refuses_a_registry_it_does_not_implement
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_index_writer::cannot_index_a_row_payload_with_inconsistent_counts
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_index_writer::counts_written_predicates_when_the_fold_has_no_digest
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_index_writer::indexes_match_published_rows_and_pack_directories
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_index_writer::transparent_frames_are_absent_from_index_claims
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_index_writer::writes_the_complete_canonical_empty_blob_vector
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::annotation_duplicates_use_storage_identity_inside_triples
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::fold_cli_exports_v2_and_continues_reading_v1
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::pack_value_can_use_later_members_and_share_reference_text
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_checks_inline_types_before_column_counts
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_deduplicates_equivalent_lines_and_scopes_blanks
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_generates_inline_typed_lexical_forms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_keeps_nested_object_and_empty_graph
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_preserves_text_and_refuses_unreadable_content
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_refuses_bad_storage_check_with_verified_bytes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::projection_uses_first_verified_earlier_value
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::recipe_charges_can_refuse_later_literal_values
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_rdf_projection::reference_columns_and_language_table_precede_value_lookup
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_commit_needs_a_link_even_when_both_claims_hold
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_composite_with_only_one_component_key_held_is_refused
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_frame_with_a_malformed_envelope_but_signatures_in_grammar_has_results
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_frame_without_an_effective_identifier_creates_no_commit_leaf
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_key_or_value_of_the_wrong_length_is_value_length_before_any_algorithm_check
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_malformed_payload_head_does_not_hide_the_stored_checksum
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_read_without_keys_reports_key_not_held_within_the_stored_bytes_capacity
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_removed_witness_does_not_heal_the_preceding_frame
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_scanned_opener_can_establish_the_generation_after_foreign_bytes
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_total_decoded_refusal_latches_even_for_a_later_empty_payload
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_walked_resync_decides_each_nested_field_at_its_start
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_walked_resync_opens_a_segment_using_the_computed_identifier
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::a_walked_resync_reports_the_byte_cap_where_it_is_reached
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::algorithm_and_held_key_steps_come_before_the_lengths
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::an_index_after_an_unknown_critical_frame_still_checks_the_wire_prefix
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::an_index_with_bad_payload_grammar_is_not_compared
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::charges_scan_candidates_after_place_and_before_tail_grammar
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::checks_payload_independently_of_storage_and_link_state
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::commit_prefix_checks_count_and_root_independently_of_transparent_frames
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::composite_post_quantum_half_is_ml_dsa_verify_with_the_label_context
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::composite_reports_the_classical_half_over_the_representative_before_the_post_quantum_half
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::covering_a_frame_without_a_type_rejects_the_type_runs
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::ed25519_equation_decodes_the_key_strictly_and_is_cofactorless
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::ed25519_strict_reports_the_first_failing_check_in_policy_order
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::frames_without_a_subject_or_in_grammar_carry_no_signature_result
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::held_keys_are_a_set_whatever_order_or_repetition_the_caller_gives
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::honest_signatures_of_both_algorithms_are_valid
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::index_wire_claims_are_checked_independently_at_each_index
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::items_held_before_the_scan_decision_count_toward_the_limit
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::locator_claim_names_the_last_well_typed_index_frame_type
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::malformed_mark_shape_precedes_a_later_mark_limit
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::malformed_or_unread_commit_content_checks_no_claims
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::misplaced_closing_items_report_placement_before_grammar_or_claims
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::offsets_check_every_retained_item_and_report_count_or_length_disagreement
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::repairs_a_link_only_with_the_direct_admitted_witness
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::resync_distinguishes_a_damaged_region_from_a_torn_prefix
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::the_complete_item_before_lost_framing_keeps_its_wire_extent
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::the_subject_binds_the_opener_and_the_effective_identifier_of_the_frame
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::v2_wire_reader::violations_before_a_resync_byte_limit_keep_their_first_offset
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::extract_transport_key_round_trips_from_graph_meta
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::format_fingerprint_groups_openpgp_hex
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::trusted_policy_counts_trusted_valid_signatures
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::verify_signed_file_with_embedded_key
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::verify_tampered_file_fails
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::verify_unsigned_file_fails_when_required
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::verify_unsigned_file_is_ok_when_signatures_are_not_required
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::verify_with_trusted_key_uses_openpgp_fingerprint_as_kid
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::verify::verify_with_wrong_out_of_band_key_reports_unverified
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::workplan::capability_documents_and_changes_match_the_sources
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::writer_transforms::encrypted_writer_frame_is_opaque_without_key_and_clear_with_key
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::writer_transforms::encrypted_writer_frame_with_wrong_key_degrades_to_missing_key
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::writer_transforms::writer_applies_zstd_level_per_frame
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::writer_transforms::writer_authors_compressed_frames_that_fold_cleanly
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::writer_transforms::writer_options_support_header_metadata_magic_toggle_and_custom_catalog
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::writer_transforms::writer_rejects_zstd_level_without_zstd_transform
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::cli_yaml_ld_inverts_fold
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::compact_context_authoring_shape_imports_annotations_and_scalars
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::json_ld_uses_the_same_intermediate_and_round_trips
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::scoped_contexts_expand_node_value_and_annotation_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::yaml_ld_preserves_directional_language_literals
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::yaml_ld_preserves_reifier_annotations_as_folded_state
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::yaml_ld_rejects_language_literals_with_type_or_non_string_values
  - kind: coverage-test-unbound
    subject: gmeow_gts::test_crate::yamlld::yaml_ld_roundtrips_the_graph_vector_corpus
  - kind: coverage-test-unbound
    subject: gmeow_gts::v2::import::tests::imported_literal_forms_share_a_value_and_charge_pack_frames
  - kind: coverage-test-unbound
    subject: gmeow_gts::v2::import::tests::singleton_storage_choice_matches_encoded_payload_at_the_limit
  - kind: coverage-test-unbound
    subject: gmeow_gts::xsd::tests::detects_invalid_recognized_xsd_lexicals_without_rejecting_terms
  - kind: coverage-test-unbound
    subject: gmeow_gts::xsd::tests::leaves_unrecognized_datatypes_unsupported
  - kind: coverage-test-unbound
    subject: gmeow_gts::xsd::tests::validates_and_canonicalizes_common_xsd_lexicals
lines:
  - item: gmeow_gts::bin_crate::gts::PROFILE_VOCABS
    orphans: 3
  - item: gmeow_gts::bin_crate::gts::VerifyArgs
    orphans: 6
  - item: gmeow_gts::bin_crate::gts::all_quads_suppressed
    orphans: 23
  - item: gmeow_gts::bin_crate::gts::blob_mt
    orphans: 15
  - item: gmeow_gts::bin_crate::gts::cmd_cat
    orphans: 79
  - item: gmeow_gts::bin_crate::gts::cmd_compact
    orphans: 74
  - item: gmeow_gts::bin_crate::gts::cmd_diff
    orphans: 35
  - item: gmeow_gts::bin_crate::gts::cmd_duckdb_disabled
    orphans: 8
  - item: gmeow_gts::bin_crate::gts::cmd_dump
    orphans: 63
  - item: gmeow_gts::bin_crate::gts::cmd_extract
    orphans: 91
  - item: gmeow_gts::bin_crate::gts::cmd_extract_key
    orphans: 28
  - item: gmeow_gts::bin_crate::gts::cmd_fold
    orphans: 41
  - item: gmeow_gts::bin_crate::gts::cmd_from_nq
    orphans: 61
  - item: gmeow_gts::bin_crate::gts::cmd_from_nt
    orphans: 61
  - item: gmeow_gts::bin_crate::gts::cmd_from_okf
    orphans: 54
  - item: gmeow_gts::bin_crate::gts::cmd_from_rdfxml
    orphans: 61
  - item: gmeow_gts::bin_crate::gts::cmd_from_trig
    orphans: 76
  - item: gmeow_gts::bin_crate::gts::cmd_from_turtle
    orphans: 61
  - item: gmeow_gts::bin_crate::gts::cmd_from_yaml_ld
    orphans: 61
  - item: gmeow_gts::bin_crate::gts::cmd_heads
    orphans: 16
  - item: gmeow_gts::bin_crate::gts::cmd_info
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::cmd_ls
    orphans: 26
  - item: gmeow_gts::bin_crate::gts::cmd_missing
    orphans: 26
  - item: gmeow_gts::bin_crate::gts::cmd_okf_disabled
    orphans: 5
  - item: gmeow_gts::bin_crate::gts::cmd_pack
    orphans: 41
  - item: gmeow_gts::bin_crate::gts::cmd_prove
    orphans: 26
  - item: gmeow_gts::bin_crate::gts::cmd_rdf_codecs_disabled
    orphans: 8
  - item: gmeow_gts::bin_crate::gts::cmd_resume
    orphans: 32
  - item: gmeow_gts::bin_crate::gts::cmd_segments
    orphans: 16
  - item: gmeow_gts::bin_crate::gts::cmd_tar_disabled
    orphans: 5
  - item: gmeow_gts::bin_crate::gts::cmd_to_duckdb
    orphans: 17
  - item: gmeow_gts::bin_crate::gts::cmd_to_nt
    orphans: 25
  - item: gmeow_gts::bin_crate::gts::cmd_to_okf
    orphans: 57
  - item: gmeow_gts::bin_crate::gts::cmd_to_parquet
    orphans: 22
  - item: gmeow_gts::bin_crate::gts::cmd_to_rdfxml
    orphans: 25
  - item: gmeow_gts::bin_crate::gts::cmd_to_sqlite
    orphans: 16
  - item: gmeow_gts::bin_crate::gts::cmd_to_trig
    orphans: 34
  - item: gmeow_gts::bin_crate::gts::cmd_to_turtle
    orphans: 25
  - item: gmeow_gts::bin_crate::gts::cmd_to_yaml_ld
    orphans: 20
  - item: gmeow_gts::bin_crate::gts::cmd_unpack
    orphans: 48
  - item: gmeow_gts::bin_crate::gts::cmd_verify
    orphans: 27
  - item: gmeow_gts::bin_crate::gts::cmd_verify_proof
    orphans: 33
  - item: gmeow_gts::bin_crate::gts::cmd_yaml_ld_disabled
    orphans: 7
  - item: gmeow_gts::bin_crate::gts::collect_suppressed
    orphans: 39
  - item: gmeow_gts::bin_crate::gts::export_graph
    orphans: 12
  - item: gmeow_gts::bin_crate::gts::has_problems
    orphans: 3
  - item: gmeow_gts::bin_crate::gts::i18n::<impl fmt::Display for LocalizedUsage>
    orphans: 2
  - item: gmeow_gts::bin_crate::gts::i18n::CliLocale
    orphans: 6
  - item: gmeow_gts::bin_crate::gts::i18n::LocalizedUsage
    orphans: 1
  - item: gmeow_gts::bin_crate::gts::i18n::LocalizedUsage::fmt
    orphans: 3
  - item: gmeow_gts::bin_crate::gts::i18n::USAGE
    orphans: 1
  - item: gmeow_gts::bin_crate::gts::i18n::USAGE_EN
    orphans: 127
  - item: gmeow_gts::bin_crate::gts::i18n::USAGE_FR_CA
    orphans: 33
  - item: gmeow_gts::bin_crate::gts::i18n::USAGE_ZH_HANS
    orphans: 33
  - item: gmeow_gts::bin_crate::gts::i18n::cli_locale
    orphans: 10
  - item: gmeow_gts::bin_crate::gts::i18n::locale_from
    orphans: 11
  - item: gmeow_gts::bin_crate::gts::i18n::rdf_codecs_usage
    orphans: 29
  - item: gmeow_gts::bin_crate::gts::i18n::unknown_command_message
    orphans: 7
  - item: gmeow_gts::bin_crate::gts::i18n::usage_text
    orphans: 64
  - item: gmeow_gts::bin_crate::gts::info_v2::FRAME_NAMES
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::info_v2::frame_name
    orphans: 5
  - item: gmeow_gts::bin_crate::gts::info_v2::inspect
    orphans: 47
  - item: gmeow_gts::bin_crate::gts::info_v2::print_damage
    orphans: 8
  - item: gmeow_gts::bin_crate::gts::info_v2::print_frame_counts
    orphans: 20
  - item: gmeow_gts::bin_crate::gts::info_v2::print_frame_problems
    orphans: 28
  - item: gmeow_gts::bin_crate::gts::info_v2::print_items
    orphans: 31
  - item: gmeow_gts::bin_crate::gts::info_v2::print_opener
    orphans: 48
  - item: gmeow_gts::bin_crate::gts::info_v2::print_refusal
    orphans: 8
  - item: gmeow_gts::bin_crate::gts::info_v2::print_resync_claim
    orphans: 19
  - item: gmeow_gts::bin_crate::gts::info_v2::print_segment
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::info_v2::print_stop
    orphans: 12
  - item: gmeow_gts::bin_crate::gts::load
    orphans: 6
  - item: gmeow_gts::bin_crate::gts::load_policy
    orphans: 14
  - item: gmeow_gts::bin_crate::gts::main
    orphans: 80
  - item: gmeow_gts::bin_crate::gts::namespace
    orphans: 5
  - item: gmeow_gts::bin_crate::gts::normalize_digest
    orphans: 7
  - item: gmeow_gts::bin_crate::gts::now_utc_iso
    orphans: 7
  - item: gmeow_gts::bin_crate::gts::output::stderr_line
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::output::stdout_line
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::output::stdout_text
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::parse_key
    orphans: 14
  - item: gmeow_gts::bin_crate::gts::parse_verify_args
    orphans: 37
  - item: gmeow_gts::bin_crate::gts::print_ledger
    orphans: 72
  - item: gmeow_gts::bin_crate::gts::profile_check
    orphans: 28
  - item: gmeow_gts::bin_crate::gts::stream_vocab_check
    orphans: 33
  - item: gmeow_gts::bin_crate::gts::suppressed_blob_digests
    orphans: 27
  - item: gmeow_gts::bin_crate::gts::tar_cli::ArchiveKind
    orphans: 5
  - item: gmeow_gts::bin_crate::gts::tar_cli::TarCliArgs
    orphans: 10
  - item: gmeow_gts::bin_crate::gts::tar_cli::TarOperation
    orphans: 7
  - item: gmeow_gts::bin_crate::gts::tar_cli::archive_kind
    orphans: 7
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_from_tar
    orphans: 45
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_tar
    orphans: 19
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_tar_create
    orphans: 49
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_tar_diff
    orphans: 30
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_tar_extract
    orphans: 22
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_tar_list
    orphans: 13
  - item: gmeow_gts::bin_crate::gts::tar_cli::cmd_to_tar
    orphans: 45
  - item: gmeow_gts::bin_crate::gts::tar_cli::graph_from_clean_bytes
    orphans: 11
  - item: gmeow_gts::bin_crate::gts::tar_cli::infer_tar_output_compression
    orphans: 15
  - item: gmeow_gts::bin_crate::gts::tar_cli::parse_tar_cli_args
    orphans: 72
  - item: gmeow_gts::bin_crate::gts::tar_cli::parse_tar_short_option
    orphans: 40
  - item: gmeow_gts::bin_crate::gts::tar_cli::path_under_directory
    orphans: 10
  - item: gmeow_gts::bin_crate::gts::tar_cli::print_files_profile_listing
    orphans: 29
  - item: gmeow_gts::bin_crate::gts::tar_cli::read_archive_graph_for_tar
    orphans: 18
  - item: gmeow_gts::bin_crate::gts::tar_cli::read_tar_as_gts_bytes
    orphans: 26
  - item: gmeow_gts::bin_crate::gts::tar_cli::resolve_tar_sources
    orphans: 6
  - item: gmeow_gts::bin_crate::gts::tar_cli::set_tar_compression
    orphans: 10
  - item: gmeow_gts::bin_crate::gts::tar_cli::set_tar_operation
    orphans: 10
  - item: gmeow_gts::bin_crate::gts::tar_cli::tar_diff_directory
    orphans: 8
  - item: gmeow_gts::bin_crate::gts::tar_cli::write_from_tar_output
    orphans: 34
  - item: gmeow_gts::bin_crate::gts::tar_cli::write_tar_output
    orphans: 32
  - item: gmeow_gts::bin_crate::gts::target_idx
    orphans: 14
  - item: gmeow_gts::bin_crate::gts::target_kind
    orphans: 12
  - item: gmeow_gts::bin_crate::gts::term_iri_value
    orphans: 8
  - item: gmeow_gts::bin_crate::gts::usage_error
    orphans: 9
  - item: gmeow_gts::bin_crate::gts::used_vocabs
    orphans: 21
  - item: gmeow_gts::bin_crate::gts::verify_cose_signatures
    orphans: 25
  - item: gmeow_gts::bin_crate::gts::verify_segments
    orphans: 38
  - item: gmeow_gts::codec::<impl fmt::Display for CodecError>
    orphans: 2
  - item: gmeow_gts::codec::<impl std::error::Error for CodecError>
    orphans: 1
  - item: gmeow_gts::codec::Codec
    orphans: 8
  - item: gmeow_gts::codec::CodecError
    orphans: 14
  - item: gmeow_gts::codec::CodecError::fmt
    orphans: 6
  - item: gmeow_gts::codec::DEFAULT_ZSTD_LEVEL
    orphans: 1
  - item: gmeow_gts::codec::Decryptor
    orphans: 2
  - item: gmeow_gts::codec::EncodeOptions
    orphans: 8
  - item: gmeow_gts::codec::RSYNCABLE_BLOCK_SIZE
    orphans: 1
  - item: gmeow_gts::codec::ZSTD_MAGIC
    orphans: 1
  - item: gmeow_gts::codec::ZSTD_MAX_BLOCK
    orphans: 1
  - item: gmeow_gts::codec::ZSTD_MAX_WINDOW
    orphans: 3
  - item: gmeow_gts::codec::ZSTD_SKIPPABLE_MAGIC
    orphans: 1
  - item: gmeow_gts::codec::check_payload_structure
    orphans: 25
  - item: gmeow_gts::codec::decode_chain
    orphans: 13
  - item: gmeow_gts::codec::decode_chain_with_decrypt
    orphans: 30
  - item: gmeow_gts::codec::decode_one
    orphans: 45
  - item: gmeow_gts::codec::encode_chain
    orphans: 8
  - item: gmeow_gts::codec::encode_chain_with_options
    orphans: 28
  - item: gmeow_gts::codec::encode_one
    orphans: 22
  - item: gmeow_gts::codec::encode_zstd
    orphans: 3
  - item: gmeow_gts::codec::encode_zstd_rsyncable
    orphans: 7
  - item: gmeow_gts::codec::tests::encoded_core_codecs_round_trip
    orphans: 20
  - item: gmeow_gts::codec::tests::gzip_encoding_is_deterministic
    orphans: 8
  - item: gmeow_gts::codec::tests::zstd_decode_accepts_payloads_over_former_safety_bound
    orphans: 15
  - item: gmeow_gts::codec::tests::zstd_level_is_per_encode_chain
    orphans: 37
  - item: gmeow_gts::codec::tests::zstd_rsyncable_decodes_concatenated_frames
    orphans: 21
  - item: gmeow_gts::codec::walk_zstd_frames
    orphans: 90
  - item: gmeow_gts::codec::zstd_level
    orphans: 3
  - item: gmeow_gts::compact::<impl GraphBuilder>
    orphans: 4
  - item: gmeow_gts::compact::<impl std::error::Error for CompactRefusedError>
    orphans: 1
  - item: gmeow_gts::compact::<impl std::fmt::Display for CompactRefusedError>
    orphans: 2
  - item: gmeow_gts::compact::CompactRefusedError
    orphans: 3
  - item: gmeow_gts::compact::CompactRefusedError::fmt
    orphans: 3
  - item: gmeow_gts::compact::GraphBuilder
    orphans: 6
  - item: gmeow_gts::compact::GraphBuilder::add
    orphans: 12
  - item: gmeow_gts::compact::GraphBuilder::literal
    orphans: 12
  - item: gmeow_gts::compact::GraphBuilder::quad
    orphans: 3
  - item: gmeow_gts::compact::RDF_TYPE
    orphans: 1
  - item: gmeow_gts::compact::XSD_DATETIME
    orphans: 1
  - item: gmeow_gts::compact::XSD_INTEGER
    orphans: 1
  - item: gmeow_gts::compact::base64url_unpadded
    orphans: 19
  - item: gmeow_gts::compact::blob_bytes
    orphans: 15
  - item: gmeow_gts::compact::blob_decode_refused
    orphans: 3
  - item: gmeow_gts::compact::blob_decoded_len
    orphans: 12
  - item: gmeow_gts::compact::blob_meta_text
    orphans: 14
  - item: gmeow_gts::compact::compact_streamable
    orphans: 117
  - item: gmeow_gts::compact::refusal_gate
    orphans: 61
  - item: gmeow_gts::compact::refuse
    orphans: 3
  - item: gmeow_gts::compact::shift_term
    orphans: 12
  - item: gmeow_gts::compact::shifted_suppressions
    orphans: 53
  - item: gmeow_gts::compact::streaming_index
    orphans: 87
  - item: gmeow_gts::compact::target_text
    orphans: 8
  - item: gmeow_gts::compact::value_idx
    orphans: 7
  - item: gmeow_gts::cose::<impl std::error::Error for Encrypt0Error>
    orphans: 1
  - item: gmeow_gts::cose::<impl std::fmt::Display for Encrypt0Error>
    orphans: 2
  - item: gmeow_gts::cose::ALG
    orphans: 1
  - item: gmeow_gts::cose::ALG_A256GCM
    orphans: 1
  - item: gmeow_gts::cose::ALG_EDDSA
    orphans: 1
  - item: gmeow_gts::cose::Algorithm
    orphans: 9
  - item: gmeow_gts::cose::Encrypt0Error
    orphans: 10
  - item: gmeow_gts::cose::Encrypt0Error::fmt
    orphans: 8
  - item: gmeow_gts::cose::Encrypt0Parts
    orphans: 7
  - item: gmeow_gts::cose::IV
    orphans: 1
  - item: gmeow_gts::cose::KID
    orphans: 1
  - item: gmeow_gts::cose::MAX_HEADER_DEPTH
    orphans: 2
  - item: gmeow_gts::cose::SigStatus
    orphans: 11
  - item: gmeow_gts::cose::Sign1
    orphans: 7
  - item: gmeow_gts::cose::TAG_ENCRYPT0
    orphans: 1
  - item: gmeow_gts::cose::TAG_SIGN1
    orphans: 1
  - item: gmeow_gts::cose::declared_algorithm
    orphans: 50
  - item: gmeow_gts::cose::decrypt0
    orphans: 35
  - item: gmeow_gts::cose::enc_structure
    orphans: 8
  - item: gmeow_gts::cose::encrypt0
    orphans: 19
  - item: gmeow_gts::cose::encrypt0_protected
    orphans: 7
  - item: gmeow_gts::cose::encrypt0_with_iv
    orphans: 45
  - item: gmeow_gts::cose::parse
    orphans: 31
  - item: gmeow_gts::cose::parse_encrypt0
    orphans: 33
  - item: gmeow_gts::cose::protected_header
    orphans: 6
  - item: gmeow_gts::cose::recipient_kid
    orphans: 5
  - item: gmeow_gts::cose::scan_head
    orphans: 29
  - item: gmeow_gts::cose::scan_header_item
    orphans: 46
  - item: gmeow_gts::cose::sig_structure
    orphans: 9
  - item: gmeow_gts::cose::sign_id
    orphans: 19
  - item: gmeow_gts::cose::signature_kid
    orphans: 5
  - item: gmeow_gts::cose::unsupported_algorithm
    orphans: 9
  - item: gmeow_gts::cose::verify_parsed
    orphans: 14
  - item: gmeow_gts::cose::verify_sig
    orphans: 12
  - item: gmeow_gts::cose::verify_signatures
    orphans: 30
  - item: gmeow_gts::db::<impl DbExportError>
    orphans: 2
  - item: gmeow_gts::db::<impl fmt::Display for DbExportError>
    orphans: 2
  - item: gmeow_gts::db::<impl std::error::Error for DbExportError>
    orphans: 1
  - item: gmeow_gts::db::DbExportError
    orphans: 5
  - item: gmeow_gts::db::DbExportError::fmt
    orphans: 3
  - item: gmeow_gts::db::DbExportError::new
    orphans: 5
  - item: gmeow_gts::db::INDEXES
    orphans: 6
  - item: gmeow_gts::db::SCHEMA
    orphans: 7
  - item: gmeow_gts::db::SqlDialect
    orphans: 6
  - item: gmeow_gts::db::TABLES
    orphans: 2
  - item: gmeow_gts::db::export_database_file
    orphans: 13
  - item: gmeow_gts::db::replace_file
    orphans: 29
  - item: gmeow_gts::db::run_sql_tool
    orphans: 46
  - item: gmeow_gts::db::sql_io
    orphans: 3
  - item: gmeow_gts::db::table_count
    orphans: 11
  - item: gmeow_gts::db::temp_sibling_path
    orphans: 18
  - item: gmeow_gts::db::term_kind_int
    orphans: 8
  - item: gmeow_gts::db::tests::load_script_decode_failure_stops_before_commit
    orphans: 20
  - item: gmeow_gts::db::tests::load_script_decodes_lazy_blob_without_caching_it
    orphans: 20
  - item: gmeow_gts::db::tests::zstd_bytes
    orphans: 6
  - item: gmeow_gts::db::tests::zstd_codec
    orphans: 6
  - item: gmeow_gts::db::to_duckdb
    orphans: 14
  - item: gmeow_gts::db::to_parquet
    orphans: 61
  - item: gmeow_gts::db::to_sqlite
    orphans: 13
  - item: gmeow_gts::db::write_copy_stmt
    orphans: 6
  - item: gmeow_gts::db::write_hex
    orphans: 12
  - item: gmeow_gts::db::write_insert_rows
    orphans: 71
  - item: gmeow_gts::db::write_load_script
    orphans: 16
  - item: gmeow_gts::db::write_sql
    orphans: 3
  - item: gmeow_gts::db::write_sql_blob
    orphans: 19
  - item: gmeow_gts::db::write_sql_fmt
    orphans: 3
  - item: gmeow_gts::db::write_sql_path
    orphans: 12
  - item: gmeow_gts::db::write_sql_text
    orphans: 15
  - item: gmeow_gts::db::write_sql_usize
    orphans: 6
  - item: gmeow_gts::dumpdir::<impl DumpError>
    orphans: 4
  - item: gmeow_gts::dumpdir::<impl From<io::Error> for DumpError>
    orphans: 2
  - item: gmeow_gts::dumpdir::<impl fmt::Display for DumpError>
    orphans: 2
  - item: gmeow_gts::dumpdir::<impl std::error::Error for DumpError>
    orphans: 1
  - item: gmeow_gts::dumpdir::DumpError
    orphans: 6
  - item: gmeow_gts::dumpdir::DumpError::fmt
    orphans: 3
  - item: gmeow_gts::dumpdir::DumpError::from
    orphans: 3
  - item: gmeow_gts::dumpdir::DumpError::io
    orphans: 6
  - item: gmeow_gts::dumpdir::DumpError::kind
    orphans: 5
  - item: gmeow_gts::dumpdir::DumpError::refused
    orphans: 6
  - item: gmeow_gts::dumpdir::DumpErrorKind
    orphans: 8
  - item: gmeow_gts::dumpdir::DumpOptions
    orphans: 10
  - item: gmeow_gts::dumpdir::DumpReport
    orphans: 14
  - item: gmeow_gts::dumpdir::DumpState
    orphans: 12
  - item: gmeow_gts::dumpdir::FILES_NS
    orphans: 1
  - item: gmeow_gts::dumpdir::append_blob_frame_row
    orphans: 33
  - item: gmeow_gts::dumpdir::append_jsonl
    orphans: 5
  - item: gmeow_gts::dumpdir::append_payload_rows
    orphans: 47
  - item: gmeow_gts::dumpdir::blob_digest_from_meta
    orphans: 11
  - item: gmeow_gts::dumpdir::blob_meta_text
    orphans: 14
  - item: gmeow_gts::dumpdir::cbor_json
    orphans: 33
  - item: gmeow_gts::dumpdir::create_writer
    orphans: 3
  - item: gmeow_gts::dumpdir::dump_bytes
    orphans: 61
  - item: gmeow_gts::dumpdir::dump_is_clean
    orphans: 6
  - item: gmeow_gts::dumpdir::dump_path
    orphans: 21
  - item: gmeow_gts::dumpdir::files_pax_json
    orphans: 16
  - item: gmeow_gts::dumpdir::files_xattrs_json
    orphans: 16
  - item: gmeow_gts::dumpdir::frame_contribution_nquads
    orphans: 38
  - item: gmeow_gts::dumpdir::frame_has_projectable_rdf
    orphans: 3
  - item: gmeow_gts::dumpdir::frame_inventory_row
    orphans: 14
  - item: gmeow_gts::dumpdir::frame_prefix
    orphans: 3
  - item: gmeow_gts::dumpdir::frames_readme
    orphans: 3
  - item: gmeow_gts::dumpdir::graph_mentions_files_profile
    orphans: 9
  - item: gmeow_gts::dumpdir::graph_readme
    orphans: 9
  - item: gmeow_gts::dumpdir::has_transform
    orphans: 3
  - item: gmeow_gts::dumpdir::header_json
    orphans: 17
  - item: gmeow_gts::dumpdir::json_optional_string
    orphans: 3
  - item: gmeow_gts::dumpdir::json_optional_u64
    orphans: 3
  - item: gmeow_gts::dumpdir::json_optional_usize
    orphans: 3
  - item: gmeow_gts::dumpdir::json_string
    orphans: 22
  - item: gmeow_gts::dumpdir::segment_inventory_row
    orphans: 17
  - item: gmeow_gts::dumpdir::staged_path
    orphans: 17
  - item: gmeow_gts::dumpdir::suppressed_blob_digests
    orphans: 32
  - item: gmeow_gts::dumpdir::term_kind_name
    orphans: 8
  - item: gmeow_gts::dumpdir::top_readme
    orphans: 42
  - item: gmeow_gts::dumpdir::value_text
    orphans: 7
  - item: gmeow_gts::dumpdir::write_annotations
    orphans: 19
  - item: gmeow_gts::dumpdir::write_blob_index
    orphans: 36
  - item: gmeow_gts::dumpdir::write_blob_meta
    orphans: 17
  - item: gmeow_gts::dumpdir::write_control
    orphans: 10
  - item: gmeow_gts::dumpdir::write_diagnostics
    orphans: 18
  - item: gmeow_gts::dumpdir::write_dump
    orphans: 38
  - item: gmeow_gts::dumpdir::write_files_profile
    orphans: 87
  - item: gmeow_gts::dumpdir::write_frame_table_rows
    orphans: 58
  - item: gmeow_gts::dumpdir::write_frames
    orphans: 49
  - item: gmeow_gts::dumpdir::write_graph
    orphans: 18
  - item: gmeow_gts::dumpdir::write_manifest
    orphans: 65
  - item: gmeow_gts::dumpdir::write_meta
    orphans: 13
  - item: gmeow_gts::dumpdir::write_opaque
    orphans: 18
  - item: gmeow_gts::dumpdir::write_payloads
    orphans: 34
  - item: gmeow_gts::dumpdir::write_quads
    orphans: 15
  - item: gmeow_gts::dumpdir::write_readmes
    orphans: 6
  - item: gmeow_gts::dumpdir::write_reifiers
    orphans: 16
  - item: gmeow_gts::dumpdir::write_signatures
    orphans: 20
  - item: gmeow_gts::dumpdir::write_suppressions
    orphans: 24
  - item: gmeow_gts::dumpdir::write_terms
    orphans: 24
  - item: gmeow_gts::example_crate::agent_memory::main
    orphans: 69
  - item: gmeow_gts::example_crate::streaming_sink_bench::<impl StreamingSink for CountingSink>
    orphans: 12
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink
    orphans: 14
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::annotation
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::blob
    orphans: 8
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::diagnostic
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::opaque
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::quad
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::reifier
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::segment_head
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::signature
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::streamable_layout
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::suppression
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::CountingSink::term
    orphans: 3
  - item: gmeow_gts::example_crate::streaming_sink_bench::linux_peak_kib
    orphans: 10
  - item: gmeow_gts::example_crate::streaming_sink_bench::main
    orphans: 54
  - item: gmeow_gts::examples::agent_memory::<impl Default for RecallOptions<'_>>
    orphans: 2
  - item: gmeow_gts::examples::agent_memory::<impl From<std::io::Error> for MemoryError>
    orphans: 2
  - item: gmeow_gts::examples::agent_memory::<impl Memory>
    orphans: 14
  - item: gmeow_gts::examples::agent_memory::<impl fmt::Display for MemoryError>
    orphans: 2
  - item: gmeow_gts::examples::agent_memory::<impl std::error::Error for MemoryError>
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::ACCORDING_TO
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::CALLED_BY_INVOCATION
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::CONFIDENCE
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::Claim
    orphans: 18
  - item: gmeow_gts::examples::agent_memory::DCT_CREATED
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::INLINE_PAYLOAD_BUDGET
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::Memory
    orphans: 5
  - item: gmeow_gts::examples::agent_memory::Memory::append
    orphans: 8
  - item: gmeow_gts::examples::agent_memory::Memory::claims
    orphans: 32
  - item: gmeow_gts::examples::agent_memory::Memory::digest_id
    orphans: 14
  - item: gmeow_gts::examples::agent_memory::Memory::file_len
    orphans: 7
  - item: gmeow_gts::examples::agent_memory::Memory::graph
    orphans: 7
  - item: gmeow_gts::examples::agent_memory::Memory::new
    orphans: 4
  - item: gmeow_gts::examples::agent_memory::Memory::path
    orphans: 5
  - item: gmeow_gts::examples::agent_memory::Memory::recall
    orphans: 47
  - item: gmeow_gts::examples::agent_memory::Memory::record_tool_call
    orphans: 112
  - item: gmeow_gts::examples::agent_memory::Memory::revise
    orphans: 28
  - item: gmeow_gts::examples::agent_memory::Memory::store
    orphans: 98
  - item: gmeow_gts::examples::agent_memory::Memory::tool_calls
    orphans: 54
  - item: gmeow_gts::examples::agent_memory::Memory::verify
    orphans: 15
  - item: gmeow_gts::examples::agent_memory::MemoryError
    orphans: 16
  - item: gmeow_gts::examples::agent_memory::MemoryError::fmt
    orphans: 16
  - item: gmeow_gts::examples::agent_memory::MemoryError::from
    orphans: 3
  - item: gmeow_gts::examples::agent_memory::PROFILE
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::RDF_TYPE
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::RDF_VALUE
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::RecallOptions
    orphans: 12
  - item: gmeow_gts::examples::agent_memory::RecallOptions::default
    orphans: 8
  - item: gmeow_gts::examples::agent_memory::Result
    orphans: 2
  - item: gmeow_gts::examples::agent_memory::RevisionOptions
    orphans: 8
  - item: gmeow_gts::examples::agent_memory::SOFTWARE_AGENT
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::SOURCE_LOCATION
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::StoreOptions
    orphans: 10
  - item: gmeow_gts::examples::agent_memory::TOOL_ARGUMENTS
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::TOOL_CALL
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::TOOL_RESULT
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::ToolCallOptions
    orphans: 12
  - item: gmeow_gts::examples::agent_memory::ToolCallRecord
    orphans: 18
  - item: gmeow_gts::examples::agent_memory::USED_TOOL
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::WAS_DERIVED_FROM
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::WAS_GENERATED_BY
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::XSD_DATETIME
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::XSD_DECIMAL
    orphans: 1
  - item: gmeow_gts::examples::agent_memory::annotations_by_reifier
    orphans: 13
  - item: gmeow_gts::examples::agent_memory::inline_or_digest
    orphans: 8
  - item: gmeow_gts::examples::agent_memory::iri
    orphans: 11
  - item: gmeow_gts::examples::agent_memory::lit
    orphans: 11
  - item: gmeow_gts::examples::agent_memory::literal_with_datatype
    orphans: 6
  - item: gmeow_gts::examples::agent_memory::now_rfc3339
    orphans: 5
  - item: gmeow_gts::examples::agent_memory::push_annotation
    orphans: 11
  - item: gmeow_gts::examples::agent_memory::push_term
    orphans: 4
  - item: gmeow_gts::examples::agent_memory::suppressed_terms
    orphans: 20
  - item: gmeow_gts::examples::agent_memory::term_value
    orphans: 7
  - item: gmeow_gts::examples::agent_memory::value_as_idx
    orphans: 6
  - item: gmeow_gts::files::<impl FileEntryKind>
    orphans: 3
  - item: gmeow_gts::files::<impl TermBuilder>
    orphans: 6
  - item: gmeow_gts::files::<impl Write for HashingWriter<'_>>
    orphans: 3
  - item: gmeow_gts::files::BlobFrameMeta
    orphans: 8
  - item: gmeow_gts::files::FILES_DEV_MAJOR
    orphans: 1
  - item: gmeow_gts::files::FILES_DEV_MINOR
    orphans: 1
  - item: gmeow_gts::files::FILES_DIGEST
    orphans: 1
  - item: gmeow_gts::files::FILES_GID
    orphans: 1
  - item: gmeow_gts::files::FILES_GROUP_NAME
    orphans: 1
  - item: gmeow_gts::files::FILES_LINK_TARGET
    orphans: 1
  - item: gmeow_gts::files::FILES_MEDIA_TYPE
    orphans: 1
  - item: gmeow_gts::files::FILES_MODE
    orphans: 1
  - item: gmeow_gts::files::FILES_MODIFIED
    orphans: 1
  - item: gmeow_gts::files::FILES_NS
    orphans: 1
  - item: gmeow_gts::files::FILES_PATH
    orphans: 1
  - item: gmeow_gts::files::FILES_PAX_KEY
    orphans: 1
  - item: gmeow_gts::files::FILES_PAX_RECORD
    orphans: 1
  - item: gmeow_gts::files::FILES_PAX_VALUE
    orphans: 1
  - item: gmeow_gts::files::FILES_SIZE
    orphans: 1
  - item: gmeow_gts::files::FILES_TYPE
    orphans: 1
  - item: gmeow_gts::files::FILES_UID
    orphans: 1
  - item: gmeow_gts::files::FILES_USER_NAME
    orphans: 1
  - item: gmeow_gts::files::FILES_XATTR
    orphans: 1
  - item: gmeow_gts::files::FILES_XATTR_NAME
    orphans: 1
  - item: gmeow_gts::files::FILES_XATTR_VALUE
    orphans: 1
  - item: gmeow_gts::files::FILE_ENTRY
    orphans: 1
  - item: gmeow_gts::files::FileBlobSource
    orphans: 12
  - item: gmeow_gts::files::FileEntry
    orphans: 38
  - item: gmeow_gts::files::FileEntryKind
    orphans: 21
  - item: gmeow_gts::files::FileEntryKind::as_str
    orphans: 14
  - item: gmeow_gts::files::FileEntryKind::parse
    orphans: 19
  - item: gmeow_gts::files::FilePaxRecord
    orphans: 8
  - item: gmeow_gts::files::FileXattr
    orphans: 8
  - item: gmeow_gts::files::HashingWriter
    orphans: 3
  - item: gmeow_gts::files::HashingWriter::flush
    orphans: 3
  - item: gmeow_gts::files::HashingWriter::write
    orphans: 4
  - item: gmeow_gts::files::InlineBlobMap
    orphans: 1
  - item: gmeow_gts::files::RDF_TYPE
    orphans: 1
  - item: gmeow_gts::files::ResolvedFile
    orphans: 10
  - item: gmeow_gts::files::STREAM_CHUNK_SIZE
    orphans: 1
  - item: gmeow_gts::files::TermBuilder
    orphans: 6
  - item: gmeow_gts::files::TermBuilder::atom
    orphans: 27
  - item: gmeow_gts::files::TermBuilder::bnode
    orphans: 3
  - item: gmeow_gts::files::TermBuilder::iri
    orphans: 3
  - item: gmeow_gts::files::TermBuilder::literal
    orphans: 3
  - item: gmeow_gts::files::TermBuilder::quad_lit
    orphans: 5
  - item: gmeow_gts::files::TermKey
    orphans: 6
  - item: gmeow_gts::files::UnpackOptions
    orphans: 20
  - item: gmeow_gts::files::XSD_DATETIME
    orphans: 1
  - item: gmeow_gts::files::XSD_INTEGER
    orphans: 1
  - item: gmeow_gts::files::append_blob_path
    orphans: 47
  - item: gmeow_gts::files::bnode_term
    orphans: 11
  - item: gmeow_gts::files::build_entries_v2_prefix
    orphans: 100
  - item: gmeow_gts::files::copy_counted_and_hash
    orphans: 28
  - item: gmeow_gts::files::create_device
    orphans: 54
  - item: gmeow_gts::files::create_fifo
    orphans: 25
  - item: gmeow_gts::files::create_socket
    orphans: 6
  - item: gmeow_gts::files::create_special_entry
    orphans: 19
  - item: gmeow_gts::files::create_special_node
    orphans: 4
  - item: gmeow_gts::files::create_special_node_platform
    orphans: 17
  - item: gmeow_gts::files::dest_path
    orphans: 28
  - item: gmeow_gts::files::diff
    orphans: 52
  - item: gmeow_gts::files::digest_file
    orphans: 7
  - item: gmeow_gts::files::entry_to_field_map
    orphans: 42
  - item: gmeow_gts::files::file_digest_and_size
    orphans: 23
  - item: gmeow_gts::files::files_v2_profile_meta
    orphans: 3
  - item: gmeow_gts::files::finish_v2_writer
    orphans: 20
  - item: gmeow_gts::files::format_datetime
    orphans: 14
  - item: gmeow_gts::files::guess_media_type
    orphans: 17
  - item: gmeow_gts::files::hex
    orphans: 10
  - item: gmeow_gts::files::inline_blob_data
    orphans: 22
  - item: gmeow_gts::files::iri_term
    orphans: 11
  - item: gmeow_gts::files::literal_term
    orphans: 11
  - item: gmeow_gts::files::materialize_link
    orphans: 21
  - item: gmeow_gts::files::materialize_symlink
    orphans: 23
  - item: gmeow_gts::files::pack
    orphans: 10
  - item: gmeow_gts::files::pack_entries_v2
    orphans: 17
  - item: gmeow_gts::files::pack_entries_v2_with_blob_paths
    orphans: 25
  - item: gmeow_gts::files::pack_to_writer
    orphans: 96
  - item: gmeow_gts::files::parse_datetime
    orphans: 12
  - item: gmeow_gts::files::parse_optional_u32
    orphans: 10
  - item: gmeow_gts::files::parse_optional_u64
    orphans: 10
  - item: gmeow_gts::files::prepare_create_node_target
    orphans: 12
  - item: gmeow_gts::files::prepare_replace_target
    orphans: 13
  - item: gmeow_gts::files::read_entries
    orphans: 96
  - item: gmeow_gts::files::read_file_entries
    orphans: 15
  - item: gmeow_gts::files::read_pax_records
    orphans: 24
  - item: gmeow_gts::files::read_xattrs
    orphans: 24
  - item: gmeow_gts::files::reject_payload_fields
    orphans: 10
  - item: gmeow_gts::files::resolve_sources
    orphans: 40
  - item: gmeow_gts::files::resolved_files_with_metadata
    orphans: 29
  - item: gmeow_gts::files::restore_owner
    orphans: 36
  - item: gmeow_gts::files::restore_path_metadata
    orphans: 34
  - item: gmeow_gts::files::safe_archive_path
    orphans: 30
  - item: gmeow_gts::files::sanitized_mode
    orphans: 8
  - item: gmeow_gts::files::suppressed_blob_digests
    orphans: 33
  - item: gmeow_gts::files::to_posix_path
    orphans: 11
  - item: gmeow_gts::files::unix_system_time
    orphans: 10
  - item: gmeow_gts::files::unpack
    orphans: 16
  - item: gmeow_gts::files::unpack_with_options
    orphans: 100
  - item: gmeow_gts::files::validate_symlink_target
    orphans: 44
  - item: gmeow_gts::files::validate_v2_entry
    orphans: 43
  - item: gmeow_gts::files::walk_dir_sorted
    orphans: 25
  - item: gmeow_gts::files::write_blob_frame
    orphans: 27
  - item: gmeow_gts::files::write_blob_preimage
    orphans: 20
  - item: gmeow_gts::files::write_blob_pub_map
    orphans: 19
  - item: gmeow_gts::files::write_cbor_bytes
    orphans: 4
  - item: gmeow_gts::files::write_cbor_bytes_header
    orphans: 3
  - item: gmeow_gts::files::write_cbor_map_len
    orphans: 3
  - item: gmeow_gts::files::write_cbor_text
    orphans: 4
  - item: gmeow_gts::files::write_cbor_type_len
    orphans: 18
  - item: gmeow_gts::files::write_file_without_following_symlink
    orphans: 43
  - item: gmeow_gts::from_nquads::<impl Interner>
    orphans: 5
  - item: gmeow_gts::from_nquads::<impl NQuadsParseError>
    orphans: 2
  - item: gmeow_gts::from_nquads::<impl Tokenizer<'a>>
    orphans: 19
  - item: gmeow_gts::from_nquads::<impl fmt::Display for NQuadsParseError>
    orphans: 2
  - item: gmeow_gts::from_nquads::<impl std::error::Error for NQuadsParseError>
    orphans: 1
  - item: gmeow_gts::from_nquads::Atom
    orphans: 8
  - item: gmeow_gts::from_nquads::Interner
    orphans: 5
  - item: gmeow_gts::from_nquads::Interner::atom
    orphans: 40
  - item: gmeow_gts::from_nquads::Interner::new
    orphans: 7
  - item: gmeow_gts::from_nquads::Interner::node
    orphans: 11
  - item: gmeow_gts::from_nquads::Interner::triple
    orphans: 18
  - item: gmeow_gts::from_nquads::NQuadsParseError
    orphans: 5
  - item: gmeow_gts::from_nquads::NQuadsParseError::fmt
    orphans: 3
  - item: gmeow_gts::from_nquads::NQuadsParseError::new
    orphans: 5
  - item: gmeow_gts::from_nquads::Node
    orphans: 5
  - item: gmeow_gts::from_nquads::ParseOptions
    orphans: 5
  - item: gmeow_gts::from_nquads::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::from_nquads::TermKey
    orphans: 11
  - item: gmeow_gts::from_nquads::Tokenizer
    orphans: 5
  - item: gmeow_gts::from_nquads::Tokenizer::at_end
    orphans: 4
  - item: gmeow_gts::from_nquads::Tokenizer::atom
    orphans: 30
  - item: gmeow_gts::from_nquads::Tokenizer::bnode
    orphans: 26
  - item: gmeow_gts::from_nquads::Tokenizer::bump_char
    orphans: 5
  - item: gmeow_gts::from_nquads::Tokenizer::close_triple
    orphans: 11
  - item: gmeow_gts::from_nquads::Tokenizer::consume
    orphans: 7
  - item: gmeow_gts::from_nquads::Tokenizer::escape
    orphans: 23
  - item: gmeow_gts::from_nquads::Tokenizer::finish_line
    orphans: 11
  - item: gmeow_gts::from_nquads::Tokenizer::iri
    orphans: 17
  - item: gmeow_gts::from_nquads::Tokenizer::language_tag
    orphans: 10
  - item: gmeow_gts::from_nquads::Tokenizer::literal
    orphans: 54
  - item: gmeow_gts::from_nquads::Tokenizer::new
    orphans: 7
  - item: gmeow_gts::from_nquads::Tokenizer::node
    orphans: 7
  - item: gmeow_gts::from_nquads::Tokenizer::object
    orphans: 18
  - item: gmeow_gts::from_nquads::Tokenizer::peek_char
    orphans: 3
  - item: gmeow_gts::from_nquads::Tokenizer::quad
    orphans: 21
  - item: gmeow_gts::from_nquads::Tokenizer::quoted_triple
    orphans: 12
  - item: gmeow_gts::from_nquads::Tokenizer::rdf12_blank_label
    orphans: 23
  - item: gmeow_gts::from_nquads::Tokenizer::rdf12_iri
    orphans: 31
  - item: gmeow_gts::from_nquads::Tokenizer::resource
    orphans: 21
  - item: gmeow_gts::from_nquads::Tokenizer::rest
    orphans: 8
  - item: gmeow_gts::from_nquads::Tokenizer::skip_ws
    orphans: 5
  - item: gmeow_gts::from_nquads::Tokenizer::span
    orphans: 10
  - item: gmeow_gts::from_nquads::Tokenizer::string
    orphans: 23
  - item: gmeow_gts::from_nquads::Tokenizer::unicode_escape
    orphans: 22
  - item: gmeow_gts::from_nquads::TripleNode
    orphans: 6
  - item: gmeow_gts::from_nquads::build_gts
    orphans: 56
  - item: gmeow_gts::from_nquads::from_nquads
    orphans: 16
  - item: gmeow_gts::from_nquads::from_ntriples
    orphans: 16
  - item: gmeow_gts::from_nquads::has_iri_scheme
    orphans: 18
  - item: gmeow_gts::from_nquads::is_bnode_char
    orphans: 3
  - item: gmeow_gts::from_nquads::is_forbidden_iri_char
    orphans: 5
  - item: gmeow_gts::from_nquads::is_lang_char
    orphans: 3
  - item: gmeow_gts::from_nquads::parse_text
    orphans: 30
  - item: gmeow_gts::from_nquads::set_reifier
    orphans: 18
  - item: gmeow_gts::from_nquads::v2_quads::<impl Tokenizer<'_>>
    orphans: 8
  - item: gmeow_gts::from_nquads::v2_quads::blank_continue
    orphans: 4
  - item: gmeow_gts::from_nquads::v2_quads::blank_start
    orphans: 11
  - item: gmeow_gts::from_nquads::v2_quads::parse_v2_quads
    orphans: 24
  - item: gmeow_gts::from_nquads::validate_iri
    orphans: 13
  - item: gmeow_gts::from_nquads::validate_language_tag
    orphans: 39
  - item: gmeow_gts::from_nquads::validate_object
    orphans: 42
  - item: gmeow_gts::from_nquads::validate_predicate
    orphans: 15
  - item: gmeow_gts::from_nquads::validate_statement
    orphans: 23
  - item: gmeow_gts::from_nquads::validate_subject
    orphans: 23
  - item: gmeow_gts::from_nquads::validate_triple_node
    orphans: 9
  - item: gmeow_gts::from_okf::<impl Default for FromOkfOptions>
    orphans: 2
  - item: gmeow_gts::from_okf::<impl GraphBuilder>
    orphans: 7
  - item: gmeow_gts::from_okf::<impl OkfParseError>
    orphans: 2
  - item: gmeow_gts::from_okf::<impl fmt::Display for OkfParseError>
    orphans: 2
  - item: gmeow_gts::from_okf::<impl std::error::Error for OkfParseError>
    orphans: 1
  - item: gmeow_gts::from_okf::Document
    orphans: 6
  - item: gmeow_gts::from_okf::FromOkfOptions
    orphans: 11
  - item: gmeow_gts::from_okf::FromOkfOptions::default
    orphans: 7
  - item: gmeow_gts::from_okf::GraphBuilder
    orphans: 8
  - item: gmeow_gts::from_okf::GraphBuilder::atom
    orphans: 34
  - item: gmeow_gts::from_okf::GraphBuilder::bnode
    orphans: 3
  - item: gmeow_gts::from_okf::GraphBuilder::iri
    orphans: 3
  - item: gmeow_gts::from_okf::GraphBuilder::literal
    orphans: 3
  - item: gmeow_gts::from_okf::GraphBuilder::quad_iri
    orphans: 5
  - item: gmeow_gts::from_okf::GraphBuilder::quad_lit
    orphans: 5
  - item: gmeow_gts::from_okf::MarkdownLink
    orphans: 4
  - item: gmeow_gts::from_okf::OkfParseError
    orphans: 5
  - item: gmeow_gts::from_okf::OkfParseError::fmt
    orphans: 3
  - item: gmeow_gts::from_okf::OkfParseError::new
    orphans: 5
  - item: gmeow_gts::from_okf::TermKey
    orphans: 9
  - item: gmeow_gts::from_okf::extract_markdown_links
    orphans: 35
  - item: gmeow_gts::from_okf::from_okf
    orphans: 8
  - item: gmeow_gts::from_okf::from_okf_with_options
    orphans: 89
  - item: gmeow_gts::from_okf::has_yaml_frontmatter
    orphans: 3
  - item: gmeow_gts::from_okf::has_yaml_frontmatter_bytes
    orphans: 3
  - item: gmeow_gts::from_okf::import_document
    orphans: 63
  - item: gmeow_gts::from_okf::import_extension_value
    orphans: 30
  - item: gmeow_gts::from_okf::import_links
    orphans: 46
  - item: gmeow_gts::from_okf::is_frontmatterless_index
    orphans: 11
  - item: gmeow_gts::from_okf::manifest_meta
    orphans: 16
  - item: gmeow_gts::from_okf::markdown_files
    orphans: 40
  - item: gmeow_gts::from_okf::minted_doc_iri
    orphans: 3
  - item: gmeow_gts::from_okf::parse_markdown
    orphans: 35
  - item: gmeow_gts::from_okf::pathbuf_to_posix
    orphans: 20
  - item: gmeow_gts::from_okf::percent_encode_path
    orphans: 14
  - item: gmeow_gts::from_okf::relative_okf_path
    orphans: 12
  - item: gmeow_gts::from_okf::required_string_field
    orphans: 12
  - item: gmeow_gts::from_okf::resolve_link_path
    orphans: 25
  - item: gmeow_gts::from_okf::safe_relative_path
    orphans: 14
  - item: gmeow_gts::from_okf::str_span
    orphans: 7
  - item: gmeow_gts::from_okf::str_tail
    orphans: 4
  - item: gmeow_gts::from_okf::string_field
    orphans: 11
  - item: gmeow_gts::from_okf::tags_field
    orphans: 22
  - item: gmeow_gts::from_okf::yaml_to_json
    orphans: 28
  - item: gmeow_gts::from_tar::<impl Drop for TempBlob>
    orphans: 2
  - item: gmeow_gts::from_tar::FromTarOptions
    orphans: 12
  - item: gmeow_gts::from_tar::STREAM_CHUNK_SIZE
    orphans: 1
  - item: gmeow_gts::from_tar::TempBlob
    orphans: 5
  - item: gmeow_gts::from_tar::TempBlob::drop
    orphans: 3
  - item: gmeow_gts::from_tar::archive_path_from_path
    orphans: 11
  - item: gmeow_gts::from_tar::create_temp_blob_file
    orphans: 28
  - item: gmeow_gts::from_tar::detect_compression
    orphans: 16
  - item: gmeow_gts::from_tar::detect_reader_compression
    orphans: 9
  - item: gmeow_gts::from_tar::entry_kind
    orphans: 51
  - item: gmeow_gts::from_tar::format_mtime
    orphans: 9
  - item: gmeow_gts::from_tar::from_tar
    orphans: 12
  - item: gmeow_gts::from_tar::from_tar_bytes
    orphans: 8
  - item: gmeow_gts::from_tar::from_tar_to_writer
    orphans: 47
  - item: gmeow_gts::from_tar::link_target_to_string
    orphans: 7
  - item: gmeow_gts::from_tar::read_pax_records
    orphans: 27
  - item: gmeow_gts::from_tar::read_tar_entries_spooled
    orphans: 23
  - item: gmeow_gts::from_tar::read_tar_entry_metadata
    orphans: 59
  - item: gmeow_gts::from_tar::read_tar_entry_spooled
    orphans: 14
  - item: gmeow_gts::from_tar::spool_file_entry
    orphans: 34
  - item: gmeow_gts::from_trig::<impl Node>
    orphans: 3
  - item: gmeow_gts::from_trig::<impl Parser<'a>>
    orphans: 45
  - item: gmeow_gts::from_trig::<impl TriGParseError>
    orphans: 2
  - item: gmeow_gts::from_trig::<impl fmt::Display for TriGParseError>
    orphans: 2
  - item: gmeow_gts::from_trig::<impl std::error::Error for TriGParseError>
    orphans: 1
  - item: gmeow_gts::from_trig::Node
    orphans: 11
  - item: gmeow_gts::from_trig::Node::is_graph_name
    orphans: 3
  - item: gmeow_gts::from_trig::Node::token
    orphans: 23
  - item: gmeow_gts::from_trig::Parser
    orphans: 9
  - item: gmeow_gts::from_trig::Parser::base_directive
    orphans: 15
  - item: gmeow_gts::from_trig::Parser::blank_node_property_list
    orphans: 9
  - item: gmeow_gts::from_trig::Parser::bnode
    orphans: 25
  - item: gmeow_gts::from_trig::Parser::bump_char
    orphans: 5
  - item: gmeow_gts::from_trig::Parser::collection
    orphans: 26
  - item: gmeow_gts::from_trig::Parser::consume
    orphans: 9
  - item: gmeow_gts::from_trig::Parser::consume_char
    orphans: 9
  - item: gmeow_gts::from_trig::Parser::consume_keyword
    orphans: 22
  - item: gmeow_gts::from_trig::Parser::datatype_iri
    orphans: 8
  - item: gmeow_gts::from_trig::Parser::emit_statement
    orphans: 20
  - item: gmeow_gts::from_trig::Parser::eof
    orphans: 4
  - item: gmeow_gts::from_trig::Parser::escape
    orphans: 35
  - item: gmeow_gts::from_trig::Parser::expect_char
    orphans: 10
  - item: gmeow_gts::from_trig::Parser::graph_block
    orphans: 4
  - item: gmeow_gts::from_trig::Parser::graph_block_after_open
    orphans: 16
  - item: gmeow_gts::from_trig::Parser::iri
    orphans: 4
  - item: gmeow_gts::from_trig::Parser::iri_raw
    orphans: 27
  - item: gmeow_gts::from_trig::Parser::literal
    orphans: 30
  - item: gmeow_gts::from_trig::Parser::long_string
    orphans: 19
  - item: gmeow_gts::from_trig::Parser::looks_like_number
    orphans: 16
  - item: gmeow_gts::from_trig::Parser::maybe_reify_and_annotate
    orphans: 61
  - item: gmeow_gts::from_trig::Parser::new
    orphans: 11
  - item: gmeow_gts::from_trig::Parser::next_bnode
    orphans: 5
  - item: gmeow_gts::from_trig::Parser::numeric_literal
    orphans: 60
  - item: gmeow_gts::from_trig::Parser::parenthesized_quoted_triple
    orphans: 15
  - item: gmeow_gts::from_trig::Parser::parse
    orphans: 65
  - item: gmeow_gts::from_trig::Parser::peek_char
    orphans: 3
  - item: gmeow_gts::from_trig::Parser::predicate
    orphans: 8
  - item: gmeow_gts::from_trig::Parser::predicate_object_list
    orphans: 29
  - item: gmeow_gts::from_trig::Parser::prefix_directive
    orphans: 11
  - item: gmeow_gts::from_trig::Parser::prefix_label
    orphans: 19
  - item: gmeow_gts::from_trig::Parser::prefixed_name
    orphans: 63
  - item: gmeow_gts::from_trig::Parser::quoted_component
    orphans: 37
  - item: gmeow_gts::from_trig::Parser::quoted_string
    orphans: 28
  - item: gmeow_gts::from_trig::Parser::reifying_triple
    orphans: 36
  - item: gmeow_gts::from_trig::Parser::resolve_iri
    orphans: 9
  - item: gmeow_gts::from_trig::Parser::rest
    orphans: 8
  - item: gmeow_gts::from_trig::Parser::skip_ws_and_comments
    orphans: 20
  - item: gmeow_gts::from_trig::Parser::span
    orphans: 7
  - item: gmeow_gts::from_trig::Parser::statement_after_subject
    orphans: 25
  - item: gmeow_gts::from_trig::Parser::subject_allows_empty_pol
    orphans: 6
  - item: gmeow_gts::from_trig::Parser::term
    orphans: 22
  - item: gmeow_gts::from_trig::Parser::try_boolean_literal
    orphans: 25
  - item: gmeow_gts::from_trig::Parser::version_string
    orphans: 11
  - item: gmeow_gts::from_trig::RDF_FIRST
    orphans: 1
  - item: gmeow_gts::from_trig::RDF_NIL
    orphans: 1
  - item: gmeow_gts::from_trig::RDF_NS
    orphans: 1
  - item: gmeow_gts::from_trig::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::from_trig::RDF_REST
    orphans: 1
  - item: gmeow_gts::from_trig::RDF_TYPE
    orphans: 1
  - item: gmeow_gts::from_trig::TriGParseError
    orphans: 5
  - item: gmeow_gts::from_trig::TriGParseError::fmt
    orphans: 3
  - item: gmeow_gts::from_trig::TriGParseError::new
    orphans: 5
  - item: gmeow_gts::from_trig::XSD_BOOLEAN
    orphans: 1
  - item: gmeow_gts::from_trig::XSD_DECIMAL
    orphans: 1
  - item: gmeow_gts::from_trig::XSD_DOUBLE
    orphans: 1
  - item: gmeow_gts::from_trig::XSD_INTEGER
    orphans: 1
  - item: gmeow_gts::from_trig::from_trig
    orphans: 10
  - item: gmeow_gts::from_trig::from_turtle
    orphans: 11
  - item: gmeow_gts::from_trig::has_iri_scheme
    orphans: 18
  - item: gmeow_gts::from_trig::is_forbidden_iri_char
    orphans: 5
  - item: gmeow_gts::from_trig::remove_dot_segments
    orphans: 31
  - item: gmeow_gts::from_trig::resolve_relative_iri
    orphans: 43
  - item: gmeow_gts::from_trig::split_base_for_path
    orphans: 13
  - item: gmeow_gts::from_trig::split_raw_path_suffix
    orphans: 4
  - item: gmeow_gts::from_trig::unescape_pn_local
    orphans: 22
  - item: gmeow_gts::from_yamlld::<impl Context>
    orphans: 4
  - item: gmeow_gts::from_yamlld::<impl Interner>
    orphans: 6
  - item: gmeow_gts::from_yamlld::<impl YamlLdParseError>
    orphans: 2
  - item: gmeow_gts::from_yamlld::<impl fmt::Display for YamlLdParseError>
    orphans: 2
  - item: gmeow_gts::from_yamlld::<impl std::error::Error for YamlLdParseError>
    orphans: 1
  - item: gmeow_gts::from_yamlld::Context
    orphans: 4
  - item: gmeow_gts::from_yamlld::Context::expand
    orphans: 16
  - item: gmeow_gts::from_yamlld::Context::from_document
    orphans: 20
  - item: gmeow_gts::from_yamlld::Context::merge
    orphans: 25
  - item: gmeow_gts::from_yamlld::Interner
    orphans: 5
  - item: gmeow_gts::from_yamlld::Interner::atom
    orphans: 36
  - item: gmeow_gts::from_yamlld::Interner::generated_bnode
    orphans: 16
  - item: gmeow_gts::from_yamlld::Interner::new
    orphans: 7
  - item: gmeow_gts::from_yamlld::Interner::triple
    orphans: 20
  - item: gmeow_gts::from_yamlld::Interner::unbound_triple
    orphans: 30
  - item: gmeow_gts::from_yamlld::RowSinks
    orphans: 6
  - item: gmeow_gts::from_yamlld::TermKey
    orphans: 11
  - item: gmeow_gts::from_yamlld::YamlLdParseError
    orphans: 5
  - item: gmeow_gts::from_yamlld::YamlLdParseError::fmt
    orphans: 3
  - item: gmeow_gts::from_yamlld::YamlLdParseError::new
    orphans: 5
  - item: gmeow_gts::from_yamlld::from_json_ld
    orphans: 11
  - item: gmeow_gts::from_yamlld::from_json_ld_value
    orphans: 40
  - item: gmeow_gts::from_yamlld::from_yaml_ld
    orphans: 11
  - item: gmeow_gts::from_yamlld::graph_nodes
    orphans: 13
  - item: gmeow_gts::from_yamlld::inferred_datatype
    orphans: 10
  - item: gmeow_gts::from_yamlld::number_literal
    orphans: 14
  - item: gmeow_gts::from_yamlld::object
    orphans: 6
  - item: gmeow_gts::from_yamlld::parse_annotation_blocks
    orphans: 44
  - item: gmeow_gts::from_yamlld::parse_annotation_properties
    orphans: 37
  - item: gmeow_gts::from_yamlld::parse_id
    orphans: 14
  - item: gmeow_gts::from_yamlld::parse_literal_object
    orphans: 49
  - item: gmeow_gts::from_yamlld::parse_node
    orphans: 52
  - item: gmeow_gts::from_yamlld::parse_property_values
    orphans: 64
  - item: gmeow_gts::from_yamlld::parse_standalone_reifiers
    orphans: 44
  - item: gmeow_gts::from_yamlld::parse_term
    orphans: 59
  - item: gmeow_gts::from_yamlld::parse_triple
    orphans: 21
  - item: gmeow_gts::from_yamlld::predicate_id
    orphans: 8
  - item: gmeow_gts::from_yamlld::scalar_lexical
    orphans: 8
  - item: gmeow_gts::from_yamlld::scoped_context
    orphans: 7
  - item: gmeow_gts::from_yamlld::set_reifier
    orphans: 12
  - item: gmeow_gts::mmr::<impl JsonParser<'a>>
    orphans: 11
  - item: gmeow_gts::mmr::<impl Proof>
    orphans: 3
  - item: gmeow_gts::mmr::HASH_ALGORITHM
    orphans: 1
  - item: gmeow_gts::mmr::Json
    orphans: 9
  - item: gmeow_gts::mmr::JsonParser
    orphans: 4
  - item: gmeow_gts::mmr::JsonParser::array
    orphans: 22
  - item: gmeow_gts::mmr::JsonParser::hex4
    orphans: 20
  - item: gmeow_gts::mmr::JsonParser::literal
    orphans: 8
  - item: gmeow_gts::mmr::JsonParser::new
    orphans: 6
  - item: gmeow_gts::mmr::JsonParser::number
    orphans: 14
  - item: gmeow_gts::mmr::JsonParser::object
    orphans: 30
  - item: gmeow_gts::mmr::JsonParser::parse
    orphans: 8
  - item: gmeow_gts::mmr::JsonParser::string
    orphans: 41
  - item: gmeow_gts::mmr::JsonParser::value
    orphans: 26
  - item: gmeow_gts::mmr::JsonParser::ws
    orphans: 9
  - item: gmeow_gts::mmr::LEAF_DOMAIN
    orphans: 1
  - item: gmeow_gts::mmr::MmrPeak
    orphans: 8
  - item: gmeow_gts::mmr::Node
    orphans: 9
  - item: gmeow_gts::mmr::PARENT_DOMAIN
    orphans: 1
  - item: gmeow_gts::mmr::PREIMAGE_VERSION
    orphans: 1
  - item: gmeow_gts::mmr::PROOF_SCHEMA
    orphans: 2
  - item: gmeow_gts::mmr::Proof
    orphans: 22
  - item: gmeow_gts::mmr::Proof::from_json
    orphans: 10
  - item: gmeow_gts::mmr::Proof::to_json
    orphans: 57
  - item: gmeow_gts::mmr::ProofSide
    orphans: 8
  - item: gmeow_gts::mmr::ProofStep
    orphans: 10
  - item: gmeow_gts::mmr::ROOT_DOMAIN
    orphans: 1
  - item: gmeow_gts::mmr::append_path
    orphans: 26
  - item: gmeow_gts::mmr::array_items
    orphans: 6
  - item: gmeow_gts::mmr::as_i128
    orphans: 7
  - item: gmeow_gts::mmr::as_text
    orphans: 7
  - item: gmeow_gts::mmr::as_usize
    orphans: 3
  - item: gmeow_gts::mmr::build_nodes
    orphans: 36
  - item: gmeow_gts::mmr::expected_peak_heights
    orphans: 10
  - item: gmeow_gts::mmr::get
    orphans: 7
  - item: gmeow_gts::mmr::is_header_item
    orphans: 6
  - item: gmeow_gts::mmr::json_escape
    orphans: 17
  - item: gmeow_gts::mmr::leaf_hash
    orphans: 7
  - item: gmeow_gts::mmr::object_entries
    orphans: 6
  - item: gmeow_gts::mmr::parent_hash
    orphans: 8
  - item: gmeow_gts::mmr::parse_hex_32
    orphans: 26
  - item: gmeow_gts::mmr::peak_index_for_leaf
    orphans: 25
  - item: gmeow_gts::mmr::peak_list
    orphans: 9
  - item: gmeow_gts::mmr::peak_width
    orphans: 7
  - item: gmeow_gts::mmr::proof_from_json
    orphans: 51
  - item: gmeow_gts::mmr::prove
    orphans: 27
  - item: gmeow_gts::mmr::prove_file
    orphans: 107
  - item: gmeow_gts::mmr::root
    orphans: 9
  - item: gmeow_gts::mmr::root_hash
    orphans: 11
  - item: gmeow_gts::mmr::string_field
    orphans: 6
  - item: gmeow_gts::mmr::tests::id
    orphans: 3
  - item: gmeow_gts::mmr::tests::proof_round_trip_and_tamper
    orphans: 16
  - item: gmeow_gts::mmr::tests::roots_change_with_order
    orphans: 6
  - item: gmeow_gts::mmr::uint
    orphans: 3
  - item: gmeow_gts::mmr::usize_field
    orphans: 7
  - item: gmeow_gts::mmr::verify_proof
    orphans: 82
  - item: gmeow_gts::model::<impl BlobEntry>
    orphans: 9
  - item: gmeow_gts::model::<impl Graph>
    orphans: 18
  - item: gmeow_gts::model::<impl IntoIterator for Graph>
    orphans: 3
  - item: gmeow_gts::model::<impl Iterator for QuadTermsIter<'a>>
    orphans: 3
  - item: gmeow_gts::model::<impl TermKind>
    orphans: 2
  - item: gmeow_gts::model::AnnotationRow
    orphans: 2
  - item: gmeow_gts::model::BlobEntry
    orphans: 18
  - item: gmeow_gts::model::BlobEntry::bytes
    orphans: 5
  - item: gmeow_gts::model::BlobEntry::cached_bytes
    orphans: 8
  - item: gmeow_gts::model::BlobEntry::decode
    orphans: 16
  - item: gmeow_gts::model::BlobEntry::decoded_bytes
    orphans: 12
  - item: gmeow_gts::model::BlobEntry::decoded_len
    orphans: 9
  - item: gmeow_gts::model::BlobEntry::decoded_vec
    orphans: 12
  - item: gmeow_gts::model::BlobEntry::is_lazy
    orphans: 5
  - item: gmeow_gts::model::BlobEntry::lazy
    orphans: 5
  - item: gmeow_gts::model::Diagnostic
    orphans: 10
  - item: gmeow_gts::model::Graph
    orphans: 39
  - item: gmeow_gts::model::Graph::IntoIter
    orphans: 1
  - item: gmeow_gts::model::Graph::Item
    orphans: 1
  - item: gmeow_gts::model::Graph::blob_bytes
    orphans: 13
  - item: gmeow_gts::model::Graph::blob_bytes_cloned
    orphans: 8
  - item: gmeow_gts::model::Graph::blob_entry
    orphans: 8
  - item: gmeow_gts::model::Graph::datatype_iri
    orphans: 23
  - item: gmeow_gts::model::Graph::decoded_blobs
    orphans: 12
  - item: gmeow_gts::model::Graph::into_iter
    orphans: 3
  - item: gmeow_gts::model::Graph::into_quads
    orphans: 8
  - item: gmeow_gts::model::Graph::quad_terms
    orphans: 12
  - item: gmeow_gts::model::Graph::reifier
    orphans: 13
  - item: gmeow_gts::model::Graph::reifier_triples
    orphans: 11
  - item: gmeow_gts::model::Graph::resolution_reaches
    orphans: 29
  - item: gmeow_gts::model::Graph::set_blob
    orphans: 4
  - item: gmeow_gts::model::Graph::set_blob_entry
    orphans: 8
  - item: gmeow_gts::model::Graph::set_blob_meta
    orphans: 8
  - item: gmeow_gts::model::Graph::set_lazy_blob
    orphans: 4
  - item: gmeow_gts::model::Graph::set_meta
    orphans: 8
  - item: gmeow_gts::model::Graph::set_reifier
    orphans: 14
  - item: gmeow_gts::model::Graph::snapshot_payload
    orphans: 5
  - item: gmeow_gts::model::Graph::triple_of
    orphans: 35
  - item: gmeow_gts::model::OpaqueNode
    orphans: 16
  - item: gmeow_gts::model::QuadTerms
    orphans: 12
  - item: gmeow_gts::model::QuadTermsIter
    orphans: 6
  - item: gmeow_gts::model::QuadTermsIter::Item
    orphans: 1
  - item: gmeow_gts::model::QuadTermsIter::next
    orphans: 9
  - item: gmeow_gts::model::ReifierRow
    orphans: 2
  - item: gmeow_gts::model::Signature
    orphans: 16
  - item: gmeow_gts::model::StreamableInfo
    orphans: 17
  - item: gmeow_gts::model::Suppression
    orphans: 10
  - item: gmeow_gts::model::Term
    orphans: 26
  - item: gmeow_gts::model::TermKind
    orphans: 12
  - item: gmeow_gts::model::TermKind::from_wire
    orphans: 10
  - item: gmeow_gts::model::is_literal_direction
    orphans: 5
  - item: gmeow_gts::native_store::<impl ExactSizeIterator for IntoQuads>
    orphans: 2
  - item: gmeow_gts::native_store::<impl Extend<RdfQuad> for NativeStore>
    orphans: 2
  - item: gmeow_gts::native_store::<impl From<RdfAdapterError> for NativeStoreError>
    orphans: 2
  - item: gmeow_gts::native_store::<impl GtsSidecar>
    orphans: 2
  - item: gmeow_gts::native_store::<impl IntoIterator for &'aNativeStore>
    orphans: 3
  - item: gmeow_gts::native_store::<impl IntoIterator for &'aNativeStore>::IntoIter
    orphans: 1
  - item: gmeow_gts::native_store::<impl IntoIterator for &'aNativeStore>::Item
    orphans: 1
  - item: gmeow_gts::native_store::<impl IntoIterator for &'aNativeStore>::into_iter
    orphans: 3
  - item: gmeow_gts::native_store::<impl Iterator for IntoQuads>
    orphans: 5
  - item: gmeow_gts::native_store::<impl NativeStore>
    orphans: 9
  - item: gmeow_gts::native_store::<impl fmt::Display for NativeStoreError>
    orphans: 2
  - item: gmeow_gts::native_store::<impl std::error::Error for NativeStoreError>
    orphans: 2
  - item: gmeow_gts::native_store::GtsSidecar
    orphans: 31
  - item: gmeow_gts::native_store::GtsSidecar::from_graph
    orphans: 17
  - item: gmeow_gts::native_store::IntoQuads
    orphans: 5
  - item: gmeow_gts::native_store::IntoQuads::Item
    orphans: 1
  - item: gmeow_gts::native_store::IntoQuads::count
    orphans: 4
  - item: gmeow_gts::native_store::IntoQuads::len
    orphans: 4
  - item: gmeow_gts::native_store::IntoQuads::next
    orphans: 4
  - item: gmeow_gts::native_store::IntoQuads::size_hint
    orphans: 4
  - item: gmeow_gts::native_store::NativeStore
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::as_dataset
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::extend
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::from_dataset
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::insert
    orphans: 4
  - item: gmeow_gts::native_store::NativeStore::into_dataset
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::is_empty
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::iter
    orphans: 4
  - item: gmeow_gts::native_store::NativeStore::len
    orphans: 5
  - item: gmeow_gts::native_store::NativeStore::new
    orphans: 5
  - item: gmeow_gts::native_store::NativeStoreError
    orphans: 6
  - item: gmeow_gts::native_store::NativeStoreError::fmt
    orphans: 5
  - item: gmeow_gts::native_store::NativeStoreError::from
    orphans: 3
  - item: gmeow_gts::native_store::NativeStoreError::source
    orphans: 5
  - item: gmeow_gts::native_store::StoreWithSidecar
    orphans: 8
  - item: gmeow_gts::native_store::graph_into_quads
    orphans: 11
  - item: gmeow_gts::native_store::graph_into_quads_with_sidecar
    orphans: 11
  - item: gmeow_gts::native_store::graph_to_store
    orphans: 12
  - item: gmeow_gts::native_store::graph_to_store_with_sidecar
    orphans: 12
  - item: gmeow_gts::native_store::store_to_gts_bytes
    orphans: 8
  - item: gmeow_gts::native_store::store_to_writer
    orphans: 16
  - item: gmeow_gts::nested::<impl NestedReadResult>
    orphans: 2
  - item: gmeow_gts::nested::GTS_MEDIA_TYPE
    orphans: 2
  - item: gmeow_gts::nested::NestedReadResult
    orphans: 10
  - item: gmeow_gts::nested::NestedReadResult::subgraph
    orphans: 8
  - item: gmeow_gts::nested::blob_media_type
    orphans: 9
  - item: gmeow_gts::nested::read_nested
    orphans: 28
  - item: gmeow_gts::nested::visit
    orphans: 79
  - item: gmeow_gts::nquads::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::nquads::render_term
    orphans: 39
  - item: gmeow_gts::nquads::to_nquads
    orphans: 53
  - item: gmeow_gts::okf::<impl Default for OkfExportOptions>
    orphans: 2
  - item: gmeow_gts::okf::<impl From<io::Error> for OkfExportError>
    orphans: 2
  - item: gmeow_gts::okf::<impl OkfExportError>
    orphans: 2
  - item: gmeow_gts::okf::<impl fmt::Display for OkfExportError>
    orphans: 2
  - item: gmeow_gts::okf::<impl std::error::Error for OkfExportError>
    orphans: 1
  - item: gmeow_gts::okf::DEFAULT_BASE_IRI
    orphans: 2
  - item: gmeow_gts::okf::ExportDocument
    orphans: 8
  - item: gmeow_gts::okf::Manifest
    orphans: 8
  - item: gmeow_gts::okf::OKF_BODY
    orphans: 2
  - item: gmeow_gts::okf::OKF_DESCRIPTION
    orphans: 2
  - item: gmeow_gts::okf::OKF_JSON
    orphans: 2
  - item: gmeow_gts::okf::OKF_LINKS
    orphans: 2
  - item: gmeow_gts::okf::OKF_LINK_OCCURRENCE
    orphans: 3
  - item: gmeow_gts::okf::OKF_LINK_TEXT
    orphans: 2
  - item: gmeow_gts::okf::OKF_NS
    orphans: 2
  - item: gmeow_gts::okf::OKF_PATH
    orphans: 2
  - item: gmeow_gts::okf::OKF_RESOURCE
    orphans: 2
  - item: gmeow_gts::okf::OKF_TAG
    orphans: 2
  - item: gmeow_gts::okf::OKF_TIMESTAMP
    orphans: 2
  - item: gmeow_gts::okf::OKF_TITLE
    orphans: 2
  - item: gmeow_gts::okf::OKF_TYPE
    orphans: 2
  - item: gmeow_gts::okf::OkfExportError
    orphans: 5
  - item: gmeow_gts::okf::OkfExportError::fmt
    orphans: 3
  - item: gmeow_gts::okf::OkfExportError::from
    orphans: 3
  - item: gmeow_gts::okf::OkfExportError::new
    orphans: 5
  - item: gmeow_gts::okf::OkfExportOptions
    orphans: 9
  - item: gmeow_gts::okf::OkfExportOptions::default
    orphans: 6
  - item: gmeow_gts::okf::OkfExportReport
    orphans: 10
  - item: gmeow_gts::okf::RDF_REIFIES
    orphans: 2
  - item: gmeow_gts::okf::XSD_BOOLEAN
    orphans: 2
  - item: gmeow_gts::okf::XSD_DATETIME
    orphans: 2
  - item: gmeow_gts::okf::XSD_DECIMAL
    orphans: 2
  - item: gmeow_gts::okf::XSD_INTEGER
    orphans: 2
  - item: gmeow_gts::okf::body_bytes
    orphans: 23
  - item: gmeow_gts::okf::collect_documents
    orphans: 89
  - item: gmeow_gts::okf::collect_unmapped
    orphans: 59
  - item: gmeow_gts::okf::consume_string_field
    orphans: 13
  - item: gmeow_gts::okf::consumed_link_reifiers
    orphans: 30
  - item: gmeow_gts::okf::iri_value
    orphans: 9
  - item: gmeow_gts::okf::known_local
    orphans: 17
  - item: gmeow_gts::okf::literal_value
    orphans: 9
  - item: gmeow_gts::okf::safe_relative_path
    orphans: 14
  - item: gmeow_gts::okf::staged_path
    orphans: 17
  - item: gmeow_gts::okf::to_okf
    orphans: 33
  - item: gmeow_gts::okf::write_document
    orphans: 32
  - item: gmeow_gts::okf::write_documents
    orphans: 27
  - item: gmeow_gts::okf::write_manifest
    orphans: 20
  - item: gmeow_gts::okf::write_okf
    orphans: 31
  - item: gmeow_gts::okf::yaml_value
    orphans: 24
  - item: gmeow_gts::openpgp::<impl OpenPgpSigningKey>
    orphans: 6
  - item: gmeow_gts::openpgp::<impl std::error::Error for OpenPgpError>
    orphans: 1
  - item: gmeow_gts::openpgp::<impl std::fmt::Display for OpenPgpError>
    orphans: 2
  - item: gmeow_gts::openpgp::ED25519_ALGO
    orphans: 2
  - item: gmeow_gts::openpgp::ED25519_OID
    orphans: 2
  - item: gmeow_gts::openpgp::OpenPgpError
    orphans: 3
  - item: gmeow_gts::openpgp::OpenPgpError::fmt
    orphans: 3
  - item: gmeow_gts::openpgp::OpenPgpSigningKey
    orphans: 8
  - item: gmeow_gts::openpgp::OpenPgpSigningKey::fingerprint
    orphans: 5
  - item: gmeow_gts::openpgp::OpenPgpSigningKey::into_parts
    orphans: 5
  - item: gmeow_gts::openpgp::OpenPgpSigningKey::kid
    orphans: 5
  - item: gmeow_gts::openpgp::OpenPgpSigningKey::raw_public
    orphans: 5
  - item: gmeow_gts::openpgp::OpenPgpSigningKey::signing_key
    orphans: 5
  - item: gmeow_gts::openpgp::Result
    orphans: 1
  - item: gmeow_gts::openpgp::TransportKey
    orphans: 8
  - item: gmeow_gts::openpgp::b64_decode
    orphans: 34
  - item: gmeow_gts::openpgp::checksum16
    orphans: 4
  - item: gmeow_gts::openpgp::err
    orphans: 3
  - item: gmeow_gts::openpgp::fingerprint
    orphans: 18
  - item: gmeow_gts::openpgp::iter_packets
    orphans: 11
  - item: gmeow_gts::openpgp::next_packet
    orphans: 86
  - item: gmeow_gts::openpgp::parse_ed25519_public_material
    orphans: 48
  - item: gmeow_gts::openpgp::parse_ed25519_secret_material
    orphans: 35
  - item: gmeow_gts::openpgp::parse_secret_signing_key
    orphans: 33
  - item: gmeow_gts::openpgp::parse_transport_key
    orphans: 31
  - item: gmeow_gts::openpgp::read_mpi
    orphans: 13
  - item: gmeow_gts::openpgp::secret_mpi_to_seed
    orphans: 16
  - item: gmeow_gts::openpgp::strip_armor
    orphans: 41
  - item: gmeow_gts::openpgp::tests::armor_private_key
    orphans: 9
  - item: gmeow_gts::openpgp::tests::b64_encode
    orphans: 23
  - item: gmeow_gts::openpgp::tests::encode_packet
    orphans: 16
  - item: gmeow_gts::openpgp::tests::fixture
    orphans: 3
  - item: gmeow_gts::openpgp::tests::fixtures_dir
    orphans: 3
  - item: gmeow_gts::openpgp::tests::mutated_secret_armor
    orphans: 5
  - item: gmeow_gts::openpgp::tests::parses_frozen_vector
    orphans: 18
  - item: gmeow_gts::openpgp::tests::parses_secret_signing_key_with_default_fingerprint_kid
    orphans: 17
  - item: gmeow_gts::openpgp::tests::parses_secret_signing_key_with_kid_override
    orphans: 9
  - item: gmeow_gts::openpgp::tests::rejects_non_pgp
    orphans: 4
  - item: gmeow_gts::openpgp::tests::secret_mpi_to_seed_left_pads_short_mpis
    orphans: 6
  - item: gmeow_gts::openpgp::tests::secret_packet_body
    orphans: 8
  - item: gmeow_gts::openpgp::tests::secret_signing_key_rejects_bad_checksum
    orphans: 11
  - item: gmeow_gts::openpgp::tests::secret_signing_key_rejects_encrypted_secret_packets
    orphans: 11
  - item: gmeow_gts::openpgp::tests::secret_signing_key_rejects_public_armor
    orphans: 6
  - item: gmeow_gts::openpgp::tests::secret_signing_key_rejects_public_secret_mismatch
    orphans: 11
  - item: gmeow_gts::openpgp::tests::secret_signing_key_rejects_unsupported_algorithms_and_versions
    orphans: 16
  - item: gmeow_gts::openpgp::tests::secret_signing_key_rejects_unsupported_trailer_structure
    orphans: 10
  - item: gmeow_gts::openpgp::tests::vectors_dir
    orphans: 3
  - item: gmeow_gts::policy::<impl Default for TrustPolicy>
    orphans: 2
  - item: gmeow_gts::policy::<impl ProfileFinding>
    orphans: 2
  - item: gmeow_gts::policy::<impl Severity>
    orphans: 2
  - item: gmeow_gts::policy::<impl TrustPolicy>
    orphans: 9
  - item: gmeow_gts::policy::<impl std::error::Error for PolicyConfigError>
    orphans: 3
  - item: gmeow_gts::policy::<impl std::fmt::Display for PolicyConfigError>
    orphans: 3
  - item: gmeow_gts::policy::DEFAULT_PSEUDONYMOUS_KID_PATTERN
    orphans: 2
  - item: gmeow_gts::policy::FILES_NS
    orphans: 2
  - item: gmeow_gts::policy::PROFILE_VOCABS
    orphans: 1
  - item: gmeow_gts::policy::PolicyConfigError
    orphans: 21
  - item: gmeow_gts::policy::PolicyConfigError::fmt
    orphans: 18
  - item: gmeow_gts::policy::PolicyConfigError::source
    orphans: 10
  - item: gmeow_gts::policy::ProfileFinding
    orphans: 14
  - item: gmeow_gts::policy::ProfileFinding::new
    orphans: 15
  - item: gmeow_gts::policy::Severity
    orphans: 10
  - item: gmeow_gts::policy::Severity::as_str
    orphans: 9
  - item: gmeow_gts::policy::SignatureTrust
    orphans: 13
  - item: gmeow_gts::policy::TrustPolicy
    orphans: 25
  - item: gmeow_gts::policy::TrustPolicy::default
    orphans: 7
  - item: gmeow_gts::policy::TrustPolicy::from_json_str
    orphans: 11
  - item: gmeow_gts::policy::TrustPolicy::from_path
    orphans: 36
  - item: gmeow_gts::policy::TrustPolicy::from_yaml_str
    orphans: 11
  - item: gmeow_gts::policy::TrustPolicy::is_pseudonymous_recipient
    orphans: 17
  - item: gmeow_gts::policy::TrustPolicy::is_trusted
    orphans: 5
  - item: gmeow_gts::policy::TrustPolicy::new
    orphans: 12
  - item: gmeow_gts::policy::TrustPolicy::to_json_string
    orphans: 11
  - item: gmeow_gts::policy::TrustPolicy::to_yaml_string
    orphans: 11
  - item: gmeow_gts::policy::anchored_literal_pattern
    orphans: 26
  - item: gmeow_gts::policy::custom_pseudonymous_pattern_matches
    orphans: 12
  - item: gmeow_gts::policy::evaluate_profile_policy
    orphans: 45
  - item: gmeow_gts::policy::evidence_head_findings
    orphans: 22
  - item: gmeow_gts::policy::has_sealed_source
    orphans: 6
  - item: gmeow_gts::policy::namespace
    orphans: 9
  - item: gmeow_gts::policy::opaque_recipient_findings
    orphans: 38
  - item: gmeow_gts::policy::profile_vocab_findings
    orphans: 31
  - item: gmeow_gts::policy::signature_policy_findings
    orphans: 75
  - item: gmeow_gts::policy::signature_trust
    orphans: 25
  - item: gmeow_gts::policy::signed_heads
    orphans: 7
  - item: gmeow_gts::policy::stream_vocab_findings
    orphans: 33
  - item: gmeow_gts::policy::term_iri_value
    orphans: 9
  - item: gmeow_gts::policy::used_vocabs
    orphans: 25
  - item: gmeow_gts::rdf::<impl BaseDirection>
    orphans: 2
  - item: gmeow_gts::rdf::<impl BlankNode>
    orphans: 3
  - item: gmeow_gts::rdf::<impl BnodeLabels>
    orphans: 3
  - item: gmeow_gts::rdf::<impl Dataset>
    orphans: 6
  - item: gmeow_gts::rdf::<impl From<BlankNode> for GraphName>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<BlankNode> for NamedOrBlankNode>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<BlankNode> for RdfTerm>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<Iri> for GraphName>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<Iri> for NamedOrBlankNode>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<Iri> for RdfTerm>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<Literal> for RdfTerm>
    orphans: 2
  - item: gmeow_gts::rdf::<impl From<RdfTriple> for RdfTerm>
    orphans: 2
  - item: gmeow_gts::rdf::<impl GraphName>
    orphans: 2
  - item: gmeow_gts::rdf::<impl Interner>
    orphans: 9
  - item: gmeow_gts::rdf::<impl IntoIterator for &'aDataset>
    orphans: 3
  - item: gmeow_gts::rdf::<impl IntoIterator for &'aDataset>::IntoIter
    orphans: 1
  - item: gmeow_gts::rdf::<impl IntoIterator for &'aDataset>::Item
    orphans: 1
  - item: gmeow_gts::rdf::<impl IntoIterator for &'aDataset>::into_iter
    orphans: 3
  - item: gmeow_gts::rdf::<impl Iri>
    orphans: 3
  - item: gmeow_gts::rdf::<impl Literal>
    orphans: 5
  - item: gmeow_gts::rdf::<impl RdfAdapterError>
    orphans: 3
  - item: gmeow_gts::rdf::<impl RdfQuad>
    orphans: 2
  - item: gmeow_gts::rdf::<impl RdfTriple>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for BaseDirection>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for BlankNode>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for GraphName>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for Iri>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for Literal>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for NamedOrBlankNode>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for RdfAdapterError>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for RdfQuad>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for RdfTerm>
    orphans: 2
  - item: gmeow_gts::rdf::<impl fmt::Display for RdfTriple>
    orphans: 2
  - item: gmeow_gts::rdf::<impl std::error::Error for RdfAdapterError>
    orphans: 1
  - item: gmeow_gts::rdf::BaseDirection
    orphans: 8
  - item: gmeow_gts::rdf::BaseDirection::as_str
    orphans: 8
  - item: gmeow_gts::rdf::BaseDirection::fmt
    orphans: 3
  - item: gmeow_gts::rdf::BlankNode
    orphans: 5
  - item: gmeow_gts::rdf::BlankNode::as_str
    orphans: 5
  - item: gmeow_gts::rdf::BlankNode::fmt
    orphans: 3
  - item: gmeow_gts::rdf::BlankNode::new
    orphans: 14
  - item: gmeow_gts::rdf::BnodeLabels
    orphans: 3
  - item: gmeow_gts::rdf::BnodeLabels::for_graph
    orphans: 27
  - item: gmeow_gts::rdf::BnodeLabels::label
    orphans: 12
  - item: gmeow_gts::rdf::Dataset
    orphans: 5
  - item: gmeow_gts::rdf::Dataset::insert
    orphans: 4
  - item: gmeow_gts::rdf::Dataset::is_empty
    orphans: 5
  - item: gmeow_gts::rdf::Dataset::iter
    orphans: 4
  - item: gmeow_gts::rdf::Dataset::len
    orphans: 5
  - item: gmeow_gts::rdf::Dataset::new
    orphans: 5
  - item: gmeow_gts::rdf::ExportOptions
    orphans: 10
  - item: gmeow_gts::rdf::GraphName
    orphans: 11
  - item: gmeow_gts::rdf::GraphName::fmt
    orphans: 7
  - item: gmeow_gts::rdf::GraphName::from
    orphans: 6
  - item: gmeow_gts::rdf::GraphName::is_default_graph
    orphans: 5
  - item: gmeow_gts::rdf::Interner
    orphans: 4
  - item: gmeow_gts::rdf::Interner::blank_node
    orphans: 11
  - item: gmeow_gts::rdf::Interner::intern
    orphans: 9
  - item: gmeow_gts::rdf::Interner::iri
    orphans: 11
  - item: gmeow_gts::rdf::Interner::literal
    orphans: 39
  - item: gmeow_gts::rdf::Interner::named_or_blank
    orphans: 6
  - item: gmeow_gts::rdf::Interner::new
    orphans: 6
  - item: gmeow_gts::rdf::Interner::term
    orphans: 29
  - item: gmeow_gts::rdf::Interner::triple
    orphans: 7
  - item: gmeow_gts::rdf::Iri
    orphans: 5
  - item: gmeow_gts::rdf::Iri::as_str
    orphans: 5
  - item: gmeow_gts::rdf::Iri::fmt
    orphans: 3
  - item: gmeow_gts::rdf::Iri::new
    orphans: 22
  - item: gmeow_gts::rdf::Literal
    orphans: 12
  - item: gmeow_gts::rdf::Literal::fmt
    orphans: 12
  - item: gmeow_gts::rdf::Literal::new_directional_language_tagged_literal
    orphans: 19
  - item: gmeow_gts::rdf::Literal::new_language_tagged_literal
    orphans: 18
  - item: gmeow_gts::rdf::Literal::new_simple_literal
    orphans: 9
  - item: gmeow_gts::rdf::Literal::new_typed_literal
    orphans: 9
  - item: gmeow_gts::rdf::NamedOrBlankNode
    orphans: 8
  - item: gmeow_gts::rdf::NamedOrBlankNode::fmt
    orphans: 6
  - item: gmeow_gts::rdf::NamedOrBlankNode::from
    orphans: 6
  - item: gmeow_gts::rdf::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::rdf::RdfAdapterError
    orphans: 5
  - item: gmeow_gts::rdf::RdfAdapterError::detail
    orphans: 5
  - item: gmeow_gts::rdf::RdfAdapterError::fmt
    orphans: 3
  - item: gmeow_gts::rdf::RdfAdapterError::new
    orphans: 5
  - item: gmeow_gts::rdf::RdfQuad
    orphans: 12
  - item: gmeow_gts::rdf::RdfQuad::fmt
    orphans: 7
  - item: gmeow_gts::rdf::RdfQuad::new
    orphans: 14
  - item: gmeow_gts::rdf::RdfTerm
    orphans: 12
  - item: gmeow_gts::rdf::RdfTerm::fmt
    orphans: 8
  - item: gmeow_gts::rdf::RdfTerm::from
    orphans: 12
  - item: gmeow_gts::rdf::RdfTriple
    orphans: 10
  - item: gmeow_gts::rdf::RdfTriple::fmt
    orphans: 3
  - item: gmeow_gts::rdf::RdfTriple::new
    orphans: 12
  - item: gmeow_gts::rdf::TermKey
    orphans: 12
  - item: gmeow_gts::rdf::from_dataset
    orphans: 8
  - item: gmeow_gts::rdf::from_rdf_dataset
    orphans: 9
  - item: gmeow_gts::rdf::from_rdf_dataset_with_profile
    orphans: 12
  - item: gmeow_gts::rdf::graph_name_id
    orphans: 7
  - item: gmeow_gts::rdf::graph_name_term
    orphans: 25
  - item: gmeow_gts::rdf::graph_quad_to_rdf
    orphans: 24
  - item: gmeow_gts::rdf::graph_term
    orphans: 6
  - item: gmeow_gts::rdf::is_internal_triple_self_binding
    orphans: 10
  - item: gmeow_gts::rdf::is_valid_blank_node_label
    orphans: 17
  - item: gmeow_gts::rdf::is_valid_language_tag
    orphans: 8
  - item: gmeow_gts::rdf::literal_term
    orphans: 35
  - item: gmeow_gts::rdf::named_or_blank_term
    orphans: 23
  - item: gmeow_gts::rdf::predicate_term
    orphans: 18
  - item: gmeow_gts::rdf::quoted_triple
    orphans: 20
  - item: gmeow_gts::rdf::rdf_term
    orphans: 30
  - item: gmeow_gts::rdf::term_value
    orphans: 5
  - item: gmeow_gts::rdf::to_dataset
    orphans: 8
  - item: gmeow_gts::rdf::to_rdf_dataset
    orphans: 9
  - item: gmeow_gts::rdf::to_rdf_dataset_lossy
    orphans: 15
  - item: gmeow_gts::rdf::to_rdf_dataset_with_options
    orphans: 15
  - item: gmeow_gts::rdf::to_rdf_quads
    orphans: 9
  - item: gmeow_gts::rdf::to_rdf_quads_with_options
    orphans: 57
  - item: gmeow_gts::rdf::validate_language_tag
    orphans: 8
  - item: gmeow_gts::rdf::writer_from_rdf_dataset
    orphans: 9
  - item: gmeow_gts::rdf::writer_from_rdf_dataset_with_profile
    orphans: 60
  - item: gmeow_gts::rdf_codecs::<impl EventGraphSink>
    orphans: 4
  - item: gmeow_gts::rdf_codecs::<impl From<EventError> for RdfCodecError>
    orphans: 2
  - item: gmeow_gts::rdf_codecs::<impl From<RdfAdapterError> for RdfCodecError>
    orphans: 2
  - item: gmeow_gts::rdf_codecs::<impl From<std::io::Error> for RdfCodecError>
    orphans: 2
  - item: gmeow_gts::rdf_codecs::<impl From<std::string::FromUtf8Error> for RdfCodecError>
    orphans: 2
  - item: gmeow_gts::rdf_codecs::<impl RdfCodecError>
    orphans: 3
  - item: gmeow_gts::rdf_codecs::<impl RdfEventSink for EventGraphSink>
    orphans: 10
  - item: gmeow_gts::rdf_codecs::<impl fmt::Display for RdfCodecError>
    orphans: 2
  - item: gmeow_gts::rdf_codecs::<impl std::error::Error for RdfCodecError>
    orphans: 1
  - item: gmeow_gts::rdf_codecs::EventGraphSink
    orphans: 12
  - item: gmeow_gts::rdf_codecs::EventGraphSink::annotation
    orphans: 5
  - item: gmeow_gts::rdf_codecs::EventGraphSink::declares_before_reference
    orphans: 3
  - item: gmeow_gts::rdf_codecs::EventGraphSink::diagnostic
    orphans: 5
  - item: gmeow_gts::rdf_codecs::EventGraphSink::end_scope
    orphans: 11
  - item: gmeow_gts::rdf_codecs::EventGraphSink::ensure_not_finished
    orphans: 10
  - item: gmeow_gts::rdf_codecs::EventGraphSink::finish
    orphans: 10
  - item: gmeow_gts::rdf_codecs::EventGraphSink::into_graph
    orphans: 9
  - item: gmeow_gts::rdf_codecs::EventGraphSink::quad
    orphans: 5
  - item: gmeow_gts::rdf_codecs::EventGraphSink::reifier
    orphans: 5
  - item: gmeow_gts::rdf_codecs::EventGraphSink::start_scope
    orphans: 11
  - item: gmeow_gts::rdf_codecs::EventGraphSink::term
    orphans: 12
  - item: gmeow_gts::rdf_codecs::EventGraphSink::try_into_graph
    orphans: 52
  - item: gmeow_gts::rdf_codecs::RDF_NS
    orphans: 1
  - item: gmeow_gts::rdf_codecs::RdfCodecError
    orphans: 5
  - item: gmeow_gts::rdf_codecs::RdfCodecError::detail
    orphans: 5
  - item: gmeow_gts::rdf_codecs::RdfCodecError::fmt
    orphans: 3
  - item: gmeow_gts::rdf_codecs::RdfCodecError::from
    orphans: 14
  - item: gmeow_gts::rdf_codecs::RdfCodecError::new
    orphans: 5
  - item: gmeow_gts::rdf_codecs::XSD_NS
    orphans: 1
  - item: gmeow_gts::rdf_codecs::annotate_native_import
    orphans: 11
  - item: gmeow_gts::rdf_codecs::ensure_default_graph_projection
    orphans: 11
  - item: gmeow_gts::rdf_codecs::event_annotation_to_model
    orphans: 14
  - item: gmeow_gts::rdf_codecs::event_diagnostic_to_model
    orphans: 18
  - item: gmeow_gts::rdf_codecs::event_quad_to_model
    orphans: 13
  - item: gmeow_gts::rdf_codecs::event_reifier_to_model
    orphans: 17
  - item: gmeow_gts::rdf_codecs::event_term_to_model
    orphans: 67
  - item: gmeow_gts::rdf_codecs::from_ntriples
    orphans: 11
  - item: gmeow_gts::rdf_codecs::from_rdf_xml
    orphans: 9
  - item: gmeow_gts::rdf_codecs::from_rdf_xml_with_base_iri
    orphans: 8
  - item: gmeow_gts::rdf_codecs::from_trig
    orphans: 11
  - item: gmeow_gts::rdf_codecs::from_turtle
    orphans: 11
  - item: gmeow_gts::rdf_codecs::graph_from_erased_source
    orphans: 10
  - item: gmeow_gts::rdf_codecs::graph_from_source
    orphans: 11
  - item: gmeow_gts::rdf_codecs::map_event_id
    orphans: 12
  - item: gmeow_gts::rdf_codecs::serialize_ntriples_graph
    orphans: 4
  - item: gmeow_gts::rdf_codecs::serialize_rdf_xml_graph
    orphans: 3
  - item: gmeow_gts::rdf_codecs::serialize_trig_graph
    orphans: 4
  - item: gmeow_gts::rdf_codecs::serialize_turtle_graph
    orphans: 11
  - item: gmeow_gts::rdf_codecs::to_ntriples
    orphans: 12
  - item: gmeow_gts::rdf_codecs::to_ntriples_from_erased_source
    orphans: 10
  - item: gmeow_gts::rdf_codecs::to_ntriples_from_source
    orphans: 9
  - item: gmeow_gts::rdf_codecs::to_rdf_xml
    orphans: 12
  - item: gmeow_gts::rdf_codecs::to_rdf_xml_from_erased_source
    orphans: 8
  - item: gmeow_gts::rdf_codecs::to_rdf_xml_from_source
    orphans: 9
  - item: gmeow_gts::rdf_codecs::to_trig
    orphans: 8
  - item: gmeow_gts::rdf_codecs::to_trig_from_erased_source
    orphans: 8
  - item: gmeow_gts::rdf_codecs::to_trig_from_source
    orphans: 9
  - item: gmeow_gts::rdf_codecs::to_turtle
    orphans: 12
  - item: gmeow_gts::rdf_codecs::to_turtle_from_erased_source
    orphans: 8
  - item: gmeow_gts::rdf_codecs::to_turtle_from_source
    orphans: 9
  - item: 'gmeow_gts::rdf_events::<impl DeclarationOrderEmitter<''a,''s>>'
    orphans: 8
  - item: gmeow_gts::rdf_events::<impl Error for EventError>
    orphans: 1
  - item: gmeow_gts::rdf_events::<impl EventError>
    orphans: 7
  - item: gmeow_gts::rdf_events::<impl GraphRdfEventSource<'a>>
    orphans: 3
  - item: gmeow_gts::rdf_events::<impl RdfEventSource for GraphRdfEventSource<'_>>
    orphans: 3
  - item: gmeow_gts::rdf_events::<impl RdfEventSource for ReaderRdfEventSource<'_>>
    orphans: 3
  - item: gmeow_gts::rdf_events::<impl ReaderRdfEventSource<'a>>
    orphans: 4
  - item: gmeow_gts::rdf_events::<impl fmt::Display for EventError>
    orphans: 2
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter
    orphans: 7
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::emit_all
    orphans: 29
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::emit_reifier
    orphans: 13
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::emit_reifier_rows
    orphans: 17
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::emit_term
    orphans: 55
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::emit_triple_deps
    orphans: 7
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::new
    orphans: 9
  - item: gmeow_gts::rdf_events::DeclarationOrderEmitter::reifier_rows
    orphans: 8
  - item: gmeow_gts::rdf_events::EventAnnotation
    orphans: 12
  - item: gmeow_gts::rdf_events::EventDiagnostic
    orphans: 10
  - item: gmeow_gts::rdf_events::EventError
    orphans: 6
  - item: gmeow_gts::rdf_events::EventError::detail
    orphans: 5
  - item: gmeow_gts::rdf_events::EventError::fmt
    orphans: 3
  - item: gmeow_gts::rdf_events::EventError::invalid_source
    orphans: 4
  - item: gmeow_gts::rdf_events::EventError::kind
    orphans: 5
  - item: gmeow_gts::rdf_events::EventError::new
    orphans: 7
  - item: gmeow_gts::rdf_events::EventError::sink
    orphans: 4
  - item: gmeow_gts::rdf_events::EventError::triple_nesting_limit
    orphans: 8
  - item: gmeow_gts::rdf_events::EventErrorKind
    orphans: 18
  - item: gmeow_gts::rdf_events::EventLiteralDirection
    orphans: 8
  - item: gmeow_gts::rdf_events::EventLocation
    orphans: 10
  - item: gmeow_gts::rdf_events::EventQuad
    orphans: 12
  - item: gmeow_gts::rdf_events::EventReifier
    orphans: 14
  - item: gmeow_gts::rdf_events::EventScopeId
    orphans: 2
  - item: gmeow_gts::rdf_events::EventTerm
    orphans: 8
  - item: gmeow_gts::rdf_events::EventTermId
    orphans: 2
  - item: gmeow_gts::rdf_events::EventTermKind
    orphans: 33
  - item: gmeow_gts::rdf_events::EventTriple
    orphans: 10
  - item: gmeow_gts::rdf_events::GraphRdfEventSource
    orphans: 6
  - item: gmeow_gts::rdf_events::GraphRdfEventSource::drive
    orphans: 3
  - item: gmeow_gts::rdf_events::GraphRdfEventSource::drive_erased
    orphans: 3
  - item: gmeow_gts::rdf_events::GraphRdfEventSource::new
    orphans: 5
  - item: gmeow_gts::rdf_events::GraphRdfEventSource::with_scope
    orphans: 5
  - item: gmeow_gts::rdf_events::RdfDatasetVisitor
    orphans: 7
  - item: gmeow_gts::rdf_events::RdfDatasetVisitor::annotation
    orphans: 2
  - item: gmeow_gts::rdf_events::RdfDatasetVisitor::diagnostic
    orphans: 2
  - item: gmeow_gts::rdf_events::RdfDatasetVisitor::quad
    orphans: 2
  - item: gmeow_gts::rdf_events::RdfDatasetVisitor::reifier
    orphans: 2
  - item: gmeow_gts::rdf_events::RdfDatasetVisitor::term
    orphans: 2
  - item: gmeow_gts::rdf_events::RdfEventSink
    orphans: 15
  - item: gmeow_gts::rdf_events::RdfEventSink::annotation
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::declares_before_reference
    orphans: 4
  - item: gmeow_gts::rdf_events::RdfEventSink::diagnostic
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::end_scope
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::finish
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::quad
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::reifier
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::start_scope
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::term
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSink::triple_term_nesting_limit
    orphans: 4
  - item: gmeow_gts::rdf_events::RdfEventSource
    orphans: 4
  - item: gmeow_gts::rdf_events::RdfEventSource::drive
    orphans: 9
  - item: gmeow_gts::rdf_events::RdfEventSource::drive_erased
    orphans: 7
  - item: gmeow_gts::rdf_events::ReaderRdfEventSource
    orphans: 7
  - item: gmeow_gts::rdf_events::ReaderRdfEventSource::drive
    orphans: 4
  - item: gmeow_gts::rdf_events::ReaderRdfEventSource::drive_erased
    orphans: 4
  - item: gmeow_gts::rdf_events::ReaderRdfEventSource::new
    orphans: 5
  - item: gmeow_gts::rdf_events::ReaderRdfEventSource::with_options
    orphans: 9
  - item: gmeow_gts::rdf_events::ReaderRdfEventSource::with_scope
    orphans: 6
  - item: gmeow_gts::rdf_events::drive_graph
    orphans: 17
  - item: gmeow_gts::rdf_events::emit_fold_order
    orphans: 18
  - item: gmeow_gts::rdf_events::event_annotation
    orphans: 9
  - item: gmeow_gts::rdf_events::event_diagnostic
    orphans: 19
  - item: gmeow_gts::rdf_events::event_direction
    orphans: 7
  - item: gmeow_gts::rdf_events::event_id
    orphans: 5
  - item: gmeow_gts::rdf_events::event_quad
    orphans: 9
  - item: gmeow_gts::rdf_events::event_reifier
    orphans: 10
  - item: gmeow_gts::rdf_events::event_term
    orphans: 40
  - item: gmeow_gts::rdf_events::event_triple
    orphans: 8
  - item: gmeow_gts::rdf_events::read_to_rdf_events
    orphans: 14
  - item: gmeow_gts::rdf_events::validate_annotation_refs
    orphans: 7
  - item: gmeow_gts::rdf_events::validate_optional_term_ref
    orphans: 10
  - item: gmeow_gts::rdf_events::validate_quad_refs
    orphans: 10
  - item: gmeow_gts::rdf_events::validate_reifier_refs
    orphans: 6
  - item: gmeow_gts::rdf_events::validate_term_ref
    orphans: 9
  - item: gmeow_gts::rdf_events::validate_triple_refs
    orphans: 6
  - item: gmeow_gts::rdf_events::visit_dataset
    orphans: 18
  - item: gmeow_gts::rdf_xml::<impl Element>
    orphans: 5
  - item: gmeow_gts::rdf_xml::<impl Name>
    orphans: 5
  - item: gmeow_gts::rdf_xml::<impl ParseContext>
    orphans: 2
  - item: gmeow_gts::rdf_xml::<impl RdfXmlParser>
    orphans: 19
  - item: gmeow_gts::rdf_xml::<impl XmlDomParser>
    orphans: 2
  - item: gmeow_gts::rdf_xml::Attribute
    orphans: 5
  - item: gmeow_gts::rdf_xml::Element
    orphans: 10
  - item: gmeow_gts::rdf_xml::Element::attr_its
    orphans: 6
  - item: gmeow_gts::rdf_xml::Element::attr_rdf
    orphans: 6
  - item: gmeow_gts::rdf_xml::Element::attr_xml
    orphans: 6
  - item: gmeow_gts::rdf_xml::Element::property_attrs
    orphans: 22
  - item: gmeow_gts::rdf_xml::ITS_DIR
    orphans: 1
  - item: gmeow_gts::rdf_xml::ITS_NS
    orphans: 1
  - item: gmeow_gts::rdf_xml::ITS_VERSION
    orphans: 1
  - item: gmeow_gts::rdf_xml::Name
    orphans: 6
  - item: gmeow_gts::rdf_xml::Name::iri
    orphans: 8
  - item: gmeow_gts::rdf_xml::Name::is_its
    orphans: 3
  - item: gmeow_gts::rdf_xml::Name::is_rdf
    orphans: 3
  - item: gmeow_gts::rdf_xml::Name::is_xml
    orphans: 3
  - item: gmeow_gts::rdf_xml::ParseContext
    orphans: 11
  - item: gmeow_gts::rdf_xml::ParseContext::for_child
    orphans: 38
  - item: gmeow_gts::rdf_xml::RDF_ABOUT
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_ANNOTATION
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_ANNOTATION_NODE_ID
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_DATATYPE
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_DESCRIPTION
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_FIRST
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_ID
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_NIL
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_NODE_ID
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_NS
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_OBJECT
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_PARSE_TYPE
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_PREDICATE
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_RESOURCE
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_REST
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_STATEMENT
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_SUBJECT
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_TYPE
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_VERSION
    orphans: 1
  - item: gmeow_gts::rdf_xml::RDF_XML_LITERAL
    orphans: 1
  - item: gmeow_gts::rdf_xml::RdfXmlParser
    orphans: 5
  - item: gmeow_gts::rdf_xml::RdfXmlParser::context_literal
    orphans: 22
  - item: gmeow_gts::rdf_xml::RdfXmlParser::fresh_bnode
    orphans: 5
  - item: gmeow_gts::rdf_xml::RdfXmlParser::fresh_collection_bnode
    orphans: 5
  - item: gmeow_gts::rdf_xml::RdfXmlParser::insert_classic_reification
    orphans: 31
  - item: gmeow_gts::rdf_xml::RdfXmlParser::insert_property_attribute_statements
    orphans: 18
  - item: gmeow_gts::rdf_xml::RdfXmlParser::insert_reifier
    orphans: 16
  - item: gmeow_gts::rdf_xml::RdfXmlParser::insert_statement
    orphans: 25
  - item: gmeow_gts::rdf_xml::RdfXmlParser::iri_ref
    orphans: 10
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_collection
    orphans: 33
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_document
    orphans: 15
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_node_element
    orphans: 38
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_property_element
    orphans: 59
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_property_value
    orphans: 69
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_triple_element
    orphans: 43
  - item: gmeow_gts::rdf_xml::RdfXmlParser::parse_typed_property
    orphans: 76
  - item: gmeow_gts::rdf_xml::RdfXmlParser::rdf_id_iri
    orphans: 12
  - item: gmeow_gts::rdf_xml::RdfXmlParser::subject_for_node
    orphans: 16
  - item: gmeow_gts::rdf_xml::RdfXmlParser::triple_object
    orphans: 34
  - item: gmeow_gts::rdf_xml::XML_BASE
    orphans: 1
  - item: gmeow_gts::rdf_xml::XML_LANG
    orphans: 1
  - item: gmeow_gts::rdf_xml::XML_NS
    orphans: 1
  - item: gmeow_gts::rdf_xml::XSD_NS
    orphans: 1
  - item: gmeow_gts::rdf_xml::XmlDomParser
    orphans: 1
  - item: gmeow_gts::rdf_xml::XmlDomParser::parse
    orphans: 67
  - item: gmeow_gts::rdf_xml::XmlNode
    orphans: 5
  - item: gmeow_gts::rdf_xml::attach_element
    orphans: 14
  - item: gmeow_gts::rdf_xml::element_child
    orphans: 6
  - item: gmeow_gts::rdf_xml::element_text
    orphans: 10
  - item: gmeow_gts::rdf_xml::escape_xml_attr
    orphans: 3
  - item: gmeow_gts::rdf_xml::escape_xml_text
    orphans: 6
  - item: gmeow_gts::rdf_xml::expand_name
    orphans: 26
  - item: gmeow_gts::rdf_xml::from_rdf_xml
    orphans: 15
  - item: gmeow_gts::rdf_xml::has_iri_scheme
    orphans: 18
  - item: gmeow_gts::rdf_xml::initial_namespaces
    orphans: 6
  - item: gmeow_gts::rdf_xml::is_xml_name
    orphans: 10
  - item: gmeow_gts::rdf_xml::is_xml_name_char
    orphans: 3
  - item: gmeow_gts::rdf_xml::is_xml_name_start
    orphans: 3
  - item: gmeow_gts::rdf_xml::named_or_blank_term
    orphans: 6
  - item: gmeow_gts::rdf_xml::parse_start
    orphans: 51
  - item: gmeow_gts::rdf_xml::rdf_iri
    orphans: 3
  - item: gmeow_gts::rdf_xml::remove_dot_segments
    orphans: 31
  - item: gmeow_gts::rdf_xml::resolve_general_reference
    orphans: 30
  - item: gmeow_gts::rdf_xml::resolve_relative_iri
    orphans: 43
  - item: gmeow_gts::rdf_xml::serialize_children_as_xml
    orphans: 9
  - item: gmeow_gts::rdf_xml::serialize_xml_node
    orphans: 33
  - item: gmeow_gts::rdf_xml::serializer_namespaces
    orphans: 20
  - item: gmeow_gts::rdf_xml::serializer_qname
    orphans: 5
  - item: gmeow_gts::rdf_xml::split_base_for_path
    orphans: 14
  - item: gmeow_gts::rdf_xml::split_property_iri
    orphans: 9
  - item: gmeow_gts::rdf_xml::split_raw_path_suffix
    orphans: 4
  - item: gmeow_gts::rdf_xml::subject_key
    orphans: 6
  - item: gmeow_gts::rdf_xml::to_rdf_xml
    orphans: 59
  - item: gmeow_gts::rdf_xml::update_ns_order
    orphans: 12
  - item: gmeow_gts::rdf_xml::write_property
    orphans: 48
  - item: gmeow_gts::rdf_xml::write_triple_node
    orphans: 26
  - item: gmeow_gts::rdf_xml::xml_error
    orphans: 3
  - item: gmeow_gts::reader::<impl ActiveStreamingSegment>
    orphans: 4
  - item: 'gmeow_gts::reader::<impl Folder<''_,''_,''_>>'
    orphans: 25
  - item: gmeow_gts::reader::<impl From<CodecError> for PayloadError>
    orphans: 2
  - item: gmeow_gts::reader::<impl Read for StreamingCountingReader<R>>
    orphans: 2
  - item: gmeow_gts::reader::<impl ReadOptions<'a>>
    orphans: 3
  - item: gmeow_gts::reader::<impl fmt::Debug for ReadOptions<'_>>
    orphans: 2
  - item: gmeow_gts::reader::ActiveStreamingSegment
    orphans: 13
  - item: gmeow_gts::reader::ActiveStreamingSegment::finish_into_result
    orphans: 41
  - item: gmeow_gts::reader::ActiveStreamingSegment::new
    orphans: 80
  - item: gmeow_gts::reader::ActiveStreamingSegment::process_frame
    orphans: 85
  - item: gmeow_gts::reader::ContentKeyResolver
    orphans: 2
  - item: gmeow_gts::reader::FileSegments
    orphans: 13
  - item: gmeow_gts::reader::Folder
    orphans: 15
  - item: gmeow_gts::reader::Folder::diag
    orphans: 11
  - item: gmeow_gts::reader::Folder::emit_blob
    orphans: 9
  - item: gmeow_gts::reader::Folder::fold_frame
    orphans: 47
  - item: gmeow_gts::reader::Folder::h_annot
    orphans: 21
  - item: gmeow_gts::reader::Folder::h_blob_frame
    orphans: 69
  - item: gmeow_gts::reader::Folder::h_decoded_blob
    orphans: 30
  - item: gmeow_gts::reader::Folder::h_index
    orphans: 22
  - item: gmeow_gts::reader::Folder::h_meta
    orphans: 8
  - item: gmeow_gts::reader::Folder::h_opaque
    orphans: 16
  - item: gmeow_gts::reader::Folder::h_quads
    orphans: 39
  - item: gmeow_gts::reader::Folder::h_reifies
    orphans: 35
  - item: gmeow_gts::reader::Folder::h_snapshot
    orphans: 82
  - item: gmeow_gts::reader::Folder::h_suppress
    orphans: 21
  - item: gmeow_gts::reader::Folder::h_terms
    orphans: 74
  - item: gmeow_gts::reader::Folder::opaque
    orphans: 29
  - item: gmeow_gts::reader::Folder::payload
    orphans: 27
  - item: gmeow_gts::reader::Folder::push_opaque
    orphans: 6
  - item: gmeow_gts::reader::Folder::push_signature
    orphans: 6
  - item: gmeow_gts::reader::Folder::resolve_codecs
    orphans: 16
  - item: gmeow_gts::reader::Folder::with_sink
    orphans: 5
  - item: gmeow_gts::reader::PayloadError
    orphans: 9
  - item: gmeow_gts::reader::PayloadError::from
    orphans: 8
  - item: gmeow_gts::reader::ReadOptions
    orphans: 10
  - item: gmeow_gts::reader::ReadOptions::fmt
    orphans: 8
  - item: gmeow_gts::reader::ReadOptions::new
    orphans: 9
  - item: gmeow_gts::reader::ReadOptions::with_content_key
    orphans: 6
  - item: gmeow_gts::reader::StreamingCountingReader
    orphans: 4
  - item: gmeow_gts::reader::StreamingCountingReader::read
    orphans: 5
  - item: gmeow_gts::reader::StreamingReadResult
    orphans: 20
  - item: gmeow_gts::reader::StreamingSink
    orphans: 8
  - item: gmeow_gts::reader::StreamingSink::annotation
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::blob
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::diagnostic
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::opaque
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::quad
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::reifier
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::segment_head
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::signature
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::streamable_layout
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::suppression
    orphans: 2
  - item: gmeow_gts::reader::StreamingSink::term
    orphans: 2
  - item: gmeow_gts::reader::absorb_segment_result
    orphans: 17
  - item: gmeow_gts::reader::as_i128
    orphans: 7
  - item: gmeow_gts::reader::as_idx
    orphans: 4
  - item: gmeow_gts::reader::as_text
    orphans: 7
  - item: gmeow_gts::reader::as_triple_components
    orphans: 8
  - item: gmeow_gts::reader::catalog_from
    orphans: 17
  - item: gmeow_gts::reader::decrypt_codec
    orphans: 16
  - item: gmeow_gts::reader::diag_code_for
    orphans: 6
  - item: gmeow_gts::reader::fold_segment_frame
    orphans: 66
  - item: gmeow_gts::reader::is_header_item
    orphans: 12
  - item: gmeow_gts::reader::next_stream_item
    orphans: 10
  - item: gmeow_gts::reader::pub_digest
    orphans: 11
  - item: gmeow_gts::reader::push_diagnostic
    orphans: 10
  - item: gmeow_gts::reader::push_first_item_damaged
    orphans: 12
  - item: gmeow_gts::reader::push_result_diagnostic
    orphans: 8
  - item: gmeow_gts::reader::read
    orphans: 16
  - item: gmeow_gts::reader::read_file_segments
    orphans: 48
  - item: gmeow_gts::reader::read_segment
    orphans: 7
  - item: gmeow_gts::reader::read_segment_with_sink
    orphans: 111
  - item: gmeow_gts::reader::read_to_sink
    orphans: 15
  - item: gmeow_gts::reader::read_to_sink_from_reader
    orphans: 112
  - item: gmeow_gts::reader::read_to_sink_with_options
    orphans: 12
  - item: gmeow_gts::reader::read_with_options
    orphans: 86
  - item: gmeow_gts::reader::text_or
    orphans: 3
  - item: gmeow_gts::reader_layout::IndexRecord
    orphans: 7
  - item: gmeow_gts::reader_layout::check_index_mmr
    orphans: 48
  - item: gmeow_gts::reader_layout::layout_check
    orphans: 86
  - item: gmeow_gts::reader_reifiers::check_conflicting_reifiers
    orphans: 35
  - item: gmeow_gts::reader_rows::RowDecode
    orphans: 6
  - item: gmeow_gts::reader_rows::check_quad_positions
    orphans: 9
  - item: gmeow_gts::reader_rows::check_statement_positions
    orphans: 35
  - item: gmeow_gts::reader_rows::check_triple_term_positions
    orphans: 8
  - item: gmeow_gts::reader_rows::decode_annotation_row
    orphans: 36
  - item: gmeow_gts::reader_rows::decode_reifier_row
    orphans: 49
  - item: gmeow_gts::reader_rows::fmt_opt
    orphans: 6
  - item: gmeow_gts::reader_union::<impl Unioner>
    orphans: 4
  - item: gmeow_gts::reader_union::InternKey
    orphans: 7
  - item: gmeow_gts::reader_union::Unioner
    orphans: 5
  - item: gmeow_gts::reader_union::Unioner::key_for
    orphans: 31
  - item: gmeow_gts::reader_union::Unioner::map_term
    orphans: 56
  - item: gmeow_gts::reader_union::Unioner::remap_suppression
    orphans: 59
  - item: gmeow_gts::reader_union::union_segments
    orphans: 66
  - item: gmeow_gts::replication::<impl Inventory>
    orphans: 3
  - item: gmeow_gts::replication::ByteRange
    orphans: 8
  - item: gmeow_gts::replication::FrameInventory
    orphans: 18
  - item: gmeow_gts::replication::Inventory
    orphans: 14
  - item: gmeow_gts::replication::Inventory::has_problems
    orphans: 10
  - item: gmeow_gts::replication::Inventory::problem_detail
    orphans: 13
  - item: gmeow_gts::replication::MissingResult
    orphans: 14
  - item: gmeow_gts::replication::MissingStatus
    orphans: 12
  - item: gmeow_gts::replication::SegmentInventory
    orphans: 26
  - item: gmeow_gts::replication::aggregate_digest
    orphans: 12
  - item: gmeow_gts::replication::as_text
    orphans: 7
  - item: gmeow_gts::replication::collect_frames
    orphans: 57
  - item: gmeow_gts::replication::diagnostic_json
    orphans: 10
  - item: gmeow_gts::replication::diagnostics_json
    orphans: 10
  - item: gmeow_gts::replication::fatal_json
    orphans: 3
  - item: gmeow_gts::replication::header_computed_id
    orphans: 3
  - item: gmeow_gts::replication::header_profile
    orphans: 7
  - item: gmeow_gts::replication::header_stored_id
    orphans: 9
  - item: gmeow_gts::replication::heads_json
    orphans: 27
  - item: gmeow_gts::replication::inventory
    orphans: 82
  - item: gmeow_gts::replication::is_header_item
    orphans: 12
  - item: gmeow_gts::replication::item_end
    orphans: 5
  - item: gmeow_gts::replication::json_escape
    orphans: 17
  - item: gmeow_gts::replication::json_hex
    orphans: 3
  - item: gmeow_gts::replication::json_optional_hex
    orphans: 3
  - item: gmeow_gts::replication::json_string
    orphans: 3
  - item: gmeow_gts::replication::layout_json
    orphans: 9
  - item: gmeow_gts::replication::missing
    orphans: 70
  - item: gmeow_gts::replication::missing_json
    orphans: 27
  - item: gmeow_gts::replication::range_json
    orphans: 8
  - item: gmeow_gts::replication::resume_after
    orphans: 23
  - item: gmeow_gts::replication::segments_json
    orphans: 39
  - item: gmeow_gts::replication::tests::missing_returns_exact_tail_range
    orphans: 21
  - item: gmeow_gts::stream::AGENT
    orphans: 2
  - item: gmeow_gts::stream::COMPACTION
    orphans: 2
  - item: gmeow_gts::stream::COMPACT_AGENT
    orphans: 3
  - item: gmeow_gts::stream::COSE
    orphans: 2
  - item: gmeow_gts::stream::DETACHED_SIGNATURE
    orphans: 2
  - item: gmeow_gts::stream::DIGEST
    orphans: 2
  - item: gmeow_gts::stream::MANIFESTATION
    orphans: 2
  - item: gmeow_gts::stream::MEDIA_TYPE
    orphans: 2
  - item: gmeow_gts::stream::ORDER
    orphans: 2
  - item: gmeow_gts::stream::ROLE
    orphans: 2
  - item: gmeow_gts::stream::SEALED_SOURCE
    orphans: 2
  - item: gmeow_gts::stream::SIZE
    orphans: 2
  - item: gmeow_gts::stream::SOURCE_FRAME
    orphans: 2
  - item: gmeow_gts::stream::SOURCE_HEAD
    orphans: 2
  - item: gmeow_gts::stream::STREAM_NS
    orphans: 2
  - item: gmeow_gts::stream::TIMESTAMP
    orphans: 2
  - item: gmeow_gts::tar::<impl TarError>
    orphans: 2
  - item: gmeow_gts::tar::<impl fmt::Display for TarError>
    orphans: 2
  - item: gmeow_gts::tar::<impl std::error::Error for TarError>
    orphans: 1
  - item: gmeow_gts::tar::TarCompression
    orphans: 11
  - item: gmeow_gts::tar::TarError
    orphans: 5
  - item: gmeow_gts::tar::TarError::fmt
    orphans: 3
  - item: gmeow_gts::tar::TarError::new
    orphans: 6
  - item: gmeow_gts::tar::ToTarOptions
    orphans: 8
  - item: gmeow_gts::tar::append_device_entry
    orphans: 37
  - item: gmeow_gts::tar::append_entry
    orphans: 41
  - item: gmeow_gts::tar::append_file
    orphans: 22
  - item: gmeow_gts::tar::append_metadata_entry
    orphans: 20
  - item: gmeow_gts::tar::append_pax_records
    orphans: 15
  - item: gmeow_gts::tar::base_header
    orphans: 20
  - item: gmeow_gts::tar::default_mode
    orphans: 6
  - item: gmeow_gts::tar::parse_mtime
    orphans: 18
  - item: gmeow_gts::tar::required_link_target
    orphans: 7
  - item: gmeow_gts::tar::to_tar
    orphans: 33
  - item: gmeow_gts::tar::to_tar_vec
    orphans: 10
  - item: gmeow_gts::tar::write_tar_stream
    orphans: 19
  - item: gmeow_gts::test_crate::agent_memory::gts_verify
    orphans: 7
  - item: gmeow_gts::test_crate::agent_memory::rust_agent_memory_flow_is_append_only_and_verifyable
    orphans: 104
  - item: gmeow_gts::test_crate::agent_memory::rust_agent_memory_validates_inputs_and_empty_packages
    orphans: 25
  - item: gmeow_gts::test_crate::agent_memory::temp_path
    orphans: 12
  - item: gmeow_gts::test_crate::census::REQUIRED_RULES
    orphans: 2
  - item: gmeow_gts::test_crate::census::TestResult
    orphans: 1
  - item: gmeow_gts::test_crate::census::census_self_test_passes_every_detector_case
    orphans: 15
  - item: gmeow_gts::test_crate::census::checked_gate
    orphans: 9
  - item: gmeow_gts::test_crate::census::checked_report
    orphans: 3
  - item: gmeow_gts::test_crate::census::debt_path
    orphans: 3
  - item: gmeow_gts::test_crate::census::debt_record_is_canonical_and_the_sources_add_nothing_to_it
    orphans: 22
  - item: gmeow_gts::test_crate::census::ledger_is_canonical_and_declares_no_helper_or_provider
    orphans: 15
  - item: gmeow_gts::test_crate::census::ledger_path
    orphans: 3
  - item: gmeow_gts::test_crate::census::production_clippy_receipt_covers_this_snapshot_and_passed
    orphans: 21
  - item: gmeow_gts::test_crate::census::workspace_census_is_clean_with_the_representation_rules_required
    orphans: 14
  - item: gmeow_gts::test_crate::census::workspace_dir
    orphans: 4
  - item: gmeow_gts::test_crate::cli::accretive_file
    orphans: 40
  - item: gmeow_gts::test_crate::cli::cat_composes_clean_inputs_as_raw_concatenation
    orphans: 12
  - item: gmeow_gts::test_crate::cli::cat_refuses_a_composition_whose_suppressions_hide_everything
    orphans: 12
  - item: gmeow_gts::test_crate::cli::cat_refuses_a_damaged_input
    orphans: 9
  - item: gmeow_gts::test_crate::cli::cat_refuses_fewer_than_two_inputs
    orphans: 6
  - item: gmeow_gts::test_crate::cli::cat_refuses_suppress_everything_composition
    orphans: 10
  - item: gmeow_gts::test_crate::cli::compact_is_reproducible_with_fixed_timestamp
    orphans: 22
  - item: gmeow_gts::test_crate::cli::compact_refuses_evidence_without_seal_then_seals
    orphans: 83
  - item: gmeow_gts::test_crate::cli::compact_requires_streamable_flag
    orphans: 20
  - item: gmeow_gts::test_crate::cli::compact_verify_info_round_trip
    orphans: 36
  - item: gmeow_gts::test_crate::cli::diff_refuses_symlink_entry
    orphans: 28
  - item: gmeow_gts::test_crate::cli::diff_reports_changes
    orphans: 32
  - item: gmeow_gts::test_crate::cli::dump_files_profile_materializes_payload_once_by_default
    orphans: 43
  - item: gmeow_gts::test_crate::cli::dump_indexes_suppressed_payload_without_materializing_by_default
    orphans: 40
  - item: gmeow_gts::test_crate::cli::dump_metadata_only_skips_payload_materialization
    orphans: 33
  - item: gmeow_gts::test_crate::cli::dump_refuses_existing_destination_unless_forced
    orphans: 34
  - item: gmeow_gts::test_crate::cli::dump_writes_best_effort_output_for_damaged_archive
    orphans: 28
  - item: gmeow_gts::test_crate::cli::dump_writes_exploration_directory_for_minimal_archive
    orphans: 51
  - item: gmeow_gts::test_crate::cli::extract_key_matches_frozen_stdout
    orphans: 22
  - item: gmeow_gts::test_crate::cli::extract_key_missing_exits_1
    orphans: 6
  - item: gmeow_gts::test_crate::cli::extract_verifies_and_asserts_media_type
    orphans: 32
  - item: gmeow_gts::test_crate::cli::files_archive_with_path
    orphans: 76
  - item: gmeow_gts::test_crate::cli::fold_emits_nquads
    orphans: 12
  - item: gmeow_gts::test_crate::cli::fold_exits_nonzero_on_diagnostics
    orphans: 8
  - item: gmeow_gts::test_crate::cli::gts
    orphans: 6
  - item: gmeow_gts::test_crate::cli::gts_with_locale
    orphans: 10
  - item: gmeow_gts::test_crate::cli::info_reports_accretive_tail
    orphans: 11
  - item: gmeow_gts::test_crate::cli::info_reports_damage_healed_by_resync_as_not_clean
    orphans: 15
  - item: gmeow_gts::test_crate::cli::info_reports_v2_damage_and_refuses_unknown_generations
    orphans: 21
  - item: gmeow_gts::test_crate::cli::info_reports_v2_wire_frames_and_legacy_graphs
    orphans: 29
  - item: gmeow_gts::test_crate::cli::localized_help_and_unknown_command
    orphans: 20
  - item: gmeow_gts::test_crate::cli::ls_lists_digest_size_and_media_type
    orphans: 10
  - item: gmeow_gts::test_crate::cli::make_tree
    orphans: 5
  - item: gmeow_gts::test_crate::cli::okf_commands_report_feature_gate_when_disabled
    orphans: 13
  - item: gmeow_gts::test_crate::cli::pack_deduplicates_identical_content
    orphans: 26
  - item: gmeow_gts::test_crate::cli::pack_refuses_symlink_entry
    orphans: 22
  - item: gmeow_gts::test_crate::cli::pack_unpack_round_trip_bit_for_bit
    orphans: 50
  - item: gmeow_gts::test_crate::cli::prove_and_verify_proof_round_trip
    orphans: 80
  - item: gmeow_gts::test_crate::cli::rdf_codecs_commands_report_feature_gate_when_disabled
    orphans: 33
  - item: gmeow_gts::test_crate::cli::replication_verbs_handle_streamable_unknown_and_torn_inputs
    orphans: 55
  - item: gmeow_gts::test_crate::cli::replication_verbs_report_ranges_and_resume_bytes
    orphans: 57
  - item: gmeow_gts::test_crate::cli::suppressed_file_archive
    orphans: 84
  - item: gmeow_gts::test_crate::cli::tar_commands_report_feature_gate_when_disabled
    orphans: 18
  - item: gmeow_gts::test_crate::cli::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::cli::unpack_refuses_destination_symlink_escape
    orphans: 24
  - item: gmeow_gts::test_crate::cli::unpack_refuses_leaf_symlink_redirect
    orphans: 24
  - item: gmeow_gts::test_crate::cli::unpack_refuses_traversal
    orphans: 22
  - item: gmeow_gts::test_crate::cli::unpack_refuses_windows_style_paths
    orphans: 26
  - item: gmeow_gts::test_crate::cli::vectors
    orphans: 3
  - item: gmeow_gts::test_crate::cli::verify_declared_files_profile_object_only_is_not_unused
    orphans: 47
  - item: gmeow_gts::test_crate::cli::verify_flags_damage_with_exit_1
    orphans: 8
  - item: gmeow_gts::test_crate::cli::verify_flags_undeclared_files_profile
    orphans: 45
  - item: gmeow_gts::test_crate::cli::verify_flags_undeclared_files_profile_object_only
    orphans: 47
  - item: gmeow_gts::test_crate::cli::verify_policy_file_trusts_did_style_signer
    orphans: 80
  - item: gmeow_gts::test_crate::cli::verify_refuses_streamable_lie
    orphans: 10
  - item: gmeow_gts::test_crate::cli::verify_warns_on_stream_vocab_without_claim
    orphans: 49
  - item: gmeow_gts::test_crate::conformance::blob_summary
    orphans: 29
  - item: gmeow_gts::test_crate::conformance::compact_reproduces_the_frozen_25b_bytes
    orphans: 17
  - item: gmeow_gts::test_crate::conformance::corpus_matches_frozen_expectations
    orphans: 60
  - item: gmeow_gts::test_crate::conformance::manifest_top_level_vectors
    orphans: 56
  - item: gmeow_gts::test_crate::conformance::prefix_fold_streaming_property
    orphans: 53
  - item: gmeow_gts::test_crate::conformance::summarize
    orphans: 25
  - item: gmeow_gts::test_crate::cose::arr32
    orphans: 3
  - item: gmeow_gts::test_crate::cose::cose_sign1_vectors_round_trip
    orphans: 44
  - item: gmeow_gts::test_crate::cose::unhex
    orphans: 6
  - item: gmeow_gts::test_crate::db::command_available
    orphans: 6
  - item: gmeow_gts::test_crate::db::duckdb_commands_require_feature
    orphans: 18
  - item: gmeow_gts::test_crate::db::duckdb_query
    orphans: 9
  - item: gmeow_gts::test_crate::db::fake_failing_tool
    orphans: 20
  - item: gmeow_gts::test_crate::db::gts
    orphans: 6
  - item: gmeow_gts::test_crate::db::gts_with_path
    orphans: 7
  - item: gmeow_gts::test_crate::db::prepend_path
    orphans: 7
  - item: gmeow_gts::test_crate::db::skip_without_duckdb
    orphans: 9
  - item: gmeow_gts::test_crate::db::sqlite_query
    orphans: 8
  - item: gmeow_gts::test_crate::db::stdout
    orphans: 8
  - item: gmeow_gts::test_crate::db::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::db::to_duckdb_exports_schema_and_rows
    orphans: 35
  - item: gmeow_gts::test_crate::db::to_parquet_cleans_staging_dir_when_replace_fails
    orphans: 32
  - item: gmeow_gts::test_crate::db::to_parquet_writes_non_empty_tables
    orphans: 42
  - item: gmeow_gts::test_crate::db::to_sqlite_exports_schema_and_rows
    orphans: 35
  - item: gmeow_gts::test_crate::db::to_sqlite_preserves_existing_output_when_blob_decode_fails
    orphans: 31
  - item: gmeow_gts::test_crate::db::to_sqlite_preserves_existing_output_when_tool_fails
    orphans: 22
  - item: gmeow_gts::test_crate::db::to_sqlite_refuses_damaged_input_before_writing
    orphans: 17
  - item: gmeow_gts::test_crate::db::vectors
    orphans: 3
  - item: gmeow_gts::test_crate::deterministic_writer::CAT
    orphans: 1
  - item: gmeow_gts::test_crate::deterministic_writer::LABEL
    orphans: 1
  - item: gmeow_gts::test_crate::deterministic_writer::blob_frame_media_types
    orphans: 20
  - item: gmeow_gts::test_crate::deterministic_writer::blob_meta
    orphans: 3
  - item: gmeow_gts::test_crate::deterministic_writer::blob_reps
    orphans: 13
  - item: gmeow_gts::test_crate::deterministic_writer::deterministic_graphs
    orphans: 54
  - item: gmeow_gts::test_crate::deterministic_writer::deterministic_writer_matches_frozen_vector
    orphans: 10
  - item: gmeow_gts::test_crate::deterministic_writer::deterministic_writer_orders_nested_triple_terms_by_depth
    orphans: 45
  - item: gmeow_gts::test_crate::deterministic_writer::deterministic_writer_reorders_equivalent_graphs
    orphans: 11
  - item: gmeow_gts::test_crate::deterministic_writer::frame_transform_ids
    orphans: 22
  - item: gmeow_gts::test_crate::deterministic_writer::frame_types
    orphans: 15
  - item: gmeow_gts::test_crate::deterministic_writer::graph_snapshot_payload_folds_like_deterministic_writer
    orphans: 25
  - item: gmeow_gts::test_crate::deterministic_writer::graph_snapshot_payload_orders_quads_by_graph_then_spo
    orphans: 27
  - item: gmeow_gts::test_crate::deterministic_writer::iri
    orphans: 11
  - item: gmeow_gts::test_crate::deterministic_writer::literal
    orphans: 11
  - item: gmeow_gts::test_crate::deterministic_writer::snapshot_blob_sort_uses_media_type_as_final_tie_breaker
    orphans: 28
  - item: gmeow_gts::test_crate::deterministic_writer::snapshot_from_graph_emits_sorted_content_addressed_blobs_before_snapshot
    orphans: 31
  - item: gmeow_gts::test_crate::deterministic_writer::snapshot_from_graph_signs_transport_key_and_snapshot_frame
    orphans: 35
  - item: gmeow_gts::test_crate::deterministic_writer::snapshot_quad_rows
    orphans: 14
  - item: gmeow_gts::test_crate::deterministic_writer::snapshot_transform_threshold_only_rewrites_default_zstd
    orphans: 37
  - item: gmeow_gts::test_crate::deterministic_writer::term_target
    orphans: 6
  - item: gmeow_gts::test_crate::deterministic_writer::triple_term
    orphans: 11
  - item: gmeow_gts::test_crate::encrypt0::encrypt0_random_iv_round_trip
    orphans: 10
  - item: gmeow_gts::test_crate::encrypt0::encrypt0_vector_seals_and_opens
    orphans: 33
  - item: gmeow_gts::test_crate::encrypt0::unhex
    orphans: 6
  - item: gmeow_gts::test_crate::files_v2::bnode
    orphans: 11
  - item: gmeow_gts::test_crate::files_v2::default_unpack_refuses_v2_links_and_special_files
    orphans: 25
  - item: gmeow_gts::test_crate::files_v2::dumpdir_surfaces_v2_fields_in_entries_jsonl
    orphans: 27
  - item: gmeow_gts::test_crate::files_v2::gts
    orphans: 6
  - item: gmeow_gts::test_crate::files_v2::iri
    orphans: 11
  - item: gmeow_gts::test_crate::files_v2::literal
    orphans: 11
  - item: gmeow_gts::test_crate::files_v2::meta_u64
    orphans: 11
  - item: gmeow_gts::test_crate::files_v2::opt_in_unpack_materializes_in_destination_symlinks
    orphans: 33
  - item: gmeow_gts::test_crate::files_v2::sample_v2_entries
    orphans: 81
  - item: gmeow_gts::test_crate::files_v2::symlink_opt_in_still_refuses_destination_escape
    orphans: 21
  - item: gmeow_gts::test_crate::files_v2::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::files_v2::unpack_strips_setid_bits_by_default
    orphans: 27
  - item: gmeow_gts::test_crate::files_v2::v1_archive_reads_as_default_file_entries
    orphans: 17
  - item: gmeow_gts::test_crate::files_v2::v2_emission_is_deterministic_after_sorting
    orphans: 8
  - item: gmeow_gts::test_crate::files_v2::v2_entries_round_trip_all_metadata
    orphans: 47
  - item: gmeow_gts::test_crate::files_v2::v2_reader_accepts_duplicate_iri_term_ids
    orphans: 23
  - item: gmeow_gts::test_crate::files_v2::v2_unpack_materializes_directories_and_regular_files
    orphans: 29
  - item: gmeow_gts::test_crate::from_nquads::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::test_crate::from_nquads::cli_from_nq_inverts_fold
    orphans: 37
  - item: gmeow_gts::test_crate::from_nquads::cli_from_nq_writes_stdout_when_no_out_path
    orphans: 21
  - item: gmeow_gts::test_crate::from_nquads::compact_blank_node_and_language_tag_delimiters_roundtrip
    orphans: 10
  - item: gmeow_gts::test_crate::from_nquads::directional_language_literals_roundtrip
    orphans: 17
  - item: gmeow_gts::test_crate::from_nquads::fold_roundtrips_through_from_nquads_for_graph_vectors
    orphans: 16
  - item: gmeow_gts::test_crate::from_nquads::gts
    orphans: 6
  - item: gmeow_gts::test_crate::from_nquads::literals_lang_and_datatype_roundtrip
    orphans: 11
  - item: gmeow_gts::test_crate::from_nquads::named_graph_reifier_and_annotation_roundtrip
    orphans: 64
  - item: gmeow_gts::test_crate::from_nquads::nquads_import_preserves_multiple_reifiers_for_the_same_statement
    orphans: 11
  - item: gmeow_gts::test_crate::from_nquads::parses_backspace_and_formfeed_escapes
    orphans: 9
  - item: gmeow_gts::test_crate::from_nquads::quoted_triple_adjacent_delimiters_roundtrip
    orphans: 12
  - item: gmeow_gts::test_crate::from_nquads::rejects_empty_blank_node_label_and_language_tag
    orphans: 10
  - item: gmeow_gts::test_crate::from_nquads::rejects_forbidden_iri_characters
    orphans: 14
  - item: gmeow_gts::test_crate::from_nquads::rejects_invalid_rdf_term_positions
    orphans: 14
  - item: gmeow_gts::test_crate::from_nquads::rejects_malformed_nquads
    orphans: 6
  - item: gmeow_gts::test_crate::from_nquads::rejects_malformed_unicode_escape_without_panicking
    orphans: 6
  - item: gmeow_gts::test_crate::from_nquads::rejects_unknown_literal_escape
    orphans: 6
  - item: gmeow_gts::test_crate::from_nquads::roundtrip
    orphans: 6
  - item: gmeow_gts::test_crate::from_nquads::sorted_lines
    orphans: 9
  - item: gmeow_gts::test_crate::from_nquads::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::from_nquads::vectors
    orphans: 3
  - item: gmeow_gts::test_crate::from_nquads::writer_allows_multiple_reifiers_for_the_same_statement
    orphans: 67
  - item: gmeow_gts::test_crate::from_trig::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::test_crate::from_trig::cli_from_trig_inverts_to_trig
    orphans: 44
  - item: gmeow_gts::test_crate::from_trig::fold_roundtrips_through_trig_for_graph_vectors
    orphans: 19
  - item: gmeow_gts::test_crate::from_trig::gts
    orphans: 6
  - item: gmeow_gts::test_crate::from_trig::parses_graph_keyword_and_a_predicate
    orphans: 16
  - item: gmeow_gts::test_crate::from_trig::parses_predicate_object_shorthand_and_lists
    orphans: 18
  - item: gmeow_gts::test_crate::from_trig::parses_prefixes_graph_blocks_reifiers_and_literals
    orphans: 22
  - item: gmeow_gts::test_crate::from_trig::pn_local_esc_keeps_escaped_delimiters_in_local_name
    orphans: 32
  - item: gmeow_gts::test_crate::from_trig::prefixed_names_stop_before_quoted_triple_close
    orphans: 13
  - item: gmeow_gts::test_crate::from_trig::rejects_forbidden_raw_iri_characters
    orphans: 14
  - item: gmeow_gts::test_crate::from_trig::rejects_malformed_or_unsupported_trig
    orphans: 6
  - item: gmeow_gts::test_crate::from_trig::resolves_base_iri_relative_references
    orphans: 16
  - item: gmeow_gts::test_crate::from_trig::sorted_lines
    orphans: 9
  - item: gmeow_gts::test_crate::from_trig::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::from_trig::to_trig_groups_named_graphs_and_keeps_reifiers
    orphans: 70
  - item: gmeow_gts::test_crate::from_trig::trig_preserves_directional_language_literals
    orphans: 15
  - item: gmeow_gts::test_crate::from_trig::trig_roundtrip_nquads
    orphans: 8
  - item: gmeow_gts::test_crate::from_trig::vectors
    orphans: 3
  - item: gmeow_gts::test_crate::graph_iter::into_quads_yields_existing_quad_rows
    orphans: 10
  - item: gmeow_gts::test_crate::graph_iter::iri
    orphans: 11
  - item: gmeow_gts::test_crate::graph_iter::quad_terms_resolves_term_references_lazily
    orphans: 27
  - item: gmeow_gts::test_crate::graph_iter::sample_graph
    orphans: 12
  - item: gmeow_gts::test_crate::lazy_blobs::BLOB_MT
    orphans: 1
  - item: gmeow_gts::test_crate::lazy_blobs::TestQuad
    orphans: 1
  - item: gmeow_gts::test_crate::lazy_blobs::blob_access_decompresses_and_caches
    orphans: 15
  - item: gmeow_gts::test_crate::lazy_blobs::blob_contract_preserves_inline_bytes_and_leaves_external_bytes_out_of_band
    orphans: 25
  - item: gmeow_gts::test_crate::lazy_blobs::blob_deferred_until_access
    orphans: 25
  - item: gmeow_gts::test_crate::lazy_blobs::blob_frame
    orphans: 28
  - item: gmeow_gts::test_crate::lazy_blobs::blob_payload_of_concatenated_zstd_frames_decodes_to_the_end
    orphans: 10
  - item: gmeow_gts::test_crate::lazy_blobs::blob_payload_that_is_not_whole_zstd_frames_is_one_opaque_node
    orphans: 20
  - item: gmeow_gts::test_crate::lazy_blobs::blob_pub
    orphans: 7
  - item: gmeow_gts::test_crate::lazy_blobs::damage_inside_a_block_with_valid_structure_fails_cleanly_on_first_read
    orphans: 28
  - item: gmeow_gts::test_crate::lazy_blobs::file_with_zstd_payload
    orphans: 15
  - item: gmeow_gts::test_crate::lazy_blobs::iv
    orphans: 3
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_decode_failure_raises_on_access
    orphans: 20
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_external_records_layout_iou
    orphans: 20
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_identity_codec_is_direct
    orphans: 12
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_lookup_and_iteration_helpers
    orphans: 22
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_meta_is_eager
    orphans: 22
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_multi_segment_union_does_not_decode
    orphans: 11
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_no_pub_digest_falls_back_to_eager_decode
    orphans: 19
  - item: gmeow_gts::test_crate::lazy_blobs::lazy_blob_unknown_codec_degrades
    orphans: 21
  - item: gmeow_gts::test_crate::lazy_blobs::term_triple
    orphans: 34
  - item: gmeow_gts::test_crate::lazy_blobs::zstd_blob_file
    orphans: 12
  - item: gmeow_gts::test_crate::lazy_blobs::zstd_bytes
    orphans: 6
  - item: gmeow_gts::test_crate::lazy_blobs::zstd_frame_declaring_a_window_the_decoder_refuses_is_one_opaque_node
    orphans: 30
  - item: gmeow_gts::test_crate::lazy_blobs::zstd_frame_naming_a_dictionary_is_one_opaque_node
    orphans: 9
  - item: gmeow_gts::test_crate::mmr::detached_proof_rejects_tampered_path
    orphans: 7
  - item: gmeow_gts::test_crate::mmr::detached_proof_verifies_without_source_file
    orphans: 8
  - item: gmeow_gts::test_crate::mmr::iv
    orphans: 3
  - item: gmeow_gts::test_crate::mmr::negative_proof_fixture_fails
    orphans: 6
  - item: gmeow_gts::test_crate::mmr::positive_proof_fixture_verifies
    orphans: 6
  - item: gmeow_gts::test_crate::mmr::proofs
    orphans: 3
  - item: gmeow_gts::test_crate::mmr::reader_diagnoses_bad_index_mmr
    orphans: 26
  - item: gmeow_gts::test_crate::native_store::graph_into_quads_returns_gts_sidecar
    orphans: 21
  - item: gmeow_gts::test_crate::native_store::graph_to_store_preserves_named_graphs_and_sidecar
    orphans: 28
  - item: gmeow_gts::test_crate::native_store::iri_term
    orphans: 11
  - item: gmeow_gts::test_crate::native_store::sidecar_clones_all_gts_only_fields
    orphans: 60
  - item: gmeow_gts::test_crate::native_store::sorted_store
    orphans: 5
  - item: gmeow_gts::test_crate::native_store::store_to_writer_roundtrips_native_quads
    orphans: 20
  - item: gmeow_gts::test_crate::nested::EX
    orphans: 1
  - item: gmeow_gts::test_crate::nested::bundle
    orphans: 5
  - item: gmeow_gts::test_crate::nested::iri
    orphans: 11
  - item: gmeow_gts::test_crate::nested::labeled_bundle
    orphans: 11
  - item: gmeow_gts::test_crate::nested::lit
    orphans: 11
  - item: gmeow_gts::test_crate::nested::nested_recursion_security_vector_descriptor_is_still_present
    orphans: 8
  - item: gmeow_gts::test_crate::nested::read_nested_charges_duplicate_nested_digest_once
    orphans: 23
  - item: gmeow_gts::test_crate::nested::read_nested_exposes_subgraph_by_blob_digest
    orphans: 18
  - item: gmeow_gts::test_crate::nested::read_nested_records_damaged_nested_payload
    orphans: 10
  - item: gmeow_gts::test_crate::nested::read_nested_stops_at_decoded_size_budget
    orphans: 13
  - item: gmeow_gts::test_crate::nested::read_nested_stops_at_recursion_limit
    orphans: 15
  - item: gmeow_gts::test_crate::nested::tiny_graph
    orphans: 10
  - item: gmeow_gts::test_crate::okf::body_after_frontmatter
    orphans: 15
  - item: gmeow_gts::test_crate::okf::cli_from_okf_inverts_to_okf
    orphans: 43
  - item: gmeow_gts::test_crate::okf::from_okf_maps_frontmatter_body_and_links
    orphans: 34
  - item: gmeow_gts::test_crate::okf::frontmatterless_index_navigation_pages_are_ignored
    orphans: 28
  - item: gmeow_gts::test_crate::okf::frontmatterless_non_index_markdown_is_rejected
    orphans: 9
  - item: gmeow_gts::test_crate::okf::graph_lines
    orphans: 3
  - item: gmeow_gts::test_crate::okf::gts
    orphans: 6
  - item: gmeow_gts::test_crate::okf::inline_body_profile_variant_exports_only_when_requested
    orphans: 33
  - item: gmeow_gts::test_crate::okf::link_extraction_handles_utf8_link_text
    orphans: 26
  - item: gmeow_gts::test_crate::okf::malformed_bundle_missing_type_is_rejected
    orphans: 15
  - item: gmeow_gts::test_crate::okf::okf_forward_roundtrip_restores_body_bytes_and_normalizes_tags
    orphans: 24
  - item: gmeow_gts::test_crate::okf::okf_reverse_roundtrip_preserves_folded_profile_graph
    orphans: 15
  - item: gmeow_gts::test_crate::okf::sample_bundle
    orphans: 37
  - item: gmeow_gts::test_crate::okf::sorted_lines
    orphans: 9
  - item: gmeow_gts::test_crate::okf::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::okf::to_okf_writes_unmapped_sidecar_instead_of_dropping_rdf
    orphans: 45
  - item: gmeow_gts::test_crate::okf::write
    orphans: 6
  - item: gmeow_gts::test_crate::okf_corpus::contains_markdown
    orphans: 20
  - item: gmeow_gts::test_crate::okf_corpus::corpus_dir
    orphans: 3
  - item: gmeow_gts::test_crate::okf_corpus::folded_lines
    orphans: 3
  - item: gmeow_gts::test_crate::okf_corpus::okf_bundles
    orphans: 26
  - item: gmeow_gts::test_crate::okf_corpus::okf_corpus_matches_folded_expectations
    orphans: 19
  - item: gmeow_gts::test_crate::okf_corpus::okf_corpus_round_trips_through_export
    orphans: 31
  - item: gmeow_gts::test_crate::okf_corpus::okf_corpus_unmapped_sidecar_is_pinned
    orphans: 52
  - item: gmeow_gts::test_crate::okf_corpus::sorted_lines
    orphans: 9
  - item: gmeow_gts::test_crate::okf_corpus::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::policy::EX
    orphans: 1
  - item: gmeow_gts::test_crate::policy::KID
    orphans: 1
  - item: gmeow_gts::test_crate::policy::claim_terms
    orphans: 7
  - item: gmeow_gts::test_crate::policy::evidence_profile_requires_signatures_and_head_commitment
    orphans: 13
  - item: gmeow_gts::test_crate::policy::evidence_sealed_source_satisfies_unsigned_source_commitment_policy
    orphans: 17
  - item: gmeow_gts::test_crate::policy::finding_codes
    orphans: 6
  - item: gmeow_gts::test_crate::policy::invalid_and_unverified_signature_statuses_are_policy_errors
    orphans: 20
  - item: gmeow_gts::test_crate::policy::iri
    orphans: 11
  - item: gmeow_gts::test_crate::policy::lit
    orphans: 11
  - item: gmeow_gts::test_crate::policy::opaque_profile_requires_pseudonymous_recipient_kids
    orphans: 45
  - item: gmeow_gts::test_crate::policy::profile_and_stream_vocabulary_findings_match_policy_codes
    orphans: 37
  - item: gmeow_gts::test_crate::policy::profile_policy_security_vector_descriptor_is_still_present
    orphans: 12
  - item: gmeow_gts::test_crate::policy::signed_graph
    orphans: 13
  - item: gmeow_gts::test_crate::policy::trust_policy_loads_from_json
    orphans: 14
  - item: gmeow_gts::test_crate::policy::trust_policy_loads_from_yaml
    orphans: 14
  - item: gmeow_gts::test_crate::policy::trusted_signer_policy_rejects_valid_but_unauthorized_signer
    orphans: 9
  - item: gmeow_gts::test_crate::policy::valid_signature_does_not_imply_trusted_signer_or_true_claim
    orphans: 23
  - item: gmeow_gts::test_crate::rdf::RDFS_LABEL
    orphans: 1
  - item: gmeow_gts::test_crate::rdf::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::test_crate::rdf::XSD_INTEGER
    orphans: 1
  - item: gmeow_gts::test_crate::rdf::generated_blank_node_labels_do_not_collide_with_explicit_labels
    orphans: 55
  - item: gmeow_gts::test_crate::rdf::gts_reifier_projection_uses_native_rdf12_triple_terms
    orphans: 62
  - item: gmeow_gts::test_crate::rdf::native_rdf12_triple_terms_import_as_gts_reifiers
    orphans: 23
  - item: gmeow_gts::test_crate::rdf::native_rdf_dataset_roundtrips_directional_literals
    orphans: 59
  - item: gmeow_gts::test_crate::rdf::native_rdf_dataset_roundtrips_through_gts
    orphans: 36
  - item: gmeow_gts::test_crate::rdf::native_rdf_import_keeps_every_reifier_binding
    orphans: 35
  - item: gmeow_gts::test_crate::rdf::native_rdf_lexical_validators_reject_unsafe_values
    orphans: 20
  - item: gmeow_gts::test_crate::rdf::sorted_dataset
    orphans: 5
  - item: gmeow_gts::test_crate::rdf::strict_export_refuses_unrepresentable_quoted_triple_positions
    orphans: 55
  - item: gmeow_gts::test_crate::rdf::strict_export_rejects_quoted_triple_graph_names
    orphans: 50
  - item: gmeow_gts::test_crate::rdf_codecs::W3C_NTRIPLES_NEGATIVE_SYNTAX
    orphans: 90
  - item: gmeow_gts::test_crate::rdf_codecs::W3C_NTRIPLES_POSITIVE_SYNTAX
    orphans: 33
  - item: gmeow_gts::test_crate::rdf_codecs::W3cRdfXmlCase
    orphans: 5
  - item: gmeow_gts::test_crate::rdf_codecs::angle_value_after
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::assert_nquads_isomorphic_to_ntriples
    orphans: 12
  - item: gmeow_gts::test_crate::rdf_codecs::canonical_native_lines
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::canonicalize_blank_nodes
    orphans: 36
  - item: gmeow_gts::test_crate::rdf_codecs::cli_turtle_and_feature_trig_imports_roundtrip
    orphans: 86
  - item: gmeow_gts::test_crate::rdf_codecs::event_graph_materialization_surfaces_ill_typed_xsd_diagnostics
    orphans: 53
  - item: gmeow_gts::test_crate::rdf_codecs::graph_meta
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::gts
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::gts_ok
    orphans: 10
  - item: gmeow_gts::test_crate::rdf_codecs::is_blank_label_char
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_codecs::is_manifest_subject_token
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_codecs::manifest_entry_tokens
    orphans: 26
  - item: gmeow_gts::test_crate::rdf_codecs::map_value
    orphans: 9
  - item: gmeow_gts::test_crate::rdf_codecs::native_codecs_roundtrip_long_private_use_language_subtags
    orphans: 26
  - item: gmeow_gts::test_crate::rdf_codecs::nquads_from_gts
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_codecs::ntriples_import_preserves_and_flags_ill_typed_xsd_literals
    orphans: 27
  - item: gmeow_gts::test_crate::rdf_codecs::ntriples_isomorphism_helper_accepts_renamed_blank_nodes
    orphans: 8
  - item: gmeow_gts::test_crate::rdf_codecs::ntriples_parser_accepts_w3c_rdf12_positive_syntax_suite
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::ntriples_parser_accepts_w3c_syntax_cases_and_rdf12_triple_terms
    orphans: 21
  - item: gmeow_gts::test_crate::rdf_codecs::ntriples_parser_rejects_w3c_rdf12_negative_syntax_suite
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::ntriples_serialization_roundtrips_through_event_source
    orphans: 20
  - item: gmeow_gts::test_crate::rdf_codecs::push_w3c_manifest_case
    orphans: 19
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_accepts_core_w3c_rdf_xml_shapes
    orphans: 75
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_accepts_w3c_rdf12_triple_terms_and_annotations
    orphans: 26
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_passes_w3c_suite_when_fixture_root_is_set
    orphans: 49
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_preserves_empty_property_attributes_on_resource_objects
    orphans: 19
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_rejects_w3c_rdf12_bad_triple_terms
    orphans: 29
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_resolves_rdf_id_against_base_without_fragment
    orphans: 27
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_parser_resolves_text_entity_references
    orphans: 13
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_serialization_roundtrips_unicode_predicate_local_names
    orphans: 27
  - item: gmeow_gts::test_crate::rdf_codecs::rdf_xml_serialization_uses_event_source_and_rejects_named_graphs
    orphans: 18
  - item: gmeow_gts::test_crate::rdf_codecs::sample_graph
    orphans: 23
  - item: gmeow_gts::test_crate::rdf_codecs::sorted_lines
    orphans: 9
  - item: gmeow_gts::test_crate::rdf_codecs::term
    orphans: 11
  - item: gmeow_gts::test_crate::rdf_codecs::text_value
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_codecs::trig_parser_accepts_named_graphs_and_rdf12_triple_forms
    orphans: 22
  - item: gmeow_gts::test_crate::rdf_codecs::trig_parser_allows_comment_only_empty_structures_inside_rdf12_triples
    orphans: 16
  - item: gmeow_gts::test_crate::rdf_codecs::trig_parser_rejects_non_empty_structures_inside_rdf12_triples_after_comments
    orphans: 18
  - item: gmeow_gts::test_crate::rdf_codecs::trig_serialization_roundtrips_through_event_source
    orphans: 12
  - item: gmeow_gts::test_crate::rdf_codecs::turtle_parser_accepts_bare_numeric_boolean_and_long_string_literals
    orphans: 63
  - item: gmeow_gts::test_crate::rdf_codecs::turtle_parser_accepts_prefixed_names_with_internal_dots
    orphans: 19
  - item: gmeow_gts::test_crate::rdf_codecs::turtle_parser_accepts_shared_turtle_grammar
    orphans: 21
  - item: gmeow_gts::test_crate::rdf_codecs::turtle_parser_expands_rdf12_reifying_triples_and_annotations
    orphans: 64
  - item: gmeow_gts::test_crate::rdf_codecs::turtle_serialization_uses_event_source_and_rejects_named_graphs
    orphans: 16
  - item: gmeow_gts::test_crate::rdf_codecs::w3c_manifest_cases
    orphans: 43
  - item: gmeow_gts::test_crate::rdf_events::<impl RdfDatasetVisitor for VisitorCounts>
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_events::<impl RdfEventSink for RecordingSink>
    orphans: 10
  - item: gmeow_gts::test_crate::rdf_events::<impl RecordingSink>
    orphans: 2
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink
    orphans: 5
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::annotation
    orphans: 7
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::declares_before_reference
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::diagnostic
    orphans: 7
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::end_scope
    orphans: 4
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::finish
    orphans: 4
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::quad
    orphans: 7
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::reifier
    orphans: 7
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::start_scope
    orphans: 4
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::strict
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_events::RecordingSink::term
    orphans: 4
  - item: gmeow_gts::test_crate::rdf_events::VisitorCounts
    orphans: 8
  - item: gmeow_gts::test_crate::rdf_events::VisitorCounts::annotation
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_events::VisitorCounts::diagnostic
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_events::VisitorCounts::quad
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_events::VisitorCounts::reifier
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_events::VisitorCounts::term
    orphans: 3
  - item: gmeow_gts::test_crate::rdf_events::dataset_visitor_is_infallible_folded_graph_visitor
    orphans: 12
  - item: gmeow_gts::test_crate::rdf_events::graph_source_drives_generic_and_erased_sinks
    orphans: 31
  - item: gmeow_gts::test_crate::rdf_events::graph_with_forward_reifier
    orphans: 40
  - item: gmeow_gts::test_crate::rdf_events::lenient_sink_accepts_forward_reifier_order
    orphans: 23
  - item: gmeow_gts::test_crate::rdf_events::reader_source_drives_events_from_gts_bytes
    orphans: 29
  - item: gmeow_gts::test_crate::rdf_events::source_rejects_out_of_range_quad_reference
    orphans: 19
  - item: gmeow_gts::test_crate::rdf_events::strict_sink_gets_declarations_before_references
    orphans: 57
  - item: gmeow_gts::test_crate::rdf_events::term
    orphans: 11
  - item: gmeow_gts::test_crate::rdf_feature_gate::default_features::assert_dependency_table
    orphans: 59
  - item: gmeow_gts::test_crate::rdf_feature_gate::default_features::assert_feature_table
    orphans: 38
  - item: gmeow_gts::test_crate::rdf_feature_gate::default_features::gmeow_gts_package
    orphans: 24
  - item: gmeow_gts::test_crate::rdf_feature_gate::default_features::optional_adapters_are_not_enabled_by_default
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_feature_gate::native_store_feature_enables_adapter_module
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_feature_gate::rdf_codecs_feature_enables_rdf_text_codec_modules
    orphans: 21
  - item: gmeow_gts::test_crate::rdf_feature_gate::rdf_feature_enables_adapter_module
    orphans: 6
  - item: gmeow_gts::test_crate::rdf_feature_gate::yaml_ld_feature_enables_codec_modules
    orphans: 8
  - item: gmeow_gts::test_crate::reader::conflicting_reifier_is_only_the_legacy_indirect_term
    orphans: 10
  - item: gmeow_gts::test_crate::reader::diagnostic_codes
    orphans: 7
  - item: gmeow_gts::test_crate::reader::forward_referencing_tt_is_diagnosed_and_dropped
    orphans: 13
  - item: gmeow_gts::test_crate::reader::identical_reifier_rows_still_collapse
    orphans: 8
  - item: gmeow_gts::test_crate::reader::indirect_self_reaching_quoted_triple_reifier_degrades
    orphans: 22
  - item: gmeow_gts::test_crate::reader::iri
    orphans: 11
  - item: gmeow_gts::test_crate::reader::literal
    orphans: 11
  - item: gmeow_gts::test_crate::reader::multi_segment_union_preserves_literal_datatype_mapping
    orphans: 26
  - item: gmeow_gts::test_crate::reader::public_reader_reports_malformed_input_diagnostics_without_panicking
    orphans: 20
  - item: gmeow_gts::test_crate::reader::read_without_panic
    orphans: 3
  - item: gmeow_gts::test_crate::reader::reader_rejects_quoted_triple_in_reifier_position
    orphans: 30
  - item: gmeow_gts::test_crate::reader::reifies_frame_is_multi_valued
    orphans: 6
  - item: gmeow_gts::test_crate::reader::segment_with_triple_term
    orphans: 16
  - item: gmeow_gts::test_crate::reader::self_describing_triple_terms_sharing_a_reifier_stay_distinct
    orphans: 12
  - item: gmeow_gts::test_crate::reader::self_reaching_quoted_triple_reifier_degrades_and_keeps_its_row
    orphans: 24
  - item: gmeow_gts::test_crate::reader::triple
    orphans: 11
  - item: gmeow_gts::test_crate::reader::triple_term
    orphans: 11
  - item: gmeow_gts::test_crate::reader::tt_with_a_literal_predicate_is_a_position_constraint
    orphans: 13
  - item: gmeow_gts::test_crate::reader::two_bindings_on_one_reifier
    orphans: 22
  - item: gmeow_gts::test_crate::reader::union_keeps_distinct_triple_terms_that_share_a_reifier
    orphans: 20
  - item: gmeow_gts::test_crate::reader::unreified_triple_term_survives_a_round_trip
    orphans: 24
  - item: gmeow_gts::test_crate::reader::wire_constructed_self_reaching_reifier_stays_total
    orphans: 29
  - item: gmeow_gts::test_crate::signed::CAT
    orphans: 1
  - item: gmeow_gts::test_crate::signed::LABEL
    orphans: 1
  - item: gmeow_gts::test_crate::signed::fixtures_dir
    orphans: 3
  - item: gmeow_gts::test_crate::signed::iri
    orphans: 11
  - item: gmeow_gts::test_crate::signed::lit_lang
    orphans: 11
  - item: gmeow_gts::test_crate::signed::signature_algorithm_dispatch_vectors
    orphans: 68
  - item: gmeow_gts::test_crate::signed::signed_file_vector
    orphans: 36
  - item: gmeow_gts::test_crate::signed::unhex
    orphans: 6
  - item: gmeow_gts::test_crate::signed::writer_signs_with_openpgp_secret_key_default_fingerprint_kid
    orphans: 25
  - item: gmeow_gts::test_crate::signed::writer_signs_with_openpgp_secret_key_override_kid
    orphans: 24
  - item: gmeow_gts::test_crate::streaming::<impl StreamingSink for RecordingSink>
    orphans: 12
  - item: gmeow_gts::test_crate::streaming::RecordingSink
    orphans: 14
  - item: gmeow_gts::test_crate::streaming::RecordingSink::annotation
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::blob
    orphans: 4
  - item: gmeow_gts::test_crate::streaming::RecordingSink::diagnostic
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::opaque
    orphans: 7
  - item: gmeow_gts::test_crate::streaming::RecordingSink::quad
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::reifier
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::segment_head
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::signature
    orphans: 7
  - item: gmeow_gts::test_crate::streaming::RecordingSink::streamable_layout
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::suppression
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::RecordingSink::term
    orphans: 3
  - item: gmeow_gts::test_crate::streaming::assert_final_state_matches
    orphans: 72
  - item: gmeow_gts::test_crate::streaming::diagnostics_shape
    orphans: 6
  - item: gmeow_gts::test_crate::streaming::streamable_shape
    orphans: 6
  - item: gmeow_gts::test_crate::streaming::streaming_sink_events_match_segment_folds_for_corpus
    orphans: 64
  - item: gmeow_gts::test_crate::streaming::streaming_sink_expected_head_mismatch_matches_full_reader
    orphans: 18
  - item: gmeow_gts::test_crate::streaming::streaming_sink_final_state_matches_full_reader_for_corpus
    orphans: 13
  - item: gmeow_gts::test_crate::streaming::streaming_sink_from_reader_matches_slice_api_for_corpus
    orphans: 80
  - item: gmeow_gts::test_crate::streaming::streaming_sink_pre_segment_mode_matches_full_reader
    orphans: 6
  - item: gmeow_gts::test_crate::tar::<impl Write for ChunkRecordingWriter>
    orphans: 3
  - item: gmeow_gts::test_crate::tar::ChunkRecordingWriter
    orphans: 5
  - item: gmeow_gts::test_crate::tar::ChunkRecordingWriter::flush
    orphans: 3
  - item: gmeow_gts::test_crate::tar::ChunkRecordingWriter::write
    orphans: 5
  - item: gmeow_gts::test_crate::tar::STREAM_ASSERT_MAX_WRITE
    orphans: 1
  - item: gmeow_gts::test_crate::tar::TarEntry
    orphans: 7
  - item: gmeow_gts::test_crate::tar::allow_special_preserves_special_entry_metadata
    orphans: 25
  - item: gmeow_gts::test_crate::tar::cli_round_trips_tar_with_owner_pax_links_and_dedup
    orphans: 68
  - item: gmeow_gts::test_crate::tar::compression_variants_round_trip_through_the_cli
    orphans: 66
  - item: gmeow_gts::test_crate::tar::fifo_tar
    orphans: 12
  - item: gmeow_gts::test_crate::tar::fixture_tar
    orphans: 37
  - item: gmeow_gts::test_crate::tar::from_tar_and_to_tar_are_pipe_friendly
    orphans: 26
  - item: gmeow_gts::test_crate::tar::from_tar_refuses_links_special_files_and_traversal_by_default
    orphans: 27
  - item: gmeow_gts::test_crate::tar::from_tar_to_writer_streams_large_payload_writes
    orphans: 24
  - item: gmeow_gts::test_crate::tar::graph_from_nquads
    orphans: 4
  - item: gmeow_gts::test_crate::tar::gts
    orphans: 6
  - item: gmeow_gts::test_crate::tar::gts_with_stdin
    orphans: 16
  - item: gmeow_gts::test_crate::tar::header
    orphans: 12
  - item: gmeow_gts::test_crate::tar::inspect_tar
    orphans: 48
  - item: gmeow_gts::test_crate::tar::large_file_tar
    orphans: 13
  - item: gmeow_gts::test_crate::tar::make_tree
    orphans: 5
  - item: gmeow_gts::test_crate::tar::pack_to_writer_streams_large_file_payloads
    orphans: 24
  - item: gmeow_gts::test_crate::tar::tar_verb_creates_and_extracts_compressed_tar_archives
    orphans: 58
  - item: gmeow_gts::test_crate::tar::tar_verb_creates_lists_extracts_and_diffs_gts_archives
    orphans: 78
  - item: gmeow_gts::test_crate::tar::tar_verb_enforces_symlink_opt_in_on_extract
    orphans: 39
  - item: gmeow_gts::test_crate::tar::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::tar::to_tar_refuses_device_numbers_outside_tar_range
    orphans: 21
  - item: gmeow_gts::test_crate::tar::to_tar_refuses_link_entries_without_targets
    orphans: 19
  - item: gmeow_gts::test_crate::tar::traversal_tar
    orphans: 16
  - item: gmeow_gts::test_crate::tar_corpus::<impl Drop for TestTempDir>
    orphans: 2
  - item: gmeow_gts::test_crate::tar_corpus::<impl TestTempDir>
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::TestTempDir
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::TestTempDir::drop
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::TestTempDir::new
    orphans: 5
  - item: gmeow_gts::test_crate::tar_corpus::TestTempDir::path
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::assert_entry_matches
    orphans: 80
  - item: gmeow_gts::test_crate::tar_corpus::assert_expected_entries
    orphans: 20
  - item: gmeow_gts::test_crate::tar_corpus::assert_expected_error
    orphans: 9
  - item: gmeow_gts::test_crate::tar_corpus::assert_folded_graph
    orphans: 11
  - item: gmeow_gts::test_crate::tar_corpus::entry_kind
    orphans: 12
  - item: gmeow_gts::test_crate::tar_corpus::fixture_name
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::from_tar_options
    orphans: 15
  - item: gmeow_gts::test_crate::tar_corpus::load_expectation
    orphans: 6
  - item: gmeow_gts::test_crate::tar_corpus::optional_string
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::optional_u64
    orphans: 3
  - item: gmeow_gts::test_crate::tar_corpus::string_array
    orphans: 11
  - item: gmeow_gts::test_crate::tar_corpus::tar_corpus_matches_pinned_expectations
    orphans: 69
  - item: gmeow_gts::test_crate::tar_corpus::tar_expectation_names
    orphans: 10
  - item: gmeow_gts::test_crate::tar_corpus::tar_manifest_entries
    orphans: 19
  - item: gmeow_gts::test_crate::tar_corpus::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::tar_corpus::unpack_options
    orphans: 14
  - item: gmeow_gts::test_crate::tar_corpus::vectors
    orphans: 3
  - item: gmeow_gts::test_crate::ulid::deterministic_ulids_order_by_timestamp_then_counter
    orphans: 20
  - item: gmeow_gts::test_crate::ulid::ulid_parse_roundtrips_and_rejects_invalid_input
    orphans: 10
  - item: gmeow_gts::test_crate::ulid::ulid_renders_canonical_crockford_base32
    orphans: 9
  - item: gmeow_gts::test_crate::v2_attachments::blob_members_promote_shared_units_and_preserve_empty_blobs
    orphans: 11
  - item: gmeow_gts::test_crate::v2_attachments::invalid_attachment_inputs_leave_the_committed_prefix_unchanged
    orphans: 14
  - item: gmeow_gts::test_crate::v2_attachments::match_attachment_prefix
    orphans: 40
  - item: gmeow_gts::test_crate::v2_attachments::packs_cut_on_body_length_at_the_published_boundary
    orphans: 21
  - item: gmeow_gts::test_crate::v2_attachments::present
    orphans: 8
  - item: gmeow_gts::test_crate::v2_attachments::value_and_blob_regions_store_shared_units_separately
    orphans: 35
  - item: gmeow_gts::test_crate::v2_attachments::vector
    orphans: 6
  - item: gmeow_gts::test_crate::v2_attachments::xof
    orphans: 8
  - item: gmeow_gts::test_crate::v2_close_segment::add_index_for_one_chunk
    orphans: 39
  - item: gmeow_gts::test_crate::v2_close_segment::closing_a_buffered_indexed_segment_reproduces_the_corpus
    orphans: 15
  - item: gmeow_gts::test_crate::v2_close_segment::finishing_requires_a_closing_index_and_refuses_existing_parity
    orphans: 29
  - item: gmeow_gts::test_crate::v2_close_segment::ordinary_prefix
    orphans: 21
  - item: gmeow_gts::test_crate::v2_close_segment::reached_full_spans_follow_the_frame_that_crosses_them
    orphans: 28
  - item: gmeow_gts::test_crate::v2_close_segment::vector
    orphans: 15
  - item: gmeow_gts::test_crate::v2_frame::frame_identifiers_and_plain_payloads_match_the_published_chain
    orphans: 30
  - item: gmeow_gts::test_crate::v2_frame::later_registry_keeps_unknown_content_and_storage
    orphans: 29
  - item: gmeow_gts::test_crate::v2_frame::plain_writer_preserves_payload_length_boundaries
    orphans: 24
  - item: gmeow_gts::test_crate::v2_frame::plain_writer_requires_one_deterministic_content_record
    orphans: 16
  - item: gmeow_gts::test_crate::v2_frame::report_the_first_envelope_violation
    orphans: 30
  - item: gmeow_gts::test_crate::v2_frame::segment_writer_reproduces_commit_prefixes_in_the_corpus
    orphans: 48
  - item: gmeow_gts::test_crate::v2_frame::stored_checks_judge_compressed_bytes_independently
    orphans: 26
  - item: gmeow_gts::test_crate::v2_frame::truncated_frame_is_bounded_by_the_input
    orphans: 13
  - item: gmeow_gts::test_crate::v2_frame::unknown_content_judges_the_head_before_the_type
    orphans: 19
  - item: gmeow_gts::test_crate::v2_frame::vector
    orphans: 6
  - item: gmeow_gts::test_crate::v2_from_nquads::array
    orphans: 3
  - item: gmeow_gts::test_crate::v2_from_nquads::decode
    orphans: 6
  - item: gmeow_gts::test_crate::v2_from_nquads::deep_triple_objects_use_earlier_ids_without_recursive_parsing
    orphans: 30
  - item: gmeow_gts::test_crate::v2_from_nquads::imports_rdf_projection_as_v2_quads_with_typed_terms
    orphans: 56
  - item: gmeow_gts::test_crate::v2_from_nquads::keeps_dictionary_ids_across_frames_and_resets_front_coding_at_restarts
    orphans: 30
  - item: gmeow_gts::test_crate::v2_from_nquads::parse
    orphans: 3
  - item: gmeow_gts::test_crate::v2_from_nquads::preserves_carriage_return_separated_statements_and_escaped_terms
    orphans: 34
  - item: gmeow_gts::test_crate::v2_from_nquads::refuses_triple_subjects_outside_the_v2_position_rules
    orphans: 5
  - item: gmeow_gts::test_crate::v2_from_nquads::uint
    orphans: 3
  - item: gmeow_gts::test_crate::v2_header::array
    orphans: 3
  - item: gmeow_gts::test_crate::v2_header::header_outcomes_and_identifiers_match_p1
    orphans: 87
  - item: gmeow_gts::test_crate::v2_header::reason
    orphans: 10
  - item: gmeow_gts::test_crate::v2_header::truncated_prefixes_produce_header_errors
    orphans: 12
  - item: gmeow_gts::test_crate::v2_header::uint
    orphans: 7
  - item: gmeow_gts::test_crate::v2_header::unhex
    orphans: 12
  - item: gmeow_gts::test_crate::v2_header::writer_refuses_a_registry_it_does_not_implement
    orphans: 11
  - item: gmeow_gts::test_crate::v2_index_writer::cannot_index_a_row_payload_with_inconsistent_counts
    orphans: 16
  - item: gmeow_gts::test_crate::v2_index_writer::count
    orphans: 3
  - item: gmeow_gts::test_crate::v2_index_writer::counts_written_predicates_when_the_fold_has_no_digest
    orphans: 23
  - item: gmeow_gts::test_crate::v2_index_writer::fold_from_published_index
    orphans: 16
  - item: gmeow_gts::test_crate::v2_index_writer::indexes_match_published_rows_and_pack_directories
    orphans: 37
  - item: gmeow_gts::test_crate::v2_index_writer::transparent_frames_are_absent_from_index_claims
    orphans: 29
  - item: gmeow_gts::test_crate::v2_index_writer::vector
    orphans: 6
  - item: gmeow_gts::test_crate::v2_index_writer::writes_the_complete_canonical_empty_blob_vector
    orphans: 34
  - item: gmeow_gts::test_crate::v2_rdf_projection::ROUND_TRIP
    orphans: 1
  - item: gmeow_gts::test_crate::v2_rdf_projection::annotation_duplicates_use_storage_identity_inside_triples
    orphans: 51
  - item: gmeow_gts::test_crate::v2_rdf_projection::append_referenced_literals
    orphans: 23
  - item: gmeow_gts::test_crate::v2_rdf_projection::fold_cli_exports_v2_and_continues_reading_v1
    orphans: 40
  - item: gmeow_gts::test_crate::v2_rdf_projection::pack_value_can_use_later_members_and_share_reference_text
    orphans: 43
  - item: gmeow_gts::test_crate::v2_rdf_projection::project
    orphans: 5
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_checks_inline_types_before_column_counts
    orphans: 14
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_deduplicates_equivalent_lines_and_scopes_blanks
    orphans: 25
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_generates_inline_typed_lexical_forms
    orphans: 34
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_keeps_nested_object_and_empty_graph
    orphans: 13
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_preserves_text_and_refuses_unreadable_content
    orphans: 32
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_refuses_bad_storage_check_with_verified_bytes
    orphans: 27
  - item: gmeow_gts::test_crate::v2_rdf_projection::projection_uses_first_verified_earlier_value
    orphans: 57
  - item: gmeow_gts::test_crate::v2_rdf_projection::recipe_charges_can_refuse_later_literal_values
    orphans: 34
  - item: gmeow_gts::test_crate::v2_rdf_projection::reference_columns_and_language_table_precede_value_lookup
    orphans: 19
  - item: gmeow_gts::test_crate::v2_wire_reader::Outcome
    orphans: 2
  - item: gmeow_gts::test_crate::v2_wire_reader::SignatureCase
    orphans: 8
  - item: gmeow_gts::test_crate::v2_wire_reader::a_commit_needs_a_link_even_when_both_claims_hold
    orphans: 34
  - item: gmeow_gts::test_crate::v2_wire_reader::a_composite_with_only_one_component_key_held_is_refused
    orphans: 42
  - item: gmeow_gts::test_crate::v2_wire_reader::a_frame_with_a_malformed_envelope_but_signatures_in_grammar_has_results
    orphans: 11
  - item: gmeow_gts::test_crate::v2_wire_reader::a_frame_without_an_effective_identifier_creates_no_commit_leaf
    orphans: 29
  - item: gmeow_gts::test_crate::v2_wire_reader::a_key_or_value_of_the_wrong_length_is_value_length_before_any_algorithm_check
    orphans: 18
  - item: gmeow_gts::test_crate::v2_wire_reader::a_malformed_payload_head_does_not_hide_the_stored_checksum
    orphans: 36
  - item: gmeow_gts::test_crate::v2_wire_reader::a_read_without_keys_reports_key_not_held_within_the_stored_bytes_capacity
    orphans: 26
  - item: gmeow_gts::test_crate::v2_wire_reader::a_removed_witness_does_not_heal_the_preceding_frame
    orphans: 32
  - item: gmeow_gts::test_crate::v2_wire_reader::a_scanned_opener_can_establish_the_generation_after_foreign_bytes
    orphans: 17
  - item: gmeow_gts::test_crate::v2_wire_reader::a_total_decoded_refusal_latches_even_for_a_later_empty_payload
    orphans: 22
  - item: gmeow_gts::test_crate::v2_wire_reader::a_walked_resync_decides_each_nested_field_at_its_start
    orphans: 37
  - item: gmeow_gts::test_crate::v2_wire_reader::a_walked_resync_opens_a_segment_using_the_computed_identifier
    orphans: 17
  - item: gmeow_gts::test_crate::v2_wire_reader::a_walked_resync_reports_the_byte_cap_where_it_is_reached
    orphans: 18
  - item: gmeow_gts::test_crate::v2_wire_reader::algorithm_and_held_key_steps_come_before_the_lengths
    orphans: 18
  - item: gmeow_gts::test_crate::v2_wire_reader::an_index_after_an_unknown_critical_frame_still_checks_the_wire_prefix
    orphans: 24
  - item: gmeow_gts::test_crate::v2_wire_reader::an_index_with_bad_payload_grammar_is_not_compared
    orphans: 30
  - item: gmeow_gts::test_crate::v2_wire_reader::assert_signature_vectors
    orphans: 17
  - item: gmeow_gts::test_crate::v2_wire_reader::bare_resync
    orphans: 16
  - item: gmeow_gts::test_crate::v2_wire_reader::bytes
    orphans: 3
  - item: gmeow_gts::test_crate::v2_wire_reader::charges_scan_candidates_after_place_and_before_tail_grammar
    orphans: 22
  - item: gmeow_gts::test_crate::v2_wire_reader::checks_payload_independently_of_storage_and_link_state
    orphans: 31
  - item: gmeow_gts::test_crate::v2_wire_reader::commit_prefix_checks_count_and_root_independently_of_transparent_frames
    orphans: 46
  - item: gmeow_gts::test_crate::v2_wire_reader::composite_post_quantum_half_is_ml_dsa_verify_with_the_label_context
    orphans: 15
  - item: gmeow_gts::test_crate::v2_wire_reader::composite_reports_the_classical_half_over_the_representative_before_the_post_quantum_half
    orphans: 17
  - item: gmeow_gts::test_crate::v2_wire_reader::covering_a_frame_without_a_type_rejects_the_type_runs
    orphans: 23
  - item: gmeow_gts::test_crate::v2_wire_reader::ed25519_equation_decodes_the_key_strictly_and_is_cofactorless
    orphans: 18
  - item: gmeow_gts::test_crate::v2_wire_reader::ed25519_strict_reports_the_first_failing_check_in_policy_order
    orphans: 23
  - item: gmeow_gts::test_crate::v2_wire_reader::frames_without_a_subject_or_in_grammar_carry_no_signature_result
    orphans: 34
  - item: gmeow_gts::test_crate::v2_wire_reader::held_keys_are_a_set_whatever_order_or_repetition_the_caller_gives
    orphans: 21
  - item: gmeow_gts::test_crate::v2_wire_reader::honest_signatures_of_both_algorithms_are_valid
    orphans: 13
  - item: gmeow_gts::test_crate::v2_wire_reader::index_wire_claims_are_checked_independently_at_each_index
    orphans: 31
  - item: gmeow_gts::test_crate::v2_wire_reader::items
    orphans: 3
  - item: gmeow_gts::test_crate::v2_wire_reader::items_held_before_the_scan_decision_count_toward_the_limit
    orphans: 19
  - item: gmeow_gts::test_crate::v2_wire_reader::locator_claim_names_the_last_well_typed_index_frame_type
    orphans: 22
  - item: gmeow_gts::test_crate::v2_wire_reader::malformed_mark_shape_precedes_a_later_mark_limit
    orphans: 13
  - item: gmeow_gts::test_crate::v2_wire_reader::malformed_or_unread_commit_content_checks_no_claims
    orphans: 35
  - item: gmeow_gts::test_crate::v2_wire_reader::misplaced_closing_items_report_placement_before_grammar_or_claims
    orphans: 31
  - item: gmeow_gts::test_crate::v2_wire_reader::offsets_check_every_retained_item_and_report_count_or_length_disagreement
    orphans: 20
  - item: gmeow_gts::test_crate::v2_wire_reader::read_outcomes
    orphans: 28
  - item: gmeow_gts::test_crate::v2_wire_reader::recorded_reason
    orphans: 15
  - item: gmeow_gts::test_crate::v2_wire_reader::recorded_status
    orphans: 8
  - item: gmeow_gts::test_crate::v2_wire_reader::repairs_a_link_only_with_the_direct_admitted_witness
    orphans: 36
  - item: gmeow_gts::test_crate::v2_wire_reader::resync_distinguishes_a_damaged_region_from_a_torn_prefix
    orphans: 45
  - item: gmeow_gts::test_crate::v2_wire_reader::signature_cases
    orphans: 66
  - item: gmeow_gts::test_crate::v2_wire_reader::the_complete_item_before_lost_framing_keeps_its_wire_extent
    orphans: 24
  - item: gmeow_gts::test_crate::v2_wire_reader::the_subject_binds_the_opener_and_the_effective_identifier_of_the_frame
    orphans: 16
  - item: gmeow_gts::test_crate::v2_wire_reader::uint
    orphans: 3
  - item: gmeow_gts::test_crate::v2_wire_reader::unhex
    orphans: 12
  - item: gmeow_gts::test_crate::v2_wire_reader::violations_before_a_resync_byte_limit_keep_their_first_offset
    orphans: 22
  - item: gmeow_gts::test_crate::verify::b64
    orphans: 23
  - item: gmeow_gts::test_crate::verify::extract_transport_key_round_trips_from_graph_meta
    orphans: 12
  - item: gmeow_gts::test_crate::verify::format_fingerprint_groups_openpgp_hex
    orphans: 8
  - item: gmeow_gts::test_crate::verify::iri
    orphans: 11
  - item: gmeow_gts::test_crate::verify::lit
    orphans: 11
  - item: gmeow_gts::test_crate::verify::public_key_armor
    orphans: 21
  - item: gmeow_gts::test_crate::verify::signed_file
    orphans: 18
  - item: gmeow_gts::test_crate::verify::trusted_policy_counts_trusted_valid_signatures
    orphans: 12
  - item: gmeow_gts::test_crate::verify::verify_signed_file_with_embedded_key
    orphans: 19
  - item: gmeow_gts::test_crate::verify::verify_tampered_file_fails
    orphans: 12
  - item: gmeow_gts::test_crate::verify::verify_unsigned_file_fails_when_required
    orphans: 7
  - item: gmeow_gts::test_crate::verify::verify_unsigned_file_is_ok_when_signatures_are_not_required
    orphans: 8
  - item: gmeow_gts::test_crate::verify::verify_with_trusted_key_uses_openpgp_fingerprint_as_kid
    orphans: 13
  - item: gmeow_gts::test_crate::verify::verify_with_wrong_out_of_band_key_reports_unverified
    orphans: 23
  - item: gmeow_gts::test_crate::workplan::capability_documents_and_changes_match_the_sources
    orphans: 13
  - item: gmeow_gts::test_crate::writer_transforms::bool_map
    orphans: 3
  - item: gmeow_gts::test_crate::writer_transforms::encrypted_writer_frame_is_opaque_without_key_and_clear_with_key
    orphans: 54
  - item: gmeow_gts::test_crate::writer_transforms::encrypted_writer_frame_with_wrong_key_degrades_to_missing_key
    orphans: 31
  - item: gmeow_gts::test_crate::writer_transforms::frame_payload_bytes
    orphans: 15
  - item: gmeow_gts::test_crate::writer_transforms::meta_bool
    orphans: 6
  - item: gmeow_gts::test_crate::writer_transforms::meta_text
    orphans: 6
  - item: gmeow_gts::test_crate::writer_transforms::text_map
    orphans: 3
  - item: gmeow_gts::test_crate::writer_transforms::writer_applies_zstd_level_per_frame
    orphans: 45
  - item: gmeow_gts::test_crate::writer_transforms::writer_authors_compressed_frames_that_fold_cleanly
    orphans: 24
  - item: gmeow_gts::test_crate::writer_transforms::writer_options_support_header_metadata_magic_toggle_and_custom_catalog
    orphans: 56
  - item: gmeow_gts::test_crate::writer_transforms::writer_rejects_zstd_level_without_zstd_transform
    orphans: 18
  - item: gmeow_gts::test_crate::yamlld::annotated_fixture
    orphans: 63
  - item: gmeow_gts::test_crate::yamlld::cli_yaml_ld_inverts_fold
    orphans: 48
  - item: gmeow_gts::test_crate::yamlld::compact_context_authoring_shape_imports_annotations_and_scalars
    orphans: 26
  - item: gmeow_gts::test_crate::yamlld::graph_lines
    orphans: 3
  - item: gmeow_gts::test_crate::yamlld::gts
    orphans: 6
  - item: gmeow_gts::test_crate::yamlld::json_ld_uses_the_same_intermediate_and_round_trips
    orphans: 10
  - item: gmeow_gts::test_crate::yamlld::scoped_contexts_expand_node_value_and_annotation_terms
    orphans: 27
  - item: gmeow_gts::test_crate::yamlld::sorted_lines
    orphans: 9
  - item: gmeow_gts::test_crate::yamlld::tmpdir
    orphans: 6
  - item: gmeow_gts::test_crate::yamlld::vectors
    orphans: 3
  - item: gmeow_gts::test_crate::yamlld::yaml_ld_preserves_directional_language_literals
    orphans: 18
  - item: gmeow_gts::test_crate::yamlld::yaml_ld_preserves_reifier_annotations_as_folded_state
    orphans: 14
  - item: gmeow_gts::test_crate::yamlld::yaml_ld_rejects_language_literals_with_type_or_non_string_values
    orphans: 21
  - item: gmeow_gts::test_crate::yamlld::yaml_ld_roundtrips_the_graph_vector_corpus
    orphans: 25
  - item: gmeow_gts::trig::RDF_NS
    orphans: 1
  - item: gmeow_gts::trig::RDF_REIFIES
    orphans: 1
  - item: gmeow_gts::trig::close_graph
    orphans: 5
  - item: gmeow_gts::trig::push_statement
    orphans: 20
  - item: gmeow_gts::trig::render_trig_term
    orphans: 34
  - item: gmeow_gts::trig::to_trig
    orphans: 53
  - item: gmeow_gts::ulid::<impl DeterministicUlidGenerator>
    orphans: 4
  - item: gmeow_gts::ulid::<impl FromStr for Ulid>
    orphans: 3
  - item: gmeow_gts::ulid::<impl Ulid>
    orphans: 7
  - item: gmeow_gts::ulid::<impl UlidError>
    orphans: 3
  - item: gmeow_gts::ulid::<impl fmt::Debug for Ulid>
    orphans: 2
  - item: gmeow_gts::ulid::<impl fmt::Display for Ulid>
    orphans: 2
  - item: gmeow_gts::ulid::<impl fmt::Display for UlidError>
    orphans: 2
  - item: gmeow_gts::ulid::<impl std::error::Error for UlidError>
    orphans: 1
  - item: gmeow_gts::ulid::CROCKFORD
    orphans: 1
  - item: gmeow_gts::ulid::DeterministicUlidGenerator
    orphans: 6
  - item: gmeow_gts::ulid::DeterministicUlidGenerator::new
    orphans: 8
  - item: gmeow_gts::ulid::DeterministicUlidGenerator::next_ulid
    orphans: 14
  - item: gmeow_gts::ulid::DeterministicUlidGenerator::with_counter
    orphans: 13
  - item: gmeow_gts::ulid::MAX_RANDOMNESS
    orphans: 1
  - item: gmeow_gts::ulid::RANDOMNESS_BYTES
    orphans: 1
  - item: gmeow_gts::ulid::TIMESTAMP_BYTES
    orphans: 1
  - item: gmeow_gts::ulid::ULID_LEN
    orphans: 1
  - item: gmeow_gts::ulid::Ulid
    orphans: 3
  - item: gmeow_gts::ulid::Ulid::Err
    orphans: 1
  - item: gmeow_gts::ulid::Ulid::MAX_TIMESTAMP_MS
    orphans: 2
  - item: gmeow_gts::ulid::Ulid::as_bytes
    orphans: 5
  - item: gmeow_gts::ulid::Ulid::fmt
    orphans: 18
  - item: gmeow_gts::ulid::Ulid::from_counter
    orphans: 17
  - item: gmeow_gts::ulid::Ulid::from_parts
    orphans: 21
  - item: gmeow_gts::ulid::Ulid::from_str
    orphans: 25
  - item: gmeow_gts::ulid::Ulid::randomness
    orphans: 7
  - item: gmeow_gts::ulid::Ulid::timestamp_ms
    orphans: 7
  - item: gmeow_gts::ulid::UlidError
    orphans: 5
  - item: gmeow_gts::ulid::UlidError::detail
    orphans: 5
  - item: gmeow_gts::ulid::UlidError::fmt
    orphans: 3
  - item: gmeow_gts::ulid::UlidError::new
    orphans: 5
  - item: gmeow_gts::ulid::decode_crockford
    orphans: 37
  - item: gmeow_gts::ulid::deterministic_label
    orphans: 10
  - item: gmeow_gts::v2::<impl Error for HeaderWriteError>
    orphans: 1
  - item: gmeow_gts::v2::<impl fmt::Display for HeaderWriteError>
    orphans: 2
  - item: gmeow_gts::v2::HeaderWriteError
    orphans: 14
  - item: gmeow_gts::v2::HeaderWriteError::fmt
    orphans: 13
  - item: gmeow_gts::v2::LAST_PROFILE
    orphans: 1
  - item: gmeow_gts::v2::encode_header
    orphans: 47
  - item: gmeow_gts::v2::frame::decode_frame
    orphans: 18
  - item: gmeow_gts::v2::frame::encode_plain_frame
    orphans: 19
  - item: gmeow_gts::v2::frame::encode_plain_frame_with_identifier
    orphans: 61
  - item: gmeow_gts::v2::import::<impl Error for ImportError>
    orphans: 2
  - item: gmeow_gts::v2::import::<impl From<SegmentWriteError> for ImportError>
    orphans: 2
  - item: gmeow_gts::v2::import::<impl ImportWriter>
    orphans: 3
  - item: gmeow_gts::v2::import::<impl StoredTerm<'_>>
    orphans: 2
  - item: gmeow_gts::v2::import::<impl TermBlocks<'a>>
    orphans: 4
  - item: gmeow_gts::v2::import::<impl TermPlan<'a>>
    orphans: 2
  - item: gmeow_gts::v2::import::<impl fmt::Display for ImportError>
    orphans: 2
  - item: gmeow_gts::v2::import::ImportError
    orphans: 29
  - item: gmeow_gts::v2::import::ImportError::fmt
    orphans: 29
  - item: gmeow_gts::v2::import::ImportError::from
    orphans: 3
  - item: gmeow_gts::v2::import::ImportError::source
    orphans: 10
  - item: gmeow_gts::v2::import::ImportWriter
    orphans: 7
  - item: gmeow_gts::v2::import::ImportWriter::append
    orphans: 43
  - item: gmeow_gts::v2::import::ImportWriter::payload_limit
    orphans: 5
  - item: gmeow_gts::v2::import::LITERAL_REF
    orphans: 1
  - item: gmeow_gts::v2::import::QUADS
    orphans: 1
  - item: gmeow_gts::v2::import::RESTART_GROUP
    orphans: 1
  - item: gmeow_gts::v2::import::ROWS_PER_FRAME
    orphans: 1
  - item: gmeow_gts::v2::import::ROW_ORDER_UNORDERED
    orphans: 1
  - item: gmeow_gts::v2::import::StoredTerm
    orphans: 4
  - item: gmeow_gts::v2::import::StoredTerm::code
    orphans: 7
  - item: gmeow_gts::v2::import::TERMS
    orphans: 1
  - item: gmeow_gts::v2::import::TERMS_PER_FRAME
    orphans: 1
  - item: gmeow_gts::v2::import::TERM_ORDER_ARRIVAL
    orphans: 1
  - item: gmeow_gts::v2::import::TermBlocks
    orphans: 6
  - item: gmeow_gts::v2::import::TermBlocks::block
    orphans: 36
  - item: gmeow_gts::v2::import::TermBlocks::language_id
    orphans: 7
  - item: gmeow_gts::v2::import::TermBlocks::text_block
    orphans: 78
  - item: gmeow_gts::v2::import::TermPlan
    orphans: 4
  - item: gmeow_gts::v2::import::TermPlan::new
    orphans: 43
  - item: gmeow_gts::v2::import::append_quads
    orphans: 20
  - item: gmeow_gts::v2::import::append_terms
    orphans: 28
  - item: gmeow_gts::v2::import::cbor_head
    orphans: 9
  - item: gmeow_gts::v2::import::delta_column
    orphans: 17
  - item: gmeow_gts::v2::import::encode_terms
    orphans: 26
  - item: gmeow_gts::v2::import::from_nquads
    orphans: 17
  - item: gmeow_gts::v2::import::front_coded_prefix
    orphans: 11
  - item: gmeow_gts::v2::import::kind_code
    orphans: 8
  - item: gmeow_gts::v2::import::record
    orphans: 3
  - item: gmeow_gts::v2::import::singleton_literal_size
    orphans: 13
  - item: gmeow_gts::v2::import::term_body_size
    orphans: 30
  - item: gmeow_gts::v2::import::term_text
    orphans: 5
  - item: gmeow_gts::v2::import::tests::imported_literal_forms_share_a_value_and_charge_pack_frames
    orphans: 40
  - item: gmeow_gts::v2::import::tests::singleton_storage_choice_matches_encoded_payload_at_the_limit
    orphans: 21
  - item: gmeow_gts::v2::import::uint
    orphans: 3
  - item: gmeow_gts::v2::import::write_dataset
    orphans: 30
  - item: gmeow_gts::v2::option
    orphans: 6
  - item: gmeow_gts::v2::writer::<impl Error for SegmentWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::<impl SegmentWriter>
    orphans: 8
  - item: gmeow_gts::v2::writer::<impl fmt::Display for SegmentWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::SegmentWriteError
    orphans: 10
  - item: gmeow_gts::v2::writer::SegmentWriteError::fmt
    orphans: 7
  - item: gmeow_gts::v2::writer::SegmentWriteError::source
    orphans: 6
  - item: gmeow_gts::v2::writer::SegmentWriter
    orphans: 12
  - item: gmeow_gts::v2::writer::SegmentWriter::append
    orphans: 20
  - item: gmeow_gts::v2::writer::SegmentWriter::append_attachment_regions
    orphans: 39
  - item: gmeow_gts::v2::writer::SegmentWriter::append_frame
    orphans: 18
  - item: gmeow_gts::v2::writer::SegmentWriter::append_index
    orphans: 41
  - item: gmeow_gts::v2::writer::SegmentWriter::as_bytes
    orphans: 5
  - item: gmeow_gts::v2::writer::SegmentWriter::commit
    orphans: 12
  - item: gmeow_gts::v2::writer::SegmentWriter::finish_indexed
    orphans: 59
  - item: gmeow_gts::v2::writer::SegmentWriter::head
    orphans: 5
  - item: gmeow_gts::v2::writer::SegmentWriter::into_bytes
    orphans: 5
  - item: gmeow_gts::v2::writer::SegmentWriter::new
    orphans: 16
  - item: gmeow_gts::v2::writer::attachments::<impl Error for AttachmentWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::attachments::<impl From<SegmentWriteError> for AttachmentWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::attachments::<impl Pack>
    orphans: 4
  - item: gmeow_gts::v2::writer::attachments::<impl SegmentWriter>
    orphans: 2
  - item: gmeow_gts::v2::writer::attachments::<impl fmt::Display for AttachmentWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::attachments::AttachmentWriteError
    orphans: 19
  - item: gmeow_gts::v2::writer::attachments::AttachmentWriteError::fmt
    orphans: 16
  - item: gmeow_gts::v2::writer::attachments::AttachmentWriteError::from
    orphans: 3
  - item: gmeow_gts::v2::writer::attachments::AttachmentWriteError::source
    orphans: 7
  - item: gmeow_gts::v2::writer::attachments::Blob
    orphans: 15
  - item: gmeow_gts::v2::writer::attachments::EXTERNAL
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::MemberKind
    orphans: 7
  - item: gmeow_gts::v2::writer::attachments::PACK_BYTES
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::Pack
    orphans: 6
  - item: gmeow_gts::v2::writer::attachments::Pack::flush
    orphans: 17
  - item: gmeow_gts::v2::writer::attachments::Pack::push
    orphans: 23
  - item: gmeow_gts::v2::writer::attachments::Pack::push_unit
    orphans: 15
  - item: gmeow_gts::v2::writer::attachments::PlannedBlob
    orphans: 5
  - item: gmeow_gts::v2::writer::attachments::Unit
    orphans: 5
  - item: gmeow_gts::v2::writer::attachments::chunker::AVERAGE
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::GEAR
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::LOOSE_SHIFT
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::MAX
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::MIN
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::STRICT_SHIFT
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::WINDOW
    orphans: 1
  - item: gmeow_gts::v2::writer::attachments::chunker::chunks
    orphans: 11
  - item: gmeow_gts::v2::writer::attachments::chunker::cut
    orphans: 23
  - item: gmeow_gts::v2::writer::attachments::chunker::gear
    orphans: 10
  - item: gmeow_gts::v2::writer::attachments::digest_value
    orphans: 3
  - item: gmeow_gts::v2::writer::attachments::entries
    orphans: 7
  - item: gmeow_gts::v2::writer::attachments::media_value
    orphans: 6
  - item: gmeow_gts::v2::writer::attachments::plan_blobs
    orphans: 50
  - item: gmeow_gts::v2::writer::attachments::uint
    orphans: 3
  - item: gmeow_gts::v2::writer::attachments::units
    orphans: 8
  - item: gmeow_gts::v2::writer::attachments::write_blobs
    orphans: 47
  - item: gmeow_gts::v2::writer::attachments::write_values
    orphans: 22
  - item: gmeow_gts::v2::writer::finish::<impl ClosingItems>
    orphans: 4
  - item: gmeow_gts::v2::writer::finish::<impl Error for SegmentFinishError>
    orphans: 2
  - item: gmeow_gts::v2::writer::finish::<impl From<FrameError> for SegmentFinishError>
    orphans: 2
  - item: gmeow_gts::v2::writer::finish::<impl SegmentWriter>
    orphans: 2
  - item: gmeow_gts::v2::writer::finish::<impl fmt::Display for SegmentFinishError>
    orphans: 2
  - item: gmeow_gts::v2::writer::finish::ClosingItems
    orphans: 4
  - item: gmeow_gts::v2::writer::finish::ClosingItems::push
    orphans: 4
  - item: gmeow_gts::v2::writer::finish::ClosingItems::push_span
    orphans: 10
  - item: gmeow_gts::v2::writer::finish::ClosingItems::with_accelerators
    orphans: 10
  - item: gmeow_gts::v2::writer::finish::INDEX
    orphans: 1
  - item: gmeow_gts::v2::writer::finish::SegmentFinishError
    orphans: 13
  - item: gmeow_gts::v2::writer::finish::SegmentFinishError::fmt
    orphans: 14
  - item: gmeow_gts::v2::writer::finish::SegmentFinishError::from
    orphans: 3
  - item: gmeow_gts::v2::writer::finish::SegmentFinishError::source
    orphans: 6
  - item: gmeow_gts::v2::writer::index::<impl Error for IndexWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::index::<impl IndexPrefix>
    orphans: 4
  - item: gmeow_gts::v2::writer::index::<impl SegmentWriter>
    orphans: 2
  - item: gmeow_gts::v2::writer::index::<impl fmt::Display for IndexWriteError>
    orphans: 2
  - item: gmeow_gts::v2::writer::index::GraphRange
    orphans: 6
  - item: gmeow_gts::v2::writer::index::INDEX
    orphans: 1
  - item: gmeow_gts::v2::writer::index::IndexFold
    orphans: 12
  - item: gmeow_gts::v2::writer::index::IndexPrefix
    orphans: 9
  - item: gmeow_gts::v2::writer::index::IndexPrefix::add
    orphans: 18
  - item: gmeow_gts::v2::writer::index::IndexPrefix::add_graph
    orphans: 29
  - item: gmeow_gts::v2::writer::index::IndexPrefix::add_graph_runs
    orphans: 12
  - item: gmeow_gts::v2::writer::index::IndexPrefix::add_pack
    orphans: 12
  - item: gmeow_gts::v2::writer::index::IndexPrefix::add_predicate_runs
    orphans: 11
  - item: gmeow_gts::v2::writer::index::IndexPrefix::add_rows
    orphans: 38
  - item: gmeow_gts::v2::writer::index::IndexPrefix::payload
    orphans: 44
  - item: gmeow_gts::v2::writer::index::IndexWriteError
    orphans: 22
  - item: gmeow_gts::v2::writer::index::IndexWriteError::fmt
    orphans: 19
  - item: gmeow_gts::v2::writer::index::IndexWriteError::source
    orphans: 8
  - item: gmeow_gts::v2::writer::index::checked_sum
    orphans: 4
  - item: gmeow_gts::v2::writer::index::digest
    orphans: 3
  - item: gmeow_gts::v2::writer::index::integers
    orphans: 3
  - item: gmeow_gts::v2::writer::index::record
    orphans: 3
  - item: gmeow_gts::v2::writer::index::require_end
    orphans: 7
  - item: gmeow_gts::v2::writer::index::rows::<impl IndexPrefix>
    orphans: 5
  - item: gmeow_gts::v2::writer::index::rows::EMPTY_GRAPHS
    orphans: 1
  - item: gmeow_gts::v2::writer::index::rows::QUADS
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::CAUCHY_PRODUCTS
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::DATA_COLUMNS
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::FIELD_ORDER
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::MAX_SHARD
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::MIN_SHARD
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::PARITY_ROWS
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::SPAN_BYTES
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::ZERO_SHARD
    orphans: 1
  - item: gmeow_gts::v2::writer::parity::cauchy_products
    orphans: 28
  - item: gmeow_gts::v2::writer::parity::encode_span
    orphans: 38
  - item: gmeow_gts::v2::writer::parity::parity_shards
    orphans: 19
  - item: gmeow_gts::v2::writer::parity::shard_digest
    orphans: 6
  - item: gmeow_gts::v2::writer::parity::shard_size
    orphans: 6
  - item: gmeow_gts::verify::<impl Default for VerifyOptions>
    orphans: 2
  - item: gmeow_gts::verify::<impl VerifyOptions>
    orphans: 5
  - item: gmeow_gts::verify::EmbeddedTransportKey
    orphans: 8
  - item: gmeow_gts::verify::ResolvedKey
    orphans: 3
  - item: gmeow_gts::verify::VerificationResult
    orphans: 36
  - item: gmeow_gts::verify::VerifyOptions
    orphans: 16
  - item: gmeow_gts::verify::VerifyOptions::default
    orphans: 7
  - item: gmeow_gts::verify::VerifyOptions::require_signatures
    orphans: 6
  - item: gmeow_gts::verify::VerifyOptions::strict
    orphans: 5
  - item: gmeow_gts::verify::VerifyOptions::trust_policy
    orphans: 6
  - item: gmeow_gts::verify::VerifyOptions::with_armored_key
    orphans: 6
  - item: gmeow_gts::verify::extract_transport_key
    orphans: 27
  - item: gmeow_gts::verify::format_fingerprint
    orphans: 19
  - item: gmeow_gts::verify::provider_from_armor
    orphans: 9
  - item: gmeow_gts::verify::resolve_verification_key
    orphans: 64
  - item: gmeow_gts::verify::verify_file
    orphans: 5
  - item: gmeow_gts::verify::verify_file_with_options
    orphans: 70
  - item: gmeow_gts::wire::<impl std::io::Read for Counting<'_>>
    orphans: 2
  - item: gmeow_gts::wire::Counting
    orphans: 4
  - item: gmeow_gts::wire::Counting::read
    orphans: 6
  - item: gmeow_gts::wire::MAGIC
    orphans: 2
  - item: gmeow_gts::wire::SELF_DESCRIBE_TAG
    orphans: 2
  - item: gmeow_gts::wire::VERSION
    orphans: 2
  - item: gmeow_gts::wire::blake3_256
    orphans: 5
  - item: gmeow_gts::wire::canonical
    orphans: 5
  - item: gmeow_gts::wire::content_id
    orphans: 5
  - item: gmeow_gts::wire::deterministic
    orphans: 20
  - item: gmeow_gts::wire::digest_str
    orphans: 5
  - item: gmeow_gts::wire::hash_excluding
    orphans: 8
  - item: gmeow_gts::wire::header_id
    orphans: 5
  - item: gmeow_gts::wire::hex
    orphans: 11
  - item: gmeow_gts::wire::iter_items
    orphans: 28
  - item: gmeow_gts::wire::map_get
    orphans: 8
  - item: gmeow_gts::wire::unwrap_header
    orphans: 21
  - item: gmeow_gts::writer::<impl Default for SnapshotOptions>
    orphans: 2
  - item: gmeow_gts::writer::<impl Default for WriterOptions>
    orphans: 2
  - item: gmeow_gts::writer::<impl From<CodecError> for WriterError>
    orphans: 2
  - item: gmeow_gts::writer::<impl Graph>
    orphans: 2
  - item: gmeow_gts::writer::<impl Writer>
    orphans: 23
  - item: gmeow_gts::writer::<impl fmt::Display for WriterError>
    orphans: 2
  - item: gmeow_gts::writer::<impl std::error::Error for WriterError>
    orphans: 1
  - item: gmeow_gts::writer::BlobRow
    orphans: 10
  - item: gmeow_gts::writer::DEFAULT_RSYNCABLE_THRESHOLD
    orphans: 2
  - item: gmeow_gts::writer::Encrypt0Options
    orphans: 8
  - item: gmeow_gts::writer::FrameOptions
    orphans: 24
  - item: gmeow_gts::writer::SnapshotOptions
    orphans: 18
  - item: gmeow_gts::writer::SnapshotOptions::default
    orphans: 11
  - item: gmeow_gts::writer::SnapshotSigner
    orphans: 10
  - item: gmeow_gts::writer::TermRemap
    orphans: 8
  - item: gmeow_gts::writer::Writer
    orphans: 15
  - item: gmeow_gts::writer::Writer::add_annot
    orphans: 14
  - item: gmeow_gts::writer::Writer::add_blob
    orphans: 12
  - item: gmeow_gts::writer::Writer::add_frame
    orphans: 30
  - item: gmeow_gts::writer::Writer::add_frame_with_options
    orphans: 115
  - item: gmeow_gts::writer::Writer::add_index
    orphans: 4
  - item: gmeow_gts::writer::Writer::add_index_impl
    orphans: 36
  - item: gmeow_gts::writer::Writer::add_index_with_mmr
    orphans: 7
  - item: gmeow_gts::writer::Writer::add_meta
    orphans: 4
  - item: gmeow_gts::writer::Writer::add_quads
    orphans: 14
  - item: gmeow_gts::writer::Writer::add_reifies
    orphans: 14
  - item: gmeow_gts::writer::Writer::add_suppress
    orphans: 17
  - item: gmeow_gts::writer::Writer::add_terms
    orphans: 5
  - item: gmeow_gts::writer::Writer::chain_ids
    orphans: 11
  - item: gmeow_gts::writer::Writer::deterministic
    orphans: 105
  - item: gmeow_gts::writer::Writer::from_store
    orphans: 15
  - item: gmeow_gts::writer::Writer::head
    orphans: 5
  - item: gmeow_gts::writer::Writer::new
    orphans: 5
  - item: gmeow_gts::writer::Writer::sign_with
    orphans: 4
  - item: gmeow_gts::writer::Writer::sign_with_openpgp_secret_key
    orphans: 19
  - item: gmeow_gts::writer::Writer::to_bytes
    orphans: 5
  - item: gmeow_gts::writer::Writer::with_layout
    orphans: 17
  - item: gmeow_gts::writer::Writer::with_options
    orphans: 77
  - item: gmeow_gts::writer::WriterError
    orphans: 10
  - item: gmeow_gts::writer::WriterError::fmt
    orphans: 9
  - item: gmeow_gts::writer::WriterError::from
    orphans: 3
  - item: gmeow_gts::writer::WriterOptions
    orphans: 16
  - item: gmeow_gts::writer::WriterOptions::default
    orphans: 8
  - item: gmeow_gts::writer::add_deterministic_blobs
    orphans: 28
  - item: gmeow_gts::writer::annotation_key
    orphans: 4
  - item: gmeow_gts::writer::choose_snapshot_transform
    orphans: 13
  - item: gmeow_gts::writer::default_catalog
    orphans: 39
  - item: gmeow_gts::writer::deterministic_term_remap
    orphans: 28
  - item: gmeow_gts::writer::digest_string
    orphans: 5
  - item: gmeow_gts::writer::graph_row
    orphans: 8
  - item: gmeow_gts::writer::iv
    orphans: 3
  - item: gmeow_gts::writer::map_text
    orphans: 9
  - item: gmeow_gts::writer::nesting_depth
    orphans: 29
  - item: gmeow_gts::writer::quad_key
    orphans: 7
  - item: gmeow_gts::writer::reifier_key
    orphans: 4
  - item: gmeow_gts::writer::remap_id
    orphans: 3
  - item: gmeow_gts::writer::remap_suppression
    orphans: 12
  - item: gmeow_gts::writer::remap_suppression_target
    orphans: 35
  - item: gmeow_gts::writer::remap_term
    orphans: 17
  - item: gmeow_gts::writer::snapshot_from_graph
    orphans: 86
  - item: gmeow_gts::writer::snapshot_payload
    orphans: 91
  - item: gmeow_gts::writer::suppression_key
    orphans: 11
  - item: gmeow_gts::writer::term_identity_value
    orphans: 44
  - item: gmeow_gts::writer::term_to_wire
    orphans: 30
  - item: gmeow_gts::writer::text_or_null
    orphans: 3
  - item: gmeow_gts::writer::uv
    orphans: 3
  - item: gmeow_gts::writer::validate_frame_options
    orphans: 33
  - item: gmeow_gts::writer::value_idx
    orphans: 7
  - item: gmeow_gts::xsd::<impl IllTypedLiteral>
    orphans: 2
  - item: gmeow_gts::xsd::<impl XsdLexicalStatus>
    orphans: 5
  - item: gmeow_gts::xsd::CanonicalInteger
    orphans: 5
  - item: gmeow_gts::xsd::ILL_TYPED_LITERAL_CODE
    orphans: 2
  - item: gmeow_gts::xsd::ILL_TYPED_LITERAL_META_KEY
    orphans: 2
  - item: gmeow_gts::xsd::IllTypedLiteral
    orphans: 12
  - item: gmeow_gts::xsd::IllTypedLiteral::diagnostic
    orphans: 12
  - item: gmeow_gts::xsd::XSD_NS
    orphans: 2
  - item: gmeow_gts::xsd::XsdLexicalStatus
    orphans: 16
  - item: gmeow_gts::xsd::XsdLexicalStatus::canonical
    orphans: 8
  - item: gmeow_gts::xsd::XsdLexicalStatus::is_invalid
    orphans: 5
  - item: gmeow_gts::xsd::XsdLexicalStatus::is_valid
    orphans: 5
  - item: gmeow_gts::xsd::XsdLexicalStatus::reason
    orphans: 8
  - item: gmeow_gts::xsd::annotate_ill_typed_literals
    orphans: 14
  - item: gmeow_gts::xsd::canonical_decimal
    orphans: 41
  - item: gmeow_gts::xsd::canonical_integer
    orphans: 29
  - item: gmeow_gts::xsd::collapse_xml_whitespace
    orphans: 7
  - item: gmeow_gts::xsd::effective_datatype_iri
    orphans: 15
  - item: gmeow_gts::xsd::ill_typed_literal_value
    orphans: 8
  - item: gmeow_gts::xsd::ill_typed_literals
    orphans: 5
  - item: gmeow_gts::xsd::ill_typed_literals_in_terms
    orphans: 21
  - item: gmeow_gts::xsd::ill_typed_literals_metadata
    orphans: 10
  - item: gmeow_gts::xsd::integer_in_range
    orphans: 5
  - item: gmeow_gts::xsd::invalid
    orphans: 3
  - item: gmeow_gts::xsd::is_decimal_mantissa
    orphans: 13
  - item: gmeow_gts::xsd::is_decimal_with_optional_exponent
    orphans: 17
  - item: gmeow_gts::xsd::is_signed_digits
    orphans: 7
  - item: gmeow_gts::xsd::is_xsd_float_lexical
    orphans: 3
  - item: gmeow_gts::xsd::parse_display
    orphans: 10
  - item: gmeow_gts::xsd::replace_xml_whitespace
    orphans: 12
  - item: gmeow_gts::xsd::split_sign
    orphans: 10
  - item: gmeow_gts::xsd::tests::detects_invalid_recognized_xsd_lexicals_without_rejecting_terms
    orphans: 7
  - item: gmeow_gts::xsd::tests::leaves_unrecognized_datatypes_unsupported
    orphans: 7
  - item: gmeow_gts::xsd::tests::validates_and_canonicalizes_common_xsd_lexicals
    orphans: 23
  - item: gmeow_gts::xsd::tests::xsd
    orphans: 3
  - item: gmeow_gts::xsd::valid
    orphans: 3
  - item: gmeow_gts::xsd::validate_float
    orphans: 10
  - item: gmeow_gts::xsd::validate_hex_binary
    orphans: 9
  - item: gmeow_gts::xsd::validate_integer_family
    orphans: 40
  - item: gmeow_gts::xsd::validate_lexical
    orphans: 39
  - item: gmeow_gts::yamlld::ANNOTATION
    orphans: 1
  - item: gmeow_gts::yamlld::AnnotationGroups
    orphans: 1
  - item: gmeow_gts::yamlld::GTS_CONTEXT
    orphans: 1
  - item: gmeow_gts::yamlld::GTS_GRAPH
    orphans: 1
  - item: gmeow_gts::yamlld::GTS_REIFIERS
    orphans: 1
  - item: gmeow_gts::yamlld::GTS_SUBJECT
    orphans: 1
  - item: gmeow_gts::yamlld::GTS_TRIPLE
    orphans: 1
  - item: gmeow_gts::yamlld::RDF_TYPE
    orphans: 1
  - item: gmeow_gts::yamlld::XSD_BOOLEAN
    orphans: 1
  - item: gmeow_gts::yamlld::XSD_DECIMAL
    orphans: 1
  - item: gmeow_gts::yamlld::XSD_INTEGER
    orphans: 1
  - item: gmeow_gts::yamlld::annotation_block
    orphans: 26
  - item: gmeow_gts::yamlld::append_value
    orphans: 14
  - item: gmeow_gts::yamlld::context_object
    orphans: 13
  - item: gmeow_gts::yamlld::expect_object
    orphans: 6
  - item: gmeow_gts::yamlld::node_object
    orphans: 12
  - item: gmeow_gts::yamlld::predicate_key
    orphans: 9
  - item: gmeow_gts::yamlld::standalone_reifier
    orphans: 28
  - item: gmeow_gts::yamlld::statement_value
    orphans: 36
  - item: gmeow_gts::yamlld::term_id
    orphans: 10
  - item: gmeow_gts::yamlld::term_sort_key
    orphans: 7
  - item: gmeow_gts::yamlld::term_sort_keys
    orphans: 5
  - item: gmeow_gts::yamlld::term_value
    orphans: 39
  - item: gmeow_gts::yamlld::to_json_ld
    orphans: 103
  - item: gmeow_gts::yamlld::to_json_ld_string
    orphans: 8
  - item: gmeow_gts::yamlld::to_yaml_ld
    orphans: 8
  - item: gmeow_gts::yamlld::triple_value
    orphans: 7
  - item: gts::<impl ApiError>
    orphans: 7
  - item: gts::<impl FormatCodec>
    orphans: 2
  - item: gts::ABI_VERSION
    orphans: 1
  - item: gts::ApiError
    orphans: 6
  - item: gts::ApiError::diagnostic
    orphans: 7
  - item: gts::ApiError::internal
    orphans: 7
  - item: gts::ApiError::invalid_argument
    orphans: 7
  - item: gts::ApiError::io
    orphans: 7
  - item: gts::ApiError::panic
    orphans: 7
  - item: gts::ApiError::parse
    orphans: 7
  - item: gts::FORMAT_CODECS
    orphans: 56
  - item: gts::FormatCodec
    orphans: 9
  - item: gts::FormatCodec::matches
    orphans: 6
  - item: gts::FormatParser
    orphans: 1
  - item: gts::FormatSerializer
    orphans: 1
  - item: gts::GtsBuffer
    orphans: 15
  - item: gts::GtsError
    orphans: 9
  - item: gts::STATUS_DIAGNOSTIC
    orphans: 1
  - item: gts::STATUS_INTERNAL
    orphans: 1
  - item: gts::STATUS_INVALID_ARGUMENT
    orphans: 1
  - item: gts::STATUS_IO
    orphans: 1
  - item: gts::STATUS_OK
    orphans: 1
  - item: gts::STATUS_PANIC
    orphans: 1
  - item: gts::STATUS_PARSE
    orphans: 1
  - item: gts::UNPACK_ALLOW_SPECIAL
    orphans: 1
  - item: gts::UNPACK_ALLOW_SYMLINKS
    orphans: 1
  - item: gts::UNPACK_INCLUDE_SUPPRESSED
    orphans: 1
  - item: gts::UNPACK_PRESERVE_SETID
    orphans: 1
  - item: gts::UNPACK_SAME_OWNER
    orphans: 1
  - item: gts::VERSION
    orphans: 1
  - item: gts::blob_size
    orphans: 6
  - item: gts::build_metadata_json
    orphans: 17
  - item: gts::c_path
    orphans: 15
  - item: gts::c_paths
    orphans: 17
  - item: gts::clean_graph
    orphans: 20
  - item: gts::clear_error_slot
    orphans: 10
  - item: gts::cstring_lossy
    orphans: 8
  - item: gts::diagnostic_json
    orphans: 7
  - item: gts::ffi_entry
    orphans: 18
  - item: gts::format_entries_json
    orphans: 16
  - item: gts::format_registry_json
    orphans: 6
  - item: gts::graph_json
    orphans: 35
  - item: gts::gts_abi_version
    orphans: 5
  - item: gts::gts_buffer_free
    orphans: 26
  - item: gts::gts_build_metadata_json
    orphans: 18
  - item: gts::gts_capabilities_json
    orphans: 51
  - item: gts::gts_error_code
    orphans: 18
  - item: gts::gts_error_free
    orphans: 17
  - item: gts::gts_error_message
    orphans: 18
  - item: gts::gts_files_diff_json
    orphans: 33
  - item: gts::gts_files_pack
    orphans: 24
  - item: gts::gts_files_unpack
    orphans: 36
  - item: gts::gts_formats_json
    orphans: 15
  - item: gts::gts_from_format
    orphans: 29
  - item: gts::gts_from_nquads
    orphans: 23
  - item: gts::gts_read_json
    orphans: 22
  - item: gts::gts_to_format
    orphans: 28
  - item: gts::gts_to_nquads
    orphans: 22
  - item: gts::gts_verify_json
    orphans: 22
  - item: gts::gts_version
    orphans: 6
  - item: gts::input_cstr
    orphans: 13
  - item: gts::input_slice
    orphans: 14
  - item: gts::input_str
    orphans: 5
  - item: gts::json_bytes
    orphans: 4
  - item: gts::normalize_format
    orphans: 7
  - item: gts::parse_format_to_gts
    orphans: 4
  - item: gts::parse_json_ld_format
    orphans: 3
  - item: gts::parse_nquads_format
    orphans: 3
  - item: gts::parse_ntriples_format
    orphans: 3
  - item: gts::parse_rdf_xml_format
    orphans: 3
  - item: gts::parse_trig_format
    orphans: 3
  - item: gts::parse_turtle_format
    orphans: 3
  - item: gts::require_out
    orphans: 7
  - item: gts::resolve_format
    orphans: 12
  - item: gts::serialize_gts_as_format
    orphans: 5
  - item: gts::serialize_json_ld_format
    orphans: 4
  - item: gts::serialize_ntriples_format
    orphans: 3
  - item: gts::serialize_rdf_xml_format
    orphans: 3
  - item: gts::serialize_trig_format
    orphans: 3
  - item: gts::serialize_turtle_format
    orphans: 3
  - item: gts::tests::call_buffer
    orphans: 42
  - item: gts::tests::capabilities_and_build_metadata_are_reported
    orphans: 30
  - item: gts::tests::format_registry_reports_supported_codecs
    orphans: 17
  - item: gts::tests::invalid_nquads_reports_structured_error
    orphans: 34
  - item: gts::tests::media_type_and_extension_dispatch_share_the_registry
    orphans: 31
  - item: gts::tests::read_verify_and_nquads_round_trip
    orphans: 38
  - item: gts::tests::registered_formats_round_trip_through_c_abi
    orphans: 39
  - item: gts::tests::sample_default_graph_gts
    orphans: 3
  - item: gts::tests::unpack_rejects_null_output_before_side_effects
    orphans: 35
  - item: gts::tests::unsupported_format_reports_structured_error
    orphans: 39
  - item: gts::tests::vectors
    orphans: 3
  - item: gts::unpack_options
    orphans: 9
  - item: gts::verify_json
    orphans: 26
  - item: gts::write_buffer
    orphans: 30
  - item: gts::write_error
    orphans: 15
---
