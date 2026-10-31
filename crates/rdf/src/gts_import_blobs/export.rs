// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicitly lossy all-inline admission, sharing the strict blob collector.

use super::{
    BlobCollector, GtsBlobLimits, GtsBlobMetadataSource, GtsBlobRetentionRefusal, GtsBlobUsage,
    GtsImportedBlob, RdfDiagnostic,
};
use purrdf_gts::reader::{BlobPayload, BlobRefusalKind};
use purrdf_lex::allocation::{Memory, Resident};
use std::sync::Arc;

/// Why an export could not produce an admitted inline payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GtsBlobOmissionKind {
    /// An explicit payload or cumulative byte ceiling.
    ByteBudget,
    /// An explicit public-metadata byte ceiling.
    MetadataBudget,
    /// A recognized codec, key or decryptor capability is unavailable.
    UnavailableCapability,
    /// A declared reference had no admitted inline payload at the end.
    MissingInlinePayload,
}

impl GtsBlobOmissionKind {
    /// Stable machine-readable reason.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ByteBudget => "blob-byte-budget",
            Self::MetadataBudget => "blob-metadata-budget",
            Self::UnavailableCapability => "blob-capability-unavailable",
            Self::MissingInlinePayload => "blob-inline-payload-unavailable",
        }
    }
}

/// One final omitted reference or payload occurrence, with owned native cause.
///
/// An unproved digest is only the container's claim. Such an occurrence cannot
/// erase admitted bytes or be deduplicated with a proved content identity.
#[derive(Clone, Debug)]
pub struct GtsBlobOmission {
    kind: GtsBlobOmissionKind,
    digest: Option<String>,
    digest_computed: bool,
    encoded_bytes: usize,
    pub(super) source: GtsBlobMetadataSource,
    limits: GtsBlobLimits,
    usage: GtsBlobUsage,
    cause: Option<RdfDiagnostic>,
}

impl GtsBlobOmission {
    /// Original typed admission reason.
    #[must_use]
    pub const fn kind(&self) -> GtsBlobOmissionKind {
        self.kind
    }
    /// Proved identity or unverified declared digest, as distinguished below.
    #[must_use]
    pub fn digest(&self) -> Option<&str> {
        self.digest.as_deref()
    }
    /// Whether this occurrence's bytes proved the digest.
    #[must_use]
    pub const fn digest_computed(&self) -> bool {
        self.digest_computed
    }
    /// Encoded bytes in the omitted occurrence.
    #[must_use]
    pub const fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
    /// Exact original physical source of the occurrence or reference.
    #[must_use]
    pub const fn source(&self) -> &GtsBlobMetadataSource {
        &self.source
    }
    /// Original caller-owned admission ceilings.
    #[must_use]
    pub const fn limits(&self) -> GtsBlobLimits {
        self.limits
    }
    /// Logical admission charges before this occurrence was omitted.
    #[must_use]
    pub const fn usage(&self) -> GtsBlobUsage {
        self.usage
    }
    /// Owned diagnostic retaining the original native cause and arguments.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<&RdfDiagnostic> {
        self.cause.as_ref()
    }
    /// Borrow the original cause without rendering or reclassifying it.
    #[must_use]
    pub fn native_cause(&self) -> Option<&(dyn purrdf_hash::error::NativeError + 'static)> {
        self.cause.as_ref().and_then(RdfDiagnostic::native_cause)
    }
}

/// Eager all-inline export admission, with truthful final omissions.
///
/// This retains the admitted payload set. It is not the chunked streaming API.
/// RDF data and frame integrity remain mandatory; only the named blob admission
/// cases are recoverable. Blob metadata is not copied into an unused lookaside.
#[derive(Debug)]
pub struct GtsInlineExport {
    /// The authoritative scoped native RDF dataset.
    dataset: Arc<crate::RdfDataset>,
    /// Admitted unique decoded blobs, in deterministic digest order.
    blobs: Vec<GtsImportedBlob>,
    /// Final omissions, excluding proved identities subsequently admitted.
    omissions: Vec<GtsBlobOmission>,
    /// Logical charges and peaks during admission, not total process memory.
    usage: GtsBlobUsage,
}

impl GtsInlineExport {
    /// Consume the eager admission result without copying its payload owners.
    pub fn into_parts(
        self,
    ) -> (
        Arc<crate::RdfDataset>,
        Vec<GtsImportedBlob>,
        Vec<GtsBlobOmission>,
        GtsBlobUsage,
    ) {
        (self.dataset, self.blobs, self.omissions, self.usage)
    }
}

/// Admit every inline payload with explicit caller ceilings and report omissions.
///
/// Required-selector imports retain their strict contract. Structural, storage,
/// integrity, RDF and non-blob frame failures still abort this operation.
pub fn import_gts_inline_for_export(
    bytes: &[u8],
    limits: GtsBlobLimits,
) -> Result<GtsInlineExport, RdfDiagnostic> {
    let mut collector = BlobCollector::new(&[], limits)?;
    collector.export = Some(Vec::new());
    let (bundle, collector) =
        crate::gts_import_sink::import_with_collector(bytes, Some(collector))?;
    let collector = collector.ok_or_else(|| {
        RdfDiagnostic::error(
            "rdf-ir-gts-import-internal",
            "inline export collector is missing",
        )
    })?;
    collector.finish_export(bundle.dataset)
}

