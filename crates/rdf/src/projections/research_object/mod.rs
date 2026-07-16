// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format-neutral research-object semantic model and native profile codecs.

mod config;
mod croissant;
mod json;
mod mapping;
mod model;

pub use config::{
    RESEARCH_ROLES, ResearchObjectConfig, ResearchObjectIdentity, ResearchObjectPolicy,
    ResearchObjectRoles, ResearchRole,
};
pub use croissant::{
    CROISSANT_ARTIFACT, CROISSANT_PROFILE, CROISSANT_ROLES, CroissantConfig, CroissantRole,
    CroissantVocabulary, project_croissant, read_croissant,
};
pub use json::{OfflineJsonLdContext, ResearchObjectPackageProjection, ResearchObjectReadOutcome};
pub use mapping::{ResearchObjectProjection, lift_research_object, project_research_object};
pub use model::{
    ResearchActivity, ResearchAgent, ResearchChecksum, ResearchDataset, ResearchField,
    ResearchObjectModel, ResearchRecordSet, ResearchResource, ResearchText, ResearchValue,
};
