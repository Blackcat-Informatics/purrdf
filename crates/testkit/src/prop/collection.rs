// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Collections of generated elements: [`vec()`], [`btree_set`], [`btree_map`].
//!
//! A collection's size is uniform over its [`SizeRange`] in random mode. Past
//! the minimum, every further element is preceded by a recorded continuation
//! flag, so a shrinker that deletes an element's span, or clears a flag, gets a
//! shorter collection that still respects the minimum.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::{Range, RangeInclusive, RangeTo, RangeToInclusive};

use super::choices::{Choices, Invalid};
use super::strategy::Strategy;

/// An inclusive range of collection sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeRange {
    min: usize,
    max: usize,
}

impl SizeRange {
    /// Sizes `min..=max`.
    pub fn new(min: usize, max: usize) -> Self {
        assert!(min <= max, "the empty size range {min}..={max}");
        Self { min, max }
    }

    /// The smallest size.
    pub const fn min(self) -> usize {
        self.min
    }

    /// The largest size.
    pub const fn max(self) -> usize {
        self.max
    }

    /// Whether `len` is in the range.
    pub const fn contains(self, len: usize) -> bool {
        self.min <= len && len <= self.max
    }
}

impl From<usize> for SizeRange {
    fn from(exact: usize) -> Self {
        Self::new(exact, exact)
    }
}

impl From<Range<usize>> for SizeRange {
    fn from(range: Range<usize>) -> Self {
        assert!(range.start < range.end, "the empty size range {range:?}");
        Self::new(range.start, range.end - 1)
    }
}

impl From<RangeInclusive<usize>> for SizeRange {
    fn from(range: RangeInclusive<usize>) -> Self {
        Self::new(*range.start(), *range.end())
    }
}

impl From<RangeTo<usize>> for SizeRange {
    fn from(range: RangeTo<usize>) -> Self {
        Self::from(0..range.end)
    }
}

impl From<RangeToInclusive<usize>> for SizeRange {
    fn from(range: RangeToInclusive<usize>) -> Self {
        Self::new(0, range.end)
    }
}

/// Sizing state for one collection draw.
pub(crate) struct Sizer {
    size: SizeRange,
    target: usize,
}

impl Sizer {
    pub(crate) fn start(choices: &mut Choices, size: SizeRange) -> Self {
        let target = choices.target_size(size.min, size.max);
        Self { size, target }
    }

    /// Whether to add another element to a collection that holds `len`: no
    /// draw below the minimum or at the maximum, a recorded flag between.
    /// `give_up` stops a random-mode collection that cannot grow.
    pub(crate) fn more(
        &self,
        choices: &mut Choices,
        len: usize,
        give_up: bool,
    ) -> Result<bool, Invalid> {
        if len < self.size.min {
            return Ok(true);
        }
        if len >= self.size.max {
            return Ok(false);
        }
        choices.forced_bool(len < self.target && !give_up)
    }
}

/// A `Vec` whose length is in `size` and whose elements come from `element`.
pub fn vec<S: Strategy>(element: S, size: impl Into<SizeRange>) -> VecStrategy<S> {
    VecStrategy {
        element,
        size: size.into(),
    }
}

/// See [`vec()`].
#[derive(Debug, Clone)]
pub struct VecStrategy<S> {
    element: S,
    size: SizeRange,
}

impl<S: Strategy> Strategy for VecStrategy<S> {
    type Value = Vec<S::Value>;

    fn generate(&self, choices: &mut Choices) -> Result<Self::Value, Invalid> {
        let sizer = Sizer::start(choices, self.size);
        let mut out = Vec::new();
        while sizer.more(choices, out.len(), false)? {
            out.push(self.element.generate(choices)?);
        }
        Ok(out)
    }
}

/// Duplicates tolerated past the minimum before a random-mode set stops
/// growing: the element space may simply be smaller than the target.
fn miss_limit(size: SizeRange) -> usize {
    size.max.saturating_mul(4).saturating_add(16)
}

/// A `BTreeSet` whose size is in `size` and whose elements come from
/// `element`. A duplicate element is redrawn; below the minimum each
/// duplicate is charged to the reject budget.
pub fn btree_set<S>(element: S, size: impl Into<SizeRange>) -> BTreeSetStrategy<S>
where
    S: Strategy,
    S::Value: Ord,
{
    BTreeSetStrategy {
        element,
        size: size.into(),
    }
}

/// See [`btree_set`].
#[derive(Debug, Clone)]
pub struct BTreeSetStrategy<S> {
    element: S,
    size: SizeRange,
}

impl<S> Strategy for BTreeSetStrategy<S>
where
    S: Strategy,
    S::Value: Ord,
{
    type Value = BTreeSet<S::Value>;

    fn generate(&self, choices: &mut Choices) -> Result<Self::Value, Invalid> {
        let sizer = Sizer::start(choices, self.size);
        let mut out = BTreeSet::new();
        let mut misses = 0usize;
        while sizer.more(choices, out.len(), misses >= miss_limit(self.size))? {
            if !out.insert(self.element.generate(choices)?) {
                misses += 1;
                if out.len() < self.size.min {
                    choices.reject("btree_set: too few distinct elements for the minimum size")?;
                }
            }
        }
        Ok(out)
    }
}

/// A `BTreeMap` whose size is in `size`, with keys from `key` and values from
/// `value`. A duplicate key is redrawn as [`btree_set`] redraws a duplicate
/// element.
pub fn btree_map<K, V>(key: K, value: V, size: impl Into<SizeRange>) -> BTreeMapStrategy<K, V>
where
    K: Strategy,
    K::Value: Ord,
    V: Strategy,
{
    BTreeMapStrategy {
        key,
        value,
        size: size.into(),
    }
}

/// See [`btree_map`].
#[derive(Debug, Clone)]
pub struct BTreeMapStrategy<K, V> {
    key: K,
    value: V,
    size: SizeRange,
}

impl<K, V> Strategy for BTreeMapStrategy<K, V>
where
    K: Strategy,
    K::Value: Ord,
    V: Strategy,
{
    type Value = BTreeMap<K::Value, V::Value>;

    fn generate(&self, choices: &mut Choices) -> Result<Self::Value, Invalid> {
        let sizer = Sizer::start(choices, self.size);
        let mut out = BTreeMap::new();
        let mut misses = 0usize;
        while sizer.more(choices, out.len(), misses >= miss_limit(self.size))? {
            let key = self.key.generate(choices)?;
            let value = self.value.generate(choices)?;
            let below_min = out.len() < self.size.min;
            match out.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(value);
                }
                Entry::Occupied(_) => {
                    misses += 1;
                    if below_min {
                        choices.reject("btree_map: too few distinct keys for the minimum size")?;
                    }
                }
            }
        }
        Ok(out)
    }
}

impl fmt::Display for SizeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..={}", self.min, self.max)
    }
}
