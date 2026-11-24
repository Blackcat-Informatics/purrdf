// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_core::canonical_labelling::{LabellingCapacity, Limit};
use purrdf_lex::allocation::try_boxed;

use crate::FastMap;
use crate::v2::canonical::PackRegion;
use crate::v2::parity::Recorder;
use crate::v2::{
    BlobEntry, CanonicalForm, ContentError, ContentLabellingRefusal, FileEnd, Profile, RdfGraph,
    ReadFailure, RebuildState,
};

use super::dictionary::Dictionary;
use super::index::IndexObservations;
use super::packs::{write_blobs, write_values};
use super::rdf::BlankLabel;
use super::references::ReferenceRecords;
use super::{Digest, SegmentWriteError, SegmentWriter};

/// The outcome and coverage of the source read used for compaction.
#[derive(Debug)]
#[non_exhaustive]
pub struct Compaction<W> {
    /// Whether a canonical file could be written.
    pub outcome: CompactionOutcome<W>,
    /// Items or regions omitted from the source read.
    pub left_out: u64,
    /// The source's end relative to valid commits.
    pub end: FileEnd,
}

/// The result of attempting to compact a whole-file fold.
#[derive(Debug)]
#[non_exhaustive]
pub enum CompactionOutcome<W> {
    /// The sink containing the canonical file.
    Written(W),
    /// The first condition that prevented compaction. The sink received no bytes.
    Stopped(CompactionStop),
}

/// A source fold cannot produce a canonical file.
#[derive(Debug)]
#[non_exhaustive]
pub enum CompactionStop {
    /// A terms frame exceeded the total formed-byte capacity.
    TotalFormed,
    /// A held RDF row or reference withholds the fold's content digest.
    OpaqueTerm,
    /// The fold's canonical labelling was refused.
    Labelling(ContentLabellingRefusal),
    /// A present blob had a declaration refused by total rebuild capacity.
    TotalRebuilt,
    /// No declaration of a present blob has all of its units.
    MissingChunk,
    /// A present blob's length disagrees with its verified bytes or units.
    BlobLength,
    /// A present blob's declared digest disagrees with its rebuilt bytes.
    BlobDigest,
}

/// Compact the fold of a whole-file read as GTS v2 canonical form 1.
///
/// The returned coverage applies to the source. Hidden statements and blob
/// declarations remain in the compacted content. The native zstd encoder
/// supplies stored bytes. Provide the source read's labelling limit and a
/// fresh weight account. The segments' own folds charge it before the file
/// fold. The optional profile code is declared in the output header.
///
/// # Errors
/// Returns a labelling, allocation, range or sink failure. A sink can
/// contain a partial segment when writing fails.
pub fn compact<W: Write>(
    graph: &RdfGraph<'_>,
    profile: Option<Profile>,
    limit: Limit,
    capacity: &mut LabellingCapacity,
    sink: W,
) -> Result<Compaction<W>, SegmentWriteError> {
    use CompactionOutcome::{Stopped, Written};

    let report = |outcome| Compaction {
        outcome,
        left_out: graph.left_out,
        end: graph.end,
    };
    if graph.frame_failures.iter().any(|item| {
        matches!(
            item.failure,
            ReadFailure::Content(ContentError::FormedCapacity(_))
        )
    }) {
        return Ok(report(Stopped(CompactionStop::TotalFormed)));
    }
    if graph.rdf.has_held_rows()
        || graph
            .reference_findings
            .iter()
            .any(|finding| finding.opaque_term)
    {
        return Ok(report(Stopped(CompactionStop::OpaqueTerm)));
    }
    if graph.labelling_size().is_none() {
        return Err(SegmentWriteError::IncompleteFold);
    }
    let source = match graph.label_for_compaction(limit, capacity) {
        Ok(source) => source,
        Err(error) => {
            let cause = try_boxed(error).map_err(io::Error::other)?;
            return Err(SegmentWriteError::SourceLabelling(cause));
        }
    };
    let labelled = match source {
        Ok(labelled) => labelled,
        Err(refusal) => return Ok(report(Stopped(CompactionStop::Labelling(refusal)))),
    };
    let blobs = match plan_blobs(graph)? {
        Ok(blobs) => blobs,
        Err(stop) => return Ok(report(Stopped(stop))),
    };
    let positions = labelled.blank_positions()?;
    let dictionary = Dictionary::new(graph, |segment, label| {
        positions
            .get(&(segment, label))
            .copied()
            .map(BlankLabel::position)
            .ok_or(SegmentWriteError::IncompleteFold)
    })?;
    let index = try_boxed(IndexObservations::default()).map_err(io::Error::other)?;
    let parity = try_boxed(Recorder::new_closing()?).map_err(io::Error::other)?;
    let mut writer = SegmentWriter::open(
        sink,
        Some(parity),
        Some(index),
        profile,
        Some(CanonicalForm::V1),
    )?;
    write_prefix(graph, dictionary, &blobs, &mut writer, |_, _| {})?;
    writer
        .finish_indexed(&labelled)
        .map(|sink| report(Written(sink)))
}

