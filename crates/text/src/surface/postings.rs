// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed scalar gram keys, persistent posting ranges and protected endpoints.
use super::{Spelling, TextError};
use crate::index::{Slice, Table};
use std::{borrow::Borrow, cmp::Ordering};

/// One position in the ordered join of sorted streams.
pub(crate) enum Joined<T> {
    Left(T),
    Right(T),
    Both(T, T),
}
impl<T> Joined<T> {
    pub(crate) fn one(self) -> T {
        match self {
            Self::Left(v) | Self::Right(v) | Self::Both(v, _) => v,
        }
    }
}
/// The sorted join shared by posting unions and delta validation.
pub(crate) fn join_by<T>(
    left: impl IntoIterator<Item = T>,
    right: impl IntoIterator<Item = T>,
    mut compare: impl FnMut(&T, &T) -> Ordering,
) -> impl Iterator<Item = Joined<T>> {
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    std::iter::from_fn(move || {
        let order = match (left.peek(), right.peek()) {
            (Some(a), Some(b)) => compare(a, b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => return None,
        };
        Some(match order {
            Ordering::Less => Joined::Left(left.next().expect("peeked left row")),
            Ordering::Greater => Joined::Right(right.next().expect("peeked right row")),
            Ordering::Equal => Joined::Both(
                left.next().expect("peeked left row"),
                right.next().expect("peeked right row"),
            ),
        })
    })
}

#[derive(Clone, Debug)]
pub(super) struct FlatPostings<K, V> {
    rows: Table<(K, V)>,
}
impl<K: Ord + Clone, V: Ord + Clone> FlatPostings<K, V> {
    pub(super) fn build(mut rows: Vec<(K, V)>) -> Result<Self, TextError> {
        rows.sort_unstable();
        rows.dedup();
        Ok(Self {
            rows: Table::from_streams(rows)?,
        })
    }
    pub(super) fn insert(&mut self, key: K, value: V) -> Result<(), TextError> {
        let entry = (key, value);
        if let Err(at) = self.rows.binary_search_by(|row| row.cmp(&entry)) {
            self.rows = self.rows.insert(at, entry)?;
        }
        Ok(())
    }
    pub(super) fn remove(&mut self, key: &K, value: &V) -> Result<(), TextError> {
        let at = self
            .rows
            .binary_search_by(|row| row.0.cmp(key).then_with(|| row.1.cmp(value)))
            .map_err(|_| TextError::data("a retired surface term has no original keyed posting"))?;
        self.rows = self.rows.remove(at)?;
        Ok(())
    }
    pub(super) fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<PostingSlice<'_, K, V>>
    where
        K: Borrow<Q>,
    {
        let start = self.rows.partition_point(|row| row.0.borrow() < key);
        let end = self.rows.partition_point(|row| row.0.borrow() <= key);
        (start != end).then(|| PostingSlice {
            rows: self.rows.slice(start, end),
        })
    }
}

pub(super) struct PostingSlice<'a, K, V> {
    rows: Slice<'a, (K, V)>,
}
impl<K, V> Copy for PostingSlice<'_, K, V> {}
impl<K, V> Clone for PostingSlice<'_, K, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K: Clone, V: Clone> Default for PostingSlice<'_, K, V> {
    fn default() -> Self {
        Self {
            rows: Slice::empty(),
        }
    }
}
impl<'a, K: Clone, V: Clone> PostingSlice<'a, K, V> {
    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }
    pub(super) fn iter(&self) -> impl ExactSizeIterator<Item = &'a V> + Clone + use<'a, K, V> {
        self.rows.iter().map(|row| &row.1)
    }
    pub(super) fn binary_search(&self, value: &V) -> Result<usize, usize>
    where
        V: Ord,
    {
        self.rows.binary_search_by(|row| row.1.cmp(value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct GramPosting {
    pub span: Spelling,
    pub position: u64,
}

#[derive(Clone, Debug)]
pub(super) struct SpanShape {
    pub offsets: Vec<usize>,
    pub eligible: Vec<bool>,
}
impl SpanShape {
    pub(super) fn new(text: &str) -> Self {
        let offsets: Vec<_> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
        let mut eligible = vec![true; offsets.len()];
        for (start, cluster) in crate::unicode::grapheme_bounds(text) {
            if crate::unicode::is_emoji_grapheme(cluster) {
                let first = offsets
                    .binary_search(&start)
                    .expect("grapheme starts at a scalar");
                let last = offsets
                    .binary_search(&(start + cluster.len()))
                    .expect("grapheme ends at a scalar");
                eligible[first + 1..last].fill(false);
            }
        }
        Self { offsets, eligible }
    }
}
pub(super) fn key(scalars: &[char]) -> u64 {
    scalars
        .iter()
        .fold(0, |key, &scalar| (key << 21) | u64::from(scalar))
}
pub(super) fn gram_rows(text: &Spelling, width: usize) -> Vec<(u64, GramPosting)> {
    let scalars: Vec<_> = text.text().chars().collect();
    scalars
        .windows(width)
        .enumerate()
        .map(|(position, gram)| {
            (
                key(gram),
                GramPosting {
                    span: text.clone(),
                    position: position as u64,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::FlatPostings;

    #[test]
    fn keyed_edits_match_fresh_union_and_retraction_and_preserve_prior_snapshots() {
        let original = [(10_u8, 0_u32), (10, 2), (30, 5)];
        let added = [(10, 2), (10, 4), (20, 1)];
        let mut table = FlatPostings::build(original.to_vec()).unwrap();
        let prior = table.clone();
        for (key, value) in added {
            table.insert(key, value).unwrap();
        }
        let fresh = FlatPostings::build(original.into_iter().chain(added).collect()).unwrap();
        for key in [10, 20, 30, 40] {
            assert_eq!(
                table
                    .get(&key)
                    .unwrap_or_default()
                    .iter()
                    .copied()
                    .collect::<Vec<_>>(),
                fresh
                    .get(&key)
                    .unwrap_or_default()
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
            );
        }
        table.remove(&10, &2).unwrap();
        assert_eq!(
            table.get(&10).unwrap().iter().copied().collect::<Vec<_>>(),
            [0, 4]
        );
        assert_eq!(
            prior.get(&10).unwrap().iter().copied().collect::<Vec<_>>(),
            [0, 2]
        );
        assert!(table.remove(&10, &2).is_err());
        table.remove(&10, &0).unwrap();
        table.remove(&10, &4).unwrap();
        assert!(table.get(&10).is_none());
    }
}
