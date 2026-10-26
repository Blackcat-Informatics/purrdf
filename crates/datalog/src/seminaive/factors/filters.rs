// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One ordered negative-filter owner for whole rules, factor matches and ground probes.

use crate::dictionary::{TermDictionary, TermIdentity};
use crate::seminaive::{
    EvalError, GroupProbe, GuardEvaluator, NoGuards, RelationStore, RuleEntry, SlotSolution,
    StepGovernor, evidence::RuleEvidence, negation, negative,
};
use crate::store::{Arity, StoreShape};
use purrdf_core::evidence::EvidenceId;

#[derive(Debug)]
enum Probe<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Atom(negative::Atom<I, N>),
    Group(negation::Frame<I, N>),
}

/// Authored filter indices belong to the unchanged enclosing rule and factor.
pub(in crate::seminaive) struct Selection<'a, D: TermDictionary, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    pub(in crate::seminaive) atoms: &'a [usize],
    pub(in crate::seminaive) groups: &'a [usize],
    pub(in crate::seminaive) entry: RuleEntry<'a, N>,
    pub(in crate::seminaive) guards: &'a dyn GuardEvaluator<D, N>,
}

/// Original atom/group positions and a pending signed or boolean witness search.
#[derive(Debug)]
pub(in crate::seminaive) struct Frame<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    atom: usize,
    group: usize,
    pending: Option<Probe<I, N>>,
    outcome: Option<bool>,
    evidenced: bool,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(in crate::seminaive) fn new(evidenced: bool) -> Self {
        Self {
            atom: 0,
            group: 0,
            pending: None,
            outcome: None,
            evidenced,
        }
    }

    pub(in crate::seminaive) fn is_complete(&self) -> bool {
        self.outcome.is_some()
    }

    pub(in crate::seminaive) fn blocked(&self) -> bool {
        self.outcome.unwrap_or(false)
    }

    /// Finish the original probe before attaching its evidence or advancing a filter.
    pub(in crate::seminaive) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        selection: &Selection<'_, D, N>,
        rel: &RelationStore<D, N>,
        solution: &mut SlotSolution<I, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        rel.interner().check_read().map_err(EvalError::Source)?;
        while !self.is_complete() && !governor.spent() {
            if let Some(&index) = selection.atoms.get(self.atom) {
                let atom = &selection.entry.runtime.negated[index];
                let pending = self.pending.get_or_insert_with(|| {
                    Probe::Atom(negative::Atom::new(atom.source, self.evidenced))
                });
                let Probe::Atom(frame) = pending else {
                    unreachable!("the original atom probe remains active")
                };
                frame.advance(atom, solution, rel, governor, evidence)?;
                if !frame.is_complete() {
                    break;
                }
                if frame.blocked() {
                    self.outcome = Some(true);
                    break;
                }
                self.atom += 1;
                self.pending = None;
            } else if let Some(&index) = selection.groups.get(self.group) {
                let pending = self
                    .pending
                    .get_or_insert_with(|| Probe::Group(negation::Frame::new(self.evidenced)));
                let Probe::Group(frame) = pending else {
                    unreachable!("the original conjunction probe remains active")
                };
                frame.advance(
                    &selection.entry.runtime.negations[index],
                    GroupProbe {
                        solution,
                        rel,
                        guards: selection.guards,
                        rule: selection.entry.index,
                        clause: selection.entry.rule,
                    },
                    governor,
                    evidence,
                )?;
                if !frame.is_complete() {
                    break;
                }
                let (blocked, negative) = frame.outcome();
                if let (Some(id), Some(owner)) = (negative, evidence.as_deref_mut()) {
                    solution.retain_negative(id, owner)?;
                }
                if blocked {
                    self.outcome = Some(true);
                    break;
                }
                self.group += 1;
                self.pending = None;
            } else {
                self.outcome = Some(false);
            }
        }
        rel.interner().check_read().map_err(EvalError::Source)
    }
}

/// The round-wide empty substitution and only filters with no positive outer slot.
#[derive(Debug)]
pub(super) struct Ground<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    atoms: Vec<usize>,
    groups: Vec<usize>,
    solution: SlotSolution<I, N>,
    filters: Frame<I, N>,
}

impl<I: TermIdentity, const N: usize> Ground<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn new(
        entry: RuleEntry<'_, N>,
        evidenced: bool,
        seed: Option<&SlotSolution<I, N>>,
    ) -> Self {
        Self {
            atoms: (0..entry.runtime.negated.len())
                .filter(|index| {
                    !entry
                        .runtime
                        .factors
                        .iter()
                        .any(|factor| factor.negated.contains(index))
                })
                .collect(),
            groups: (0..entry.runtime.negations.len())
                .filter(|index| {
                    !entry
                        .runtime
                        .factors
                        .iter()
                        .any(|factor| factor.negations.contains(index))
                })
                .collect(),
            solution: seed
                .cloned()
                .unwrap_or_else(|| SlotSolution::empty(entry.plan.variables().len())),
            filters: Frame::new(evidenced),
        }
    }

    /// An unfinished ground search establishes neither blocking nor absence evidence.
    pub(super) fn outcome(&self) -> Option<(bool, Option<EvidenceId>)> {
        self.filters
            .is_complete()
            .then_some((self.filters.blocked(), self.solution.negative_evidence))
    }

    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        entry: RuleEntry<'_, N>,
        rel: &RelationStore<D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        self.filters.advance(
            &Selection {
                atoms: &self.atoms,
                groups: &self.groups,
                entry,
                guards: &NoGuards,
            },
            rel,
            &mut self.solution,
            governor,
            evidence,
        )
    }
}
