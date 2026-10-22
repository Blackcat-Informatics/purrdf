// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Append coordinates remain logical u64 identities after vacant metadata is reclaimed.

use super::{LoggedRow, PartitionId, RowId, StorageError, TermIdentity};
use crate::allocation::{flush_retired_bytes, reserve_ordered_in};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, Resident};

type Row<I> = (RowId, PartitionId, I, I);

#[derive(Debug)]
pub(super) enum RowLog<I: TermIdentity> {
    Dense(Vec<LoggedRow<I>>),
    Sparse {
        live: OrderedTable<u64, Row<I>>,
        next: u64,
        supplemental: bool,
        retired_bytes: usize,
    },
}

impl<I: TermIdentity> Clone for RowLog<I> {
    fn clone(&self) -> Self {
        match self {
            Self::Dense(rows) => Self::Dense(rows.clone()),
            Self::Sparse { live, next, .. } => Self::Sparse {
                live: live
                    .try_clone(&mut Memory::new(&mut Resident))
                    .expect("cloning resident row metadata requires physical storage"),
                next: *next,
                supplemental: false,
                retired_bytes: 0,
            },
        }
    }
}

impl<I: TermIdentity> PartialEq for RowLog<I> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Dense(a), Self::Dense(b)) => a == b,
            (
                Self::Sparse {
                    live: a, next: an, ..
                },
                Self::Sparse {
                    live: b, next: bn, ..
                },
            ) => an == bn && a.ordered().eq(b.ordered()),
            _ => false,
        }
    }
}
impl<I: TermIdentity> Eq for RowLog<I> {}

impl<I: TermIdentity> Default for RowLog<I> {
    fn default() -> Self {
        Self::Dense(Vec::new())
    }
}

impl<I: TermIdentity> RowLog<I> {
    pub(super) fn next(&self) -> u64 {
        match self {
            Self::Dense(rows) => rows.len() as u64,
            Self::Sparse { next, .. } => *next,
        }
    }

    pub(super) fn retained(&self) -> usize {
        match self {
            Self::Dense(rows) => rows.len(),
            Self::Sparse { live, .. } => live.len(),
        }
    }

    pub(super) const fn is_sparse(&self) -> bool {
        matches!(self, Self::Sparse { .. })
    }

