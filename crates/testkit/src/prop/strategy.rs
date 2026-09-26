// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The [`Strategy`] trait and its combinators.

use std::fmt;
use std::ops::{Range, RangeInclusive};
use std::sync::Arc;

use super::choices::{Choices, Invalid};

/// A generator of values of one type, driven by a [`Choices`] source.
///
/// A strategy is a pure function of the choices it draws: the same recorded
/// sequence always produces the same value. Shrinking relies on nothing else.
pub trait Strategy {
    /// The generated type. `Debug`, so a failing value can be printed.
    type Value: fmt::Debug;

    /// Draw one value.
    fn generate(&self, choices: &mut Choices) -> Result<Self::Value, Invalid>;

    /// Transform every generated value with `map`.
    fn prop_map<O, F>(self, map: F) -> Map<Self, F>
    where
        Self: Sized,
        O: fmt::Debug,
        F: Fn(Self::Value) -> O,
    {
        Map { source: self, map }
    }

    /// Keep only the values `predicate` accepts. Each rejected value is
    /// redrawn and charged to the run's reject budget; `whence` names the
    /// filter when the budget runs out.
    fn prop_filter<F>(self, whence: impl Into<String>, predicate: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Value) -> bool,
    {
        Filter {
            source: self,
            whence: whence.into(),
            predicate,
        }
    }

    /// Transform every generated value with `map`, redrawing (as
    /// [`Strategy::prop_filter`] does) whenever it answers `None`.
    fn prop_filter_map<O, F>(self, whence: impl Into<String>, map: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        O: fmt::Debug,
        F: Fn(Self::Value) -> Option<O>,
    {
        FilterMap {
            source: self,
            whence: whence.into(),
            map,
        }
    }

    /// Generate a value, build a second strategy from it, and generate from
    /// that. Both draws come from the same sequence, so the pair shrinks
    /// together.
    fn prop_flat_map<S, F>(self, build: F) -> FlatMap<Self, F>
    where
        Self: Sized,
        S: Strategy,
        F: Fn(Self::Value) -> S,
    {
        FlatMap {
            source: self,
            build,
        }
    }

    /// A recursive strategy whose leaves are `self` and whose inner nodes are
    /// built by `recurse` from the strategy one level down.
    ///
    /// `depth` levels are stacked. At each level the recursive arm is chosen
    /// with probability `desired_size / (2 · expected_branch_size)^k` (capped
    /// at 0.9) for the `k`-th level counted from the top, so the expected tree
    /// holds about `desired_size` leaves; the leaf arm is the shrink target.
    fn prop_recursive<R, F>(
        self,
        depth: u32,
        desired_size: u32,
        expected_branch_size: u32,
        recurse: F,
    ) -> BoxedStrategy<Self::Value>
    where
        Self: Sized + 'static,
        R: Strategy<Value = Self::Value> + 'static,
        F: Fn(BoxedStrategy<Self::Value>) -> R,
    {
        let mut probabilities = Vec::with_capacity(depth as usize);
        let step = u64::from(expected_branch_size) * 2;
        let mut denominator = step;
        for _ in 0..depth {
            probabilities.push(f64::from(desired_size) / denominator as f64);
            denominator = denominator.saturating_mul(step);
        }
        let mut strategy = self.boxed();
        while let Some(probability) = probabilities.pop() {
            let probability = probability.min(0.9);
            let leaf_weight = (100.0 * (1.0 - probability)) as u32;
            let recursive_weight = (100.0 * probability) as u32;
            let recursed = recurse(strategy.clone()).boxed();
            strategy =
                Union::new_weighted(vec![(leaf_weight, strategy), (recursive_weight, recursed)])
                    .boxed();
        }
        strategy
    }

    /// Erase the strategy's type behind a cheaply clonable handle.
    fn boxed(self) -> BoxedStrategy<Self::Value>
    where
        Self: Sized + 'static,
    {
        BoxedStrategy(Arc::new(self))
    }
}

impl<S: Strategy + ?Sized> Strategy for &S {
    type Value = S::Value;

