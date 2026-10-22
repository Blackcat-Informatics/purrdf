// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Head-bound probes through the original rule, guard and factor owners.

use super::{
    ArgShape, BudgetReport, BudgetResource, Candidate, Delta, DeltaIndexes, Derivation, EvalError,
    Executable, FixpointState, GuardEvaluator, HeadKey, HeadTerm, JoinStrategy, ResumeProgress,
    RoundSnapshot, RuleBindings, RuleEntry, STATED_JOIN_STEPS, SlotSolution, StepGovernor,
    StopSignal, TermDictionary, is_stopped, rule,
};
use crate::clause::ReadSource;
use crate::id::RowId;
use crate::store::{Arity, RelationStore, StoreShape};
use std::fmt;
use std::sync::atomic::Ordering;

/// A head probe could not borrow its original completed model or execute its rule.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ProbeError<E = core::convert::Infallible> {
    /// The original evaluation has not settled all strata.
    ModelIncomplete,
    /// The original completed model was already moved out of its session.
    ModelTaken,
    /// A canonical witness was requested before the probe exhausted its matches.
    ProbeIncomplete,
    /// The authored rule coordinate does not belong to this program.
    RuleOutOfBounds {
        /// Requested original rule ordinal.
        rule: usize,
        /// Actual number of authored rules.
        available: usize,
    },
    /// The authored head coordinate does not belong to this rule.
    HeadOutOfBounds {
        /// Original rule ordinal.
        rule: usize,
        /// Requested head ordinal.
        head: usize,
        /// Actual number of head conjuncts.
        available: usize,
    },
    /// Original source, guard, storage or invariant failure.
    Failed(EvalError<E>),
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError for ProbeError<E> {
    purrdf_hash::error_visit! {
        ModelIncomplete => "datalog-probe-model-incomplete" {},
        ModelTaken => "datalog-probe-model-taken" {},
        ProbeIncomplete => "datalog-probe-incomplete" {},
        RuleOutOfBounds { rule, available } => "datalog-probe-rule" { "rule" => Unsigned(*rule as u128), "available" => Unsigned(*available as u128) },
        HeadOutOfBounds { rule, head, available } => "datalog-probe-head" { "rule" => Unsigned(*rule as u128), "head" => Unsigned(*head as u128), "available" => Unsigned(*available as u128) },
        Failed(cause) => "datalog-probe-failed" { "cause" => Cause(cause) },
    }
}

impl<E: fmt::Display> fmt::Display for ProbeError<E> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelIncomplete => {
                out.write_str("a head probe requires the original completed model")
            }
            Self::ModelTaken => {
                out.write_str("the original completed model has already been taken")
            }
            Self::ProbeIncomplete => {
                out.write_str("the head probe has not exhausted its original matches")
            }
            Self::RuleOutOfBounds { rule, available } => write!(
                out,
                "rule {rule} is outside the program's {available} rules"
            ),
            Self::HeadOutOfBounds {
                rule,
                head,
                available,
            } => write!(
                out,
                "head {head} is outside rule {rule}'s {available} heads"
            ),
            Self::Failed(error) => error.fmt(out),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ProbeError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }
}

/// A read-only query pins the original model, program, guards and native target.
///
/// The real rule frames receive the bound head variables before any body atom.
/// Pausing retains those frames and returned callback batches; it never restarts
/// the body or inserts/removes a model row. Runtime IDs are neither serialized
/// nor hashed into a persistent query identity.
pub struct HeadProbe<'a, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    state: &'a FixpointState<D, N>,
    guards: &'a dyn GuardEvaluator<D, N>,
    frame: Frame<'a, D, N>,
}

/// Native pending work owns its bindings and body frames without borrowing the model.
/// A transaction can retain this beside its state and lend that state on each resume.
/// The owner keeps model contents and guard semantics fixed until this work completes;
/// the public query enforces that rule through its shared model borrow.
pub(super) struct Frame<'r, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    entry: RuleEntry<'r, N>,
    demand: RuleBindings<D::Id, N>,
    indexes: DeltaIndexes<D::Id>,
    body: Option<rule::Frame<'r, D::Id, N>>,
    output: Option<super::RoundBuffer<'r, D::Id, N>>,
    steps: u64,
    complete: bool,
    failure: Option<EvalError<D::Error>>,
}

