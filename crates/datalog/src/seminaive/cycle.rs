// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned progress through the original certified leapfrog traversal.

use super::{
    Delta, LeapfrogRun, SCAN_DELTA, SCAN_FULL, SCAN_OLD_ONLY, Scan, SlotSolution, SlotValue,
    StepGovernor, TermDictionary, TermId, TermIdentity, cycle_atom_partition, cycle_atom_slots,
    keep_row,
};
use crate::cursor::{VALUE_OBJECT, VALUE_SUBJECT, ValuePosition};
use crate::id::PartitionId;
use crate::store::{Arity, PartitionRef, Relation, StoreShape};

/// Traversal probes, absent from shipping state and code after optimization.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Work {
    #[cfg(test)]
    counts: core::cell::Cell<[u64; 5]>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Probe {
    ValueNext,
    ValueSeek,
    FilterSeek,
    PartitionLookup,
    LeafRow,
}

impl Work {
    #[inline]
    pub(super) fn record(&self, probe: Probe) {
        #[cfg(test)]
        {
            let mut counts = self.counts.get();
            counts[probe as usize] += 1;
            self.counts.set(counts);
        }
        #[cfg(not(test))]
        let _ = probe;
    }
}

/// One original value position and its distinct-value scan filter.
#[derive(Debug)]
struct Filtered<const SCAN: u8, const COLUMN: u8, I: TermIdentity> {
    rows: ValuePosition<COLUMN, I>,
    delta: Delta,
    current: Option<I>,
}

