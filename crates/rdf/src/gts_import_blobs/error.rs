// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned budget context, independent of the imported source's lifetime.

use super::{GtsBlobLimits, GtsBlobUsage};
use purrdf_gts::model::DiagnosticCause;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

/// Original CBOR sink refusal, with the caller's exact metadata allowance.
#[derive(Debug)]
pub(super) struct MetadataBudgetRefusal {
    pub limit: usize,
    pub written: usize,
    pub cause: purrdf_hash::error::IoFailure,
}

impl core::fmt::Display for MetadataBudgetRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "selected blob metadata exceeds its {} byte budget after {} bytes: {}",
            self.limit, self.written, self.cause
        )
    }
}

impl std::error::Error for MetadataBudgetRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

impl StructuredError for MetadataBudgetRefusal {
    purrdf_hash::error_visit! { @record self { limit, written, cause } => "gts-blob-metadata-limit" {
        "limit" => ByteCount(self.limit as u128),
        "written" => ByteCount(self.written as u128),
        "cause" => Cause(&self.cause),
    } }
}

impl From<MetadataBudgetRefusal> for crate::RdfDiagnostic {
    fn from(failure: MetadataBudgetRefusal) -> Self {
        let attach = || -> Result<Self, Self> {
            let mut resident = purrdf_lex::allocation::Resident;
            let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
            let message = memory.format(&failure)?;
            let cause = purrdf_gts::model::retain_native_cause(failure)?;
            Ok(Self::error("rdf-ir-gts-blob-limit", message)
                .with_native_cause(cause, &mut memory)?)
        };
        match attach() {
            Ok(error) | Err(error) => error,
        }
    }
}

/// A selected payload refused with the actual ceilings and retained charges.
#[non_exhaustive]
#[derive(Debug)]
pub struct GtsBlobRetentionRefusal {
    /// Published digest, which may be an unverified container claim.
    pub digest: Option<String>,
    /// Whether this occurrence's bytes proved the published identity.
    pub digest_computed: bool,
    /// Encoded bytes in the refused occurrence.
    pub encoded_bytes: usize,
    /// Caller-supplied byte ceilings.
    pub limits: GtsBlobLimits,
    /// Cumulative allowance for this occurrence after refunding a proved
    /// matching digest's old bytes, before recording this refusal's metadata.
    /// A reader may refuse its encoded or per-blob decode ceiling first.
    pub available_decoded_bytes: usize,
    /// Charges and peaks immediately before the refused decode.
    pub usage: GtsBlobUsage,
    /// Segment of the refused occurrence.
    pub segment_index: u64,
    /// Frame of the refused occurrence.
    pub frame_index: u64,
    /// Original transform or reader refusal, kept by its existing shared owner.
    pub(super) cause: Option<DiagnosticCause>,
}

impl GtsBlobRetentionRefusal {
    /// Borrow the original refusal without copying or rendering its fields.
    #[must_use]
    pub fn native_cause(&self) -> Option<&(dyn purrdf_hash::error::NativeError + 'static)> {
        match self.cause.as_ref()? {
            DiagnosticCause::Native(cause) => Some(&***cause),
            DiagnosticCause::Storage(cause) => Some(cause),
        }
    }
}

impl core::fmt::Display for GtsBlobRetentionRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "blob {} in segment {}, frame {} has {} encoded bytes; ceilings: encoded {}, decoded {}, retained {}; {} decoded bytes remain ({} payload and {} refused-metadata bytes retained)",
            self.digest.as_deref().unwrap_or("unidentified"),
            self.segment_index,
            self.frame_index,
            self.encoded_bytes,
            self.limits.max_encoded_bytes,
            self.limits.max_decoded_bytes,
            self.limits.max_total_decoded_bytes,
            self.available_decoded_bytes,
            self.usage.retained_payload_bytes,
            self.usage.retained_refusal_metadata_bytes
        )?;
        if let Some(cause) = self.native_cause() {
            write!(f, ": {cause}")?;
        }
        Ok(())
    }
}

purrdf_hash::error_sources!(GtsBlobRetentionRefusal, native native_cause);

impl StructuredError for GtsBlobRetentionRefusal {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("gts-blob-retention-refusal");
        visitor.value(
            Some("digest"),
            self.digest
                .as_deref()
                .map_or(ErrorValue::Absent, ErrorValue::Text),
        );
        visitor.value(
            Some("digest_computed"),
            ErrorValue::Boolean(self.digest_computed),
        );
        visitor.value(
            Some("encoded_bytes"),
            ErrorValue::ByteCount(self.encoded_bytes as u128),
        );
        visitor.value(Some("limits"), ErrorValue::Cause(&self.limits));
        visitor.value(
            Some("available_decoded_bytes"),
            ErrorValue::ByteCount(self.available_decoded_bytes as u128),
        );
        visitor.value(Some("usage"), ErrorValue::Cause(&self.usage));
        visitor.value(
            Some("segment_index"),
            ErrorValue::Unsigned(u128::from(self.segment_index)),
        );
        visitor.value(
            Some("frame_index"),
            ErrorValue::Unsigned(u128::from(self.frame_index)),
        );
        visitor.value(
            Some("cause"),
            self.native_cause()
                .map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause)),
        );
    }
}

impl StructuredError for GtsBlobLimits {
    purrdf_hash::error_visit! { @record self { max_encoded_bytes, max_decoded_bytes, max_total_decoded_bytes, max_metadata_bytes, max_frame_decoded_bytes } => "gts-blob-limits" {
        "max_encoded_bytes" => ByteCount(self.max_encoded_bytes as u128),
        "max_decoded_bytes" => ByteCount(self.max_decoded_bytes as u128),
        "max_total_decoded_bytes" => ByteCount(self.max_total_decoded_bytes as u128),
        "max_metadata_bytes" => ByteCount(self.max_metadata_bytes as u128),
        "max_frame_decoded_bytes" => ByteCount(self.max_frame_decoded_bytes as u128),
    } }
}

impl StructuredError for GtsBlobUsage {
    purrdf_hash::error_visit! { @record self { retained_payload_bytes, retained_refusal_metadata_bytes, peak_payload_bytes, peak_retained_bytes, peak_selected_digests } => "gts-blob-usage" {
        "retained_payload_bytes" => ByteCount(self.retained_payload_bytes as u128),
        "retained_refusal_metadata_bytes" => ByteCount(self.retained_refusal_metadata_bytes as u128),
        "peak_payload_bytes" => ByteCount(self.peak_payload_bytes as u128),
        "peak_retained_bytes" => ByteCount(self.peak_retained_bytes as u128),
        "peak_selected_digests" => Unsigned(self.peak_selected_digests as u128),
    } }
}