impl<D: TermDictionary, const N: usize> fmt::Debug for HeadProbe<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("HeadProbe")
            .field("rule", &self.frame.entry.index)
            .field("target", &self.frame.demand)
            .field("join_steps", &self.frame.steps)
            .field("complete", &self.frame.complete)
            .finish_non_exhaustive()
    }
}

impl<'a, D: TermDictionary, const N: usize> HeadProbe<'a, D, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new(
        exe: &'a Executable<N>,
        state: &'a FixpointState<D, N>,
        guards: &'a dyn GuardEvaluator<D, N>,
        index: usize,
        head: usize,
        terms: [D::Id; N],
    ) -> Result<Self, ProbeError<D::Error>> {
        Ok(Self {
            state,
            guards,
            frame: Frame::new(exe, state, index, head, terms)?,
        })
    }

    /// Successful cumulative body/guard/head credits, excluding refused attempts.
    pub const fn join_steps(&self) -> u64 {
        self.frame.steps
    }

    /// Resume under a cumulative join ceiling and a fresh optional stop signal.
    ///
    /// # Errors
    /// Preserves and poisons original source/guard failures. Physical refusals
    /// retain the native frames and return a typed paused progress value.
    pub fn resume(
        &mut self,
        max_join_steps: u64,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, ProbeError<D::Error>> {
        self.frame
            .resume(self.state, self.guards, max_join_steps, stop)
    }

    /// Borrow the canonical completed local application, or a completed absence.
    ///
    /// # Errors
    /// Refuses an incomplete probe, poisoned execution or dead original source.
    pub fn witness(&self) -> Result<Option<HeadWitness<'_, D, N>>, ProbeError<D::Error>> {
        self.frame.witness(self.state)
    }
}

impl<'r, D: TermDictionary, const N: usize> Frame<'r, D, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new(
        exe: &'r Executable<N>,
        state: &FixpointState<D, N>,
        index: usize,
        head: usize,
        terms: [D::Id; N],
    ) -> Result<Self, ProbeError<D::Error>> {
        let dictionary = state.rel.interner();
        dictionary
            .check_read()
            .map_err(|error| ProbeError::Failed(EvalError::Source(error)))?;
        if index >= exe.rule_count() {
            return Err(ProbeError::RuleOutOfBounds {
                rule: index,
                available: exe.rule_count(),
            });
        }
        let (_, plan) = exe.rule_entry(index);
        let runtime = &exe.runtimes()[index];
        let Some(arguments) = runtime.head.get(head) else {
            return Err(ProbeError::HeadOutOfBounds {
                rule: index,
                head,
                available: runtime.head.len(),
            });
        };
        let mut seed = SlotSolution::empty(plan.variables().len());
        let matches = bind_native(
            &mut seed,
            arguments.each_ref().map(|arg| match arg {
                ArgShape::Slot(slot) => (Some(*slot), None),
                ArgShape::Const(surface) => (None, Some(surface.as_str())),
            }),
            terms,
            dictionary,
        )
        .map_err(|error| ProbeError::Failed(EvalError::Source(error)))?;
        Ok(Self::seeded(
            exe,
            state,
            index,
            RuleBindings {
                target: Some((head, terms)),
                seed,
            },
            matches,
        ))
    }

    /// Bind an original positive atom, retaining every head including present rows.
    pub(super) fn from_body(
        exe: &'r Executable<N>,
        state: &FixpointState<D, N>,
        index: usize,
        body: usize,
        terms: [D::Id; N],
    ) -> Result<Self, EvalError<D::Error>> {
        let (_, plan) = exe.rule_entry(index);
        let atom = plan
            .operators()
            .iter()
            .find(|op| op.body_index() == body)
            .expect("maintenance names an original positive atom");
        let mut seed = SlotSolution::empty(plan.variables().len());
        let matches = bind_native(
            &mut seed,
            atom.shape()
                .positions()
                .each_ref()
                .map(|p| (p.slot(), p.constant())),
            terms,
            state.rel.interner(),
        )
        .map_err(EvalError::Source)?;
        Ok(Self::seeded(
            exe,
            state,
            index,
            RuleBindings { target: None, seed },
            matches,
        ))
    }

    /// Re-enter an affected absence-dependent rule through the original kernels.
    pub(super) fn all(exe: &'r Executable<N>, state: &FixpointState<D, N>, index: usize) -> Self {
        let (_, plan) = exe.rule_entry(index);
        Self::seeded(
            exe,
            state,
            index,
            RuleBindings {
                target: None,
                seed: SlotSolution::empty(plan.variables().len()),
            },
            true,
        )
    }

    fn seeded(
        exe: &'r Executable<N>,
        state: &FixpointState<D, N>,
        index: usize,
        demand: RuleBindings<D::Id, N>,
        matches: bool,
    ) -> Self {
        let (rule, plan) = exe.rule_entry(index);
        Self {
            entry: RuleEntry {
                index,
                rule,
                plan,
                runtime: &exe.runtimes()[index],
                delta: Delta::all(state.rel.row_slots()),
            },
            demand,
            indexes: DeltaIndexes { runs: Vec::new() },
            body: None,
            output: (!matches).then(super::RoundBuffer::new),
            steps: 0,
            complete: !matches,
            failure: None,
        }
    }

    pub(super) fn into_output(self) -> super::RoundBuffer<'r, D::Id, N> {
        self.output.expect("only completed native work can publish")
    }

    pub(super) const fn join_steps(&self) -> u64 {
        self.steps
    }
}

