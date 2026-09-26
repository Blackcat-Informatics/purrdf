// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Choosing from fixed values: [`select`] and [`Index`].

use std::fmt;

use super::choices::{Choices, Invalid};
use super::strategy::Strategy;

/// One of `values`, uniformly, shrinking to the first.
pub fn select<T: Clone + fmt::Debug>(values: impl Into<Vec<T>>) -> Select<T> {
    let values = values.into();
    assert!(!values.is_empty(), "select needs at least one value");
    Select { values }
}

/// See [`select`].
#[derive(Debug, Clone)]
pub struct Select<T> {
    values: Vec<T>,
}

impl<T: Clone + fmt::Debug> Strategy for Select<T> {
    type Value = T;

    fn generate(&self, choices: &mut Choices) -> Result<T, Invalid> {
        let index = choices.draw((self.values.len() - 1) as u64)?;
        Ok(self.values[index as usize].clone())
    }
}

/// A position in a collection whose length is known only later: a uniform
/// fraction of the length, shrinking to the first element. Generate one with
/// `any::<Index>()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index(u64);

impl Index {
    /// The position this index denotes in a collection of `len` elements.
    pub fn index(self, len: usize) -> usize {
        assert!(len > 0, "an index into an empty collection");
        ((u128::from(self.0) * len as u128) >> 64) as usize
    }

    /// The element this index denotes in `slice`.
    pub fn get<T>(self, slice: &[T]) -> &T {
        &slice[self.index(slice.len())]
    }
}

/// The strategy behind `any::<Index>()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexStrategy;

impl Strategy for IndexStrategy {
    type Value = Index;

    fn generate(&self, choices: &mut Choices) -> Result<Index, Invalid> {
        Ok(Index(choices.draw(u64::MAX)?))
    }
}
