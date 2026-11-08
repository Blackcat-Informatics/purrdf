// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned depth-first negative conjunctions, including original guard batches.

use super::{
    DictionaryTerm, EvalError, GroupArg, GroupGuard, GroupProbe, GuardSite, LocalValue,
    NegationRuntime, ProbeValue, SlotValue, StepGovernor, TermDictionary, TermId, TermIdentity,
    evidence::RuleEvidence,
    guards,
    negative::{ScanStep, Selection},
};
use crate::dictionary::DictionaryValue;
use crate::store::{Arity, StoreShape};
use purrdf_core::evidence::{EvidenceCombination, EvidenceId};

#[derive(Debug)]
struct Trace {
    current: EvidenceId,
    witnesses: Option<EvidenceId>,
}

#[derive(Debug)]
struct Undo {
    locals: Vec<usize>,
    previous: Option<EvidenceId>,
}

#[derive(Debug)]
struct Atom<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    index: usize,
    scan: Selection<I, N>,
    undo: Option<Undo>,
}

#[derive(Debug)]
enum Call<I: TermIdentity> {
    Reserve,
    Invoke(guards::Invocation<I>),
    Rows(std::vec::IntoIter<Vec<DictionaryValue<I>>>),
}

#[derive(Debug)]
struct Guard<I: TermIdentity> {
    index: usize,
    call: Call<I>,
    bound: bool,
}

#[derive(Debug)]
enum Level<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Atom(Atom<I, N>),
    Guard(Guard<I>),
}

