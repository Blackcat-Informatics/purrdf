// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, caller-configured RDF 1.2→SKOS concept-scheme projection.

mod config;
mod mapping;

pub use config::{
    SkosClassRoles, SkosConfig, SkosDocumentationRoles, SkosGraphSelection, SkosLabelRoles,
    SkosRelationRoles, SkosSourceRoles, SkosTargetRoles,
};
pub use mapping::{SkosProjection, project_skos};
