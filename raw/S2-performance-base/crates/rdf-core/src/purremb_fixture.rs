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
    AppliedStage, ArtifactIdentity, ArtifactIdentityKind, ContentDigest, DimensionalityPolicy,
    DistanceMetric, EmbeddingFamilyContract, PrefixPostprocessing, StageImplementation,
    VectorDtype,
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

    /// The plain cosine `f32` family contract of `dimensions` fixed dimensions: the
    /// `model`, `engine` and `tokenizer` artifacts, the `execution` and `pooling` stages,
    /// the subject-projection stage named `projection`, and no preprocessing, chunking,
    /// normalization, truncation, prefix post-processing or extensions.
    ///
    /// # Panics
    ///
    /// Panics if `dimensions` is not a valid fixed dimensionality.
    #[must_use]
    pub fn cosine_contract(&self, projection: &str, dimensions: u32) -> EmbeddingFamilyContract {
        EmbeddingFamilyContract {
            model: self.artifact("model"),
            engine: self.artifact("engine"),
            tokenizer: self.artifact("tokenizer"),
            execution: self.stage("execution"),
            subject_projection: self.stage(projection),
            preprocessing: AppliedStage::NotApplied,
            chunking: AppliedStage::NotApplied,
            pooling: self.stage("pooling"),
            normalization: AppliedStage::NotApplied,
            truncation: AppliedStage::NotApplied,
            dtype: VectorDtype::F32,
            metric: DistanceMetric::Cosine,
            dimensionality: DimensionalityPolicy::fixed(dimensions, PrefixPostprocessing::None)
                .expect("a fixed dimensionality"),
            extensions: Vec::new(),
        }
    }
}

/// The little-endian `u32` at `offset` of an image a test edits on purpose: the
/// panicking form of [`crate::bytes::read_u32_le`].
///
/// # Panics
///
/// Panics if the field does not fit inside `bytes`.
#[must_use]
pub fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    crate::bytes::read_u32_le(bytes, offset).expect("a u32 field inside the image")
}

/// The little-endian `u64` at `offset`: the panicking form of
/// [`crate::bytes::read_u64_le`].
///
/// # Panics
///
/// Panics if the field does not fit inside `bytes`.
#[must_use]
pub fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    crate::bytes::read_u64_le(bytes, offset).expect("a u64 field inside the image")
}

/// Overwrite the little-endian `u32` at `offset`: the panicking form of
/// [`crate::bytes::put_u32_le`].
///
/// # Panics
///
/// Panics if the field does not fit inside `bytes`.
pub fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    crate::bytes::put_u32_le(bytes, offset, value).expect("a u32 field inside the image");
}

/// Overwrite the little-endian `u64` at `offset`: the panicking form of
/// [`crate::bytes::put_u64_le`].
///
/// # Panics
///
/// Panics if the field does not fit inside `bytes`.
pub fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    crate::bytes::put_u64_le(bytes, offset, value).expect("a u64 field inside the image");
}

/// A `u32` field of a PURREMB image as a `usize`.
fn field_u32(bytes: &[u8], offset: usize) -> usize {
    usize::try_from(read_u32(bytes, offset)).expect("a u32 field fits usize")
}

/// A `u64` field of a PURREMB image as a `usize`.
fn field_u64(bytes: &[u8], offset: usize) -> usize {
    usize::try_from(read_u64(bytes, offset)).expect("a u64 field fits usize")
}

/// The byte offset of the section directory entry for `(kind, instance)` in a PURREMB
/// image, for tests that corrupt a sealed artifact on purpose.
///
/// # Panics
///
/// Panics if the image holds no such entry.
#[must_use]
pub fn directory_entry(bytes: &[u8], kind: u32, instance: u32) -> usize {
    let header = crate::PURREMB_HEADER_LENGTH as usize;
    let entry_length = crate::PURREMB_DIRECTORY_ENTRY_LENGTH as usize;
    let (kind, instance) = (kind as usize, instance as usize);
    (0..field_u32(bytes, 20))
        .map(|index| header + index * entry_length)
        .find(|&offset| {
            field_u32(bytes, offset) == kind && field_u32(bytes, offset + 8) == instance
        })
        .expect("section directory entry")
}

/// The `(offset, length)` of the section `(kind, instance)` in a PURREMB image.
///
/// # Panics
///
/// Panics if the image holds no such section.
#[must_use]
pub fn section_span(bytes: &[u8], kind: u32, instance: u32) -> (usize, usize) {
    let entry = directory_entry(bytes, kind, instance);
    (field_u64(bytes, entry + 16), field_u64(bytes, entry + 24))
}

/// Re-seal a deliberately edited PURREMB image: recompute the digest of every listed
/// `(kind, instance)` section, then the artifact root over the header and directory, and
/// write the root into the header and the trailer. The framing then verifies, so a test
/// reaches the deeper check its corruption targets.
///
/// # Panics
///
/// Panics if a listed section is absent or the image is truncated.
pub fn reseal(bytes: &mut [u8], sections: &[(u32, u32)]) {
    for &(kind, instance) in sections {
        let entry = directory_entry(bytes, kind, instance);
        let (offset, length) = section_span(bytes, kind, instance);
        let digest = ContentDigest::of(&bytes[offset..offset + length]);
        bytes[entry + 32..entry + 64].copy_from_slice(digest.as_bytes());
    }
    let header_length = crate::PURREMB_HEADER_LENGTH as usize;
    let directory_end =
        header_length + field_u32(bytes, 20) * crate::PURREMB_DIRECTORY_ENTRY_LENGTH as usize;
    let mut header = vec![0_u8; header_length];
    header.copy_from_slice(&bytes[..header_length]);
    header[64..96].fill(0);
    let root = crate::derive_artifact_root(&header, &bytes[header_length..directory_end]);
    bytes[64..96].copy_from_slice(root.as_bytes());
    let trailer = field_u64(bytes, 48);
    bytes[trailer + 24..trailer + 56].copy_from_slice(root.as_bytes());
}

/// PURREMB §13.2's scaled L2 fold, transcribed from the specification's written order: the
/// oracle the float-environment tests compare the shipping norm against, and run on a
/// flushing thread to show what that thread would compute. `black_box` keeps it from being
/// folded at compile time under the default environment.
#[must_use]
pub fn reference_norm(values: &[f64]) -> f64 {
    let mut scale = 0.0_f64;
    let mut ssq = 1.0_f64;
    for &value in core::hint::black_box(values) {
        let value = value.abs();
        if value == 0.0 {
            continue;
        }
        if scale < value {
            let ratio = scale / value;
            let square = ratio * ratio;
            let product = ssq * square;
            ssq = 1.0 + product;
            scale = value;
        } else {
            let ratio = value / scale;
            let square = ratio * ratio;
            ssq += square;
        }
    }
    let root = ssq.sqrt();
    core::hint::black_box(scale) * root
}