    pub(super) fn reserve(&mut self) -> Result<(), StorageError> {
        assert!(
            !self.is_supplemental(),
            "supplemental row growth retains its original memory"
        );
        match self {
            Self::Dense(rows) => rows
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed),
            Self::Sparse { live, .. } => {
                let bytes = live.storage_bytes()?;
                live.reserve_insert(&mut Memory::resume(&mut Resident, bytes))
            }
        }
    }

    pub(super) const fn is_supplemental(&self) -> bool {
        matches!(
            self,
            Self::Sparse {
                supplemental: true,
                ..
            }
        )
    }

    pub(super) fn reserve_in<A: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        let Self::Sparse {
            live,
            supplemental: true,
            retired_bytes,
            ..
        } = self
        else {
            return self.reserve();
        };
        flush_retired_bytes(retired_bytes, memory)?;
        reserve_ordered_in(live, memory, retired_bytes)
    }

    /// Build new metadata under the supplied owner; existing buffers keep theirs.
    pub(super) fn copy_sparse_in<A: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, A>,
    ) -> Result<Self, StorageError> {
        memory.scope(|memory| {
            let mut live = OrderedTable::default();
            for row in self.from(0) {
                assert!(live.insert(row.0.index(), row, memory)?.is_none());
            }
            Ok(Self::Sparse {
                live,
                next: self.next(),
                supplemental: true,
                retired_bytes: 0,
            })
        })
    }

    /// One-time construction changes metadata layout, never arrangements or identities.
    pub(super) fn prepare_sparse(&self) -> Result<Option<Self>, StorageError> {
        if self.is_sparse() {
            return Ok(None);
        }
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut rows = self.copy_sparse_in(&mut memory)?;
        if let Self::Sparse { supplemental, .. } = &mut rows {
            *supplemental = false;
        }
        Ok(Some(rows))
    }

    pub(super) fn append_in<A: Admission + ?Sized>(
        &mut self,
        row: Row<I>,
        memory: &mut Memory<'_, A>,
    ) {
        assert_eq!(
            row.0.index(),
            self.next(),
            "append identities never alias a retired row"
        );
        match self {
            Self::Dense(rows) => rows.push((Some(row.0), row.1, row.2, row.3)),
            Self::Sparse {
                live,
                next,
                supplemental,
                ..
            } => {
                publish_live(live, row, *supplemental, memory);
                *next += 1; // The preceding checked RowId mint refuses u64::MAX.
            }
        }
    }

    pub(super) fn remove(&mut self, row: RowId) -> Row<I> {
        match self {
            Self::Dense(rows) => {
                let (live, partition, subject, object) = &mut rows[row.resident_index()];
                assert_eq!(live.take(), Some(row));
                (row, *partition, *subject, *object)
            }
            Self::Sparse { live, .. } => live
                .remove(&row.index())
                .expect("a live arrangement owns its row metadata"),
        }
    }

    pub(super) fn check_restore(&self, row: Row<I>) {
        assert!(
            row.0.index() < self.next(),
            "restoration never mints a foreign append coordinate"
        );
        match self {
            Self::Dense(rows) => {
                assert_eq!(rows[row.0.resident_index()], (None, row.1, row.2, row.3));
            }
            Self::Sparse { live, .. } => assert!(live.get(&row.0.index()).is_none()),
        }
    }

    pub(super) fn restore_in<A: Admission + ?Sized>(
        &mut self,
        row: Row<I>,
        memory: &mut Memory<'_, A>,
    ) {
        self.check_restore(row);
        match self {
            Self::Dense(rows) => rows[row.0.resident_index()].0 = Some(row.0),
            Self::Sparse {
                live, supplemental, ..
            } => {
                publish_live(live, row, *supplemental, memory);
            }
        }
    }

    pub(super) fn from(&self, since: u64) -> impl Iterator<Item = Row<I>> + '_ {
        let dense = match self {
            Self::Dense(rows) => {
                let start =
                    usize::try_from(since.min(rows.len() as u64)).expect("bounded resident slice");
                Some(rows[start..].iter())
            }
            Self::Sparse { .. } => None,
        };
        let sparse = match self {
            Self::Sparse { live, .. } => Some(live.range(since..)),
            Self::Dense(_) => None,
        };
        dense
            .into_iter()
            .flatten()
            .filter_map(|&(row, partition, subject, object)| {
                row.map(|row| (row, partition, subject, object))
            })
            .chain(sparse.into_iter().flatten().map(|(_, &row)| row))
    }
}

