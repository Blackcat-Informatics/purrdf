// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LPG mapping progress and bounded transactional artifact streaming.

use std::collections::BTreeSet;
use std::io;

use purrdf_core::LossLedger;
use serde::{Deserialize, Serialize};

use super::super::package::validate_artifact_path;
use super::super::{ProjectionArtifactSink, ProjectionError, ProjectionLimits};
use super::carrier_util::LpgTextWriter;
use super::model::LpgGraph;

const PROGRESS_RECORD_STRIDE: usize = 4_096;
const PROGRESS_BYTE_STRIDE: usize = 64 * 1_024;
const SINK_CHUNK_BYTES: usize = 16 * 1_024;

/// Stable phase for one RDF-to-LPG mapping/package operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LpgProgressPhase {
    /// Reading and selecting RDF 1.2 records.
    Scanning,
    /// Constructing and validating the canonical LPG model.
    Building,
    /// Emitting bounded artifact chunks.
    Writing,
    /// The complete package is ready to commit.
    Complete,
    /// The operation failed and its sink transaction was aborted.
    Aborted,
}

impl LpgProgressPhase {
    /// Stable lowercase machine label for this phase.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scanning => "scanning",
            Self::Building => "building",
            Self::Writing => "writing",
            Self::Complete => "complete",
            Self::Aborted => "aborted",
        }
    }
}

/// Exact counters from one RDF-to-LPG mapping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgProjectionReport {
    /// Logical named-graph declarations/statements/reifiers/annotations scanned.
    pub input_records: usize,
    /// Records retained in the canonical LPG model, including nested node rows.
    pub model_records: usize,
    /// Canonical LPG nodes.
    pub nodes: usize,
    /// Canonical LPG edges.
    pub edges: usize,
}

/// Result of direct RDF-to-artifact-sink LPG projection.
#[derive(Debug, Clone)]
pub struct LpgStreamProjection {
    /// Always-computed RDF-to-LPG semantic-lowering ledger.
    pub loss_ledger: LossLedger,
    /// Exact scanned/model/node/edge counters.
    pub report: LpgProjectionReport,
}

pub(super) fn graph_report(graph: &LpgGraph) -> Result<LpgProjectionReport, ProjectionError> {
    let nested = graph.nodes.iter().try_fold(0usize, |count, node| {
        count
            .checked_add(node.labels.len())
            .and_then(|value| value.checked_add(node.properties.len()))
            .ok_or_else(|| ProjectionError::limit("LPG report record count overflow"))
    })?;
    let model_records = [
        graph.nodes.len(),
        graph.edges.len(),
        graph.reifiers.len(),
        graph.annotations.len(),
        graph.named_graphs.len(),
        nested,
    ]
    .into_iter()
    .try_fold(0usize, |count, amount| {
        count
            .checked_add(amount)
            .ok_or_else(|| ProjectionError::limit("LPG report record count overflow"))
    })?;
    Ok(LpgProjectionReport {
        input_records: 0,
        model_records,
        nodes: graph.nodes.len(),
        edges: graph.edges.len(),
    })
}

/// Monotonic progress snapshot for mapping and artifact emission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LpgProgress {
    /// Current operation phase.
    pub phase: LpgProgressPhase,
    /// Mapping counters reached so far.
    pub report: LpgProjectionReport,
    /// Fully finished artifacts.
    pub artifacts: usize,
    /// Body bytes accepted by the transactional sink.
    pub bytes: usize,
    /// Active artifact path during writing.
    pub path: Option<String>,
}

/// Fallible observer for deterministic LPG progress snapshots.
pub trait LpgProgressObserver {
    /// Observe one monotonic snapshot.
    ///
    /// # Errors
    ///
    /// Returning an error fails the operation and aborts any active sink package.
    fn observe(&mut self, progress: &LpgProgress) -> Result<(), ProjectionError>;
}

impl<F> LpgProgressObserver for F
where
    F: FnMut(&LpgProgress) -> Result<(), ProjectionError>,
{
    fn observe(&mut self, progress: &LpgProgress) -> Result<(), ProjectionError> {
        self(progress)
    }
}

