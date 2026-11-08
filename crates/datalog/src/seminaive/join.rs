// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned operator and semi-naive anchor progress through the native join kernels.

use super::{
    AtomOperator, Delta, JoinGroup, JoinScan, JoinSnapshot, JoinStrategy, LeapfrogRun, RulePlan,
    Scan, SlotSolution, StepGovernor, TermDictionary, TermId, TermIdentity, atom,
    constant_pattern_count, constant_pattern_counts, cycle, decomposition_seed, delta_can_match,
    scan_for,
};
use crate::store::{Arity, StoreShape};

/// Coordinates always address the original immutable rule plan.
#[derive(Debug, Clone, Copy)]
pub(super) enum Step {
    Atom {
        operator: usize,
        access: atom::Access,
    },
    Cycle {
        group: usize,
        scan: JoinScan,
    },
}

impl Step {
    /// Resolve an authored atom coordinate once when preparing this sequence.
    pub(super) fn authored_atom<const N: usize>(
        plan: &RulePlan<N>,
        positive: usize,
        access: atom::Access,
    ) -> Self {
        Self::Atom {
            operator: plan
                .operators()
                .iter()
                .position(|operator| operator.positive_position() == positive)
                .expect("the original plan has every authored atom"),
            access,
        }
    }
}

/// The original scan, seed and binding law of a selected positive region.
#[derive(Debug, Clone, Copy)]
pub(super) struct Selection {
    pub(super) scan: JoinScan,
    pub(super) seed: Option<usize>,
    pub(super) cyclic: bool,
    pub(super) rebound: bool,
}

/// Prepare the same physical groups for whole bodies, guard segments and factors.
/// The boolean records the extra seed source a certified cycle captures again.
pub(super) fn selected_steps<const N: usize>(
    plan: &RulePlan<N>,
    selection: Selection,
    includes: impl Fn(&AtomOperator<N>) -> bool,
) -> (Vec<Step>, bool) {
    let mut steps = Vec::new();
    let seed_in_cycle = selection.cyclic
        && selection.seed.is_some_and(|seed| {
            plan.join_groups().iter().any(|group| {
                matches!(group, JoinGroup::Leapfrog(cycle)
            if cycle.atoms().iter().any(|atom| atom.positive_position() == seed))
            })
        });
    let access = |positive| {
        let scan = selection.scan.at(positive);
        if selection.rebound {
            atom::Access::Rebound(scan)
        } else {
            atom::Access::Planned(scan)
        }
    };
    if let Some(seed) = selection.seed {
        steps.push(Step::authored_atom(
            plan,
            seed,
            atom::Access::Rebound(Scan::Delta),
        ));
    }
    if selection.cyclic {
        for (index, group) in plan.join_groups().iter().enumerate() {
            let first = match group {
                JoinGroup::Binary(atom) => atom.positive_position(),
                JoinGroup::Leapfrog(cycle) => cycle.atoms()[0].positive_position(),
            };
            if !includes(plan.operator_at(first)) {
                continue;
            }
            match group {
                JoinGroup::Binary(atom) => {
                    let positive = atom.positive_position();
                    if selection.seed != Some(positive) {
                        steps.push(Step::authored_atom(plan, positive, access(positive)));
                    }
                }
                JoinGroup::Leapfrog(_) => steps.push(Step::Cycle {
                    group: index,
                    scan: selection.scan,
                }),
            }
        }
    } else {
        for (index, operator) in plan.operators().iter().enumerate() {
            let positive = operator.positive_position();
            if includes(operator) && selection.seed != Some(positive) {
                steps.push(Step::Atom {
                    operator: index,
                    access: access(positive),
                });
            }
        }
    }
    (steps, seed_in_cycle)
}

#[derive(Debug)]
enum Kernel<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Atom(atom::Frame<I, N>),
    Cycle(cycle::Frame<I, N>),
}

/// One original positive segment, retaining its current operator until complete.
#[derive(Debug)]
pub(super) struct Sequence<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    steps: Vec<Step>,
    next: usize,
    partial: Vec<SlotSolution<I, N>>,
    active: Option<Kernel<I, N>>,
    #[cfg(test)]
    started: u64,
}

