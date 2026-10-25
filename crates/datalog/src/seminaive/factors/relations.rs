// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned Full/New discovery and productive-anchor Old/Full context scheduling.

use super::{FactorRelations, matches};
use crate::dictionary::{TermDictionary, TermIdentity};
use crate::id::TermId;
use crate::seminaive::{
    EvalError, JoinScan, JoinSnapshot, RuleEntry, Scan, StepGovernor, delta_can_match,
    evidence::RuleEvidence,
};
use crate::store::{Arity, StoreShape};

#[derive(Debug)]
enum Phase {
    Full {
        factor: usize,
    },
    New {
        factor: usize,
        anchor: usize,
    },
    Context {
        factor: usize,
        productive: Vec<bool>,
    },
    Complete,
}

/// The original disjoint delta anchors and only the context productive anchors need.
#[derive(Debug)]
pub(super) struct Frame<I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    phase: Phase,
    active: Option<matches::Frame<I, N>>,
    relations: FactorRelations<I, N>,
}

impl<I: TermIdentity, const N: usize> Frame<I, N>
where
    Arity<N>: StoreShape<N>,
{
    /// Begin the original all-row seed or productive-delta discovery law.
    pub(super) fn new(factors: usize, full: bool) -> Self {
        Self {
            phase: if full {
                Phase::Full { factor: 0 }
            } else {
                Phase::New {
                    factor: 0,
                    anchor: 0,
                }
            },
            active: None,
            relations: Vec::with_capacity(factors),
        }
    }

    pub(super) fn is_complete(&self) -> bool {
        matches!(self.phase, Phase::Complete)
    }

    /// Incomplete mode relations remain internal, never an apparently complete body.
    pub(super) fn into_output(self) -> FactorRelations<I, N> {
        if self.is_complete() {
            self.relations
        } else {
            Vec::new()
        }
    }

    /// Resume the active native match before probing another factor or anchor.
    pub(super) fn advance<D: TermDictionary<Id = I>>(
        &mut self,
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
        while !governor.spent() {
            let factors = &entry.runtime.factors;
            let (position, scan, replace) = match &mut self.phase {
                Phase::Complete => break,
                Phase::Full { factor } => {
                    if *factor == factors.len() {
                        self.phase = Phase::Complete;
                        continue;
                    }
                    (*factor, JoinScan::Uniform(Scan::Full), false)
                }
                Phase::New { factor, anchor } => {
                    if *factor == factors.len() {
                        let productive: Vec<_> =
                            self.relations.iter().map(|rows| !rows.is_empty()).collect();
                        if productive.iter().any(|&active| active) {
                            self.phase = Phase::Context {
                                factor: 0,
                                productive,
                            };
                        } else {
                            self.relations.clear();
                            self.phase = Phase::Complete;
                        }
                        continue;
                    }
                    if self.relations.len() == *factor {
                        self.relations.push(Vec::new());
                    }
                    let Some(&position) = factors[*factor].positions.get(*anchor) else {
                        *factor += 1;
                        *anchor = 0;
                        continue;
                    };
                    if self.active.is_none()
                        && !delta_can_match(entry.plan.operator_at(position), snapshot)
                            .map_err(EvalError::Source)?
                    {
                        *anchor += 1;
                        continue;
                    }
                    (*factor, JoinScan::DeltaPosition(position), false)
                }
                Phase::Context { factor, productive } => {
                    if *factor == factors.len() {
                        self.phase = Phase::Complete;
                        continue;
                    }
                    if productive[..*factor].iter().any(|&active| active) {
                        (*factor, JoinScan::Uniform(Scan::Full), true)
                    } else if productive[*factor + 1..].iter().any(|&active| active) {
                        (*factor, JoinScan::Uniform(Scan::OldOnly), false)
                    } else {
                        if self.relations[*factor].is_empty() {
                            self.relations.clear();
                            self.phase = Phase::Complete;
                        } else {
                            *factor += 1;
                        }
                        continue;
                    }
                }
            };
            if self.active.is_none() {
                self.active = Some(
                    matches::Frame::new(&factors[position], entry, snapshot, scan)
                        .map_err(EvalError::Source)?,
                );
            }
            let active = self
                .active
                .as_mut()
                .expect("the original mode match is retained");
            active.advance(&factors[position], entry, snapshot, governor, evidence)?;
            if !active.is_complete() {
                break;
            }
            let rows = self
                .active
                .take()
                .expect("the mode match completed")
                .into_output();
            match &mut self.phase {
                Phase::Full { factor } => {
                    if rows.is_empty() {
                        self.relations.clear();
                        self.phase = Phase::Complete;
                    } else {
                        self.relations.push(rows);
                        *factor += 1;
                    }
                }
                Phase::New { factor, anchor } => {
                    self.relations[*factor].extend(rows);
                    *anchor += 1;
                }
                Phase::Context { factor, .. } => {
                    if replace {
                        self.relations[*factor] = rows;
                    } else {
                        self.relations[*factor].extend(rows);
                    }
                    if self.relations[*factor].is_empty() {
                        self.relations.clear();
                        self.phase = Phase::Complete;
                    } else {
                        *factor += 1;
                    }
                }
                Phase::Complete => unreachable!("an original mode match is active"),
            }
        }
        snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)
    }
}
