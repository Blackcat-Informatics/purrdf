// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::ops::Range;
use std::collections::TryReserveError;

use crate::v2::fields::push;
use crate::v2::references::{Leaf, LeafNode};
use crate::v2::terms::{LITERAL_VALUE_KIND, LITERAL_VALUE_REF_KIND, TRIPLE_KIND};
use crate::v2::{DecodedTerms, TermDefinition, TermReservation};

/// Pre-union size of a segment's folded frames under P3 §6.5.
#[derive(Default)]
pub(super) struct LabellingSize {
    count: u128,
    pub(super) blob_start: usize,
    reserved_end: u128,
    triple_runs: Vec<Range<u128>>,
    forward_candidates: Vec<u64>,
}

impl LabellingSize {
    pub(super) const fn rows_or_entries(&mut self, count: usize) {
        self.count += count as u128;
    }

    pub(super) fn terms(
        &mut self,
        reservation: &TermReservation<'_>,
        decoded: &DecodedTerms<'_>,
    ) -> Result<(), TryReserveError> {
        let mut start = reservation.start;
        for (kind, count) in reservation.runs() {
            let count = u128::from(count.get());
            let end = start + count;
            match kind {
                TRIPLE_KIND => push(&mut self.triple_runs, start..end)?,
                LITERAL_VALUE_KIND | LITERAL_VALUE_REF_KIND => self.count += count,
                _ => {}
            }
            start = end;
        }
        self.reserved_end = reservation.end;
        for definition in &decoded.terms {
            if let TermDefinition::Triple([_, _, object]) = definition {
                self.candidate(*object)?;
            }
        }
        Ok(())
    }

    pub(super) fn leaf(&mut self, leaf: Leaf<'_>) -> Result<(), TryReserveError> {
        // The root itself is not a component of another triple in the leaf.
        for node in leaf.nodes().skip(1) {
            match node {
                LeafNode::Triple => self.count += 1,
                LeafNode::Term(id) => self.candidate(id)?,
                LeafNode::Blank { .. } => {}
            }
        }
        Ok(())
    }

    pub(super) fn size(&self, blob_claims: usize) -> u128 {
        let forward_triples = self
            .forward_candidates
            .iter()
            .filter(|&&id| self.is_triple(id))
            .count();
        self.count + (blob_claims - self.blob_start) as u128 + forward_triples as u128
    }

    fn candidate(&mut self, id: u64) -> Result<(), TryReserveError> {
        if u128::from(id) >= self.reserved_end {
            return push(&mut self.forward_candidates, id);
        }
        self.count += u128::from(self.is_triple(id));
        Ok(())
    }

    fn is_triple(&self, id: u64) -> bool {
        let id = u128::from(id);
        let index = self.triple_runs.partition_point(|run| run.end <= id);
        self.triple_runs
            .get(index)
            .is_some_and(|run| run.contains(&id))
    }
}
