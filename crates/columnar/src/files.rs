// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fixed in-memory set of Parquet files.

use crate::schema::Table;

/// Exactly one Parquet file for each member of [`Table::ALL`].
///
/// The array form uses [`Table::ALL`] order. Keeping the set closed prevents a
/// caller from accidentally omitting an empty table, which would make the
/// five-table dataset projection incomplete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParquetFiles {
    files: [Vec<u8>; 5],
}

impl ParquetFiles {
    /// Construct the complete file set in [`Table::ALL`] order.
    #[must_use]
    pub fn from_array(files: [Vec<u8>; 5]) -> Self {
        Self { files }
    }

    /// Borrow one table's Parquet bytes.
    #[must_use]
    pub fn get(&self, table: Table) -> &[u8] {
        &self.files[table_index(table)]
    }

    /// Iterate every `(table, bytes)` pair in canonical dependency order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (Table, &[u8])> {
        Table::ALL
            .into_iter()
            .zip(self.files.iter().map(Vec::as_slice))
    }

    /// Consume the set as an array in [`Table::ALL`] order.
    #[must_use]
    pub fn into_array(self) -> [Vec<u8>; 5] {
        self.files
    }
}

const fn table_index(table: Table) -> usize {
    match table {
        Table::Terms => 0,
        Table::Quads => 1,
        Table::Reifiers => 2,
        Table::Annotations => 3,
        Table::Blobs => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_and_table_views_share_canonical_order() {
        let files = ParquetFiles::from_array([vec![0], vec![1], vec![2], vec![3], vec![4]]);
        for (index, (table, bytes)) in files.iter().enumerate() {
            assert_eq!(table, Table::ALL[index]);
            assert_eq!(bytes, [index as u8]);
            assert_eq!(files.get(table), bytes);
        }
    }
}
