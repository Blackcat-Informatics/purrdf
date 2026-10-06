// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical native rows shared by file encoders and relational hosts.

use crate::column::Int64Rows;
use crate::parquet::{ColumnValues, TableData, write_table};
use crate::{ColumnarError, ColumnarWrite, Compression, ParquetFiles, Table};
use purrdf_core::LossLedger;

/// A cell in the native v1 five-table projection.
///
/// BYTE_ARRAY cells borrow their bytes from the projection. The column's
/// [`crate::ColumnSchema::utf8`] flag distinguishes text from blob payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionCell<'a> {
    /// An absent optional value.
    Null,
    /// An exact signed integer.
    Int64(i64),
    /// A borrowed text or binary value.
    Bytes(&'a [u8]),
}

/// A validated canonical five-table projection with its own dense term ids.
///
/// Ids follow canonical RDF term-value order, independently of a dataset's
/// interned ids or a GTS log's append-order ids. Blank scopes, graph declarations,
/// direction and RDF 1.2 side tables are carried by the native v1 schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnarProjection {
    tables: [TableData; 5],
}

impl ColumnarProjection {
    pub(crate) const fn from_tables(tables: [TableData; 5]) -> Self {
        Self { tables }
    }

    fn table(&self, table: Table) -> &TableData {
        &self.tables[crate::files::table_index(table)]
    }

    /// The number of rows in one table.
    #[must_use]
    pub fn row_count(&self, table: Table) -> usize {
        self.table(table).row_count
    }

    /// Visit rows in canonical order, with cells in native schema-column order.
    ///
    /// One small cell vector is allocated per yielded row. String and blob data
    /// stay borrowed; no per-cell string or payload copy occurs here.
    #[must_use]
    pub fn rows(&self, table: Table) -> ProjectionRows<'_> {
        let data = self.table(table);
        ProjectionRows {
            remaining: data.row_count,
            columns: data
                .columns
                .iter()
                .map(|column| match column {
                    ColumnValues::Int64(values) => ColumnRows::Int64(values.rows()),
                    ColumnValues::ByteArray(values) => ColumnRows::Bytes(values.iter()),
                })
                .collect(),
        }
    }

    /// Encode the projection using the unchanged native v1 Parquet writer.
    ///
    /// # Errors
    ///
    /// Returns an error if a Parquet encoding bound is exceeded.
    pub fn to_parquet(&self, compression: Compression) -> Result<ColumnarWrite, ColumnarError> {
        let mut encoded = Vec::with_capacity(Table::ALL.len());
        for table in &self.tables {
            encoded.push(write_table(table, compression)?);
        }
        let files: [Vec<u8>; 5] = encoded.try_into().map_err(|_| {
            ColumnarError::malformed("table set", "writer did not produce exactly five files")
        })?;
        Ok(ColumnarWrite {
            files: ParquetFiles::from_array(files),
            losses: LossLedger::default(),
        })
    }
}

#[derive(Debug)]
enum ColumnRows<'a> {
    Int64(Int64Rows<'a>),
    Bytes(std::slice::Iter<'a, Option<Vec<u8>>>),
}

impl<'a> ColumnRows<'a> {
    fn next_cell(&mut self) -> ProjectionCell<'a> {
        match self {
            Self::Int64(rows) => rows
                .next()
                .flatten()
                .map_or(ProjectionCell::Null, ProjectionCell::Int64),
            Self::Bytes(rows) => rows
                .next()
                .and_then(Option::as_deref)
                .map_or(ProjectionCell::Null, ProjectionCell::Bytes),
        }
    }
}

/// A borrowed traversal of a canonical table's rows.
#[derive(Debug)]
pub struct ProjectionRows<'a> {
    columns: Vec<ColumnRows<'a>>,
    remaining: usize,
}

impl<'a> Iterator for ProjectionRows<'a> {
    type Item = Vec<ProjectionCell<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(self.columns.iter_mut().map(ColumnRows::next_cell).collect())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for ProjectionRows<'_> {}
impl std::iter::FusedIterator for ProjectionRows<'_> {}
