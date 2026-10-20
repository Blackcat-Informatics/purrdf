// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-selected integer-column indexes over immutable row buffers.

use super::RowColumns;
use crate::dictionary::TermIdentity;
use crate::id::RowId;
use core::cmp::Ordering;
use core::fmt;
use purrdf_lex::allocation::StorageError;

/// An invalid column index declaration, query prefix, or allocation refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowIndexError {
    /// A declared coordinate does not exist in the original rows.
    ColumnOutOfBounds {
        /// Original caller-selected coordinate.
        column: usize,
        /// Number of cells in each original row.
        width: usize,
    },
    /// A column was named twice in the same index declaration.
    DuplicateColumn {
        /// Repeated caller-selected coordinate.
        column: usize,
    },
    /// A query supplies more key cells than the index owns.
    PrefixTooLong {
        /// Number of supplied cells.
        provided: usize,
        /// Number of indexed columns.
        indexed: usize,
    },
    /// Original native storage refusal.
    Storage(StorageError),
}

purrdf_lex::variant_from!(RowIndexError { Storage(StorageError) });

impl purrdf_hash::error::StructuredError for RowIndexError {
    purrdf_hash::error_visit! {
        ColumnOutOfBounds { column, width } => "datalog-row-index-column" { "column" => Unsigned(*column as u128), "width" => Unsigned(*width as u128) },
        DuplicateColumn { column } => "datalog-row-index-duplicate" { "column" => Unsigned(*column as u128) },
        PrefixTooLong { provided, indexed } => "datalog-row-index-prefix" { "provided" => Unsigned(*provided as u128), "indexed" => Unsigned(*indexed as u128) },
        Storage(error) => "datalog-row-index-storage" { "cause" => Cause(error) },
    }
}

impl fmt::Display for RowIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnOutOfBounds { column, width } => {
                write!(
                    formatter,
                    "row index column {column} is outside row width {width}"
                )
            }
            Self::DuplicateColumn { column } => {
                write!(formatter, "row index repeats column {column}")
            }
            Self::PrefixTooLong { provided, indexed } => write!(
                formatter,
                "row index has {indexed} key columns, but the query supplies {provided} cells"
            ),
            Self::Storage(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RowIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

/// Sort original row positions by the requested integer columns. Equal keys keep
/// insertion order through an explicit position tie-break, even for an empty key.
pub(super) fn sort_row_positions<I: TermIdentity>(
    positions: &mut [usize],
    columns: &[&[I]],
    order: &[usize],
) {
    positions.sort_unstable_by(|&left, &right| {
        order
            .iter()
            .map(|&column| columns[column][left].cmp(&columns[column][right]))
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or_else(|| left.cmp(&right))
    });
}

/// A sorted permutation over caller-selected columns of the original integer rows.
///
/// The borrow pins the rows for the index's lifetime. Keys compare original
/// integer identities; no dictionary, lexical rendering, or copied cell table
/// participates. Queries return every matching original row, including duplicates,
/// in key order followed by insertion order for equal keys.
#[derive(Debug)]
pub struct RowIndex<'a, I: TermIdentity, const N: usize, R: Copy = RowId> {
    rows: &'a RowColumns<I, N, R>,
    order: Vec<usize>,
    positions: Vec<usize>,
}

impl<'a, I: TermIdentity, const N: usize, R: Copy> RowIndex<'a, I, N, R> {
    /// Index exactly `columns`, in the caller's declared key order.
    ///
    /// An empty key is valid and retains insertion order. Every non-empty key
    /// coordinate must exist and occur only once.
    ///
    /// # Errors
    ///
    /// Returns the original invalid coordinate or a typed allocation refusal.
    pub fn new(rows: &'a RowColumns<I, N, R>, columns: &[usize]) -> Result<Self, RowIndexError> {
        for (position, &column) in columns.iter().enumerate() {
            if column >= N {
                return Err(RowIndexError::ColumnOutOfBounds { column, width: N });
            }
            if columns[..position].contains(&column) {
                return Err(RowIndexError::DuplicateColumn { column });
            }
        }
        let mut order = Vec::new();
        order
            .try_reserve_exact(columns.len())
            .map_err(|_| StorageError::AllocationFailed)?;
        order.extend_from_slice(columns);
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(rows.len())
            .map_err(|_| StorageError::AllocationFailed)?;
        positions.extend(0..rows.len());
        let columns: [&[I]; N] = core::array::from_fn(|column| rows.column(column));
        sort_row_positions(&mut positions, &columns, &order);
        Ok(Self {
            rows,
            order,
            positions,
        })
    }

