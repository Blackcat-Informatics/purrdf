// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
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
//! # Portability
//!
//! Pure serde over the report types — no PyO3, no oxigraph-family edge, no ambient
//! I/O — so the crate stays `wasm32-unknown-unknown`-clean like every release crate.
//!
//! [`RdfDiagnostic`]: purrdf_core::RdfDiagnostic
//! [`ValidationReport`]: purrdf_shapes::report::ValidationReport

#![forbid(unsafe_code)]

pub mod build;
pub mod entail;
pub mod model;
pub mod path_syntax;
pub mod rules;
pub mod shacl;

pub use build::{
    SarifOptions, SarifReport, SarifSources, build_diagnostics_sarif, build_report_sarif,
    build_report_sarif_with, diagnostics_to_sarif_string, report_to_sarif_string,
};
pub use entail::entail_to_ntriples_string;
pub use model::{Level, SARIF_SCHEMA, SARIF_VERSION, SarifLog, to_json_pretty};
pub use shacl::validate_to_sarif_string;
