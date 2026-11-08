// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned negative-probe positions, preserving the original reservation law.

use super::{
    Bound, NegatedAtom, POSITION_OBJECT, POSITION_SUBJECT, ProbeValue, RelationStore, SlotSolution,
    StepGovernor, TermDictionary, TermId, TermIdentity,
};
use crate::clause::ReadSource;
use crate::cursor::RowPosition;
use crate::id::{PartitionId, RowId};
use crate::store::{Arity, PartitionPosition, StoreShape};
use purrdf_core::evidence::{EvidenceCombination, EvidenceId};

/// A selected row is retained when its reservation refuses the next credit.
#[derive(Debug)]
struct Rows<I: TermIdentity> {
    partition: PartitionId,
    position: RowPosition<I>,
    first: bool,
    pending: Option<(I, I, RowId)>,
}

/// The original partition sweep, including a partition not yet admitted.
#[derive(Debug)]
pub(super) struct Selection<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    partitions: PartitionPosition<I, N>,
    bound: Bound<I>,
    pending: Option<PartitionId>,
    started: bool,
    rows: Option<Rows<I>>,
    #[cfg(test)]
    row_visits: u64,
    #[cfg(test)]
    partition_visits: u64,
}

/// Exhaustion and a refused reservation are distinct native scan outcomes.
#[derive(Debug)]
pub(super) enum ScanStep<I: TermIdentity, const N: usize> {
    Matched([I; N], RowId),
    Paused,
    Exhausted,
}

impl<I: TermIdentity, const N: usize> Selection<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Freeze bounds over the original selected source once per probe input.
    pub(super) fn new<D: TermDictionary<Id = I>>(
        rel: &RelationStore<D, N>,
        values: [Option<I>; N],
    ) -> Self {
        let bound = match (values[POSITION_SUBJECT], values[POSITION_OBJECT]) {
            (Some(subject), Some(object)) => Bound::Both(subject, object),
            (Some(subject), None) => Bound::Subject(subject),
            (None, Some(object)) => Bound::Object(object),
            (None, None) => Bound::Any,
        };
        Self {
            partitions: PartitionPosition::new(rel, values),
            bound,
            pending: None,
            started: false,
            rows: None,
            #[cfg(test)]
            row_visits: 0,
            #[cfg(test)]
            partition_visits: 0,
        }
    }

    /// Group atoms reserve every partition and row; a standalone atom's initial
    /// probe already reserved its first partition and each partition's first row.
    pub(super) fn next<const ALL: bool, D: TermDictionary<Id = I>>(
        &mut self,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
    ) -> ScanStep<I, N> {
        loop {
            if self.rows.is_none() {
                if self.pending.is_none() {
                    self.pending = self.partitions.next(rel, None).map(|part| {
                        #[cfg(test)]
                        {
                            self.partition_visits += 1;
                        }
                        part.id()
                    });
                }
                let Some(partition) = self.pending else {
                    return ScanStep::Exhausted;
                };
                if (ALL || self.started) && !governor.charge() {
                    return ScanStep::Paused;
                }
                self.started = true;
                self.rows = Some(Rows {
                    partition,
                    position: RowPosition::new(
                        rel.partition_with_id(partition).arrangement(),
                        self.bound,
                    ),
                    first: true,
                    pending: None,
                });
                self.pending = None;
            }
            let rows = self.rows.as_mut().expect("the partition was admitted");
            if rows.pending.is_none() {
                rows.pending = rows
                    .position
                    .next(rel.partition_with_id(rows.partition).arrangement());
                #[cfg(test)]
                if rows.pending.is_some() {
                    self.row_visits += 1;
                }
            }
            let Some((subject, object, row)) = rows.pending else {
                self.rows = None;
                continue;
            };
            if (ALL || !rows.first) && !governor.charge() {
                return ScanStep::Paused;
            }
            rows.first = false;
            rows.pending = None;
            return ScanStep::Matched(
                rel.partition_with_id(rows.partition).row(subject, object),
                row,
            );
        }
    }

    #[cfg(test)]
    pub(super) fn counts(&self) -> (u64, u64) {
        (self.row_visits, self.partition_visits)
    }
}

/// A negative atom's original bounds, native scan and pending admission.
/// The surrounding execution owns the unchanged atom, substitution and source.
#[derive(Debug)]
pub(super) struct Probe<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    source: ReadSource,
    started: bool,
    selection: Option<Selection<I, N>>,
    complete: bool,
    found: bool,
    #[cfg(test)]
    rows: u64,
    #[cfg(test)]
    partitions: u64,
}

