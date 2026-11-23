// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::ops::Range;
use std::collections::TryReserveError;

use super::{ParityCheck, ParityTable, SPAN_BYTES};
use crate::v2::{LinkState, PayloadRefusal, fields::push};

/// A parity diagnostic in the generation 2 graph projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParityFinding {
    /// Physical input offset of the parity frame or the start of the tail.
    pub offset: usize,
    /// The parity check that produced this finding.
    pub kind: ParityFindingKind,
}

/// A parity check finding before attempting to reconstruct any bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParityFindingKind {
    /// A recognized parity frame is damaged.
    Damaged,
    /// The payload could not be verified at the reader's capacity.
    Unchecked(PayloadRefusal),
    /// The parity frame uses an unrecognized geometry from a later registry.
    UnknownGeometry(u64),
    /// The frame placement or data bytes disagree with the covered span.
    Stale,
    /// The given number of data shards fail their digest check.
    DataMismatch(usize),
    /// The given number of parity shards fail their digest check.
    ParityMismatch(usize),
    /// The given number of failing data shards overlap only passing items.
    Unexplained(usize),
    /// The given number of logical bytes follow the final protected position.
    UnprotectedTail(usize),
}

struct Extent {
    physical: Range<usize>,
    logical_start: usize,
}

impl Extent {
    fn logical_end(&self) -> usize {
        self.logical_start + self.physical.len()
    }
}

struct Pending<'a> {
    offset: usize,
    check: ParityCheck<'a>,
    position: usize,
    last_item_start: usize,
}

pub(in crate::v2) struct ParityStream<'a> {
    extents: Vec<Extent>,
    first: Extent,
    opener_length: usize,
    opener_passes: Option<bool>,
    failing: Vec<Range<usize>>,
    pending: Vec<Pending<'a>>,
    length: usize,
    last_item_start: usize,
}

impl<'a> ParityStream<'a> {
    pub(in crate::v2) fn new(opener: Range<usize>) -> Self {
        let opener_length = opener.len();
        Self {
            extents: Vec::new(),
            length: opener_length,
            first: Extent {
                physical: opener,
                logical_start: 0,
            },
            opener_length,
            opener_passes: None,
            failing: Vec::new(),
            pending: Vec::new(),
            last_item_start: 0,
        }
    }

    pub(in crate::v2) fn ordinary(
        &mut self,
        bytes: Range<usize>,
        link: LinkState,
        passes: bool,
    ) -> Result<(), TryReserveError> {
        self.opener_passes.get_or_insert(link != LinkState::Broken);
        self.append(bytes, passes)
    }

    pub(in crate::v2) fn damaged_region(
        &mut self,
        bytes: Range<usize>,
    ) -> Result<(), TryReserveError> {
        self.append(bytes, false)
    }

    fn append(&mut self, bytes: Range<usize>, passes: bool) -> Result<(), TryReserveError> {
        let start = self.length;
        let end = start + bytes.len();
        if !passes {
            push(&mut self.failing, start..end)?;
        }
        if self.extents.is_empty() && self.first.physical.end == bytes.start {
            self.first.physical.end = bytes.end;
        } else if let Some(last) = self.extents.last_mut()
            && last.physical.end == bytes.start
        {
            last.physical.end = bytes.end;
        } else {
            push(
                &mut self.extents,
                Extent {
                    physical: bytes,
                    logical_start: start,
                },
            )?;
        }
        self.length = end;
        self.last_item_start = start;
        Ok(())
    }

    pub(in crate::v2) fn observe(
        &mut self,
        offset: usize,
        check: ParityCheck<'a>,
    ) -> Result<(), TryReserveError> {
        push(
            &mut self.pending,
            Pending {
                offset,
                check,
                position: self.length,
                last_item_start: self.last_item_start,
            },
        )
    }

