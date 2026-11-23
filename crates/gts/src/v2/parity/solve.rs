// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use std::collections::TryReserveError;

use purrdf_hash::blake3::Hasher;

use super::field::{add_data_shard, inverse, matrix_coefficient, multiply};
use super::{
    DATA_COLUMNS, DATA_MASKS, MASK_WIDTH, MAX_SHARD_BYTES, PARITY_ROWS, ParityMismatches,
    ParitySpanLength, ParityTable, SPAN_BYTES, matches_shard, shard_hasher,
};

const MAX_DEPTH: usize = SPAN_BYTES / MAX_SHARD_BYTES / DATA_COLUMNS;

/// Replacement bytes for one data or parity shard.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParityPatch {
    /// Index within the corresponding digest list.
    pub index: usize,
    /// Replacement bytes. A short data shard excludes its zero padding.
    pub bytes: Vec<u8>,
}

/// Candidate patches from solving a span. The file's affected items still
/// require a framing and verification walk with the other candidate spans.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParitySolution {
    /// Replacement bytes for explained failing data shards.
    pub data: Vec<ParityPatch>,
    /// Replacement bytes for failing parity shards.
    pub parity: Vec<ParityPatch>,
    /// Every solved shard agrees with its digest, including unexplained data.
    pub solved_digests_match: bool,
}

/// A logical span could not be solved.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParitySolveError {
    /// The supplied bytes differ in length from the span.
    Span(ParitySpanLength),
    /// A stripe has more unknown shards than the code can reconstruct.
    TooManyErasures {
        /// The zero-based stripe number within the span.
        stripe: usize,
        /// The number of failing data and parity shards.
        count: usize,
    },
    /// Allocation failed while solving the span.
    Resource(TryReserveError),
}

impl fmt::Display for ParitySolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Span(error) => error.fmt(f),
            Self::TooManyErasures { stripe, count } => {
                write!(f, "GTS parity stripe {stripe} has {count} erasures")
            }
            Self::Resource(error) => write!(f, "could not allocate for GTS parity ({error})"),
        }
    }
}