    fn generate(&self, choices: &mut Choices) -> Result<S::Value, Invalid> {
        (**self).generate(choices)
    }
}

/// A type-erased, clonable strategy.
pub struct BoxedStrategy<T>(Arc<dyn Strategy<Value = T>>);

impl<T> Clone for BoxedStrategy<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> fmt::Debug for BoxedStrategy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BoxedStrategy")
    }
}

impl<T: fmt::Debug> Strategy for BoxedStrategy<T> {
    type Value = T;

    fn generate(&self, choices: &mut Choices) -> Result<T, Invalid> {
        self.0.generate(choices)
    }

    fn boxed(self) -> Self {
        self
    }
}

/// Always the one value. Draws nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Just<T>(pub T);

impl<T: Clone + fmt::Debug> Strategy for Just<T> {
    type Value = T;

    fn generate(&self, _choices: &mut Choices) -> Result<T, Invalid> {
        Ok(self.0.clone())
    }
}

/// See [`Strategy::prop_map`].
#[derive(Clone)]
pub struct Map<S, F> {
    source: S,
    map: F,
}

impl<S: fmt::Debug, F> fmt::Debug for Map<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl<S: Strategy, O: fmt::Debug, F: Fn(S::Value) -> O> Strategy for Map<S, F> {
    type Value = O;

    fn generate(&self, choices: &mut Choices) -> Result<O, Invalid> {
        self.source.generate(choices).map(&self.map)
    }
}

/// See [`Strategy::prop_filter`].
#[derive(Clone)]
pub struct Filter<S, F> {
    source: S,
    whence: String,
    predicate: F,
}

impl<S: fmt::Debug, F> fmt::Debug for Filter<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filter")
            .field("source", &self.source)
            .field("whence", &self.whence)
            .finish_non_exhaustive()
    }
}

impl<S: Strategy, F: Fn(&S::Value) -> bool> Strategy for Filter<S, F> {
    type Value = S::Value;

    fn generate(&self, choices: &mut Choices) -> Result<S::Value, Invalid> {
        loop {
            let value = self.source.generate(choices)?;
            if (self.predicate)(&value) {
                return Ok(value);
            }
            choices.reject(&self.whence)?;
        }
    }
}

/// See [`Strategy::prop_filter_map`].
#[derive(Clone)]
pub struct FilterMap<S, F> {
    source: S,
    whence: String,
    map: F,
}

impl<S: fmt::Debug, F> fmt::Debug for FilterMap<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterMap")
            .field("source", &self.source)
            .field("whence", &self.whence)
            .finish_non_exhaustive()
    }
}

impl<S: Strategy, O: fmt::Debug, F: Fn(S::Value) -> Option<O>> Strategy for FilterMap<S, F> {
    type Value = O;

    fn generate(&self, choices: &mut Choices) -> Result<O, Invalid> {
        loop {
            if let Some(value) = (self.map)(self.source.generate(choices)?) {
                return Ok(value);
            }
            choices.reject(&self.whence)?;
        }
    }
}

/// See [`Strategy::prop_flat_map`].
#[derive(Clone)]
pub struct FlatMap<S, F> {
    source: S,
    build: F,
}

impl<S: fmt::Debug, F> fmt::Debug for FlatMap<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlatMap")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl<S: Strategy, T: Strategy, F: Fn(S::Value) -> T> Strategy for FlatMap<S, F> {
    type Value = T::Value;

    fn generate(&self, choices: &mut Choices) -> Result<T::Value, Invalid> {
        let outer = self.source.generate(choices)?;
        (self.build)(outer).generate(choices)
    }
}

/// A weighted choice between strategies of one value type. The first
/// alternative is the shrink target. Build one with [`crate::prop_oneof!`].
pub struct Union<T> {
    weights: Vec<u32>,
    arms: Vec<BoxedStrategy<T>>,
}

impl<T> Clone for Union<T> {
    fn clone(&self) -> Self {
        Self {
            weights: self.weights.clone(),
            arms: self.arms.clone(),
        }
    }
}