    /// Original row positions matching a prefix of the declared key.
    ///
    /// Both bounds are binary searches over the existing permutation. A query
    /// neither enumerates unrelated rows nor constructs another index.
    ///
    /// # Errors
    ///
    /// Returns the supplied prefix length when it exceeds the declared key.
    pub fn positions(&self, prefix: &[I]) -> Result<&[usize], RowIndexError> {
        if prefix.len() > self.order.len() {
            return Err(RowIndexError::PrefixTooLong {
                provided: prefix.len(),
                indexed: self.order.len(),
            });
        }
        let lower = self
            .positions
            .partition_point(|&row| self.compare_prefix(row, prefix).is_lt());
        let upper = lower
            + self.positions[lower..]
                .partition_point(|&row| !self.compare_prefix(row, prefix).is_gt());
        Ok(&self.positions[lower..upper])
    }

    /// Original complete rows and handles matching a key prefix, without allocation.
    ///
    /// # Errors
    ///
    /// Returns the original oversized-prefix refusal.
    pub fn select<'s>(
        &'s self,
        prefix: &[I],
    ) -> Result<
        impl ExactSizeIterator<Item = ([I; N], R)> + DoubleEndedIterator + use<'s, 'a, I, N, R>,
        RowIndexError,
    > {
        Ok(self.positions(prefix)?.iter().map(|&position| {
            self.rows
                .row(position)
                .expect("an index position refers to its original complete row")
        }))
    }

    fn compare_prefix(&self, row: usize, prefix: &[I]) -> Ordering {
        for (&column, key) in self.order.iter().zip(prefix) {
            let ordering = self.rows.column(column)[row].cmp(key);
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_column_keys_select_original_rows_and_duplicate_handles() {
        let mut rows = RowColumns::<u32, 6, u32>::default();
        for (cells, handle) in [
            ([7, 2, 9, 4, 1, 8], 9001),
            ([7, 3, 9, 2, 4, 1], 3),
            ([2, 3, 9, 8, 5, 1], 42),
            ([7, 2, 9, 4, 1, 8], 9001),
        ] {
            rows.try_push(cells, handle).unwrap();
        }
        let index = RowIndex::new(&rows, &[2, 0, 5]).unwrap();
        assert_eq!(index.positions(&[]).unwrap(), [2, 1, 0, 3]);
        assert_eq!(index.positions(&[9, 7]).unwrap(), [1, 0, 3]);
        assert_eq!(index.positions(&[9, 7, 8]).unwrap(), [0, 3]);
        assert_eq!(index.positions(&[9, 6]).unwrap(), []);
        assert_eq!(
            index.select(&[9, 7, 8]).unwrap().collect::<Vec<_>>(),
            [([7, 2, 9, 4, 1, 8], 9001); 2]
        );
        let other = RowIndex::new(&rows, &[4, 1]).unwrap();
        assert_eq!(other.positions(&[]).unwrap(), [0, 3, 1, 2]);
        assert_eq!(other.select(&[4]).unwrap().next_back(), rows.row(1));
    }

    #[test]
    fn invalid_index_declarations_and_prefixes_are_typed_refusals() {
        let rows = RowColumns::<u32, 2, u32>::default();
        assert!(matches!(
            RowIndex::new(&rows, &[2]),
            Err(RowIndexError::ColumnOutOfBounds {
                column: 2,
                width: 2
            })
        ));
        assert!(matches!(
            RowIndex::new(&rows, &[1, 1]),
            Err(RowIndexError::DuplicateColumn { column: 1 })
        ));
        let index = RowIndex::new(&rows, &[1]).unwrap();
        assert_eq!(
            index.positions(&[2, 3]),
            Err(RowIndexError::PrefixTooLong {
                provided: 2,
                indexed: 1
            })
        );
        let empty = RowColumns::<u32, 0, u32>::default();
        assert_eq!(
            RowIndex::new(&empty, &[]).unwrap().positions(&[]).unwrap(),
            []
        );
    }
}
