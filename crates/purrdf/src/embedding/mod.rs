// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independently certified source attachments and canonical PURREMB writers.
//! Source-free borrowed views, numeric verification and resident certificates
//! remain in the kernel; this layer owns certified metadata construction.

mod metadata;
mod source;
mod wire;
mod writer;

pub use metadata::{CanonicalMetadataInput, CertifiedPurrpckSource};
pub use purrdf_core::embedding::*;
pub use source::{
    SourceVerificationReport, bind_external_pack, verify_embedding_source, verify_external_pack,
};
pub use wire::EncodedArtifact;
pub use writer::{
    CanonicalMetadataSections, EmbeddingBuilder, EmbeddingStreamWriter, MatrixInput, MatrixRow,
    ProjectionSpec,
};
