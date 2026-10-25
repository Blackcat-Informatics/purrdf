// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The original indexed atom scan, with owned progress between source borrows.

use super::{
    AtomOperator, AtomShape, Bound, ConstantPositions, Delta, DeltaIndexes, INDEX_ANY, INDEX_BOTH,
    INDEX_OBJECT, INDEX_REBOUND, INDEX_SUBJECT, POSITION_OBJECT, POSITION_SUBJECT, PositionPlan,
    PositionValue, RelationStore, SCAN_DELTA, SCAN_FULL, SCAN_OLD_ONLY, SlotSolution, SlotValue,
    SourceRow, StepGovernor, TermDictionary, TermId, TermIdentity, constant_positions, keep_row,
};
use crate::cursor::RowPosition;
use crate::id::{PartitionId, RowId};
use crate::plan::IndexChoice;
use crate::store::{Arity, DeltaRun, PartitionPosition, StoreShape};

/// Original planned bounds or bounds rebound after a physical reordering.
#[derive(Debug, Clone, Copy)]
pub(super) enum Access {
    Planned(super::Scan),
    Rebound(super::Scan),
}

/// A matched row whose reservation may have paused before solution expansion.
type Pending<I, const N: usize> = ([I; N], RowId);

/// The original read authority and its once-selected native delta arrangement.
struct Source<'a, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    rel: &'a RelationStore<D, N>,
    run: Option<&'a DeltaRun<D::Id>>,
    delta: Delta,
}

/// One input's selected partitions and original row position.
#[derive(Debug)]
struct Scan<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    partitions: PartitionPosition<I, N>,
    bound: Bound<I>,
    active: Option<(PartitionId, RowPosition<I>)>,
    pending: Option<Pending<I, N>>,
}

/// The single atom kernel's input, output and native enumeration state.
///
/// The enclosing execution owns the unchanged program, store and delta indexes.
/// A resumed drive uses those same authorities; no source borrow is retained here.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    inputs: Vec<SlotSolution<I, N>>,
    output: Vec<SlotSolution<I, N>>,
    constants: ConstantPositions<I, N>,
    input: usize,
    scan: Option<Scan<I, N>>,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Hoist the original operator's constants once, even for an empty input.
    pub(super) fn new<D: TermDictionary<Id = I>>(
        operator: &AtomOperator<N>,
        rel: &RelationStore<D, N>,
        inputs: Vec<SlotSolution<I, N>>,
    ) -> Result<Self, D::Error> {
        let source = rel.read_source(operator.shape().read_source());
        Ok(Self {
            inputs,
            output: Vec::new(),
            constants: constant_positions(operator.shape(), source)?,
            input: 0,
            scan: None,
        })
    }

    /// Whether every input and selected row has completed.
    pub(super) fn is_complete(&self) -> bool {
        self.input == self.inputs.len()
    }

    /// Move the original output buffer; the round owner retains refusal evidence.
    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        self.output
    }

    /// Select the existing const-specialized kernel once per drive, never per row.
    pub(super) fn advance_selected<D: TermDictionary<Id = I>>(
        &mut self,
        operator: &AtomOperator<N>,
        rel: &RelationStore<D, N>,
        delta: Delta,
        indexes: &DeltaIndexes<I>,
        access: Access,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), D::Error> {
        let (scan, rebound) = match access {
            Access::Planned(scan) => (scan, false),
            Access::Rebound(scan) => (scan, true),
        };
        match scan {
            super::Scan::Delta => self
                .advance_index::<SCAN_DELTA, D>(operator, rel, delta, indexes, rebound, governor),
            super::Scan::Full => {
                self.advance_index::<SCAN_FULL, D>(operator, rel, delta, indexes, rebound, governor)
            }
            super::Scan::OldOnly => self.advance_index::<SCAN_OLD_ONLY, D>(
                operator, rel, delta, indexes, rebound, governor,
            ),
        }
    }

    fn advance_index<const SCAN: u8, D: TermDictionary<Id = I>>(
        &mut self,
        operator: &AtomOperator<N>,
        rel: &RelationStore<D, N>,
        delta: Delta,
        indexes: &DeltaIndexes<I>,
        rebound: bool,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), D::Error> {
        if rebound {
            return self.advance::<SCAN, INDEX_REBOUND, D>(operator, rel, delta, indexes, governor);
        }
        match operator.index() {
            IndexChoice::Any => {
                self.advance::<SCAN, INDEX_ANY, D>(operator, rel, delta, indexes, governor)
            }
            IndexChoice::Subject => {
                self.advance::<SCAN, INDEX_SUBJECT, D>(operator, rel, delta, indexes, governor)
            }
            IndexChoice::Object => {
                self.advance::<SCAN, INDEX_OBJECT, D>(operator, rel, delta, indexes, governor)
            }
            IndexChoice::Both => {
                self.advance::<SCAN, INDEX_BOTH, D>(operator, rel, delta, indexes, governor)
            }
        }
    }

    /// Drive the same operator until completion or a refused candidate credit.
    /// A refused match stays pending, so a fresh credit pool continues without
    /// another row visit, lookup of constants, or replay of an old input prefix.
    pub(super) fn advance<const SCAN: u8, const INDEX: u8, D: TermDictionary<Id = I>>(
        &mut self,
        operator: &AtomOperator<N>,
        rel: &RelationStore<D, N>,
        delta: Delta,
        indexes: &DeltaIndexes<I>,
        governor: &mut StepGovernor<'_>,
    ) -> Result<(), D::Error> {
        let rel = rel.read_source(operator.shape().read_source());
        rel.interner().check_read()?;
        let run = if SCAN == SCAN_DELTA {
            indexes.get(delta, rel.row_slots())
        } else {
            None
        };
        let source = Source { rel, run, delta };
        while self.input < self.inputs.len() && !governor.spent() {
            let input = &self.inputs[self.input];
            if self.scan.is_none() {
                self.scan = prepare::<INDEX, D, N>(operator.shape(), rel, input, &self.constants);
                if self.scan.is_none() {
                    self.input += 1;
                    continue;
                }
            }
            let scan = self.scan.as_mut().expect("the input scan was prepared");
            if !scan.advance::<SCAN, INDEX, D>(operator, &source, input, &mut self.output, governor)
            {
                break;
            }
            self.scan = None;
            self.input += 1;
        }
        // Original storage failures keep precedence over a resource boundary.
        rel.interner().check_read()
    }
}

