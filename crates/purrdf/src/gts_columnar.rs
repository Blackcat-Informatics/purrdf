// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit eager GTS inline admission shared by relational and Parquet exports.

use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_rdf::{ContentStore, LossLedger, RdfDataset, RdfLocation};
use purrdf_rdf::{GtsBlobLimits, GtsBlobOmission, GtsBlobUsage, RdfDiagnostic};
use std::sync::Arc;

/// Closed native failures; omitted payloads are reported separately.
#[derive(Debug)]
pub enum GtsColumnarError {
    /// Original strict GTS or RDF import failure.
    Import(RdfDiagnostic),
    /// Original native canonical projection or encoding failure.
    Columnar(purrdf_columnar::ColumnarWriteError<std::convert::Infallible>),
    /// Original storage admission failure.
    Storage(StorageError),
}

impl std::fmt::Display for GtsColumnarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Import(error) => error.fmt(f),
            Self::Columnar(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
        }
    }
}
purrdf_lex::error_source!(GtsColumnarError => {
    Self::Import(cause) => cause,
    Self::Columnar(cause) => cause,
    Self::Storage(cause) => cause,
});
impl purrdf_lex::error::StructuredError for GtsColumnarError {
    fn visit<'a>(&'a self, visitor: &mut dyn purrdf_lex::error::ErrorVisitor<'a>) {
        use purrdf_lex::error::ErrorValue;
        let (code, cause): (_, &dyn purrdf_lex::error::StructuredError) = match self {
            Self::Import(cause) => ("gts-columnar-import", cause),
            Self::Columnar(cause) => ("gts-columnar-projection", cause),
            Self::Storage(cause) => ("gts-columnar-storage", cause),
        };
        visitor.variant(code);
        visitor.value(Some("cause"), ErrorValue::Cause(cause));
    }
}
purrdf_lex::variant_from!(GtsColumnarError {
    Import(RdfDiagnostic), Columnar(purrdf_columnar::ColumnarWriteError<std::convert::Infallible>), Storage(StorageError)
});

/// Immutable actual losses and their original typed blob evidence.
#[derive(Debug)]
pub struct GtsBlobExportReport {
    losses: LossLedger,
    omissions: Vec<GtsBlobOmission>,
    usage: GtsBlobUsage,
}

impl GtsBlobExportReport {
    /// Preserve actual downstream writer losses in this admission report.
    pub fn append_writer_losses(&mut self, losses: LossLedger) -> Result<(), StorageError> {
        self.losses
            .append_with_memory(losses, &mut Memory::new(&mut Resident))
    }
    /// Actual conversion losses, retaining every original pair and location.
    #[must_use]
    pub const fn losses(&self) -> &LossLedger {
        &self.losses
    }
    /// Final omitted occurrences or references and their owned native causes.
    #[must_use]
    pub fn omissions(&self) -> &[GtsBlobOmission] {
        &self.omissions
    }
    /// Logical admission charges and peaks, excluding other retained owners.
    #[must_use]
    pub const fn usage(&self) -> GtsBlobUsage {
        self.usage
    }
}

/// Eager preparation; selected payloads remain resident until projection ends.
#[derive(Debug)]
pub struct GtsColumnarSource {
    dataset: Arc<RdfDataset>,
    blobs: ContentStore,
    report: GtsBlobExportReport,
}

impl GtsColumnarSource {
    /// Produce canonical relational rows and the always-present admission report.
    pub fn project(
        self,
    ) -> Result<(purrdf_columnar::ColumnarProjection, GtsBlobExportReport), GtsColumnarError> {
        let rows = purrdf_columnar::project(self.dataset.as_ref(), &self.blobs)?;
        Ok((rows, self.report))
    }

    /// Produce canonical file bytes and preserve actual writer losses unchanged.
    pub fn write(
        mut self,
        compression: purrdf_columnar::Compression,
    ) -> Result<(purrdf_columnar::ParquetFiles, GtsBlobExportReport), GtsColumnarError> {
        let output = purrdf_columnar::write(self.dataset.as_ref(), &self.blobs, compression)?;
        self.report.append_writer_losses(output.losses)?;
        Ok((output.files, self.report))
    }
}

/// Admit inline blobs with caller policy; never performs a discovery fold.
///
/// This preserves the strict event import's RDF behavior. That native bundle
/// currently has no loss ledger; its diagnostics are hard errors. Only actual
/// blob omissions and actual downstream writer losses enter this report.
pub fn prepare_gts_columnar(
    bytes: &[u8],
    limits: GtsBlobLimits,
) -> Result<GtsColumnarSource, GtsColumnarError> {
    let (dataset, imported, omissions, usage) =
        purrdf_rdf::import_gts_inline_for_export(bytes, limits)?.into_parts();
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut blobs = ContentStore::new();
    for blob in imported {
        // GTS proved BLAKE3 over these bytes; the native table derives its SHA-256.
        // This interim eager API still owns the admitted payload set.
        blobs.insert(memory.collect(blob.bytes.iter().copied())?);
    }
    let report = blob_report(omissions, usage)?;
    Ok(GtsColumnarSource {
        dataset,
        blobs,
        report,
    })
}

/// Scoped RDF data, verified staging descriptors and the original loss report.
#[derive(Debug)]
pub struct GtsStagedColumnarSource {
    /// Authoritative native RDF data, without retained inline payloads.
    pub dataset: Arc<RdfDataset>,
    /// Selected payload handles and original provenance.
    pub blobs: Vec<purrdf_rdf::GtsStagedBlob>,
    /// Actual admission omissions and logical charges.
    pub report: GtsBlobExportReport,
}

/// Prepare GTS exports using caller-owned provisional blob staging.
pub fn prepare_gts_columnar_to(
    bytes: &[u8],
    limits: GtsBlobLimits,
    sink: &mut dyn purrdf_rdf::GtsBlobStager,
) -> Result<GtsStagedColumnarSource, GtsColumnarError> {
    let (dataset, mut blobs, omissions, usage) =
        purrdf_rdf::import_gts_inline_to(bytes, limits, sink)?.into_parts();
    blobs.sort_unstable_by_key(|blob| blob.bytes.sha256);
    Ok(GtsStagedColumnarSource {
        dataset,
        blobs,
        report: blob_report(omissions, usage)?,
    })
}

fn blob_report(
    omissions: Vec<GtsBlobOmission>,
    usage: GtsBlobUsage,
) -> Result<GtsBlobExportReport, GtsColumnarError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let profile = purrdf_rdf::loss::gts_to_columnar_loss_ledger();
    let mut losses = LossLedger::default();
    for omission in &omissions {
        let mut entry = profile
            .entries()
            .iter()
            .find(|entry| entry.code == omission.kind().code())
            .cloned()
            .ok_or_else(|| {
                RdfDiagnostic::error(
                    "gts-columnar-loss-profile",
                    "blob omission has no registered loss definition",
                )
            })?;
        entry.location = Some(Box::new(
            RdfLocation::logical("gts:blob")
                .with_gts_segment(omission.source().segment_index)
                .with_gts_frame(omission.source().frame_index),
        ));
        losses.record_with_memory(entry, &mut memory)?;
    }
    Ok(GtsBlobExportReport {
        losses,
        omissions,
        usage,
    })
}