impl std::error::Error for ParitySolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Span(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::TooManyErasures { .. } => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ParitySolveError {
    purrdf_hash::error_visit! {
        Span(cause) => "gts-v2-parity-solve-span" { "cause" => Cause(cause) },
        TooManyErasures { stripe, count } => "gts-v2-parity-erasures" {
            "stripe" => Unsigned(*stripe as u128), "count" => Unsigned(*count as u128)
        },
        Resource(cause) => "gts-v2-parity-solve-resource" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(ParitySolveError { Span(ParitySpanLength), Resource(TryReserveError) });

impl ParityTable<'_> {
    /// Solve the failing shards of a placed span.
    ///
    /// Supply its original logical bytes. For each failing data shard, return
    /// whether it overlaps an item that did not pass verification. A span
    /// with no repairs to attempt or with only unexplained data mismatches
    /// returns `None`. The result contains candidate bytes even when a
    /// solved digest disagrees, for the subsequent shared recheck.
    ///
    /// # Errors
    /// Returns a length error, a stripe erasure limit or a resource failure.
    pub fn solve_shards(
        &self,
        span: &[u8],
        mut explained: impl FnMut(usize) -> bool,
    ) -> Result<Option<ParitySolution>, ParitySolveError> {
        let mismatch = self.compare_shards(span)?;
        let mut writable = [0_u64; DATA_MASKS];
        for shard in mismatch.data_shards().filter(|&shard| explained(shard)) {
            writable[shard / MASK_WIDTH] |= 1 << (shard % MASK_WIDTH);
        }
        if writable == [0; DATA_MASKS] && (mismatch.data != [0; DATA_MASKS] || mismatch.parity == 0)
        {
            return Ok(None);
        }
        let depth = self.geometry.parity_shards / PARITY_ROWS;
        let mut erasures = [0; MAX_DEPTH];
        for shard in mismatch.data_shards() {
            erasures[shard % depth] += 1;
        }
        for shard in mismatch.parity_shards() {
            erasures[shard / PARITY_ROWS] += 1;
        }
        if let Some((stripe, &count)) = erasures
            .iter()
            .enumerate()
            .find(|(_, count)| **count > PARITY_ROWS)
        {
            return Err(ParitySolveError::TooManyErasures { stripe, count });
        }
        let mut solution = ParitySolution {
            data: Vec::new(),
            parity: Vec::new(),
            solved_digests_match: true,
        };
        solution
            .data
            .try_reserve_exact(writable.iter().map(|word| word.count_ones() as usize).sum())?;
        solution
            .parity
            .try_reserve_exact(mismatch.parity.count_ones() as usize)?;
        let initial = shard_hasher();
        for (stripe, &count) in erasures[..depth].iter().enumerate() {
            if count != 0 {
                self.solve_stripe(
                    span,
                    stripe,
                    depth,
                    &mismatch,
                    &writable,
                    &initial,
                    &mut solution,
                )?;
            }
        }
        solution.data.sort_unstable_by_key(|patch| patch.index);
        Ok(Some(solution))
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "pass the validated span and existing masks without allocating a second stripe description"
    )]
    fn solve_stripe(
        &self,
        span: &[u8],
        stripe: usize,
        depth: usize,
        mismatch: &ParityMismatches,
        writable: &[u64; DATA_MASKS],
        initial: &Hasher,
        solution: &mut ParitySolution,
    ) -> Result<(), TryReserveError> {
        let size = self.geometry.shard_bytes;
        let index = |column: usize| stripe + column * depth;
        let columns = (self.geometry.data_shards - stripe).div_ceil(depth);
        let data = |column| {
            let begin = index(column) * size;
            &span[begin..begin + (span.len() - begin).min(size)]
        };
        let parity_base = stripe * PARITY_ROWS;
        let missing_parity = |row: usize| mismatch.parity & (1_u64 << (parity_base + row)) != 0;
        let mut unknown = [usize::MAX; PARITY_ROWS];
        let mut count = 0;
        for column in 0..columns {
            if contains(&mismatch.data, index(column)) {
                unknown[count] = column;
                count += 1;
            }
        }
        let unknown = &unknown[..count];
        let mut rhs = zeroed_shards(count * size)?;
        let mut matrix = [[0; PARITY_ROWS * 2]; PARITY_ROWS];
        for (equation, row) in (0..PARITY_ROWS)
            .filter(|&row| !missing_parity(row))
            .take(count)
            .enumerate()
        {
            let stored_offset = (parity_base + row) * size;
            let residual = &mut rhs[equation * size..][..size];
            residual.copy_from_slice(&self.parity[stored_offset..][..size]);
            for column in (0..columns).filter(|&column| !contains(&mismatch.data, index(column))) {
                add_data_shard(residual, data(column), row, column);
            }
            for (column, &source) in unknown.iter().enumerate() {
                matrix[equation][column] = matrix_coefficient(row, source);
            }
            matrix[equation][PARITY_ROWS + equation] = 1;
        }
        invert(&mut matrix, count);
        let mut rebuilt = zeroed_shards(count * size)?;
        for (variable, &column) in unknown.iter().enumerate() {
            let bytes = &mut rebuilt[variable * size..][..size];
            for (equation, residual) in rhs.chunks_exact(size).enumerate() {
                let factor = matrix[variable][PARITY_ROWS + equation];
                for (output, &input) in bytes.iter_mut().zip(residual) {
                    *output ^= multiply(factor, input);
                }
            }
            let shard = index(column);
            let length = data(column).len();
            let mut hasher = initial.clone();
            hasher.update(bytes);
            solution.solved_digests_match &= matches_shard(&hasher, &self.data_digests[shard])
                && bytes[length..].iter().all(|&padding| padding == 0);
            if contains(writable, shard) {
                let mut patch = Vec::new();
                patch.try_reserve_exact(length)?;
                patch.extend_from_slice(&bytes[..length]);
                solution.data.push(ParityPatch {
                    index: shard,
                    bytes: patch,
                });
            }
        }
        for row in (0..PARITY_ROWS).filter(|&row| missing_parity(row)) {
            let mut bytes = zeroed_shards(size)?;
            for column in 0..columns {
                let shard = unknown.binary_search(&column).map_or_else(
                    |_| data(column),
                    |variable| &rebuilt[variable * size..][..size],
                );
                add_data_shard(&mut bytes, shard, row, column);
            }
            let mut hasher = initial.clone();
            hasher.update(&bytes);
            let index = parity_base + row;
            solution.solved_digests_match &= matches_shard(&hasher, &self.parity_digests[index]);
            solution.parity.push(ParityPatch { index, bytes });
        }
        Ok(())
    }
}

fn contains(mask: &[u64; DATA_MASKS], shard: usize) -> bool {
    mask[shard / MASK_WIDTH] & (1 << (shard % MASK_WIDTH)) != 0
}

fn zeroed_shards(len: usize) -> Result<Vec<u8>, TryReserveError> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(len)?;
    bytes.resize(len, 0);
    Ok(bytes)
}