/// Local bindings, native DFS positions and evidence stay owned across suspension.
/// The enclosing execution retains the original program, source and workspace.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    started: bool,
    locals: Vec<Option<LocalValue<I>>>,
    stack: Vec<Level<I, N>>,
    next: Option<usize>,
    trace: Option<Trace>,
    outcome: Option<(bool, Option<EvidenceId>)>,
    #[cfg(test)]
    visits: (u64, u64),
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Bind the original evidence mode without allocating before admission.
    pub(super) fn new(evidenced: bool) -> Self {
        Self {
            started: false,
            locals: Vec::new(),
            stack: Vec::new(),
            next: None,
            trace: evidenced.then_some(Trace {
                current: EvidenceId::DEFAULT,
                witnesses: None,
            }),
            outcome: None,
            #[cfg(test)]
            visits: (0, 0),
        }
    }

    /// Whether the original conjunction search has made its final decision.
    pub(super) fn is_complete(&self) -> bool {
        self.outcome.is_some()
    }

    /// Pending search carries no decision or absence evidence.
    pub(super) fn outcome(&self) -> (bool, Option<EvidenceId>) {
        self.outcome.unwrap_or((false, None))
    }

    fn finish<D: TermDictionary<Id = I>>(
        &mut self,
        probe: GroupProbe<'_, D, N>,
        found: bool,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        self.outcome = Some(match &self.trace {
            None => (found, None),
            Some(trace) => {
                let owner = evidence
                    .as_deref_mut()
                    .expect("the original evidence workspace");
                let (id, blocked) = owner.negative(trace.witnesses)?;
                (blocked, Some(id))
            }
        });
        self.stack = Vec::new();
        self.locals = Vec::new();
        self.next = None;
        probe.rel.interner().check_read().map_err(EvalError::Source)
    }

    /// Resume the same original DFS; a refused admission never unwinds its parents.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        runtime: &NegationRuntime<N>,
        probe: GroupProbe<'_, D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        probe
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        if !self.started {
            if !governor.charge() {
                return probe.rel.interner().check_read().map_err(EvalError::Source);
            }
            self.started = true;
            self.locals = vec![None; runtime.locals];
            self.next = Some(0);
        }
        while !self.is_complete() {
            if let Some(position) = self.next.take() {
                if let Some(atom) = runtime.atoms.get(position) {
                    let mut values = [None; N];
                    let mut absent = false;
                    for (coordinate, arg) in atom.iter().enumerate() {
                        match NegationRuntime::probe_value(arg, probe, &self.locals)
                            .map_err(EvalError::Source)?
                        {
                            ProbeValue::Known(id) => values[coordinate] = Some(id),
                            ProbeValue::Free => {}
                            ProbeValue::Absent => {
                                absent = true;
                                break;
                            }
                        }
                    }
                    if !absent {
                        let source = probe.rel.read_source(runtime.sources[position]);
                        source.interner().check_read().map_err(EvalError::Source)?;
                        self.stack.push(Level::Atom(Atom {
                            index: position,
                            scan: Selection::new(source, values),
                            undo: None,
                        }));
                    }
                } else if position - runtime.atoms.len() < runtime.guards.len() {
                    self.stack.push(Level::Guard(Guard {
                        index: position - runtime.atoms.len(),
                        call: Call::Reserve,
                        bound: false,
                    }));
                } else if let Some(trace) = &mut self.trace {
                    let owner = evidence
                        .as_deref_mut()
                        .expect("the original evidence workspace");
                    trace.witnesses = Some(match trace.witnesses {
                        None => trace.current,
                        Some(previous) => owner.combine(
                            previous,
                            trace.current,
                            EvidenceCombination::Alternatives,
                        )?,
                    });
                } else {
                    self.finish(probe, true, evidence)?;
                }
                continue;
            }
            let Some(level) = self.stack.pop() else {
                self.finish(probe, false, evidence)?;
                break;
            };
            match level {
                Level::Atom(mut atom) => {
                    if let Some(undo) = atom.undo.take() {
                        for local in undo.locals {
                            self.locals[local] = None;
                        }
                        if let (Some(trace), Some(previous)) = (&mut self.trace, undo.previous) {
                            trace.current = previous;
                        }
                    }
                    let source = probe.rel.read_source(runtime.sources[atom.index]);
                    source.interner().check_read().map_err(EvalError::Source)?;
                    #[cfg(test)]
                    let before = atom.scan.counts();
                    let step = atom.scan.next::<true, D>(source, governor);
                    #[cfg(test)]
                    {
                        let after = atom.scan.counts();
                        self.visits.0 += after.0 - before.0;
                        self.visits.1 += after.1 - before.1;
                    }
                    match step {
                        ScanStep::Paused => {
                            self.stack.push(Level::Atom(atom));
                            break;
                        }
                        ScanStep::Exhausted => {}
                        ScanStep::Matched(terms, row) => {
                            let mut newly = Vec::new();
                            let mut consistent = true;
                            for (position, arg) in runtime.atoms[atom.index].iter().enumerate() {
                                if let GroupArg::Local(local) = arg {
                                    match &self.locals[*local] {
                                        None => {
                                            self.locals[*local] =
                                                Some(LocalValue::Interned(terms[position]));
                                            newly.push(*local);
                                        }
                                        Some(LocalValue::Interned(id))
                                            if *id == terms[position] => {}
                                        Some(_) => {
                                            consistent = false;
                                            break;
                                        }
                                    }
                                }
                            }
                            if consistent {
                                let previous = if let Some(trace) = &mut self.trace {
                                    let previous = trace.current;
                                    let owner = evidence
                                        .as_deref_mut()
                                        .expect("the original evidence workspace");
                                    let id = owner.source(row)?;
                                    trace.current = owner.combine(
                                        previous,
                                        id,
                                        EvidenceCombination::Conjunction,
                                    )?;
                                    Some(previous)
                                } else {
                                    None
                                };
                                atom.undo = Some(Undo {
                                    locals: newly,
                                    previous,
                                });
                                self.next = Some(atom.index + 1);
                            } else {
                                for local in newly {
                                    self.locals[local] = None;
                                }
                            }
                            self.stack.push(Level::Atom(atom));
                        }
                    }
                }
                Level::Guard(mut guard) => {
                    let lowered = &runtime.guards[guard.index];
                    if guard.bound {
                        for &local in &lowered.outputs {
                            self.locals[local] = None;
                        }
                        guard.bound = false;
                    }
                    if matches!(guard.call, Call::Reserve) {
                        if !governor.charge() {
                            self.stack.push(Level::Guard(guard));
                            break;
                        }
                        guard.call = Call::Invoke(guards::Invocation::Uncalled);
                    }
                    if let Call::Invoke(invocation) = &mut guard.call {
                        let context = guards::Context {
                            rule: probe.rule,
                            site: GuardSite::Negation {
                                negation: runtime.index,
                                guard: guard.index,
                            },
                            guard: &probe.clause.negations()[runtime.index].guards()[guard.index],
                            model: probe.rel,
                        };
                        invocation.advance_with(&context, governor, || {
                            let inputs = inputs(lowered, probe, &self.locals)?;
                            context.invoke(probe.guards, &inputs)
                        })?;
                        if !invocation.is_complete() {
                            self.stack.push(Level::Guard(guard));
                            break;
                        }
                        let invocation =
                            std::mem::replace(invocation, guards::Invocation::Uncalled);
                        guard.call = Call::Rows(invocation.into_rows().into_iter());
                    }
                    let Call::Rows(rows) = &mut guard.call else {
                        unreachable!("the original invocation completed its batch admission")
                    };
                    if let Some(row) = rows.next() {
                        for (&local, value) in lowered.outputs.iter().zip(&row) {
                            self.locals[local] = Some(
                                match value
                                    .as_term()
                                    .canonicalize(probe.rel.interner())
                                    .map_err(EvalError::Source)?
                                {
                                    DictionaryTerm::Id(id) => LocalValue::Interned(id),
                                    DictionaryTerm::Surface(surface) => {
                                        LocalValue::Computed(surface.into())
                                    }
                                },
                            );
                        }
                        guard.bound = true;
                        self.next = Some(runtime.atoms.len() + guard.index + 1);
                        self.stack.push(Level::Guard(guard));
                    }
                }
            }
        }
        probe.rel.interner().check_read().map_err(EvalError::Source)
    }
}

