// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic physical operating targets for the closed five-table writer.

use crate::{ColumnarError, Compression, Table};

/// Standard split-block Bloom filter sizing for each admitted column chunk.
/// Filters are opt-in and never affect values or exact native reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloomOptions {
    /// Target bits per present row (duplicates may overestimate cardinality).
    pub bits_per_value: u32,
    /// Hard bitset cap, a multiple of 32 bytes, from 32 through 128 MiB.
    pub max_bytes: u32,
}

impl Default for BloomOptions {
    fn default() -> Self {
        Self {
            bits_per_value: 10,
            max_bytes: 64 * 1024,
        }
    }
}

impl BloomOptions {
    fn validate(self) -> Result<(), ColumnarError> {
        for (field, value, minimum, maximum) in [
            ("bloom.bits_per_value", self.bits_per_value, 1, 64),
            ("bloom.max_bytes", self.max_bytes, 32, 128 * 1024 * 1024),
        ] {
            if value < minimum || value > maximum {
                return Err(ColumnarError::InvalidOptions {
                    field,
                    value: u64::from(value),
                    minimum: u64::from(minimum),
                    maximum: u64::from(maximum),
                });
            }
        }
        if !self.max_bytes.is_multiple_of(32) {
            return Err(ColumnarError::InvalidOptions {
                field: "bloom.max_bytes (multiple of 32)",
                value: u64::from(self.max_bytes),
                minimum: 32,
                maximum: 128 * 1024 * 1024,
            });
        }
        Ok(())
    }
}

/// Physical encoding candidates admitted for one column chunk.
///
/// Candidates are measured on actual page data. PLAIN remains the deterministic
/// fallback when another encoding is inapplicable, exceeds its workspace or
/// dictionary limits, or does not reduce the encoded cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncodingPolicy {
    /// Compare PLAIN, applicable delta codecs and a bounded chunk dictionary.
    #[default]
    Adaptive,
    /// Emit PLAIN values only.
    Plain,
    /// Compare a bounded chunk dictionary with PLAIN.
    Dictionary,
    /// Compare applicable delta codecs with PLAIN.
    Delta,
}

/// An explicit physical override for one schema column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnOptions {
    /// Table containing the column.
    pub table: Table,
    /// Zero-based column ordinal in the table's fixed schema.
    pub column: usize,
    /// Codec override; absent means the write's default codec.
    pub compression: Option<Compression>,
    /// Candidate override; absent means the write's default policy.
    pub encoding: Option<EncodingPolicy>,
    /// Enable/size a filter for this column, including a binary column.
    pub bloom: Option<BloomOptions>,
}

/// Runtime physical layout choices; these do not change the RDF projection.
///
/// Byte targets measure canonical PLAIN values and page-local definition levels.
/// Page headers and footer metadata are separate storage. These are
/// operating targets, not rejection limits: one large value may occupy its own
/// larger page and group, subject to Parquet's signed 32-bit page fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOptions {
    /// Codec for every column unless a column-specific choice is supplied.
    pub compression: Compression,
    /// Default physical value-encoding policy.
    pub encoding: EncodingPolicy,
    /// Maximum distinct physical dictionary values; zero disables dictionaries.
    pub dictionary_max_entries: u32,
    /// Maximum dictionary ownership and replacement-allocation bytes, including
    /// payload/index capacities; zero disables dictionaries. Page/compression
    /// output buffers are separate owners, not part of this dictionary budget.
    pub dictionary_max_bytes: u64,
    /// Unique per-column overrides. Unknown or duplicate columns are refused
    /// before checked source preparation or the first output callback.
    pub columns: Vec<ColumnOptions>,
    /// Optional filters for integer and UTF-8 columns. Binary payloads are
    /// excluded unless a column override explicitly enables their filter.
    pub bloom: Option<BloomOptions>,
    /// Preferred maximum uncompressed bytes in one column's data page.
    pub page_bytes: u64,
    /// Preferred maximum rows in one data page, including nulls.
    pub page_rows: u32,
    /// Preferred aggregate uncompressed column bytes in one row group.
    pub row_group_bytes: u64,
    /// Preferred rows in one row group.
    pub row_group_rows: u64,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            compression: Compression::Uncompressed,
            encoding: EncodingPolicy::Adaptive,
            dictionary_max_entries: 4096,
            dictionary_max_bytes: 1024 * 1024,
            columns: Vec::new(),
            bloom: None,
            page_bytes: 1024 * 1024,
            page_rows: 65_536,
            row_group_bytes: 128 * 1024 * 1024,
            row_group_rows: 1_048_576,
        }
    }
}

