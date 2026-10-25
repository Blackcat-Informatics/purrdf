// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native Parquet metadata and profile refusals.

use crate::parquet::{ColumnProfile, MetadataProfile, PageProfile, RowGroupProfile, SchemaProfile};
use crate::{ColumnSchema, Table};
use core::{fmt, num::TryFromIntError};

impl purrdf_hash::error::StructuredError for crate::Compression {
    purrdf_hash::error_visit! {
        Uncompressed => "columnar-compression-uncompressed" {},
        Zstd => "columnar-compression-zstd" {},
    }
}

/// Why the native Parquet envelope, metadata or physical layout is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetadataReason {
    /// The original key/value metadata differs from the closed profile.
    Metadata {
        /// Original metadata and its exact required key/value pairs.
        profile: MetadataProfile,
    },
    /// The schema root differs from the fixed flat profile.
    SchemaRoot {
        /// Original parsed node, including all absent fields.
        observed: SchemaProfile,
        /// Required number of child nodes.
        expected_children: usize,
    },
    /// A schema leaf differs from its required column.
    SchemaLeaf {
        /// Original parsed node.
        observed: SchemaProfile,
        /// Required native column schema.
        expected: ColumnSchema,
    },
    /// The row group differs from the required one-group layout.
    RowGroup {
        /// Original parsed group, retaining its existing column buffers.
        observed: RowGroupProfile,
        /// Required row count from the file metadata.
        expected_rows: i64,
        /// Required starting offset.
        expected_offset: i64,
        /// Required column count from the table schema.
        expected_columns: usize,
    },
    /// A column chunk differs from its required flat profile.
    Column {
        /// Original parsed chunk, retaining its existing encodings and path.
        observed: ColumnProfile,
        /// Required native column schema.
        expected: ColumnSchema,
        /// Required row count from the enclosing group.
        expected_rows: i64,
    },
    /// A Data Page V2 header differs from the required value profile.
    Page {
        /// Original complete parsed header.
        observed: PageProfile,
        /// Required table row count.
        expected_rows: usize,
        /// Required table codec.
        compression: crate::Compression,
    },
    /// Adding an uncompressed column size overflows the signed total.
    UncompressedTotalOverflow {
        /// Total before addition.
        total: i64,
        /// Original column size to add.
        size: i64,
    },
    /// A zero-row file contains bytes before its footer.
    EmptyPages {
        /// Original footer start.
        footer_start: usize,
    },
    /// The uncompressed total disagrees with the column chunks.
    UncompressedTotal {
        /// Declared total.
        declared: i64,
        /// Total established by the chunks.
        actual: i64,
    },
    /// A page does not begin at the preceding page's end.
    ColumnOffset {
        /// Original column index.
        column: usize,
        /// Declared offset.
        actual: usize,
        /// Required contiguous offset.
        expected: usize,
    },
    /// A table mixes two otherwise supported codecs.
    MixedCompression {
        /// Codec established by preceding columns.
        previous: crate::Compression,
        /// Original current codec.
        actual: crate::Compression,
    },
    /// Adding a page header and body overflows the address space.
    PageLengthOverflow {
        /// Original header byte count.
        header: usize,
        /// Original body byte count.
        body: usize,
    },
    /// A column's uncompressed size disagrees with its page.
    UncompressedColumnSize {
        /// Original table.
        table: Table,
        /// Original column name.
        column: &'static str,
        /// Declared byte count.
        declared: i64,
        /// Actual page byte count.
        actual: i64,
    },
    /// A column's compressed size disagrees with its page.
    CompressedColumnSize {
        /// Original table.
        table: Table,
        /// Original column name.
        column: &'static str,
        /// Declared byte count.
        declared: i64,
        /// Actual page byte count.
        actual: i64,
    },
    /// Advancing an offset overflows the address space.
    OffsetOverflow {
        /// Original offset.
        offset: usize,
        /// Original byte increment.
        length: usize,
    },
    /// The final page does not end at the footer boundary.
    PageEnd {
        /// Original final page end.
        actual: usize,
        /// Original footer start.
        expected: usize,
    },
    /// The compressed total disagrees with the column chunks.
    CompressedTotal {
        /// Original optional declared total.
        declared: Option<i64>,
        /// Total established by the chunks.
        actual: i64,
    },
    /// Leading or trailing file magic is absent.
    Magic,
    /// The declared footer length exceeds its enclosing file.
    FooterLength {
        /// Original footer byte count.
        length: usize,
        /// Original file byte count.
        file_length: usize,
    },
    /// The footer overlaps the leading magic.
    FooterOverlap {
        /// Original footer start.
        start: usize,
    },
    /// A file repeats a key/value metadata key.
    DuplicateKey {
        /// Original metadata key, moved from its parsed owner.
        key: String,
    },
    /// Compact bytes remain after the footer's final field.
    TrailingCompact {
        /// Original end position of the parsed struct.
        offset: usize,
        /// Original enclosing byte count.
        length: usize,
    },
    /// The logical-type union repeats its STRING member.
    DuplicateString,
    /// A STRING marker has fields despite its empty schema.
    NonemptyString,
    /// The schema has a different number of nodes.
    SchemaNodes {
        /// Original node count.
        actual: usize,
        /// Required node count.
        expected: usize,
    },
    /// A zero-row table has row groups.
    EmptyRowGroups {
        /// Original row-group count.
        actual: usize,
    },
    /// A nonempty table has a different number of row groups.
    RowGroups {
        /// Original row-group count.
        actual: usize,
    },
    /// A required Compact field was encountered twice.
    DuplicateField,
    /// A required Compact field is absent.
    AbsentField,
    /// A signed wire count cannot be represented as a host index.
    Nonnegative {
        /// Original signed value.
        value: i64,
        /// Original narrowing refusal.
        cause: TryFromIntError,
    },
}

