// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;

use super::{RangeReason, Stop};
use crate::v2::fields::{Fields, push};
use crate::v2::index::{
    DirectoryRange, check_statistics, read_content_digest, read_graph_entry, read_pack_member,
    read_type_run,
};
use crate::v2::{DIGEST_LENGTH, FrameType};

type Digest = [u8; DIGEST_LENGTH];

#[derive(Clone, Copy)]
pub(super) struct CoveredRun {
    pub(super) kind: FrameType,
    pub(super) count: u64,
}

#[derive(Clone, Copy)]
pub(super) struct GraphRange {
    pub(super) kind: FrameType,
    pub(super) first_frame: u64,
    pub(super) first_row: u64,
    pub(super) row_count: u64,
}

pub(super) struct GraphEntry {
    pub(super) graph: Option<u64>,
    pub(super) ranges: Vec<GraphRange>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PackClaim {
    pub(super) digest: Digest,
    pub(super) position: u64,
    pub(super) member: u64,
}

pub(super) struct Claims {
    pub(super) runs: Vec<CoveredRun>,
    pub(super) graphs: Vec<GraphEntry>,
    pub(super) members: Vec<PackClaim>,
}

impl Claims {
    /// Read grammar in its wire order. Intrinsic claim checks run only after
    /// the payload has passed grammar and the answer records the frame count.
    pub(super) fn read(payload: &[u8], registry: NonZeroU64) -> Result<Self, Stop> {
        let mut fields = Fields::new(payload);
        fields.record(6)?;
        let run_count = fields.array()?;
        let mut runs = Vec::new();
        for _ in 0..run_count {
            let (kind, count) =
                read_type_run(&mut fields, registry).map_err(Stop::content_error)?;
            let kind = Self::frame_type(kind)?;
            push(&mut runs, CoveredRun { kind, count })?;
        }
        let graph_count = fields.array()?;
        let mut graphs = Vec::new();
        for _ in 0..graph_count {
            let (graph, range_count) =
                read_graph_entry(&mut fields, registry).map_err(Stop::content_error)?;
            let mut ranges = Vec::new();
            for _ in 0..range_count {
                let range =
                    DirectoryRange::read(&mut fields, registry).map_err(Stop::content_error)?;
                push(
                    &mut ranges,
                    GraphRange {
                        kind: Self::frame_type(range.kind)?,
                        first_frame: range.frame,
                        first_row: range.row,
                        row_count: range.count as u64,
                    },
                )?;
            }
            push(&mut graphs, GraphEntry { graph, ranges })?;
        }
        check_statistics(&mut fields, None)?;
        fields.fixed_bytes::<DIGEST_LENGTH>()?;
        read_content_digest(&mut fields, registry).map_err(Stop::content_error)?;
        let member_count = fields.array()?;
        let mut members = Vec::new();
        for _ in 0..member_count {
            let (digest, position, member) = read_pack_member(&mut fields)?;
            push(
                &mut members,
                PackClaim {
                    digest,
                    position,
                    member,
                },
            )?;
        }
        fields.finish()?;
        Ok(Self {
            runs,
            graphs,
            members,
        })
    }

    fn frame_type(code: u64) -> Result<FrameType, Stop> {
        FrameType::from_code(code).ok_or(Stop::Unknown(RangeReason::Payload))
    }

    pub(super) fn verify(&self, frame_count: u64) -> Result<(), Stop> {
        let mut total = 0_u128;
        let mut previous = None;
        for run in &self.runs {
            if run.count == 0 || previous == Some(run.kind) {
                return Err(Stop::Unknown(RangeReason::IndexClaims));
            }
            previous = Some(run.kind);
            total += u128::from(run.count);
        }
        if total != u128::from(frame_count)
            || !self
                .graphs
                .windows(2)
                .all(|pair| pair[0].graph < pair[1].graph)
            || !self.members.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(Stop::Unknown(RangeReason::IndexClaims));
        }
        for graph in &self.graphs {
            if !graph.ranges.iter().all(|range| {
                range.row_count != 0
                    && matches!(
                        range.kind,
                        FrameType::Quads
                            | FrameType::ReifierBindings
                            | FrameType::Annotations
                            | FrameType::EmptyGraphs
                    )
            }) || !graph.ranges.windows(2).all(|pair| {
                (pair[0].first_frame, pair[0].first_row) < (pair[1].first_frame, pair[1].first_row)
            }) {
                return Err(Stop::Unknown(RangeReason::IndexClaims));
            }
        }
        Ok(())
    }
}
