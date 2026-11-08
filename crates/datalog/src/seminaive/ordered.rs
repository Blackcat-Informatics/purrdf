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
                if snapshot.delta != Delta::all(snapshot.rel.row_slots())
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
        if !base && snapshot.delta != Delta::all(snapshot.rel.row_slots()) {
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
        governor: &mut StepGovernor<'_>,
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
                    partial: vec![snapshot.initial(entry.plan)],
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
                            seed: None,
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
            seed: None,
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
            probe: None,
            rel: &store,
            depth: (&[0; 10][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
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
        let mut rounds = super::super::round::Frame::new(&[entry], round).unwrap();
        rounds
            .advance(
                round,
                super::super::RoundExecution::Sequential,
                JoinStrategy::Planned,
                0,
                super::super::RoundControl {
                    guards: &callbacks,
                    stop: None,
                },
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
            rounds.advance(
                round,
                super::super::RoundExecution::Sequential,
                JoinStrategy::Planned,
                0,
                super::super::RoundControl {
                    guards: &callbacks,
                    stop: None
                },
                &mut None
            ),
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

    fn capture_rows<const N: usize>(
        snapshot: super::super::RoundSnapshot<'_, HostDictionary, N>,
    ) -> super::super::evidence::Capture<purrdf_lex::allocation::Resident, u64, N>
    where
        Arity<N>: StoreShape<N>,
    {
        super::super::evidence::Capture::new(
            crate::evidence::DerivationEvidence::with_admission(
                snapshot.rel.retained_rows(),
                purrdf_lex::allocation::Resident,
            ),
            crate::test_support::evidence_scope(),
        )
    }

    fn probe_counts<const N: usize>(store: &RelationStore<HostDictionary, N>) -> (usize, usize)
    where
        Arity<N>: StoreShape<N>,
    {
        (
            store.interner().lookups.load(Ordering::Relaxed),
            store.interner().resolves.load(Ordering::Relaxed),
        )
    }

    fn compare_buffers<const N: usize>(
        actual: &super::super::RoundBuffer<'_, u64, N>,
        expected: &super::super::RoundBuffer<'_, u64, N>,
    ) where
        Arity<N>: StoreShape<N>,
    {
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
        let counts = constant_pattern_counts(entry.plan, snapshot.rel, entry.delta).unwrap();
        let capture = || capture_rows(snapshot);
        let probes = || probe_counts(snapshot.rel);
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
                    compare_buffers(&current, &reference);
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
                compare_buffers(&buffer, &expected);
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

    fn phase_rules<const N: usize>(
        atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>,
        unit: ClauseAtom<N>,
    ) -> [DlClause<N>; 6]
    where
        Arity<N>: StoreShape<N>,
    {
        use crate::clause::HeadDisjunct;
        use crate::guard::Negation;
        [
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
        ]
    }

    fn check_rule_phases<const N: usize>(
        atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>,
        unit: ClauseAtom<N>,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let store = fixture::<N>();
        check_commit_phases(&atom);
        check_stratum_phases(&atom);
        let rules = phase_rules(atom, unit);
        check_round_phases(&rules, &store);
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
                    probe: None,
                    rel: &store,
                    depth: (&[0, 1, u32::MAX, 2, 3, 0, 0, 1, 2, 3][..]).into(),
                    assumed: &(0..10)
                        .map(|index| crate::id::RowId::from_index(index).unwrap())
                        .collect(),
                    delta_indexes: Some(&indexes),
                };
                for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                    check_whole_rule(entry, snapshot, strategy);
                }
            }
        }
    }

    fn check_whole_round<const N: usize>(
        entries: &[RuleEntry<'_, N>],
        snapshot: super::super::RoundSnapshot<'_, HostDictionary, N>,
        strategy: JoinStrategy,
        execution: super::super::RoundExecution,
        evidenced: bool,
    ) where
        Arity<N>: StoreShape<N>,
    {
        use super::super::{RoundExecution, evidence, reference_evaluate_round, round};
        let callbacks = Callbacks::default();
        let mut original = capture_rows(snapshot);
        let expected = reference_evaluate_round(
            entries,
            snapshot,
            execution,
            strategy,
            u64::MAX,
            &callbacks,
            evidenced
                .then_some(&mut original as &mut dyn evidence::RuleEvidence<HostDictionary, N>),
        )
        .unwrap();
        let calls = callbacks.calls.into_inner().unwrap();
        let mut owner = capture_rows(snapshot);
        let callbacks = Callbacks::default();
        let before = probe_counts(snapshot.rel);
        let mut frame = round::Frame::new(entries, snapshot).unwrap();
        frame
            .advance(
                snapshot,
                execution,
                strategy,
                u64::MAX,
                super::super::RoundControl {
                    guards: &callbacks,
                    stop: None,
                },
                &mut evidenced
                    .then_some(&mut owner as &mut dyn evidence::RuleEvidence<HostDictionary, N>),
            )
            .unwrap();
        assert!(frame.is_complete());
        let visits = frame.row_visits();
        let current = frame.into_buffer(snapshot.rel).unwrap();
        compare_buffers(&current, &expected);
        assert_eq!(current.join_steps, expected.join_steps);
        let after = probe_counts(snapshot.rel);
        let probes = (after.0 - before.0, after.1 - before.1);
        let steps = expected.join_steps;
        let serial = evidenced
            || matches!(execution, RoundExecution::Sequential)
            || entries.iter().any(|entry| entry.rule.is_guarded());
        for cut in 0..=steps + 1 {
            if serial && cut <= steps {
                let original_callbacks = Callbacks::default();
                let current_callbacks = Callbacks::default();
                let mut original_owner = capture_rows(snapshot);
                let mut current_owner = capture_rows(snapshot);
                let original = reference_evaluate_round(
                    entries,
                    snapshot,
                    execution,
                    strategy,
                    cut,
                    &original_callbacks,
                    evidenced.then_some(
                        &mut original_owner as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                    ),
                )
                .unwrap();
                let current = super::super::evaluate_round_with_evidence(
                    entries,
                    snapshot,
                    execution,
                    strategy,
                    cut,
                    super::super::RoundControl {
                        guards: &current_callbacks,
                        stop: None,
                    },
                    evidenced.then_some(
                        &mut current_owner as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                    ),
                )
                .unwrap();
                compare_buffers(&current, &original);
                assert_eq!(current.join_steps, original.join_steps);
                assert_eq!(current.join_refused, original.join_refused);
                assert_eq!(
                    current_callbacks.calls.into_inner().unwrap(),
                    original_callbacks.calls.into_inner().unwrap()
                );
                if evidenced {
                    assert_eq!(
                        current_owner.rows.workspace().canonical_index().unwrap(),
                        original_owner.rows.workspace().canonical_index().unwrap()
                    );
                }
            }
            let callbacks = Callbacks::default();
            let mut owner = capture_rows(snapshot);
            let before = probe_counts(snapshot.rel);
            let mut frame = round::Frame::new(entries, snapshot).unwrap();
            let pages = if cut > steps {
                vec![1; usize::try_from(steps + 1).unwrap()]
            } else {
                vec![cut, u64::MAX]
            };
            for (page, allowance) in pages.into_iter().enumerate() {
                if frame.is_complete() {
                    break;
                }
                // In particular, serial resumption skips later parallel-complete rules.
                let scheduling = if page == 0 {
                    execution
                } else {
                    match execution {
                        RoundExecution::Parallel => RoundExecution::Sequential,
                        RoundExecution::Sequential => RoundExecution::Parallel,
                    }
                };
                frame
                    .advance(
                        snapshot,
                        scheduling,
                        strategy,
                        allowance,
                        super::super::RoundControl {
                            guards: &callbacks,
                            stop: None,
                        },
                        &mut evidenced.then_some(
                            &mut owner as &mut dyn evidence::RuleEvidence<HostDictionary, N>,
                        ),
                    )
                    .unwrap();
            }
            assert!(frame.is_complete(), "round cut {cut}");
            assert_eq!(frame.row_visits(), visits, "round visits, cut {cut}");
            let actual = frame.into_buffer(snapshot.rel).unwrap();
            compare_buffers(&actual, &expected);
            assert_eq!(actual.join_steps, steps, "round work, cut {cut}");
            assert!(!actual.join_refused);
            let after = probe_counts(snapshot.rel);
            assert_eq!(
                (after.0 - before.0, after.1 - before.1),
                probes,
                "round dictionary probes, cut {cut}"
            );
            assert_eq!(callbacks.calls.into_inner().unwrap(), calls);
            if evidenced {
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
            }
        }
    }

    fn check_round_phases<const N: usize>(
        rules: &[DlClause<N>; 6],
        store: &RelationStore<HostDictionary, N>,
    ) where
        Arity<N>: StoreShape<N>,
    {
        use super::super::RoundExecution;
        let plans: Vec<_> = rules.iter().map(RulePlan::for_rule).collect();
        let runtimes: Vec<_> = rules
            .iter()
            .zip(&plans)
            .map(|(rule, plan)| RuleRuntime::new(rule, plan))
            .collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        pool.install(|| {
            check_round_callback_failure(rules, &plans, &runtimes, store);
            for selection in [&[0, 1, 2, 3, 4, 5][..], &[4, 5, 4][..]] {
                for delta in [Delta::all(10), Delta { lo: 6, hi: 8 }, Delta::all(0)] {
                    let entries: Vec<_> = selection
                        .iter()
                        .enumerate()
                        .map(|(index, &selected)| RuleEntry {
                            index,
                            rule: &rules[selected],
                            plan: &plans[selected],
                            runtime: &runtimes[selected],
                            delta,
                        })
                        .collect();
                    let snapshot = super::super::RoundSnapshot {
                        probe: None,
                        rel: store,
                        depth: (&[0, 1, u32::MAX, 2, 3, 0, 0, 1, 2, 3][..]).into(),
                        assumed: &(0..10)
                            .map(|index| crate::id::RowId::from_index(index).unwrap())
                            .collect(),
                        delta_indexes: None,
                    };
                    for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                        for execution in [RoundExecution::Sequential, RoundExecution::Parallel] {
                            for evidenced in [false, true] {
                                check_whole_round(
                                    &entries, snapshot, strategy, execution, evidenced,
                                );
                            }
                        }
                    }
                }
            }
        });
    }

    fn check_round_callback_failure<const N: usize>(
        rules: &[DlClause<N>],
        plans: &[RulePlan<N>],
        runtimes: &[RuleRuntime<N>],
        store: &RelationStore<HostDictionary, N>,
    ) where
        Arity<N>: StoreShape<N>,
    {
        use super::super::{RoundExecution, reference_evaluate_round, round};
        let entries = [0, 1].map(|index| RuleEntry {
            index,
            rule: &rules[0],
            plan: &plans[0],
            runtime: &runtimes[0],
            delta: Delta::all(10),
        });
        let snapshot = super::super::RoundSnapshot {
            probe: None,
            rel: store,
            depth: (&[0; 10][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: None,
        };
        let callbacks = || Callbacks {
            fail_seed: Some(2),
            ..Callbacks::default()
        };
        let original = callbacks();
        assert_eq!(
            reference_evaluate_round(
                &entries,
                snapshot,
                RoundExecution::Sequential,
                JoinStrategy::Planned,
                u64::MAX,
                &original,
                None
            )
            .unwrap_err(),
            EvalError::Source(SourceLost(17))
        );
        let resumed = callbacks();
        let mut frame = round::Frame::new(&entries, snapshot).unwrap();
        let error = (0..1000)
            .find_map(|page| {
                match frame.advance(
                    snapshot,
                    RoundExecution::Parallel,
                    JoinStrategy::Planned,
                    u64::from(page != 0),
                    super::super::RoundControl {
                        guards: &resumed,
                        stop: None,
                    },
                    &mut None,
                ) {
                    Err(error) => Some(error),
                    Ok(()) => {
                        assert!(!frame.is_complete(), "the second callback must refuse");
                        None
                    }
                }
            })
            .expect("one-credit progress must reach the second callback");
        assert_eq!(error, EvalError::Source(SourceLost(17)));
        assert_eq!(resumed.seeds.load(Ordering::Relaxed), 2);
        assert_eq!(
            resumed.calls.into_inner().unwrap(),
            original.calls.into_inner().unwrap()
        );
    }

    fn check_stratum_phases<const N: usize>(atom: &impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        use super::super::{
            BudgetResource, DerivationMode, EvalOptions, FixpointState, NoGuards, RoundExecution,
            StratumRun, compile, reference_run_stratum, stratum,
        };
        use crate::store::TermInterner;
        let program = compile(vec![
            DlClause::datalog(
                atom("x", "https://example.org/reach", "y"),
                vec![atom("x", "https://example.org/p", "y")],
            ),
            DlClause::datalog(
                atom("x", "https://example.org/reach", "z"),
                vec![
                    atom("x", "https://example.org/reach", "y"),
                    atom("y", "https://example.org/p", "z"),
                ],
            ),
            DlClause::datalog(
                atom("x", "https://example.org/safe", "y"),
                vec![
                    atom("x", "https://example.org/p", "y"),
                    atom("y", "https://example.org/reach", "x").into_negated(),
                ],
            ),
        ])
        .unwrap();
        assert!(program.stratum_count() >= 2);
        let mut seed = RelationStore::<TermInterner, N>::with_dictionary(TermInterner::default());
        for (left, right) in [("c", "d"), ("b", "c"), ("a", "b"), ("c", "b")] {
            let left = format!("<https://example.org/{left}>");
            let right = format!("<https://example.org/{right}>");
            let row = [
                left.as_str(),
                "<https://example.org/p>",
                right.as_str(),
                "<https://example.org/g>",
            ];
            seed.insert_row(std::array::from_fn(|position| row[position]))
                .unwrap();
        }
        for mode in [DerivationMode::ModelOnly, DerivationMode::Record] {
            let options = EvalOptions::default()
                .with_derivation_mode(mode)
                .with_max_join_steps(u64::MAX)
                .with_max_stored_facts(u64::MAX);
            let run = StratumRun {
                stop: None,
                guards: &NoGuards,
                execution: RoundExecution::Sequential,
                strategy: JoinStrategy::Planned,
            };
            let mut expected = FixpointState::seeded(seed.clone(), options);
            for index in 0..program.stratum_count() {
                reference_run_stratum(
                    &program,
                    program.runtimes(),
                    index,
                    &mut expected,
                    run,
                    None,
                )
                .unwrap();
            }
            let expected = expected.finish();
            for execution in [RoundExecution::Sequential, RoundExecution::Parallel] {
                for resource in [BudgetResource::JoinSteps, BudgetResource::StoredFacts] {
                    let end = match resource {
                        BudgetResource::JoinSteps => expected.work().join_steps(),
                        BudgetResource::StoredFacts => {
                            u64::try_from(expected.facts().row_count()).unwrap()
                        }
                        _ => unreachable!(),
                    };
                    let start = if resource == BudgetResource::StoredFacts {
                        u64::try_from(seed.row_count()).unwrap()
                    } else {
                        0
                    };
                    for cut in start..=end {
                        let mut state = FixpointState::seeded(seed.clone(), options);
                        if resource == BudgetResource::JoinSteps {
                            state.limits.join_steps = cut;
                        } else {
                            state.limits.stored_facts = cut;
                        }
                        let mut pauses = 0;
                        for index in 0..program.stratum_count() {
                            let mut frame =
                                stratum::Frame::new(&program, program.runtimes(), index, &state);
                            while !frame.is_complete() {
                                match frame.advance(
                                    &mut state,
                                    StratumRun { execution, ..run },
                                    &mut None,
                                ) {
                                    Ok(()) => assert!(frame.is_complete()),
                                    Err(EvalError::BudgetExhausted {
                                        resource: actual,
                                        report,
                                    }) => {
                                        assert_eq!(actual, resource);
                                        pauses += 1;
                                        assert!(pauses <= end + 1);
                                        // Cut zero walks every subsequent credit boundary;
                                        // the other cuts resume directly to completion.
                                        if resource == BudgetResource::JoinSteps {
                                            assert_eq!(report.join_steps(), state.join_steps + 1);
                                            assert_eq!(state.join_steps, state.limits.join_steps);
                                            state.limits.join_steps = if cut == start {
                                                state.join_steps + 1
                                            } else {
                                                u64::MAX
                                            };
                                        } else {
                                            assert_eq!(
                                                report.observed_stored_facts(),
                                                state.rel.row_count() + 1
                                            );
                                            state.limits.stored_facts = if cut == start {
                                                state.limits.stored_facts + 1
                                            } else {
                                                u64::MAX
                                            };
                                        }
                                        if index == 0 {
                                            assert!(state.rel.facts_sorted().iter().all(|fact| {
                                                fact.predicate != "<https://example.org/safe>"
                                            }));
                                        }
                                    }
                                    Err(error) => panic!("unexpected refusal: {error:?}"),
                                }
                            }
                        }
                        state.limits.join_steps = u64::MAX;
                        state.limits.stored_facts = u64::MAX;
                        let actual = state.finish();
                        assert_eq!(
                            actual.facts().facts_sorted(),
                            expected.facts().facts_sorted()
                        );
                        assert_eq!(actual.derivations(), expected.derivations());
                        assert_eq!(actual.work(), expected.work());
                        assert_eq!(actual.budget(), expected.budget());
                    }
                }
            }
        }
    }

    fn check_commit_phases<const N: usize>(atom: &impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        use super::super::{
            DerivationMode, EvalOptions, FixpointState, RoundExecution, RoundSnapshot, commit,
            compile, evaluate_round_with_evidence, reference_commit_round,
        };
        use crate::dictionary::DictionaryValue;
        use crate::store::TermInterner;
        use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};
        struct Fixed;
        impl<const N: usize> GuardEvaluator<TermInterner, N> for Fixed
        where
            Arity<N>: StoreShape<N>,
        {
            fn evaluate(
                &self,
                call: &GuardCall<'_, TermInterner, N>,
            ) -> Result<GuardRows, CallbackError> {
                assert_eq!(call.guard.name(), "fixed");
                Ok(vec![vec![DictionaryValue::Surface("\"generated\"".into())]])
            }
        }
        let program = compile(vec![
            DlClause::datalog(
                atom("a", "https://example.org/generated", "z"),
                vec![atom("a", "https://example.org/p", "b")],
            )
            .with_guards(vec![Guard::assign("fixed", Vec::new(), "z")]),
        ])
        .unwrap();
        let mut seed = RelationStore::<TermInterner, N>::with_dictionary(TermInterner::default());
        for subject in ["z", "a", "m"] {
            let subject = format!("<https://example.org/{subject}>");
            let row = [
                subject.as_str(),
                "<https://example.org/p>",
                "<https://example.org/value>",
                "<https://example.org/g>",
            ];
            seed.insert_row(std::array::from_fn(|position| row[position]))
                .unwrap();
        }
        let (clause, plan) = program.rule_entry(0);
        let runtime = &program.runtimes()[0];
        let entry = RuleEntry {
            index: 0,
            rule: clause,
            plan,
            runtime,
            delta: Delta::all(seed.row_slots()),
        };
        let round = |state: &FixpointState<TermInterner, N>| {
            evaluate_round_with_evidence(
                &[entry],
                RoundSnapshot {
                    probe: None,
                    rel: &state.rel,
                    depth: state.depths(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: None,
                },
                RoundExecution::Sequential,
                JoinStrategy::Planned,
                u64::MAX,
                super::super::RoundControl {
                    guards: &Fixed,
                    stop: None,
                },
                None,
            )
            .unwrap()
        };
        let compare = |actual: &FixpointState<TermInterner, N>,
                       expected: &FixpointState<TermInterner, N>| {
            assert_eq!(
                actual.rel.facts_in_row_order(),
                expected.rel.facts_in_row_order()
            );
            assert_eq!(actual.depth, expected.depth);
            assert_eq!(actual.derivations, expected.derivations);
            assert_eq!(actual.committed_facts, expected.committed_facts);
            assert_eq!(actual.report(), expected.report());
            assert_eq!(actual.generating_rules, expected.generating_rules);
        };
        for mode in [DerivationMode::ModelOnly, DerivationMode::Record] {
            let options = EvalOptions::default()
                .with_derivation_mode(mode)
                .with_max_stored_facts(6);
            let mut expected = FixpointState::seeded(seed.clone(), options);
            let original = round(&expected);
            assert_eq!(original.entries.len(), 3);
            assert!(original.generates_terms);
            expected.join_steps = original.join_steps;
            let generating = original.generating_rules.clone();
            reference_commit_round(original, &mut expected).unwrap();
            expected.term_generating_rounds += 1;
            expected.generating_rules = generating;
            for cut in 0..=3 {
                let mut state = FixpointState::seeded(seed.clone(), options);
                let original = round(&state);
                state.join_steps = original.join_steps;
                let mut frame = commit::Frame::new(original, &state).unwrap();
                state.limits.stored_facts = 3 + cut;
                let result = frame.advance(&mut state);
                assert_eq!(state.rel.row_count(), usize::try_from(3 + cut).unwrap());
                if cut < 3 {
                    let EvalError::BudgetExhausted { resource, report } = result.unwrap_err()
                    else {
                        panic!("the first unadmitted winner must retain its fact refusal")
                    };
                    assert_eq!(resource, super::super::BudgetResource::StoredFacts);
                    assert_eq!(
                        report.observed_stored_facts(),
                        usize::try_from(4 + cut).unwrap()
                    );
                    assert!(!frame.is_complete());
                    assert_eq!(state.term_generating_rounds, 0);
                    state.limits.stored_facts = 6;
                    frame.advance(&mut state).unwrap();
                } else {
                    result.unwrap();
                }
                assert!(frame.is_complete());
                compare(&state, &expected);
                frame.advance(&mut state).unwrap();
                compare(&state, &expected);
            }
            let mut state = FixpointState::seeded(seed.clone(), options);
            let original = round(&state);
            state.join_steps = original.join_steps;
            let mut frame = commit::Frame::new(original, &state).unwrap();
            let terms = state.rel.interner().term_count();
            state.rel.limit_terms_for_test(Some(terms + 1));
            assert_eq!(
                frame.advance(&mut state),
                Err(EvalError::Storage(StorageError::Capacity(
                    CapacityError::new(
                        CapacitySpace::DatalogTerms,
                        u128::from(terms) + 2,
                        u128::from(terms) + 1,
                    )
                )))
            );
            assert_eq!(state.rel.row_count(), 3);
            assert_eq!(state.rel.interner().term_count(), terms);
            assert_eq!(state.term_generating_rounds, 0);
            assert!(!frame.is_complete());
            state.rel.limit_terms_for_test(None);
            frame.advance(&mut state).unwrap();
            compare(&state, &expected);
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
