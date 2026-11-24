// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::collections::TryReserveError;

use super::{ContentError, Digest, Fields, FrameType, LAST_FRAME_TYPE, WireError, push};
use crate::v2::Row;
use crate::v2::rows::graph_name;
use crate::v2::terms::read_enum;
use crate::{FastMap, FastSet};

type PackLocation = (Digest, u64, u64);
type GraphName = Option<u64>;
type RunStart = (GraphName, usize);

#[derive(Debug, PartialEq, Eq)]
struct DirectoryRange {
    kind: u64,
    frame: u64,
    row: u64,
    count: u128,
}

#[derive(Debug, Default)]
struct GraphDirectory {
    ranges: FastMap<GraphName, Vec<DirectoryRange>>,
    last: [Option<RunStart>; 4],
}

impl GraphDirectory {
    fn record(&mut self, rows: &[Row], frame: u64) -> Result<(), TryReserveError> {
        for (position, row) in rows.iter().copied().enumerate() {
            let (slot, kind, graph) = match row {
                Row::Quad(_, graph) => (0, FrameType::Quads, graph),
                Row::ReifierBinding(_, graph) => (1, FrameType::ReifierBindings, graph),
                Row::Annotation(_, _, graph) => (2, FrameType::Annotations, graph),
                Row::EmptyGraph(graph) => (3, FrameType::EmptyGraphs, Some(graph)),
            };
            if let Some((preceding, index)) = self.last[slot]
                && preceding == graph
            {
                self.ranges
                    .get_mut(&graph)
                    .expect("a run belongs to its recorded graph")[index]
                    .count += 1;
                continue;
            }
            if !self.ranges.contains_key(&graph) {
                self.ranges.try_reserve(1)?;
            }
            let ranges = self.ranges.entry(graph).or_default();
            let index = ranges.len();
            push(
                ranges,
                DirectoryRange {
                    kind: kind as u64,
                    frame,
                    row: position as u64,
                    count: 1,
                },
            )?;
            self.last[slot] = Some((graph, index));
        }
        Ok(())
    }
}

pub(in crate::v2) struct IndexContent {
    directory: Option<GraphDirectory>,
    pack_members: Option<FastSet<PackLocation>>,
}

impl Default for IndexContent {
    fn default() -> Self {
        Self {
            directory: Some(GraphDirectory::default()),
            pack_members: Some(FastSet::default()),
        }
    }
}

impl IndexContent {
    pub(in crate::v2) fn opaque_frame(&mut self, kind: Option<FrameType>) {
        match kind {
            Some(
                FrameType::Quads
                | FrameType::ReifierBindings
                | FrameType::Annotations
                | FrameType::EmptyGraphs,
            ) => self.directory = None,
            Some(FrameType::Pack) => self.pack_members = None,
            _ => {}
        }
    }

    pub(in crate::v2) fn record_rows(
        &mut self,
        rows: &[Row],
        frame: u64,
    ) -> Result<(), TryReserveError> {
        if let Some(directory) = &mut self.directory {
            directory.record(rows, frame)?;
        }
        Ok(())
    }

    pub(in crate::v2) fn record_unit(
        &mut self,
        digest: Digest,
        frame: u64,
        member: usize,
    ) -> Result<(), TryReserveError> {
        if let Some(members) = &mut self.pack_members {
            members.try_reserve(1)?;
            members.insert((digest, frame, member as u64));
        }
        Ok(())
    }
}

pub(super) fn check_graph_directory(
    fields: &mut Fields<'_>,
    registry: NonZeroU64,
    history: Option<&IndexContent>,
) -> Result<Option<bool>, ContentError> {
    let directory = history.and_then(|history| history.directory.as_ref());
    let count = fields.array()?;
    let mut matches = directory.is_some_and(|directory| count == directory.ranges.len() as u64);
    let mut last_graph = None;
    for _ in 0..count {
        fields.record(2)?;
        let graph = graph_name(fields, registry)?;
        matches &= last_graph.is_none_or(|last| last < graph);
        last_graph = Some(graph);
        let expected = directory.and_then(|directory| directory.ranges.get(&graph));
        let count = fields.array()?;
        matches &= expected.is_some_and(|ranges| count == ranges.len() as u64);
        let mut runs = expected.into_iter().flatten();
        for _ in 0..count {
            fields.record(4)?;
            let observed = DirectoryRange {
                kind: read_enum(fields, LAST_FRAME_TYPE, registry)?,
                frame: fields.uint()?,
                row: fields.uint()?,
                count: u128::from(fields.uint()?),
            };
            matches &= runs.next() == Some(&observed);
        }
    }
    Ok(directory.map(|_| matches))
}

pub(super) fn check_pack_members(
    fields: &mut Fields<'_>,
    history: Option<&IndexContent>,
) -> Result<Option<bool>, WireError> {
    let members = history.and_then(|history| history.pack_members.as_ref());
    let count = fields.array()?;
    let mut matches = members.is_some_and(|members| count == members.len() as u64);
    let mut previous = None;
    for _ in 0..count {
        fields.record(3)?;
        let location = (*fields.fixed_bytes()?, fields.uint()?, fields.uint()?);
        matches &= previous.is_none_or(|previous| previous < location)
            && members.is_some_and(|members| members.contains(&location));
        previous = Some(location);
    }
    Ok(members.map(|_| matches))
}