/// Resolve one input's bounds with the original compiled index law.
fn prepare<const INDEX: u8, D: TermDictionary, const N: usize>(
    shape: &AtomShape<N>,
    rel: &RelationStore<D, N>,
    solution: &SlotSolution<D::Id, N>,
    constants: &ConstantPositions<D::Id, N>,
) -> Option<Scan<D::Id, N>>
where
    Arity<N>: StoreShape<N>,
{
    let mut values = [PositionValue::Free; N];
    for position in 0..N {
        values[position] = match (&shape.positions()[position], constants[position]) {
            (PositionPlan::Constant(_), Some(resolved)) => resolved,
            (PositionPlan::Constant(_), None) => {
                unreachable!("a constant position always has a hoisted resolution")
            }
            (PositionPlan::Bound(slot) | PositionPlan::Free(slot), _) if INDEX == INDEX_REBOUND => {
                match solution.value(*slot) {
                    SlotValue::Interned(id) => PositionValue::Known(id),
                    SlotValue::Computed(_) => PositionValue::Missing,
                    SlotValue::Unbound => PositionValue::Free,
                }
            }
            (PositionPlan::Bound(slot), _) => solution
                .get(*slot)
                .map_or(PositionValue::Missing, PositionValue::Known),
            (PositionPlan::Free(_), _) => PositionValue::Free,
        };
    }
    if values.contains(&PositionValue::Missing) {
        return None;
    }
    let bound = match INDEX {
        INDEX_REBOUND => match (
            values[POSITION_SUBJECT].known(),
            values[POSITION_OBJECT].known(),
        ) {
            (Some(subject), Some(object)) => Bound::Both(subject, object),
            (Some(subject), None) => Bound::Subject(subject),
            (None, Some(object)) => Bound::Object(object),
            (None, None) => Bound::Any,
        },
        INDEX_ANY => Bound::Any,
        INDEX_SUBJECT => Bound::Subject(values[POSITION_SUBJECT].known()?),
        INDEX_OBJECT => Bound::Object(values[POSITION_OBJECT].known()?),
        INDEX_BOTH => Bound::Both(
            values[POSITION_SUBJECT].known()?,
            values[POSITION_OBJECT].known()?,
        ),
        _ => unreachable!("INDEX is a planned index code"),
    };
    Some(Scan {
        partitions: PartitionPosition::new(rel, values.map(PositionValue::known)),
        bound,
        active: None,
        pending: None,
    })
}