type BlobPlan<'a, 'file> = Vec<(&'a BlobEntry<'file>, Option<&'a [u8]>)>;

// Re-form only the ordinary frames, preserving the source blank labels
// and the actual opener as the wire anchor. The sink sees one flush per
// complete expected frame so it can compare the decoded content in place.
pub(in crate::v2) fn write_structure<W: Write>(
    graph: &RdfGraph<'_>,
    labelled: Option<&crate::v2::ContentLabelling>,
    opener: Digest,
    sink: W,
    region: impl Fn(&mut W, PackRegion),
) -> Result<W, SegmentWriteError> {
    let blobs = plan_blobs(graph)?.map_err(|_| SegmentWriteError::IncompleteFold)?;
    let dictionary = Dictionary::new(graph, |_, label| Ok(BlankLabel::Stored(label)))?;
    let index = try_boxed(IndexObservations::default()).map_err(io::Error::other)?;
    let mut writer = SegmentWriter::from_opener(sink, opener, index);
    write_prefix(graph, dictionary, &blobs, &mut writer, region)?;
    writer.append_index(
        labelled.map_or([0; 3], crate::v2::ContentLabelling::index_counts),
        labelled.and_then(|labels| labels.content_digest.as_ref()),
    )?;
    Ok(writer.into_inner())
}

fn write_prefix<W: Write>(
    graph: &RdfGraph<'_>,
    mut dictionary: Dictionary<'_>,
    blobs: &[(&BlobEntry<'_>, Option<&[u8]>)],
    writer: &mut SegmentWriter<W>,
    region: impl Fn(&mut W, PackRegion),
) -> Result<(), SegmentWriteError> {
    let references = ReferenceRecords::new(&graph.references, |term| dictionary.id(term))?;
    region(&mut writer.sink, PackRegion::Value);
    write_values(writer, &mut dictionary.values)?;
    region(&mut writer.sink, PackRegion::Blob);
    write_blobs(writer, blobs)?;
    dictionary.write_rdf(graph, writer)?;
    references.write(writer)?;
    writer.add_commit()?;
    Ok(())
}

fn plan_blobs<'a, 'file>(
    graph: &'a RdfGraph<'file>,
) -> io::Result<Result<BlobPlan<'a, 'file>, CompactionStop>> {
    let mut verified = FastMap::default();
    verified
        .try_reserve(graph.blobs.len())
        .map_err(io::Error::other)?;
    for blob in &graph.blobs {
        verified.insert(blob.name.digest, blob);
    }
    let mut plan = Vec::new();
    plan.try_reserve_exact(graph.blob_set.len())
        .map_err(io::Error::other)?;
    // The blob set is sorted by the BLAKE3 name, whose bytes come after the
    // same array, algorithm and byte-string heads for every blob.
    for entry in &graph.blob_set {
        let bytes = if entry.present {
            let blob = verified.get(&entry.name.digest);
            let Some(blob) = blob.filter(|blob| blob.name.length == entry.name.length) else {
                return Ok(Err(blob_stop(graph, entry, blob.is_some())));
            };
            Some(blob.bytes.as_ref())
        } else {
            None
        };
        plan.push((entry, bytes));
    }
    Ok(Ok(plan))
}

fn blob_stop(graph: &RdfGraph<'_>, entry: &BlobEntry<'_>, mut length: bool) -> CompactionStop {
    let mut missing = false;
    let mut digest = false;
    for recipe in graph
        .rebuilds
        .iter()
        .filter(|recipe| !recipe.is_value && recipe.name.digest == entry.name.digest)
    {
        match recipe.state {
            RebuildState::Refused { .. } => return CompactionStop::TotalRebuilt,
            RebuildState::MissingUnit => missing = true,
            RebuildState::BlobLength => {
                length |= recipe.has_all_units;
                missing |= !recipe.has_all_units;
            }
            RebuildState::BlobDigest => digest = true,
            RebuildState::Verified => {}
        }
    }
    if length {
        CompactionStop::BlobLength
    } else if missing && !digest {
        CompactionStop::MissingChunk
    } else {
        CompactionStop::BlobDigest
    }
}