impl<T> fmt::Debug for Union<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Union")
            .field("weights", &self.weights)
            .finish_non_exhaustive()
    }
}

impl<T: fmt::Debug + 'static> Union<T> {
    /// Equally weighted alternatives.
    pub fn new<S>(arms: impl IntoIterator<Item = S>) -> Self
    where
        S: Strategy<Value = T> + 'static,
    {
        Self::new_weighted(arms.into_iter().map(|arm| (1, arm)).collect())
    }

    /// Alternatives chosen in proportion to their weights.
    pub fn new_weighted<S>(arms: Vec<(u32, S)>) -> Self
    where
        S: Strategy<Value = T> + 'static,
    {
        assert!(!arms.is_empty(), "a union needs at least one alternative");
        assert!(
            arms.iter().any(|(weight, _)| *weight > 0),
            "a union needs a positive total weight"
        );
        let (weights, arms) = arms
            .into_iter()
            .map(|(weight, arm)| (weight, arm.boxed()))
            .unzip();
        Self { weights, arms }
    }
}

impl<T: fmt::Debug> Strategy for Union<T> {
    type Value = T;

    fn generate(&self, choices: &mut Choices) -> Result<T, Invalid> {
        let index = choices.weighted_index(&self.weights)?;
        self.arms[index].generate(choices)
    }
}

macro_rules! tuple_strategy {
    ($(($($name:ident $index:tt),+))+) => {$(
        impl<$($name: Strategy),+> Strategy for ($($name,)+) {
            type Value = ($($name::Value,)+);

            fn generate(&self, choices: &mut Choices) -> Result<Self::Value, Invalid> {
                Ok(($(self.$index.generate(choices)?,)+))
            }
        }
    )+};
}

tuple_strategy! {
    (A 0)
    (A 0, B 1)
    (A 0, B 1, C 2)
    (A 0, B 1, C 2, D 3)
    (A 0, B 1, C 2, D 3, E 4)
    (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11)
}

macro_rules! range_strategy {
    ($($t:ty)+) => {$(
        impl Strategy for Range<$t> {
            type Value = $t;

            /// Uniform over the range; shrinks towards `start`.
            fn generate(&self, choices: &mut Choices) -> Result<$t, Invalid> {
                assert!(self.start < self.end, "the empty range {self:?} has no values");
                let span = (i128::from(self.end) - i128::from(self.start) - 1) as u64;
                let offset = choices.draw(span)?;
                Ok((i128::from(self.start) + i128::from(offset)) as $t)
            }
        }

        impl Strategy for RangeInclusive<$t> {
            type Value = $t;

            /// Uniform over the range; shrinks towards `start`.
            fn generate(&self, choices: &mut Choices) -> Result<$t, Invalid> {
                let (start, end) = (*self.start(), *self.end());
                assert!(start <= end, "the empty range {self:?} has no values");
                let span = (i128::from(end) - i128::from(start)) as u64;
                let offset = choices.draw(span)?;
                Ok((i128::from(start) + i128::from(offset)) as $t)
            }
        }
    )+};
}

range_strategy!(u8 u16 u32 u64 i8 i16 i32 i64);

macro_rules! size_range_strategy {
    ($($t:ty => $wide:ty)+) => {$(
        impl Strategy for Range<$t> {
            type Value = $t;

            /// Uniform over the range; shrinks towards `start`.
            fn generate(&self, choices: &mut Choices) -> Result<$t, Invalid> {
                ((self.start as $wide)..(self.end as $wide))
                    .generate(choices)
                    .map(|value| value as $t)
            }
        }

        impl Strategy for RangeInclusive<$t> {
            type Value = $t;

            /// Uniform over the range; shrinks towards `start`.
            fn generate(&self, choices: &mut Choices) -> Result<$t, Invalid> {
                ((*self.start() as $wide)..=(*self.end() as $wide))
                    .generate(choices)
                    .map(|value| value as $t)
            }
        }
    )+};
}

size_range_strategy!(usize => u64 isize => i64);
