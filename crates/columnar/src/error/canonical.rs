// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original canonical-row shape operands.

use crate::{PhysicalType, Repetition};

/// A canonical row cannot satisfy its fixed column schema.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum CanonicalReason {
    /// A cell's physical kind or presence violates the column schema.
    Cell {
        /// Original column name.
        column: &'static str,
        /// Original physical kind; absent for a null cell.
        actual: Option<PhysicalType>,
        /// Required physical kind.
        expected: PhysicalType,
        /// Required column cardinality.
        repetition: Repetition,
    },
    /// A row carries a different number of cells from its table schema.
    ColumnCount {
        /// Original cell count.
        actual: usize,
        /// Required cell count.
        expected: usize,
    },
}

impl purrdf_hash::error::StructuredError for CanonicalReason {
    purrdf_hash::error_visit! {
        Cell { column, actual, expected, repetition } => "columnar-canonical-cell" {
            "column" => Code(column),
            "actual" => Value(actual.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |kind| purrdf_hash::error::ErrorValue::Record(kind))),
            "expected" => Record(expected),
            "repetition" => Code(match repetition { Repetition::Required => "required", Repetition::Optional => "optional" }),
        },
        ColumnCount { actual, expected } => "columnar-canonical-column-count" {
            "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128),
        },
    }
}

impl core::fmt::Display for CanonicalReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Cell { .. } => "cell type or required presence",
            Self::ColumnCount { .. } => "column count",
        })
    }
}
impl std::error::Error for CanonicalReason {}
