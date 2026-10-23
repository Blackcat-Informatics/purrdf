// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent chronological source observations over the original read owner.

use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_hash::multiset::Multiset256;
use purrdf_lex::allocation::{StorageError, shared::Shared};

use super::{
    DatasetView, GlobalTermId, HashTable, QueryStorage, SegmentedError, SegmentedOperation,
    SegmentedStackQuery, SegmentedStackSnapshot, SegmentedStackSource, SegmentedStackTerm,
    SourceTerm, hash_of, reserve_table_with_memory,
};
use crate::ir::source_directory::{DirectoryChange, DirectoryEntry, DirectoryRead, SourceRecord};
use crate::ir::statistics::Relation;
use crate::source::{
    SourceChangeWork, SourceFamily, SourceRowRef, SourceSide, SourceVersion, SourceVisit,
    record_hash,
};
use crate::{GraphMatch, ProbePattern, QuadIds, RdfDataset};

mod directory;
use directory::EffectiveDirectory;

#[derive(Debug)]
pub(crate) struct SegmentedSourceVersion {
    pub(crate) snapshot: SegmentedStackSnapshot,
    pub(crate) records: Multiset256,
    directory: EffectiveDirectory,
    // A missing cache is a lazy prefix of this CURRENT snapshot. Its certified
    // source directories remain authoritative; no historical locator is kept.
    prefixes: Vec<Option<EffectiveDirectory>>,
    // Prefix buffers and shared control storage die before their original grant.
    _storage: QueryStorage,
}

#[derive(Debug)]
struct OwnedRow {
    subject: SegmentedStackTerm,
    predicate: SegmentedStackTerm,
    object: SegmentedStackTerm,
    graph: Option<SegmentedStackTerm>,
}

impl OwnedRow {
    fn as_ref(&self) -> SourceRowRef<'_> {
        SourceRowRef {
            subject: self.subject.value(),
            predicate: self.predicate.value(),
            object: self.object.value(),
            graph: self.graph.as_ref().map(SegmentedStackTerm::value),
        }
    }
}

#[derive(Debug)]
struct Candidate {
    digest: [u8; 32],
    row: OwnedRow,
}

#[derive(Debug)]
struct Candidates {
    rows: Vec<Candidate>,
    index: HashTable<usize>,
    storage: QueryStorage,
}

impl Candidates {
    fn new(operation: &SegmentedOperation) -> Result<Self, SegmentedError> {
        Ok(Self {
            rows: Vec::new(),
            index: HashTable::new(),
            storage: QueryStorage::new(operation)?,
        })
    }

    fn insert(&mut self, row: OwnedRow) -> Result<(), SegmentedError> {
        let digest = self.storage.run(|memory| row.as_ref().hashes(memory))?[0];
        let hash = hash_of(&digest);
        for &index in self.index.iter_hash(hash) {
            let other = &self.rows[index];
            if other.digest == digest
                && self.storage.run(|memory| -> Result<bool, StorageError> {
                    let a = row.as_ref();
                    let b = other.row.as_ref();
                    for (a, b) in [
                        (a.subject, b.subject),
                        (a.predicate, b.predicate),
                        (a.object, b.object),
                    ] {
                        if !a.try_cmp_with_memory(b, memory)?.is_eq() {
                            return Ok(false);
                        }
                    }
                    match (a.graph, b.graph) {
                        (None, None) => Ok(true),
                        (Some(a), Some(b)) => Ok(a.try_cmp_with_memory(b, memory)?.is_eq()),
                        _ => Ok(false),
                    }
                })?
            {
                return Ok(());
            }
        }
        let index = self.rows.len();
        self.storage.run(|memory| {
            memory.reserve_for_push(&mut self.rows)?;
            reserve_table_with_memory(
                &mut self.index,
                index.checked_add(1).ok_or(StorageError::SizeOverflow)?,
                |index| hash_of(&self.rows[*index].digest),
                memory,
            )
        })?;
        self.rows.push(Candidate { digest, row });
        self.index
            .insert_unique(hash, index, |index| hash_of(&self.rows[*index].digest));
        Ok(())
    }
}

struct Baseline {
    snapshot: SegmentedStackSnapshot,
}

