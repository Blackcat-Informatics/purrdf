// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned guard invocation, row admission and authored body-guard progress.

use super::{
    EvalError, GuardCall, GuardEvaluator, GuardSite, INLINE_BINDING_SLOTS, RelationStore,
    RuleEntry, SlotSolution, SmallVec, StepGovernor, TermDictionary, TermId, TermIdentity,
};
use crate::dictionary::DictionaryTerm;
use crate::guard::{CallbackDiagnostic, Guard, GuardRows};
use crate::store::{Arity, StoreShape};

/// Stable invocation metadata; inputs need borrowing only for the original call.
pub(super) struct Context<'a, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) rule: usize,
    pub(super) site: GuardSite,
    pub(super) guard: &'a Guard,
    pub(super) model: &'a RelationStore<D, N>,
}

impl<D: TermDictionary, const N: usize> Context<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn refuse(&self, message: CallbackDiagnostic) -> EvalError<D::Error> {
        EvalError::Guard {
            rule: self.rule,
            site: self.site,
            guard: self.guard.name().to_owned(),
            message,
        }
    }

    /// The one built-in/caller dispatch, with the original input borrows.
    pub(super) fn invoke(
        &self,
        guards: &dyn GuardEvaluator<D, N>,
        inputs: &[DictionaryTerm<'_, D::Id>],
    ) -> Result<GuardRows<D::Id>, EvalError<D::Error>> {
        self.model
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        match self.guard.operation().filter(inputs) {
            Some(true) => Ok(vec![Vec::new()]),
            Some(false) => Ok(Vec::new()),
            None => guards
                .evaluate(&GuardCall {
                    rule: self.rule,
                    site: self.site,
                    guard: self.guard,
                    inputs,
                    model: self.model,
                })
                .map_err(|error| error.into_eval_error(|message| self.refuse(message))),
        }
    }
}

/// The original returned batch and its next unadmitted row; never an opaque replay.
#[derive(Debug)]
pub(super) enum Invocation<I: TermIdentity = TermId> {
    Uncalled,
    Returned { rows: GuardRows<I>, next: usize },
}

impl<I: TermIdentity> Invocation<I> {
    /// Whether the original callback/filter has returned and every row was admitted.
    pub(super) fn is_complete(&self) -> bool {
        matches!(self, Self::Returned { rows, next } if *next == rows.len())
    }

    /// Move a fully admitted batch, withholding any unfinished invocation.
    pub(super) fn into_rows(self) -> GuardRows<I> {
        match self {
            Self::Returned { rows, next } if next == rows.len() => rows,
            _ => Vec::new(),
        }
    }

    /// Invoke once, then retain row admission and validation at the exact credit cut.
    /// A fresh callback's width error precedes its final source-health check, as in
    /// the original kernel. Resumption validates source authority before more work.
    pub(super) fn advance<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        guards: &dyn GuardEvaluator<D, N>,
        call: &GuardCall<'_, D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        let context = Context {
            rule: call.rule,
            site: call.site,
            guard: call.guard,
            model: call.model,
        };
        self.advance_with(&context, governor, || context.invoke(guards, call.inputs))
    }

    /// Prepare inputs only when invoking; a resumed admission reuses the batch.
    pub(super) fn advance_with<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        context: &Context<'_, D, N>,
        governor: &mut StepGovernor<'_>,
        invoke: impl FnOnce() -> Result<GuardRows<I>, EvalError<D::Error>>,
    ) -> Result<(), EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        context
            .model
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        if matches!(self, Self::Uncalled) {
            if governor.stopped() {
                return Ok(());
            }
            *self = Self::Returned {
                rows: invoke()?,
                next: 0,
            };
        }
        let Self::Returned { rows, next } = self else {
            unreachable!("the invocation returned its original row batch")
        };
        let width = context.guard.outputs().len();
        for row in rows.iter().skip(*next) {
            if !governor.charge() {
                break;
            }
            if row.len() != width {
                return Err(context.refuse(CallbackDiagnostic::RowWidth {
                    actual: row.len(),
                    expected: width,
                }));
            }
            *next += 1;
        }
        context
            .model
            .interner()
            .check_read()
            .map_err(EvalError::Source)
    }
}

/// Original inputs, current guard/input and its actual pending returned batch.
#[derive(Debug)]
pub(super) struct Body<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    position: usize,
    end: usize,
    inputs: Vec<SlotSolution<I, N>>,
    input: usize,
    output: Vec<SlotSolution<I, N>>,
    call: Option<Invocation<I>>,
}

