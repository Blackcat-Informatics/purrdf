// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned authored guard barriers, native positive segments and delta anchors.

use super::{
    Delta, EvalError, GuardEvaluator, JoinScan, JoinSnapshot, JoinStrategy, RuleEntry, Scan,
    SlotSolution, StepGovernor, TermDictionary, TermId, TermIdentity, delta_can_match, guards,
    join,
};
use crate::store::{Arity, StoreShape};

/// One original positive region, including its cycle seed's source coordinate.
#[derive(Debug)]
pub(super) struct Segment<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    sequence: join::Sequence<I, N>,
    remove_seed: Option<usize>,
}

impl<I: TermIdentity, const N: usize> Segment<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Freeze this guard-bounded region's original seed and physical steps once.
    fn new<D: TermDictionary<Id = I>>(
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, D, N>,
        strategy: JoinStrategy,
        scan: JoinScan,
        body: std::ops::Range<usize>,
        inputs: Vec<SlotSolution<I, N>>,
    ) -> Self {
        let plan = entry.plan;
        let cyclic = !matches!(scan, JoinScan::Uniform(_))
            && plan.has_cyclic_subplan()
            && matches!(strategy, JoinStrategy::Planned);
        let seed = match scan {
            JoinScan::DeltaPosition(anchor)
                if snapshot.delta != Delta::all(snapshot.rel.row_count())
                    && body.contains(&plan.operator_at(anchor).body_index()) =>
            {
                Some(anchor)
            }
            _ => None,
        };
        let selection = join::Selection {
            scan,
            seed,
            cyclic,
            rebound: true,
        };
        let (steps, seed_in_cycle) = join::selected_steps(plan, selection, |operator| {
            body.contains(&operator.body_index())
        });
        Self {
            sequence: join::Sequence::new(steps, inputs),
            remove_seed: seed_in_cycle.then_some(body.start),
        }
    }

    /// Whether this segment's actual native sequence has exhausted every input.
    fn is_complete(&self) -> bool {
        self.sequence.is_complete()
    }

    /// Remove only the duplicate seed source, preserving every earlier premise.
    fn into_output(self) -> Vec<SlotSolution<I, N>> {
        let mut output = self.sequence.into_output();
        if let Some(seed) = self.remove_seed {
            for solution in &mut output {
                solution.sources.remove(seed);
            }
        }
        output
    }
}

#[derive(Debug)]
enum Phase<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Unit,
    Ready,
    Segment(Segment<I, N>),
    Guard(guards::Body<I, N>),
}

#[derive(Debug)]
struct Anchor<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    ordinal: usize,
    barrier: usize,
    start: usize,
    end: usize,
    partial: Vec<SlotSolution<I, N>>,
    phase: Phase<I, N>,
}

