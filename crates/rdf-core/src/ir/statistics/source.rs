// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact physical source accounting, constructed from validated original rows.

use purrdf_hash::multiset::Multiset256;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::{FrozenTerms, QuadIds, Relation, TermId, TermRef};
use crate::source::{SourceRowHasher, SourceSummary, record_hash};

impl SourceSummary {
    pub(super) fn words(self) -> [u64; 13] {
        let mut words = [0; 13];
        words[0] = u64::try_from(self.literals[0].population())
            .expect("native literal population is bounded by u64 statement count");
        for (offset, sum) in [
            self.literals[0].sum(),
            self.literals[1].sum(),
            self.records.sum(),
        ]
        .into_iter()
        .enumerate()
        {
            for (index, bytes) in sum.as_chunks::<8>().0.iter().enumerate() {
                words[1 + offset * 4 + index] = u64::from_le_bytes(*bytes);
            }
        }
        words
    }

    pub(super) fn decode(
        words: &[u64],
        rows: u64,
    ) -> Result<Self, super::super::pack::bits::PackBitsError> {
        use super::super::pack::bits::PackBitsError::Malformed;
        if words.len() != 13 || words[0] > rows {
            return Err(Malformed("invalid source summary population"));
        }
        let mut sums = [[0; 32]; 3];
        for (sum, words) in sums.iter_mut().zip(words[1..].as_chunks::<4>().0) {
            for (bytes, word) in sum.as_chunks_mut::<8>().0.iter_mut().zip(words) {
                *bytes = word.to_le_bytes();
            }
        }
        if words[0] == 0 && (sums[0] != [0; 32] || sums[1] != [0; 32]) {
            return Err(Malformed("nonzero empty literal source summary"));
        }
        Ok(Self {
            literals: [
                Multiset256::from_parts(sums[0], u128::from(words[0])),
                Multiset256::from_parts(sums[1], u128::from(words[0])),
            ],
            records: Multiset256::from_parts(sums[2], u128::from(rows)),
        })
    }

    /// The record kind participates in whole-source identity. Literal source
    /// projections deliberately preserve occurrences across physical layers.
    pub(super) fn add_row<S: Admission + ?Sized>(
        &mut self,
        relation: Relation,
        quad: QuadIds,
        dictionary: &FrozenTerms,
        pending: &mut Vec<TermId>,
        memory: &mut Memory<'_, S>,
    ) -> Result<([u8; 32], [u8; 32]), StorageError> {
        let mut row = SourceRowHasher::new(quad.g.is_some());
        if let Some(graph) = quad.g {
            dictionary.visit_canonical_bytes(graph, pending, memory, |bytes| row.graph(bytes))?;
        }
        for term in [quad.s, quad.p, quad.o] {
            dictionary.visit_canonical_bytes(term, pending, memory, |bytes| row.term(bytes))?;
        }
        let hashes = row.finish();
        if matches!(dictionary.resolve(quad.o), TermRef::Literal { .. }) {
            for (summary, hash) in self.literals.iter_mut().zip(hashes) {
                summary
                    .insert(hash)
                    .map_err(|_| StorageError::SizeOverflow)?;
            }
        }
        let identity = record_hash(relation, hashes[0]);
        let union = if relation == Relation::Annotations {
            record_hash(Relation::Asserted, hashes[0])
        } else {
            identity
        };
        self.records
            .insert(identity)
            .map_err(|_| StorageError::SizeOverflow)?;
        Ok((identity, union))
    }

    pub(super) fn combine(&mut self, other: Self) -> Result<(), StorageError> {
        let mut combined = *self;
        for (summary, other) in combined.literals.iter_mut().zip(other.literals) {
            summary
                .combine(other)
                .map_err(|_| StorageError::SizeOverflow)?;
        }
        combined
            .records
            .combine(other.records)
            .map_err(|_| StorageError::SizeOverflow)?;
        *self = combined;
        Ok(())
    }
}
