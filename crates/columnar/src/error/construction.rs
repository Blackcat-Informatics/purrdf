// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original scalar operands from fixed-schema construction and encoding.

use core::fmt;

use crate::{PhysicalType, Table};

/// A native fixed-schema table construction or encoding refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConstructionReason {
    /// The table has a different number of columns.
    ColumnCount {
        /// Original table.
        table: Table,
        /// Original column count.
        actual: usize,
        /// Required column count.
        expected: usize,
    },
    /// A column has a different row count from its siblings.
    ColumnRows {
        /// Original table.
        table: Table,
        /// Original column name.
        column: &'static str,
        /// Original column row count.
        actual: usize,
        /// Required row count.
        expected: usize,
    },
    /// A column has a different physical type from its schema.
    ColumnType {
        /// Original table.
        table: Table,
        /// Original column name.
        column: &'static str,
        /// Original physical type.
        actual: PhysicalType,
        /// Required physical type.
        expected: PhysicalType,
    },
    /// A required column contains absent cells.
    RequiredColumn {
        /// Original table.
        table: Table,
        /// Original column name.
        column: &'static str,
        /// Actual number of null cells.
        nulls: usize,
    },
    /// Adding an uncompressed column size overflows the signed total.
    UncompressedTotalOverflow {
        /// Total before addition.
        total: i64,
        /// Original column size to add.
        size: i64,
    },
    /// The stored table row count disagrees with its columns.
    RowCount {
        /// Stored count.
        stored: usize,
        /// Count established by the columns.
        actual: usize,
    },
    /// Adding definitions and value bytes overflows the address space.
    PageSizeOverflow {
        /// Original definition-level byte count.
        definitions: usize,
        /// Original value byte count.
        values: usize,
    },
}

impl purrdf_hash::error::StructuredError for ConstructionReason {
    purrdf_hash::error_visit! {
        ColumnCount { table, actual, expected } => "columnar-parquet-column-count" {
            "table" => Code(table.name()), "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128),
        },
        ColumnRows { table, column, actual, expected } => "columnar-parquet-column-rows" {
            "table" => Code(table.name()), "column" => Code(column),
            "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128),
        },
        ColumnType { table, column, actual, expected } => "columnar-parquet-column-type" {
            "table" => Code(table.name()), "column" => Code(column),
            "actual" => Record(actual), "expected" => Record(expected),
        },
        RequiredColumn { table, column, nulls } => "columnar-parquet-required-column" {
            "table" => Code(table.name()), "column" => Code(column), "nulls" => Unsigned(*nulls as u128),
        },
        UncompressedTotalOverflow { total, size } => "columnar-parquet-uncompressed-total-overflow" {
            "total" => Signed(i128::from(*total)), "size" => Signed(i128::from(*size)),
        },
        RowCount { stored, actual } => "columnar-parquet-row-count" {
            "stored" => Unsigned(*stored as u128), "actual" => Unsigned(*actual as u128),
        },
        PageSizeOverflow { definitions, values } => "columnar-parquet-page-size-overflow" {
            "definitions" => ByteCount(*definitions as u128), "values" => ByteCount(*values as u128),
        },
    }
}

impl fmt::Display for ConstructionReason {
    /// Render the established diagnostic from its original scalar operands.
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnCount {
                table,
                actual,
                expected,
            } => write!(
                output,
                "{} has {actual} columns, expected {expected}",
                table.name()
            ),
            Self::ColumnRows {
                table,
                column,
                actual,
                expected,
            } => write!(
                output,
                "{}.{column} has {actual} rows, expected {expected}",
                table.name()
            ),
            Self::ColumnType {
                table,
                column,
                actual,
                expected,
            } => write!(
                output,
                "{}.{column} has physical type {actual:?}, expected {expected:?}",
                table.name()
            ),
            Self::RequiredColumn { table, column, .. } => write!(
                output,
                "required column {}.{column} contains null",
                table.name()
            ),
            Self::UncompressedTotalOverflow { .. } => {
                output.write_str("uncompressed byte total overflows i64")
            }
            Self::RowCount { stored, actual } => write!(
                output,
                "stored row count {stored} disagrees with columns {actual}"
            ),
            Self::PageSizeOverflow { .. } => output.write_str("length addition overflows usize"),
        }
    }
}

impl std::error::Error for ConstructionReason {}