#[derive(Debug, Default)]
pub(super) struct IgnoreProgress;

impl LpgProgressObserver for IgnoreProgress {
    fn observe(&mut self, _progress: &LpgProgress) -> Result<(), ProjectionError> {
        Ok(())
    }
}

pub(super) struct MappingProgress<'a, O> {
    observer: &'a mut O,
    report: LpgProjectionReport,
}

impl<'a, O: LpgProgressObserver> MappingProgress<'a, O> {
    pub(super) fn new(observer: &'a mut O) -> Result<Self, ProjectionError> {
        let mut tracker = Self {
            observer,
            report: LpgProjectionReport::default(),
        };
        tracker.emit(LpgProgressPhase::Scanning)?;
        Ok(tracker)
    }

    pub(super) fn scanned(&mut self) -> Result<(), ProjectionError> {
        self.report.input_records = checked_increment(self.report.input_records, "input record")?;
        if self.report.input_records == 1
            || self
                .report
                .input_records
                .is_multiple_of(PROGRESS_RECORD_STRIDE)
        {
            self.emit(LpgProgressPhase::Scanning)?;
        }
        Ok(())
    }

    pub(super) fn node(&mut self) -> Result<(), ProjectionError> {
        self.report.nodes = checked_increment(self.report.nodes, "node")?;
        if self.report.nodes == 1 || self.report.nodes.is_multiple_of(PROGRESS_RECORD_STRIDE) {
            self.emit(LpgProgressPhase::Building)?;
        }
        Ok(())
    }

    pub(super) fn edge(&mut self) -> Result<(), ProjectionError> {
        self.report.edges = checked_increment(self.report.edges, "edge")?;
        if self.report.edges == 1 || self.report.edges.is_multiple_of(PROGRESS_RECORD_STRIDE) {
            self.emit(LpgProgressPhase::Building)?;
        }
        Ok(())
    }

    pub(super) const fn set_model_records(&mut self, model_records: usize) {
        self.report.model_records = model_records;
    }

    pub(super) fn finish(&mut self) -> Result<LpgProjectionReport, ProjectionError> {
        self.emit(LpgProgressPhase::Building)?;
        Ok(self.report)
    }

    pub(super) fn abort(&mut self) {
        let _ = self.emit(LpgProgressPhase::Aborted);
    }

    fn emit(&mut self, phase: LpgProgressPhase) -> Result<(), ProjectionError> {
        self.observer.observe(&LpgProgress {
            phase,
            report: self.report,
            artifacts: 0,
            bytes: 0,
            path: None,
        })
    }
}

fn checked_increment(value: usize, description: &str) -> Result<usize, ProjectionError> {
    value
        .checked_add(1)
        .ok_or_else(|| ProjectionError::limit(format!("LPG {description} count overflow")))
}

pub(super) struct LpgSinkSession<'a, S: ProjectionArtifactSink, O> {
    sink: &'a mut S,
    observer: &'a mut O,
    limits: ProjectionLimits,
    report: LpgProjectionReport,
    paths: BTreeSet<String>,
    artifacts: usize,
    total_bytes: usize,
    archive_bytes: usize,
    active: bool,
}

