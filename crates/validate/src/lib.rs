// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-validate` — the **SARIF 2.1.0 reporting boundary** for PurRDF.
//!
//! PurRDF keeps its kernel (`purrdf-core`) *structured but SARIF-free*: parse
//! failures are [`RdfDiagnostic`]s, SHACL results are [`ValidationReport`]s, and
//! neither knows anything about SARIF or serde. This crate is where that
//! structured data crosses the boundary into a **source-traced, byte-deterministic
//! SARIF 2.1.0 log** for editors, CI, and code-scanning dashboards.
//!
//! # What lives here (and why here)
//!
//! * The hand-rolled SARIF serde model (no heavyweight SARIF dependency).
//! * The mappings from PurRDF severities/rules/locations to SARIF
//!   `level`/`ruleId`/`physicalLocation`/`logicalLocation`.
//! * The resolution of runtime-only provenance ids (`UnitId`) to public slice
//!   IRIs — this is the serialization boundary where S0.5 permits it; the numeric
//!   ids never enter the emitted JSON.
//!
//! Hosting the writer in this leaf keeps the kernel ring-fence intact: `purrdf-core`
//! and `purrdf-shapes` never gain a SARIF or serde-derive concern.
//!
//! # The shared string boundary
//!
//! SARIF is the crate's origin, not the whole of it. This is also where the
//! language bindings' **string-in / string-out** entry points live, so the C-ABI,
//! WASM and PyO3 callers share one implementation instead of three:
//!
//! * [`shacl::validate_to_sarif_string`] — SHACL validation → SARIF JSON.
//! * [`shacl::validate_changes_to_sarif_string`] — the incremental twin: a
//!   data graph plus both halves of a change, validated through the engine's
//!   change path, returning the SARIF log beside the scope it describes.
//! * [`entail::entail_to_ntriples_string`] — SHACL-AF `sh:rule` entailment →
//!   canonical N-Triples.
//! * [`regime`] — SPARQL entailment-regime materialization → canonical N-Quads
//!   plus a deterministically rendered [`ReasoningReport`]. Despite the name, this
//!   is *not* the same thing as [`entail`]; that module's docs spell the
//!   difference out.
//!
//! [`ReasoningReport`]: purrdf_entail::ReasoningReport
//!
//! # Portability
//!
//! Pure serde over the report types — no PyO3, no oxigraph-family edge, no ambient
//! I/O — so the crate stays `wasm32-unknown-unknown`-clean like every release crate.
//!
//! [`RdfDiagnostic`]: purrdf_core::RdfDiagnostic
//! [`ValidationReport`]: purrdf_shapes::report::ValidationReport
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

pub mod build;
pub mod entail;
pub mod model;
pub mod path_syntax;
pub mod product;
pub mod regime;
pub mod rules;
pub mod shacl;

pub use build::{
    SarifOptions, SarifReport, SarifSources, build_diagnostics_sarif, build_report_sarif,
    build_report_sarif_with, diagnostics_to_sarif_string, report_to_sarif_string,
};
pub use entail::entail_to_ntriples_string;
pub use model::{Level, ResultKind, SARIF_SCHEMA, SARIF_VERSION, SarifLog, to_json_pretty};
pub use product::{
    IdentityComponentDiff, ShapesProductDiff, ShapesProductRefusal, admit_shapes_product,
    admit_shapes_product_expecting, admit_shapes_product_with_implementations,
    certify_shapes_product, diff_shapes_products, explain_shapes_product, pack_shapes_product,
    pack_shapes_product_from_dataset, parse_identity_digest, prepared_to_product,
    prepared_to_product_with_implementations, rebuild_shapes_product,
    rebuild_shapes_product_expecting, validate_with_rebuilt_shapes_product,
    validate_with_rebuilt_shapes_product_expecting, validate_with_shapes_product,
    validate_with_shapes_product_expecting,
};
pub use regime::{
    ABSENT_DL_PROOF, DL_PROOF_BANNER, DL_PROOF_CHECK_BANNER, DL_PROOF_GOLDEN_VECTORS,
    DlProofVector, INCONSISTENT_DOCUMENT, ImportList, PROGRAM_REGIME_NAMES, PROOF_SERVICE_NAMES,
    REGIME_GOLDEN_VECTORS, REGIME_NAMES, REPORT_FORMAT_BANNER, RegimeClosure, RegimeVector,
    certain_answers_to_string, check_absent_proof_is_not_verifiable, check_dl_proof,
    check_dl_proof_golden_vectors, check_inconsistent_refusal, check_regime_golden_vectors,
    decode_dl_proof, dl_proof_golden_vectors, graph_entails_to_string, implemented_rules_string,
    materialize_to_nquads_string, parse_regime, prove_to_string, regime_golden_vectors,
    regime_name, regime_plan, regime_rule_set, render_dl_proof, render_entail_error,
    render_reasoning_report, rules_string, verify_entailment_to_string,
};
// The engine's own change-path scope, re-exported because
// [`shacl::validate_changes_to_sarif_string`] RETURNS one: a binding that cannot
// name the type it is handed would have to re-spell it, and two spellings of one
// answer is how the two arms end up collapsed.
pub use purrdf_shapes::engine::ChangeScope;
/// The validation-request options and the conformance-disallow set they carry,
/// re-exported so a host binding names them without depending on the engine
/// crate — [`SarifOptions::validation`] is where they travel.
pub use purrdf_shapes::engine::ValidationOptions;
pub use purrdf_shapes::report::ConformanceDisallows;
pub use shacl::{validate_changes_to_sarif_string, validate_to_sarif_string};
