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

    fn finish_input_events<A: Admission>(
        model: &mut crate::seminaive::Maintained<
            '_,
            u64,
            purrdf_lex::allocation::Resident,
            crate::store::TermInterner,
            3,
            A,
        >,
    ) {
        let mut limit = 0;
        loop {
            let progress =
                model.resume_until(crate::seminaive::EvalOptions::default(), limit, None);
            let pending = model.pending.as_ref().unwrap();
            if pending.input == pending.changes.len() && pending.input_state.is_none() {
                assert_eq!(pending.retired_bytes, 0);
                return;
            }
            match progress {
                Err(super::super::MaintenanceError::WorkLimit { needed, .. }) => limit = needed,
                other => panic!("expected an original input-event cut: {other:?}"),
            }
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
                + model.session.state.as_ref().unwrap().native_bytes
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
                        assert!(
                            pauses < 128,
                            "the small original model must retain factory progress"
                        );
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
            model
                .apply(
                    ChangeBatch::new(1, [(terms, if assert_derived { 1 } else { -1 })], Resident)
                        .unwrap(),
                )
                .unwrap();
            let limit = model.transaction_work();
            assert!(matches!(
                model.resume_until(options, limit, None),
                Err(super::super::MaintenanceError::WorkLimit { .. })
            ));
            model.storage.admission.refuse_shrink = true;
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

    #[test]
    fn deferred_publication_keeps_completed_native_output_across_growth_refusals() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause},
            dictionary::DictionaryTerm,
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
            snapshot: Digest32::new([11; 32]),
            guards: Digest32::new([12; 32]),
            evidence: Digest32::new([13; 32]),
        };
        let options = EvalOptions::default();
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: None,
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        input.insert_row(["s", "in", "o"]).unwrap();
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program, input, &NoGuards, context, options, &mut grant,
        )
        .unwrap();
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        let out = model.facts().unwrap().term_id("out").unwrap();
        let fresh = model
            .admit_terms(["x", "in", "o"].map(DictionaryTerm::Surface))
            .unwrap();
        let separate = model
            .admit_terms(["x", "unrelated", "o"].map(DictionaryTerm::Surface))
            .unwrap();
        model
            .apply(ChangeBatch::new(7, [(fresh, 1), (separate, 1)], Resident).unwrap())
            .unwrap();
        model.storage.admission.maximum_grows = model.storage.admission.grows;
        let mut saw_retained_native_frame = false;
        let mut saw_pending_publication = false;
        let mut saw_new_partition_refusal = false;
        let mut refusals = 0;
        loop {
            match model.resume(options, None).unwrap() {
                ResumeProgress::Complete => break,
                ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed)) => {
                    refusals += 1;
                    let pending = model.pending.as_ref().unwrap();
                    saw_retained_native_frame |= pending.active.is_some();
                    saw_pending_publication |= pending.publication.is_some();
                    saw_new_partition_refusal |= pending
                        .changes
                        .get(pending.input)
                        .is_some_and(|(terms, _)| *terms == separate);
                    let state = model.session.state.as_ref().unwrap();
                    assert_eq!(
                        state.rel.rows_for_predicate(out).count(),
                        state.native.as_ref().unwrap().nodes.len()
                    );
                    let joins = model.budget().join_steps();
                    let held = live.get();
                    assert_eq!(model.pending_watermark(), Some(&7));
                    assert_eq!(model.applied_watermark(), None);
                    // Refusing the identical factory again cannot replay a native join.
                    assert!(matches!(
                        model.resume(options, None).unwrap(),
                        ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
                    ));
                    assert_eq!(model.budget().join_steps(), joins);
                    assert_eq!(model.storage.live(), held);
                    assert_eq!(live.get(), held);
                    model.storage.admission.maximum_grows += 1;
                }
                other @ ResumeProgress::Paused(_) => {
                    panic!("unexpected deferred refusal: {other:?}")
                }
            }
        }
        assert!(
            refusals > 0
                && saw_retained_native_frame
                && saw_pending_publication
                && saw_new_partition_refusal
        );
        assert_eq!(model.applied_watermark(), Some(&7));
        assert_eq!(model.facts().unwrap().row_count(), 5);
        assert_eq!(model.witnesses().unwrap().len(), 2);
        let delta = model.model_delta().unwrap().unwrap();
        assert_eq!(delta.changes().len(), 3);
        assert!(delta.changes().iter().all(|(_, weight)| *weight == 1));
        assert_eq!(model.storage.live(), live.get());
        drop(model);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn replaced_native_table_keeps_its_refused_old_charge_pending() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause},
            dictionary::DictionaryTerm,
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
            snapshot: Digest32::new([21; 32]),
            guards: Digest32::new([22; 32]),
            evidence: Digest32::new([23; 32]),
        };
        let options = EvalOptions::default();
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: None,
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for subject in ["a", "b", "c"] {
            input.insert_row([subject, "in", "o"]).unwrap();
        }
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program, input, &NoGuards, context, options, &mut grant,
        )
        .unwrap();
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
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
                .nodes
                .capacity(),
            3
        );
        let fresh = model
            .admit_terms(["d", "in", "o"].map(DictionaryTerm::Surface))
            .unwrap();
        model
            .apply(ChangeBatch::new(9, [(fresh, 1)], Resident).unwrap())
            .unwrap();
        finish_input_events(&mut model);
        model.storage.admission.refuse_shrink = true;
        let first = model.resume(options, None);
        let grows = model.storage.admission.grows;
        let held = live.get();
        let joins = model.budget().join_steps();
        let second = model.resume(options, None);
        model.storage.admission.refuse_shrink = false;
        assert!(matches!(
            first.unwrap(),
            ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(matches!(
            second.unwrap(),
            ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(
            model.storage.admission.grows, grows,
            "a refused old-table release is retried before another table grows"
        );
        assert_eq!(live.get(), held);
        assert_eq!(model.budget().join_steps(), joins);
        assert_eq!(model.pending_watermark(), Some(&9));
        assert_eq!(model.applied_watermark(), None);
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        assert_eq!(model.applied_watermark(), Some(&9));
        assert_eq!(model.facts().unwrap().row_count(), 8);
        assert_eq!(model.witnesses().unwrap().len(), 4);
        assert_eq!(model.storage.live(), live.get());
        drop(model);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn replaced_winner_array_and_completed_publication_retry_their_original_releases() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct},
            dictionary::DictionaryTerm,
            guard::NoGuards,
            maintenance::ChangeBatch,
            seminaive::{
                EvalError, EvalOptions, Maintained, ResumeProgress, SessionContext, compile,
            },
            store::{RelationStore, TermInterner},
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;
        let atom = |predicate: &str| {
            ClauseAtom::triple(
                ClauseTerm::var("s"),
                ClauseTerm::literal(predicate),
                ClauseTerm::var("o"),
            )
        };
        let heads = (0..6).map(|index| atom(&format!("out{index}"))).collect();
        let program = compile(vec![DlClause::new(
            vec![HeadDisjunct::new(heads)],
            Vec::new(),
            vec![atom("in")],
        )])
        .unwrap();
        let context = SessionContext {
            snapshot: Digest32::new([31; 32]),
            guards: Digest32::new([32; 32]),
            evidence: Digest32::new([33; 32]),
        };
        let options = EvalOptions::default();
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: None,
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        input.insert_row(["s", "in", "o"]).unwrap();
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program, input, &NoGuards, context, options, &mut grant,
        )
        .unwrap();
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        let fresh = model
            .admit_terms(["x", "in", "o"].map(DictionaryTerm::Surface))
            .unwrap();
        model
            .apply(ChangeBatch::new(10, [(fresh, 1)], Resident).unwrap())
            .unwrap();
        finish_input_events(&mut model);
        model.storage.admission.refuse_shrink = true;
        let first = model.resume(options, None);
        let pending = model.pending.as_ref().unwrap();
        let replaced_array_retired = pending.retired_bytes;
        let preparing = pending.publication.is_some();
        let joins = model.budget().join_steps();
        let second = model.resume(options, None);
        model.storage.admission.refuse_shrink = false;
        assert!(matches!(
            first.unwrap(),
            ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(matches!(
            second.unwrap(),
            ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(preparing && replaced_array_retired > 0);
        assert_eq!(
            model.pending.as_ref().unwrap().retired_bytes,
            replaced_array_retired
        );
        assert_eq!(model.budget().join_steps(), joins);
        assert_eq!(model.pending_watermark(), Some(&10));
        assert_eq!(model.applied_watermark(), None);
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        assert_eq!(model.facts().unwrap().row_count(), 14);
        assert_eq!(model.witnesses().unwrap().len(), 12);

        let fresh = model
            .admit_terms(["y", "in", "o"].map(DictionaryTerm::Surface))
            .unwrap();
        model
            .apply(ChangeBatch::new(11, [(fresh, 1)], Resident).unwrap())
            .unwrap();
        // Step through the original work allowance to the last native winner;
        // earlier replacement-table/vector refunds are permitted normally.
        let mut limit = 0;
        loop {
            let progress = model.resume_until(options, limit, None);
            if model
                .session
                .state
                .as_ref()
                .unwrap()
                .native
                .as_ref()
                .unwrap()
                .nodes
                .len()
                == 17
            {
                break;
            }
            match progress {
                Err(super::super::MaintenanceError::WorkLimit { needed, .. }) => limit = needed,
                other => panic!("expected a retained original winner cut: {other:?}"),
            }
        }
        model.storage.admission.refuse_shrink = true;
        let first = model.resume(options, None);
        let pending = model.pending.as_ref().unwrap();
        let completed_arrays_retired = pending.retired_bytes;
        let completed = pending.publication.is_none();
        let joins = model.budget().join_steps();
        let work = model.transaction_work();
        let second = model.resume(options, None);
        model.storage.admission.refuse_shrink = false;
        assert!(matches!(
            first.unwrap(),
            ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(matches!(
            second.unwrap(),
            ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
        ));
        assert!(completed && completed_arrays_retired > 0);
        assert_eq!(
            model.pending.as_ref().unwrap().retired_bytes,
            completed_arrays_retired
        );
        assert_eq!(model.budget().join_steps(), joins);
        assert_eq!(model.transaction_work(), work);
        assert_eq!(model.pending_watermark(), Some(&11));
        assert_eq!(model.applied_watermark(), Some(&10));
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        assert_eq!(model.facts().unwrap().row_count(), 21);
        assert_eq!(model.applied_watermark(), Some(&11));
        assert_eq!(model.storage.live(), live.get());
        drop(model);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn preparation_task_spills_use_the_original_grant_before_mutation() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct},
            guard::NoGuards,
            maintenance::ChangeBatch,
            seminaive::{
                EvalError, EvalOptions, Maintained, ResumeProgress, SessionContext, compile,
            },
            store::{RelationStore, TermInterner},
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;
        let atom = |predicate: &str| {
            ClauseAtom::triple(
                ClauseTerm::var("s"),
                ClauseTerm::literal(predicate),
                ClauseTerm::var("o"),
            )
        };
        let heads = (0..12).map(|index| atom(&format!("out{index}"))).collect();
        let program = compile(vec![DlClause::new(
            vec![HeadDisjunct::new(heads)],
            Vec::new(),
            vec![atom("in")],
        )])
        .unwrap();
        let context = SessionContext {
            snapshot: Digest32::new([41; 32]),
            guards: Digest32::new([42; 32]),
            evidence: Digest32::new([43; 32]),
        };
        let options = EvalOptions::default();
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: None,
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        input.insert_row(["s", "in", "o"]).unwrap();
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program, input, &NoGuards, context, options, &mut grant,
        )
        .unwrap();
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        let remove =
            ["s", "in", "o"].map(|surface| model.facts().unwrap().term_id(surface).unwrap());
        model.storage.admission.maximum_grows = model.storage.admission.grows;
        model
            .begin(ChangeBatch::new(50, [(remove, -1)], Resident).unwrap())
            .unwrap();
        let mut preparation_refusals = 0;
        let mut task_spill = false;
        loop {
            let progress = model.resume(options, None).unwrap();
            let tasks = model
                .preparing
                .as_ref()
                .map(|frame| &frame.tasks)
                .or_else(|| model.pending.as_ref().map(|pending| &pending.tasks));
            task_spill |=
                tasks.is_some_and(|tasks| tasks.iter().any(|(_, work)| work.heap_capacity() > 0));
            match progress {
                ResumeProgress::Complete => break,
                ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed)) => {
                    assert_eq!(model.pending_watermark(), Some(&50));
                    assert_eq!(model.applied_watermark(), None);
                    if model.preparing.is_some() {
                        preparation_refusals += 1;
                        assert_eq!(
                            model.session.state.as_ref().unwrap().rel.row_count(),
                            13,
                            "task admission precedes every native model withdrawal"
                        );
                        let joins = model.budget().join_steps();
                        let held = live.get();
                        assert!(matches!(
                            model.resume(options, None).unwrap(),
                            ResumeProgress::Paused(EvalError::Storage(
                                StorageError::AdmissionFailed
                            ))
                        ));
                        assert_eq!(model.budget().join_steps(), joins);
                        assert_eq!(model.storage.live(), held);
                        assert_eq!(live.get(), held);
                    }
                    model.storage.admission.maximum_grows += 1;
                }
                other @ ResumeProgress::Paused(_) => panic!("unexpected task refusal: {other:?}"),
            }
        }
        assert!(
            preparation_refusals > 2 && task_spill,
            "the original single-level task list exceeds its inline slots"
        );
        assert_eq!(model.applied_watermark(), Some(&50));
        assert_eq!(model.facts().unwrap().row_count(), 0);
        assert_eq!(model.witnesses().unwrap().len(), 0);
        assert_eq!(model.model_delta().unwrap().unwrap().changes().len(), 13);
        assert_eq!(model.storage.live(), live.get());
        drop(model);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn original_sealed_object_permutations_are_constructed_under_the_supplied_grant() {
        use crate::{
            clause::DlClause,
            guard::NoGuards,
            seminaive::{
                EvalError, EvalOptions, Maintained, ResumeProgress, SessionContext, compile,
            },
            store::{Bound, RelationStore, TermInterner},
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;
        struct Observed<'a> {
            grant: Grant<'a>,
            last_growth: usize,
        }
        impl Admission for Observed<'_> {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                let previous = self.grant.live.get();
                self.grant.resize(bytes)?;
                if bytes > previous {
                    self.last_growth = bytes - previous;
                }
                Ok(())
            }
        }
        let live = Cell::new(0);
        let mut grant = Observed {
            grant: Grant {
                live: &live,
                maximum: usize::MAX,
                grows: 0,
                maximum_grows: usize::MAX,
                refuse_shrink: false,
                destroyed: None,
            },
            last_growth: 0,
        };
        let program = compile(Vec::<DlClause<3>>::new()).unwrap();
        let options = EvalOptions::default();
        let context = SessionContext {
            snapshot: Digest32::new([71; 32]),
            guards: Digest32::new([72; 32]),
            evidence: Digest32::new([73; 32]),
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for index in 0..64 {
            input
                .insert_row([&format!("subject-{index}"), "in", "o"])
                .unwrap();
        }
        let original = input.rows_from(0).collect::<Vec<_>>();
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program, input, &NoGuards, context, options, &mut grant,
        )
        .unwrap();
        model.storage.admission.grant.maximum_grows = model.storage.admission.grant.grows;
        let mut refusals = 0;
        loop {
            match model.resume(options, None).unwrap() {
                ResumeProgress::Complete => break,
                ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed)) => {
                    refusals += 1;
                    assert!(
                        refusals < 128,
                        "the original finite index retains admitted progress"
                    );
                    assert_eq!(
                        model
                            .session
                            .state
                            .as_ref()
                            .unwrap()
                            .rel
                            .rows_from(0)
                            .collect::<Vec<_>>(),
                        original
                    );
                    let held = live.get();
                    assert!(matches!(
                        model.resume(options, None).unwrap(),
                        ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed))
                    ));
                    assert_eq!(live.get(), held);
                    assert_eq!(model.storage.live(), held);
                    model.storage.admission.grant.maximum_grows += 1;
                }
                other @ ResumeProgress::Paused(_) => {
                    panic!("unexpected original index refusal: {other:?}")
                }
            }
        }
        assert!(refusals > 0);
        assert_eq!(
            model.storage.admission.last_growth,
            64 * size_of::<usize>(),
            "the final original permutation is allocated by the provided owner"
        );
        let facts = model.facts().unwrap();
        let predicate = facts.term_id("in").unwrap();
        let object = facts.term_id("o").unwrap();
        assert_eq!(
            facts
                .partition_for(&[None, Some(predicate), None])
                .unwrap()
                .select(Bound::Object(object))
                .remaining_count(),
            64
        );
        assert_eq!(facts.rows_from(0).collect::<Vec<_>>(), original);
        assert_eq!(model.budget().join_steps(), 0);
        assert_eq!(live.get(), model.storage.live());
        drop(model);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn sealing_and_size_tier_merges_admit_buffers_before_original_publication() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause},
            dictionary::DictionaryTerm,
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
        let options = EvalOptions::default();
        let context = SessionContext {
            snapshot: Digest32::new([61; 32]),
            guards: Digest32::new([62; 32]),
            evidence: Digest32::new([63; 32]),
        };
        let live = Cell::new(0);
        let mut grant = Grant {
            live: &live,
            maximum: usize::MAX,
            grows: 0,
            maximum_grows: usize::MAX,
            refuse_shrink: false,
            destroyed: None,
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for index in 0..63 {
            input
                .insert_row([&format!("subject-{index}"), "in", "o"])
                .unwrap();
        }
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_in(
            &program, input, &NoGuards, context, options, &mut grant,
        )
        .unwrap();
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        let input_predicate = model.facts().unwrap().term_id("in").unwrap();
        let original = model.facts().unwrap().rows_from(0).collect::<Vec<_>>();
        let mut start = 63;
        for (watermark, count) in [(70, 1), (71, 64), (72, 64)] {
            model.storage.admission.maximum_grows = usize::MAX;
            let mut changes = Vec::new();
            for index in start..start + count {
                let text = format!("subject-{index}");
                let terms = model
                    .admit_terms([text.as_str(), "in", "o"].map(DictionaryTerm::Surface))
                    .unwrap();
                changes.push((terms, 1));
            }
            model
                .apply(ChangeBatch::new(watermark, changes, Resident).unwrap())
                .unwrap();
            model.storage.admission.maximum_grows = model.storage.admission.grows;
            let mut refusals = 0;
            loop {
                match model.resume(options, None).unwrap() {
                    ResumeProgress::Complete => break,
                    ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed)) => {
                        refusals += 1;
                        assert!(
                            refusals < 2048,
                            "the original finite merge retains admitted factory progress"
                        );
                        let state = model.session.state.as_ref().unwrap();
                        let inputs = state.rel.rows_for_predicate(input_predicate).count();
                        assert_eq!(
                            state.rel.row_count(),
                            inputs + state.native.as_ref().unwrap().nodes.len(),
                            "no row is published without its original admitted proof"
                        );
                        assert_eq!(model.pending_watermark(), Some(&watermark));
                        assert_ne!(model.applied_watermark(), Some(&watermark));
                        let joins = model.budget().join_steps();
                        let held = live.get();
                        assert!(matches!(
                            model.resume(options, None).unwrap(),
                            ResumeProgress::Paused(EvalError::Storage(
                                StorageError::AdmissionFailed
                            ))
                        ));
                        assert_eq!(model.budget().join_steps(), joins);
                        assert_eq!(live.get(), held);
                        assert_eq!(model.storage.live(), held);
                        model.storage.admission.maximum_grows += 1;
                    }
                    other @ ResumeProgress::Paused(_) => {
                        panic!("unexpected original seal refusal: {other:?}")
                    }
                }
            }
            assert!(refusals > 0);
            start += count;
            assert_eq!(model.applied_watermark(), Some(&watermark));
            assert_eq!(model.facts().unwrap().row_count(), start * 2);
            assert_eq!(model.witnesses().unwrap().len(), start);
            assert_eq!(
                model.model_delta().unwrap().unwrap().changes().len(),
                count * 2
            );
            assert_eq!(
                model
                    .facts()
                    .unwrap()
                    .rows_from(0)
                    .take(original.len())
                    .collect::<Vec<_>>(),
                original
            );
        }
        assert_eq!(live.get(), model.storage.live());
        drop(model);
        assert_eq!(live.get(), 0);
    }

    #[test]
    fn rejected_batches_and_settled_reclamation_retry_every_original_release() {
        use crate::{
            clause::{ClauseAtom, ClauseTerm, DlClause},
            dictionary::TermDictionary,
            guard::NoGuards,
            maintenance::ChangeBatch,
            seminaive::{
                EvalError, EvalOptions, Maintained, MaintenanceError, ResumeProgress,
                SessionContext, compile,
            },
            store::{RelationStore, TermExpired, TermInterner},
        };
        use purrdf_hash::hex::Digest32;
        use purrdf_lex::allocation::Resident;

        struct Releases<'a> {
            grant: Grant<'a>,
            count: usize,
            maximum: usize,
        }
        impl Admission for Releases<'_> {
            fn resize(&mut self, live: usize) -> Result<(), StorageError> {
                if live < self.grant.live.get() {
                    if self.count == self.maximum {
                        return Err(StorageError::AdmissionFailed);
                    }
                    self.count += 1;
                }
                self.grant.resize(live)
            }
        }
        let atom = |predicate| {
            ClauseAtom::triple(
                ClauseTerm::var("s"),
                ClauseTerm::literal(predicate),
                ClauseTerm::var("o"),
            )
        };
        let program = compile(vec![DlClause::datalog(atom("out"), vec![atom("in")])]).unwrap();
        let options = EvalOptions::default();
        let context = SessionContext {
            snapshot: Digest32::new([51; 32]),
            guards: Digest32::new([52; 32]),
            evidence: Digest32::new([53; 32]),
        };
        let live = Cell::new(0);
        let mut admission = Releases {
            grant: Grant {
                live: &live,
                maximum: usize::MAX,
                grows: 0,
                maximum_grows: usize::MAX,
                refuse_shrink: false,
                destroyed: None,
            },
            count: 0,
            maximum: usize::MAX,
        };
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        input.insert_row(["s", "in", "o"]).unwrap();
        let mut model = Maintained::<u64, Resident, _, 3, _>::new_reclaiming_in(
            &program,
            input.into_reclaiming().unwrap(),
            &NoGuards,
            context,
            options,
            &mut admission,
        )
        .unwrap();
        assert_eq!(
            model.resume(options, None).unwrap(),
            ResumeProgress::Complete
        );
        let terms = ["s", "in", "o"].map(|surface| {
            model
                .facts()
                .unwrap()
                .interner()
                .lookup_term(surface)
                .unwrap()
                .unwrap()
        });
        model
            .begin(ChangeBatch::new(60, [(terms, -2)], Resident).unwrap())
            .unwrap();
        assert!(matches!(
            model.prepare_until(u128::MAX, None),
            Err(MaintenanceError::Multiplicity { weight: -1 })
        ));
        assert!(model.retired_bytes > 0);
        assert_eq!(model.facts().unwrap().row_count(), 2);
        assert_eq!(model.pending_watermark(), None);
        model.storage.admission.maximum = model.storage.admission.count;

        let mut settled_release = false;
        let mut refusals = 0;
        for (watermark, changes) in [(61, vec![(terms, -1)]), (62, Vec::new())] {
            model
                .begin(ChangeBatch::new(watermark, changes, Resident).unwrap())
                .unwrap();
            loop {
                match model.resume(options, None).unwrap() {
                    ResumeProgress::Complete => break,
                    ResumeProgress::Paused(EvalError::Storage(StorageError::AdmissionFailed)) => {
                        refusals += 1;
                        let joins = model.budget().join_steps();
                        let held = live.get();
                        let work = model.transaction_work();
                        let complete = model.pending.as_ref().is_some_and(|pending| {
                            pending.phase == super::super::MutationPhase::Complete
                        });
                        settled_release |= complete;
                        if complete {
                            let relation = model.relation(terms[1]).unwrap();
                            assert_eq!(
                                relation.status(),
                                crate::seminaive::StratumStatus::Complete
                            );
                            assert_eq!(relation.applied_watermark(), Some(&watermark));
                            assert!(matches!(model.facts(), Err(MaintenanceError::Busy)));
                        }
                        assert_eq!(model.pending_watermark(), Some(&watermark));
                        assert_ne!(model.applied_watermark(), Some(&watermark));
                        assert!(matches!(
                            model.resume(options, None).unwrap(),
                            ResumeProgress::Paused(EvalError::Storage(
                                StorageError::AdmissionFailed
                            ))
                        ));
                        assert_eq!(model.budget().join_steps(), joins);
                        if complete {
                            assert_eq!(
                                model.transaction_work(),
                                work,
                                "completed native settlement is never charged again"
                            );
                        }
                        assert_eq!(model.storage.live(), held);
                        assert_eq!(live.get(), held);
                        model.storage.admission.maximum += 1;
                    }
                    other @ ResumeProgress::Paused(_) => {
                        panic!("unexpected original release refusal: {other:?}")
                    }
                }
            }
            assert_eq!(model.applied_watermark(), Some(&watermark));
            assert_eq!(model.facts().unwrap().row_count(), 0);
            if watermark == 61 {
                assert_eq!(model.model_delta().unwrap().unwrap().changes().len(), 2);
                assert_eq!(
                    model
                        .facts()
                        .unwrap()
                        .interner()
                        .resolve_term(terms[0])
                        .unwrap(),
                    "s"
                );
            } else {
                assert_eq!(model.model_delta().unwrap().unwrap().changes(), []);
                assert_eq!(
                    model.facts().unwrap().interner().resolve_term(terms[0]),
                    Err(TermExpired { id: terms[0] })
                );
            }
        }
        assert!(refusals > 5 && settled_release);
        assert_eq!(model.retired_bytes, 0);
        model.storage.admission.maximum = usize::MAX;
        drop(model);
        assert_eq!(live.get(), 0);
    }
}