impl<'a, S, O> LpgSinkSession<'a, S, O>
where
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    pub(super) fn new(
        sink: &'a mut S,
        observer: &'a mut O,
        limits: ProjectionLimits,
        report: LpgProjectionReport,
    ) -> Result<Self, ProjectionError> {
        sink.begin_package()?;
        let mut session = Self {
            sink,
            observer,
            limits,
            report,
            paths: BTreeSet::new(),
            artifacts: 0,
            total_bytes: 0,
            archive_bytes: 1_024,
            active: true,
        };
        if let Err(error) = session.emit(LpgProgressPhase::Writing, None) {
            session.abort();
            return Err(error);
        }
        Ok(session)
    }

    pub(super) fn write_artifact<F>(&mut self, path: &str, encode: F) -> Result<(), ProjectionError>
    where
        F: FnOnce(&mut LpgArtifactWriter<'_, S, O>) -> Result<(), ProjectionError>,
    {
        let result = self.write_artifact_inner(path, encode);
        if result.is_err() {
            self.abort();
        }
        result
    }

    fn write_artifact_inner<F>(&mut self, path: &str, encode: F) -> Result<(), ProjectionError>
    where
        F: FnOnce(&mut LpgArtifactWriter<'_, S, O>) -> Result<(), ProjectionError>,
    {
        validate_artifact_path(path)?;
        if !self.paths.insert(path.to_owned()) {
            return Err(ProjectionError::package("duplicate artifact path").at_path(path));
        }
        if self.artifacts >= self.limits.max_artifacts() {
            return Err(ProjectionError::limit(format!(
                "package exceeds the {}-artifact limit",
                self.limits.max_artifacts()
            ))
            .at_path(path));
        }
        self.sink.begin_artifact(path)?;
        let mut writer = LpgArtifactWriter {
            sink: self.sink,
            observer: self.observer,
            limits: self.limits,
            report: self.report,
            path,
            finished_artifacts: self.artifacts,
            artifact_bytes: 0,
            total_before: self.total_bytes,
            next_progress_bytes: PROGRESS_BYTE_STRIDE,
            buffer: [0; SINK_CHUNK_BYTES],
            buffered: 0,
            failure: None,
        };
        let encode_result = encode(&mut writer);
        if let Some(error) = writer.failure.take() {
            return Err(error);
        }
        encode_result?;
        writer.flush_buffer()?;
        let artifact_bytes = writer.artifact_bytes;
        self.total_bytes = self
            .total_bytes
            .checked_add(artifact_bytes)
            .ok_or_else(|| ProjectionError::limit("package byte count overflow"))?;
        let padded = artifact_bytes
            .checked_add(511)
            .map(|value| value / 512 * 512)
            .ok_or_else(|| ProjectionError::limit("archive padding overflow").at_path(path))?;
        self.archive_bytes = self
            .archive_bytes
            .checked_add(512)
            .and_then(|value| value.checked_add(padded))
            .ok_or_else(|| ProjectionError::limit("archive byte count overflow").at_path(path))?;
        if self.archive_bytes > self.limits.max_archive_bytes() {
            return Err(ProjectionError::limit(format!(
                "archive exceeds the {}-byte limit",
                self.limits.max_archive_bytes()
            ))
            .at_path(path));
        }
        self.sink.finish_artifact()?;
        self.artifacts += 1;
        self.emit(LpgProgressPhase::Writing, Some(path))
    }

    pub(super) fn commit(mut self) -> Result<(), ProjectionError> {
        if let Err(error) = self.emit(LpgProgressPhase::Complete, None) {
            self.abort();
            return Err(error);
        }
        if let Err(error) = self.sink.commit_package() {
            self.abort();
            return Err(error);
        }
        self.active = false;
        Ok(())
    }

    fn emit(&mut self, phase: LpgProgressPhase, path: Option<&str>) -> Result<(), ProjectionError> {
        self.observer.observe(&LpgProgress {
            phase,
            report: self.report,
            artifacts: self.artifacts,
            bytes: self.total_bytes,
            path: path.map(str::to_owned),
        })
    }

    fn abort(&mut self) {
        if self.active {
            self.sink.abort_package();
            self.active = false;
            let _ = self.emit(LpgProgressPhase::Aborted, None);
        }
    }
}

impl<S: ProjectionArtifactSink, O> Drop for LpgSinkSession<'_, S, O> {
    fn drop(&mut self) {
        if self.active {
            self.sink.abort_package();
        }
    }
}

pub(super) struct LpgArtifactWriter<'a, S, O> {
    sink: &'a mut S,
    observer: &'a mut O,
    limits: ProjectionLimits,
    report: LpgProjectionReport,
    path: &'a str,
    finished_artifacts: usize,
    artifact_bytes: usize,
    total_before: usize,
    next_progress_bytes: usize,
    buffer: [u8; SINK_CHUNK_BYTES],
    buffered: usize,
    failure: Option<ProjectionError>,
}