fn publish_live<I: TermIdentity, A: Admission + ?Sized>(
    live: &mut OrderedTable<u64, Row<I>>,
    row: Row<I>,
    supplemental: bool,
    memory: &mut Memory<'_, A>,
) {
    let bytes = live
        .storage_bytes()
        .expect("the admitted row arena has a valid layout");
    let inserted = if supplemental {
        live.insert(row.0.index(), row, memory)
    } else {
        live.insert(
            row.0.index(),
            row,
            &mut Memory::resume(&mut Resident, bytes),
        )
    };
    assert!(
        inserted
            .expect("row storage was physically reserved before publication")
            .is_none()
    );
    assert_eq!(
        live.storage_bytes().expect("published row arena layout"),
        bytes,
        "publication cannot grow its admitted row arena"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clause::{ClauseAtom, ClauseTerm, DlClause},
        guard::NoGuards,
        maintenance::ChangeBatch,
        seminaive::{
            EvalOptions, Maintained, ResumeProgress, SessionContext, compile, evaluate_guarded,
        },
        store::{RelationStore, TermInterner},
    };
    use purrdf_hash::hex::Digest32;
    use purrdf_lex::allocation::Resident;

    #[test]
    fn supplemental_native_metadata_factories_preserve_original_publication_and_clones() {
        use crate::dictionary::{DictionaryError, DictionaryTerm};
        use std::cell::Cell;

        struct Grant<'a> {
            live: &'a Cell<usize>,
            maximum: usize,
            refuse_shrink: bool,
            requested: usize,
        }
        impl Admission for Grant<'_> {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.maximum {
                    self.requested = bytes;
                    return Err(StorageError::AdmissionFailed);
                }
                if self.refuse_shrink && bytes < self.live.get() {
                    return Err(StorageError::AdmissionFailed);
                }
                self.live.set(bytes);
                Ok(())
            }
        }
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            refuse_shrink: false,
            requested: 0,
        };
        let mut memory = Memory::new(&mut grant);
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for subject in ["a", "b", "c"] {
            input.insert_row([subject, "in", "o"]).unwrap();
        }
        let original = input.rows_from(0).collect::<Vec<_>>();
        input.admit_native_metadata_in(&mut memory).unwrap();
        assert!(input.row_log.is_supplemental());
        assert_eq!(input.rows_from(0).collect::<Vec<_>>(), original);
        assert!(live.get() > 0);
        assert_eq!(live.get(), memory.admitted_bytes());
        let held = live.get();
        let RowLog::Sparse { live: rows, .. } = &input.row_log else {
            panic!("native row arena")
        };
        let row_bytes = rows.storage_bytes().unwrap();
        assert!(held > row_bytes && input.lookup_supplemental);
        memory.admission_mut().maximum = held;
        let mut independent = input.clone();
        assert!(!independent.row_log.is_supplemental());
        assert!(!independent.lookup_supplemental);
        independent.insert_row(["independent", "in", "o"]).unwrap();
        assert_eq!(
            live.get(),
            held,
            "clones construct their own resident metadata"
        );
        drop(independent);
        input
            .insert_terms_in(["d", "in", "o"].map(DictionaryTerm::Surface), &mut memory)
            .unwrap();
        for _ in 0..2 {
            assert!(matches!(
                input.insert_terms_in(
                    ["fresh", "in", "o"].map(DictionaryTerm::Surface),
                    &mut memory
                ),
                Err(DictionaryError::Storage(StorageError::AdmissionFailed))
            ));
            assert_eq!(input.row_count(), 4);
            assert_eq!(input.term_id("fresh"), None);
            assert_eq!(memory.admitted_bytes(), held);
            assert_eq!(live.get(), held);
        }
        memory.admission_mut().maximum = usize::MAX;
        memory.admission_mut().refuse_shrink = true;
        assert!(matches!(
            input.insert_terms_in(
                ["fresh", "in", "o"].map(DictionaryTerm::Surface),
                &mut memory
            ),
            Err(DictionaryError::Storage(StorageError::AdmissionFailed))
        ));
        let protected = live.get();
        assert!(protected > held);
        let RowLog::Sparse { retired_bytes, .. } = &input.row_log else {
            panic!("original sparse owner")
        };
        assert_eq!(*retired_bytes, row_bytes);
        assert!(matches!(
            input.insert_terms_in(
                ["fresh", "in", "o"].map(DictionaryTerm::Surface),
                &mut memory
            ),
            Err(DictionaryError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(live.get(), protected);
        assert_eq!(input.row_count(), 4);
        assert_eq!(input.term_id("fresh"), None);
        memory.admission_mut().refuse_shrink = false;
        input
            .insert_terms_in(
                ["fresh", "in", "o"].map(DictionaryTerm::Surface),
                &mut memory,
            )
            .unwrap();
        assert_eq!(live.get(), protected - row_bytes);
        assert_eq!(memory.admitted_bytes(), live.get());
        assert_eq!(input.rows_from(0).take(3).collect::<Vec<_>>(), original);
        assert_eq!(input.rows_from(0).last().unwrap().0.index(), 4);
        let lookup_bytes = input.by_key.storage_bytes().unwrap();
        let held = live.get();
        memory.admission_mut().maximum = held;
        for _ in 0..2 {
            assert!(matches!(
                input.insert_terms_in(
                    ["fresh", "other", "o"].map(DictionaryTerm::Surface),
                    &mut memory
                ),
                Err(DictionaryError::Storage(StorageError::AdmissionFailed))
            ));
            assert_eq!(input.row_count(), 5);
            assert_eq!(input.term_id("other"), None);
            assert_eq!(live.get(), held);
        }
        memory.admission_mut().maximum = usize::MAX;
        memory.admission_mut().refuse_shrink = true;
        assert!(matches!(
            input.insert_terms_in(
                ["fresh", "other", "o"].map(DictionaryTerm::Surface),
                &mut memory
            ),
            Err(DictionaryError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(input.lookup_retired_bytes, lookup_bytes);
        let protected = live.get();
        assert!(matches!(
            input.insert_terms_in(
                ["fresh", "other", "o"].map(DictionaryTerm::Surface),
                &mut memory
            ),
            Err(DictionaryError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(live.get(), protected);
        assert_eq!(input.row_count(), 5);
        assert_eq!(input.term_id("other"), None);
        memory.admission_mut().refuse_shrink = false;
        input
            .insert_terms_in(
                ["fresh", "other", "o"].map(DictionaryTerm::Surface),
                &mut memory,
            )
            .unwrap();
        assert_eq!(live.get(), protected - lookup_bytes);
        assert_eq!(input.lookup_retired_bytes, 0);
        assert_eq!(input.rows_from(0).last().unwrap().0.index(), 5);
        input.prepare_maintenance_indexes_in(&mut memory).unwrap();
        assert!(input.order.is_supplemental());
        let lookup_bytes = input.by_key.storage_bytes().unwrap();
        memory.admission_mut().maximum = live.get();
        let mut directory_refused = false;
        loop {
            match input.insert_terms_in(
                ["fresh", "third", "o"].map(DictionaryTerm::Surface),
                &mut memory,
            ) {
                Ok(Some(_)) => break,
                Err(DictionaryError::Storage(StorageError::AdmissionFailed)) => {
                    directory_refused |= input.by_key.storage_bytes().unwrap() > lookup_bytes;
                    assert_eq!(input.row_count(), 6);
                    assert_eq!(input.term_id("third"), None);
                    let held = live.get();
                    assert!(matches!(
                        input.insert_terms_in(
                            ["fresh", "third", "o"].map(DictionaryTerm::Surface),
                            &mut memory
                        ),
                        Err(DictionaryError::Storage(StorageError::AdmissionFailed))
                    ));
                    assert_eq!(live.get(), held);
                    assert_eq!(memory.admitted_bytes(), held);
                    let requested = memory.admission_mut().requested;
                    assert!(requested > memory.admission_mut().maximum);
                    memory.admission_mut().maximum = requested;
                }
                other => panic!("unexpected original directory admission: {other:?}"),
            }
        }
        assert!(
            directory_refused,
            "directory factories refuse after original lookup growth"
        );
        assert_eq!(input.rows_from(0).last().unwrap().0.index(), 6);
        memory.admission_mut().maximum = usize::MAX;
        memory.admission_mut().refuse_shrink = true;
        for predicate in ["third", "other"] {
            let ids = ["fresh", predicate, "o"].map(|surface| input.term_id(surface).unwrap());
            input.remove_ids(ids).unwrap().unwrap();
            input.retire_empty_partition(ids).unwrap();
        }
        let held = live.get();
        for _ in 0..2 {
            assert_eq!(
                input.flush_metadata_retirement_in(&mut memory),
                Err(StorageError::AdmissionFailed)
            );
            assert_eq!(live.get(), held);
            assert_eq!(memory.admitted_bytes(), held);
            assert_eq!(input.row_count(), 5);
        }
        memory.admission_mut().refuse_shrink = false;
        input.flush_metadata_retirement_in(&mut memory).unwrap();
        assert!(live.get() < held);
        assert_eq!(input.rows_from(0).take(3).collect::<Vec<_>>(), original);
        let bytes = memory.admitted_bytes();
        drop(input);
        memory.release_bytes(bytes).unwrap();
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn maintained_partition_order_uses_original_integer_keys_without_text() {
        let dictionary = crate::dictionary::IntegerDictionary::<u64>::default();
        let mut store = RelationStore::<_, 4>::with_dictionary(dictionary);
        let high = u64::from(u32::MAX) + 100;
        store
            .insert_ids([high, high + 3, high + 1, high + 5])
            .unwrap();
        store.prepare_maintenance_indexes().unwrap();
        store
            .insert_ids([high, high + 2, high + 1, high + 6])
            .unwrap();
        store
            .insert_ids([high, high + 3, high + 1, high + 4])
            .unwrap();
        let keys: Vec<_> = store
            .partitions(None, None)
            .map(|p| (p.predicate(), p.graph()))
            .collect();
        assert_eq!(
            keys,
            [
                (high + 2, high + 6),
                (high + 3, high + 4),
                (high + 3, high + 5)
            ]
        );
    }

    #[test]
    fn original_key_failure_does_not_publish_a_partial_directory() {
        use crate::test_support::{HostDictionary, SourceLost};
        let mut dictionary = HostDictionary::default();
        dictionary
            .surfaces
            .extend([(1, "s".into()), (2, "p".into()), (3, "o".into())]);
        let mut store = RelationStore::<_, 3>::with_dictionary(dictionary);
        store.insert_ids([1, 2, 3]).unwrap();
        store.interner.surfaces.remove(&2);
        assert!(matches!(
            store.prepare_maintenance_indexes(),
            Err(crate::dictionary::DictionaryError::Source(SourceLost(2)))
        ));
        assert!(!store.order.is_sparse());
        assert_eq!(store.row_count(), 1);
        assert_eq!(store.rows_from(0).next().unwrap().1, [1, 2, 3]);
        store.interner.surfaces.insert(2, "p".into());
        store.prepare_maintenance_indexes().unwrap();
        assert!(store.order.is_sparse());
    }

    #[test]
    fn retired_partition_slots_are_reused_without_reusing_original_ids() {
        let mut store = RelationStore::new();
        let sentinel = store
            .insert("anchor", "middle", "value", "g")
            .unwrap()
            .unwrap()
            .2;
        let sentinel_partition = store.partitions(None, None).next().unwrap().id();
        let mut terms = Vec::new();
        for index in 0..1024 {
            terms.push(
                store
                    .intern_terms(
                        [
                            "subject",
                            &format!("p{index:04}"),
                            "object",
                            &format!("g{index:04}"),
                        ]
                        .map(crate::dictionary::DictionaryTerm::Surface),
                    )
                    .unwrap(),
            );
        }
        store.prepare_maintenance_indexes().unwrap();
        let mut previous = sentinel_partition;
        for ids in terms {
            let row = store.insert_ids(ids).unwrap().unwrap().2;
            let partition = store.partition_for(&ids.map(Some)).unwrap().id();
            assert!(
                partition > previous,
                "retired identities never alias a replacement"
            );
            previous = partition;
            let withdrawn = store.withdraw_ids(ids).unwrap().unwrap();
            assert_eq!(withdrawn.id(), row);
            store.retire_empty_partition(ids).unwrap();
            assert_eq!(store.keys.len(), 2, "physical slots follow concurrent keys");
            assert_eq!(store.by_key.len(), 1);
            assert_eq!(store.free_partitions.len(), 1);
            assert_eq!(store.retained_rows(), 1);
            assert_eq!(store.rows_from(0).next().unwrap().0, sentinel);
            assert_eq!(
                store.partitions(None, None).next().unwrap().id(),
                sentinel_partition
            );
        }
        for (predicate, graph) in [("z", "b"), ("a", "z"), ("z", "a")] {
            store.insert("s", predicate, "o", graph).unwrap();
        }
        let keys: Vec<_> = store
            .partitions(None, None)
            .map(|p| {
                (
                    store.interner.resolve(p.predicate()),
                    store.interner.resolve(p.graph()),
                )
            })
            .collect();
        assert_eq!(keys, [("a", "z"), ("middle", "g"), ("z", "a"), ("z", "b")]);
        let graphs: Vec<_> = store
            .partitions(store.term_id("z"), None)
            .map(|p| store.interner.resolve(p.graph()))
            .collect();
        assert_eq!(graphs, ["a", "b"]);
    }

    #[test]
    fn sparse_row_log_keeps_wide_coordinates_and_consumes_original_tokens() {
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for row in [["a", "edge", "b"], ["a", "edge", "c"]] {
            input.insert_row(row).unwrap();
        }
        input.prepare_maintenance_indexes().unwrap();
        let wide = u64::from(u32::MAX) + 71;
        let RowLog::Sparse { next, .. } = &mut input.row_log else {
            panic!("maintenance owns sparse metadata");
        };
        *next = wide;
        let original = input.insert_row(["b", "edge", "c"]).unwrap().unwrap().2;
        assert_eq!(original.index(), wide);
        let ids = ["b", "edge", "c"].map(|s| input.term_id(s).unwrap());
        let token = input.withdraw_ids(ids).unwrap().unwrap();
        assert_eq!(token.id(), original);
        assert_eq!(input.retained_rows(), 2);
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        input.admit_restore_in(&token, &mut memory).unwrap();
        input.restore_row_in(token, &mut memory);
        assert_eq!(input.rows_from(wide).next().unwrap(), (original, ids));
        assert_eq!(input.row_slots(), wide + 1);
        let atom = |s: &str, p: &str, o: &str| {
            ClauseAtom::triple(
                ClauseTerm::var(s),
                ClauseTerm::literal(p),
                ClauseTerm::var(o),
            )
        };
        let program = compile(vec![
            DlClause::datalog(atom("x", "path", "y"), vec![atom("x", "edge", "y")]),
            DlClause::datalog(
                atom("x", "path", "z"),
                vec![atom("x", "path", "y"), atom("y", "edge", "z")],
            ),
        ])
        .unwrap();
        let options = EvalOptions::default();
        let mut model = Maintained::<u64, Resident, _, 3>::new(
            &program,
            input,
            &NoGuards,
            SessionContext {
                snapshot: Digest32::new([0xc1; 32]),
                guards: Digest32::new([0xc2; 32]),
                evidence: Digest32::new([0xc3; 32]),
            },
            options,
        )
        .unwrap();
        assert!(matches!(
            model.resume(options, None),
            Ok(ResumeProgress::Complete)
        ));
        let survivor = ["a", "path", "c"].map(|s| model.facts().unwrap().term_id(s).unwrap());
        let survivor_id = model
            .facts()
            .unwrap()
            .rows_from(wide)
            .find(|(_, row)| *row == survivor)
            .unwrap()
            .0;
        let extent = model.facts().unwrap().row_slots();
        assert_eq!(extent, wide + 4);
        for (position, weight) in [-1, 1].into_iter().enumerate() {
            model
                .apply(ChangeBatch::new(position as u64, [(ids, weight)], Resident).unwrap())
                .unwrap();
            assert!(matches!(
                model.resume(options, None),
                Ok(ResumeProgress::Complete)
            ));
            assert_eq!(
                model.facts().unwrap().retained_rows(),
                if weight < 0 { 4 } else { 6 }
            );
            assert_eq!(model.model_delta().unwrap().unwrap().changes().len(), 2);
            assert_eq!(
                model
                    .facts()
                    .unwrap()
                    .rows_from(wide)
                    .find(|(_, row)| *row == survivor)
                    .unwrap()
                    .0,
                survivor_id
            );
        }
        assert_eq!(model.facts().unwrap().row_slots(), extent + 2);
        let replacement = model
            .facts()
            .unwrap()
            .rows_from(extent)
            .find(|(_, row)| *row == ids)
            .unwrap()
            .0;
        assert_ne!(
            replacement, original,
            "final deletion never aliases the old coordinate"
        );
        let cold = evaluate_guarded(
            &program,
            model.facts().unwrap().clone(),
            &NoGuards,
            &options,
            None,
        )
        .unwrap();
        assert_eq!(
            cold.facts().facts_sorted(),
            model.facts().unwrap().facts_sorted()
        );
    }
}
