// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Caller-vocabulary VoID dataset descriptions, partitions, and linksets.

mod config;
mod mapping;

pub use config::{
    VOID_ROLES, VoidConfig, VoidDatasetPrefix, VoidExecutionLimits, VoidExternalLinkMapping,
    VoidGraphSelector, VoidRole, VoidSourceRoles, VoidStaticStatement, VoidStaticValue,
    VoidVocabulary,
};
pub use mapping::project_void;
