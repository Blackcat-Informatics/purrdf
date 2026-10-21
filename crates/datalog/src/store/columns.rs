// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original integer rows in dense columns, independent of RDF term spelling.

use crate::dictionary::TermIdentity;
use crate::id::RowId;
use purrdf_lex::allocation::StorageError;

/// A column buffer of caller-owned identities and row handles.
///
/// Every row has exactly `N` cells. Neither cells nor handles are translated,
/// interned or rendered. Zero-width rows retain their handles as well. All
/// duplicate rows and handles remain observable. The relation owning the buffer
/// decides membership and assigns handles.
#[derive(Debug, Clone)]
pub struct RowColumns<I: TermIdentity, const N: usize, R: Copy = RowId> {
    columns: [Vec<I>; N],
    rows: Vec<R>,
}

impl<I: TermIdentity, const N: usize, R: Copy> Default for RowColumns<I, N, R> {
    fn default() -> Self {
        Self {
            columns: core::array::from_fn(|_| Vec::new()),
            rows: Vec::new(),
        }
    }
}

impl<I: TermIdentity, const N: usize, R: Copy> RowColumns<I, N, R> {
    /// The number of complete rows, including zero-width rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the buffer holds no row.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Borrow one complete cell column in insertion order.
    ///
    /// # Panics
    ///
    /// Panics if `position` is not below `N`.
    pub fn column(&self, position: usize) -> &[I] {
        &self.columns[position]
    }

    /// The original handles, in insertion order and parallel to every column.
    pub fn handles(&self) -> &[R] {
        &self.rows
    }

    /// Replace the handle of an existing physical row, preserving its cells.
    ///
    /// Returns the old handle, or `None` when `position` is outside this buffer.
    /// Membership and handle identity remain the owning relation's responsibility.
    pub fn replace_handle(&mut self, position: usize, handle: R) -> Option<R> {
        self.rows
            .get_mut(position)
            .map(|row| core::mem::replace(row, handle))
    }

    /// The complete row at `position`, or `None` beyond the buffer.
    pub fn row(&self, position: usize) -> Option<([I; N], R)> {
        let handle = *self.rows.get(position)?;
        Some((
            core::array::from_fn(|column| self.columns[column][position]),
            handle,
        ))
    }

    /// Every complete row in insertion order, with its original handle.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ([I; N], R)> + DoubleEndedIterator + '_ {
        (0..self.len()).map(|position| {
            self.row(position)
                .expect("an occupied column position has its complete row")
        })
    }

    /// Reserve every column before publishing any part of an append.
    ///
    /// Existing row contents and lengths are unchanged on failure. A preceding
    /// successful reservation may have increased a column's capacity.
    ///
    /// # Errors
    ///
    /// Returns a typed size or allocation refusal.
    pub fn reserve_rows(&mut self, additional: usize) -> Result<(), StorageError> {
        self.len()
            .checked_add(additional)
            .ok_or(StorageError::SizeOverflow)?;
        for column in &mut self.columns {
            column
                .try_reserve(additional)
                .map_err(|_| StorageError::AllocationFailed)?;
        }
        self.rows
            .try_reserve(additional)
            .map_err(|_| StorageError::AllocationFailed)
    }

    /// Append one complete row after reserving all of its cells and its handle.
    ///
    /// # Errors
    ///
    /// Returns the original size or allocation refusal, without publishing a row.
    pub fn try_push(&mut self, cells: [I; N], handle: R) -> Result<(), StorageError> {
        self.reserve_rows(1)?;
        self.push_reserved(cells, handle);
        Ok(())
    }

    /// Publish a row whose complete allocation was already admitted by its owner.
    pub(crate) fn push_reserved(&mut self, cells: [I; N], handle: R) {
        let position = self.len();
        assert!(
            self.rows.capacity() > position
                && self
                    .columns
                    .iter()
                    .all(|column| column.capacity() > position),
            "every row column must be reserved before publication"
        );
        for (column, cell) in self.columns.iter_mut().zip(cells) {
            column.push(cell);
        }
        self.rows.push(handle);
    }

    /// Remove all rows while retaining the allocated column capacities.
    pub fn clear(&mut self) {
        for column in &mut self.columns {
            column.clear();
        }
        self.rows.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_integer_widths_keep_original_cells_and_handles() {
        fn check<const N: usize>() {
            let mut rows = RowColumns::<u32, N, u64>::default();
            let first = core::array::from_fn(|column| u32::try_from(column).unwrap());
            let second = first.map(|value| u32::MAX - value);
            rows.try_push(first, 9001).unwrap();
            rows.try_push(second, 3).unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows.handles(), [9001, 3]);
            assert_eq!(
                rows.iter().collect::<Vec<_>>(),
                [(first, 9001), (second, 3)]
            );
            assert_eq!(rows.iter().next_back(), Some((second, 3)));
            assert_eq!(rows.row(2), None);
            for column in 0..N {
                assert_eq!(rows.column(column), [first[column], second[column]]);
            }
            rows.clear();
            assert!(rows.is_empty());
            rows.try_push(second, 17).unwrap();
            assert_eq!(rows.row(0), Some((second, 17)));
        }
        check::<0>();
        check::<1>();
        check::<2>();
        check::<4>();
        check::<5>();
        check::<6>();
        check::<65>();
    }

    #[test]
    fn size_refusal_keeps_the_original_complete_rows() {
        let mut rows = RowColumns::<u32, 6, u32>::default();
        let cells = [2, 3, 5, 7, 11, 13];
        rows.try_push(cells, 42).unwrap();
        assert!(matches!(
            rows.reserve_rows(usize::MAX),
            Err(StorageError::SizeOverflow)
        ));
        assert_eq!(rows.row(0), Some((cells, 42)));
        assert_eq!(rows.len(), 1);
    }
}