impl purrdf_hash::error::StructuredError for MetadataReason {
    purrdf_hash::error_visit! {
        Metadata { profile } => "columnar-parquet-metadata" { "profile" => Record(profile) },
        SchemaRoot { observed, expected_children } => "columnar-parquet-schema-root" {
            "observed" => Record(observed), "expected_children" => Unsigned(*expected_children as u128),
        },
        SchemaLeaf { observed, expected } => "columnar-parquet-schema-leaf" {
            "observed" => Record(observed), "expected" => Record(expected),
        },
        RowGroup { observed, expected_rows, expected_offset, expected_columns } => "columnar-parquet-row-group" {
            "observed" => Record(observed), "expected_rows" => Signed(i128::from(*expected_rows)),
            "expected_offset" => Signed(i128::from(*expected_offset)), "expected_columns" => Unsigned(*expected_columns as u128),
        },
        Column { observed, expected, expected_rows } => "columnar-parquet-column" {
            "observed" => Record(observed), "expected" => Record(expected),
            "expected_rows" => Signed(i128::from(*expected_rows)),
        },
        Page { observed, expected_rows, compression } => "columnar-parquet-page" {
            "observed" => Record(observed), "expected_rows" => Unsigned(*expected_rows as u128),
            "compression" => Record(compression),
        },
        UncompressedTotalOverflow { total, size } => "columnar-parquet-uncompressed-total-overflow" {
            "total" => Signed(i128::from(*total)), "size" => Signed(i128::from(*size)),
        },
        EmptyPages { footer_start } => "columnar-parquet-empty-pages" { "footer_start" => ByteCount(*footer_start as u128) },
        UncompressedTotal { declared, actual } => "columnar-parquet-uncompressed-total" {
            "declared" => Signed(i128::from(*declared)), "actual" => Signed(i128::from(*actual)),
        },
        ColumnOffset { column, actual, expected } => "columnar-parquet-column-offset" {
            "column" => Unsigned(*column as u128), "actual" => ByteCount(*actual as u128), "expected" => ByteCount(*expected as u128),
        },
        MixedCompression { previous, actual } => "columnar-parquet-mixed-compression" {
            "previous" => Record(previous), "actual" => Record(actual),
        },
        PageLengthOverflow { header, body } => "columnar-parquet-page-length-overflow" {
            "header" => ByteCount(*header as u128), "body" => ByteCount(*body as u128),
        },
        UncompressedColumnSize { table, column, declared, actual } => "columnar-parquet-uncompressed-column-size" {
            "table" => Code(table.name()), "column" => Code(column),
            "declared" => Signed(i128::from(*declared)), "actual" => Signed(i128::from(*actual)),
        },
        CompressedColumnSize { table, column, declared, actual } => "columnar-parquet-compressed-column-size" {
            "table" => Code(table.name()), "column" => Code(column),
            "declared" => Signed(i128::from(*declared)), "actual" => Signed(i128::from(*actual)),
        },
        OffsetOverflow { offset, length } => "columnar-parquet-offset-overflow" {
            "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128),
        },
        PageEnd { actual, expected } => "columnar-parquet-page-end" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(*expected as u128),
        },
        CompressedTotal { declared, actual } => "columnar-parquet-compressed-total" {
            "declared" => Value(declared.map_or(purrdf_hash::error::ErrorValue::Absent,
                |value| purrdf_hash::error::ErrorValue::Signed(i128::from(value)))),
            "actual" => Signed(i128::from(*actual)),
        },
        Magic => "columnar-parquet-magic" {},
        FooterLength { length, file_length } => "columnar-parquet-footer-length" {
            "length" => ByteCount(*length as u128), "file_length" => ByteCount(*file_length as u128),
        },
        FooterOverlap { start } => "columnar-parquet-footer-overlap" { "start" => ByteCount(*start as u128) },
        DuplicateKey { key } => "columnar-parquet-duplicate-key" { "key" => Text(key) },
        TrailingCompact { offset, length } => "columnar-parquet-trailing-compact" {
            "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128),
        },
        DuplicateString => "columnar-parquet-duplicate-string" {},
        NonemptyString => "columnar-parquet-nonempty-string" {},
        SchemaNodes { actual, expected } => "columnar-parquet-schema-nodes" {
            "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128),
        },
        EmptyRowGroups { actual } => "columnar-parquet-empty-row-groups" { "actual" => Unsigned(*actual as u128) },
        RowGroups { actual } => "columnar-parquet-row-groups" { "actual" => Unsigned(*actual as u128) },
        DuplicateField => "columnar-parquet-duplicate-field" {},
        AbsentField => "columnar-parquet-absent-field" {},
        Nonnegative { value, cause } => "columnar-parquet-nonnegative" {
            "value" => Signed(i128::from(*value)), "cause" => Cause(cause),
        },
    }
}