/// Borrow the original local/outer values only for an invocation's first call.
fn inputs<'a, D: TermDictionary, const N: usize>(
    guard: &'a GroupGuard,
    probe: GroupProbe<'a, D, N>,
    locals: &'a [Option<LocalValue<D::Id>>],
) -> Result<Vec<DictionaryTerm<'a, D::Id>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut inputs = Vec::with_capacity(guard.inputs.len());
    for input in &guard.inputs {
        let value = match input {
            GroupArg::Outer(slot) => probe
                .solution
                .value(*slot)
                .as_term()
                .expect("a bound outer input"),
            GroupArg::Local(local) => match &locals[*local] {
                Some(LocalValue::Interned(id)) => SlotValue::Interned(*id),
                Some(LocalValue::Computed(surface)) => SlotValue::Computed(surface),
                None => unreachable!("a local input is bound before its guard"),
            }
            .as_term()
            .expect("a bound local input"),
            GroupArg::Const(surface) => DictionaryTerm::Surface(surface)
                .canonicalize(probe.rel.interner())
                .map_err(EvalError::Source)?,
        };
        inputs.push(value);
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
    use crate::guard::{
        CallbackError, Guard as Declaration, GuardCall, GuardEvaluator, GuardRows, Negation,
    };
    use crate::id::RowId;
    use crate::plan::RulePlan;
    use crate::seminaive::{GroupEvidence, RuleRuntime, SlotSolution, evidence, slot_of};
    use crate::test_support::{HostDictionary, SourceLost, evidence_scope};
    use purrdf_lex::allocation::Resident;
    use std::sync::{Mutex, atomic::Ordering};

    const WIDE: u64 = (1 << 54) + 1;
    const GRAPH: u64 = 1 << 63;
    const FRESH: &str = "<https://example.org/fresh>";

    #[derive(Default)]
    struct Callbacks {
        accept: bool,
        calls: Mutex<Vec<(GuardSite, String)>>,
    }

    impl<const N: usize> GuardEvaluator<HostDictionary, N> for Callbacks
    where
        Arity<N>: StoreShape<N>,
    {
        fn evaluate(
            &self,
            call: &GuardCall<'_, HostDictionary, N>,
        ) -> Result<GuardRows<u64>, CallbackError<SourceLost>> {
            self.calls
                .lock()
                .unwrap()
                .push((call.site, format!("{:?}", call.inputs)));
            Ok(match call.guard.name() {
                "expand" => {
                    let DictionaryTerm::Id(id) = call.inputs[0] else {
                        panic!("original native input")
                    };
                    vec![
                        vec![DictionaryValue::Id(id)],
                        vec![DictionaryValue::Id(WIDE)],
                        vec![DictionaryValue::Surface(FRESH.into())],
                    ]
                }
                "accept" => {
                    if self.accept
                        && matches!(call.inputs[0], DictionaryTerm::Surface(text) if text == FRESH)
                    {
                        vec![Vec::new()]
                    } else {
                        Vec::new()
                    }
                }
                name => panic!("unexpected guard {name}"),
            })
        }
    }

    fn fixture<const N: usize>() -> super::super::RelationStore<HostDictionary, N>
    where
        Arity<N>: StoreShape<N>,
    {
        let mut rel = super::super::RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                (7, "<https://example.org/p>".into()),
                (8, "<https://example.org/q>".into()),
                (9, "<https://example.org/r>".into()),
                (17, "<https://example.org/seed>".into()),
                (GRAPH, "<https://example.org/g>".into()),
            ]
            .into_iter()
            .collect(),
            ..HostDictionary::default()
        });
        for row in [
            [WIDE, 7, 10],
            [WIDE, 7, 11],
            [10, 8, 0],
            [10, 8, u64::MAX],
            [11, 8, 22],
            [0, 9, 0],
            [u64::MAX, 9, 123],
            [22, 9, 22],
        ] {
            rel.insert_ids(std::array::from_fn(|position| {
                if position < 3 { row[position] } else { GRAPH }
            }))
            .unwrap();
        }
        rel
    }

    fn program<const N: usize>(
        kind: usize,
        atom: &impl Fn(&str, &str, &str) -> ClauseAtom<N>,
    ) -> DlClause<N> {
        let atoms = match kind {
            0 => vec![
                atom("?x", "https://example.org/p", "?a"),
                atom("?a", "https://example.org/q", "?b"),
                atom("?b", "https://example.org/r", "?b"),
            ],
            1 => vec![atom("?b", "https://example.org/r", "?b")],
            2 | 3 => Vec::new(),
            _ => vec![atom("?x", "https://example.org/absent", "?b")],
        };
        let guards = match kind {
            0..=2 => vec![
                Declaration::assign(
                    "expand",
                    vec![if kind == 2 { "?x" } else { "?b" }.into()],
                    "?calc",
                ),
                Declaration::filter("accept", vec!["?calc".into(), "?x".into()]),
            ],
            3 => vec![Declaration::not_equal("?x", "?y")],
            _ => Vec::new(),
        };
        DlClause::datalog(
            atom("?x", "https://example.org/result", "?y"),
            vec![atom("?x", "https://example.org/seed", "?y")],
        )
        .with_negations(vec![Negation::new(atoms, guards)])
    }

    fn capture<const N: usize>(rows: usize) -> evidence::Capture<Resident, u64, N>
    where
        Arity<N>: StoreShape<N>,
    {
        use purrdf_core::evidence::{
            EvidenceRecord, Frame as World, Information, InformationMap, SearchCompleteness,
        };
        let mut rows = crate::evidence::DerivationEvidence::with_admission(rows, Resident);
        rows.seed(RowId::from_index(0).unwrap(), |memory| {
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
        evidence::Capture::new(rows, evidence_scope())
    }

    fn oracle<const N: usize>(
        runtime: &NegationRuntime<N>,
        probe: GroupProbe<'_, HostDictionary, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<HostDictionary, N>>,
    ) -> (bool, Option<EvidenceId>)
    where
        Arity<N>: StoreShape<N>,
    {
        let Some(owner) = evidence.as_deref_mut() else {
            return (runtime.holds(probe, governor).unwrap(), None);
        };
        assert!(governor.charge());
        let mut trace = Some(GroupEvidence {
            owner,
            current: EvidenceId::DEFAULT,
            witnesses: None,
        });
        runtime
            .match_atom(
                0,
                probe,
                &mut vec![None; runtime.locals],
                governor,
                &mut trace,
            )
            .unwrap();
        let trace = trace.unwrap();
        let (id, blocked) = trace.owner.negative(trace.witnesses).unwrap();
        (blocked, Some(id))
    }

    fn compare<const N: usize>(
        runtime: &NegationRuntime<N>,
        probe: GroupProbe<'_, HostDictionary, N>,
        accept: bool,
        evidenced: bool,
    ) where
        Arity<N>: StoreShape<N>,
    {
        let lookups = || probe.rel.interner().lookups.load(Ordering::Relaxed);
        let callbacks = || Callbacks {
            accept,
            ..Callbacks::default()
        };
        let before = lookups();
        let original_calls = callbacks();
        let mut original = capture::<N>(probe.rel.row_count());
        let mut full = StepGovernor::new(u64::MAX);
        let expected = oracle(
            runtime,
            GroupProbe {
                guards: &original_calls,
                ..probe
            },
            &mut full,
            &mut evidenced.then_some(&mut original as &mut dyn RuleEvidence<HostDictionary, N>),
        );
        let steps = full.pool.used.load(Ordering::Relaxed);
        let probes = lookups() - before;
        let calls = original_calls.calls.into_inner().unwrap();
        let mut reference_visits = None;
        for cut in 0..=steps {
            let before = lookups();
            let callbacks = callbacks();
            let probe = GroupProbe {
                guards: &callbacks,
                ..probe
            };
            let mut owner = capture::<N>(probe.rel.row_count());
            let mut frame = Frame::new(evidenced);
            let mut first = StepGovernor::new(cut);
            frame
                .advance(
                    runtime,
                    probe,
                    &mut first,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let mut rest = StepGovernor::new(u64::MAX);
            if !frame.is_complete() {
                assert!(first.spent());
                assert_eq!(first.observed(), cut + 1);
                assert_eq!(frame.outcome(), (false, None));
                frame
                    .advance(
                        runtime,
                        probe,
                        &mut rest,
                        &mut evidenced
                            .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                    )
                    .unwrap();
            }
            assert!(frame.is_complete());
            assert_eq!(
                frame.outcome(),
                expected,
                "decision and signed witness, cut {cut}"
            );
            assert_eq!(
                first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
                steps,
                "successful credits, cut {cut}"
            );
            assert_eq!(
                *callbacks.calls.lock().unwrap(),
                calls,
                "opaque invocation trace, cut {cut}"
            );
            assert_eq!(
                lookups() - before,
                probes,
                "original term probes, cut {cut}"
            );
            if let Some(visits) = reference_visits {
                assert_eq!(frame.visits, visits);
            } else {
                reference_visits = Some(frame.visits);
            }
            if evidenced {
                assert_eq!(
                    owner.rows.workspace().canonical_index().unwrap(),
                    original.rows.workspace().canonical_index().unwrap()
                );
                assert_eq!(
                    owner.rows.workspace().get(expected.1.unwrap()).unwrap(),
                    original.rows.workspace().get(expected.1.unwrap()).unwrap()
                );
            }
        }
        let before = lookups();
        let callbacks = callbacks();
        let probe = GroupProbe {
            guards: &callbacks,
            ..probe
        };
        let mut owner = capture::<N>(probe.rel.row_count());
        let mut frame = Frame::new(evidenced);
        let mut admitted = 0;
        while !frame.is_complete() {
            let mut page = StepGovernor::new(1);
            frame
                .advance(
                    runtime,
                    probe,
                    &mut page,
                    &mut evidenced
                        .then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
                )
                .unwrap();
            let credits = page.pool.used.load(Ordering::Relaxed);
            assert!(credits > 0 || frame.is_complete());
            admitted += credits;
        }
        assert_eq!(frame.outcome(), expected);
        assert_eq!(admitted, steps);
        assert_eq!(frame.visits, reference_visits.unwrap());
        assert_eq!(*callbacks.calls.lock().unwrap(), calls);
        assert_eq!(lookups() - before, probes);
        if evidenced {
            assert_eq!(
                owner.rows.workspace().canonical_index().unwrap(),
                original.rows.workspace().canonical_index().unwrap()
            );
        }
        let mut empty = StepGovernor::new(0);
        frame
            .advance(
                runtime,
                probe,
                &mut empty,
                &mut evidenced.then_some(&mut owner as &mut dyn RuleEvidence<HostDictionary, N>),
            )
            .unwrap();
        assert_eq!(empty.observed(), 0);
        assert_eq!(*callbacks.calls.lock().unwrap(), calls);
    }

    fn check_shapes<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
    where
        Arity<N>: StoreShape<N>,
    {
        let rel = fixture::<N>();
        let resolves = rel.interner().resolves.load(Ordering::Relaxed);
        for kind in 0..5 {
            let rule = program(kind, &atom);
            let plan = RulePlan::for_rule(&rule);
            let runtime = RuleRuntime::new(&rule, &plan);
            let mut solution = SlotSolution::empty(plan.variables().len());
            solution.bindings[slot_of("?x", plan.variables())] = Some(WIDE);
            solution.bindings[slot_of("?y", plan.variables())] = Some(u64::MAX);
            for accept in [false, true] {
                for evidenced in [false, true] {
                    compare(
                        &runtime.negations[0],
                        GroupProbe {
                            solution: &solution,
                            rel: &rel,
                            guards: &super::super::NoGuards,
                            rule: 2,
                            clause: &rule,
                        },
                        accept,
                        evidenced,
                    );
                }
            }
        }
        assert_eq!(rel.interner().resolves.load(Ordering::Relaxed), resolves);
    }

    #[test]
    fn triple_negative_groups_resume_the_original_dfs_guards_and_signed_witnesses() {
        check_shapes(|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
    }

    #[test]
    fn quad_negative_groups_resume_the_original_dfs_guards_and_signed_witnesses() {
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
    fn negative_group_resume_checks_source_before_revisiting_a_callback_batch() {
        let rel = fixture::<3>();
        let rule = program(2, &|subject, predicate, object| {
            ClauseAtom::triple(
                ClauseTerm::var(subject),
                ClauseTerm::iri(predicate),
                ClauseTerm::var(object),
            )
        });
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let mut solution = SlotSolution::empty(plan.variables().len());
        solution.bindings[slot_of("?x", plan.variables())] = Some(WIDE);
        let callbacks = Callbacks::default();
        let probe = GroupProbe {
            solution: &solution,
            rel: &rel,
            guards: &callbacks,
            rule: 2,
            clause: &rule,
        };
        let mut frame = Frame::new(false);
        frame
            .advance(
                &runtime.negations[0],
                probe,
                &mut StepGovernor::new(3),
                &mut None,
            )
            .unwrap();
        assert!(!frame.is_complete());
        assert_eq!(callbacks.calls.lock().unwrap().len(), 1);
        let lookups = rel.interner().lookups.load(Ordering::Relaxed);
        rel.interner().lost.store(true, Ordering::Relaxed);
        assert_eq!(
            frame.advance(
                &runtime.negations[0],
                probe,
                &mut StepGovernor::new(0),
                &mut None
            ),
            Err(EvalError::Source(SourceLost(99)))
        );
        assert_eq!(callbacks.calls.lock().unwrap().len(), 1);
        assert_eq!(rel.interner().lookups.load(Ordering::Relaxed), lookups);
    }

    #[test]
    fn negative_conjunction_depth_is_owned_on_the_heap_across_a_pause() {
        const DEPTH: usize = 2048;
        let rel = fixture::<3>();
        let atom = |predicate: &str| {
            ClauseAtom::triple(
                ClauseTerm::var("?x"),
                ClauseTerm::iri(predicate),
                ClauseTerm::var("?a"),
            )
        };
        let rule = DlClause::datalog(
            ClauseAtom::triple(
                ClauseTerm::var("?x"),
                ClauseTerm::iri("https://example.org/result"),
                ClauseTerm::var("?y"),
            ),
            vec![ClauseAtom::triple(
                ClauseTerm::var("?x"),
                ClauseTerm::iri("https://example.org/seed"),
                ClauseTerm::var("?y"),
            )],
        )
        .with_negations(vec![Negation::new(
            vec![atom("https://example.org/p"); DEPTH],
            Vec::new(),
        )]);
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let mut solution = SlotSolution::empty(plan.variables().len());
        solution.bindings[slot_of("?x", plan.variables())] = Some(WIDE);
        solution.bindings[slot_of("?y", plan.variables())] = Some(u64::MAX);
        let probe = GroupProbe {
            solution: &solution,
            rel: &rel,
            guards: &super::super::NoGuards,
            rule: 0,
            clause: &rule,
        };
        let mut frame = Frame::new(false);
        let mut first = StepGovernor::new(DEPTH as u64);
        frame
            .advance(&runtime.negations[0], probe, &mut first, &mut None)
            .unwrap();
        assert!(!frame.is_complete());
        let mut rest = StepGovernor::new(u64::MAX);
        frame
            .advance(&runtime.negations[0], probe, &mut rest, &mut None)
            .unwrap();
        assert!(frame.is_complete());
        assert_eq!(frame.outcome(), (true, None));
        assert_eq!(
            first.pool.used.load(Ordering::Relaxed) + rest.pool.used.load(Ordering::Relaxed),
            1 + 2 * DEPTH as u64
        );
        assert_eq!(frame.visits, (DEPTH as u64, DEPTH as u64));
    }
}
