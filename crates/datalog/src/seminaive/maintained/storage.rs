// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The original supplemental admission, destroyed after its model payloads.

use purrdf_lex::allocation::{Admission, Memory};

pub(super) struct Storage<A: Admission> {
    admission: A,
    live: usize,
}

impl<A: Admission> Storage<A> {
    pub(super) const fn new(admission: A) -> Self {
        Self { admission, live: 0 }
    }

    pub(super) const fn live(&self) -> usize {
        self.live
    }

    pub(super) fn with_memory<T>(&mut self, body: impl FnOnce(&mut Memory<'_, A>) -> T) -> T {
        let mut memory = Memory::resume(&mut self.admission, self.live);
        let result = body(&mut memory);
        self.live = memory.admitted_bytes();
        result
    }
}

impl<A: Admission> Drop for Storage<A> {
    fn drop(&mut self) {
        if self.live != 0 {
            self.admission
                .resize(0)
                .expect("the original model admission must release its destroyed payloads");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Storage;
    use purrdf_lex::allocation::{Admission, StorageError};
    use std::cell::Cell;

    struct Grant<'a> {
        live: &'a Cell<usize>,
        maximum: usize,
        grows: usize,
        maximum_grows: usize,
        refuse_shrink: bool,
        destroyed: Option<&'a Cell<usize>>,
    }

    impl Admission for Grant<'_> {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live > self.maximum {
                return Err(StorageError::AdmissionFailed);
            }
            if self.refuse_shrink && live < self.live.get() {
                return Err(StorageError::AdmissionFailed);
            }
            if live > self.live.get() {
                if self.grows == self.maximum_grows {
                    return Err(StorageError::AdmissionFailed);
                }
                self.grows += 1;
            }
            if live == 0
                && let Some(destroyed) = self.destroyed
            {
                assert_eq!(
                    destroyed.get(),
                    3,
                    "payloads die before their original grant shrinks"
                );
            }
            self.live.set(live);
            Ok(())
        }
    }

    struct Payload<'a>(&'a Cell<usize>);
    impl Drop for Payload<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn original_borrowed_grant_outlives_its_last_payload() {
        struct Owner<'a> {
            payloads: Vec<Payload<'a>>,
            storage: Storage<&'a mut Grant<'a>>,
        }
        let destroyed = Cell::new(0);
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: Some(&destroyed),
        };
        let mut owner = Owner {
            payloads: Vec::new(),
            storage: Storage::new(&mut grant),
        };
        owner
            .storage
            .with_memory(|memory| memory.reserve(&mut owner.payloads, 3))
            .unwrap();
        owner.payloads.extend((0..3).map(|_| Payload(&destroyed)));
        assert_eq!(owner.storage.live(), live.get());
        assert!(live.get() >= 3 * size_of::<Payload<'_>>());
        drop(owner);
        assert_eq!(live.get(), 0);
        assert_eq!(destroyed.get(), 3);
    }

    #[test]
    fn initial_model_factories_use_the_supplied_grant_before_construction() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause},
            guard::NoGuards,
            seminaive::{EvalError, EvalOptions, Maintained, SessionContext, compile},
            store::{RelationStore, TermInterner},
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;
        let atom = |predicate| {
            ClauseAtom::triple(
                ClauseTerm::var("s"),
                ClauseTerm::literal(predicate),
                ClauseTerm::var("o"),
            )
        };
        let program = compile(vec![DlClause::datalog(atom("out"), vec![atom("in")])]).unwrap();
        let input = || {
            let mut store =
                RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
            store.insert_row(["s", "in", "o"]).unwrap();
            store
        };
        let context = SessionContext {
            snapshot: Digest32::new([1; 32]),
            guards: Digest32::new([2; 32]),
            evidence: Digest32::new([3; 32]),
        };
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: 0,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: None,
        };
        let result = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program,
            input(),
            &NoGuards,
            context,
            EvalOptions::default(),
            &mut grant,
        );
        assert!(matches!(
            result,
            Err(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        drop(result);
        assert_eq!(live.get(), 0);
        grant.maximum = usize::MAX;
        let model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program,
            input(),
            &NoGuards,
            context,
            EvalOptions::default(),
            &mut grant,
        )
        .unwrap();
        assert_eq!(model.storage.live(), live.get());
        let weights_bytes = purrdf_core::hash::hash_table_allocation_bound::<(
            [crate::id::TermId; 3],
            i64,
        )>(model.weights.capacity())
        .unwrap();
        assert_eq!(
            live.get(),
            weights_bytes
                + model.levels.capacity() * size_of::<usize>()
                + model.impacts.predicates.capacity() * size_of::<String>()
                + model.impacts.edges.capacity() * size_of::<crate::plan::DependencyEdge>()
                + model
                    .impacts
                    .predicates
                    .iter()
                    .map(String::capacity)
                    .sum::<usize>()
        );
        let initial_bytes = live.get();
        let initial_grows = model.storage.admission.grows;
        drop(model);
        assert_eq!(live.get(), 0);
        // Let the first map succeed, then refuse the next actual vector layout.
        grant.maximum = weights_bytes;
        let result = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program,
            input(),
            &NoGuards,
            context,
            EvalOptions::default(),
            &mut grant,
        );
        assert!(matches!(
            result,
            Err(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        drop(result);
        assert_eq!(live.get(), 0);
        // Refuse each actual initial factory, including dependency scratch and
        // temporary surfaces. Every failed local graph dies before its refund.
        for allowed in 0..initial_grows {
            grant.maximum = usize::MAX;
            grant.grows = 0;
            grant.maximum_grows = allowed;
            let result = Maintained::<u64, Resident, _, 3, _>::new_in(
                &program,
                input(),
                &NoGuards,
                context,
                EvalOptions::default(),
                &mut grant,
            );
            assert!(matches!(
                result,
                Err(EvalError::Storage(StorageError::AdmissionFailed))
            ));
            drop(result);
            assert_eq!(live.get(), 0);
        }
        grant.maximum = usize::MAX;
        grant.grows = 0;
        grant.maximum_grows = usize::MAX;
        let model = Maintained::<u64, Resident, _, 3, _>::new_reclaiming_in(
            &program,
            input().into_reclaiming().unwrap(),
            &NoGuards,
            context,
            EvalOptions::default(),
            &mut grant,
        )
        .unwrap();
        let uses = model
            .session
            .state
            .as_ref()
            .unwrap()
            .native
            .as_ref()
            .unwrap()
            .term_uses
            .as_ref()
            .unwrap();
        assert_eq!(uses.counts.len(), 3);
        assert_eq!(
            live.get(),
            initial_bytes + uses.counts_bytes + uses.zero.retained
        );
        assert_eq!(model.storage.live(), live.get());
        drop(model);
        assert_eq!(live.get(), 0);
        grant.maximum = usize::MAX;
        grant.grows = 0;
        grant.maximum_grows = initial_grows;
        let result = Maintained::<u64, Resident, _, 3, _>::new_reclaiming_in(
            &program,
            input().into_reclaiming().unwrap(),
            &NoGuards,
            context,
            EvalOptions::default(),
            &mut grant,
        );
        assert!(matches!(
            result,
            Err(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        drop(result);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn native_publication_and_retirement_preserve_the_original_grant_at_refusal() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause},
            guard::NoGuards,
            maintenance::ChangeBatch,
            seminaive::{
                EvalError, EvalOptions, Maintained, ResumeProgress, SessionContext, compile,
            },
            store::{RelationStore, TermInterner},
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;
        let atom = |predicate| {
            ClauseAtom::triple(
                ClauseTerm::var("s"),
                ClauseTerm::literal(predicate),
                ClauseTerm::var("o"),
            )
        };
        let program = compile(vec![DlClause::datalog(atom("out"), vec![atom("in")])]).unwrap();
        let context = SessionContext {
            snapshot: Digest32::new([7; 32]),
            guards: Digest32::new([8; 32]),
            evidence: Digest32::new([9; 32]),
        };
        let options = EvalOptions::default();
        for assert_derived in [false, true] {
            let live = Cell::new(0);
            let mut grant = Grant {
                live: &live,
                maximum: usize::MAX,
                grows: 0,
                maximum_grows: usize::MAX,
                refuse_shrink: false,
                destroyed: None,
            };
            let mut input =
                RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
            input.insert_row(["s", "in", "o"]).unwrap();
            let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
                &program, input, &NoGuards, context, options, &mut grant,
            )
            .unwrap();
            model.storage.admission.maximum_grows = model.storage.admission.grows;
            let mut pauses = 0;
            loop {
                match model.resume(options, None).unwrap() {
                    ResumeProgress::Complete => break,
                    ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed)) => {
                        pauses += 1;
                        let state = model.session.state.as_ref().unwrap();
                        assert_eq!(
                            state.rel.row_count(),
                            state.native.as_ref().unwrap().nodes.len() + 1,
                            "a native row is never published without its admitted proof"
                        );
                        assert_eq!(model.storage.live(), live.get());
                        model.storage.admission.maximum_grows += 1;
                    }
                    other @ ResumeProgress::Paused(_) => {
                        panic!("unexpected publication refusal: {other:?}")
                    }
                }
            }
            assert!(pauses > 0);
            assert_eq!(model.facts().unwrap().row_count(), 2);
            assert_eq!(model.witnesses().unwrap().len(), 1);
            let terms = ["s", if assert_derived { "out" } else { "in" }, "o"]
                .map(|surface| model.facts().unwrap().term_id(surface).unwrap());
            model.storage.admission.maximum_grows = usize::MAX;
            model.storage.admission.refuse_shrink = true;
            model
                .apply(
                    ChangeBatch::new(1, [(terms, if assert_derived { 1 } else { -1 })], Resident)
                        .unwrap(),
                )
                .unwrap();
            assert!(matches!(
                model.resume(options, None).unwrap(),
                ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
            ));
            assert_eq!(model.pending_watermark(), Some(&1));
            assert_eq!(model.applied_watermark(), None);
            let native = model
                .session
                .state
                .as_ref()
                .unwrap()
                .native
                .as_ref()
                .unwrap();
            assert!(native.retired_bytes > 0);
            let retired = native.retired_bytes;
            let work = model.transaction_work();
            let cursors = model
                .pending
                .as_ref()
                .map(|pending| (pending.deletion, pending.input));
            assert!(matches!(
                model.resume(options, None).unwrap(),
                ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
            ));
            assert_eq!(model.transaction_work(), work);
            assert_eq!(
                model
                    .pending
                    .as_ref()
                    .map(|pending| (pending.deletion, pending.input)),
                cursors
            );
            assert_eq!(
                model
                    .session
                    .state
                    .as_ref()
                    .unwrap()
                    .native
                    .as_ref()
                    .unwrap()
                    .retired_bytes,
                retired
            );
            assert_eq!(model.pending_watermark(), Some(&1));
            assert_eq!(model.storage.live(), live.get());
            model.storage.admission.refuse_shrink = false;
            assert_eq!(
                model.resume(options, None).unwrap(),
                ResumeProgress::Complete
            );
            assert_eq!(model.applied_watermark(), Some(&1));
            assert_eq!(model.pending_watermark(), None);
            assert_eq!(
                model.facts().unwrap().row_count(),
                if assert_derived { 2 } else { 0 }
            );
            assert_eq!(model.witnesses().unwrap().len(), 0);
            assert_eq!(
                model
                    .session
                    .state
                    .as_ref()
                    .unwrap()
                    .native
                    .as_ref()
                    .unwrap()
                    .retired_bytes,
                0
            );
            assert_eq!(model.storage.live(), live.get());
            drop(model);
            assert_eq!(live.get(), 0);
        }
    }
}