impl<const SCAN: u8, const COLUMN: u8, I: TermIdentity> Filtered<SCAN, COLUMN, I> {
    fn new(
        rel: &Relation<I>,
        other: Option<I>,
        delta: Delta,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Self {
        let mut cursor = Self {
            rows: ValuePosition::new(rel, other),
            delta,
            current: None,
        };
        cursor.fill(rel, None, work, governor);
        cursor
    }

    fn fill(
        &mut self,
        rel: &Relation<I>,
        prior: Option<I>,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I> {
        self.current = None;
        loop {
            if governor.stopped() {
                break;
            }
            work.record(Probe::ValueNext);
            let Some((value, row)) = self.rows.next(rel) else {
                break;
            };
            if keep_row::<SCAN>(self.delta, row) && Some(value) != prior {
                self.current = Some(value);
                break;
            }
        }
        self.current
    }

    fn seek(
        &mut self,
        rel: &Relation<I>,
        target: I,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I> {
        work.record(Probe::FilterSeek);
        if self.current.is_some_and(|value| value >= target) {
            return self.current;
        }
        work.record(Probe::ValueSeek);
        self.rows.seek(rel, target);
        self.fill(rel, None, work, governor)
    }

    fn advance(
        &mut self,
        rel: &Relation<I>,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I> {
        self.fill(rel, self.current, work, governor)
    }
}

/// Scan and orientation remain const-specialized at cursor construction.
#[derive(Debug)]
enum Values<I: TermIdentity> {
    DeltaSubject(Filtered<SCAN_DELTA, VALUE_SUBJECT, I>),
    FullSubject(Filtered<SCAN_FULL, VALUE_SUBJECT, I>),
    OldOnlySubject(Filtered<SCAN_OLD_ONLY, VALUE_SUBJECT, I>),
    DeltaObject(Filtered<SCAN_DELTA, VALUE_OBJECT, I>),
    FullObject(Filtered<SCAN_FULL, VALUE_OBJECT, I>),
    OldOnlyObject(Filtered<SCAN_OLD_ONLY, VALUE_OBJECT, I>),
}

#[derive(Debug)]
struct Cursor<I: TermIdentity> {
    partition: PartitionId,
    delta_source: bool,
    values: Values<I>,
}

impl<I: TermIdentity> Cursor<I> {
    /// Retain subject-ordered progress in this original partition and scan mode.
    fn subject<const N: usize>(
        partition: PartitionRef<'_, I, N>,
        other: Option<I>,
        scan: Scan,
        delta: Delta,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Self
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = partition.arrangement();
        Self {
            partition: partition.id(),
            delta_source: matches!(scan, Scan::Delta),
            values: match scan {
                Scan::Delta => {
                    Values::DeltaSubject(Filtered::new(rel, other, delta, work, governor))
                }
                Scan::Full => Values::FullSubject(Filtered::new(rel, other, delta, work, governor)),
                Scan::OldOnly => {
                    Values::OldOnlySubject(Filtered::new(rel, other, delta, work, governor))
                }
            },
        }
    }

    /// Retain object-ordered progress in this original partition and scan mode.
    fn object<const N: usize>(
        partition: PartitionRef<'_, I, N>,
        other: Option<I>,
        scan: Scan,
        delta: Delta,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Self
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = partition.arrangement();
        Self {
            partition: partition.id(),
            delta_source: matches!(scan, Scan::Delta),
            values: match scan {
                Scan::Delta => {
                    Values::DeltaObject(Filtered::new(rel, other, delta, work, governor))
                }
                Scan::Full => Values::FullObject(Filtered::new(rel, other, delta, work, governor)),
                Scan::OldOnly => {
                    Values::OldOnlyObject(Filtered::new(rel, other, delta, work, governor))
                }
            },
        }
    }

    /// Inspect the retained distinct value without advancing or rendering it.
    fn current(&self) -> Option<I> {
        match &self.values {
            Values::DeltaSubject(cursor) => cursor.current,
            Values::FullSubject(cursor) => cursor.current,
            Values::OldOnlySubject(cursor) => cursor.current,
            Values::DeltaObject(cursor) => cursor.current,
            Values::FullObject(cursor) => cursor.current,
            Values::OldOnlyObject(cursor) => cursor.current,
        }
    }

    /// Reborrow the original immutable arrangement, never a reconstructed trie.
    fn relation<'r, D: TermDictionary<Id = I>, const N: usize>(
        &self,
        run: &'r LeapfrogRun<'_, D, N>,
    ) -> &'r Relation<I>
    where
        Arity<N>: StoreShape<N>,
    {
        let partition = run.rel.partition_with_id(self.partition);
        let partition = if self.delta_source {
            run.indexes
                .get(run.delta, run.rel.row_slots())
                .map_or(partition, |delta| {
                    delta
                        .project(partition)
                        .expect("the retained cursor belongs to its original delta")
                })
        } else {
            partition
        };
        partition.arrangement()
    }

    fn seek<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        run: &LeapfrogRun<'_, D, N>,
        target: I,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I>
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = self.relation(run);
        match &mut self.values {
            Values::DeltaSubject(cursor) => cursor.seek(rel, target, work, governor),
            Values::FullSubject(cursor) => cursor.seek(rel, target, work, governor),
            Values::OldOnlySubject(cursor) => cursor.seek(rel, target, work, governor),
            Values::DeltaObject(cursor) => cursor.seek(rel, target, work, governor),
            Values::FullObject(cursor) => cursor.seek(rel, target, work, governor),
            Values::OldOnlyObject(cursor) => cursor.seek(rel, target, work, governor),
        }
    }

    fn advance<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        run: &LeapfrogRun<'_, D, N>,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I>
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = self.relation(run);
        match &mut self.values {
            Values::DeltaSubject(cursor) => cursor.advance(rel, work, governor),
            Values::FullSubject(cursor) => cursor.advance(rel, work, governor),
            Values::OldOnlySubject(cursor) => cursor.advance(rel, work, governor),
            Values::DeltaObject(cursor) => cursor.advance(rel, work, governor),
            Values::FullObject(cursor) => cursor.advance(rel, work, governor),
            Values::OldOnlyObject(cursor) => cursor.advance(rel, work, governor),
        }
    }
}

/// The same sorted distinct-value intersection, retaining its actual cursors.
#[derive(Debug)]
struct Intersection<I: TermIdentity> {
    cursors: Vec<Cursor<I>>,
}

impl<I: TermIdentity> Intersection<I> {
    /// Find a common value, advancing the first cursor before returning it.
    fn next<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        run: &LeapfrogRun<'_, D, N>,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut target = self.cursors.iter().filter_map(Cursor::current).max()?;
        loop {
            if governor.stopped() {
                return None;
            }
            let mut aligned = true;
            for cursor in &mut self.cursors {
                let value = cursor.seek(run, target, work, governor)?;
                if value > target {
                    target = value;
                    aligned = false;
                }
            }
            if aligned {
                self.cursors[0].advance(run, work, governor);
                return Some(target);
            }
        }
    }

    /// Check an external binding against every original native value cursor.
    fn contains<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        run: &LeapfrogRun<'_, D, N>,
        wanted: I,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> bool
    where
        Arity<N>: StoreShape<N>,
    {
        self.cursors
            .iter_mut()
            .all(|cursor| cursor.seek(run, wanted, work, governor) == Some(wanted))
    }
}

#[derive(Debug)]
enum Level<I: TermIdentity> {
    Fixed(Option<I>),
    Open(Intersection<I>),
}

