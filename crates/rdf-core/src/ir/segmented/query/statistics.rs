// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-local evidence translated only for the caller's bound values.

use std::hash::{Hash, Hasher};

use super::{SegmentedStackQuery, SegmentedStackSource};
use crate::statistics::layered::{LayeredStatistics, Request, combine};
use crate::{DatasetView, GlobalTermId, PredicateStatistics, ProbePattern, TermId};
use purrdf_lex::allocation::StorageError;

impl LayeredStatistics for SegmentedStackQuery<'_> {
    fn statistics_source(&self) -> &'static str {
        "segmented-chronological-source-bounds-v1"
    }

    fn statistics_revision(&self) -> u64 {
        let mut hash = purrdf_hash::fixed::FixedHasher::default();
        self.snapshot.layers.len().hash(&mut hash);
        for layer in self.snapshot.layers.iter() {
            layer.receipt.snapshot().hash(&mut hash);
            layer.metadata.statistics_revision.hash(&mut hash);
        }
        self.snapshot
            .head
            .as_ref()
            .map(|head| head.statistics_revision)
            .hash(&mut hash);
        hash.finish()
    }

    fn physical_statistic(
        &self,
        request: Request,
        pattern: ProbePattern<GlobalTermId>,
    ) -> Option<PredicateStatistics> {
        self.check_ready().ok()?;
        let layered = self.source_count() > 1;
        let count = combine(
            (0..self.source_count()).map(|index| {
                if let Some(source) = self.sessions.get(index) {
                    let pattern = self
                        .translate_pattern(
                            source,
                            SegmentedStackSource::Layer(index),
                            pattern,
                            |index| {
                                GlobalTermId::from_index(index)
                                    .map_err(|error| StorageError::from(error).into())
                            },
                            |error| error,
                        )
                        .or_else(|error| self.operation.fail(error))
                        .ok()?;
                    match pattern {
                        Some(pattern) => request.read(source.statistics()?, pattern, layered),
                        None => combine(std::iter::empty(), false),
                    }
                } else {
                    let source = &**self.snapshot.head.as_ref()?.dataset();
                    let pattern = self
                        .translate_pattern(
                            source,
                            SegmentedStackSource::Head,
                            pattern,
                            |index| {
                                let index = u32::try_from(index)
                                    .map_err(|_| super::SegmentedError::AddressExhausted)?;
                                TermId::from_index(index)
                                    .map_err(|error| StorageError::from(error).into())
                            },
                            |error| match error {},
                        )
                        .or_else(|error| self.operation.fail(error))
                        .ok()?;
                    match pattern {
                        Some(pattern) => request.read(source.statistics()?, pattern, layered),
                        None => combine(std::iter::empty(), false),
                    }
                }
            }),
            layered,
        )?;
        self.check_ready().ok()?;
        Some(count)
    }
}