/// One native substitution law shared by authored head and body bindings.
fn bind_native<D: TermDictionary, const N: usize>(
    seed: &mut SlotSolution<D::Id, N>,
    arguments: [(Option<usize>, Option<&str>); N],
    terms: [D::Id; N],
    dictionary: &D,
) -> Result<bool, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    dictionary.check_read()?;
    let mut matches = true;
    for (argument, value) in arguments.iter().zip(terms) {
        match argument {
            (Some(slot), None) => match seed.bindings[*slot] {
                Some(previous) if previous != value => {
                    matches = false;
                    break;
                }
                _ => seed.bindings[*slot] = Some(value),
            },
            (None, Some(surface)) => {
                if dictionary.lookup_term(surface)? != Some(value) {
                    matches = false;
                    break;
                }
            }
            _ => unreachable!("a compiled argument is exactly one constant or slot"),
        }
    }
    dictionary.check_read()?;
    Ok(matches)
}

impl<D: TermDictionary, const N: usize> Frame<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn report(&self, state: &FixpointState<D, N>, limit: u64) -> BudgetReport {
        let mut report = state.report();
        report.join_steps = self.steps;
        report.limits.join_steps = limit;
        report.flags |= STATED_JOIN_STEPS;
        report
    }

    /// Resume under a cumulative join ceiling and a fresh optional stop signal.
    ///
    /// # Errors
    /// Preserves and poisons original source/guard failures. Physical refusals
    /// retain the native frames and return a typed paused progress value.
    pub(super) fn resume(
        &mut self,
        state: &FixpointState<D, N>,
        guards: &dyn GuardEvaluator<D, N>,
        max_join_steps: u64,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ResumeProgress<D::Error>, ProbeError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(ProbeError::Failed(error.clone()));
        }
        if let Err(error) = state.rel.interner().check_read() {
            let error = EvalError::Source(error);
            self.failure = Some(error.clone());
            return Err(ProbeError::Failed(error));
        }
        if self.steps > max_join_steps {
            return Ok(ResumeProgress::Paused(EvalError::BudgetExhausted {
                resource: BudgetResource::JoinSteps,
                report: self.report(state, max_join_steps),
            }));
        }
        if self.complete {
            return Ok(ResumeProgress::Complete);
        }
        if is_stopped(stop) {
            return Ok(ResumeProgress::Paused(EvalError::Stopped {
                report: self.report(state, max_join_steps),
            }));
        }
        let snapshot = RoundSnapshot {
            rel: &state.rel,
            depth: state.depths(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: Some(&self.indexes),
            probe: Some(&self.demand),
        };
        if self.body.is_none() {
            // The all-row span never needs decomposition advice. Native head
            // bindings choose rebound indexes without an unrelated global count.
            match rule::Frame::new(self.entry, snapshot, JoinStrategy::Planned, &[]) {
                Ok(frame) => self.body = Some(frame),
                Err(EvalError::Storage(error)) => {
                    return Ok(ResumeProgress::Paused(EvalError::Storage(error)));
                }
                Err(error) => {
                    self.failure = Some(error.clone());
                    return Err(ProbeError::Failed(error));
                }
            }
        }
        let mut governor = StepGovernor::new(max_join_steps.saturating_sub(self.steps));
        let frame = self
            .body
            .as_mut()
            .expect("the original rule frame was entered");
        let result = frame.advance(self.entry, snapshot, guards, &mut governor, &mut None);
        self.steps = self
            .steps
            .saturating_add(governor.pool.used.load(Ordering::Relaxed));
        if let Err(error) = result {
            if matches!(error, EvalError::Storage(_)) {
                return Ok(ResumeProgress::Paused(error));
            }
            self.failure = Some(error.clone());
            return Err(ProbeError::Failed(error));
        }
        if !frame.is_complete() {
            assert!(
                governor.spent(),
                "an unfinished native rule exhausted its governor"
            );
            let mut report = self.report(state, max_join_steps);
            report.join_steps = report.join_steps.saturating_add(1);
            return Ok(ResumeProgress::Paused(EvalError::BudgetExhausted {
                resource: BudgetResource::JoinSteps,
                report,
            }));
        }
        let buffer = self
            .body
            .take()
            .expect("the original rule completed")
            .into_buffer();
        self.output = Some(buffer);
        self.complete = true;
        Ok(ResumeProgress::Complete)
    }

    /// Borrow the canonical completed local application, or a completed absence.
    ///
    /// # Errors
    /// Refuses an incomplete probe, poisoned execution or dead original source.
    pub(super) fn witness<'a>(
        &'a self,
        state: &'a FixpointState<D, N>,
    ) -> Result<Option<HeadWitness<'a, D, N>>, ProbeError<D::Error>> {
        if let Some(error) = &self.failure {
            return Err(ProbeError::Failed(error.clone()));
        }
        state
            .rel
            .interner()
            .check_read()
            .map_err(|error| ProbeError::Failed(EvalError::Source(error)))?;
        if !self.complete {
            return Err(ProbeError::ProbeIncomplete);
        }
        Ok(self
            .output
            .as_ref()
            .and_then(|buffer| buffer.entries.first_key_value())
            .map(|(_, candidate)| HeadWitness {
                rel: &state.rel,
                terms: &self
                    .demand
                    .target
                    .as_ref()
                    .expect("a public probe binds its head")
                    .1,
                candidate,
            }))
    }
}