impl<I: TermIdentity, const N: usize> Probe<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Retain the authored read authority before any reservation or term lookup.
    pub(super) fn new(source: ReadSource) -> Self {
        Self {
            source,
            started: false,
            selection: None,
            complete: false,
            found: false,
            #[cfg(test)]
            rows: 0,
            #[cfg(test)]
            partitions: 0,
        }
    }

    /// Whether the visitor stopped the probe or every original row was examined.
    pub(super) fn is_complete(&self) -> bool {
        self.complete
    }

    /// Continue the native scan, invoking the visitor once for each admitted row.
    /// The initial probe also reserves its first partition and first matching row;
    /// further partitions and further rows each reserve their original credit.
    pub(super) fn advance<D: TermDictionary<Id = I>, E>(
        &mut self,
        atom: &NegatedAtom<N>,
        solution: &SlotSolution<I, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
        mut visit: impl FnMut(RowId) -> Result<bool, E>,
        source_error: impl Fn(D::Error) -> E,
    ) -> Result<bool, E> {
        let rel = rel.read_source(self.source);
        rel.interner().check_read().map_err(&source_error)?;
        if !self.started && governor.charge() {
            self.started = true;
            let mut values = [None; N];
            for (position, arg) in atom.args.iter().enumerate() {
                match arg.probe(solution, rel).map_err(&source_error)? {
                    ProbeValue::Known(id) => values[position] = Some(id),
                    ProbeValue::Free => {}
                    ProbeValue::Absent => {
                        self.complete = true;
                        break;
                    }
                }
            }
            if !self.complete {
                self.selection = Some(Selection::new(rel, values));
            }
        }
        while !self.complete {
            let Some(selection) = &mut self.selection else {
                break;
            };
            let step = selection.next::<false, D>(rel, governor);
            #[cfg(test)]
            {
                (self.rows, self.partitions) = selection.counts();
            }
            match step {
                ScanStep::Paused => break,
                ScanStep::Exhausted => self.complete = true,
                ScanStep::Matched(_, row) => {
                    if visit(row)? {
                        self.found = true;
                        self.complete = true;
                    }
                }
            }
        }
        rel.interner().check_read().map_err(&source_error)?;
        Ok(self.found)
    }
}

/// Signed search must retain every alternative before it can decide absence.
#[derive(Debug)]
enum Capture {
    Boolean,
    Signed(Option<EvidenceId>),
}

/// The native probe and its signed witness fold, with no premature absence claim.
#[derive(Debug)]
pub(super) struct Atom<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    probe: Probe<I, N>,
    capture: Capture,
    outcome: Option<bool>,
}

