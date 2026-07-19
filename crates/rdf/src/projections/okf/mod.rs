// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, caller-configured RDF 1.2 term views as OKF knowledge bundles.

mod config;

pub use config::{
    OKF_TERMS_PROFILE, OkfBodySection, OkfBodyStyle, OkfBodyValueMode, OkfCardinality, OkfCategory,
    OkfConceptSelector, OkfFieldMapping, OkfFrontmatterMappings, OkfGenerationConfig,
    OkfGraphSelection, OkfIndexConfig, OkfLinkPathStyle, OkfLinkSection, OkfLinkStyle,
    OkfLinkTargetMode, OkfPathStrategy, OkfResourceMapping, OkfTermRendering, OkfValueMode,
};
