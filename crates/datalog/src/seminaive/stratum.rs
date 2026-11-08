// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One stratum's original delta, round and publication owners across refusal.

use super::{
    BudgetResource, Delta, EvalError, Executable, FixpointState, RoundControl, RoundSnapshot,
    RuleEntry, RuleRuntime, StratumRun, TermDictionary, TermId, TermIdentity, check_budget, commit,
    evidence::RuleEvidence, is_stopped, round,
};
use crate::store::{Arity, StoreShape};
use purrdf_lex::allocation::{Admission, Memory, Resident};

#[derive(Debug)]
enum Phase<'r, I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Ready,
    Evaluating(round::Frame<'r, I, N>),
    Publishing(commit::Frame<'r, I, N>),
    Evidence { had_facts: bool },
    Complete,
}

/// The original program remains borrowed; the mutable store is never self-borrowed.
#[derive(Debug)]
pub(super) struct Frame<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    exe: &'r Executable<N>,
    runtimes: &'r [RuleRuntime<N>],
    stratum: usize,
    delta: Delta,
    first_round: bool,
    round_lo: u64,
    phase: Phase<'r, I, N>,
}

impl<'r, I: TermIdentity, const N: usize> Frame<'r, I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new<D: TermDictionary<Id = I>>(
        exe: &'r Executable<N>,
        runtimes: &'r [RuleRuntime<N>],
        stratum: usize,
        state: &FixpointState<D, N>,
    ) -> Self {
        Self {
            exe,
            runtimes,
            stratum,
            delta: Delta::all(state.rel.row_slots()),
            first_round: true,
            round_lo: state.rel.row_slots(),
            phase: Phase::Ready,
        }
    }

    pub(super) fn is_complete(&self) -> bool {
        matches!(self.phase, Phase::Complete)
    }

    /// Resume only governor refusals; source and evidence failures remain hard errors.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        state: &mut FixpointState<D, N>,
        run: StratumRun<'_, D, N>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        let before = state.native_bytes;
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, before);
        let result = self.advance_in_memory(state, run, evidence, &mut memory);
        state.account_native_memory(before, memory.admitted_bytes());
        result
    }

    pub(super) fn advance_in_memory<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        state: &mut FixpointState<D, N>,
        run: StratumRun<'_, D, N>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), EvalError<D::Error>> {
        check_budget(state)?;
        loop {
            match &mut self.phase {
                Phase::Ready => {
                    if is_stopped(run.stop) {
                        return Err(EvalError::Stopped {
                            report: state.report(),
                        });
                    }
                    let entries: Vec<_> = self
                        .exe
                        .stratum_rule_indices(self.stratum)
                        .iter()
                        .filter_map(|&index| {
                            let (rule, plan) = self.exe.rule_entry(index);
                            (self.first_round || !plan.positive().is_empty()).then_some(RuleEntry {
                                index,
                                rule,
                                plan,
                                runtime: &self.runtimes[index],
                                delta: self.delta,
                            })
                        })
                        .collect();
                    if entries.is_empty() {
                        self.phase = Phase::Complete;
                        continue;
                    }
                    let frame = round::Frame::new(
                        &entries,
                        RoundSnapshot {
                            probe: None,
                            rel: &state.rel,
                            depth: state.depths(),
                            assumed: &purrdf_core::FastSet::default(),
                            delta_indexes: None,
                        },
                    )?;
                    self.round_lo = state.rel.row_slots();
                    self.first_round = false;
                    self.phase = Phase::Evaluating(frame);
                }
                Phase::Evaluating(frame) => {
                    let before = frame.successful_steps();
                    frame.advance(
                        RoundSnapshot {
                            probe: None,
                            rel: &state.rel,
                            depth: state.depths(),
                            assumed: &purrdf_core::FastSet::default(),
                            delta_indexes: None,
                        },
                        run.execution,
                        run.strategy,
                        state.allowance(),
                        RoundControl {
                            guards: run.guards,
                            stop: None,
                        },
                        evidence,
                    )?;
                    state.join_steps = state
                        .join_steps
                        .saturating_add(frame.successful_steps().saturating_sub(before));
                    if frame.refused() {
                        let mut report = state.report();
                        report.join_steps = report.join_steps.saturating_add(1);
                        return Err(EvalError::BudgetExhausted {
                            resource: BudgetResource::JoinSteps,
                            report,
                        });
                    }
                    debug_assert!(frame.is_complete());
                    check_budget(state)?;
                    let Phase::Evaluating(frame) = std::mem::replace(&mut self.phase, Phase::Ready)
                    else {
                        unreachable!("the original round just completed")
                    };
                    let buffer = frame.into_buffer(&state.rel)?;
                    self.phase = if buffer.is_empty() {
                        Phase::Evidence { had_facts: false }
                    } else {
                        Phase::Publishing(commit::Frame::new(buffer, state)?)
                    };
                }
                Phase::Publishing(frame) => {
                    assert!(
                        frame.advance_until(state, memory, |_| true)?,
                        "unlimited publication completes its original winners"
                    );
                    check_budget(state)?;
                    self.phase = Phase::Evidence { had_facts: true };
                }
                Phase::Evidence { had_facts } => {
                    if let Some(owner) = evidence.as_mut()
                        && owner.commit(&state.rel)?
                    {
                        self.delta = Delta::all(state.rel.row_slots());
                        self.phase = Phase::Ready;
                    } else if *had_facts {
                        self.delta = Delta {
                            lo: self.round_lo,
                            hi: state.rel.row_slots(),
                        };
                        self.phase = Phase::Ready;
                    } else {
                        self.phase = Phase::Complete;
                    }
                }
                Phase::Complete => return Ok(()),
            }
        }
    }
}
