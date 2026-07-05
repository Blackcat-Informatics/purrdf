// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-slice` — native slice catalog: manifest-based discovery, typed
//! artifact inventory, and content-addressed IDs for the PurRDF ontology slices.

pub mod analysis;
pub mod artifact;
pub mod cache;
pub mod catalog;
pub mod claim_view;
pub mod diagnostics;
pub mod dsl_stats_emit;
pub mod error;
pub mod fix_deps;
pub mod list_functions;
pub mod mapping_support;
pub mod ownership;
pub mod prefix_emit;
pub mod prefix_lint;
pub mod rdf_query;
pub mod standpoint_emit;
pub mod standpoint_modality;
pub mod vocab;

pub use analysis::{
    AnalysisError, AnalysisGraph, bundle_content_id, emit_analysis_graph, is_forbidden_edge,
};
pub use artifact::{ArtifactRecord, ArtifactRole};
pub use cache::{
    CacheKey, LinkUnit, Phase, ProductUnit, ToolchainContext, dependency_closure, link_unit_key,
    link_units, product_unit, product_unit_key, source_unit_key,
};
pub use catalog::{ManifestView, SliceCatalog, SliceRecord, SliceTier};
pub use claim_view::{CLAIM_VIEW_FILE, emit_claim_view};
pub use diagnostics::ProjectionDiagnostic;
pub use dsl_stats_emit::emit_dsl_stats;
pub use error::SliceError;
pub use fix_deps::{ManifestPatch, compute_fix_deps};
pub use list_functions::emit_list_functions;
pub use ownership::{
    ArtifactEvidence, DependencyEdge, EdgeEvidence, EdgeKind, OwnershipAnalyzer,
    OwnershipDiagnostic, OwnershipReport, OwnershipStatus, ReconciliationStatus, SliceIri,
    TermOwnership,
};
pub use prefix_emit::{emit_core_prefixes, emit_jsonld_context};
pub use prefix_lint::lint_prefix_consistency;
pub use rdf_query::NamedNode;
pub use standpoint_emit::emit_standpoint_sets;
pub use vocab::SliceVocab;