impl<I: TermIdentity> Level<I> {
    fn next<D: TermDictionary<Id = I>, const N: usize>(
        &mut self,
        run: &LeapfrogRun<'_, D, N>,
        work: &Work,
        governor: &StepGovernor<'_>,
    ) -> Option<I>
    where
        Arity<N>: StoreShape<N>,
    {
        match self {
            Self::Fixed(value) => value.take(),
            Self::Open(intersection) => intersection.next(run, work, governor),
        }
    }
}

/// One cyclic component's native inputs, descent stack and uncommitted output.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    inputs: std::vec::IntoIter<SlotSolution<I, N>>,
    working: Option<SlotSolution<I, N>>,
    levels: Vec<Level<I>>,
    depth: usize,
    output: Vec<SlotSolution<I, N>>,
    work: Work,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Move the existing substitutions into the component's initial descent state.
    pub(super) fn new(inputs: Vec<SlotSolution<I, N>>) -> Self {
        Self {
            inputs: inputs.into_iter(),
            working: None,
            levels: Vec::new(),
            depth: 0,
            output: Vec::new(),
            work: Work::default(),
        }
    }

    /// Every input is exhausted and no assigned leaf remains pending.
    pub(super) fn is_complete(&self) -> bool {
        self.inputs.len() == 0 && self.working.is_none()
    }

    /// Move admitted substitutions and source rows without revisiting a cursor.
    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        self.output
    }

    /// Clear only a binding introduced by this descent; keep external bindings.
    fn backtrack<D: TermDictionary<Id = I>>(&mut self, run: &LeapfrogRun<'_, D, N>) {
        if self.depth == 0 {
            self.working = None;
            self.levels.clear();
        } else {
            self.depth -= 1;
            if matches!(self.levels[self.depth], Level::Open(_)) {
                let slot = run.cycle.variable_slots()[self.depth];
                self.working
                    .as_mut()
                    .expect("the descent has an input")
                    .bindings[slot] = None;
            }
        }
    }

    /// Drive until the next leaf credit refuses or every input is exhausted.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        run: &LeapfrogRun<'_, D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), D::Error> {
        run.rel.interner().check_read()?;
        while !governor.spent() {
            if self.working.is_none() {
                let Some(input) = self.inputs.next() else {
                    break;
                };
                if run
                    .cycle
                    .variable_slots()
                    .iter()
                    .any(|&slot| matches!(input.value(slot), SlotValue::Computed(_)))
                {
                    continue;
                }
                self.working = Some(input);
                self.depth = 0;
            }
            if self.depth == run.cycle.variable_slots().len() {
                if !governor.charge() {
                    break;
                }
                let working = self.working.as_mut().expect("the leaf has an input");
                let original = working.sources.len();
                if run.append_sources(working, &self.work)? {
                    self.output.push(working.clone());
                    working.sources.truncate(original);
                }
                self.backtrack(run);
                continue;
            }
            if self.levels.len() == self.depth {
                let input = self.working.as_ref().expect("the level has an input");
                match make_level(run, self.depth, input, &self.work, governor)? {
                    Some(level) => self.levels.push(level),
                    None => {
                        self.backtrack(run);
                        continue;
                    }
                }
            }
            let level = &mut self.levels[self.depth];
            if let Some(value) = level.next(run, &self.work, governor) {
                if matches!(level, Level::Open(_)) {
                    let slot = run.cycle.variable_slots()[self.depth];
                    let working = self.working.as_mut().expect("the level has an input");
                    debug_assert!(working.bindings[slot].is_none());
                    working.bindings[slot] = Some(value);
                }
                self.depth += 1;
            } else {
                self.levels.pop();
                self.backtrack(run);
            }
        }
        run.rel.interner().check_read()
    }
}