fn invert(matrix: &mut [[u8; PARITY_ROWS * 2]; PARITY_ROWS], count: usize) {
    for column in 0..count {
        // Every square submatrix of the Geometry 1 Cauchy matrix is
        // nonsingular. Elimination must find a pivot among its remaining rows.
        let pivot = (column..count)
            .find(|&row| matrix[row][column] != 0)
            .expect("the admitted Cauchy minor has a pivot");
        matrix.swap(column, pivot);
        let scale = inverse(matrix[column][column]);
        for cell in &mut matrix[column] {
            *cell = multiply(*cell, scale);
        }
        let normalized = matrix[column];
        for (row, equation) in matrix.iter_mut().take(count).enumerate() {
            if row != column {
                let factor = equation[column];
                for (cell, coefficient) in equation.iter_mut().zip(normalized) {
                    *cell ^= multiply(coefficient, factor);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DigestEntry, Geometry, SHARD_DIGEST_BYTES};
    use super::{PARITY_ROWS, ParityTable, add_data_shard, shard_hasher};

    fn digest(bytes: &[u8]) -> DigestEntry {
        let mut hash = shard_hasher();
        hash.update(bytes);
        let mut entry = [0x50; SHARD_DIGEST_BYTES + 1];
        entry[1..].copy_from_slice(&hash.finalize().as_bytes()[..SHARD_DIGEST_BYTES]);
        entry
    }

    #[test]
    fn a_solution_with_nonzero_padding_keeps_its_patch_and_fails_verification() {
        // The table is internally consistent with a final data shard that
        // has a real byte of 1 followed by 63 illegal padding bytes.
        let data_digests = [digest(&[0; 64]), digest(&[1; 64])];
        let mut parity = [[0; 64]; PARITY_ROWS];
        for (row, bytes) in parity.iter_mut().enumerate() {
            add_data_shard(bytes, &[1; 64], row, 1);
        }
        let parity_digests = parity.each_ref().map(|bytes| digest(bytes));
        let table = ParityTable {
            span: 0..65,
            geometry: Geometry::for_span(0, 65).unwrap(),
            data_digests: &data_digests,
            parity_digests: &parity_digests,
            parity: parity.as_flattened(),
        };
        let solution = table.solve_shards(&[0; 65], |_| true).unwrap().unwrap();
        assert!(!solution.solved_digests_match);
        let [patch] = solution.data.as_slice() else {
            panic!("expected one patch for the last data shard");
        };
        assert_eq!((patch.index, patch.bytes.as_slice()), (1, [1].as_slice()));
    }
}
