// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
pub mod regime;
pub mod rules;
pub mod shacl;

pub use build::{
    SarifOptions, SarifReport, SarifSources, build_diagnostics_sarif, build_report_sarif,
    build_report_sarif_with, diagnostics_to_sarif_string, report_to_sarif_string,
};
pub use entail::entail_to_ntriples_string;
pub use model::{Level, SARIF_SCHEMA, SARIF_VERSION, SarifLog, to_json_pretty};
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
pub use shacl::validate_to_sarif_string;