impl WriteOptions {
    pub(crate) fn validate(&self) -> Result<(), ColumnarError> {
        for (field, value, minimum, maximum) in [
            ("page_bytes", self.page_bytes, 1, i32::MAX as u64),
            ("page_rows", u64::from(self.page_rows), 1, i32::MAX as u64),
            ("row_group_bytes", self.row_group_bytes, 1, i64::MAX as u64),
            ("row_group_rows", self.row_group_rows, 1, i64::MAX as u64),
            (
                "dictionary_max_entries",
                u64::from(self.dictionary_max_entries),
                0,
                i32::MAX as u64,
            ),
            (
                "dictionary_max_bytes",
                self.dictionary_max_bytes,
                0,
                i32::MAX as u64,
            ),
        ] {
            if value < minimum || value > maximum {
                return Err(ColumnarError::InvalidOptions {
                    field,
                    value,
                    minimum,
                    maximum,
                });
            }
        }
        self.compression.validate()?;
        if let Some(bloom) = self.bloom {
            bloom.validate()?;
        }
        for (position, column) in self.columns.iter().enumerate() {
            if column.column >= column.table.schema().columns.len() {
                return Err(ColumnarError::InvalidColumn {
                    table: column.table,
                    column: column.column,
                    columns: column.table.schema().columns.len(),
                });
            }
            if self.columns[..position]
                .iter()
                .any(|previous| previous.table == column.table && previous.column == column.column)
            {
                return Err(ColumnarError::DuplicateColumn {
                    table: column.table,
                    column: column.column,
                });
            }
            if let Some(compression) = column.compression {
                compression.validate()?;
            }
            if let Some(bloom) = column.bloom {
                bloom.validate()?;
            }
        }
        Ok(())
    }

    pub(crate) fn column(&self, table: Table, column: usize) -> (Compression, EncodingPolicy) {
        self.columns
            .iter()
            .find(|entry| entry.table == table && entry.column == column)
            .map_or((self.compression, self.encoding), |entry| {
                (
                    entry.compression.unwrap_or(self.compression),
                    entry.encoding.unwrap_or(self.encoding),
                )
            })
    }

    pub(crate) fn bloom(&self, table: Table, column: usize) -> Option<BloomOptions> {
        let schema = &table.schema().columns[column];
        self.columns
            .iter()
            .find(|entry| entry.table == table && entry.column == column)
            .and_then(|entry| entry.bloom)
            .or_else(|| {
                (schema.utf8 || schema.physical_type == crate::PhysicalType::Int64)
                    .then_some(self.bloom)
                    .flatten()
            })
    }
}

/// Physical output counts for a table in [`crate::Table::ALL`] order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableWriteReport {
    /// Bytes acknowledged by the output sink, including the footer.
    pub bytes: u64,
    /// Canonical table rows.
    pub rows: u64,
    /// Physical row groups.
    pub row_groups: u64,
    /// Data pages, excluding dictionary and auxiliary metadata.
    pub data_pages: u64,
}

/// Result of a complete five-table sink export.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriteReport {
    /// Reports in [`crate::Table::ALL`] order.
    pub tables: [TableWriteReport; 5],
    /// Losses observed during the canonical RDF projection.
    pub losses: purrdf_core::LossLedger,
}
