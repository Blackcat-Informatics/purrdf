// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed inspection of the parser's original profile records.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

use super::{
    ParsedColumnChunk, ParsedColumnMeta, ParsedDataPageV2, ParsedPageHeader, ParsedRowGroup,
    ParsedSchemaElement,
};

/// An original parsed schema node retained by a profile refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProfile {
    pub(super) nodes: Vec<ParsedSchemaElement>,
    pub(super) selected: usize,
}

/// An original parsed row group retained by a profile refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowGroupProfile(pub(super) Vec<ParsedRowGroup>);

/// An original parsed column chunk retained by a profile refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnProfile {
    pub(super) columns: Vec<ParsedColumnChunk>,
    pub(super) selected: usize,
}

/// An original parsed page header retained by a profile refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageProfile(pub(super) ParsedPageHeader);

impl StructuredError for SchemaProfile {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        // Only a checked schema index enters this private owner on refusal.
        visitor.variant("columnar-schema-profile");
        visitor.value(
            Some("original"),
            ErrorValue::Record(&self.nodes[self.selected]),
        );
    }
}
impl StructuredError for RowGroupProfile {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        // This owner is created only after the one-group cardinality check.
        visitor.variant("columnar-row-group-profile");
        visitor.value(Some("original"), ErrorValue::Record(&self.0[0]));
    }
}
impl StructuredError for ColumnProfile {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        // Only a checked column index enters this private owner on refusal.
        visitor.variant("columnar-column-profile");
        visitor.value(
            Some("original"),
            ErrorValue::Record(&self.columns[self.selected]),
        );
    }
}

impl StructuredError for PageProfile {
    purrdf_hash::error_visit! {
        @tuple self (original) => "columnar-page-profile" { "original" => Record(&self.0) }
    }
}

impl StructuredError for ParsedSchemaElement {
    purrdf_hash::error_visit! {
        @record self { physical_type, repetition, name, num_children, converted_type, logical_utf8 }
        => "columnar-parsed-schema-element" {
            "physical_type" => Value(self.physical_type.map_or(ErrorValue::Absent, |value| ErrorValue::Signed(i128::from(value)))),
            "repetition" => Value(self.repetition.map_or(ErrorValue::Absent, |value| ErrorValue::Signed(i128::from(value)))),
            "name" => Text(&self.name),
            "num_children" => Value(self.num_children.map_or(ErrorValue::Absent, |value| ErrorValue::Signed(i128::from(value)))),
            "converted_type" => Value(self.converted_type.map_or(ErrorValue::Absent, |value| ErrorValue::Signed(i128::from(value)))),
            "logical_utf8" => Boolean(self.logical_utf8),
        }
    }
}

impl StructuredError for ParsedColumnMeta {
    purrdf_hash::error_visit! {
        @record self { physical_type, encodings, path, codec, num_values, total_uncompressed_size, total_compressed_size, data_page_offset }
        => "columnar-parsed-column-metadata" {
            "physical_type" => Signed(i128::from(self.physical_type)),
            "encodings" => List(self.encodings.iter().map(|value| ErrorValue::Signed(i128::from(*value)))),
            "path" => List(self.path.iter().map(|value| ErrorValue::Text(value))),
            "codec" => Signed(i128::from(self.codec)),
            "num_values" => Signed(i128::from(self.num_values)),
            "total_uncompressed_size" => Signed(i128::from(self.total_uncompressed_size)),
            "total_compressed_size" => Signed(i128::from(self.total_compressed_size)),
            "data_page_offset" => Signed(i128::from(self.data_page_offset)),
        }
    }
}

impl StructuredError for ParsedColumnChunk {
    purrdf_hash::error_visit! {
        @record self { file_offset, metadata } => "columnar-parsed-column-chunk" {
            "file_offset" => Signed(i128::from(self.file_offset)),
            "metadata" => Record(&self.metadata),
        }
    }
}

impl StructuredError for ParsedRowGroup {
    purrdf_hash::error_visit! {
        @record self { columns, total_byte_size, num_rows, file_offset, total_compressed_size }
        => "columnar-parsed-row-group" {
            "columns" => List(self.columns.iter().map(|column| ErrorValue::Record(column))),
            "total_byte_size" => Signed(i128::from(self.total_byte_size)),
            "num_rows" => Signed(i128::from(self.num_rows)),
            "file_offset" => Value(self.file_offset.map_or(ErrorValue::Absent, |value| ErrorValue::Signed(i128::from(value)))),
            "total_compressed_size" => Value(self.total_compressed_size.map_or(ErrorValue::Absent, |value| ErrorValue::Signed(i128::from(value)))),
        }
    }
}

impl StructuredError for ParsedDataPageV2 {
    purrdf_hash::error_visit! {
        @record self { num_values, num_nulls, num_rows, encoding, definition_levels_byte_length, repetition_levels_byte_length, is_compressed }
        => "columnar-parsed-data-page-v2" {
            "num_values" => Signed(i128::from(self.num_values)),
            "num_nulls" => Signed(i128::from(self.num_nulls)),
            "num_rows" => Signed(i128::from(self.num_rows)),
            "encoding" => Signed(i128::from(self.encoding)),
            "definition_levels_byte_length" => Signed(i128::from(self.definition_levels_byte_length)),
            "repetition_levels_byte_length" => Signed(i128::from(self.repetition_levels_byte_length)),
            "is_compressed" => Boolean(self.is_compressed),
        }
    }
}

impl StructuredError for ParsedPageHeader {
    purrdf_hash::error_visit! {
        @record self { page_type, uncompressed_page_size, compressed_page_size, data_v2, header_len }
        => "columnar-parsed-page-header" {
            "page_type" => Signed(i128::from(self.page_type)),
            "uncompressed_page_size" => Signed(i128::from(self.uncompressed_page_size)),
            "compressed_page_size" => Signed(i128::from(self.compressed_page_size)),
            "data_v2" => Record(&self.data_v2),
            "header_len" => ByteCount(self.header_len as u128),
        }
    }
}

/// Original key/value metadata, moved once into its diagnostic owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataProfile {
    pub(super) observed: std::collections::BTreeMap<String, String>,
    pub(super) table: crate::Table,
}

fn visit_pairs<'error>(
    visitor: &mut dyn ErrorVisitor<'error>,
    name: &'static str,
    pairs: impl ExactSizeIterator<Item = (&'error str, &'error str)>,
) {
    visitor.begin_list(Some(name), pairs.len());
    for (key, value) in pairs {
        visitor.begin_record(None, "columnar-metadata-pair");
        visitor.value(Some("key"), ErrorValue::Text(key));
        visitor.value(Some("value"), ErrorValue::Text(value));
        visitor.end_record();
    }
    visitor.end_list();
}

impl StructuredError for MetadataProfile {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let Self { observed, table } = self;
        visitor.variant("columnar-metadata-profile");
        visit_pairs(
            visitor,
            "observed",
            observed
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
        visit_pairs(
            visitor,
            "expected",
            super::expected_metadata(*table).into_iter(),
        );
    }
}

struct ExpectedMetadata(crate::Table);

impl std::fmt::Debug for ExpectedMetadata {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output
            .debug_map()
            .entries(super::expected_metadata(self.0))
            .finish()
    }
}

impl std::fmt::Display for MetadataProfile {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            output,
            "observed {:?}, expected {:?}",
            self.observed,
            ExpectedMetadata(self.table)
        )
    }
}