impl<I: TermIdentity, const N: usize> Sequence<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new(steps: Vec<Step>, inputs: Vec<SlotSolution<I, N>>) -> Self {
        let next = if inputs.is_empty() { steps.len() } else { 0 };
        Self {
            steps,
            next,
            partial: inputs,
            active: None,
            #[cfg(test)]
            started: 0,
        }
    }

    pub(super) fn is_complete(&self) -> bool {
        self.next == self.steps.len()
    }

    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        if self.is_complete() {
            self.partial
        } else {
            Vec::new()
        }
    }

    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        plan: &RulePlan<N>,
        snapshot: JoinSnapshot<'_, D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), D::Error> {
        snapshot.rel.interner().check_read()?;
        while !self.is_complete() {
            let step = self.steps[self.next];
            if self.active.is_none() {
                if self.partial.is_empty() {
                    self.next = self.steps.len();
                    break;
                }
                if governor.spent() {
                    break;
                }
                let inputs = std::mem::take(&mut self.partial);
                self.active = Some(match step {
                    Step::Atom { operator, .. } => Kernel::Atom(atom::Frame::new(
                        &plan.operators()[operator],
                        snapshot.rel,
                        inputs,
                    )?),
                    Step::Cycle { .. } => Kernel::Cycle(cycle::Frame::new(inputs)),
                });
                #[cfg(test)]
                {
                    self.started += 1;
                }
            }
            let complete = match (
                step,
                self.active.as_mut().expect("the operator was prepared"),
            ) {
                (Step::Atom { operator, access }, Kernel::Atom(frame)) => {
                    frame.advance_selected(
                        &plan.operators()[operator],
                        snapshot.rel,
                        snapshot.delta,
                        snapshot.indexes,
                        access,
                        governor,
                    )?;
                    frame.is_complete()
                }
                (Step::Cycle { group, scan }, Kernel::Cycle(frame)) => {
                    let JoinGroup::Leapfrog(cycle) = &plan.join_groups()[group] else {
                        unreachable!("a cycle step names its original certified group")
                    };
                    frame.advance(
                        &LeapfrogRun {
                            plan,
                            cycle,
                            scan,
                            rel: snapshot.rel,
                            delta: snapshot.delta,
                            indexes: snapshot.indexes,
                        },
                        governor,
                    )?;
                    frame.is_complete()
                }
                _ => unreachable!("an active native frame retains its original operator"),
            };
            if !complete {
                break;
            }
            self.partial = match self.active.take().expect("the operator completed") {
                Kernel::Atom(frame) => frame.into_output(),
                Kernel::Cycle(frame) => frame.into_output(),
            };
            self.next += 1;
        }
        snapshot.rel.interner().check_read()
    }
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Unit,
    Binary { base: bool },
    Hybrid,
}

#[derive(Debug)]
struct Anchor<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    sequence: Sequence<I, N>,
    reordered: bool,
    seed_in_cycle: bool,
}