impl<S, O> LpgArtifactWriter<'_, S, O>
where
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    pub(super) fn push(&mut self, value: &str) -> Result<(), ProjectionError> {
        self.write_bytes(value.as_bytes())
    }

    pub(super) fn push_hex(&mut self, value: &[u8]) -> Result<(), ProjectionError> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut chunk = [0u8; 8_192];
        for source in value.chunks(chunk.len() / 2) {
            for (index, byte) in source.iter().copied().enumerate() {
                chunk[index * 2] = HEX[usize::from(byte >> 4)];
                chunk[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
            }
            self.write_bytes(&chunk[..source.len() * 2])?;
        }
        Ok(())
    }

    pub(super) fn write_bytes(&mut self, mut chunk: &[u8]) -> Result<(), ProjectionError> {
        while !chunk.is_empty() {
            let chunk_bytes = chunk.len().min(SINK_CHUNK_BYTES - self.buffered);
            let artifact_bytes = self
                .artifact_bytes
                .checked_add(chunk_bytes)
                .ok_or_else(|| ProjectionError::limit("artifact byte count overflow"))?;
            if artifact_bytes > self.limits.max_artifact_bytes() {
                return Err(ProjectionError::limit(format!(
                    "artifact exceeds the {}-byte limit",
                    self.limits.max_artifact_bytes()
                ))
                .at_path(self.path));
            }
            let total_bytes = self
                .total_before
                .checked_add(artifact_bytes)
                .ok_or_else(|| ProjectionError::limit("package byte count overflow"))?;
            if total_bytes > self.limits.max_total_bytes() {
                return Err(ProjectionError::limit(format!(
                    "package exceeds the {}-byte body limit",
                    self.limits.max_total_bytes()
                ))
                .at_path(self.path));
            }

            let buffered = self.buffered + chunk_bytes;
            self.buffer[self.buffered..buffered].copy_from_slice(&chunk[..chunk_bytes]);
            self.buffered = buffered;
            self.artifact_bytes = artifact_bytes;
            chunk = &chunk[chunk_bytes..];
            if self.buffered == SINK_CHUNK_BYTES {
                self.flush_buffer()?;
            }
        }
        Ok(())
    }

    fn flush_buffer(&mut self) -> Result<(), ProjectionError> {
        if self.buffered == 0 {
            return Ok(());
        }
        self.sink.write_chunk(&self.buffer[..self.buffered])?;
        self.buffered = 0;
        if self.artifact_bytes >= self.next_progress_bytes {
            let total_bytes = self
                .total_before
                .checked_add(self.artifact_bytes)
                .ok_or_else(|| ProjectionError::limit("package byte count overflow"))?;
            self.observer.observe(&LpgProgress {
                phase: LpgProgressPhase::Writing,
                report: self.report,
                artifacts: self.finished_artifacts,
                bytes: total_bytes,
                path: Some(self.path.to_owned()),
            })?;
            self.next_progress_bytes = self
                .artifact_bytes
                .checked_add(PROGRESS_BYTE_STRIDE)
                .ok_or_else(|| ProjectionError::limit("progress byte count overflow"))?;
        }
        Ok(())
    }
}

impl<S, O> LpgTextWriter for LpgArtifactWriter<'_, S, O>
where
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    fn push(&mut self, value: &str) -> Result<(), ProjectionError> {
        Self::push(self, value)
    }

    fn push_hex(&mut self, value: &[u8]) -> Result<(), ProjectionError> {
        Self::push_hex(self, value)
    }
}

impl<S, O> io::Write for LpgArtifactWriter<'_, S, O>
where
    S: ProjectionArtifactSink,
    O: LpgProgressObserver,
{
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self.write_bytes(buffer) {
            Ok(()) => Ok(buffer.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other(
                    "projection artifact sink rejected a chunk",
                ))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.flush_buffer() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other(
                    "projection artifact sink rejected a chunk",
                ))
            }
        }
    }
}
