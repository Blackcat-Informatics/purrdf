// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed scalar gram keys, contiguous sorted postings and protected endpoints.
use super::SurfaceTerm;
use std::borrow::Borrow;
use std::cmp::Ordering;

/// One position in the ordered join of two already sorted streams.
pub(crate) enum Joined<T> {
    Left(T),
    Right(T),
    Both(T, T),
}

impl<T> Joined<T> {
    pub(crate) fn one(self) -> T {
        match self {
            Self::Left(value) | Self::Right(value) | Self::Both(value, _) => value,
        }
    }
}

/// The one sorted join used by posting unions and incremental index tables.
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
pub(super) struct FlatPostings<K, P> {
    keys: Vec<K>,
    offsets: Vec<usize>,
    postings: Vec<P>,
}
impl<K: Ord, P: Ord> FlatPostings<K, P> {
    pub(super) fn build(mut rows: Vec<(K, P)>) -> Self {
        rows.sort_unstable();
        rows.dedup();
        let mut result = Self {
            keys: Vec::new(),
            offsets: Vec::new(),
            postings: Vec::with_capacity(rows.len()),
        };
        for (key, posting) in rows {
            if result.keys.last() != Some(&key) {
                result.keys.push(key);
                result.offsets.push(result.postings.len());
            }
            result.postings.push(posting);
        }
        result.offsets.push(result.postings.len());
        result
    }

    /// Merge sorted runs, coalescing equal keys and equal postings. Only the
    /// added table was built afresh; retained postings are never sorted again.
    pub(super) fn merge(self, added: Self) -> Self {
        fn lengths(offsets: &[usize]) -> impl Iterator<Item = usize> {
            offsets.windows(2).map(|pair| pair[1] - pair[0])
        }
        let Self {
            keys,
            offsets,
            postings,
        } = self;
        let Self {
            keys: added_keys,
            offsets: added_offsets,
            postings: added_postings,
        } = added;
        let mut result = Self {
            keys: Vec::new(),
            offsets: Vec::new(),
            postings: Vec::with_capacity(postings.len() + added_postings.len()),
        };
        let mut postings = postings.into_iter();
        let mut added_postings = added_postings.into_iter();
        let left = keys.into_iter().zip(lengths(&offsets));
        let right = added_keys.into_iter().zip(lengths(&added_offsets));
        for run in join_by(left, right, |a, b| a.0.cmp(&b.0)) {
            let (key, left_count, right_count) = match run {
                Joined::Left((key, count)) => (key, count, 0),
                Joined::Right((key, count)) => (key, 0, count),
                Joined::Both((key, left), (_, right)) => (key, left, right),
            };
            result.offsets.push(result.postings.len());
            result.keys.push(key);
            result.postings.extend(
                join_by(
                    postings.by_ref().take(left_count),
                    added_postings.by_ref().take(right_count),
                    P::cmp,
                )
                .map(Joined::one),
            );
        }
        result.offsets.push(result.postings.len());
        result
    }

    /// Remove postings and remap retained ordinals without rebuilding or sorting
    /// their keys. The caller's remap must preserve strict order within a key.
    /// The large posting allocation and the offset allocation are reused.
    pub(super) fn retain_remapped(self, mut remap: impl FnMut(P) -> Option<P>) -> Self
    where
        P: Copy,
    {
        let Self {
            keys,
            mut offsets,
            mut postings,
        } = self;
        let mut retained_keys = Vec::with_capacity(keys.len());
        let mut written = 0;
        for (ordinal, key) in keys.into_iter().enumerate() {
            let start = offsets[ordinal];
            let end = offsets[ordinal + 1];
            let output_start = written;
            let mut previous = None;
            for read in start..end {
                if let Some(posting) = remap(postings[read]) {
                    debug_assert!(
                        previous.is_none_or(|prior| prior < posting),
                        "posting remap must preserve strict key order"
                    );
                    previous = Some(posting);
                    postings[written] = posting;
                    written += 1;
                }
            }
            if written != output_start {
                offsets[retained_keys.len()] = output_start;
                retained_keys.push(key);
            }
        }
        offsets[retained_keys.len()] = written;
        offsets.truncate(retained_keys.len() + 1);
        postings.truncate(written);
        Self {
            keys: retained_keys,
            offsets,
            postings,
        }
    }
    pub(super) fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&[P]>
    where
        K: Borrow<Q>,
    {
        let at = self
            .keys
            .binary_search_by(|candidate| candidate.borrow().cmp(key))
            .ok()?;
        Some(&self.postings[self.offsets[at]..self.offsets[at + 1]])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct GramPosting {
    pub span: u32,
    pub position: u64,
}

#[derive(Clone, Debug)]
pub(super) struct SpanShape {
    pub offsets: Vec<usize>,
    pub eligible: Vec<bool>,
}
impl SpanShape {
    fn new(text: &str) -> Self {
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
pub(super) fn build_grams(
    spans: &[SurfaceTerm],
) -> ([FlatPostings<u64, GramPosting>; 3], Vec<SpanShape>) {
    let mut rows: [Vec<(u64, GramPosting)>; 3] = std::array::from_fn(|_| Vec::new());
    let mut shapes = Vec::with_capacity(spans.len());
    for (span, term) in spans.iter().enumerate() {
        let scalars: Vec<_> = term.text.chars().collect();
        for (index, rows) in rows.iter_mut().enumerate() {
            for (position, gram) in scalars.windows(index + 1).enumerate() {
                rows.push((
                    key(gram),
                    GramPosting {
                        span: span as u32,
                        position: position as u64,
                    },
                ));
            }
        }
        shapes.push(SpanShape::new(&term.text));
    }
    (rows.map(FlatPostings::build), shapes)
}

#[cfg(test)]
mod tests {
    use super::FlatPostings;

    #[test]
    fn ordered_delta_merge_matches_fresh_sorted_union() {
        let original = [(10_u8, 0_u32), (10, 2), (30, 5)];
        let added = [(10, 2), (10, 4), (20, 1)];
        let merged =
            FlatPostings::build(original.to_vec()).merge(FlatPostings::build(added.to_vec()));
        let fresh = FlatPostings::build(original.into_iter().chain(added).collect());
        assert_eq!(merged.keys, fresh.keys);
        assert_eq!(merged.offsets, fresh.offsets);
        assert_eq!(merged.postings, fresh.postings);
    }

    #[test]
    fn retraction_matches_fresh_postings_and_keeps_the_posting_buffer() {
        let original =
            FlatPostings::build(vec![(10_u8, 0_u32), (10, 1), (10, 2), (20, 0), (30, 1)]);
        let allocation = original.postings.as_ptr();
        let capacity = original.postings.capacity();
        let map = [Some(0), None, Some(1)];
        let updated = original.retain_remapped(|posting| map[posting as usize]);
        let fresh = FlatPostings::build(vec![(10, 0), (10, 1), (20, 0)]);
        assert_eq!(updated.keys, fresh.keys);
        assert_eq!(updated.offsets, fresh.offsets);
        assert_eq!(updated.postings, fresh.postings);
        assert_eq!(updated.postings.as_ptr(), allocation);
        assert_eq!(updated.postings.capacity(), capacity);
        let empty = updated.retain_remapped(|_| None);
        assert_eq!(empty.offsets, [0]);
        assert_eq!(empty.keys, [] as [u8; 0]);
        assert_eq!(empty.postings, [] as [u32; 0]);
    }
}