/// A whole positive body: its frozen advice, anchors and current native sequence.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    mode: Mode,
    anchors: std::ops::Range<usize>,
    unit_pending: bool,
    full_counts: Vec<Option<usize>>,
    active: Option<Anchor<I, N>>,
    output: Vec<SlotSolution<I, N>>,
    #[cfg(test)]
    started: u64,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new<D: TermDictionary<Id = I>>(
        plan: &RulePlan<N>,
        snapshot: JoinSnapshot<'_, D, N>,
        strategy: JoinStrategy,
        full_counts: Option<&[Option<usize>]>,
    ) -> Result<Self, D::Error> {
        snapshot.rel.interner().check_read()?;
        // Advice was frozen before any task could warm an access path. Own that
        // same small table across suspension rather than recomputing its answers.
        let full_counts = if let Some(seed) = snapshot.seed {
            plan.operators()
                .iter()
                .map(|operator| constant_pattern_count(operator, snapshot.rel, None, Some(seed)))
                .collect::<Result<_, _>>()?
        } else {
            match full_counts {
                Some(counts) => counts.to_vec(),
                None => constant_pattern_counts(plan, snapshot.rel, snapshot.delta)?,
            }
        };
        let base = plan
            .operators()
            .iter()
            .any(|operator| operator.shape().read_source() == crate::clause::ReadSource::Base);
        let mode = if plan.positive().is_empty() {
            Mode::Unit
        } else if !base && plan.has_cyclic_subplan() && matches!(strategy, JoinStrategy::Planned) {
            Mode::Hybrid
        } else {
            Mode::Binary { base }
        };
        let anchors = match mode {
            Mode::Unit => 0..0,
            Mode::Binary { base: true } => 0..1,
            Mode::Binary { base: false } | Mode::Hybrid => {
                snapshot.delta_positions(plan.positive().len())
            }
        };
        Ok(Self {
            mode,
            anchors,
            unit_pending: matches!(mode, Mode::Unit),
            full_counts,
            active: None,
            output: Vec::new(),
            #[cfg(test)]
            started: 0,
        })
    }

    pub(super) fn is_complete(&self) -> bool {
        !self.unit_pending && self.anchors.is_empty() && self.active.is_none()
    }

    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        self.output
    }

    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        plan: &RulePlan<N>,
        snapshot: JoinSnapshot<'_, D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), D::Error> {
        snapshot.rel.interner().check_read()?;
        if self.unit_pending {
            if governor.charge() {
                self.output.push(snapshot.initial(plan));
                self.unit_pending = false;
            }
            return snapshot.rel.interner().check_read();
        }
        loop {
            if self.active.is_none() {
                if governor.spent() {
                    break;
                }
                let Some(anchor) = self.anchors.next() else {
                    break;
                };
                let base = matches!(self.mode, Mode::Binary { base: true });
                let operator = match self.mode {
                    Mode::Binary { .. } => &plan.operators()[anchor],
                    Mode::Hybrid => plan.operator_at(anchor),
                    Mode::Unit => unreachable!("the empty conjunction has no atom anchors"),
                };
                if !base && !delta_can_match(operator, snapshot)? {
                    continue;
                }
                self.active = Some(self.prepare(plan, snapshot, anchor)?);
            }
            let anchor = self.active.as_mut().expect("the anchor was prepared");
            anchor.sequence.advance(plan, snapshot, governor)?;
            if !anchor.sequence.is_complete() {
                break;
            }
            let anchor = self.active.take().expect("the anchor completed");
            #[cfg(test)]
            {
                self.started += anchor.sequence.started;
            }
            let mut partial = anchor.sequence.into_output();
            for solution in &mut partial {
                if anchor.seed_in_cycle {
                    solution.sources.remove(0);
                }
                if anchor.reordered {
                    solution
                        .sources
                        .sort_unstable_by_key(|source| source.body_index);
                } else {
                    let swaps = match self.mode {
                        Mode::Binary { .. } => plan.operator_source_order_swaps(),
                        Mode::Hybrid => plan.hybrid_source_order_swaps(),
                        Mode::Unit => unreachable!("the empty conjunction has no source rows"),
                    };
                    for &(left, right) in swaps {
                        solution.sources.swap(left, right);
                    }
                }
            }
            self.output.extend(partial);
        }
        snapshot.rel.interner().check_read()
    }

    fn prepare<D: TermDictionary<Id = I>>(
        &self,
        plan: &RulePlan<N>,
        snapshot: JoinSnapshot<'_, D, N>,
        anchor: usize,
    ) -> Result<Anchor<I, N>, D::Error> {
        let mut steps = Vec::new();
        let (reordered, seed_in_cycle) = match self.mode {
            Mode::Binary { base } => {
                let partial = !base && snapshot.delta != Delta::all(snapshot.rel.row_slots());
                let bound = (!base && !partial && snapshot.seed.is_some())
                    .then(|| {
                        self.full_counts
                            .iter()
                            .enumerate()
                            .filter_map(|(position, count)| count.map(|count| (count, position)))
                            .min()
                            .map(|(_, position)| position)
                    })
                    .flatten();
                let seed = if partial {
                    decomposition_seed(plan, snapshot, anchor, &self.full_counts)?
                } else {
                    bound.unwrap_or(anchor)
                };
                let reordered = partial || bound.is_some_and(|position| position != 0);
                let order = [seed, anchor]
                    .into_iter()
                    .enumerate()
                    .filter(|&(index, position)| reordered && (index == 0 || position != seed))
                    .map(|(_, position)| position)
                    .chain((0..plan.operators().len()).filter(|&position| {
                        !reordered || (position != seed && position != anchor)
                    }));
                for operator in order {
                    let scan = if base {
                        Scan::Full
                    } else {
                        scan_for(operator, anchor)
                    };
                    steps.push(Step::Atom {
                        operator,
                        access: if reordered || snapshot.seed.is_some() {
                            atom::Access::Rebound(scan)
                        } else {
                            atom::Access::Planned(scan)
                        },
                    });
                }
                (reordered, false)
            }
            Mode::Hybrid => {
                let reordered = snapshot.delta != Delta::all(snapshot.rel.row_slots());
                let selection = Selection {
                    scan: JoinScan::DeltaPosition(anchor),
                    seed: reordered.then_some(anchor),
                    cyclic: true,
                    rebound: reordered || snapshot.seed.is_some(),
                };
                let (selected, seed_in_cycle) = selected_steps(plan, selection, |_| true);
                steps = selected;
                (reordered, seed_in_cycle)
            }
            Mode::Unit => unreachable!("the empty conjunction needs no sequence"),
        };
        Ok(Anchor {
            sequence: Sequence::new(steps, vec![snapshot.initial(plan)]),
            reordered,
            seed_in_cycle,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
    use crate::seminaive::DeltaIndexes;
    use crate::store::RelationStore;
    use crate::test_support::{HostDictionary, SourceLost};
    use std::sync::atomic::Ordering;

    fn check_cuts<const N: usize>(
        plan: &RulePlan<N>,
        snapshot: JoinSnapshot<'_, HostDictionary, N>,
        strategy: JoinStrategy,
    ) -> Vec<SlotSolution<u64, N>>
    where
        Arity<N>: StoreShape<N>,
    {
        let lookups = || snapshot.rel.interner().lookups.load(Ordering::Relaxed);
        let before = lookups();
        let mut reference = Frame::new(plan, snapshot, strategy, None).unwrap();
        let mut governor = StepGovernor::new(u64::MAX);
        reference.advance(plan, snapshot, &mut governor).unwrap();
        assert!(reference.is_complete());
        let steps = governor.pool.used.load(Ordering::Relaxed);
        let visits = governor.row_visits;
        let starts = reference.started;
        let probes = lookups() - before;
        let expected = reference.into_output();
        for cut in 0..=steps {
            let before = lookups();
            let mut frame = Frame::new(plan, snapshot, strategy, None).unwrap();
            let mut first = StepGovernor::new(cut);
            frame.advance(plan, snapshot, &mut first).unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                frame.advance(plan, snapshot, &mut rest).unwrap();
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
            assert_eq!(frame.started, starts, "native frame starts, cut {cut}");
            assert_eq!(
                lookups() - before,
                probes,
                "original dictionary probes, cut {cut}"
            );
            assert_eq!(
                frame.into_output(),
                expected,
                "bindings and authored sources, cut {cut}"
            );
        }
        let before = lookups();
        let mut frame = Frame::new(plan, snapshot, strategy, None).unwrap();
        let (mut resumed_steps, mut resumed_visits) = (0, 0);
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame.advance(plan, snapshot, &mut page).unwrap();
            let used = page.pool.used.load(Ordering::Relaxed);
            resumed_steps += used;
            resumed_visits += page.row_visits;
            assert!(used != 0 || frame.is_complete());
        }
        assert_eq!((resumed_steps, resumed_visits), (steps, visits));
        assert_eq!(frame.started, starts);
        assert_eq!(lookups() - before, probes);
        assert_eq!(frame.into_output(), expected);
        expected
    }

    fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom<3> {
        ClauseAtom::triple(
            ClauseTerm::var(subject),
            ClauseTerm::iri(predicate),
            ClauseTerm::var(object),
        )
    }

    fn fixture() -> RelationStore<HostDictionary, 3> {
        let dictionary = HostDictionary {
            surfaces: [
                (7, "<https://example.org/p>".to_owned()),
                (11, "<https://example.org/q>".to_owned()),
                (13, "<https://example.org/r>".to_owned()),
                (17, "<https://example.org/seed>".to_owned()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        };
        let mut store = RelationStore::with_dictionary(dictionary);
        let node = |i: usize| match i {
            0 => 0,
            1 => (1 << 54) + 1,
            2 => u64::MAX,
            _ => (1 << 40) + i as u64,
        };
        for i in 0..7 {
            for j in 0..7 {
                store.insert_ids([node(i), 7, node(j)]).unwrap();
            }
        }
        for i in 0..7 {
            store.insert_ids([node(i), 11, node((i + 1) % 7)]).unwrap();
        }
        for i in 0..7 {
            store.insert_ids([node(i), 13, node((i + 2) % 7)]).unwrap();
        }
        for i in 0..7 {
            store.insert_ids([1 << 62, 17, node(i)]).unwrap();
        }
        // Seal multiple subject/object batches without introducing another triangle.
        for i in 0..90 {
            store.insert_ids([node(i + 20), 7, node(i + 200)]).unwrap();
        }
        store
    }

    #[test]
    fn native_binding_seeds_constrain_original_joins_at_every_cut() {
        let store = fixture();
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let binary = vec![
            atom("a", "https://example.org/p", "b"),
            atom("b", "https://example.org/q", "c"),
        ];
        let mut cyclic = binary.clone();
        cyclic.push(atom("c", "https://example.org/r", "a"));
        let mut hybrid = vec![atom("root", "https://example.org/seed", "a")];
        hybrid.extend(cyclic.clone());
        for body in [binary, cyclic, hybrid] {
            let rule = DlClause::datalog(atom("a", "https://example.org/result", "c"), body);
            let plan = RulePlan::for_rule(&rule);
            for delta in [Delta::all(store.row_slots()), Delta { lo: 40, hi: 60 }] {
                let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
                let snapshot = JoinSnapshot {
                    seed: None,
                    rel: &store,
                    delta,
                    indexes: &indexes,
                };
                for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                    let all = check_cuts(&plan, snapshot, strategy);
                    for slot in 0..plan.variables().len() {
                        for value in [0, (1 << 54) + 1, u64::MAX, u64::MAX - 1] {
                            let mut seed = SlotSolution::empty(plan.variables().len());
                            seed.bindings[slot] = Some(value);
                            let mut expected = all
                                .iter()
                                .filter(|solution| solution.get(slot) == Some(value))
                                .cloned()
                                .collect::<Vec<_>>();
                            let mut actual = check_cuts(
                                &plan,
                                JoinSnapshot {
                                    seed: Some(&seed),
                                    ..snapshot
                                },
                                strategy,
                            );
                            let order = |solutions: &mut Vec<SlotSolution<u64, 3>>| {
                                solutions.sort_unstable_by(|left, right| {
                                    left.bindings.cmp(&right.bindings)
                                });
                            };
                            order(&mut expected);
                            order(&mut actual);
                            assert_eq!(actual, expected, "native slot {slot}, value {value}");
                        }
                    }
                }
            }
        }
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    #[test]
    fn positive_join_frames_resume_across_operators_and_delta_anchors() {
        let store = fixture();
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let binary = vec![
            atom("a", "https://example.org/p", "b"),
            atom("b", "https://example.org/q", "c"),
        ];
        let mut cyclic = binary.clone();
        cyclic.push(atom("c", "https://example.org/r", "a"));
        let mut hybrid = vec![atom("root", "https://example.org/seed", "a")];
        hybrid.extend(cyclic.clone());
        let mut saw_cycle = false;
        for body in [binary, cyclic, hybrid] {
            let rule = DlClause::datalog(atom("a", "https://example.org/result", "c"), body);
            let plan = RulePlan::for_rule(&rule);
            saw_cycle |= plan.has_cyclic_subplan();
            for delta in [
                Delta::all(store.row_slots()),
                Delta { lo: 40, hi: 60 },
                Delta { lo: 0, hi: 1 },
                Delta {
                    lo: 70,
                    hi: store.row_count() as u64,
                },
                Delta::all(0),
            ] {
                let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
                for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
                    let output = check_cuts(
                        &plan,
                        JoinSnapshot {
                            seed: None,
                            rel: &store,
                            delta,
                            indexes: &indexes,
                        },
                        strategy,
                    );
                    for solution in &output {
                        assert_eq!(
                            solution
                                .sources
                                .iter()
                                .map(|source| source.body_index)
                                .collect::<Vec<_>>(),
                            (0..rule.body().len()).collect::<Vec<_>>()
                        );
                    }
                    if delta == Delta::all(store.row_slots()) {
                        assert_eq!(output.len(), if plan.has_cyclic_subplan() { 7 } else { 49 });
                    }
                }
            }
        }
        assert!(saw_cycle);
        assert_eq!(
            store.interner().resolves.load(Ordering::Relaxed),
            resolves,
            "all operators retain original native identities without lexical projection"
        );
    }

    #[test]
    fn positive_join_frames_resume_quad_graph_bindings_with_original_ids() {
        const WIDE: u64 = (1 << 54) + 1;
        const NAMED: u64 = 1 << 63;
        const DEFAULT: u64 = u64::MAX - 1;
        let dictionary = HostDictionary {
            surfaces: [
                (7, "<https://example.org/p>".to_owned()),
                (11, "<https://example.org/q>".to_owned()),
                (NAMED, "<https://example.org/g>".to_owned()),
                (DEFAULT, String::new()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        };
        let mut store = RelationStore::<_, 4>::with_dictionary(dictionary);
        for graph in [NAMED, DEFAULT] {
            store.insert_ids([0, 7, WIDE, graph]).unwrap();
            store.insert_ids([WIDE, 11, u64::MAX, graph]).unwrap();
        }
        let atom = |subject, predicate, object| {
            ClauseAtom::quad(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
                ClauseTerm::var("g"),
            )
        };
        let rule = DlClause::datalog(
            atom("a", "https://example.org/result", "c"),
            vec![
                atom("a", "https://example.org/p", "b"),
                atom("b", "https://example.org/q", "c"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        for delta in [
            Delta::all(4),
            Delta { lo: 0, hi: 1 },
            Delta { lo: 2, hi: 3 },
            Delta::all(0),
        ] {
            let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
            let output = check_cuts(
                &plan,
                JoinSnapshot {
                    seed: None,
                    rel: &store,
                    delta,
                    indexes: &indexes,
                },
                JoinStrategy::Planned,
            );
            assert_eq!(
                output.len(),
                if delta == Delta::all(4) {
                    2
                } else {
                    usize::from(delta != Delta::all(0))
                }
            );
            for solution in output {
                assert_eq!(solution.sources[0].terms[..3], [0, 7, WIDE]);
                assert_eq!(solution.sources[1].terms[..3], [WIDE, 11, u64::MAX]);
                assert_eq!(solution.sources[0].terms[3], solution.sources[1].terms[3]);
            }
        }
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    #[test]
    fn positive_join_frames_keep_the_unit_binding_and_source_error_precedence() {
        let store = fixture();
        let unit = DlClause::datalog(
            ClauseAtom::triple(
                ClauseTerm::iri("https://example.org/a"),
                ClauseTerm::iri("https://example.org/result"),
                ClauseTerm::iri("https://example.org/c"),
            ),
            vec![],
        );
        let plan = RulePlan::for_rule(&unit);
        let delta = Delta::all(store.row_slots());
        let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
        let snapshot = JoinSnapshot {
            seed: None,
            rel: &store,
            delta,
            indexes: &indexes,
        };
        let output = check_cuts(&plan, snapshot, JoinStrategy::Planned);
        assert_eq!(output, [SlotSolution::empty(0)]);

        let rule = DlClause::datalog(
            atom("a", "https://example.org/result", "c"),
            vec![
                atom("a", "https://example.org/p", "b"),
                atom("b", "https://example.org/q", "c"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        let mut frame = Frame::new(&plan, snapshot, JoinStrategy::Planned, None).unwrap();
        frame
            .advance(&plan, snapshot, &mut StepGovernor::new(1))
            .unwrap();
        assert!(!frame.is_complete());
        let lookups = store.interner().lookups.load(Ordering::Relaxed);
        store.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            frame.advance(&plan, snapshot, &mut StepGovernor::new(0)),
            Err(SourceLost(99))
        );
        assert_eq!(store.interner().lookups.load(Ordering::Relaxed), lookups);
        // Missing constants complete without inventing source identities.
        let empty = RelationStore::<HostDictionary, 3>::with_dictionary(HostDictionary::default());
        let indexes = DeltaIndexes::build(&empty, [Delta::all(0)]).unwrap();
        let snapshot = JoinSnapshot {
            seed: None,
            rel: &empty,
            delta: Delta::all(0),
            indexes: &indexes,
        };
        assert_eq!(check_cuts(&plan, snapshot, JoinStrategy::Planned), []);
    }
}