impl<I: TermIdentity, const N: usize> Body<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Move the existing substitutions into an authored guard range.
    pub(super) fn new(positions: std::ops::Range<usize>, inputs: Vec<SlotSolution<I, N>>) -> Self {
        Self {
            position: positions.start.min(positions.end),
            end: positions.end,
            output: if positions.is_empty() {
                Vec::new()
            } else {
                Vec::with_capacity(inputs.len())
            },
            inputs,
            input: 0,
            call: None,
        }
    }

    /// Whether every input has passed through every requested guard.
    pub(super) fn is_complete(&self) -> bool {
        self.position == self.end
    }

    /// Move completed substitutions; an unfinished stage is retained by its owner.
    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        if self.is_complete() {
            self.inputs
        } else {
            Vec::new()
        }
    }

    /// Continue the original callback/filter path without repeating an invocation.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        entry: RuleEntry<'_, N>,
        rel: &RelationStore<D, N>,
        guards: &dyn GuardEvaluator<D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), EvalError<D::Error>> {
        rel.interner().check_read().map_err(EvalError::Source)?;
        while self.position < self.end {
            let guard = &entry.runtime.guards[self.position];
            let declared = &entry.rule.guards()[self.position];
            while self.input < self.inputs.len() && !governor.spent() {
                let solution = &self.inputs[self.input];
                let inputs = guard
                    .inputs
                    .iter()
                    .map(|&slot| {
                        solution
                            .value(slot)
                            .as_term()
                            .expect("a guard input is bound before the guard (DlClause scoping)")
                    })
                    .collect::<SmallVec<[_; INLINE_BINDING_SLOTS]>>();
                let call = self.call.get_or_insert(Invocation::Uncalled);
                call.advance(
                    guards,
                    &GuardCall {
                        rule: entry.index,
                        site: GuardSite::Body(self.position),
                        guard: declared,
                        inputs: &inputs,
                        model: rel,
                    },
                    governor,
                )?;
                if !call.is_complete() {
                    break;
                }
                let rows = self
                    .call
                    .take()
                    .expect("the invocation completed")
                    .into_rows();
                'returned: for row in rows {
                    let mut next = solution.clone();
                    for (&slot, value) in guard.outputs.iter().zip(&row) {
                        if !next
                            .unify_guard_value(slot, value, rel)
                            .map_err(EvalError::Source)?
                        {
                            continue 'returned;
                        }
                    }
                    self.output.push(next);
                }
                self.input += 1;
            }
            if self.input != self.inputs.len() {
                break;
            }
            self.inputs = std::mem::take(&mut self.output);
            self.input = 0;
            self.position += 1;
            if self.position < self.end {
                self.output = Vec::with_capacity(self.inputs.len());
            }
        }
        rel.interner().check_read().map_err(EvalError::Source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause, ReadSource};
    use crate::dictionary::{DictionaryTerm, DictionaryValue};
    use crate::guard::{CallbackError, Guard, GuardReads};
    use crate::plan::RulePlan;
    use crate::seminaive::{Delta, RuleRuntime, SourceRow};
    use crate::test_support::{HostDictionary, SourceLost};
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct Producer {
        calls: AtomicUsize,
        order: Mutex<Vec<(GuardSite, String)>>,
    }

    impl GuardEvaluator<HostDictionary, 3> for Producer {
        fn evaluate(
            &self,
            call: &GuardCall<'_, HostDictionary, 3>,
        ) -> Result<GuardRows<u64>, CallbackError<SourceLost>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.order
                .lock()
                .unwrap()
                .push((call.site, format!("{:?}", call.inputs)));
            Ok(match call.guard.name() {
                "produce" => vec![
                    vec![DictionaryValue::Id(0)],
                    vec![DictionaryValue::Id(u64::MAX)],
                    vec![DictionaryValue::Surface(
                        "<https://example.org/fresh>".into(),
                    )],
                ],
                "keep" => {
                    if matches!(call.inputs[0], DictionaryTerm::Id(0)) {
                        Vec::new()
                    } else {
                        vec![Vec::new()]
                    }
                }
                other => panic!("unexpected guard {other}"),
            })
        }
    }

    fn check_cuts(
        entry: RuleEntry<'_, 3>,
        store: &RelationStore<HostDictionary, 3>,
        positions: std::ops::Range<usize>,
        inputs: &[SlotSolution<u64, 3>],
    ) -> Vec<SlotSolution<u64, 3>> {
        let probes = || store.interner().lookups.load(Ordering::Relaxed);
        let before = probes();
        let producer = Producer::default();
        let mut reference = Body::new(positions.clone(), inputs.to_vec());
        let mut governor = StepGovernor::new(u64::MAX);
        reference
            .advance(entry, store, &producer, &mut governor)
            .unwrap();
        assert!(reference.is_complete());
        let steps = governor.pool.used.load(Ordering::Relaxed);
        let lookups = probes() - before;
        let order = producer.order.into_inner().unwrap();
        let expected = reference.into_output();
        for cut in 0..=steps {
            let before = probes();
            let producer = Producer::default();
            let mut frame = Body::new(positions.clone(), inputs.to_vec());
            let mut first = StepGovernor::new(cut);
            frame.advance(entry, store, &producer, &mut first).unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                frame.advance(entry, store, &producer, &mut rest).unwrap();
            }
            assert!(frame.is_complete());
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "successful credits, cut {cut}"
            );
            assert_eq!(probes() - before, lookups, "dictionary probes, cut {cut}");
            assert_eq!(
                producer.calls.load(Ordering::Relaxed),
                order.len(),
                "callback count, cut {cut}"
            );
            assert_eq!(
                producer.order.into_inner().unwrap(),
                order,
                "callback order, cut {cut}"
            );
            assert_eq!(
                frame.into_output(),
                expected,
                "bindings and sources, cut {cut}"
            );
        }
        let before = probes();
        let producer = Producer::default();
        let mut frame = Body::new(positions, inputs.to_vec());
        let mut resumed_steps = 0;
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame.advance(entry, store, &producer, &mut page).unwrap();
            let used = page.pool.used.load(Ordering::Relaxed);
            resumed_steps += used;
            assert!(used != 0 || frame.is_complete());
        }
        assert_eq!(resumed_steps, steps);
        assert_eq!(probes() - before, lookups);
        assert_eq!(producer.order.into_inner().unwrap(), order);
        assert_eq!(frame.into_output(), expected);
        expected
    }

    #[test]
    fn body_guards_resume_every_credit_cut_without_reinvoking_callbacks() {
        const WIDE: u64 = (1 << 54) + 1;
        let mut store = RelationStore::<_, 3>::with_dictionary(HostDictionary {
            surfaces: std::iter::once((7, "<https://example.org/p>".to_owned())).collect(),
            ..HostDictionary::default()
        });
        let atom = |predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var("a"),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        };
        let rule = DlClause::datalog(
            atom("https://example.org/result", "value"),
            vec![atom("https://example.org/p", "b")],
        )
        .with_guards(vec![
            Guard::assign("produce", vec!["a".into()], "value"),
            Guard::filter("keep", vec!["value".into()]),
        ]);
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let slot = |name| {
            plan.variables()
                .iter()
                .position(|variable| variable == name)
                .unwrap()
        };
        let mut inputs = Vec::new();
        for subject in [0, u64::MAX] {
            let terms = [subject, 7, WIDE];
            let (_, _, row) = store.insert_ids(terms).unwrap().unwrap();
            let mut input = SlotSolution::empty(plan.variables().len());
            input.bindings[slot("a")] = Some(subject);
            input.bindings[slot("b")] = Some(WIDE);
            input.sources.push(SourceRow {
                source: ReadSource::Evaluation,
                body_index: 0,
                terms,
                row,
            });
            inputs.push(input);
        }
        let entry = RuleEntry {
            index: 4,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta: Delta::all(2),
        };
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        assert_eq!(check_cuts(entry, &store, 0..2, &inputs).len(), 4);
        assert_eq!(check_cuts(entry, &store, 0..1, &inputs).len(), 6);
        assert_eq!(check_cuts(entry, &store, 0..0, &inputs), inputs);
        for positions in [0..1, 0..2] {
            let unbound = check_cuts(entry, &store, positions.clone(), &inputs);
            for value in [
                DictionaryValue::Id(0),
                DictionaryValue::Id(u64::MAX),
                DictionaryValue::Surface("<https://example.org/fresh>".into()),
                DictionaryValue::Surface("<https://example.org/absent>".into()),
            ] {
                let mut seeded = inputs.clone();
                for input in &mut seeded {
                    input.bind_value(slot("value"), &value, &store).unwrap();
                }
                let expected = unbound
                    .iter()
                    .filter(|solution| {
                        solution.value(slot("value")).as_term() == Some(value.as_term())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(
                    check_cuts(entry, &store, positions.clone(), &seeded),
                    expected
                );
            }
        }
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    struct Returned {
        rows: GuardRows<u64>,
        calls: AtomicUsize,
        lose: bool,
        error: Option<CallbackError<SourceLost>>,
    }

    impl GuardEvaluator<HostDictionary, 3> for Returned {
        fn evaluate(
            &self,
            call: &GuardCall<'_, HostDictionary, 3>,
        ) -> Result<GuardRows<u64>, CallbackError<SourceLost>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.lose {
                call.model.interner().lost.store(true, Ordering::Relaxed);
            }
            self.error
                .clone()
                .map_or_else(|| Ok(self.rows.clone()), Err)
        }
    }

    #[test]
    fn invocation_keeps_the_original_returned_batch_until_its_final_credit() {
        let store = RelationStore::<HostDictionary, 3>::with_dictionary(HostDictionary::default());
        let guard = Guard::assign("returned", Vec::new(), "value");
        let call = GuardCall {
            rule: 3,
            site: GuardSite::Body(2),
            guard: &guard,
            inputs: &[],
            model: &store,
        };
        for count in [0, 1, 5] {
            for cut in 0..=count {
                let returned = Returned {
                    rows: (0..count)
                        .map(|row| vec![DictionaryValue::Id(row)])
                        .collect(),
                    calls: AtomicUsize::new(0),
                    lose: false,
                    error: None,
                };
                let mut invocation = Invocation::Uncalled;
                let mut first = StepGovernor::new(cut);
                invocation.advance(&returned, &call, &mut first).unwrap();
                let Invocation::Returned { rows, .. } = &invocation else {
                    panic!("callback returned");
                };
                let pointer = rows.as_ptr();
                if !invocation.is_complete() {
                    invocation
                        .advance(&returned, &call, &mut StepGovernor::new(u64::MAX))
                        .unwrap();
                }
                assert!(invocation.is_complete());
                let rows = invocation.into_rows();
                assert_eq!(
                    rows.as_ptr(),
                    pointer,
                    "the callback batch is moved, not rebuilt"
                );
                assert_eq!(rows, returned.rows);
                assert_eq!(returned.calls.load(Ordering::Relaxed), 1);
            }
        }
    }

    #[test]
    fn invocation_preserves_width_and_source_failure_precedence_across_a_pause() {
        let store = RelationStore::<HostDictionary, 3>::with_dictionary(HostDictionary::default());
        let guard = Guard::new(
            "returned",
            Vec::new(),
            vec!["value".into()],
            GuardReads::Bindings,
        );
        let call = GuardCall {
            rule: 3,
            site: GuardSite::Body(2),
            guard: &guard,
            inputs: &[],
            model: &store,
        };
        let returned = Returned {
            rows: vec![vec![DictionaryValue::Id(0)], Vec::new()],
            calls: AtomicUsize::new(0),
            lose: false,
            error: None,
        };
        let width_error = EvalError::Guard {
            rule: 3,
            site: GuardSite::Body(2),
            guard: "returned".into(),
            message: CallbackDiagnostic::RowWidth {
                actual: 0,
                expected: 1,
            },
        };
        let mut invocation = Invocation::Uncalled;
        invocation
            .advance(&returned, &call, &mut StepGovernor::new(1))
            .unwrap();
        assert!(!invocation.is_complete());
        assert_eq!(
            invocation.advance(&returned, &call, &mut StepGovernor::new(1)),
            Err(width_error.clone())
        );
        assert_eq!(returned.calls.load(Ordering::Relaxed), 1);
        let malformed = Returned {
            rows: vec![Vec::new()],
            calls: AtomicUsize::new(0),
            lose: true,
            error: None,
        };
        assert_eq!(
            Invocation::Uncalled.advance(&malformed, &call, &mut StepGovernor::new(1)),
            Err(width_error)
        );
        store.interner().lost.store(false, Ordering::Relaxed);
        let failure = Returned {
            rows: Vec::new(),
            calls: AtomicUsize::new(0),
            lose: true,
            error: Some(CallbackError::Source(SourceLost(17))),
        };
        assert_eq!(
            Invocation::Uncalled.advance(&failure, &call, &mut StepGovernor::new(0)),
            Err(EvalError::Source(SourceLost(17)))
        );
        store.interner().lost.store(false, Ordering::Relaxed);
        let mut invocation = Invocation::Uncalled;
        invocation
            .advance(&returned, &call, &mut StepGovernor::new(0))
            .unwrap();
        let calls = returned.calls.load(Ordering::Relaxed);
        store.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            invocation.advance(&returned, &call, &mut StepGovernor::new(0)),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(returned.calls.load(Ordering::Relaxed), calls);
    }
}
