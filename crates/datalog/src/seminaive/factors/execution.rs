// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-factor execution: preconditions, mode discovery and authored head products.

use super::{FactorRelations, Product, Projection, filters, product, project, relations};
use crate::dictionary::{TermDictionary, TermIdentity};
use crate::id::TermId;
use crate::seminaive::{
    ArgShape, BTreeSet, Delta, EvalError, JoinSnapshot, RoundBuffer, RoundSnapshot, RuleEntry,
    StepGovernor, delta_can_match, evidence::RuleEvidence,
};
use crate::store::{Arity, StoreShape};
use purrdf_core::evidence::EvidenceId;

#[derive(Debug)]
struct Products<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    relations: FactorRelations<I, N>,
    head: usize,
    projections: Option<Vec<Projection>>,
    anchor: usize,
    active: Option<product::Frame>,
}

#[derive(Debug)]
enum Phase<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Eligibility,
    Ground(Box<filters::Ground<I, N>>),
    Relations {
        frame: Box<relations::Frame<I, N>>,
        ground: Option<EvidenceId>,
    },
    Products(Products<I, N>),
    Complete,
}

/// The original factor rule and every pending winner, with no relation self-borrow.
/// The enclosing execution retains the unchanged program and evidence workspace.
#[derive(Debug)]
pub(in crate::seminaive) struct Frame<'r, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    phase: Phase<I, N>,
    buffer: RoundBuffer<'r, I, N>,
}

impl<'r, I: TermIdentity, const N: usize> Frame<'r, I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(in crate::seminaive) fn new() -> Self {
        Self {
            phase: Phase::Eligibility,
            buffer: RoundBuffer::new(),
        }
    }

    pub(in crate::seminaive) fn is_complete(&self) -> bool {
        matches!(self.phase, Phase::Complete)
    }

    /// Pending output stays internal; the round owner decides publication and refusal.
    pub(in crate::seminaive) fn into_buffer(self) -> RoundBuffer<'r, I, N> {
        self.buffer
    }

    /// Advance the original phase, head and anchor before considering any later work.
    pub(in crate::seminaive) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
        entry: RuleEntry<'r, N>,
        snapshot: RoundSnapshot<'_, D, N>,
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
                .expect("the original round retains its delta indexes"),
        };
        while !governor.spent() {
            match &mut self.phase {
                Phase::Complete => break,
                Phase::Eligibility => {
                    let mut can_match = false;
                    for operator in entry.plan.operators() {
                        if delta_can_match(operator, join).map_err(EvalError::Source)? {
                            can_match = true;
                            break;
                        }
                    }
                    self.phase = if can_match {
                        Phase::Ground(Box::new(filters::Ground::new(
                            entry,
                            evidence.is_some(),
                            snapshot.probe.map(|probe| &probe.seed),
                        )))
                    } else {
                        Phase::Complete
                    };
                }
                Phase::Ground(frame) => {
                    frame.advance(entry, snapshot.rel, governor, evidence)?;
                    let Some((blocked, ground)) = frame.outcome() else {
                        break;
                    };
                    self.phase = if blocked {
                        Phase::Complete
                    } else {
                        Phase::Relations {
                            frame: Box::new(relations::Frame::new(
                                entry.runtime.factors.len(),
                                entry.delta == Delta::all(snapshot.rel.row_slots()),
                            )),
                            ground,
                        }
                    };
                }
                Phase::Relations { frame, .. } => {
                    frame.advance(entry, join, governor, evidence)?;
                    if !frame.is_complete() {
                        break;
                    }
                    let Phase::Relations { frame, ground } =
                        std::mem::replace(&mut self.phase, Phase::Complete)
                    else {
                        unreachable!("the original mode discovery just completed")
                    };
                    let mut relations = (*frame).into_output();
                    if relations.is_empty() {
                        continue;
                    }
                    // Carry the round-wide signed premise once, through the same first
                    // factor witnesses used by the original product evidence law.
                    if let (Some(id), Some(owner)) = (ground, evidence.as_deref_mut()) {
                        for solution in &mut relations[0] {
                            solution.retain_negative(id, owner)?;
                        }
                    }
                    self.phase = Phase::Products(Products {
                        relations,
                        head: 0,
                        projections: None,
                        anchor: 0,
                        active: None,
                    });
                }
                Phase::Products(state) => {
                    let Some(head) = entry.runtime.head.get(state.head) else {
                        self.phase = Phase::Complete;
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
                    if state.projections.is_none() {
                        state.projections = Some(
                            entry
                                .runtime
                                .factors
                                .iter()
                                .zip(&state.relations)
                                .map(|(factor, matches)| {
                                    let slots: BTreeSet<_> = head
                                        .iter()
                                        .filter_map(|arg| match arg {
                                            ArgShape::Slot(slot) if factor.slots.contains(slot) => {
                                                Some(*slot)
                                            }
                                            ArgShape::Slot(_) | ArgShape::Const(_) => None,
                                        })
                                        .collect();
                                    project(
                                        matches,
                                        &slots.into_iter().collect::<Vec<_>>(),
                                        snapshot,
                                        entry.delta,
                                        evidence.is_some(),
                                    )
                                })
                                .collect::<Result<_, _>>()
                                .map_err(EvalError::Source)?,
                        );
                    }
                    let projections = state
                        .projections
                        .as_ref()
                        .expect("the authored head projection is retained");
                    if state.anchor == projections.len() {
                        state.projections = None;
                        state.head += 1;
                        state.anchor = 0;
                        continue;
                    }
                    let product = Product::new(
                        entry,
                        snapshot,
                        &state.relations,
                        projections,
                        state.head,
                        state.anchor,
                    );
                    let active = state
                        .active
                        .get_or_insert_with(|| product::Frame::new(&product.tables));
                    active.advance(&product, &mut self.buffer, governor, evidence)?;
                    if !active.is_complete() {
                        break;
                    }
                    state.active = None;
                    state.anchor += 1;
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
