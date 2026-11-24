// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;

use crate::FastMap;
use crate::v2::references::StateCatalog;
use crate::v2::{
    BlobEntry, DIGEST_LENGTH, ExternalBlob, FormedBudget, Item, PayloadBudget, RdfFold,
    ReadCapacity, ReadError, ReadFailure, ReferenceFold, ResyncReach, SignatureInputs,
    StateLocation, VerifiedBlob,
};

use super::{GraphReader, HeldItems, RdfGraph, RebuildBudget, decode_opener};

/// Content folded from the run of segments named by a commit.
#[derive(Debug)]
#[non_exhaustive]
pub struct StateFold<'a> {
    /// The run's RDF fold through the naming commit.
    pub rdf: RdfFold,
    /// Reference records resolved using the run through that commit.
    pub references: ReferenceFold,
    /// Verified inline bytes declared by the run.
    pub blobs: Vec<VerifiedBlob<'a>>,
    /// References to named blobs declared without inline bytes.
    pub external_blobs: Vec<ExternalBlob<'a>>,
    /// Folded blob declarations grouped by name.
    pub blob_set: Vec<BlobEntry<'a>>,
}

#[derive(Default)]
pub(super) struct StateSource<'a> {
    input: Option<&'a [u8]>,
    starts: FastMap<u64, ReadPoint>,
    pub(super) commits: StateCatalog,
}

impl fmt::Debug for StateSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateSource")
            .field("run_count", &self.starts.len())
            .finish_non_exhaustive()
    }
}

pub(super) struct ReadPoint {
    payload: PayloadBudget,
    formed: FormedBudget,
    rebuilt: RebuildBudget,
    items: u64,
    item_limit: u64,
    nesting_depth: Option<usize>,
    scan_offset: usize,
}

impl ReadPoint {
    pub(super) fn capture(reader: &GraphReader<'_, '_>, items: u64) -> Self {
        Self {
            payload: reader.payload.clone(),
            formed: reader.formed.clone(),
            rebuilt: reader.rebuilt,
            items,
            item_limit: reader.item_limit,
            nesting_depth: reader.item_nesting_depth,
            scan_offset: reader.scan.next_offset(),
        }
    }

    pub(super) fn reader<'a, 'keys>(
        &self,
        input: &'a [u8],
        signatures: &'keys SignatureInputs<'keys>,
        first_segment: u64,
    ) -> GraphReader<'a, 'keys> {
        let mut reader = GraphReader::new(
            input,
            ReadCapacity::default(),
            None,
            self.nesting_depth,
            signatures,
        );
        reader.graph.state_source = StateSource::default();
        reader.payload.clone_from(&self.payload);
        reader.formed.clone_from(&self.formed);
        reader.rebuilt = self.rebuilt;
        reader.items = self.items;
        reader.item_limit = self.item_limit;
        reader.first_segment = first_segment;
        reader.scan.restore_offset(self.scan_offset);
        reader
    }
}

impl<'a> StateSource<'a> {
    pub(super) fn new(input: &'a [u8]) -> Self {
        Self {
            input: Some(input),
            ..Self::default()
        }
    }

