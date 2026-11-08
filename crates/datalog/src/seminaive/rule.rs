// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned whole-rule phases over the original positive, guard and signed kernels.

use super::{
    Delta, EvalError, GuardEvaluator, HeadEvidence, JoinSnapshot, JoinStrategy, RoundBuffer,
    RoundSnapshot, RuleEntry, SlotSolution, StepGovernor, TermDictionary, TermId, TermIdentity,
    emit_solution, evidence::RuleEvidence, factors, guards, join, ordered,
};
use crate::store::{Arity, StoreShape};

#[derive(Debug)]
struct Filtering<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    inputs: std::vec::IntoIter<SlotSolution<I, N>>,
    kept: Vec<SlotSolution<I, N>>,
    active: Option<(SlotSolution<I, N>, factors::filters::Frame<I, N>)>,
    atoms: Vec<usize>,
    groups: Vec<usize>,
}

#[derive(Debug)]
struct Emitting<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    inputs: std::vec::IntoIter<SlotSolution<I, N>>,
    active: Option<SlotSolution<I, N>>,
    head: usize,
}

impl<I: TermIdentity, const N: usize> Emitting<I, N>
where
    Arity<N>: StoreShape<N>,
{
    fn new(inputs: Vec<SlotSolution<I, N>>) -> Self {
        Self {
            inputs: inputs.into_iter(),
            active: None,
            head: 0,
        }
    }
}

#[derive(Debug)]
enum Phase<'r, I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Factor(factors::Frame<'r, I, N>),
    Positive(join::Frame<I, N>),
    Ordered(ordered::Frame<I, N>),
    Guards(guards::Body<I, N>),
    Filtering(Filtering<I, N>),
    Emitting(Emitting<I, N>),
    Complete,
}

/// Each original phase completes before the next starts; no body prefix is replayed.
#[derive(Debug)]
pub(super) struct Frame<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    phase: Phase<'r, I, N>,
    buffer: RoundBuffer<'r, I, N>,
}