impl<'snapshot> SegmentedStackQuery<'snapshot> {
    fn owned_directory_row(
        &self,
        origin: SegmentedStackSource,
        record: SourceRecord,
    ) -> Result<OwnedRow, SegmentedError> {
        let row = if let SegmentedStackSource::Layer(index) = origin {
            self.sessions[index]
                .source_record_row(record)?
                .map_ids(GlobalTermId::index)
        } else {
            self.resident(origin)
                .ok_or(SegmentedError::Corrupt("missing native directory source"))?
                .source_record_row(record)
                .map_ids(|id| id.index() as u64)
        };
        self.owned_source_row(origin, row)
    }

    fn initial_additions<D: DirectoryRead<Record = SourceRecord>>(
        &self,
        source: &D,
        origin: SegmentedStackSource,
        previous: &EffectiveDirectory,
        candidates: &mut Candidates,
    ) -> Result<(), SegmentedError>
    where
        SegmentedError: From<D::Error>,
    {
        for index in 0..source.len()? {
            let record = source.record(index)?;
            if previous.find(record.digest)?.is_none() {
                candidates.insert(self.owned_directory_row(origin, record)?)?;
            }
        }
        Ok(())
    }

    fn owned_effective_row(&self, key: [u8; 32]) -> Result<OwnedRow, SegmentedError> {
        if let Some(head) = &self.snapshot.head
            && let Some(record) = head
                .dataset
                .predicate_statistics()
                .effective_source_directory()
                .find(key)?
        {
            return self.owned_source_row(
                SegmentedStackSource::Head,
                head.dataset
                    .source_record_row(record)
                    .map_ids(|id| id.index() as u64),
            );
        }
        for index in (0..self.sessions.len()).rev() {
            if let Some(record) = self.sessions[index].source_directory().find(key)? {
                let row = self.sessions[index].source_record_row(record)?;
                return self.owned_source_row(
                    SegmentedStackSource::Layer(index),
                    row.map_ids(GlobalTermId::index),
                );
            }
        }
        self.check_ready()?;
        Err(SegmentedError::Corrupt(
            "effective source key has no row in its original snapshot",
        ))
    }

