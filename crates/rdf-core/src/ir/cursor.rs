// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compact probe cursors for the layered views (composite and delta).
//!
//! A layered view's probe chooses among several sources or carriers. Composing
//! that choice from `chain`, `Option::into_iter().flat_map(..)` and friends
//! inlines every inactive branch into the cursor, and each `flat_map` reserves a
//! front AND a back inner cursor for double-ended iteration no probe asks for, so
//! nested layers grow multiplicatively. These adapters keep a cursor as large as
//! its one active branch, and building one never allocates.

/// `outer.flat_map(map)` walked forward only: it holds one inner cursor, where
/// `FlatMap` reserves a front and a back one for double-ended iteration that a
/// probe never asks for.
pub(crate) const fn forward_flat_map<I, U, F>(outer: I, map: F) -> ForwardFlatMap<I, U, F>
where
    I: Iterator,
    U: Iterator,
    F: FnMut(I::Item) -> U,
{
    ForwardFlatMap {
        outer,
        map,
        inner: None,
    }
}

pub(crate) struct ForwardFlatMap<I, U, F> {
    outer: I,
    map: F,
    inner: Option<U>,
}

impl<I, U, F> Iterator for ForwardFlatMap<I, U, F>
where
    I: Iterator,
    U: Iterator,
    F: FnMut(I::Item) -> U,
{
    type Item = U::Item;

    #[inline]
    fn next(&mut self) -> Option<U::Item> {
        loop {
            if let Some(row) = self.inner.as_mut().and_then(Iterator::next) {
                return Some(row);
            }
            let Some(next) = self.outer.next() else {
                self.inner = None;
                return None;
            };
            self.inner = Some((self.map)(next));
        }
    }

    // The live inner cursor bounds the rows below; the upper bound is known only
    // once the outer cursor has nothing left to map.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self
            .inner
            .as_ref()
            .map_or((0, Some(0)), Iterator::size_hint);
        match self.outer.size_hint() {
            (0, Some(0)) => (low, high),
            _ => (low, None),
        }
    }

    #[inline]
    fn fold<Acc, G>(self, init: Acc, mut fold: G) -> Acc
    where
        G: FnMut(Acc, U::Item) -> Acc,
    {
        let Self {
            outer,
            mut map,
            inner,
        } = self;
        let acc = match inner {
            Some(inner) => inner.fold(init, &mut fold),
            None => init,
        };
        outer.fold(acc, |acc, item| map(item).fold(acc, &mut fold))
    }
}

// Once a fused outer cursor is spent, `next` clears the inner cursor and
// asks the outer again, so every later call returns `None` too.
impl<I, U, F> std::iter::FusedIterator for ForwardFlatMap<I, U, F>
where
    I: std::iter::FusedIterator,
    U: Iterator,
    F: FnMut(I::Item) -> U,
{
}

/// `rows` when present, else nothing, holding at most the one cursor.
pub(crate) fn optional<I: Iterator>(rows: Option<I>) -> impl Iterator<Item = I::Item> {
    type Unused<T> = std::iter::Empty<T>;
    match rows {
        Some(rows) => Cursor::First(rows),
        None => Cursor::<I, Unused<I::Item>, Unused<I::Item>, Unused<I::Item>>::Empty,
    }
}

/// The one active branch of a probe, chosen before any row is pulled.
///
/// Each arm holds a single carrier's (or statement table's) iterator inline, so
/// a cursor is as large as its largest arm, not the sum of every inactive
/// branch, and selecting an arm never allocates.
pub(crate) enum Cursor<A, B, C, D> {
    Empty,
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}

impl<T, A, B, C, D> Iterator for Cursor<A, B, C, D>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
    D: Iterator<Item = T>,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Empty => None,
            Self::First(rows) => rows.next(),
            Self::Second(rows) => rows.next(),
            Self::Third(rows) => rows.next(),
            Self::Fourth(rows) => rows.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Empty => (0, Some(0)),
            Self::First(rows) => rows.size_hint(),
            Self::Second(rows) => rows.size_hint(),
            Self::Third(rows) => rows.size_hint(),
            Self::Fourth(rows) => rows.size_hint(),
        }
    }

    // Internal iteration (`count`, `for_each`, `sum`, ...) reaches the active
    // arm's own `fold` rather than a `next` loop through this dispatch.
    #[inline]
    fn fold<Acc, F>(self, init: Acc, fold: F) -> Acc
    where
        F: FnMut(Acc, T) -> Acc,
    {
        match self {
            Self::Empty => init,
            Self::First(rows) => rows.fold(init, fold),
            Self::Second(rows) => rows.fold(init, fold),
            Self::Third(rows) => rows.fold(init, fold),
            Self::Fourth(rows) => rows.fold(init, fold),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{forward_flat_map, optional};

    #[test]
    fn forward_flat_map_matches_flat_map_rows_and_bounds() {
        let groups = || vec![vec![1, 2, 3], vec![], vec![4]].into_iter();
        let mut forward = forward_flat_map(groups(), Vec::into_iter);
        let mut flat = groups().flat_map(Vec::into_iter);
        loop {
            assert_eq!(forward.size_hint(), flat.size_hint());
            let row = forward.next();
            assert_eq!(row, flat.next());
            if row.is_none() {
                break;
            }
        }
        // Spent and fused: it stays spent, with an exact empty bound.
        assert_eq!(forward.next(), None);
        assert_eq!(forward.size_hint(), (0, Some(0)));
        assert_eq!(
            forward_flat_map(groups(), Vec::into_iter).fold(Vec::new(), |mut rows, row| {
                rows.push(row);
                rows
            }),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn forward_flat_map_bounds_the_live_inner_cursor() {
        let mut rows = forward_flat_map(std::iter::once(vec![1, 2, 3]), Vec::into_iter);
        // Before the outer cursor is mapped nothing is known above zero.
        assert_eq!(rows.size_hint(), (0, None));
        assert_eq!(rows.next(), Some(1));
        // The outer cursor is spent, so the inner cursor's bound is exact.
        assert_eq!(rows.size_hint(), (2, Some(2)));
    }

    #[test]
    fn optional_holds_its_rows_or_none() {
        assert_eq!(optional(Some([1, 2].into_iter())).size_hint(), (2, Some(2)));
        assert_eq!(
            optional(Some([1, 2].into_iter())).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(
            optional(None::<std::array::IntoIter<i32, 2>>).size_hint(),
            (0, Some(0))
        );
        assert_eq!(optional(None::<std::array::IntoIter<i32, 2>>).next(), None);
    }
}