purrdf_lex::error_source!(MetadataReason => { Self::Nonnegative { cause, .. } => cause });

impl fmt::Display for MetadataReason {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metadata { profile } => profile.fmt(output),
            Self::SchemaRoot { .. } => {
                output.write_str("root shape differs from the fixed flat schema")
            }
            Self::SchemaLeaf { expected, .. } => write!(
                output,
                "column {} differs from the fixed schema",
                expected.name
            ),
            Self::RowGroup { .. } => output.write_str(
                "row count, offset, size, or column count differs from the fixed profile",
            ),
            Self::Column { expected, .. } => write!(
                output,
                "column {} differs from the fixed profile",
                expected.name
            ),
            Self::Page { .. } => output.write_str(
                "counts, encoding, repetition levels, or compression flag differ from profile",
            ),
            Self::UncompressedTotalOverflow { .. } => {
                output.write_str("uncompressed byte total overflows i64")
            }
            Self::EmptyPages { .. } => output.write_str("zero-row file contains page bytes"),
            Self::UncompressedTotal { .. } => {
                output.write_str("total_byte_size disagrees with column chunks")
            }
            Self::ColumnOffset {
                column,
                actual,
                expected,
            } => write!(
                output,
                "column {column} starts at {actual}, expected contiguous offset {expected}"
            ),
            Self::MixedCompression { .. } => output.write_str("one table mixes compression codecs"),
            Self::PageLengthOverflow { .. } => output.write_str("page length overflows usize"),
            Self::UncompressedColumnSize {
                table,
                column,
                declared,
                actual,
            } => write!(
                output,
                "{}.{column} uncompressed size {declared} disagrees with page {actual}",
                table.name()
            ),
            Self::CompressedColumnSize {
                table,
                column,
                declared,
                actual,
            } => write!(
                output,
                "{}.{column} compressed size {declared} disagrees with page {actual}",
                table.name()
            ),
            Self::OffsetOverflow { .. } => output.write_str("offset overflows usize"),
            Self::PageEnd { actual, expected } => {
                write!(output, "pages end at {actual}, footer begins at {expected}")
            }
            Self::CompressedTotal { .. } => {
                output.write_str("total_compressed_size disagrees with column chunks")
            }
            Self::Magic => output.write_str("missing PAR1 magic"),
            Self::FooterLength { .. } => output.write_str("declared length exceeds file"),
            Self::FooterOverlap { .. } => output.write_str("footer overlaps leading magic"),
            Self::DuplicateKey { key } => write!(output, "duplicate key {key}"),
            Self::TrailingCompact { .. } => output.write_str("trailing compact bytes"),
            Self::DuplicateString => output.write_str("duplicate STRING member"),
            Self::NonemptyString => output.write_str("STRING marker struct is not empty"),
            Self::SchemaNodes { actual, expected } => {
                write!(output, "has {actual} nodes, expected {expected}")
            }
            Self::EmptyRowGroups { .. } => {
                output.write_str("zero-row table must have no row groups")
            }
            Self::RowGroups { actual } => write!(output, "has {actual}, expected one"),
            Self::DuplicateField => output.write_str("duplicate field"),
            Self::AbsentField => output.write_str("required field is absent"),
            Self::Nonnegative { .. } => output.write_str("value is negative or exceeds usize"),
        }
    }
}