/// A selected local application borrows its original model and native premises.
/// Lexical proof projection is explicit and cannot reattach a different source.
pub struct HeadWitness<'a, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) rel: &'a RelationStore<D, N>,
    pub(super) terms: &'a [D::Id; N],
    pub(super) candidate: &'a Candidate<D::Id, N>,
}

impl<D: TermDictionary, const N: usize> fmt::Debug for HeadWitness<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.debug_struct("HeadWitness")
            .field("terms", self.terms)
            .field("candidate", self.candidate)
            .finish_non_exhaustive()
    }
}

impl<D: TermDictionary, const N: usize> HeadWitness<'_, D, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Original authored rule ordinal.
    pub const fn rule(&self) -> usize {
        self.candidate.rule
    }
    /// Original authored head ordinal.
    pub const fn head(&self) -> usize {
        self.candidate.head
    }
    /// Native target in the original dictionary's identity space.
    pub const fn terms(&self) -> &[D::Id; N] {
        self.terms
    }
    /// Original positive premises, in authored body order, without lexical copies.
    pub fn premises(&self) -> impl ExactSizeIterator<Item = (ReadSource, RowId, &[D::Id; N])> {
        self.candidate
            .sources
            .iter()
            .map(|source| (source.source, source.row, &source.terms))
    }
    /// Explicit lexical local proof over the same original source.
    ///
    /// # Errors
    /// Preserves the original dictionary failure during projection.
    pub fn derivation(&self) -> Result<Derivation<N>, D::Error> {
        let key = HeadKey {
            positions: self.terms.map(HeadTerm::Interned),
        };
        Ok(Derivation {
            fact: key.fact(self.rel)?,
            rule: self.candidate.rule,
            head: self.candidate.head,
            sources: self.candidate.source_facts(self.rel)?,
            proof_height: self.candidate.proof_height,
        })
    }
}
