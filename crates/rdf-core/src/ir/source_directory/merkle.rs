// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical BLAKE3 Merkle nodes over a sorted multiset of member digests.
//!
//! A node is a function of the members alone, never of how a carrier chunks,
//! pages or checkpoints them: it is the binary Patricia trie over the bits of
//! the member digests.
//!
//! * A run of `n` equal digests `d` is a leaf. Its node is `d` itself when `n`
//!   is one, and otherwise the BLAKE3 keyed hash of `d ‖ n` (`n` as a
//!   little-endian `u64`) under the key derived from [`LEAF_DOMAIN`].
//! * Members holding at least two distinct digests split at the first bit
//!   where their least and greatest digests differ. The node is the BLAKE3
//!   keyed hash of the left node (that bit clear) followed by the right node,
//!   under the key derived from [`BRANCH_DOMAIN`].
//!
//! Each key is `blake3::derive_key(domain, b"")`. A member digest is itself a
//! domain-separated BLAKE3 output, so a single member cannot stand for a
//! branch or a repeated leaf. Equal nodes therefore mean equal multisets,
//! unlike the additive sum, whose collisions a party choosing rows can build.

use std::sync::OnceLock;

use purrdf_hash::{Domain, blake3};

use super::first_differing_bit;

/// The key law of a node with two nonempty children.
pub(crate) const BRANCH_DOMAIN: Domain = Domain::new(b"purrdf-core/source-directory-branch/v1");
/// The key law of a run of at least two equal member digests.
pub(crate) const LEAF_DOMAIN: Domain = Domain::new(b"purrdf-core/source-directory-leaf/v1");

/// The canonical node of a nonempty member multiset.
pub(crate) type DirectoryNode = [u8; 32];

pub(crate) fn branch(left: &DirectoryNode, right: &DirectoryNode) -> DirectoryNode {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    let key = KEY.get_or_init(|| blake3::derive_key(BRANCH_DOMAIN.as_str(), b""));
    let mut input = [0; 64];
    input[..32].copy_from_slice(left);
    input[32..].copy_from_slice(right);
    *blake3::keyed_hash(key, &input).as_bytes()
}

pub(crate) fn leaf(digest: [u8; 32], count: u64) -> DirectoryNode {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    debug_assert_ne!(count, 0, "a leaf holds at least one member");
    if count == 1 {
        return digest;
    }
    let key = KEY.get_or_init(|| blake3::derive_key(LEAF_DOMAIN.as_str(), b""));
    let mut input = [0; 40];
    input[..32].copy_from_slice(&digest);
    crate::bytes::put_u64_le(&mut input, 32, count).expect("a 40-byte leaf input");
    *blake3::keyed_hash(key, &input).as_bytes()
}

/// One branch node, reported as it is completed. Its boundary is the ordinal
/// of the first member of its right child; no two branches share a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Branch {
    pub(crate) boundary: u64,
    pub(crate) population: u64,
    pub(crate) node: DirectoryNode,
}

#[derive(Clone, Copy)]
struct Entry {
    node: DirectoryNode,
    start: u64,
    population: u64,
    // The bit separating this subtree from the one below it on the spine.
    bit: u16,
}

const EMPTY: Entry = Entry {
    node: [0; 32],
    start: 0,
    population: 0,
    bit: 0,
};

/// The spine bound for any member count.
pub(crate) const SPINE: usize = 257;

/// Build the canonical node of digests pushed in ascending order.
///
/// The right spine holds subtrees whose separating bits strictly increase
/// upwards, so a 256-bit digest bounds it at [`SPINE`] entries, and it never
/// holds more entries than distinct digests. `N` is the bound the caller can
/// prove: [`SPINE`] in general, or the member count of a short range, so that
/// recomputing a small range does not initialise a full spine. The work is
/// linear in the members and nothing is allocated.
pub(crate) struct NodeBuilder<const N: usize> {
    spine: [Entry; N],
    depth: usize,
    run: Option<([u8; 32], u64, u64)>,
    bit: u16,
    next: u64,
}

impl<const N: usize> NodeBuilder<N> {
    /// A builder whose first member has ordinal `start`.
    pub(crate) const fn new(start: u64) -> Self {
        Self {
            spine: [EMPTY; N],
            depth: 0,
            run: None,
            bit: 0,
            next: start,
        }
    }

    /// Add the next member. Digests arrive in ascending order.
    pub(crate) fn push(&mut self, digest: [u8; 32], emit: &mut impl FnMut(Branch)) {
        match self.run {
            Some((run, _, ref mut count)) if run == digest => *count += 1,
            Some((run, start, count)) => {
                debug_assert!(run < digest, "directory members arrive in ascending order");
                let bit = first_differing_bit(&run, &digest).expect("distinct digests differ");
                self.shift(
                    Entry {
                        node: leaf(run, count),
                        start,
                        population: count,
                        bit: self.bit,
                    },
                    emit,
                );
                self.bit = bit;
                self.run = Some((digest, self.next, 1));
            }
            None => self.run = Some((digest, self.next, 1)),
        }
        self.next += 1;
    }

    fn shift(&mut self, entry: Entry, emit: &mut impl FnMut(Branch)) {
        while self.depth >= 2 && self.spine[self.depth - 1].bit > entry.bit {
            self.merge(emit);
        }
        self.spine[self.depth] = entry;
        self.depth += 1;
    }

    fn merge(&mut self, emit: &mut impl FnMut(Branch)) {
        let right = self.spine[self.depth - 1];
        let left = self.spine[self.depth - 2];
        let node = branch(&left.node, &right.node);
        let population = left.population + right.population;
        emit(Branch {
            boundary: right.start,
            population,
            node,
        });
        self.depth -= 1;
        self.spine[self.depth - 1] = Entry {
            node,
            start: left.start,
            population,
            bit: left.bit,
        };
    }

    /// The node of every member pushed, or `None` when there was none.
    pub(crate) fn finish(mut self, emit: &mut impl FnMut(Branch)) -> Option<DirectoryNode> {
        let (run, start, count) = self.run?;
        self.shift(
            Entry {
                node: leaf(run, count),
                start,
                population: count,
                bit: self.bit,
            },
            emit,
        );
        while self.depth >= 2 {
            self.merge(emit);
        }
        Some(self.spine[0].node)
    }
}

/// The node of `digests`, which are sorted ascending.
pub(crate) fn node_of(digests: &[[u8; 32]]) -> Option<DirectoryNode> {
    fn build<const N: usize>(digests: &[[u8; 32]]) -> Option<DirectoryNode> {
        let mut builder = NodeBuilder::<N>::new(0);
        for digest in digests {
            builder.push(*digest, &mut |_| {});
        }
        builder.finish(&mut |_| {})
    }
    if digests.len() <= SMALL {
        build::<SMALL>(digests)
    } else {
        build::<SPINE>(digests)
    }
}

/// The spine bound of a range of at most this many members.
pub(crate) const SMALL: usize = super::CHECKPOINT_ROWS;
