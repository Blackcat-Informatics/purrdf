// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One factor's owned native positive sequence and negative-filter progress.

use super::{Factor, JoinScan, JoinSnapshot, RuleEntry, SlotSolution, filters};
use crate::dictionary::{TermDictionary, TermIdentity};
use crate::id::TermId;
use crate::seminaive::{Delta, EvalError, StepGovernor, evidence::RuleEvidence, join};
use crate::store::{Arity, StoreShape};

#[derive(Debug)]
struct Input<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    solution: SlotSolution<I, N>,
    filters: filters::Frame<I, N>,
}

#[derive(Debug)]
enum Phase<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Join {
        sequence: join::Sequence<I, N>,
        seed_in_cycle: bool,
    },
    Filter {
        inputs: std::vec::IntoIter<SlotSolution<I, N>>,
        active: Option<Input<I, N>>,
    },
    Complete,
}

/// Original physical steps, active substitution and signed filters for one mode.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    phase: Phase<I, N>,
    output: Vec<SlotSolution<I, N>>,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Freeze this factor's existing scan law and certified native groups once.
    pub(super) fn new<D: TermDictionary<Id = I>>(
        factor: &Factor,
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, D, N>,
        scan: JoinScan,
    ) -> Result<Self, D::Error> {
        snapshot.rel.interner().check_read()?;
        let seed = match scan {
            JoinScan::DeltaPosition(anchor)
                if entry.delta != Delta::all(snapshot.rel.row_slots()) =>
            {
                Some(anchor)
            }
            _ => None,
        };
        let (steps, seed_in_cycle) = join::selected_steps(
            entry.plan,
            join::Selection {
                scan,
                seed,
                cyclic: entry.plan.has_cyclic_subplan(),
                rebound: seed.is_some() || snapshot.seed.is_some(),
            },
            |operator| factor.positions.contains(&operator.positive_position()),
        );
        Ok(Self {
            phase: Phase::Join {
                sequence: join::Sequence::new(steps, vec![snapshot.initial(entry.plan)]),
                seed_in_cycle,
            },
            output: Vec::new(),
        })
    }

    pub(super) fn is_complete(&self) -> bool {
        matches!(self.phase, Phase::Complete)
    }

    /// Pending matches remain owned; only the completed factor relation is exposed.
    pub(super) fn into_output(self) -> Vec<SlotSolution<I, N>> {
        if self.is_complete() {
            self.output
        } else {
            Vec::new()
        }
    }

    /// Continue native joining and each original negative witness without replay.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        factor: &Factor,
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        loop {
            match &mut self.phase {
                Phase::Complete => break,
                Phase::Join { sequence, .. } => {
                    sequence
                        .advance(entry.plan, snapshot, governor)
                        .map_err(EvalError::Source)?;
                    if !sequence.is_complete() {
                        break;
                    }
                    let Phase::Join {
                        sequence,
                        seed_in_cycle,
                    } = std::mem::replace(&mut self.phase, Phase::Complete)
                    else {
                        unreachable!("the native factor sequence just completed")
                    };
                    let mut solutions = sequence.into_output();
                    for solution in &mut solutions {
                        if seed_in_cycle {
                            solution.sources.remove(0);
                        }
                        solution.sources.sort_by_key(|source| source.body_index);
                    }
                    self.output = Vec::with_capacity(solutions.len());
                    self.phase = Phase::Filter {
                        inputs: solutions.into_iter(),
                        active: None,
                    };
                }
                Phase::Filter { inputs, active } => {
                    if active.is_none() {
                        if inputs.len() == 0 {
                            self.phase = Phase::Complete;
                            continue;
                        }
                        if governor.spent() {
                            break;
                        }
                        *active = Some(Input {
                            solution: inputs.next().expect("an original input remains"),
                            filters: filters::Frame::new(evidence.is_some()),
                        });
                    }
                    let input = active
                        .as_mut()
                        .expect("the original substitution is active");
                    input.filters.advance(
                        &filters::Selection {
                            atoms: &factor.negated,
                            groups: &factor.negations,
                            entry,
                            guards: &super::super::NoGuards,
                        },
                        snapshot.rel,
                        &mut input.solution,
                        governor,
                        evidence,
                    )?;
                    if !input.filters.is_complete() {
                        break;
                    }
                    if !input.filters.blocked() {
                        self.output.push(
                            active
                                .take()
                                .expect("all original filters completed")
                                .solution,
                        );
                    } else {
                        *active = None;
                    }
                }
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
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
    use crate::guard::Negation;
    use crate::id::RowId;
    use crate::plan::RulePlan;
    use crate::seminaive::factors::{
        Frame as WholeFactor, Product, Projection, full_matches, partial_relations, product,
        project, reference_evaluate_with_evidence, reference_matches, relations,
    };
    use crate::seminaive::{
        ArgShape, DeltaIndexes, RoundBuffer, RoundSnapshot, RuleRuntime, Scan, evidence,
    };
    use crate::store::RelationStore;
    use crate::test_support::{HostDictionary, SourceLost, evidence_scope};
    use purrdf_lex::allocation::Resident;
    use std::sync::atomic::Ordering;

    const WIDE: u64 = (1 << 54) + 1;
    const GRAPH: u64 = 1 << 63;

    fn fixture<const N: usize>() -> RelationStore<HostDictionary, N>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut rel = RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                (7, "<https://example.org/p>".into()),
                (8, "<https://example.org/q>".into()),
                (9, "<https://example.org/r>".into()),
                (17, "<https://example.org/enabled>".into()),
                (19, "<https://example.org/block>".into()),
                (21, "<https://example.org/deny>".into()),
                (GRAPH, "<https://example.org/g>".into()),
            ]
            .into_iter()
            .chain(
                [
                    0,
                    u64::MAX,
                    64,
                    10,
                    11,
                    12,
                    WIDE,
                    22,
                    24,
                    44,
                    45,
                    50,
                    51,
                    60,
                    61,
                ]
                .into_iter()
                .map(|id| (id, format!("<https://example.org/t{id}>"))),
            )
            .collect(),
            ..HostDictionary::default()
        });
        for row in [
            [0, 7, 10],
            [u64::MAX, 7, 11],
            [64, 7, 12],
            [10, 8, WIDE],
            [11, 8, 22],
            [12, 8, 24],
            [WIDE, 9, 0],
            [22, 9, u64::MAX],
            [24, 9, 64],
            [44, 17, 45],
            [10, 19, 50],
            [10, 19, 51],
            [22, 21, 60],
            [22, 21, 61],
        ] {
            rel.insert_ids(std::array::from_fn(|position| {
                if position < 3 { row[position] } else { GRAPH }
            }))
            .unwrap();
        }
        rel
    }

    fn rule<const N: usize>(atom: &impl Fn(&str, &str, &str) -> ClauseAtom<N>) -> DlClause<N> {
        DlClause::datalog(
            atom("?a", "https://example.org/result", "?u"),
            vec![
                atom("?a", "https://example.org/p", "?b"),
                atom("?b", "https://example.org/q", "?c"),
                atom("?c", "https://example.org/r", "?a"),
                atom("?u", "https://example.org/enabled", "?v"),
                atom("?b", "https://example.org/block", "?any").into_negated(),
            ],
        )
        .with_negations(vec![Negation::new(
            vec![atom("?c", "https://example.org/deny", "?local")],
            Vec::new(),
        )])
    }

    fn capture<const N: usize>() -> evidence::Capture<Resident, u64, N>
    where
        Arity<N>: StoreShape<N>,
    {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame as World, Information, InformationMap, SearchCompleteness,
        };
        let scope = evidence_scope();
        let mut rows = crate::evidence::DerivationEvidence::with_admission(14, Resident);
        rows.seed(RowId::from_index(10).unwrap(), |memory| {
            EvidenceRecord::from_parts(
                &InformationMap::uniform(Information::Opposed),
                SearchCompleteness::Complete,
                &World::actual(),
                &[],
                &[],
                memory,
            )
        })
        .unwrap();
        rows.seed(RowId::from_index(13).unwrap(), |memory| {
            EvidenceRecord::from_parts(
                &InformationMap::default(),
                SearchCompleteness::Complete,
                &World::actual(),
                &[scope.missing_scope_reason],
                &[],
                memory,
            )
        })
        .unwrap();
        evidence::Capture::new(rows, scope)
    }

    fn compare<const N: usize>(
        factor: &Factor,
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, HostDictionary, N>,
        scan: JoinScan,
        evidenced: bool,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let lookups = || snapshot.rel.interner().lookups.load(Ordering::Relaxed);
        let mut original = capture::<N>();
        let mut full = StepGovernor::new(u64::MAX);
        let before = lookups();
        let expected = reference_matches(
            factor,
            entry,
            RoundSnapshot {
                probe: None,
                rel: snapshot.rel,
                depth: (&[0; 14][..]).into(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: Some(snapshot.indexes),
            },
            &mut full,
            scan,
            &mut evidenced.then_some(&mut original as &mut dyn RuleEvidence<HostDictionary, N>),
        )
        .unwrap();
        let probes = lookups() - before;
        let steps = full.pool.used.load(Ordering::Relaxed);
        let visits = full.row_visits;
        for cut in 0..=steps {
            let before = lookups();
            let mut owner = capture::<N>();
            let mut frame = Frame::new(factor, entry, snapshot, scan).unwrap();
            let mut first = StepGovernor::new(cut);
            frame
                .advance(
                    factor,
                    entry,
                    snapshot,
                    &mut first,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                frame
                    .advance(
                        factor,
                        entry,
                        snapshot,
                        &mut rest,
                        &mut evidenced
                            .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                    )
                    .unwrap();
            }
            assert!(frame.is_complete());
            assert_eq!(
                frame.into_output(),
                expected,
                "original factor relation, cut {cut}"
            );
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "successful credits, cut {cut}"
            );
            assert_eq!(
                first.row_visits + rest.row_visits,
                visits,
                "native row work, cut {cut}"
            );
            assert_eq!(lookups() - before, probes, "dictionary probes, cut {cut}");
            if evidenced {
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
            }
        }
        let before = lookups();
        let mut owner = capture::<N>();
        let mut frame = Frame::new(factor, entry, snapshot, scan).unwrap();
        let (mut credits, mut rows) = (0, 0);
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance(
                    factor,
                    entry,
                    snapshot,
                    &mut page,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let admitted = page.pool.used.load(Ordering::Relaxed);
            assert!(admitted > 0 || frame.is_complete());
            credits += admitted;
            rows += page.row_visits;
        }
        assert_eq!(frame.into_output(), expected);
        assert_eq!((credits, rows), (steps, visits));
        assert_eq!(lookups() - before, probes);
        if evidenced {
            assert_eq!(
                owner.rows.workspace().canonical_index().unwrap(),
                original.rows.workspace().canonical_index().unwrap()
            );
        }
    }

    fn check_shapes<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = fixture::<N>();
        let resolves = rel.interner().resolves.load(Ordering::Relaxed);
        let rule = rule(&atom);
        let plan = RulePlan::for_rule(&rule);
        assert!(plan.has_cyclic_subplan());
        let runtime = RuleRuntime::new(&rule, &plan);
        assert_eq!(runtime.factors.len(), 2);
        assert_eq!(
            runtime
                .factors
                .iter()
                .map(|factor| factor.negated.len())
                .sum::<usize>(),
            1
        );
        assert_eq!(
            runtime
                .factors
                .iter()
                .map(|factor| factor.negations.len())
                .sum::<usize>(),
            1
        );
        for delta in [
            Delta::all(14),
            Delta { lo: 0, hi: 1 },
            Delta { lo: 3, hi: 5 },
            Delta { lo: 9, hi: 10 },
            Delta::all(0),
        ] {
            let indexes = DeltaIndexes::build(&rel, [delta]).unwrap();
            let entry = RuleEntry {
                index: 3,
                rule: &rule,
                plan: &plan,
                runtime: &runtime,
                delta,
            };
            let snapshot = JoinSnapshot {
                seed: None,
                rel: &rel,
                delta,
                indexes: &indexes,
            };
            for factor in &runtime.factors {
                let scans = [
                    JoinScan::Uniform(Scan::Full),
                    JoinScan::Uniform(Scan::OldOnly),
                    JoinScan::Uniform(Scan::Delta),
                ]
                .into_iter()
                .chain(
                    factor
                        .positions
                        .iter()
                        .map(|&position| JoinScan::DeltaPosition(position)),
                );
                for scan in scans {
                    for evidenced in [false, true] {
                        compare(factor, entry, snapshot, scan, evidenced);
                    }
                }
            }
        }
        assert_eq!(rel.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    fn compare_relations<const N: usize>(
        entry: RuleEntry<'_, N>,
        snapshot: JoinSnapshot<'_, HostDictionary, N>,
        evidenced: bool,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let round = RoundSnapshot {
            probe: None,
            rel: snapshot.rel,
            depth: (&[0; 14][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: Some(snapshot.indexes),
        };
        let all = entry.delta == Delta::all(snapshot.rel.row_slots());
        let lookups = || snapshot.rel.interner().lookups.load(Ordering::Relaxed);
        let mut original = capture::<N>();
        let mut full = StepGovernor::new(u64::MAX);
        let before = lookups();
        let mut evidence =
            evidenced.then_some(&mut original as &mut dyn RuleEvidence<HostDictionary, N>);
        let expected = if all {
            let mut relations = Vec::new();
            for factor in &entry.runtime.factors {
                let rows = full_matches(factor, entry, round, &mut full, &mut evidence).unwrap();
                if rows.is_empty() {
                    relations.clear();
                    break;
                }
                relations.push(rows);
            }
            relations
        } else {
            partial_relations(entry, round, &mut full, &mut evidence).unwrap()
        };
        let probes = lookups() - before;
        let steps = full.pool.used.load(Ordering::Relaxed);
        let visits = full.row_visits;
        for cut in 0..=steps {
            let mut frame = relations::Frame::new(entry.runtime.factors.len(), all);
            let mut owner = capture::<N>();
            let before = lookups();
            let mut first = StepGovernor::new(cut);
            frame
                .advance(
                    entry,
                    snapshot,
                    &mut first,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                frame
                    .advance(
                        entry,
                        snapshot,
                        &mut rest,
                        &mut evidenced
                            .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                    )
                    .unwrap();
            }
            assert!(frame.is_complete());
            assert_eq!(
                frame.into_output(),
                expected,
                "original modes and relation order, cut {cut}"
            );
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "successful credits, cut {cut}"
            );
            assert_eq!(
                first.row_visits + rest.row_visits,
                visits,
                "original row work, cut {cut}"
            );
            assert_eq!(
                lookups() - before,
                probes,
                "original delta eligibility and mode probes, cut {cut}"
            );
            if evidenced {
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
            }
        }
        let mut frame = relations::Frame::new(entry.runtime.factors.len(), all);
        let mut owner = capture::<N>();
        let before = lookups();
        let (mut credits, mut rows) = (0, 0);
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance(
                    entry,
                    snapshot,
                    &mut page,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let admitted = page.pool.used.load(Ordering::Relaxed);
            assert!(admitted > 0 || frame.is_complete());
            credits += admitted;
            rows += page.row_visits;
        }
        assert_eq!(frame.into_output(), expected);
        assert_eq!((credits, rows), (steps, visits));
        assert_eq!(lookups() - before, probes);
        if evidenced {
            assert_eq!(
                owner.rows.workspace().canonical_index().unwrap(),
                original.rows.workspace().canonical_index().unwrap()
            );
        }
    }

    fn check_relation_modes<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = fixture::<N>();
        for rule in [
            rule(&atom),
            DlClause::datalog(
                atom("?a", "https://example.org/result", "?u"),
                vec![
                    atom("?a", "https://example.org/p", "?b"),
                    atom("?u", "https://example.org/absent", "?v"),
                ],
            ),
        ] {
            let plan = RulePlan::for_rule(&rule);
            let runtime = RuleRuntime::new(&rule, &plan);
            assert_eq!(runtime.factors.len(), 2);
            for delta in [
                Delta::all(14),
                Delta { lo: 0, hi: 1 },
                Delta { lo: 3, hi: 5 },
                Delta { lo: 9, hi: 10 },
                Delta { lo: 10, hi: 14 },
                Delta::all(0),
            ] {
                let indexes = DeltaIndexes::build(&rel, [delta]).unwrap();
                let entry = RuleEntry {
                    index: 3,
                    rule: &rule,
                    plan: &plan,
                    runtime: &runtime,
                    delta,
                };
                let snapshot = JoinSnapshot {
                    seed: None,
                    rel: &rel,
                    delta,
                    indexes: &indexes,
                };
                for evidenced in [false, true] {
                    compare_relations(entry, snapshot, evidenced);
                }
            }
        }
    }

    #[test]
    fn triple_relation_modes_resume_productive_anchors_without_probing_again() {
        check_relation_modes(|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
    }

    #[test]
    fn quad_relation_modes_resume_productive_anchors_without_probing_again() {
        check_relation_modes(|subject, predicate, object| {
            ClauseAtom::quad(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
                ClauseTerm::iri("https://example.org/g"),
            )
        });
    }

    /// Force a different allocation while the original buffer remains live.
    fn relocate<T>(original: Vec<T>) -> Vec<T> {
        let mut moved = Vec::with_capacity(original.len() + 1);
        moved.extend(original);
        moved
    }

    fn compare_whole_factor<const N: usize>(
        entry: RuleEntry<'_, N>,
        snapshot: RoundSnapshot<'_, HostDictionary, N>,
        evidenced: bool,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let counters = || {
            (
                snapshot.rel.interner().lookups.load(Ordering::Relaxed),
                snapshot.rel.interner().resolves.load(Ordering::Relaxed),
            )
        };
        let mut original = capture::<N>();
        let mut full = StepGovernor::new(u64::MAX);
        let before = counters();
        let expected = reference_evaluate_with_evidence(
            entry,
            snapshot,
            &mut full,
            evidenced.then_some(&mut original as &mut dyn RuleEvidence<HostDictionary, N>),
        )
        .unwrap();
        let after = counters();
        let probes = (after.0 - before.0, after.1 - before.1);
        let steps = full.pool.used.load(Ordering::Relaxed);
        let visits = full.row_visits;
        for cut in 0..=steps {
            let mut frame = WholeFactor::new();
            let mut owner = capture::<N>();
            let before = counters();
            let mut first = StepGovernor::new(cut);
            frame
                .advance(
                    entry,
                    snapshot,
                    &mut first,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                frame
                    .advance(
                        entry,
                        snapshot,
                        &mut rest,
                        &mut evidenced
                            .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                    )
                    .unwrap();
            }
            assert!(frame.is_complete());
            let buffer = frame.into_buffer();
            assert_eq!(
                buffer.entries, expected.entries,
                "original whole-factor candidates, cut {cut}"
            );
            assert_eq!(buffer.confirmed.len(), expected.confirmed.len());
            for (row, confirmation) in &buffer.confirmed {
                assert_eq!(confirmation.derivation, expected.confirmed[row].derivation);
                assert_eq!(
                    confirmation.sum_source_height,
                    expected.confirmed[row].sum_source_height
                );
            }
            assert_eq!(buffer.generates_terms, expected.generates_terms);
            assert_eq!(buffer.generating_rules, expected.generating_rules);
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "whole-factor successful work, cut {cut}"
            );
            assert_eq!(first.row_visits + rest.row_visits, visits);
            let after = counters();
            assert_eq!(
                (after.0 - before.0, after.1 - before.1),
                probes,
                "no eligibility, filter or projection replay, cut {cut}"
            );
            if evidenced {
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
            }
        }
        let mut frame = WholeFactor::new();
        let mut owner = capture::<N>();
        let before = counters();
        let (mut credits, mut rows) = (0, 0);
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance(
                    entry,
                    snapshot,
                    &mut page,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let admitted = page.pool.used.load(Ordering::Relaxed);
            assert!(admitted > 0 || frame.is_complete());
            credits += admitted;
            rows += page.row_visits;
        }
        assert_eq!(frame.into_buffer().entries, expected.entries);
        assert_eq!((credits, rows), (steps, visits));
        let after = counters();
        assert_eq!((after.0 - before.0, after.1 - before.1), probes);
        if evidenced {
            assert_eq!(
                owner.rows.workspace().canonical_index().unwrap(),
                original.rows.workspace().canonical_index().unwrap()
            );
        }
    }

    fn check_whole_factors<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = fixture::<N>();
        let body = vec![
            atom("?a", "https://example.org/p", "?b"),
            atom("?u", "https://example.org/enabled", "?v"),
        ];
        let mut blocked = body.clone();
        blocked.push(atom("?ground", "https://example.org/block", "?other").into_negated());
        let mut missing = body.clone();
        missing.push(atom("?ground", "https://example.org/absent", "?other").into_negated());
        let rules = [
            rule(&atom),
            DlClause::new(
                vec![HeadDisjunct::new(vec![
                    atom("?a", "https://example.org/result", "?u"),
                    atom("?b", "https://example.org/result", "?v"),
                ])],
                Vec::new(),
                body.clone(),
            ),
            DlClause::datalog(atom("?a", "https://example.org/result", "?u"), blocked)
                .with_negations(vec![Negation::new(
                    vec![atom("?x", "https://example.org/deny", "?y")],
                    Vec::new(),
                )]),
            DlClause::datalog(atom("?a", "https://example.org/result", "?u"), missing)
                .with_negations(vec![Negation::new(
                    vec![
                        atom("?x", "https://example.org/block", "?y"),
                        atom("?y", "https://example.org/absent", "?z"),
                    ],
                    Vec::new(),
                )]),
            DlClause::datalog(atom("?a", "https://example.org/result", "?u"), body).with_negations(
                vec![Negation::new(
                    vec![atom("?x", "https://example.org/deny", "?y")],
                    Vec::new(),
                )],
            ),
        ];
        for rule in rules {
            let plan = RulePlan::for_rule(&rule);
            let runtime = RuleRuntime::new(&rule, &plan);
            assert_eq!(runtime.factors.len(), 2);
            for delta in [
                Delta::all(14),
                Delta { lo: 0, hi: 1 },
                Delta { lo: 3, hi: 5 },
                Delta { lo: 9, hi: 10 },
                Delta { lo: 10, hi: 14 },
                Delta::all(0),
            ] {
                let indexes = DeltaIndexes::build(&rel, [delta]).unwrap();
                let entry = RuleEntry {
                    index: 3,
                    rule: &rule,
                    plan: &plan,
                    runtime: &runtime,
                    delta,
                };
                let snapshot = RoundSnapshot {
                    probe: None,
                    rel: &rel,
                    depth: (&[0, 2, u32::MAX - 1, 1, 3, u32::MAX, 4, 1, 0, 0, 2, 5, 0, 4][..])
                        .into(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: Some(&indexes),
                };
                for evidenced in [false, true] {
                    compare_whole_factor(entry, snapshot, evidenced);
                }
            }
        }
        let rule = rule(&atom);
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let delta = Delta::all(14);
        let indexes = DeltaIndexes::build(&rel, [delta]).unwrap();
        let entry = RuleEntry {
            index: 3,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta,
        };
        let snapshot = RoundSnapshot {
            probe: None,
            rel: &rel,
            depth: (&[0; 14][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: Some(&indexes),
        };
        let mut frame = WholeFactor::new();
        frame
            .advance(entry, snapshot, &mut StepGovernor::new(0), &mut None)
            .unwrap();
        assert!(!frame.is_complete());
        let before = rel.interner().lookups.load(Ordering::Relaxed);
        rel.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            frame.advance(entry, snapshot, &mut StepGovernor::new(0), &mut None),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(rel.interner().lookups.load(Ordering::Relaxed), before);
    }

    #[test]
    fn triple_whole_factors_resume_ground_modes_heads_and_products_at_every_cut() {
        check_whole_factors(|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
    }

    #[test]
    fn quad_whole_factors_resume_ground_modes_heads_and_products_at_every_cut() {
        check_whole_factors(|subject, predicate, object| {
            ClauseAtom::quad(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
                ClauseTerm::iri("https://example.org/g"),
            )
        });
    }

    fn compare_product<const N: usize>(
        entry: RuleEntry<'_, N>,
        snapshot: RoundSnapshot<'_, HostDictionary, N>,
        relations: &mut Vec<Vec<SlotSolution<u64, N>>>,
        projections: &mut Vec<Projection>,
        head: usize,
        anchor: usize,
        evidenced: bool,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let mut original = capture::<N>();
        let mut expected = RoundBuffer::new();
        let mut full = StepGovernor::new(u64::MAX);
        let execution = Product::new(entry, snapshot, relations, projections, head, anchor);
        if !execution.tables.iter().any(|table| table.is_empty()) {
            execution
                .reference_visit(
                    &mut expected,
                    &mut full,
                    &mut evidenced
                        .then_some(&mut original as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
        }
        let steps = full.pool.used.load(Ordering::Relaxed);
        for cut in 0..=steps {
            let mut owner = capture::<N>();
            let mut buffer = RoundBuffer::new();
            let execution = Product::new(entry, snapshot, relations, projections, head, anchor);
            let mut frame = product::Frame::new(&execution.tables);
            let mut first = StepGovernor::new(cut);
            frame
                .advance(
                    &execution,
                    &mut buffer,
                    &mut first,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            // Neither a BTreeMap iterator nor a solution borrow may survive a pause.
            // Move both the frontiers and every original relation before resuming.
            *relations = relocate(std::mem::take(relations))
                .into_iter()
                .map(relocate)
                .collect();
            *projections = relocate(std::mem::take(projections));
            let execution = Product::new(entry, snapshot, relations, projections, head, anchor);
            let mut rest = StepGovernor::new(u64::MAX);
            frame
                .advance(
                    &execution,
                    &mut buffer,
                    &mut rest,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            assert!(frame.is_complete());
            assert_eq!(
                buffer.entries, expected.entries,
                "original product, cut {cut}"
            );
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps
            );
            assert_eq!(first.row_visits + rest.row_visits, 0);
            if evidenced {
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
            }
        }
        let mut owner = capture::<N>();
        let mut buffer = RoundBuffer::new();
        let execution = Product::new(entry, snapshot, relations, projections, head, anchor);
        let mut frame = product::Frame::new(&execution.tables);
        let mut credits = 0;
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance(
                    &execution,
                    &mut buffer,
                    &mut page,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let admitted = page.pool.used.load(Ordering::Relaxed);
            assert!(admitted > 0 || frame.is_complete());
            credits += admitted;
        }
        assert_eq!(buffer.entries, expected.entries);
        assert_eq!(credits, steps);
        if evidenced {
            assert_eq!(
                owner.rows.workspace().canonical_index().unwrap(),
                original.rows.workspace().canonical_index().unwrap()
            );
        }
    }

    fn check_products<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = fixture::<N>();
        let rule = DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?a", "https://example.org/result", "?c"),
                atom("?a", "https://example.org/result", "?f"),
            ])],
            Vec::new(),
            vec![
                atom("?a", "https://example.org/p", "?b"),
                atom("?c", "https://example.org/q", "?d"),
                atom("?e", "https://example.org/block", "?f"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        assert_eq!(runtime.factors.len(), 3);
        let snapshot = RoundSnapshot {
            probe: None,
            rel: &rel,
            depth: (&[0, 2, u32::MAX - 1, 1, 3, u32::MAX, 4, 1, 0, 0, 2, 5, 0, 4][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: None,
        };
        for delta in [
            Delta::all(14),
            Delta { lo: 0, hi: 1 },
            Delta { lo: 3, hi: 5 },
            Delta { lo: 10, hi: 12 },
            Delta::all(0),
        ] {
            let entry = RuleEntry {
                index: 3,
                rule: &rule,
                plan: &plan,
                runtime: &runtime,
                delta,
            };
            let mut relations = runtime
                .factors
                .iter()
                .map(|factor| {
                    full_matches(
                        factor,
                        entry,
                        snapshot,
                        &mut StepGovernor::new(u64::MAX),
                        &mut None,
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            for (head, arguments) in runtime.head.iter().enumerate() {
                for evidenced in [false, true] {
                    let mut projections = runtime
                        .factors
                        .iter()
                        .zip(&relations)
                        .map(|(factor, relation)| {
                            let slots = arguments
                                .iter()
                                .filter_map(|arg| match arg {
                                    ArgShape::Slot(slot) if factor.slots.contains(slot) => {
                                        Some(*slot)
                                    }
                                    ArgShape::Slot(_) | ArgShape::Const(_) => None,
                                })
                                .collect::<std::collections::BTreeSet<_>>();
                            project(
                                relation,
                                &slots.into_iter().collect::<Vec<_>>(),
                                snapshot,
                                delta,
                                evidenced,
                            )
                            .unwrap()
                        })
                        .collect::<Vec<_>>();
                    for anchor in 0..3 {
                        compare_product(
                            entry,
                            snapshot,
                            &mut relations,
                            &mut projections,
                            head,
                            anchor,
                            evidenced,
                        );
                    }
                }
            }
        }
        let delta = Delta::all(14);
        let entry = RuleEntry {
            index: 3,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta,
        };
        let relations = runtime
            .factors
            .iter()
            .map(|factor| {
                full_matches(
                    factor,
                    entry,
                    snapshot,
                    &mut StepGovernor::new(u64::MAX),
                    &mut None,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let projections = relations
            .iter()
            .map(|relation| project(relation, &[], snapshot, delta, false).unwrap())
            .collect::<Vec<_>>();
        let execution = Product::new(entry, snapshot, &relations, &projections, 0, 0);
        let mut frame = product::Frame::new(&execution.tables);
        let mut buffer = RoundBuffer::new();
        frame
            .advance(
                &execution,
                &mut buffer,
                &mut StepGovernor::new(0),
                &mut None,
            )
            .unwrap();
        assert!(!frame.is_complete());
        rel.interner().lost.store(true, Ordering::Relaxed);
        let mut page = StepGovernor::new(u64::MAX);
        assert_eq!(
            frame.advance(&execution, &mut buffer, &mut page, &mut None),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(page.pool.used.load(Ordering::Relaxed), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn triple_products_resume_original_frontiers_after_relation_moves_at_every_cut() {
        check_products(|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
    }

    #[test]
    fn quad_products_resume_original_frontiers_after_relation_moves_at_every_cut() {
        check_products(|subject, predicate, object| {
            ClauseAtom::quad(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
                ClauseTerm::iri("https://example.org/g"),
            )
        });
    }

    #[test]
    fn triple_factor_modes_resume_native_cycles_and_both_signed_filters_at_every_cut() {
        check_shapes(|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
    }

    #[test]
    fn quad_factor_modes_resume_native_cycles_and_both_signed_filters_at_every_cut() {
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
    fn factor_resume_validates_the_original_source_before_another_native_probe() {
        let rel = fixture::<3>();
        let rule = rule(&|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let delta = Delta::all(14);
        let indexes = DeltaIndexes::build(&rel, [delta]).unwrap();
        let entry = RuleEntry {
            index: 3,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta,
        };
        let snapshot = JoinSnapshot {
            seed: None,
            rel: &rel,
            delta,
            indexes: &indexes,
        };
        let factor = &runtime.factors[0];
        let mut frame = Frame::new(factor, entry, snapshot, JoinScan::Uniform(Scan::Full)).unwrap();
        frame
            .advance(
                factor,
                entry,
                snapshot,
                &mut StepGovernor::new(0),
                &mut None,
            )
            .unwrap();
        assert!(!frame.is_complete());
        let probes = rel.interner().lookups.load(Ordering::Relaxed);
        rel.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            frame.advance(
                factor,
                entry,
                snapshot,
                &mut StepGovernor::new(0),
                &mut None
            ),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(rel.interner().lookups.load(Ordering::Relaxed), probes);
    }
}
