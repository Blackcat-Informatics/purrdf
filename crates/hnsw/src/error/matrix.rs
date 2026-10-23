// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native matrix-shape and graph-connectivity refusals.

/// The original operands of a matrix or connectivity refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatrixRefusal {
    /// No matrix rows.
    ZeroRows {
        /// Original column count.
        dimensions: usize,
    },
    /// No matrix dimensions.
    ZeroDimensions {
        /// Original row count.
        rows: usize,
    },
    /// No graph entry point exists.
    NoEntryPoint {
        /// Number of candidate rows.
        rows: usize,
    },
    /// Connectivity repair exhausted its pass bound.
    Connectivity {
        /// Number of graph nodes.
        rows: usize,
        /// Layer-zero degree bound.
        bound: usize,
        /// Number of completed repair passes.
        passes: usize,
    },
    /// Every reachable host is at its degree bound.
    Unattachable {
        /// Unreachable row.
        row: usize,
        /// Layer-zero degree bound.
        bound: usize,
    },
    /// An assembled matrix has an empty axis.
    EmptyAxis {
        /// Original row count.
        rows: usize,
        /// Original column count.
        dimensions: usize,
    },
    /// Supplied scalar count disagrees with the matrix shape.
    ValueCount {
        /// Original row count.
        rows: usize,
        /// Original column count.
        dimensions: usize,
        /// Scalar count implied by the shape.
        expected: usize,
        /// Number of supplied scalars.
        actual: usize,
    },
    /// A row disagrees with the first row's width.
    Ragged {
        /// First row with a differing width.
        row: usize,
        /// Original width of that row.
        actual: usize,
        /// Width of the first row.
        expected: usize,
    },
    /// Decoded components disagree with the effective dimension.
    RowWidth {
        /// Decoded row index.
        row: usize,
        /// Number of decoded components.
        actual: usize,
        /// Declared effective dimension.
        expected: usize,
    },
    /// Query components disagree with the index dimension.
    QueryWidth {
        /// Original query width.
        actual: usize,
        /// Index vector dimension.
        expected: usize,
    },
}

impl purrdf_hash::error::StructuredError for MatrixRefusal {
    purrdf_hash::error_visit! {
        ZeroRows { dimensions } => "hnsw-matrix-zero-rows" { "dimensions" => Unsigned(*dimensions as u128) },
        ZeroDimensions { rows } => "hnsw-matrix-zero-dimensions" { "rows" => Unsigned(*rows as u128) },
        NoEntryPoint { rows } => "hnsw-matrix-no-entry-point" { "rows" => Unsigned(*rows as u128) },
        Connectivity { rows, bound, passes } => "hnsw-matrix-connectivity" {
            "rows" => Unsigned(*rows as u128), "bound" => Unsigned(*bound as u128), "passes" => Unsigned(*passes as u128),
        },
        Unattachable { row, bound } => "hnsw-matrix-unattachable" { "row" => Unsigned(*row as u128), "bound" => Unsigned(*bound as u128) },
        EmptyAxis { rows, dimensions } => "hnsw-matrix-empty-axis" { "rows" => Unsigned(*rows as u128), "dimensions" => Unsigned(*dimensions as u128) },
        ValueCount { rows, dimensions, expected, actual } => "hnsw-matrix-value-count" {
            "rows" => Unsigned(*rows as u128), "dimensions" => Unsigned(*dimensions as u128), "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128),
        },
        Ragged { row, actual, expected } => "hnsw-matrix-ragged" { "row" => Unsigned(*row as u128), "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
        RowWidth { row, actual, expected } => "hnsw-matrix-row-width" { "row" => Unsigned(*row as u128), "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
        QueryWidth { actual, expected } => "hnsw-matrix-query-width" { "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
    }
}

impl core::fmt::Display for MatrixRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroRows { dimensions: _ } => f.write_str("the matrix has zero rows"),
            Self::ZeroDimensions { rows: _ } => f.write_str("the matrix has zero dimensions"),
            Self::NoEntryPoint { rows: _ } => f.write_str("a matrix with no rows has no entry point"),
            Self::Connectivity { rows: _, bound: _, passes: _ } => f.write_str("connectivity repair did not converge; the degree bound admits no spanning structure over this corpus"),
            Self::Unattachable { row, bound } => write!(f, "row {row} cannot be reattached: every reachable node has exhausted its degree bound of {bound}"),
            Self::EmptyAxis { rows, dimensions } => write!(f, "a {rows}x{dimensions} matrix has an empty axis"),
            Self::ValueCount { rows, dimensions, expected, actual } => write!(f, "a {rows}x{dimensions} matrix holds {expected} values, but {actual} were supplied"),
            Self::Ragged { row, actual, expected } => write!(f, "row {row} holds {actual} values while row 0 holds {expected}; the matrix would be ragged"),
            Self::RowWidth { row, actual, expected } => write!(f, "row {row} decoded {actual} component(s); the effective dimension is {expected}"),
            Self::QueryWidth { actual, expected } => write!(f, "the query has {actual} component(s) but this index ranks {expected}-dimensional vectors"),
        }
    }
}

impl std::error::Error for MatrixRefusal {}
