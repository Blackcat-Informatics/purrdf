// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format-neutral research-object semantic model and native profile codecs.

mod config;
mod croissant;
mod datacite;
mod json;
mod mapping;
mod model;
mod ro_crate;

pub use config::{
    RESEARCH_ROLES, ResearchObjectConfig, ResearchObjectIdentity, ResearchObjectPolicy,
    ResearchObjectRoles, ResearchRole,
};
pub use croissant::{
    CROISSANT_ARTIFACT, CROISSANT_PROFILE, CROISSANT_ROLES, CroissantConfig, CroissantRole,
    CroissantVocabulary, project_croissant, read_croissant,
};
pub use datacite::{
    DATACITE_ARTIFACT, DATACITE_PROFILE, DataCiteConfig, DataCiteControlledValues,
    project_datacite, read_datacite,
};
pub use json::{OfflineJsonLdContext, ResearchObjectPackageProjection, ResearchObjectReadOutcome};
pub use mapping::{ResearchObjectProjection, lift_research_object, project_research_object};
pub use model::{
    ResearchActivity, ResearchAgent, ResearchChecksum, ResearchDataset, ResearchField,
    ResearchObjectModel, ResearchRecordSet, ResearchResource, ResearchText, ResearchValue,
};
pub use ro_crate::{
    RO_CRATE_ARTIFACT, RO_CRATE_PROFILE, RO_CRATE_ROLES, RoCrateConfig, RoCrateRole,
    RoCrateVocabulary, project_ro_crate, read_ro_crate,
};
