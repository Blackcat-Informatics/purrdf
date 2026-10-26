// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen round advice, original rule owners and ordered winner reduction.

use super::{
    DeltaIndexes, EvalError, GuardEvaluator, JoinStrategy, NoGuards, RelationStore, RoundBuffer,
    RoundControl, RoundExecution, RoundSnapshot, RuleEntry, StepGovernor, TermDictionary, TermId,
    TermIdentity, constant_pattern_counts, evidence::RuleEvidence, rule,
};
use crate::store::{Arity, StoreShape};
use rayon::prelude::*;
use std::sync::atomic::Ordering;

#[derive(Debug)]
struct Rule<'r, I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    entry: RuleEntry<'r, N>,
    counts: Vec<Option<usize>>,
    frame: Option<rule::Frame<'r, I, N>>,
}

impl<I: TermIdentity, const N: usize> Rule<'_, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Enter a rule only at its original scheduling point, including source reads.
    fn advance<'s, D: TermDictionary<Id = I>>(
        &mut self,
        snapshot: RoundSnapshot<'_, D, N>,
        strategy: JoinStrategy,
        guards: &dyn GuardEvaluator<D, N>,
        mut governor: StepGovernor<'s>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<StepGovernor<'s>, EvalError<D::Error>> {
        if self.frame.is_none() {
            self.frame = Some(rule::Frame::new(
                self.entry,
                snapshot,
                strategy,
                &self.counts,
            )?);
        }
        let frame = self.frame.as_mut().expect("the original rule was entered");
        if !frame.is_complete() {
            frame.advance(self.entry, snapshot, guards, &mut governor, evidence)?;
        }
        Ok(governor)
    }
}

/// Retains rule-local work and freezes access-path advice before any task runs.
#[derive(Debug)]
pub(super) struct Frame<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    rules: Vec<Rule<'r, I, N>>,
    indexes: DeltaIndexes<I>,
    merged: usize,
    buffer: RoundBuffer<'r, I, N>,
    steps: u64,
    refused: bool,
    stopped: bool,
    #[cfg(test)]
    row_visits: u64,
}

impl<'r, I: TermIdentity, const N: usize> Frame<'r, I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new<D: TermDictionary<Id = I>>(
        entries: &[RuleEntry<'r, N>],
        snapshot: RoundSnapshot<'_, D, N>,
    ) -> Result<Self, EvalError<D::Error>> {
        let rules = entries
            .iter()
            .map(|&entry| {
                Ok(Rule {
                    entry,
                    counts: constant_pattern_counts(entry.plan, snapshot.rel, entry.delta)?,
                    frame: None,
                })
            })
            .collect::<Result<Vec<_>, D::Error>>()
            .map_err(EvalError::Source)?;
        let indexes = DeltaIndexes::build(snapshot.rel, entries.iter().map(|entry| entry.delta))?;
        Ok(Self {
            rules,
            indexes,
            merged: 0,
            buffer: RoundBuffer::new(),
            steps: 0,
            refused: false,
            stopped: false,
            #[cfg(test)]
            row_visits: 0,
        })
    }

    pub(super) fn is_complete(&self) -> bool {
        self.merged == self.rules.len()
    }

    pub(super) fn refused(&self) -> bool {
        self.refused
    }

    pub(super) fn stopped(&self) -> bool {
        self.stopped
    }

    /// Successful cumulative credits; a page's refusal sentinel is not work.
    pub(super) fn successful_steps(&self) -> u64 {
        self.steps
    }

    #[cfg(test)]
    pub(super) fn row_visits(&self) -> u64 {
        self.row_visits
    }

    /// Resume original frames; a new page supplies credits, never new rule advice.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        snapshot: RoundSnapshot<'_, D, N>,
        execution: RoundExecution,
        strategy: JoinStrategy,
        allowance: u64,
        control: RoundControl<'_, D, N>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let snapshot = RoundSnapshot {
            delta_indexes: Some(&self.indexes),
            ..snapshot
        };
        let pool = StepGovernor::new(allowance).with_stop(control.stop);
        let guarded = self.rules.iter().any(|rule| rule.entry.rule.is_guarded());
        if evidence.is_some() || guarded || !execution.should_parallelize(self.rules.len()) {
            while self.merged < self.rules.len() {
                let position = self.merged;
                let task = self.rules[position].advance(
                    snapshot,
                    strategy,
                    control.guards,
                    pool.task(),
                    evidence,
                )?;
                #[cfg(test)]
                {
                    self.row_visits = self.row_visits.saturating_add(task.row_visits);
                }
                #[cfg(not(test))]
                drop(task);
                Self::merge_ready(
                    &mut self.rules,
                    &mut self.merged,
                    &mut self.buffer,
                    snapshot.rel,
                )?;
                if pool.spent() {
                    break;
                }
            }
        } else {
            // Indexed collection keeps the original first error and program order.
            let tasks: Vec<_> = self.rules[self.merged..]
                .par_iter_mut()
                .map(|rule| rule.advance(snapshot, strategy, &NoGuards, pool.task(), &mut None))
                .collect();
            for task in tasks {
                let task = task?;
                #[cfg(test)]
                {
                    self.row_visits = self.row_visits.saturating_add(task.row_visits);
                }
                #[cfg(not(test))]
                drop(task);
            }
            Self::merge_ready(
                &mut self.rules,
                &mut self.merged,
                &mut self.buffer,
                snapshot.rel,
            )?;
        }
        self.steps = self
            .steps
            .saturating_add(pool.pool.used.load(Ordering::Relaxed));
        self.refused = pool.refused();
        self.stopped = pool.stopped();
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)
    }

    /// Only a completed prefix is reduced before entering later serial callbacks.
    fn merge_ready<D: TermDictionary<Id = I>>(
        rules: &mut [Rule<'r, I, N>],
        merged: &mut usize,
        buffer: &mut RoundBuffer<'r, I, N>,
        rel: &RelationStore<D, N>,
    ) -> Result<(), EvalError<D::Error>> {
        while let Some(rule) = rules.get_mut(*merged) {
            if !rule.frame.as_ref().is_some_and(rule::Frame::is_complete) {
                break;
            }
            let frame = rule.frame.take().expect("the original rule completed");
            buffer
                .merge_from(frame.into_buffer(), rel)
                .map_err(EvalError::Source)?;
            *merged += 1;
        }
        Ok(())
    }

    /// Ordinary hard-refusal callers consume pending buffers in the original order.
    pub(super) fn into_buffer<D: TermDictionary<Id = I>>(
        mut self,
        rel: &RelationStore<D, N>,
    ) -> Result<RoundBuffer<'r, I, N>, EvalError<D::Error>> {
        for rule in &mut self.rules[self.merged..] {
            if let Some(frame) = rule.frame.take() {
                self.buffer
                    .merge_from(frame.into_buffer(), rel)
                    .map_err(EvalError::Source)?;
            }
        }
        self.buffer.join_steps = self.steps.saturating_add(u64::from(self.refused));
        self.buffer.join_refused = self.refused;
        self.buffer.stopped = self.stopped;
        rel.interner().check_read().map_err(EvalError::Source)?;
        Ok(self.buffer)
    }
}