impl<I: TermIdentity, const N: usize> Atom<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Bind the original source and evidence mode for the whole probe.
    pub(super) fn new(source: ReadSource, evidenced: bool) -> Self {
        Self {
            probe: Probe::new(source),
            capture: if evidenced {
                Capture::Signed(None)
            } else {
                Capture::Boolean
            },
            outcome: None,
        }
    }

    /// Whether the original search has produced its final signed/boolean decision.
    pub(super) fn is_complete(&self) -> bool {
        self.outcome.is_some()
    }

    /// A pending search cannot establish a negative decision.
    pub(super) fn blocked(&self) -> bool {
        self.outcome.unwrap_or(false)
    }

    /// Retain signed alternatives across pages and attach the decision exactly once.
    /// The execution owner keeps the same substitution and evidence workspace alive.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        atom: &NegatedAtom<N>,
        solution: &mut SlotSolution<I, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
    ) -> Result<(), super::EvalError<D::Error>> {
        use super::EvalError;
        if self.is_complete() {
            return rel
                .read_source(self.probe.source)
                .interner()
                .check_read()
                .map_err(EvalError::Source);
        }
        self.outcome = match &mut self.capture {
            Capture::Boolean => {
                let found = self.probe.advance(
                    atom,
                    solution,
                    rel,
                    governor,
                    |_| Ok::<_, EvalError<D::Error>>(true),
                    EvalError::Source,
                )?;
                self.probe.is_complete().then_some(found)
            }
            Capture::Signed(witnesses) => {
                let owner = evidence
                    .as_deref_mut()
                    .expect("an evidenced probe retains its original workspace");
                self.probe.advance(
                    atom,
                    solution,
                    rel,
                    governor,
                    |row| {
                        let id = owner.source(row)?;
                        *witnesses = Some(match *witnesses {
                            None => id,
                            Some(previous) => {
                                owner.combine(previous, id, EvidenceCombination::Alternatives)?
                            }
                        });
                        Ok::<_, EvalError<D::Error>>(false)
                    },
                    EvalError::Source,
                )?;
                if self.probe.is_complete() {
                    let (negative, blocked) = owner.negative(*witnesses)?;
                    solution.retain_negative(negative, owner)?;
                    Some(blocked)
                } else {
                    None
                }
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seminaive::{ArgShape, evidence};
    use crate::test_support::{HostDictionary, SourceLost, evidence_scope};
    use std::sync::atomic::Ordering;

    const WIDE: u64 = (1 << 54) + 1;
    const GRAPH: u64 = 1 << 63;
    const POPULATION: usize = crate::store::TAIL_SEAL_THRESHOLD + 5;

    fn fixture<const N: usize>() -> RelationStore<HostDictionary, N>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut rel = RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                (7, "<https://example.org/p>".into()),
                (8, "<https://example.org/q>".into()),
                (9, "<https://example.org/unrepresented>".into()),
                (GRAPH, "<https://example.org/g>".into()),
                (GRAPH + 1, "<https://example.org/h>".into()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        });
        for predicate in [7, 8] {
            for graph in [GRAPH, GRAPH + 1].into_iter().take(N - 2) {
                for index in 0..POPULATION {
                    let subject = [0, u64::MAX, WIDE][index % 3];
                    rel.insert_ids(std::array::from_fn(|position| match position {
                        0 => subject,
                        1 => predicate,
                        2 => 1000 + index as u64,
                        _ => graph,
                    }))
                    .unwrap();
                }
            }
        }
        rel
    }

    fn check_cuts<const N: usize>(
        atom: &NegatedAtom<N>,
        solution: &SlotSolution<u64, N>,
        rel: &RelationStore<HostDictionary, N>,
        stop_after: Option<usize>,
    ) -> (usize, u64)
    where
        Arity<N>: StoreShape<N>,
    {
        let lookups = || rel.interner().lookups.load(Ordering::Relaxed);
        let drive = |probe: &mut Probe<u64, N>, rows: &mut Vec<RowId>, governor: &mut _| {
            probe.advance(
                atom,
                solution,
                rel,
                governor,
                |row| {
                    rows.push(row);
                    Ok::<_, SourceLost>(stop_after == Some(rows.len()))
                },
                core::convert::identity,
            )
        };
        let before = lookups();
        let mut reference = Probe::new(atom.source);
        let mut expected = Vec::new();
        let mut governor = StepGovernor::new(u64::MAX);
        let found = drive(&mut reference, &mut expected, &mut governor).unwrap();
        assert!(reference.is_complete());
        let used = governor.pool.used.load(Ordering::Relaxed);
        let probes = lookups() - before;
        for cut in 0..=used {
            let before = lookups();
            let mut probe = Probe::new(atom.source);
            let mut rows = Vec::new();
            let mut first = StepGovernor::new(cut);
            drive(&mut probe, &mut rows, &mut first).unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !probe.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                drive(&mut probe, &mut rows, &mut rest).unwrap();
            }
            assert!(probe.is_complete());
            assert_eq!(probe.found, found);
            assert_eq!(rows, expected, "original row trace, cut {cut}");
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                used,
                "successful credits, cut {cut}"
            );
            assert_eq!(
                (probe.partitions, probe.rows),
                (reference.partitions, reference.rows)
            );
            assert_eq!(lookups() - before, probes, "term probes, cut {cut}");
        }
        let before = lookups();
        let mut probe = Probe::new(atom.source);
        let mut rows = Vec::new();
        let mut credits = 0;
        while !probe.is_complete() {
            let mut page = StepGovernor::new(1);
            drive(&mut probe, &mut rows, &mut page).unwrap();
            let admitted = page.pool.used.load(Ordering::Relaxed);
            assert!(admitted > 0 || probe.is_complete());
            credits += admitted;
        }
        assert_eq!(rows, expected);
        assert_eq!(credits, used);
        assert_eq!(
            (probe.partitions, probe.rows),
            (reference.partitions, reference.rows)
        );
        assert_eq!(lookups() - before, probes);
        (expected.len(), used)
    }

    fn check_shapes<const N: usize>()
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = fixture::<N>();
        let resolves = rel.interner().resolves.load(Ordering::Relaxed);
        let mut atom = NegatedAtom {
            args: std::array::from_fn(ArgShape::Slot),
            source: ReadSource::Evaluation,
        };
        let mut solution = SlotSolution::empty(N);
        let all = 2 * (N - 2) * POPULATION;
        assert_eq!(check_cuts(&atom, &solution, &rel, None), (all, all as u64));
        for subject in [0, u64::MAX, WIDE, 44] {
            solution.bindings[0] = Some(subject);
            for object in [None, Some(1003), Some(999)] {
                solution.bindings[2] = object;
                for predicate in [None, Some(7), Some(9)] {
                    solution.bindings[1] = predicate;
                    for stop in [None, Some(1), Some(5)] {
                        check_cuts(&atom, &solution, &rel, stop);
                    }
                }
            }
        }
        solution.bindings.fill(None);
        // Repeated free existential positions are independently unconstrained.
        atom.args[2] = ArgShape::Slot(0);
        assert_eq!(check_cuts(&atom, &solution, &rel, None).0, all);
        atom.args[1] = ArgShape::Const("<https://example.org/p>".into());
        assert_eq!(check_cuts(&atom, &solution, &rel, None).0, all / 2);
        atom.args[1] = ArgShape::Const("<https://example.org/absent>".into());
        assert_eq!(check_cuts(&atom, &solution, &rel, None), (0, 1));
        assert_eq!(rel.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    #[test]
    fn triple_negative_probe_retains_every_partition_row_and_credit_cut() {
        check_shapes::<3>();
    }

    #[test]
    fn quad_negative_probe_retains_every_partition_row_and_credit_cut() {
        check_shapes::<4>();
    }

    #[test]
    fn resumed_negative_probe_checks_original_source_before_work_and_keeps_first_error() {
        let rel = fixture::<3>();
        let atom = NegatedAtom {
            args: [
                ArgShape::Slot(0),
                ArgShape::Const("<https://example.org/p>".into()),
                ArgShape::Slot(1),
            ],
            source: ReadSource::Evaluation,
        };
        let solution = SlotSolution::empty(2);
        let mut probe = Probe::new(atom.source);
        let mut rows = Vec::new();
        probe
            .advance(
                &atom,
                &solution,
                &rel,
                &mut StepGovernor::new(1),
                |row| {
                    rows.push(row);
                    Ok::<_, SourceLost>(false)
                },
                core::convert::identity,
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(!probe.is_complete());
        let before = (
            probe.rows,
            probe.partitions,
            rel.interner().lookups.load(Ordering::Relaxed),
        );
        rel.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            probe.advance(
                &atom,
                &solution,
                &rel,
                &mut StepGovernor::new(0),
                |_| Ok::<_, SourceLost>(false),
                core::convert::identity
            ),
            Err(SourceLost(99))
        );
        assert_eq!(
            (
                probe.rows,
                probe.partitions,
                rel.interner().lookups.load(Ordering::Relaxed)
            ),
            before
        );
        rel.interner().lost.store(false, Ordering::Relaxed);
        assert_eq!(
            probe.advance(
                &atom,
                &solution,
                &rel,
                &mut StepGovernor::new(1),
                |_| {
                    rel.interner().lost.store(true, Ordering::Relaxed);
                    Err(SourceLost(17))
                },
                core::convert::identity
            ),
            Err(SourceLost(17))
        );
    }

    fn capture<const N: usize>(
        rows: usize,
    ) -> evidence::Capture<purrdf_lex::allocation::Resident, u64, N>
    where
        Arity<N>: StoreShape<N>,
    {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
        };
        let mut evidence = crate::evidence::DerivationEvidence::with_admission(
            rows,
            purrdf_lex::allocation::Resident,
        );
        for index in 0..rows {
            evidence
                .seed(RowId::from_index(index as u64).unwrap(), |memory| {
                    EvidenceRecord::from_parts(
                        &InformationMap::uniform(if index % 2 == 0 {
                            Information::Supported
                        } else {
                            Information::Opposed
                        }),
                        SearchCompleteness::Complete,
                        &Frame::actual(),
                        &[],
                        &[],
                        memory,
                    )
                })
                .unwrap();
        }
        evidence::Capture::new(evidence, evidence_scope())
    }

    #[test]
    fn signed_negative_atom_retains_witnesses_and_withholds_absence_until_complete() {
        use purrdf_core::evidence::{Information, SearchCompleteness};
        let rel = fixture::<3>();
        let mut atom = NegatedAtom {
            args: [
                ArgShape::Slot(0),
                ArgShape::Const("<https://example.org/p>".into()),
                ArgShape::Slot(1),
            ],
            source: ReadSource::Evaluation,
        };
        for present in [true, false] {
            if !present {
                atom.args[1] = ArgShape::Const("<https://example.org/absent>".into());
            }
            let mut expected = SlotSolution::empty(2);
            let mut reference = Atom::new(atom.source, true);
            let mut original = capture::<3>(rel.row_count());
            let mut full = StepGovernor::new(u64::MAX);
            reference
                .advance(
                    &atom,
                    &mut expected,
                    &rel,
                    &mut full,
                    &mut Some(&mut original),
                )
                .unwrap();
            assert!(reference.is_complete());
            let steps = full.pool.used.load(Ordering::Relaxed);
            let id = expected.negative_evidence.unwrap();
            let record = original.rows.workspace().get(id).unwrap();
            assert_eq!(
                record.information().fallback(),
                if present {
                    Information::Both
                } else {
                    Information::Supported
                }
            );
            assert_eq!(record.completeness(), SearchCompleteness::Complete);
            for cut in 0..=steps {
                let mut solution = SlotSolution::empty(2);
                let mut frame = Atom::new(atom.source, true);
                let mut owner = capture::<3>(rel.row_count());
                let mut first = StepGovernor::new(cut);
                frame
                    .advance(
                        &atom,
                        &mut solution,
                        &rel,
                        &mut first,
                        &mut Some(&mut owner),
                    )
                    .unwrap();
                let mut rest = StepGovernor::new(u64::MAX);
                if !frame.is_complete() {
                    assert!(!frame.blocked());
                    assert_eq!(
                        solution.negative_evidence, None,
                        "unfinished search cannot license absence"
                    );
                    frame
                        .advance(&atom, &mut solution, &rel, &mut rest, &mut Some(&mut owner))
                        .unwrap();
                }
                assert!(frame.is_complete());
                assert_eq!(frame.blocked(), reference.blocked());
                assert_eq!(solution, expected);
                assert_eq!(
                    first.pool.used.load(Ordering::Relaxed)
                        + rest.pool.used.load(Ordering::Relaxed),
                    steps
                );
                assert_eq!(
                    owner
                        .rows
                        .workspace()
                        .get(solution.negative_evidence.unwrap())
                        .unwrap(),
                    record
                );
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
                let mut exhausted = StepGovernor::new(0);
                frame
                    .advance(
                        &atom,
                        &mut solution,
                        &rel,
                        &mut exhausted,
                        &mut Some(&mut owner),
                    )
                    .unwrap();
                assert_eq!(
                    exhausted.observed(),
                    0,
                    "a completed decision is attached once"
                );
            }
            let mut solution = SlotSolution::empty(2);
            let mut frame = Atom::new(atom.source, true);
            let mut owner = capture::<3>(rel.row_count());
            let mut admitted = 0;
            while !frame.is_complete() {
                let mut page = StepGovernor::new(1);
                frame
                    .advance(&atom, &mut solution, &rel, &mut page, &mut Some(&mut owner))
                    .unwrap();
                admitted += page.pool.used.load(Ordering::Relaxed);
                if !frame.is_complete() {
                    assert_eq!(solution.negative_evidence, None);
                    assert!(!frame.blocked());
                }
            }
            assert_eq!(admitted, steps);
            assert_eq!(solution, expected);
            assert_eq!(
                (frame.probe.rows, frame.probe.partitions),
                (reference.probe.rows, reference.probe.partitions)
            );
            assert_eq!(
                owner.rows.workspace().canonical_index().unwrap(),
                original.rows.workspace().canonical_index().unwrap()
            );
        }
    }

    #[test]
    fn boolean_negative_atom_reserves_the_original_probe_and_stops_at_first_match() {
        let rel = fixture::<3>();
        let atom = NegatedAtom {
            args: std::array::from_fn(ArgShape::Slot),
            source: ReadSource::Evaluation,
        };
        for subject in [None, Some(44)] {
            let mut solution = SlotSolution::empty(3);
            solution.bindings[0] = subject;
            let mut frame = Atom::new(atom.source, false);
            let mut empty = StepGovernor::new(0);
            frame
                .advance(&atom, &mut solution, &rel, &mut empty, &mut None)
                .unwrap();
            assert!(!frame.is_complete());
            assert!(!frame.blocked());
            assert_eq!(solution.negative_evidence, None);
            let mut full = StepGovernor::new(u64::MAX);
            frame
                .advance(&atom, &mut solution, &rel, &mut full, &mut None)
                .unwrap();
            assert!(frame.is_complete());
            assert_eq!(frame.blocked(), subject.is_none());
            assert_eq!(
                full.pool.used.load(Ordering::Relaxed),
                if subject.is_none() { 1 } else { 2 }
            );
            assert_eq!(frame.probe.rows, u64::from(subject.is_none()));
            assert_eq!(solution.negative_evidence, None);
        }
    }
}