impl<'r, I: TermIdentity, const N: usize> Frame<'r, I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Consume frozen round advice once, before any task warms a native access path.
    pub(super) fn new<D: TermDictionary<Id = I>>(
        entry: RuleEntry<'r, N>,
        snapshot: RoundSnapshot<'_, D, N>,
        strategy: JoinStrategy,
        full_counts: &[Option<usize>],
    ) -> Result<Self, EvalError<D::Error>> {
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let join = JoinSnapshot {
            seed: snapshot.probe.map(|probe| &probe.seed),
            rel: snapshot.rel,
            delta: entry.delta,
            indexes: snapshot
                .delta_indexes
                .expect("the round retains delta indexes"),
        };
        let phase = if entry.delta != Delta::all(snapshot.rel.row_slots())
            && entry.rule.guard_positions().is_none()
            && entry.plan.operators().len() > 1
            && !entry.rule.reads_base()
            && full_counts.contains(&Some(0))
        {
            Phase::Complete
        } else if entry.rule.guard_positions().is_none()
            && entry.runtime.factors.len() > 1
            && matches!(strategy, JoinStrategy::Planned)
        {
            Phase::Factor(factors::Frame::new())
        } else if entry.rule.guard_positions().is_some() {
            Phase::Ordered(ordered::Frame::new(entry, join, strategy)?)
        } else {
            Phase::Positive(
                join::Frame::new(entry.plan, join, strategy, Some(full_counts))
                    .map_err(EvalError::Source)?,
            )
        };
        Ok(Self {
            phase,
            buffer: RoundBuffer::new(),
        })
    }

    pub(super) fn is_complete(&self) -> bool {
        matches!(self.phase, Phase::Complete)
    }

    /// A refused caller can retain this frame; only its enclosing round publishes.
    pub(super) fn into_buffer(self) -> RoundBuffer<'r, I, N> {
        match self.phase {
            Phase::Factor(frame) => frame.into_buffer(),
            _ => self.buffer,
        }
    }

    fn after_body(&mut self, entry: RuleEntry<'r, N>, inputs: Vec<SlotSolution<I, N>>) {
        self.phase = if entry.runtime.negated.is_empty() && entry.runtime.negations.is_empty() {
            Phase::Emitting(Emitting::new(inputs))
        } else {
            Phase::Filtering(Filtering {
                kept: Vec::with_capacity(inputs.len()),
                inputs: inputs.into_iter(),
                active: None,
                atoms: (0..entry.runtime.negated.len()).collect(),
                groups: (0..entry.runtime.negations.len()).collect(),
            })
        };
    }

    /// Resume the same filter probe or head ordinal before considering later input.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        entry: RuleEntry<'r, N>,
        snapshot: RoundSnapshot<'_, D, N>,
        guards: &dyn GuardEvaluator<D, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let join = JoinSnapshot {
            seed: snapshot.probe.map(|probe| &probe.seed),
            rel: snapshot.rel,
            delta: entry.delta,
            indexes: snapshot
                .delta_indexes
                .expect("the round retains delta indexes"),
        };
        while !governor.spent() {
            match &mut self.phase {
                Phase::Complete => break,
                Phase::Factor(frame) => {
                    frame.advance(entry, snapshot, governor, evidence)?;
                    if !frame.is_complete() {
                        break;
                    }
                    let Phase::Factor(frame) = std::mem::replace(&mut self.phase, Phase::Complete)
                    else {
                        unreachable!("the original factor frame completed")
                    };
                    self.buffer = frame.into_buffer();
                }
                Phase::Positive(frame) => {
                    frame
                        .advance(entry.plan, join, governor)
                        .map_err(EvalError::Source)?;
                    if !frame.is_complete() {
                        break;
                    }
                    let Phase::Positive(frame) =
                        std::mem::replace(&mut self.phase, Phase::Complete)
                    else {
                        unreachable!("the original positive frame completed")
                    };
                    self.phase = Phase::Guards(guards::Body::new(
                        0..entry.runtime.guards.len(),
                        frame.into_output(),
                    ));
                }
                Phase::Ordered(frame) => {
                    frame.advance(entry, join, guards, governor)?;
                    if !frame.is_complete() {
                        break;
                    }
                    let Phase::Ordered(frame) = std::mem::replace(&mut self.phase, Phase::Complete)
                    else {
                        unreachable!("the original ordered body completed")
                    };
                    self.after_body(entry, frame.into_output());
                }
                Phase::Guards(frame) => {
                    frame.advance(entry, snapshot.rel, guards, governor)?;
                    if !frame.is_complete() {
                        break;
                    }
                    let Phase::Guards(frame) = std::mem::replace(&mut self.phase, Phase::Complete)
                    else {
                        unreachable!("the original guard batch completed")
                    };
                    self.after_body(entry, frame.into_output());
                }
                Phase::Filtering(state) => {
                    if state.active.is_none() {
                        let Some(input) = state.inputs.next() else {
                            let kept = std::mem::take(&mut state.kept);
                            self.phase = Phase::Emitting(Emitting::new(kept));
                            continue;
                        };
                        state.active =
                            Some((input, factors::filters::Frame::new(evidence.is_some())));
                    }
                    let (input, frame) = state
                        .active
                        .as_mut()
                        .expect("the original input is retained");
                    frame.advance(
                        &factors::filters::Selection {
                            atoms: &state.atoms,
                            groups: &state.groups,
                            entry,
                            guards,
                        },
                        snapshot.rel,
                        input,
                        governor,
                        evidence,
                    )?;
                    if !frame.is_complete() {
                        break;
                    }
                    let (input, frame) =
                        state.active.take().expect("the original filter completed");
                    if !frame.blocked() {
                        state.kept.push(input);
                    }
                }
                Phase::Emitting(state) => {
                    if state.active.is_none() {
                        state.active = state.inputs.next();
                        state.head = 0;
                    }
                    let Some(input) = &state.active else {
                        self.phase = Phase::Complete;
                        continue;
                    };
                    let Some(head) = entry.runtime.head.get(state.head) else {
                        state.active = None;
                        continue;
                    };
                    if snapshot
                        .probe
                        .and_then(|probe| probe.target)
                        .is_some_and(|(head, _)| head != state.head)
                    {
                        state.head += 1;
                        continue;
                    }
                    // The original terminal body row prepays only its first head.
                    if state.head != 0 && !governor.charge() {
                        break;
                    }
                    emit_solution(
                        &mut self.buffer,
                        head,
                        input,
                        entry.index,
                        snapshot,
                        evidence,
                        HeadEvidence {
                            head: state.head,
                            witness: None,
                            uses_absence: !entry.runtime.negated.is_empty()
                                || !entry.runtime.negations.is_empty(),
                        },
                    )?;
                    state.head += 1;
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