impl<I: TermIdentity, const N: usize> Scan<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Resume the current physical run, retaining a refused match before expansion.
    fn advance<const SCAN: u8, const INDEX: u8, D: TermDictionary<Id = I>>(
        &mut self,
        operator: &AtomOperator<N>,
        source: &Source<'_, D, N>,
        input: &SlotSolution<I, N>,
        output: &mut Vec<SlotSolution<I, N>>,
        governor: &mut StepGovernor<'_>,
    ) -> bool {
        let shape = operator.shape();
        let Source { rel, run, delta } = *source;
        loop {
            if governor.stopped() {
                return false;
            }
            if self.active.is_none() {
                let Some(partition) = self.partitions.next(rel, run) else {
                    return true;
                };
                self.active = Some((
                    partition.id(),
                    RowPosition::new(partition.arrangement(), self.bound),
                ));
            }
            let (id, rows) = self.active.as_mut().expect("the partition was selected");
            let original = rel.partition_with_id(*id);
            let partition = run.map_or(original, |run| {
                run.project(original)
                    .expect("the suspended partition belongs to its original delta")
            });
            loop {
                if governor.stopped() {
                    return false;
                }
                if self.pending.is_none() {
                    let Some((subject, object, row)) = rows.next(partition.arrangement()) else {
                        break;
                    };
                    #[cfg(test)]
                    {
                        governor.row_visits += 1;
                    }
                    if !keep_row::<SCAN>(delta, row) {
                        continue;
                    }
                    let matched = partition.row(subject, object);
                    if !shape
                        .equalities()
                        .iter()
                        .all(|&(left, right)| matched[left] == matched[right])
                    {
                        continue;
                    }
                    self.pending = Some((matched, row));
                }
                if !governor.charge() {
                    return false;
                }
                let (matched, row) = self.pending.take().expect("a matched row was admitted");
                let mut merged = input.clone();
                for (position, plan) in shape.positions().iter().enumerate() {
                    if let PositionPlan::Free(slot) | PositionPlan::Bound(slot) = plan
                        && (INDEX == INDEX_REBOUND || matches!(plan, PositionPlan::Free(_)))
                    {
                        merged.bindings[*slot] = Some(matched[position]);
                    }
                }
                merged.sources.push(SourceRow {
                    source: shape.read_source(),
                    body_index: operator.body_index(),
                    terms: matched,
                    row,
                });
                output.push(merged);
            }
            self.active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
    use crate::plan::RulePlan;
    use crate::seminaive::{SCAN_FULL, SCAN_OLD_ONLY};
    use crate::store::TermInterner;
    use crate::test_support::{HostDictionary, SourceLost};
    use std::sync::atomic::Ordering;

    fn check_cuts<const SCAN: u8, const INDEX: u8, D: TermDictionary, const N: usize>(
        operator: &AtomOperator<N>,
        store: &RelationStore<D, N>,
        delta: Delta,
        indexes: &DeltaIndexes<D::Id>,
        inputs: &[SlotSolution<D::Id, N>],
    ) where
        Arity<N>: StoreShape<N>,
    {
        let mut reference = Frame::new(operator, store, inputs.to_vec()).unwrap();
        let mut governor = StepGovernor::new(u64::MAX);
        reference
            .advance::<SCAN, INDEX, D>(operator, store, delta, indexes, &mut governor)
            .unwrap();
        assert!(reference.is_complete());
        let steps = governor.pool.used.load(Ordering::Relaxed);
        let visits = governor.row_visits;
        let expected = reference.into_output();
        for cut in 0..=steps {
            let mut frame = Frame::new(operator, store, inputs.to_vec()).unwrap();
            let mut first = StepGovernor::new(cut);
            frame
                .advance::<SCAN, INDEX, D>(operator, store, delta, indexes, &mut first)
                .unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                frame
                    .advance::<SCAN, INDEX, D>(operator, store, delta, indexes, &mut rest)
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
                "visits, cut {cut}"
            );
            assert_eq!(frame.into_output(), expected, "output, cut {cut}");
        }

        let mut frame = Frame::new(operator, store, inputs.to_vec()).unwrap();
        let (mut resumed_steps, mut resumed_visits) = (0, 0);
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance::<SCAN, INDEX, D>(operator, store, delta, indexes, &mut page)
                .unwrap();
            resumed_steps += page.pool.used.load(Ordering::Relaxed);
            resumed_visits += page.row_visits;
            assert!(page.pool.used.load(Ordering::Relaxed) != 0 || frame.is_complete());
        }
        assert_eq!((resumed_steps, resumed_visits), (steps, visits));
        assert_eq!(frame.into_output(), expected);
    }

    fn check_modes<const INDEX: u8, const N: usize>(
        pattern: ClauseAtom<N>,
        store: &RelationStore<TermInterner, N>,
        delta: Delta,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let rule = DlClause::datalog(pattern.clone(), vec![pattern]);
        let plan = RulePlan::for_rule(&rule);
        let inputs = vec![SlotSolution::empty(plan.variables().len()); 2];
        let indexes = DeltaIndexes::build(store, [delta]).unwrap();
        let operator = &plan.operators()[0];
        check_cuts::<SCAN_FULL, INDEX, _, N>(operator, store, delta, &indexes, &inputs);
        check_cuts::<SCAN_DELTA, INDEX, _, N>(operator, store, delta, &indexes, &inputs);
        check_cuts::<SCAN_OLD_ONLY, INDEX, _, N>(operator, store, delta, &indexes, &inputs);
    }

    #[test]
    fn atom_frames_resume_every_credit_cut_without_replaying_rows_or_inputs() {
        let mut store = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for index in 0..130 {
            store
                .insert_row([
                    "<https://example.org/s>",
                    "<https://example.org/p>",
                    &format!("<https://example.org/o{index:03}>"),
                ])
                .unwrap();
        }
        for row in [
            [
                "<https://example.org/z>",
                "<https://example.org/p>",
                "<https://example.org/o005>",
            ],
            [
                "<https://example.org/s>",
                "<https://example.org/q>",
                "<https://example.org/o005>",
            ],
            [
                "<https://example.org/s>",
                "<https://example.org/p>",
                "<https://example.org/s>",
            ],
        ] {
            store.insert_row(row).unwrap();
        }
        let delta = Delta {
            lo: 67,
            hi: store.row_count() as u64,
        };
        let pattern = |subject, object| {
            ClauseAtom::triple(subject, ClauseTerm::iri("https://example.org/p"), object)
        };
        check_modes::<INDEX_ANY, 3>(
            pattern(ClauseTerm::var("s"), ClauseTerm::var("o")),
            &store,
            delta,
        );
        check_modes::<INDEX_SUBJECT, 3>(
            pattern(
                ClauseTerm::iri("https://example.org/s"),
                ClauseTerm::var("o"),
            ),
            &store,
            delta,
        );
        check_modes::<INDEX_OBJECT, 3>(
            pattern(
                ClauseTerm::var("s"),
                ClauseTerm::iri("https://example.org/o005"),
            ),
            &store,
            delta,
        );
        check_modes::<INDEX_BOTH, 3>(
            pattern(
                ClauseTerm::iri("https://example.org/s"),
                ClauseTerm::iri("https://example.org/o005"),
            ),
            &store,
            delta,
        );
        check_modes::<INDEX_REBOUND, 3>(
            pattern(ClauseTerm::var("s"), ClauseTerm::var("s")),
            &store,
            delta,
        );
        check_modes::<INDEX_ANY, 3>(
            ClauseAtom::triple(
                ClauseTerm::var("s"),
                ClauseTerm::var("p"),
                ClauseTerm::var("o"),
            ),
            &store,
            delta,
        );
        let pattern = pattern(ClauseTerm::var("s"), ClauseTerm::var("o"));
        let plan = RulePlan::for_rule(&DlClause::datalog(pattern.clone(), vec![pattern]));
        let slot = plan
            .variables()
            .iter()
            .position(|name| name == "s")
            .unwrap();
        let mut bound = SlotSolution::empty(plan.variables().len());
        bound.bindings[slot] = store.term_id("<https://example.org/s>");
        let mut missing = SlotSolution::empty(plan.variables().len());
        missing
            .computed
            .push((slot, "<https://example.org/absent>".into()));
        let inputs = [bound, missing, SlotSolution::empty(plan.variables().len())];
        let indexes = DeltaIndexes::build(&store, [delta]).unwrap();
        let operator = &plan.operators()[0];
        check_cuts::<SCAN_FULL, INDEX_REBOUND, _, 3>(operator, &store, delta, &indexes, &inputs);
        check_cuts::<SCAN_DELTA, INDEX_REBOUND, _, 3>(operator, &store, delta, &indexes, &inputs);
        check_cuts::<SCAN_OLD_ONLY, INDEX_REBOUND, _, 3>(
            operator, &store, delta, &indexes, &inputs,
        );
    }

    #[test]
    fn atom_frames_resume_original_quad_graphs_across_partitions() {
        let mut store = RelationStore::new();
        for index in 0..131 {
            store
                .insert(
                    "<https://example.org/s>",
                    "<https://example.org/p>",
                    &format!("<https://example.org/o{index:03}>"),
                    if index % 2 == 0 {
                        "<https://example.org/g2>"
                    } else {
                        "<https://example.org/g1>"
                    },
                )
                .unwrap();
        }
        let delta = Delta {
            lo: 67,
            hi: store.row_count() as u64,
        };
        check_modes::<INDEX_ANY, 4>(
            ClauseAtom::quad(
                ClauseTerm::var("s"),
                ClauseTerm::iri("https://example.org/p"),
                ClauseTerm::var("o"),
                ClauseTerm::var("g"),
            ),
            &store,
            delta,
        );
    }

    #[test]
    fn suspended_native_matches_keep_their_ids_constants_and_original_source_error() {
        const P: u64 = 0xf000_0000_0000_0007;
        const GRAPH: u64 = 0x8000_0000_0000_0000;
        let dictionary = HostDictionary {
            surfaces: [
                (P, "<https://example.org/p>".into()),
                (GRAPH, String::new()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        };
        let mut store = RelationStore::with_dictionary(dictionary);
        for object in [u64::MAX, 1, (1_u64 << 54) + 11] {
            store.insert_ids([0, P, object, GRAPH]).unwrap();
        }
        let pattern = ClauseAtom::positive(
            ClauseTerm::var("s"),
            "https://example.org/p",
            ClauseTerm::var("o"),
        );
        let rule = DlClause::datalog(pattern.clone(), vec![pattern]);
        let plan = RulePlan::for_rule(&rule);
        let operator = &plan.operators()[0];
        let delta = Delta::all(store.row_slots());
        let indexes = DeltaIndexes { runs: Vec::new() };
        let input = vec![SlotSolution::empty(plan.variables().len())];
        let lookups = store.interner().lookups.load(Ordering::Relaxed);
        let resolves = store.interner().resolves.load(Ordering::Relaxed);
        let mut frame = Frame::new(operator, &store, input.clone()).unwrap();
        for _ in 0..4 {
            let mut page = StepGovernor::new(1);
            frame
                .advance::<SCAN_FULL, INDEX_ANY, _>(operator, &store, delta, &indexes, &mut page)
                .unwrap();
            if frame.is_complete() {
                break;
            }
        }
        assert!(frame.is_complete());
        assert_eq!(
            store.interner().lookups.load(Ordering::Relaxed),
            lookups + 2
        );
        assert_eq!(store.interner().resolves.load(Ordering::Relaxed), resolves);
        let output = frame.into_output();
        assert_eq!(output.len(), 3);
        assert!(
            output
                .iter()
                .any(|solution| solution.sources[0].terms == [0, P, u64::MAX, GRAPH])
        );

        let mut frame = Frame::new(operator, &store, input).unwrap();
        let mut refused = StepGovernor::new(0);
        frame
            .advance::<SCAN_FULL, INDEX_ANY, _>(operator, &store, delta, &indexes, &mut refused)
            .unwrap();
        assert!(!frame.is_complete());
        assert_eq!(frame.output.len(), 0);
        store.interner().lost.store(true, Ordering::Relaxed);
        let mut retry = StepGovernor::new(u64::MAX);
        assert_eq!(
            frame.advance::<SCAN_FULL, INDEX_ANY, _>(operator, &store, delta, &indexes, &mut retry),
            Err(SourceLost(99))
        );
        assert_eq!(retry.pool.used.load(Ordering::Relaxed), 0);
        assert_eq!(retry.row_visits, 0);
        assert_eq!(frame.output.len(), 0);
    }
}
