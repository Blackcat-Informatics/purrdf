// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed scalar gram keys, contiguous sorted postings and protected endpoints.
use super::SurfaceTerm;
use std::borrow::Borrow;

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
