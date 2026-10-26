// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An owned odometer over the original projected factor frontiers.

use super::{Frontier, Product, Witnesses};
use crate::dictionary::TermDictionary;
use crate::seminaive::{EvalError, RoundBuffer, StepGovernor, evidence::RuleEvidence};
use crate::store::{Arity, StoreShape};

/// Only coordinates persist; the original relations and projections may move.
#[derive(Debug)]
pub(super) struct Frame {
    positions: Vec<usize>,
    complete: bool,
}

impl Frame {
    pub(super) fn new(tables: &[&[Frontier]]) -> Self {
        Self {
            positions: vec![0; tables.len()],
            complete: tables.is_empty() || tables.iter().any(|table| table.is_empty()),
        }
    }

    pub(super) const fn is_complete(&self) -> bool {
        self.complete
    }

    /// Emit each original combination once, advancing only after admitted output.
    pub(super) fn advance<'r, D: TermDictionary, const N: usize>(
        &mut self,
        product: &Product<'_, 'r, D, N>,
        buffer: &mut RoundBuffer<'r, D::Id, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>>
    where
        Arity<N>: StoreShape<N>,
    {
        product
            .snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)?;
        let mut chosen = Vec::with_capacity(self.positions.len());
        while !self.complete && !governor.spent() {
            chosen.clear();
            chosen.extend(
                self.positions
                    .iter()
                    .enumerate()
                    .map(|(factor, &row)| Witnesses {
                        frontier: &product.tables[factor][row],
                        relation: &product.relations[factor],
                    }),
            );
            if !product.emit(&chosen, buffer, governor, evidence)? {
                break;
            }
            self.complete = true;
            for factor in (0..self.positions.len()).rev() {
                self.positions[factor] += 1;
                if self.positions[factor] < product.tables[factor].len() {
                    self.complete = false;
                    break;
                }
                self.positions[factor] = 0;
            }
        }
        product
            .snapshot
            .rel
            .interner()
            .check_read()
            .map_err(EvalError::Source)
    }
}