    fn compare_effective_directory<D: DirectoryRead<Record = SourceRecord>>(
        &self,
        before: &Self,
        previous: &EffectiveDirectory,
        source: &D,
        origin: SegmentedStackSource,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<EffectiveDirectory, SegmentedError>
    where
        SegmentedError: From<D::Error>,
    {
        let mut storage = QueryStorage::new(&self.operation)?;
        let mut next = previous.clone();
        let measured = storage.run(|memory| {
            crate::ir::source_directory::visit_changes(
                previous,
                source,
                memory,
                |change, _| -> Result<(), SegmentedError> {
                    let row = match change {
                        DirectoryChange::Previous(key) => {
                            next = next.set(key, false, &self.operation)?;
                            before.owned_effective_row(key)?
                        }
                        DirectoryChange::Next(record) => {
                            next = next.set(record.digest, true, &self.operation)?;
                            self.owned_directory_row(origin, record)?
                        }
                    };
                    candidates.insert(row)
                },
            )
        })?;
        work.directory.ranges += measured.ranges;
        work.directory.members += measured.members;
        work.directory.changed += measured.changed;
        work.physical += measured.changed;
        Ok(next)
    }

    fn compacted_candidates(
        &self,
        before: &Self,
        previous: &EffectiveDirectory,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<EffectiveDirectory, SegmentedError> {
        let next = self.compare_effective_directory(
            before,
            previous,
            &self.sessions[0].source_directory(),
            SegmentedStackSource::Layer(0),
            candidates,
            work,
        )?;
        self.diff_compacted_head(before, self.sessions.len() == 1, candidates, work)?;
        if next.summary() != self.sessions[0].source_union()? {
            return Err(SegmentedError::Corrupt(
                "compacted source directory disagrees with certified image",
            ));
        }
        Ok(next)
    }

    fn diff_compacted_head(
        &self,
        before: &Self,
        full: bool,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<(), SegmentedError> {
        // The old effective directory already includes its head. A missing new
        // head needs no second removal pass. An unchanged head is idempotent;
        // changed head operations are independently compared through native dirs.
        if !full {
            self.diff_heads(before, candidates, work)?;
        } else if self.snapshot.head.is_some() {
            let rows =
                |head: Option<&crate::SegmentedResidentHead>| -> Result<usize, SegmentedError> {
                    let Some(head) = head else {
                        return Ok(0);
                    };
                    head.dataset
                        .predicate_statistics()
                        .source_directory()
                        .len()
                        .checked_add(head.metadata.removals().map_or(0, |data| {
                            data.predicate_statistics().source_directory().len()
                        }))
                        .ok_or(SegmentedError::AddressExhausted)
                };
            let old = rows(before.snapshot.head.as_ref())?;
            let new = rows(self.snapshot.head.as_ref())?;
            if new <= old.saturating_sub(new) {
                // A reset head's old operations are already represented in the
                // independent full/base difference. Inspect only new operations.
                // Otherwise |old| <= 2|new|: native differences cost at most
                // 3|new| records and still skip unchanged cumulative heads.
                work.physical += self.add_layer_candidates(self.sessions.len(), candidates)?;
            } else {
                self.diff_heads(before, candidates, work)?;
            }
        }
        Ok(())
    }

    fn source_directories<'a>(
        &'a self,
        count: usize,
        storage: &mut QueryStorage,
    ) -> Result<
        (
            Vec<usize>,
            Vec<
                impl DirectoryRead<Error = SegmentedError, Record = SourceRecord>
                + 'a
                + use<'a, 'snapshot>,
            >,
        ),
        SegmentedError,
    > {
        storage.run(|memory| -> Result<_, SegmentedError> {
            let mut roots = Vec::new();
            for (index, source) in self
                .sessions
                .get(..count)
                .ok_or(SegmentedError::Corrupt("directory prefix outside snapshot"))?
                .iter()
                .enumerate()
            {
                let root = source.source_union()?;
                memory.push(&mut roots, (root.population(), *root.sum(), index))?;
            }
            roots.sort_unstable();
            roots.dedup_by_key(|entry| (entry.0, entry.1));
            let indices = memory.collect(roots.iter().map(|entry| entry.2))?;
            let sources = memory.collect(
                indices
                    .iter()
                    .map(|&index| self.sessions[index].source_directory()),
            )?;
            memory.release_vec(roots)?;
            Ok((indices, sources))
        })
    }

    fn compare_groups<A: DirectoryRead, B: DirectoryRead<Record = SourceRecord>>(
        &self,
        (previous, next, next_indices): (&[A], &[B], &[usize]),
        mut old_row: impl FnMut(usize, A::Record) -> Result<OwnedRow, SegmentedError>,
        mut raw: Option<EffectiveDirectory>,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
        storage: &mut QueryStorage,
    ) -> Result<Option<EffectiveDirectory>, SegmentedError>
    where
        SegmentedError: From<A::Error> + From<B::Error>,
    {
        let measured = storage.run(|memory| {
            crate::ir::source_directory::visit_directory_sets(
                previous,
                next,
                memory,
                |index, change, _| -> Result<(), SegmentedError> {
                    let row = match change {
                        DirectoryChange::Previous(record) => {
                            if let Some(directory) = &mut raw {
                                *directory =
                                    directory.set(record.digest(), false, &self.operation)?;
                            }
                            old_row(index, record)?
                        }
                        DirectoryChange::Next(record) => {
                            if let Some(directory) = &mut raw {
                                *directory = directory.set(record.digest, true, &self.operation)?;
                            }
                            self.owned_directory_row(
                                SegmentedStackSource::Layer(next_indices[index]),
                                record,
                            )?
                        }
                    };
                    candidates.insert(row)
                },
            )
        })?;
        work.directory.ranges += measured.ranges;
        work.directory.members += measured.members;
        work.directory.changed += measured.changed;
        work.physical += measured.changed;
        Ok(raw)
    }

    fn regrouped_candidates(
        &self,
        before: &Self,
        previous: &SegmentedSourceVersion,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<Option<EffectiveDirectory>, SegmentedError> {
        let mut storage = QueryStorage::new(&self.operation)?;
        let (new_indices, next) = self.source_directories(self.sessions.len(), &mut storage)?;
        let (old_indices, old) = before.source_directories(before.sessions.len(), &mut storage)?;
        let mut new_population = 0_u128;
        let mut old_population = 0_u128;
        for &index in &new_indices {
            new_population = new_population
                .checked_add(self.sessions[index].source_union()?.population())
                .ok_or(SegmentedError::AddressExhausted)?;
        }
        for &index in &old_indices {
            old_population = old_population
                .checked_add(before.sessions[index].source_union()?.population())
                .ok_or(SegmentedError::AddressExhausted)?;
        }
        let sealed = previous.prefixes.last().and_then(Option::as_ref);
        let base = if sealed.is_some_and(|sealed| {
            new_population.abs_diff(sealed.summary().population())
                < new_population.abs_diff(previous.records.population())
        }) {
            sealed.expect("checked sealed directory")
        } else {
            &previous.directory
        };
        let use_bag = new_population.abs_diff(old_population)
            < new_population.abs_diff(base.summary().population());
        let raw = if use_bag {
            let no_removals = before
                .snapshot
                .layers
                .iter()
                .skip(1)
                .all(|layer| layer.metadata.removals().is_none());
            let raw = if no_removals { sealed.cloned() } else { None };
            let raw = self.compare_groups(
                (&old, &next, &new_indices),
                |index, record| {
                    before.owned_directory_row(
                        SegmentedStackSource::Layer(old_indices[index]),
                        record,
                    )
                },
                raw,
                candidates,
                work,
                &mut storage,
            )?;
            for index in 1..before.sessions.len() {
                work.physical += before
                    .add_source_candidates(SegmentedStackSource::Removals(index), candidates)?;
            }
            raw
        } else {
            self.compare_groups(
                (std::slice::from_ref(base), &next, &new_indices),
                |_, key| before.owned_effective_row(key),
                Some(base.clone()),
                candidates,
                work,
                &mut storage,
            )?
        };
        for index in 1..self.sessions.len() {
            work.physical +=
                self.add_source_candidates(SegmentedStackSource::Removals(index), candidates)?;
        }
        // Sealed-only comparisons must include both heads' operation changes.
        // Only the full effective baseline already accounts for retired head
        // rows, allowing the compacted-head reset shortcut.
        self.diff_compacted_head(
            before,
            !use_bag && std::ptr::eq(base, &raw const previous.directory),
            candidates,
            work,
        )?;
        Ok(
            if self
                .snapshot
                .layers
                .iter()
                .skip(1)
                .all(|layer| layer.metadata.removals().is_none())
            {
                raw
            } else {
                None
            },
        )
    }

    fn lazy_compacted_candidates(
        &self,
        before: &Self,
        folded: usize,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<(), SegmentedError> {
        let mut storage = QueryStorage::new(&self.operation)?;
        let (old_indices, old) = before.source_directories(folded, &mut storage)?;
        let (new_indices, next) = self.source_directories(1, &mut storage)?;
        self.compare_groups(
            (&old, &next, &new_indices),
            |index, record| {
                before.owned_directory_row(SegmentedStackSource::Layer(old_indices[index]), record)
            },
            None,
            candidates,
            work,
            &mut storage,
        )?;
        for index in 1..folded {
            work.physical +=
                before.add_source_candidates(SegmentedStackSource::Removals(index), candidates)?;
        }
        self.diff_heads(before, candidates, work)
    }

    /// A changed first image may replace several old prefixes while the exact
    /// suffix descriptors survive. Compare canonical prefix keys, never the
    /// removed physical layers. A one-image publication uses the old full union.
    fn compacted_prefix(
        &self,
        previous: &SegmentedSourceVersion,
        work: &mut SourceChangeWork,
    ) -> Option<usize> {
        if self.sessions.is_empty() {
            return None;
        }
        let old = &previous.snapshot.layers;
        let new = &self.snapshot.layers;
        let mut suffix = 0;
        while suffix < old.len().min(new.len()) {
            work.descriptors += 1;
            if !Arc::ptr_eq(&old[old.len() - suffix - 1], &new[new.len() - suffix - 1]) {
                break;
            }
            suffix += 1;
        }
        if new.len() == 1 && suffix == 0 {
            Some(old.len())
        } else if new.len() - suffix == 1 && old.len() > suffix {
            Some(old.len() - suffix)
        } else {
            None
        }
    }

    fn owned_row(&self, row: QuadIds<GlobalTermId>) -> Result<OwnedRow, SegmentedError> {
        Ok(OwnedRow {
            subject: self.term_value(row.s)?,
            predicate: self.term_value(row.p)?,
            object: self.term_value(row.o)?,
            graph: row.g.map(|id| self.term_value(id)).transpose()?,
        })
    }

    fn owned_source_row(
        &self,
        origin: SegmentedStackSource,
        row: QuadIds<u64>,
    ) -> Result<OwnedRow, SegmentedError> {
        let value = |index| {
            self.source_value(SourceTerm {
                source: origin,
                index,
            })
        };
        Ok(OwnedRow {
            subject: value(row.s)?,
            predicate: value(row.p)?,
            object: value(row.o)?,
            graph: row.g.map(value).transpose()?,
        })
    }

    fn add_source_candidates(
        &self,
        origin: SegmentedStackSource,
        candidates: &mut Candidates,
    ) -> Result<u64, SegmentedError> {
        let mut count = 0;
        for row in self.physical_source_rows(origin) {
            candidates.insert(self.owned_row(row)?)?;
            count += 1;
        }
        self.check_ready()?;
        Ok(count)
    }

    fn add_layer_candidates(
        &self,
        index: usize,
        candidates: &mut Candidates,
    ) -> Result<u64, SegmentedError> {
        let count = self.add_source_candidates(self.source_at(index), candidates)?;
        let removals = if index == self.sessions.len() {
            SegmentedStackSource::HeadRemovals
        } else {
            SegmentedStackSource::Removals(index)
        };
        Ok(count + self.add_source_candidates(removals, candidates)?)
    }

    fn diff_resident(
        &self,
        before: &Self,
        carriers: (Option<&Arc<RdfDataset>>, Option<&Arc<RdfDataset>>),
        origin: SegmentedStackSource,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<(), SegmentedError> {
        match carriers {
            (Some(old), Some(new)) if Arc::ptr_eq(old, new) => {}
            (Some(old), Some(new)) => {
                let mut storage = QueryStorage::new(&self.operation)?;
                let measured = storage.run(|memory| {
                    old.visit_source_changes_with_memory(new, memory, |side, _, row, _| {
                        let source = if side == SourceSide::Previous {
                            before
                        } else {
                            self
                        };
                        candidates.insert(
                            source.owned_source_row(origin, row.map_ids(|id| id.index() as u64))?,
                        )
                    })
                })?;
                work.directory.ranges += measured.ranges;
                work.directory.members += measured.members;
                work.directory.changed += measured.changed;
                work.physical += measured.changed;
            }
            (Some(_), None) => work.physical += before.add_source_candidates(origin, candidates)?,
            (None, Some(_)) => work.physical += self.add_source_candidates(origin, candidates)?,
            (None, None) => {}
        }
        Ok(())
    }

    fn diff_heads(
        &self,
        before: &Self,
        candidates: &mut Candidates,
        work: &mut SourceChangeWork,
    ) -> Result<(), SegmentedError> {
        let a = before.snapshot.head.as_ref();
        let b = self.snapshot.head.as_ref();
        self.diff_resident(
            before,
            (
                a.map(crate::SegmentedResidentHead::dataset),
                b.map(crate::SegmentedResidentHead::dataset),
            ),
            SegmentedStackSource::Head,
            candidates,
            work,
        )?;
        self.diff_resident(
            before,
            (
                a.and_then(|head| head.metadata.removals()),
                b.and_then(|head| head.metadata.removals()),
            ),
            SegmentedStackSource::HeadRemovals,
            candidates,
            work,
        )
    }

    fn removal_root(carrier: Option<&Arc<RdfDataset>>) -> Multiset256 {
        carrier.map_or_else(Multiset256::default, |source| {
            source.predicate_statistics().source_union()
        })
    }

    fn source_baseline(
        &self,
        previous: &SegmentedSourceVersion,
        work: &mut SourceChangeWork,
    ) -> Result<Baseline, SegmentedError> {
        let old = &previous.snapshot;
        let new = self.snapshot;
        let same = old.layers.len() == new.layers.len()
            && old.layers.iter().zip(new.layers.iter()).all(|(a, b)| {
                work.descriptors += 1;
                Arc::ptr_eq(a, b)
            });
        if same {
            return Ok(Baseline {
                snapshot: old.clone(),
            });
        }
        if new.layers.len() == old.layers.len() + 1
            && old.layers.iter().zip(new.layers.iter()).all(|(a, b)| {
                work.descriptors += 1;
                Arc::ptr_eq(a, b)
            })
            && let Some(head) = &old.head
            && self.sessions[new.layers.len() - 1].source_union()?
                == head.dataset.predicate_statistics().source_union()
            && Self::removal_root(
                new.layers
                    .last()
                    .and_then(|layer| layer.metadata.removals()),
            ) == Self::removal_root(head.metadata.removals())
        {
            return Ok(Baseline {
                snapshot: SegmentedStackSnapshot {
                    layers: Arc::clone(&new.layers),
                    head: None,
                },
            });
        }
        if let Some(first) = self.sessions.first() {
            let root = first.source_union()?;
            if new.layers.len() == 1 && root == previous.records {
                // Reapplying the unchanged head A ∪ (S \ R) is idempotent.
                // Keep it when the caller retained it across compaction, so its
                // rows do not become a spurious corpus-sized physical addition.
                let mut snapshot = SegmentedStackSnapshot {
                    layers: Arc::clone(&new.layers),
                    head: old.head.clone(),
                };
                if snapshot != *new {
                    snapshot.head = None;
                }
                return Ok(Baseline { snapshot });
            }
            for (index, recorded) in previous.prefixes.iter().enumerate() {
                let Some(recorded) = recorded else {
                    continue;
                };
                if recorded.summary() != root || old.layers.len() - index != new.layers.len() {
                    continue;
                }
                if old.layers[index + 1..]
                    .iter()
                    .zip(new.layers[1..].iter())
                    .all(|(a, b)| {
                        work.descriptors += 1;
                        Arc::ptr_eq(a, b)
                    })
                {
                    return Ok(Baseline {
                        snapshot: SegmentedStackSnapshot {
                            layers: Arc::clone(&new.layers),
                            head: old.head.clone(),
                        },
                    });
                }
            }
        }
        Ok(Baseline {
            snapshot: old.clone(),
        })
    }

    pub(super) fn observed_source_changes<B>(
        &self,
        previous: &SourceVersion,
        mut visit: impl FnMut(SourceSide, SourceFamily, SourceRowRef<'_>) -> ControlFlow<B>,
    ) -> Result<SourceVisit<B>, SegmentedError> {
        self.check_ready()?;
        if previous.0.snapshot == *self.snapshot {
            return Ok(SourceVisit::Complete {
                version: previous.clone(),
                work: SourceChangeWork::default(),
            });
        }
        let mut work = SourceChangeWork::default();
        let baseline = self.source_baseline(&previous.0, &mut work)?;
        let folded = self.compacted_prefix(&previous.0, &mut work);
        let before = previous
            .0
            .snapshot
            .query_with_operation(self.operation.clone())?;
        let mut candidates = Candidates::new(&self.operation)?;
        let mut regrouped = false;
        let compacted = if self.sessions.is_empty() {
            let directory = if let Some(head) = &self.snapshot.head {
                self.compare_effective_directory(
                    &before,
                    &previous.0.directory,
                    head.dataset
                        .predicate_statistics()
                        .effective_source_directory(),
                    SegmentedStackSource::Head,
                    &mut candidates,
                    &mut work,
                )?
            } else {
                self.compare_effective_directory(
                    &before,
                    &previous.0.directory,
                    &crate::ir::source_directory::SourceDirectory::default(),
                    SegmentedStackSource::Head,
                    &mut candidates,
                    &mut work,
                )?
            };
            Some(directory)
        } else if let Some(folded) = folded {
            if self.sessions.len() == 1 {
                Some(self.compacted_candidates(
                    &before,
                    &previous.0.directory,
                    &mut candidates,
                    &mut work,
                )?)
            } else if let Some(base) = &previous.0.prefixes[folded - 1] {
                Some(self.compacted_candidates(&before, base, &mut candidates, &mut work)?)
            } else {
                self.lazy_compacted_candidates(&before, folded, &mut candidates, &mut work)?;
                None
            }
        } else {
            let candidate_before = baseline
                .snapshot
                .query_with_operation(self.operation.clone())?;
            let same_layers = baseline.snapshot.layers.len() == self.snapshot.layers.len()
                && baseline
                    .snapshot
                    .layers
                    .iter()
                    .zip(self.snapshot.layers.iter())
                    .all(|(a, b)| Arc::ptr_eq(a, b));
            if same_layers {
                self.diff_heads(&candidate_before, &mut candidates, &mut work)?;
                None
            } else {
                regrouped = true;
                self.regrouped_candidates(&before, &previous.0, &mut candidates, &mut work)?
            }
        };
        let mut records = previous.0.records;
        let mut directory = if self.sessions.is_empty()
            || ((self.sessions.len() == 1 || regrouped) && self.snapshot.head.is_none())
        {
            compacted
                .clone()
                .unwrap_or_else(|| previous.0.directory.clone())
        } else {
            previous.0.directory.clone()
        };
        let result = self.advance_records(
            &before,
            (before.source_count(), self.source_count()),
            (&mut records, &mut directory),
            &candidates,
            &mut work,
            &mut visit,
        )?;
        before.check_ready()?;
        self.check_ready()?;
        if let ControlFlow::Break(value) = result {
            return Ok(SourceVisit::Stopped(value));
        }
        let mut storage = QueryStorage::new(&self.operation)?;
        let mut prefixes = Vec::new();
        // Sealing may transfer the complete previous head into new immutable
        // layers while a fresh head is already present. The new sealed union is
        // then the previous full directory, not its former sealed-only prefix.
        let sealed_equivalent = baseline.snapshot.head.is_none()
            && baseline.snapshot.layers.len() == self.snapshot.layers.len()
            && baseline
                .snapshot
                .layers
                .iter()
                .zip(self.snapshot.layers.iter())
                .all(|(a, b)| Arc::ptr_eq(a, b));
        let common = previous
            .0
            .snapshot
            .layers
            .iter()
            .zip(self.snapshot.layers.iter())
            .take_while(|(a, b)| Arc::ptr_eq(a, b))
            .count();
        for index in 0..self.sessions.len() {
            let known = if index + 1 == self.sessions.len() && self.snapshot.head.is_none() {
                Some(directory.clone())
            } else if index + 1 == self.sessions.len() && sealed_equivalent {
                Some(previous.0.directory.clone())
            } else if index < common {
                previous.0.prefixes[index].clone()
            } else if regrouped {
                if index + 1 == self.sessions.len() {
                    compacted.clone()
                } else {
                    None
                }
            } else if index == 0 && folded.is_some() {
                compacted.clone()
            } else {
                let old_index = if let Some(folded) = folded
                    && index != 0
                {
                    Some(folded + index - 1)
                } else {
                    previous
                        .0
                        .prefixes
                        .len()
                        .checked_sub(1)
                        .map(|last| index.min(last))
                };
                let (known, old_limit) = old_index.map_or_else(
                    || (Some(previous.0.directory.clone()), before.source_count()),
                    |old| (previous.0.prefixes[old].clone(), old + 1),
                );
                if let Some(mut known) = known {
                    let mut summary = known.summary();
                    let _ = self.advance_records(
                        &before,
                        (old_limit, index + 1),
                        (&mut summary, &mut known),
                        &candidates,
                        &mut SourceChangeWork::default(),
                        &mut |_, _, _| ControlFlow::<std::convert::Infallible>::Continue(()),
                    )?;
                    Some(known)
                } else {
                    None
                }
            };
            storage.run(|memory| memory.push(&mut prefixes, known))?;
        }
        let version = self.seal_version(prefixes, records, directory, storage)?;
        Ok(SourceVisit::Complete { version, work })
    }

    fn has_source_row(
        &self,
        row: SourceRowRef<'_>,
        family: SourceFamily,
        sources: usize,
    ) -> Result<bool, SegmentedError> {
        if sources == 0 {
            return Ok(false);
        }
        let Some(s) = self.term_id_by_value(row.subject)? else {
            return Ok(false);
        };
        let Some(p) = self.term_id_by_value(row.predicate)? else {
            return Ok(false);
        };
        let Some(o) = self.term_id_by_value(row.object)? else {
            return Ok(false);
        };
        let g = match row.graph {
            None => GraphMatch::Default,
            Some(graph) => {
                let Some(g) = self.term_id_by_value(graph)? else {
                    return Ok(false);
                };
                GraphMatch::Named(g)
            }
        };
        let present = self
            .source_rows(
                family,
                ProbePattern {
                    s: Some(s),
                    p: Some(p),
                    o: Some(o),
                    g,
                },
                sources,
            )
            .next()
            .is_some();
        self.check_ready()?;
        Ok(present)
    }

    fn advance_records<B>(
        &self,
        before: &Self,
        limits: (usize, usize),
        (records, directory): (&mut Multiset256, &mut EffectiveDirectory),
        candidates: &Candidates,
        work: &mut SourceChangeWork,
        visit: &mut impl FnMut(SourceSide, SourceFamily, SourceRowRef<'_>) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B>, SegmentedError> {
        work.candidates += candidates.rows.len() as u64;
        for candidate in &candidates.rows {
            for family in [SourceFamily::Statements, SourceFamily::Reifiers] {
                let row = candidate.row.as_ref();
                let was = before.has_source_row(row, family, limits.0)?;
                let now = self.has_source_row(row, family, limits.1)?;
                if was == now {
                    continue;
                }
                let role = if family == SourceFamily::Statements {
                    Relation::Asserted
                } else {
                    Relation::Reifiers
                };
                let hash = record_hash(role, candidate.digest);
                let side = if now {
                    records
                        .insert(hash)
                        .map_err(|_| SegmentedError::AddressExhausted)?;
                    SourceSide::Next
                } else {
                    records.remove(hash).map_err(|_| {
                        SegmentedError::Corrupt("source transition population underflow")
                    })?;
                    SourceSide::Previous
                };
                *directory = directory.set(hash, now, &self.operation)?;
                work.emitted += 1;
                if let ControlFlow::Break(value) = visit(side, family, row) {
                    return Ok(ControlFlow::Break(value));
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn seal_version(
        &self,
        prefixes: Vec<Option<EffectiveDirectory>>,
        records: Multiset256,
        directory: EffectiveDirectory,
        mut storage: QueryStorage,
    ) -> Result<SourceVersion, SegmentedError> {
        if directory.summary() != records {
            return Err(SegmentedError::Corrupt(
                "effective source directory disagrees with observed records",
            ));
        }
        storage.run(|memory| {
            memory.add_bytes(Shared::<SegmentedSourceVersion>::allocation_layout().size())
        })?;
        self.check_ready()?;
        let version = Shared::try_new(SegmentedSourceVersion {
            snapshot: self.snapshot.clone(),
            records,
            directory,
            prefixes,
            _storage: storage,
        })
        .map_err(|_| SegmentedError::AllocationFailed("source version"))?;
        Ok(SourceVersion(version))
    }

    pub(super) fn capture_source_version(&self) -> Result<SourceVersion, SegmentedError> {
        self.check_ready()?;
        let mut storage = QueryStorage::new(&self.operation)?;
        let mut prefixes = Vec::new();
        let mut records = Multiset256::default();
        let mut directory = EffectiveDirectory::default();
        for index in 0..self.source_count() {
            if index == 0 {
                records = if let Some(first) = self.sessions.first() {
                    directory = EffectiveDirectory::from_source(
                        &first.source_directory(),
                        &self.operation,
                    )?;
                    first.source_union()?
                } else {
                    let dataset = &self
                        .snapshot
                        .head
                        .as_ref()
                        .expect("the first resident source exists")
                        .dataset;
                    directory = EffectiveDirectory::from_source(
                        dataset.predicate_statistics().effective_source_directory(),
                        &self.operation,
                    )?;
                    dataset.predicate_statistics().source_union()
                };
            } else {
                let (source, removals) = if index < self.sessions.len() {
                    (
                        self.sessions[index].source_union()?,
                        self.snapshot.layers[index].metadata.removals(),
                    )
                } else {
                    let head = self
                        .snapshot
                        .head
                        .as_ref()
                        .expect("the resident source exists");
                    (
                        head.dataset.predicate_statistics().source_union(),
                        head.metadata.removals(),
                    )
                };
                if source == records && removals.is_none() {
                    if index < self.sessions.len() {
                        storage
                            .run(|memory| memory.push(&mut prefixes, Some(directory.clone())))?;
                    }
                    continue;
                }
                if removals.is_none() {
                    directory = if index < self.sessions.len() {
                        directory.with_source_union(
                            &self.sessions[index].source_directory(),
                            &self.operation,
                        )?
                    } else {
                        let head = self
                            .snapshot
                            .head
                            .as_ref()
                            .expect("the resident source exists");
                        directory.with_source_union(
                            head.dataset
                                .predicate_statistics()
                                .effective_source_directory(),
                            &self.operation,
                        )?
                    };
                    records = directory.summary();
                    if index < self.sessions.len() {
                        storage
                            .run(|memory| memory.push(&mut prefixes, Some(directory.clone())))?;
                    }
                    continue;
                }
                let mut candidates = Candidates::new(&self.operation)?;
                let origin = self.source_at(index);
                let removals = if index < self.sessions.len() {
                    self.initial_additions(
                        &self.sessions[index].source_directory(),
                        origin,
                        &directory,
                        &mut candidates,
                    )?;
                    SegmentedStackSource::Removals(index)
                } else {
                    let head = self
                        .snapshot
                        .head
                        .as_ref()
                        .expect("the resident source exists");
                    self.initial_additions(
                        head.dataset
                            .predicate_statistics()
                            .effective_source_directory(),
                        origin,
                        &directory,
                        &mut candidates,
                    )?;
                    SegmentedStackSource::HeadRemovals
                };
                let _ = self.add_source_candidates(removals, &mut candidates)?;
                let ControlFlow::Continue(()) = self.advance_records(
                    self,
                    (index, index + 1),
                    (&mut records, &mut directory),
                    &candidates,
                    &mut SourceChangeWork::default(),
                    &mut |_, _, _| ControlFlow::<std::convert::Infallible>::Continue(()),
                )?;
            }
            if index < self.sessions.len() {
                storage.run(|memory| memory.push(&mut prefixes, Some(directory.clone())))?;
            }
        }
        self.seal_version(prefixes, records, directory, storage)
    }
}