impl BlobCollector<'_> {
    pub(crate) const fn is_inline_export(&self) -> bool {
        self.export.is_some()
    }

    pub(crate) fn expects_capability_refusal(&self) -> bool {
        self.export.is_some()
            && self.refused.last().is_some_and(|refusal| {
                refusal.kind == BlobRefusalKind::UnavailableCapability && refusal.cause.is_none()
            })
    }

    pub(super) fn export_source(&self) -> Result<GtsBlobMetadataSource, RdfDiagnostic> {
        let frame = self.frame.as_ref().ok_or_else(|| {
            RdfDiagnostic::error(
                "rdf-ir-gts-blob-provenance",
                "omitted blob has no source frame",
            )
        })?;
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        Ok(GtsBlobMetadataSource {
            segment_index: frame.segment_index,
            frame_index: frame.frame_index,
            frame_id: memory.collect(frame.frame_id.iter().copied())?,
            frame_range: frame.range.clone(),
            segment_head: Vec::new(),
        })
    }

    pub(super) fn omit_or_error(
        &mut self,
        payload: BlobPayload<'_>,
        kind: GtsBlobOmissionKind,
        error: RdfDiagnostic,
    ) -> Result<(), RdfDiagnostic> {
        if self.export.is_none() {
            return Err(error);
        }
        self.record_export_omission(payload, kind, Some(error))
    }

    pub(super) fn record_export_omission(
        &mut self,
        payload: BlobPayload<'_>,
        kind: GtsBlobOmissionKind,
        cause: Option<RdfDiagnostic>,
    ) -> Result<(), RdfDiagnostic> {
        let source = self.export_source()?;
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let omission = GtsBlobOmission {
            kind,
            digest: Some(memory.string(payload.digest)?),
            digest_computed: payload.bytes.is_some_and(|bytes| {
                payload.codecs.is_empty() && purrdf_gts::wire::digest_str(bytes) == payload.digest
            }),
            encoded_bytes: payload.encoded_len,
            source,
            limits: self.limits,
            usage: self.usage,
            cause,
        };
        let omissions = self.export.as_mut().ok_or_else(|| {
            RdfDiagnostic::error(
                "rdf-ir-gts-import-internal",
                "omission recorded outside inline export",
            )
        })?;
        memory.push(omissions, omission)?;
        Ok(())
    }

    fn finish_export(
        mut self,
        dataset: Arc<crate::RdfDataset>,
    ) -> Result<GtsInlineExport, RdfDiagnostic> {
        let mut omissions = self.export.take().ok_or_else(|| {
            RdfDiagnostic::error(
                "rdf-ir-gts-import-internal",
                "inline export state is missing",
            )
        })?;
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for refusal in std::mem::take(&mut self.refused) {
            let source = refusal.source.ok_or_else(|| {
                RdfDiagnostic::error(
                    "rdf-ir-gts-blob-provenance",
                    "omitted reader payload has no source",
                )
            })?;
            let kind = match refusal.kind {
                BlobRefusalKind::ByteBudget => GtsBlobOmissionKind::ByteBudget,
                BlobRefusalKind::UnavailableCapability => {
                    GtsBlobOmissionKind::UnavailableCapability
                }
            };
            let failure = GtsBlobRetentionRefusal {
                digest: refusal.blob.digest.clone(),
                digest_computed: refusal.blob.digest_computed,
                encoded_bytes: refusal.blob.encoded_len,
                limits: self.limits,
                available_decoded_bytes: refusal.available_decoded_bytes,
                usage: refusal.usage,
                segment_index: source.segment_index,
                frame_index: source.frame_index,
                cause: refusal.cause,
            };
            let cause = RdfDiagnostic::error(
                "rdf-ir-gts-blob-omitted",
                "inline payload admission was refused",
            )
            .with_native_cause(
                purrdf_gts::model::retain_native_cause(failure)?,
                &mut memory,
            )?;
            memory.push(
                &mut omissions,
                GtsBlobOmission {
                    kind,
                    digest: refusal.blob.digest,
                    digest_computed: refusal.blob.digest_computed,
                    encoded_bytes: refusal.blob.encoded_len,
                    source,
                    limits: self.limits,
                    usage: refusal.usage,
                    cause: Some(cause),
                },
            )?;
        }
        omissions.retain(|omission| {
            let Some(digest) = omission.digest.as_deref() else {
                return true;
            };
            if omission.kind == GtsBlobOmissionKind::MissingInlinePayload {
                return self.selected.get_by(|key| digest.cmp(key)).is_none()
                    && self.inlined.get_by(|key| digest.cmp(key)).is_none();
            }
            !(omission.digest_computed && self.selected.get_by(|key| digest.cmp(key)).is_some())
        });
        // Deduplicate only proved identities. Unproved claims remain located
        // occurrences, never aliases of another occurrence's verified content.
        omissions.sort_by(|a, b| {
            a.digest
                .cmp(&b.digest)
                .then(a.digest_computed.cmp(&b.digest_computed))
                .then(b.source.segment_index.cmp(&a.source.segment_index))
                .then(b.source.frame_index.cmp(&a.source.frame_index))
        });
        omissions.dedup_by(|a, b| a.digest_computed && b.digest_computed && a.digest == b.digest);
        omissions.sort_by(|a, b| {
            a.source
                .segment_index
                .cmp(&b.source.segment_index)
                .then(a.source.frame_index.cmp(&b.source.frame_index))
                .then(a.digest.cmp(&b.digest))
                .then(a.kind.cmp(&b.kind))
        });
        let blobs = self.take_export_blobs()?;
        Ok(GtsInlineExport {
            dataset,
            blobs,
            omissions,
            usage: self.usage,
        })
    }

    pub(super) fn take_export_blobs(&mut self) -> Result<Vec<GtsImportedBlob>, RdfDiagnostic> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut blobs = Vec::new();
        memory.reserve(&mut blobs, self.selected.len())?;
        while let Some((_, blob)) = self.selected.pop_first() {
            memory.push(&mut blobs, blob)?;
        }
        Ok(blobs)
    }
}