/// The whole ordered body, with no callback or positive prefix replay on resume.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    anchors: std::ops::Range<usize>,
    base: bool,
    strategy: JoinStrategy,
    active: Option<Anchor<I, N>>,
    output: Vec<SlotSolution<I, N>>,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Freeze the original body-change decision and anchor range before any guard.
    pub(super) fn new<D: TermDictionary<Id = I>>(
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, D, N>,
        strategy: JoinStrategy,
    ) -> Result<Self, EvalError<D::Error>> {
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let base = entry
            .plan
            .operators()
            .iter()
            .any(|operator| operator.shape().read_source() == crate::clause::ReadSource::Base);
        let mut anchors = if base {
            0..1
        } else {
            snapshot.delta_positions(entry.plan.positive().len())
        };
        if !base && snapshot.delta != Delta::all(snapshot.rel.row_count()) {
            // An empty later anchor must not erase an earlier guard invocation.
            // Only an unchanged whole body removes the round's guard obligation.
            let mut changed = false;
            for operator in entry.plan.operators() {
                if delta_can_match(operator, snapshot).map_err(EvalError::Source)? {
                    changed = true;
                    break;
                }
            }
            if !changed {
                anchors = 0..0;
            }
        }
        Ok(Self {
            anchors,
            base,
            strategy,
            active: None,
            output: Vec::new(),
        })
    }

    /// Whether every original anchor and authored barrier has completed.
    pub(super) fn is_complete(&self) -> bool {
        self.anchors.is_empty() && self.active.is_none()
    }

    /// Move completed-anchor output; pending stages stay owned until resumed.
    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        self.output
    }

    /// Continue the same native segment or returned guard batch at its next credit.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, D, N>,
        guards: &dyn GuardEvaluator<D, N>,
        governor: &mut StepGovernor,
    ) -> Result<(), EvalError<D::Error>> {
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let barriers = entry
            .rule
            .guard_positions()
            .expect("ordered execution has original authored guard barriers");
        loop {
            if self.active.is_none() {
                if governor.spent() {
                    break;
                }
                let Some(ordinal) = self.anchors.next() else {
                    break;
                };
                self.active = Some(Anchor {
                    ordinal,
                    barrier: 0,
                    start: 0,
                    end: 0,
                    partial: vec![SlotSolution::empty(entry.plan.variables().len())],
                    phase: if barriers.first() == Some(&0) {
                        Phase::Unit
                    } else {
                        Phase::Ready
                    },
                });
            }
            let anchor = self
                .active
                .as_mut()
                .expect("the original anchor was selected");
            let scan = if self.base {
                JoinScan::Uniform(Scan::Full)
            } else {
                JoinScan::DeltaPosition(anchor.ordinal)
            };
            let finished = match &mut anchor.phase {
                Phase::Unit => {
                    if !governor.charge() {
                        break;
                    }
                    anchor.phase = Phase::Ready;
                    false
                }
                Phase::Ready => {
                    anchor.end = barriers
                        .get(anchor.barrier)
                        .copied()
                        .unwrap_or_else(|| entry.rule.body().len());
                    anchor.phase = Phase::Segment(Segment::new(
                        entry,
                        snapshot,
                        self.strategy,
                        scan,
                        anchor.start..anchor.end,
                        std::mem::take(&mut anchor.partial),
                    ));
                    false
                }
                Phase::Segment(segment) => {
                    segment
                        .sequence
                        .advance(entry.plan, snapshot, governor)
                        .map_err(EvalError::Source)?;
                    if !segment.is_complete() {
                        break;
                    }
                    let Phase::Segment(segment) =
                        std::mem::replace(&mut anchor.phase, Phase::Ready)
                    else {
                        unreachable!("the native segment just completed")
                    };
                    anchor.partial = segment.into_output();
                    if anchor.partial.is_empty() || anchor.barrier == barriers.len() {
                        true
                    } else {
                        anchor.phase = Phase::Guard(guards::Body::new(
                            anchor.barrier..anchor.barrier + 1,
                            std::mem::take(&mut anchor.partial),
                        ));
                        false
                    }
                }
                Phase::Guard(body) => {
                    body.advance(entry, snapshot.rel, guards, governor)?;
                    if !body.is_complete() {
                        break;
                    }
                    let Phase::Guard(body) = std::mem::replace(&mut anchor.phase, Phase::Ready)
                    else {
                        unreachable!("the original guard stage just completed")
                    };
                    anchor.partial = body.into_output();
                    anchor.start = anchor.end;
                    anchor.barrier += 1;
                    false
                }
            };
            if finished {
                let mut anchor = self.active.take().expect("the original anchor completed");
                for solution in &mut anchor.partial {
                    solution
                        .sources
                        .sort_unstable_by_key(|source| source.body_index);
                }
                self.output.extend(anchor.partial);
            }
        }
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::{BodyLiteral, ClauseAtom, ClauseTerm, DlClause};
    use crate::dictionary::{DictionaryTerm, DictionaryValue};
    use crate::guard::{CallbackError, Guard, GuardCall, GuardRows, GuardSite};
    use crate::plan::RulePlan;
    use crate::seminaive::{DeltaIndexes, RuleRuntime};
    use crate::store::RelationStore;
    use crate::test_support::{HostDictionary, SourceLost};
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    const GRAPH: u64 = 1 << 63;
    const WIDE: u64 = (1 << 54) + 1;

    #[derive(Default)]
    struct Callbacks {
        calls: Mutex<Vec<(GuardSite, Vec<u64>)>>,
        seeds: AtomicUsize,
        fail_seed: Option<usize>,
    }

    impl<const N: usize> GuardEvaluator<HostDictionary, N> for Callbacks
    where
        Arity<N>: StoreShape<N>,
    {
        fn evaluate(
            &self,
            call: &GuardCall<'_, HostDictionary, N>,
        ) -> Result<GuardRows<u64>, CallbackError<SourceLost>> {
            let ids = call
                .inputs
                .iter()
                .map(|term| match term {
                    DictionaryTerm::Id(id) => *id,
                    DictionaryTerm::Surface(_) => {
                        panic!("this fixture's guard inputs are original IDs")
                    }
                })
                .collect::<Vec<_>>();
            self.calls.lock().unwrap().push((call.site, ids.clone()));
            Ok(match call.guard.name() {
                "seed" => {
                    let ordinal = self.seeds.fetch_add(1, Ordering::Relaxed) + 1;
                    if self.fail_seed == Some(ordinal) {
                        return Err(CallbackError::Source(SourceLost(17)));
                    }
                    vec![
                        vec![DictionaryValue::Id(0)],
                        vec![DictionaryValue::Id(u64::MAX)],
                    ]
                }
                "gate" => vec![Vec::new()],
                "expand" => {
                    let id = if ids[0] < 20 { ids[0] + 10 } else { ids[0] };
                    vec![
                        vec![DictionaryValue::Id(id)],
                        vec![DictionaryValue::Id(WIDE)],
                        vec![DictionaryValue::Surface(
                            "<https://example.org/fresh>".into(),
                        )],
                    ]
                }
                name => panic!("unexpected callback {name}"),
            })
        }
    }

    fn fixture<const N: usize>() -> RelationStore<HostDictionary, N>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut store = RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                // Confirmed heads render original facts, unlike body-only probes.
                (0, "<https://example.org/id/zero>".into()),
                (u64::MAX, "<https://example.org/id/max>".into()),
                (WIDE, "<https://example.org/id/wide>".into()),
                (10, "<https://example.org/id/ten>".into()),
                (11, "<https://example.org/id/eleven>".into()),
                (20, "<https://example.org/id/twenty>".into()),
                (21, "<https://example.org/id/twenty-one>".into()),
                (77, "<https://example.org/id/root>".into()),
                (7, "<https://example.org/p>".into()),
                (8, "<https://example.org/q>".into()),
                (9, "<https://example.org/r>".into()),
                (17, "<https://example.org/seed>".into()),
                (19, "<https://example.org/tail>".into()),
                (GRAPH, "<https://example.org/g>".into()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        });
        for row in [
            [0, 7, 10],
            [u64::MAX, 7, 11],
            [10, 8, 20],
            [11, 8, 21],
            [20, 9, 0],
            [21, 9, u64::MAX],
            [77, 17, 0],
            [77, 17, u64::MAX],
            [20, 19, 0],
            [21, 19, u64::MAX],
        ] {
            store
                .insert_ids(std::array::from_fn(|position| {
                    if position < 3 { row[position] } else { GRAPH }
                }))
                .unwrap();
        }
        store
    }

    fn rule<const N: usize>(
        cyclic: bool,
        atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>,
    ) -> DlClause<N> {
        use BodyLiteral::{Atom, Guard as Compute};
        if cyclic {
            DlClause::datalog(atom("root", "https://example.org/result", "z"), Vec::new())
                .with_ordered_body(vec![
                    Atom(atom("root", "https://example.org/seed", "a")),
                    Compute(Guard::filter("gate", vec!["a".into()])),
                    Atom(atom("a", "https://example.org/p", "b")),
                    Atom(atom("b", "https://example.org/q", "c")),
                    Atom(atom("c", "https://example.org/r", "a")),
                    Compute(Guard::assign("expand", vec!["c".into()], "z")),
                    Atom(atom("z", "https://example.org/tail", "a")),
                ])
        } else {
            DlClause::datalog(atom("a", "https://example.org/result", "c"), Vec::new())
                .with_ordered_body(vec![
                    Compute(Guard::assign("seed", Vec::new(), "a")),
                    Atom(atom("a", "https://example.org/p", "b")),
                    Compute(Guard::assign("expand", vec!["b".into()], "c")),
                    Atom(atom("c", "https://example.org/r", "a")),
                ])
        }
    }

    fn check_cuts<const N: usize>(
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, HostDictionary, N>,
        strategy: JoinStrategy,
    ) -> Vec<SlotSolution<u64, N>>
    where
        Arity<N>: StoreShape<N>,
    {
        let probes = || snapshot.rel.interner().lookups.load(Ordering::Relaxed);
        let before = probes();
        let callbacks = Callbacks::default();
        let mut reference = Frame::new(entry, snapshot, strategy).unwrap();
        let mut governor = StepGovernor::new(u64::MAX);
        reference
            .advance(entry, snapshot, &callbacks, &mut governor)
            .unwrap();
        assert!(reference.is_complete());
        let steps = governor.pool.used.load(Ordering::Relaxed);
        let visits = governor.row_visits;
        let lookups = probes() - before;
        let calls = callbacks.calls.into_inner().unwrap();
        let expected = reference.into_output();
        for cut in 0..=steps {
            let before = probes();
            let callbacks = Callbacks::default();
            let mut frame = Frame::new(entry, snapshot, strategy).unwrap();
            let mut first = StepGovernor::new(cut);
            frame
                .advance(entry, snapshot, &callbacks, &mut first)
                .unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                frame
                    .advance(entry, snapshot, &callbacks, &mut rest)
                    .unwrap();
            }
            assert!(frame.is_complete());
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "successful credits, cut {cut}"
            );
            assert_eq!(
                first.row_visits + rest.row_visits,
                visits,
                "native rows, cut {cut}"
            );
            assert_eq!(probes() - before, lookups, "dictionary probes, cut {cut}");
            assert_eq!(
                callbacks.calls.into_inner().unwrap(),
                calls,
                "opaque calls, cut {cut}"
            );
            assert_eq!(
                frame.into_output(),
                expected,
                "bindings and authored sources, cut {cut}"
            );
        }
        let before = probes();
        let callbacks = Callbacks::default();
        let mut frame = Frame::new(entry, snapshot, strategy).unwrap();
        let (mut resumed_steps, mut resumed_visits) = (0, 0);
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance(entry, snapshot, &callbacks, &mut page)
                .unwrap();
            let used = page.pool.used.load(Ordering::Relaxed);
            resumed_steps += used;
            resumed_visits += page.row_visits;
            assert!(used != 0 || frame.is_complete());
        }
        assert_eq!((resumed_steps, resumed_visits), (steps, visits));
        assert_eq!(probes() - before, lookups);
        assert_eq!(callbacks.calls.into_inner().unwrap(), calls);
        assert_eq!(frame.into_output(), expected);
        expected
    }

    fn check_shapes<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        let store = fixture::<N>();
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        for cyclic in [false, true] {
            let rule = rule(cyclic, &atom);
            let plan = RulePlan::for_rule(&rule);
            assert_eq!(plan.has_cyclic_subplan(), cyclic);
            let runtime = RuleRuntime::new(&rule, &plan);
            for delta in [
                Delta::all(10),
                Delta { lo: 0, hi: 1 },
                Delta { lo: 2, hi: 3 },
                Delta { lo: 6, hi: 7 },
                Delta { lo: 8, hi: 9 },
                Delta::all(0),
            ] {
                let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
                let entry = RuleEntry {
                    index: 2,
                    rule: &rule,
                    plan: &plan,
                    runtime: &runtime,
                    delta,
                };
                for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                    let output = check_cuts(
                        entry,
                        JoinSnapshot {
                            rel: &store,
                            delta,
                            indexes: &indexes,
                        },
                        strategy,
                    );
                    if delta == Delta::all(10) {
                        assert_eq!(output.len(), 2);
                    }
                    for solution in output {
                        assert_eq!(
                            solution
                                .sources
                                .iter()
                                .map(|source| source.body_index)
                                .collect::<Vec<_>>(),
                            (0..rule.body().len()).collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    #[test]
    fn ordered_body_frames_resume_every_credit_cut_across_triple_guard_barriers() {
        check_shapes(|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
    }

    #[test]
    fn ordered_body_frames_resume_every_credit_cut_across_quad_guard_barriers() {
        check_shapes(|subject, predicate, object| {
            ClauseAtom::quad(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
                ClauseTerm::iri("https://example.org/g"),
            )
        });
    }

    #[test]
    fn ordered_resume_keeps_early_guard_failures_and_original_source_authority() {
        let store = fixture::<3>();
        let rule = rule(false, |subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let delta = Delta { lo: 0, hi: 1 };
        let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
        let entry = RuleEntry {
            index: 2,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta,
        };
        let snapshot = JoinSnapshot {
            rel: &store,
            delta,
            indexes: &indexes,
        };
        let callbacks = Callbacks {
            fail_seed: Some(2),
            ..Callbacks::default()
        };
        let mut frame = Frame::new(entry, snapshot, JoinStrategy::Planned).unwrap();
        frame
            .advance(entry, snapshot, &callbacks, &mut StepGovernor::new(1))
            .unwrap();
        assert_eq!(callbacks.seeds.load(Ordering::Relaxed), 1);
        assert_eq!(
            frame.advance(
                entry,
                snapshot,
                &callbacks,
                &mut StepGovernor::new(u64::MAX)
            ),
            Err(EvalError::Source(SourceLost(17))),
            "the empty later delta cannot erase its earlier guard"
        );
        assert_eq!(callbacks.seeds.load(Ordering::Relaxed), 2);
        let callbacks = Callbacks::default();
        let mut frame = Frame::new(entry, snapshot, JoinStrategy::Planned).unwrap();
        frame
            .advance(entry, snapshot, &callbacks, &mut StepGovernor::new(0))
            .unwrap();
        assert!(callbacks.calls.lock().unwrap().is_empty());
        let round = super::super::RoundSnapshot {
            rel: &store,
            depth: &[0; 10],
            assumed: &[],
            delta_indexes: Some(&indexes),
        };
        let counts = super::super::constant_pattern_counts(&plan, &store, delta).unwrap();
        let mut whole =
            super::super::rule::Frame::new(entry, round, JoinStrategy::Planned, &counts).unwrap();
        whole
            .advance(
                entry,
                round,
                &callbacks,
                &mut StepGovernor::new(0),
                &mut None,
            )
            .unwrap();
        let lookups = store.interner().lookups.load(Ordering::Relaxed);
        store.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            frame.advance(entry, snapshot, &callbacks, &mut StepGovernor::new(0)),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(store.interner().lookups.load(Ordering::Relaxed), lookups);
        assert!(callbacks.calls.lock().unwrap().is_empty());
        assert_eq!(
            whole.advance(
                entry,
                round,
                &callbacks,
                &mut StepGovernor::new(0),
                &mut None
            ),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(store.interner().lookups.load(Ordering::Relaxed), lookups);
        assert!(callbacks.calls.lock().unwrap().is_empty());
    }

    /// Compare the original whole rule, including the barrier before head emission.
    fn check_whole_rule<const N: usize>(
        entry: RuleEntry<'_, N>,
        snapshot: super::super::RoundSnapshot<'_, HostDictionary, N>,
        strategy: JoinStrategy,
    ) where
        Arity<N>: StoreShape<N>,
    {
        use super::super::{constant_pattern_counts, evidence, reference_evaluate_rule, rule};
        use purrdf_lex::allocation::Resident;
        let counts = constant_pattern_counts(entry.plan, snapshot.rel, entry.delta).unwrap();
        let capture = || {
            evidence::Capture::<_, u64, N>::new(
                crate::evidence::DerivationEvidence::with_admission(
                    snapshot.rel.row_count(),
                    Resident,
                ),
                crate::test_support::evidence_scope(),
            )
        };
        let probes = || {
            (
                snapshot.rel.interner().lookups.load(Ordering::Relaxed),
                snapshot.rel.interner().resolves.load(Ordering::Relaxed),
            )
        };
        let compare = |actual: &super::super::RoundBuffer<'_, u64, N>,
                       expected: &super::super::RoundBuffer<'_, u64, N>| {
            assert_eq!(actual.entries, expected.entries);
            assert_eq!(actual.confirmed.len(), expected.confirmed.len());
            for (row, confirmation) in &actual.confirmed {
                assert_eq!(confirmation.derivation, expected.confirmed[row].derivation);
                assert_eq!(
                    confirmation.sum_source_height,
                    expected.confirmed[row].sum_source_height
                );
            }
            assert_eq!(actual.generates_terms, expected.generates_terms);
            assert_eq!(actual.generating_rules, expected.generating_rules);
        };
        for evidenced in [false, true] {
            let callbacks = Callbacks::default();
            let mut original = capture();
            let expected = reference_evaluate_rule(
                entry,
                snapshot,
                strategy,
                StepGovernor::new(u64::MAX),
                &callbacks,
                evidenced
                    .then_some(&mut original as &mut dyn evidence::RuleEvidence<HostDictionary, N>),
                &counts,
            )
            .unwrap();
            let calls = callbacks.calls.into_inner().unwrap();
            let callbacks = Callbacks::default();
            let mut owner = capture();
            let before = probes();
            let mut frame = rule::Frame::new(entry, snapshot, strategy, &counts).unwrap();
            let mut full = StepGovernor::new(u64::MAX);
            frame
                .advance(
                    entry,
                    snapshot,
                    &callbacks,
                    &mut full,
                    &mut evidenced.then_some(
                        &mut owner as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                    ),
                )
                .unwrap();
            assert!(frame.is_complete());
            assert_eq!(frame.into_buffer().entries, expected.entries);
            let after = probes();
            let probe_count = (after.0 - before.0, after.1 - before.1);
            let steps = full.pool.used.load(Ordering::Relaxed);
            assert_eq!(steps, expected.join_steps);
            let visits = full.row_visits;
            for cut in 0..=steps + 1 {
                if cut <= steps {
                    let reference_callbacks = Callbacks::default();
                    let current_callbacks = Callbacks::default();
                    let mut reference_owner = capture();
                    let mut current_owner = capture();
                    let reference = reference_evaluate_rule(
                        entry,
                        snapshot,
                        strategy,
                        StepGovernor::new(cut),
                        &reference_callbacks,
                        evidenced.then_some(
                            &mut reference_owner
                                as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                        ),
                        &counts,
                    )
                    .unwrap();
                    let current = super::super::evaluate_rule_with_evidence(
                        entry,
                        snapshot,
                        strategy,
                        StepGovernor::new(cut),
                        &current_callbacks,
                        evidenced.then_some(
                            &mut current_owner
                                as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                        ),
                        Some(&counts),
                    )
                    .unwrap();
                    compare(&current, &reference);
                    assert_eq!(
                        current.join_steps, reference.join_steps,
                        "refusal work, cut {cut}"
                    );
                    assert_eq!(
                        reference_callbacks.calls.into_inner().unwrap(),
                        current_callbacks.calls.into_inner().unwrap()
                    );
                    if evidenced {
                        assert_eq!(
                            reference_owner.rows.workspace().canonical_index().unwrap(),
                            current_owner.rows.workspace().canonical_index().unwrap()
                        );
                    }
                }
                let callbacks = Callbacks::default();
                let mut owner = capture();
                let before = probes();
                let mut frame = rule::Frame::new(entry, snapshot, strategy, &counts).unwrap();
                let mut credits = 0;
                let mut rows = 0;
                // The extra case resumes one credit at a time, rather than once.
                let pages = if cut > steps {
                    vec![1; usize::try_from(steps + 1).unwrap()]
                } else {
                    vec![cut, u64::MAX]
                };
                for credit in pages {
                    if frame.is_complete() {
                        break;
                    }
                    let mut governor = StepGovernor::new(credit);
                    frame
                        .advance(
                            entry,
                            snapshot,
                            &callbacks,
                            &mut governor,
                            &mut evidenced.then_some(
                                &mut owner as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                            ),
                        )
                        .unwrap();
                    credits += governor.pool.used.load(Ordering::Relaxed);
                    rows += governor.row_visits;
                }
                assert!(frame.is_complete(), "cut {cut}");
                let buffer = frame.into_buffer();
                compare(&buffer, &expected);
                assert_eq!((credits, rows), (steps, visits), "work, cut {cut}");
                let after = probes();
                assert_eq!(
                    (after.0 - before.0, after.1 - before.1),
                    probe_count,
                    "probes, cut {cut}"
                );
                assert_eq!(
                    callbacks.calls.into_inner().unwrap(),
                    calls,
                    "callbacks, cut {cut}"
                );
                if evidenced {
                    assert_eq!(
                        owner.rows.workspace().canonical_index().unwrap(),
                        original.rows.workspace().canonical_index().unwrap()
                    );
                }
            }
        }
    }

    fn check_rule_phases<const N: usize>(
        atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>,
        unit: ClauseAtom<N>,
    ) where
        Arity<N>: StoreShape<N>,
    {
        use crate::clause::HeadDisjunct;
        use crate::guard::Negation;
        let store = fixture::<N>();
        let rules = [
            rule(false, &atom),
            rule(true, &atom),
            DlClause::new(
                vec![HeadDisjunct::new(vec![
                    atom("a", "https://example.org/result", "z"),
                    atom("z", "https://example.org/generated", "a"),
                    atom("a", "https://example.org/p", "b"),
                ])],
                Vec::new(),
                vec![
                    atom("a", "https://example.org/p", "b"),
                    atom("a", "https://example.org/absent", "b").into_negated(),
                ],
            )
            .with_guards(vec![Guard::assign("expand", vec!["b".into()], "z")]),
            rule(true, &atom).with_negations(vec![Negation::new(
                vec![atom("c", "https://example.org/r", "local")],
                vec![Guard::filter("gate", vec!["local".into()])],
            )]),
            DlClause::datalog(
                atom("a", "https://example.org/result", "d"),
                vec![
                    atom("a", "https://example.org/p", "b"),
                    atom("c", "https://example.org/q", "d"),
                ],
            ),
            DlClause::datalog(unit, Vec::new()),
        ];
        for rule in rules {
            let plan = RulePlan::for_rule(&rule);
            let runtime = RuleRuntime::new(&rule, &plan);
            for delta in [
                Delta::all(10),
                Delta { lo: 0, hi: 1 },
                Delta { lo: 6, hi: 8 },
                Delta::all(0),
            ] {
                let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
                let entry = RuleEntry {
                    index: 2,
                    rule: &rule,
                    plan: &plan,
                    runtime: &runtime,
                    delta,
                };
                let snapshot = super::super::RoundSnapshot {
                    rel: &store,
                    depth: &[0, 1, u32::MAX, 2, 3, 0, 0, 1, 2, 3],
                    assumed: &[true; 10],
                    delta_indexes: Some(&indexes),
                };
                for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                    check_whole_rule(entry, snapshot, strategy);
                }
            }
        }
    }

    #[test]
    fn triple_whole_rules_resume_guards_signed_filters_and_conjunctive_heads() {
        check_rule_phases(
            |subject, predicate, object| {
                ClauseAtom::triple(
                    ClauseTerm::var(subject),
                    ClauseTerm::iri(predicate),
                    ClauseTerm::var(object),
                )
            },
            ClauseAtom::triple(
                ClauseTerm::iri("https://example.org/unit"),
                ClauseTerm::iri("https://example.org/p"),
                ClauseTerm::iri("https://example.org/unit"),
            ),
        );
    }

    #[test]
    fn quad_whole_rules_resume_guards_signed_filters_and_conjunctive_heads() {
        check_rule_phases(
            |subject, predicate, object| {
                ClauseAtom::quad(
                    ClauseTerm::var(subject),
                    ClauseTerm::iri(predicate),
                    ClauseTerm::var(object),
                    ClauseTerm::iri("https://example.org/g"),
                )
            },
            ClauseAtom::quad(
                ClauseTerm::iri("https://example.org/unit"),
                ClauseTerm::iri("https://example.org/p"),
                ClauseTerm::iri("https://example.org/unit"),
                ClauseTerm::iri("https://example.org/g"),
            ),
        );
    }
}
