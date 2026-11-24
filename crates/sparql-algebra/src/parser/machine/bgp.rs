// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::algebra::GraphPattern;
use crate::ast::TriplePattern;

/// A parser operand. Only a mergeable group of triples may defer its public algebra.
#[expect(
    clippy::large_enum_variant,
    reason = "keep the existing inline algebra in the admitted group frame without a separate allocation"
)]
pub(super) enum Pattern {
    Algebra(GraphPattern),
    Bgp(Bgp),
}

impl From<GraphPattern> for Pattern {
    fn from(pattern: GraphPattern) -> Self {
        match pattern {
            GraphPattern::Bgp { patterns } => Self::Bgp(Bgp::Rows(patterns)),
            pattern => Self::Algebra(pattern),
        }
    }
}

impl Pattern {
    pub(super) fn finish<S: Admission + ?Sized>(
        self,
        chunks: &mut ChunkArena,
        memory: &mut Memory<'_, S>,
    ) -> Result<GraphPattern, StorageError> {
        match self {
            Self::Algebra(pattern) => Ok(pattern),
            Self::Bgp(bgp) => Ok(GraphPattern::Bgp {
                patterns: chunks.flatten(bgp, memory)?,
            }),
        }
    }
}

pub(super) enum Bgp {
    Rows(Vec<TriplePattern>),
    Chunks(Chain),
}

pub(super) struct Chain {
    first: usize,
    last: usize,
    len: usize,
}

/// Each slot owns one admitted triple array. A chain joins in constant time and
/// drains iteratively, even when every source group adds a single triple.
#[derive(Default)]
pub(super) struct ChunkArena {
    slots: Vec<Chunk>,
    free: Option<usize>,
}

struct Chunk {
    rows: Vec<TriplePattern>,
    /// Next chunk in an active chain, or the next free slot after draining.
    next: Option<usize>,
}

impl ChunkArena {
    pub(super) fn join<S: Admission + ?Sized>(
        &mut self,
        left: Bgp,
        right: Bgp,
        memory: &mut Memory<'_, S>,
    ) -> Result<Bgp, StorageError> {
        let left = self.chain(left, memory)?;
        let right = self.chain(right, memory)?;
        self.slots[left.last].next = Some(right.first);
        Ok(Bgp::Chunks(Chain {
            first: left.first,
            last: right.last,
            len: left
                .len
                .checked_add(right.len)
                .ok_or(StorageError::SizeOverflow)?,
        }))
    }

    fn chain<S: Admission + ?Sized>(
        &mut self,
        bgp: Bgp,
        memory: &mut Memory<'_, S>,
    ) -> Result<Chain, StorageError> {
        let rows = match bgp {
            Bgp::Chunks(chain) => return Ok(chain),
            Bgp::Rows(rows) => rows,
        };
        debug_assert_ne!(rows, [] as [TriplePattern; 0]);
        let len = rows.len();
        let chunk = Chunk { rows, next: None };
        let index = if let Some(index) = self.free {
            self.free = self.slots[index].next;
            self.slots[index] = chunk;
            index
        } else {
            let index = self.slots.len();
            memory.push(&mut self.slots, chunk)?;
            index
        };
        Ok(Chain {
            first: index,
            last: index,
            len,
        })
    }

    fn flatten<S: Admission + ?Sized>(
        &mut self,
        bgp: Bgp,
        memory: &mut Memory<'_, S>,
    ) -> Result<Vec<TriplePattern>, StorageError> {
        let chain = match bgp {
            Bgp::Rows(rows) => return Ok(rows),
            Bgp::Chunks(chain) => chain,
        };
        let (mut rows, mut next) = self.take(chain.first);
        memory.reserve(&mut rows, chain.len)?;
        while let Some(index) = next {
            let (source, remainder) = self.take(index);
            memory.append(&mut rows, source)?;
            next = remainder;
        }
        Ok(rows)
    }

    fn take(&mut self, index: usize) -> (Vec<TriplePattern>, Option<usize>) {
        let chunk = &mut self.slots[index];
        let rows = std::mem::take(&mut chunk.rows);
        let next = std::mem::replace(&mut chunk.next, self.free);
        self.free = Some(index);
        (rows, next)
    }

    pub(super) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        debug_assert!(self.slots.iter().all(|chunk| chunk.rows.is_empty()));
        memory.release_vec(self.slots)
    }
}