    pub(in crate::v2) fn finish(
        &self,
        bytes: &[u8],
        findings: &mut Vec<ParityFinding>,
    ) -> Result<(), TryReserveError> {
        use ParityFindingKind as Kind;
        let mut protected_end = self.opener_length;
        let mut has_parity = false;
        for pending in &self.pending {
            let mut report = |kind| {
                push(
                    findings,
                    ParityFinding {
                        offset: pending.offset,
                        kind,
                    },
                )
            };
            match &pending.check {
                ParityCheck::UnknownGeometry { code, report: true } => {
                    report(Kind::UnknownGeometry(*code))?;
                    continue;
                }
                ParityCheck::UnknownGeometry { .. } => continue,
                ParityCheck::Damaged => report(Kind::Damaged)?,
                ParityCheck::Unchecked(reason) => report(Kind::Unchecked(*reason))?,
                ParityCheck::Table {
                    table,
                    payload_damaged,
                } => {
                    if *payload_damaged {
                        report(Kind::Damaged)?;
                    }
                    if let Some(end) = self.check_span(bytes, pending, table, report)? {
                        protected_end = protected_end.max(end);
                    }
                }
            }
            has_parity = true;
        }
        if has_parity && protected_end < self.length {
            let extent = self.extent_at(protected_end);
            push(
                findings,
                ParityFinding {
                    offset: extent.physical.start + (protected_end - extent.logical_start),
                    kind: Kind::UnprotectedTail(self.length - protected_end),
                },
            )?;
        }
        Ok(())
    }

    fn check_span(
        &self,
        bytes: &[u8],
        pending: &Pending<'_>,
        table: &ParityTable<'_>,
        mut report: impl FnMut(ParityFindingKind) -> Result<(), TryReserveError>,
    ) -> Result<Option<usize>, TryReserveError> {
        use ParityFindingKind as Kind;
        if table.span.end > pending.position as u64
            || table.span.end <= pending.last_item_start as u64
            || (table.geometry.span_bytes < SPAN_BYTES && table.span.end != pending.position as u64)
        {
            report(Kind::Stale)?;
            return Ok(None);
        }
        // Placement bounds both endpoints by an admitted slice length.
        let start = table.span.start as usize;
        let end = table.span.end as usize;
        let mismatch = table.compare_stream_shards(self.chunks(bytes, start..end));
        let data = mismatch.data_shards().count();
        let parity = mismatch.parity_shards().count();
        if data != 0 {
            report(Kind::DataMismatch(data))?;
        }
        if parity != 0 {
            report(Kind::ParityMismatch(parity))?;
        }
        let size = table.geometry.shard_bytes;
        let unexplained = mismatch
            .data_shards()
            .filter(|&index| {
                let shard_start = start + index * size;
                let shard_end = shard_start + (end - shard_start).min(size);
                (shard_start >= self.opener_length || self.opener_passes == Some(true))
                    && self
                        .failing
                        .get(
                            self.failing
                                .partition_point(|range| range.end <= shard_start),
                        )
                        .is_none_or(|range| range.start >= shard_end)
            })
            .count();
        if data != 0 && unexplained == data {
            report(Kind::Stale)?;
            return Ok(None);
        }
        if unexplained != 0 {
            report(Kind::Unexplained(unexplained))?;
        }
        Ok(Some(end))
    }

    fn extents_from(&self, start: usize) -> &[Extent] {
        &self.extents[self
            .extents
            .partition_point(|extent| extent.logical_end() <= start)..]
    }

    fn extent_at(&self, start: usize) -> &Extent {
        if start < self.first.logical_end() {
            &self.first
        } else {
            // The caller established a nonempty remainder of the stream.
            &self.extents_from(start)[0]
        }
    }

    fn chunks<'b>(&'b self, bytes: &'b [u8], span: Range<usize>) -> impl Iterator<Item = &'b [u8]> {
        let first = (span.start < self.first.logical_end()).then_some(&self.first);
        first
            .into_iter()
            .chain(self.extents_from(span.start))
            .take_while(move |extent| extent.logical_start < span.end)
            .map(move |extent| {
                let start = span.start.saturating_sub(extent.logical_start);
                let end = span.end.min(extent.logical_end()) - extent.logical_start;
                &bytes[extent.physical.start + start..extent.physical.start + end]
            })
    }
}