/// Prepare the original level intersection, preserving an external binding as fixed.
fn make_level<D: TermDictionary, const N: usize>(
    run: &LeapfrogRun<'_, D, N>,
    variable_position: usize,
    solution: &SlotSolution<D::Id, N>,
    work: &Work,
    governor: &StepGovernor<'_>,
) -> Result<Option<Level<D::Id>>, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    let variable_slot = run.cycle.variable_slots()[variable_position];
    let mut cursors = Vec::new();
    for &planned in run.cycle.atoms() {
        let operator = run.plan.operator_at(planned.positive_position());
        let shape = operator.shape();
        let (subject_slot, object_slot) = cycle_atom_slots(shape);
        if subject_slot != variable_slot && object_slot != variable_slot {
            continue;
        }
        let scan = run.scan.at(planned.positive_position());
        work.record(Probe::PartitionLookup);
        let Some(partition) =
            cycle_atom_partition(shape, run.rel, run.delta_run(planned.positive_position()))?
        else {
            return Ok(None);
        };
        let cursor = if subject_slot == variable_slot {
            Cursor::subject(
                partition,
                solution.get(object_slot),
                scan,
                run.delta,
                work,
                governor,
            )
        } else {
            Cursor::object(
                partition,
                solution.get(subject_slot),
                scan,
                run.delta,
                work,
                governor,
            )
        };
        cursors.push(cursor);
    }
    if cursors.is_empty() {
        return Ok(None);
    }
    let mut intersection = Intersection { cursors };
    Ok(Some(match solution.get(variable_slot) {
        Some(value) => Level::Fixed(
            intersection
                .contains(run, value, work, governor)
                .then_some(value),
        ),
        None => Level::Open(intersection),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
    use crate::plan::{JoinGroup, RulePlan};
    use crate::seminaive::{DeltaIndexes, JoinScan};
    use crate::store::{RelationStore, TermInterner};
    use crate::test_support::{HostDictionary, SourceLost};
    use std::sync::atomic::Ordering;

    fn check_cuts<D: TermDictionary, const N: usize>(
        run: &LeapfrogRun<'_, D, N>,
        inputs: &[SlotSolution<D::Id, N>],
    ) -> Vec<SlotSolution<D::Id, N>>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut reference = Frame::new(inputs.to_vec());
        let mut governor = StepGovernor::new(u64::MAX);
        reference.advance(run, &mut governor).unwrap();
        assert!(reference.is_complete());
        let steps = governor.pool.used.load(Ordering::Relaxed);
        let work = reference.work.clone();
        let expected = reference.into_output();
        for cut in 0..=steps {
            let mut frame = Frame::new(inputs.to_vec());
            let mut first = StepGovernor::new(cut);
            frame.advance(run, &mut first).unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                frame.advance(run, &mut rest).unwrap();
            }
            assert!(frame.is_complete());
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "successful credits, cut {cut}"
            );
            assert_eq!(frame.work, work, "native traversal, cut {cut}");
            assert_eq!(frame.into_output(), expected, "output, cut {cut}");
        }
        let mut frame = Frame::new(inputs.to_vec());
        let mut resumed_steps = 0;
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame.advance(run, &mut page).unwrap();
            let used = page.pool.used.load(Ordering::Relaxed);
            resumed_steps += used;
            assert!(used != 0 || frame.is_complete());
        }
        assert_eq!(resumed_steps, steps);
        assert_eq!(
            frame.work, work,
            "native traversal through one-credit pages"
        );
        assert_eq!(frame.into_output(), expected);
        expected
    }

    fn triangle() -> DlClause<3> {
        let atom = |subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        };
        DlClause::datalog(
            atom("a", "https://example.org/result", "c"),
            vec![
                atom("a", "https://example.org/p", "b"),
                atom("b", "https://example.org/q", "c"),
                atom("c", "https://example.org/r", "a"),
            ],
        )
    }

    #[test]
    fn cyclic_frames_resume_every_credit_cut_without_replaying_trie_work() {
        let mut store = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        let node = |i: usize| format!("<https://example.org/n{i}>");
        for i in 0..13 {
            for j in 0..13 {
                store
                    .insert_row([&node(i), "<https://example.org/p>", &node(j)])
                    .unwrap();
            }
            store
                .insert_row([&node(i), "<https://example.org/q>", &node((i + 1) % 13)])
                .unwrap();
            store
                .insert_row([&node(i), "<https://example.org/r>", &node((i + 2) % 13)])
                .unwrap();
        }
        let plan = RulePlan::for_rule(&triangle());
        let cycle = plan
            .join_groups()
            .iter()
            .find_map(|group| match group {
                JoinGroup::Leapfrog(cycle) => Some(cycle),
                JoinGroup::Binary(_) => None,
            })
            .unwrap();
        let empty = SlotSolution::empty(plan.variables().len());
        let indexes = DeltaIndexes::build(&store, [Delta::all(store.row_slots())]).unwrap();
        let full = LeapfrogRun {
            plan: &plan,
            cycle,
            scan: JoinScan::Uniform(Scan::Full),
            rel: &store,
            delta: Delta::all(store.row_slots()),
            indexes: &indexes,
        };
        let reference = check_cuts(&full, std::slice::from_ref(&empty));
        assert_eq!(reference.len(), 13, "the triangle's analytic cardinality");
        let mut inputs = vec![empty.clone(), empty];
        // Bind each descent level separately, then all levels. A prefix source must
        // survive every suspension and each leaf's temporary source expansion.
        for &slot in cycle.variable_slots() {
            let mut input = SlotSolution::empty(plan.variables().len());
            input.bindings[slot] = reference[0].bindings[slot];
            input.sources.push(reference[0].sources[0]);
            inputs.push(input);
        }
        inputs.push(reference[0].clone());
        let mut impossible = reference[0].clone();
        let slot = cycle.variable_slots()[0];
        impossible.bindings[slot] = reference[1].bindings[slot];
        inputs.push(impossible);
        let mut computed = SlotSolution::empty(plan.variables().len());
        computed
            .computed
            .push((slot, "<https://example.org/absent>".into()));
        inputs.push(computed);
        check_cuts(&full, &inputs);
        for delta in [
            Delta::all(store.row_slots()),
            Delta { lo: 70, hi: 150 },
            Delta::all(0),
        ] {
            let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
            for scan in [
                JoinScan::Uniform(Scan::Full),
                JoinScan::Uniform(Scan::Delta),
                JoinScan::Uniform(Scan::OldOnly),
                JoinScan::DeltaPosition(0),
                JoinScan::DeltaPosition(1),
                JoinScan::DeltaPosition(2),
            ] {
                check_cuts(
                    &LeapfrogRun {
                        scan,
                        delta,
                        indexes: &indexes,
                        ..full
                    },
                    &inputs,
                );
            }
        }
        let empty_store =
            RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        let no_indexes = DeltaIndexes::build(&empty_store, [Delta::all(0)]).unwrap();
        assert_eq!(
            check_cuts(
                &LeapfrogRun {
                    rel: &empty_store,
                    delta: Delta::all(0),
                    indexes: &no_indexes,
                    ..full
                },
                &[SlotSolution::empty(plan.variables().len())]
            ),
            []
        );
    }

    #[test]
    fn cyclic_frames_keep_native_identities_and_recheck_source_before_resuming() {
        const P: u64 = 7;
        const Q: u64 = 19;
        const R: u64 = 3;
        const WIDE: u64 = (1 << 54) + 1;
        const GRAPH: u64 = 1 << 63;
        let dictionary = HostDictionary {
            surfaces: [
                (P, "<https://example.org/p>".to_owned()),
                (Q, "<https://example.org/q>".to_owned()),
                (R, "<https://example.org/r>".to_owned()),
                (GRAPH, "<https://example.org/graph>".to_owned()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        };
        let mut store = RelationStore::<_, 4>::with_dictionary(dictionary);
        for row in [
            [0, P, WIDE, GRAPH],
            [WIDE, Q, u64::MAX, GRAPH],
            [u64::MAX, R, 0, GRAPH],
        ] {
            store.insert_ids(row).unwrap();
        }
        let atom = |subject, predicate, object| {
            ClauseAtom::quad(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
                ClauseTerm::iri("https://example.org/graph"),
            )
        };
        let rule = DlClause::datalog(
            atom("a", "https://example.org/result", "c"),
            vec![
                atom("a", "https://example.org/p", "b"),
                atom("b", "https://example.org/q", "c"),
                atom("c", "https://example.org/r", "a"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        let cycle = plan
            .join_groups()
            .iter()
            .find_map(|group| match group {
                JoinGroup::Leapfrog(cycle) => Some(cycle),
                JoinGroup::Binary(_) => None,
            })
            .unwrap();
        let delta = Delta::all(store.row_slots());
        let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
        let run = LeapfrogRun {
            plan: &plan,
            cycle,
            scan: JoinScan::Uniform(Scan::Full),
            rel: &store,
            delta,
            indexes: &indexes,
        };
        let inputs = vec![SlotSolution::empty(plan.variables().len())];
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let output = check_cuts(&run, &inputs);
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0]
                .sources
                .iter()
                .map(|source| source.terms)
                .collect::<Vec<_>>(),
            vec![
                [0, P, WIDE, GRAPH],
                [WIDE, Q, u64::MAX, GRAPH],
                [u64::MAX, R, 0, GRAPH]
            ]
        );
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
        let mut frame = Frame::new(inputs);
        frame.advance(&run, &mut StepGovernor::new(0)).unwrap();
        assert!(!frame.is_complete());
        let work = frame.work.clone();
        let lookups = store.interner().lookups.load(Ordering::Relaxed);
        store.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            frame.advance(&run, &mut StepGovernor::new(0)),
            Err(SourceLost(99))
        );
        assert_eq!(
            frame.work, work,
            "source loss is checked ahead of trie work"
        );
        assert_eq!(store.interner().lookups.load(Ordering::Relaxed), lookups);
    }
}
