// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Private lookup state for insertion-ordered digest tables during a fold.

use std::collections::{HashMap, hash_map::Entry};

// Tiny folds already fit in the ordered table's cache footprint. Bound the
// scan, then build the index once when a seventeenth distinct digest arrives.
const INLINE_DIGESTS: usize = 16;

/// Created together with an empty table and kept for that table's whole fold.
/// Public graphs retain their ordinary vectors; index iteration never emits data.
#[derive(Default)]
pub(crate) struct DigestIndex {
    positions: HashMap<String, usize, purrdf_hash::fixed::FixedState>,
}

impl DigestIndex {
    /// Insert once or replace in place without changing the table's order.
    pub(crate) fn set<T>(&mut self, table: &mut Vec<(String, T)>, digest: String, value: T) {
        if self.positions.is_empty() {
            if table.len() < INLINE_DIGESTS
                || purrdf_lex::assoc::get(table, digest.as_str()).is_some()
            {
                purrdf_lex::assoc::insert(table, digest, value);
                return;
            }
            self.positions.reserve(table.len() + 1);
            self.positions.extend(
                table
                    .iter()
                    .enumerate()
                    .map(|(index, (key, _))| (key.clone(), index)),
            );
        }
        match self.positions.entry(digest) {
            Entry::Occupied(position) => table[*position.get()].1 = value,
            Entry::Vacant(position) => {
                let index = table.len();
                table.push((position.key().clone(), value));
                position.insert(index);
            }
        }
    }

    /// Borrow a value from its authoritative ordered table.
    pub(crate) fn get<'a, T>(&self, table: &'a [(String, T)], digest: &str) -> Option<&'a T> {
        if self.positions.is_empty() {
            return purrdf_lex::assoc::get(table, digest);
        }
        self.positions.get(digest).map(|&index| &table[index].1)
    }
}