    pub(super) const fn input(&self) -> Option<&'a [u8]> {
        self.input
    }

    fn replay(
        &self,
        location: StateLocation,
        identifier: &[u8; DIGEST_LENGTH],
    ) -> Result<StateFold<'a>, ReadError> {
        let offset = location.start;
        let missing = || ReadError {
            offset,
            failure: ReadFailure::MissingContent,
        };
        let input = self.input.ok_or_else(missing)?;
        let prefix = input.get(..location.end).ok_or_else(missing)?;
        let start = self
            .starts
            .get(&location.first_segment)
            .ok_or_else(missing)?;
        let inputs = SignatureInputs::default();
        let mut reader = start.reader(prefix, &inputs, location.first_segment);
        reader.state_replay = true;
        if location.healed_commit {
            let suffix = &input[location.end..];
            let witness = match start.nesting_depth {
                Some(depth) => Item::first_with_depth(suffix, depth),
                None => Item::first(suffix),
            }
            .map_err(|error| ReadError {
                offset: location.end,
                failure: ReadFailure::Framing(error),
            })?;
            reader.state_commit_witness = Some(witness.ok_or_else(missing)?);
        }

        let opener = reader.next_item(offset)?.ok_or_else(missing)?;
        let end = offset + opener.bytes().len();
        let header = decode_opener(opener).map_err(|failure| ReadError { offset, failure })?;
        let segment =
            reader.open_segment(header, offset..end, None, ResyncReach::Walked, start.items)?;
        let graph = reader.read_segment(segment, offset, end, HeldItems::default())?;
        if graph.state_source.commits.named(identifier) != Some(location) {
            return Err(missing());
        }
        Ok(StateFold {
            rdf: graph.rdf,
            references: graph.references,
            blobs: graph.blobs,
            external_blobs: graph.external_blobs,
            blob_set: graph.blob_set,
        })
    }
}

impl GraphReader<'_, '_> {
    pub(super) fn remember_state_start(
        &mut self,
        ordinal: u64,
        offset: usize,
        items_at_opener: u64,
    ) -> Result<(), ReadError> {
        if self.graph.state_source.input.is_some() {
            let point = ReadPoint::capture(self, items_at_opener);
            self.graph
                .state_source
                .starts
                .try_reserve(1)
                .map_err(|error| ReadError {
                    offset,
                    failure: error.into(),
                })?;
            self.graph.state_source.starts.insert(ordinal, point);
        }
        Ok(())
    }
}

impl<'a> RdfGraph<'a> {
    /// Read the verified run named by a commit's effective identifier.
    ///
    /// Replays the run through the commit using the capacities left at the
    /// run's opener. Its blank nodes retain their original segment ordinals.
    /// A missing identifier or an unavailable state yields `None`.
    ///
    /// # Errors
    /// Returns a framing, fold, or storage failure during replay.
    pub fn read_state(
        &self,
        identifier: &[u8; DIGEST_LENGTH],
    ) -> Result<Option<StateFold<'a>>, ReadError> {
        self.state_source
            .commits
            .named(identifier)
            .map(|location| self.state_source.replay(location, identifier))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::canonical_labelling::{LabellingCapacity, Limit};

    use crate::v2::{ReadCapacity, ReadGraph, SegmentWriter, read};

    #[test]
    fn state_replay_does_not_charge_items_already_held_by_the_previous_run() {
        let mut first = SegmentWriter::new(Vec::new()).unwrap();
        first.add_blob(b"first run", None).unwrap();
        let mut first = first.into_inner();
        // Replace the final empty signature list with a malformed field.
        *first.last_mut().unwrap() = 0xf6;

        let mut second = SegmentWriter::new(Vec::new()).unwrap();
        let identifier = second.add_commit().unwrap();
        let bytes = [first.as_slice(), second.into_inner().as_slice()].concat();
        let capacity = ReadCapacity {
            items: 4,
            ..ReadCapacity::default()
        };
        let ReadGraph::Second(graph) = read(&bytes, capacity).unwrap() else {
            panic!("writer emitted generation 2");
        };
        assert!(graph.item_refusal.is_none());
        let report = graph
            .label_read(Limit::new(0), &mut LabellingCapacity::new(1, 0))
            .unwrap()
            .unwrap();
        assert_eq!(report.segments.len(), 2);
        let file = report.file.unwrap().content_digest;
        assert!(file.is_some());
        assert!(
            report
                .segments
                .into_iter()
                .all(|segment| segment.content_digest.unwrap() == file)
        );
        let state = graph.read_state(&identifier).unwrap().unwrap();
        assert_eq!(state.rdf.lines().count(), 0);
        assert_eq!(state.blob_set.len(), 0);
    }
}
