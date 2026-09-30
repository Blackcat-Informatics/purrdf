// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The PURREMB artifact and stage identities every embedding test and bench declares
//! its family contract with.
//!
//! This is test support, not API: it is hidden from the documentation and carries no
//! stability promise. It lives in the library so that each crate's tests import one
//! definition rather than compiling a copy of it. The identities are distinct per
//! `name` and derive from it alone: the IRI is the fixture's base followed by the
//! name, and the content digest is the digest of the name's bytes. The parts a suite's
//! sealed bytes or goldens depend on (the IRI bases, the stage payload, the media
//! types, the artifact salt) are fields, so a suite keeps the identities it has always
//! sealed.

use crate::{
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, ContentDigest, StageImplementation,
};

/// One suite's identity vocabulary: where its artifact and stage IRIs live and what
/// the stages carry.
#[derive(Clone, Copy, Debug)]
pub struct Identities {
    /// The text an artifact's IRI starts with; the name follows it.
    pub artifact_base: &'static str,
    /// The text a stage's IRI starts with; the name follows it.
    pub stage_base: &'static str,
    /// The artifact's media type.
    pub artifact_media: &'static str,
    /// The artifact's salt, if the suite seals one.
    pub artifact_salt: Option<&'static [u8]>,
    /// The stage's parameter media type.
    pub stage_media: &'static str,
    /// The stage's parameter bytes.
    pub stage_payload: &'static [u8],
}

impl Identities {
    /// Artifacts and stages both under `base`: `application/octet-stream`, no salt, the
    /// one-byte payload `[1]`.
    #[must_use]
    pub const fn at(base: &'static str) -> Self {
        Self {
            artifact_base: base,
            stage_base: base,
            artifact_media: "application/octet-stream",
            artifact_salt: None,
            stage_media: "application/octet-stream",
            stage_payload: &[1],
        }
    }

    /// The artifact identity named `name`.
    ///
    /// # Panics
    ///
    /// Panics if the fixture's fields do not form a valid identity (an empty IRI or media
    /// type, or one carrying NUL).
    #[must_use]
    pub fn artifact(&self, name: &str) -> ArtifactIdentity {
        ArtifactIdentity::new(
            format!("{}{name}", self.artifact_base),
            self.artifact_media,
            ContentDigest::of(name.as_bytes()),
            self.artifact_salt.map(<[u8]>::to_vec),
            ArtifactIdentityKind::Single,
        )
        .expect("the fixture artifact identity is well formed")
    }

    /// The applied stage named `name`.
    ///
    /// # Panics
    ///
    /// Panics if the fixture's fields do not form a valid stage.
    #[must_use]
    pub fn stage(&self, name: &str) -> AppliedStage {
        AppliedStage::Applied(
            StageImplementation::new(
                format!("{}{name}", self.stage_base),
                ContentDigest::of(name.as_bytes()),
                self.stage_media,
                self.stage_payload.to_vec(),
            )
            .expect("the fixture stage is well formed"),
        )
    }
}
