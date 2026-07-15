// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Narrow, deterministic Parquet Data Page V2 encoder/decoder.

use std::collections::BTreeMap;

use purrdf_core::ir::pack::bits::{read_varint, write_varint};
use structured_zstd::decoding::FrameDecoder;
use structured_zstd::encoding::{CompressionLevel, compress_slice_to_vec};

use crate::compact::{CompactField, CompactReader, CompactWriter, StructState, TYPE_STRUCT};
use crate::{ColumnSchema, ColumnarError, PhysicalType, Repetition, Table};

const PARQUET_MAGIC: &[u8; 4] = b"PAR1";
const FILE_VERSION: i32 = 1;
const SCHEMA_VERSION: &str = "1";
const META_SCHEMA_VERSION: &str = "purrdf.columnar.schema-version";
const META_TABLE: &str = "purrdf.columnar.table";

const PARQUET_TYPE_INT64: i32 = 2;
const PARQUET_TYPE_BYTE_ARRAY: i32 = 6;
const REPETITION_REQUIRED: i32 = 0;
const REPETITION_OPTIONAL: i32 = 1;
const CONVERTED_TYPE_UTF8: i32 = 0;
const ENCODING_PLAIN: i32 = 0;
const ENCODING_RLE: i32 = 3;
const CODEC_UNCOMPRESSED: i32 = 0;
const CODEC_ZSTD: i32 = 6;
const PAGE_TYPE_DATA_V2: i32 = 3;

/// Page compression selected for one deterministic five-table write.
///
/// This is a runtime value, never a Cargo feature: both modes are always
/// compiled and readable on every supported target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// Store PLAIN value bodies verbatim.
    #[default]
    Uncompressed,
    /// Compress each PLAIN value body as one deterministic Zstandard frame.
    Zstd,
}

impl Compression {
    const fn parquet_id(self) -> i32 {
        match self {
            Self::Uncompressed => CODEC_UNCOMPRESSED,
            Self::Zstd => CODEC_ZSTD,
        }
    }

    fn from_parquet_id(value: i32) -> Result<Self, ColumnarError> {
        match value {
            CODEC_UNCOMPRESSED => Ok(Self::Uncompressed),
            CODEC_ZSTD => Ok(Self::Zstd),
            _ => Err(ColumnarError::Unsupported {
                context: "Parquet compression codec",
                value: i64::from(value),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColumnValues {
    Int64(Vec<Option<i64>>),
    ByteArray(Vec<Option<Vec<u8>>>),
}

impl ColumnValues {
    pub(crate) fn int64(values: Vec<Option<i64>>) -> Self {
        Self::Int64(values)
    }

    pub(crate) fn bytes(values: Vec<Option<Vec<u8>>>) -> Self {
        Self::ByteArray(values)
    }

    fn len(&self) -> usize {
        match self {
            Self::Int64(values) => values.len(),
            Self::ByteArray(values) => values.len(),
        }
    }

    fn physical_type(&self) -> PhysicalType {
        match self {
            Self::Int64(_) => PhysicalType::Int64,
            Self::ByteArray(_) => PhysicalType::ByteArray,
        }
    }

    fn null_count(&self) -> usize {
        match self {
            Self::Int64(values) => values.iter().filter(|value| value.is_none()).count(),
            Self::ByteArray(values) => values.iter().filter(|value| value.is_none()).count(),
        }
    }

    fn definition_levels(&self) -> impl Iterator<Item = bool> + '_ {
        enum Iter<'a> {
            Int(std::slice::Iter<'a, Option<i64>>),
            Bytes(std::slice::Iter<'a, Option<Vec<u8>>>),
        }
        impl Iterator for Iter<'_> {
            type Item = bool;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Int(values) => values.next().map(Option::is_some),
                    Self::Bytes(values) => values.next().map(Option::is_some),
                }
            }
        }
        match self {
            Self::Int64(values) => Iter::Int(values.iter()),
            Self::ByteArray(values) => Iter::Bytes(values.iter()),
        }
    }

    fn encode_plain(&self) -> Result<Vec<u8>, ColumnarError> {
        let mut out = Vec::new();
        match self {
            Self::Int64(values) => {
                out.reserve(values.len().saturating_sub(self.null_count()) * size_of::<i64>());
                for value in values.iter().flatten() {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            Self::ByteArray(values) => {
                for value in values.iter().flatten() {
                    let len =
                        i32::try_from(value.len()).map_err(|_| ColumnarError::LimitExceeded {
                            context: "PLAIN BYTE_ARRAY length",
                            value: value.len() as u64,
                            maximum: i32::MAX as u64,
                        })?;
                    out.extend_from_slice(&len.to_le_bytes());
                    out.extend_from_slice(value);
                }
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableData {
    pub(crate) table: Table,
    pub(crate) columns: Vec<ColumnValues>,
    pub(crate) row_count: usize,
}

impl TableData {
    pub(crate) fn new(table: Table, columns: Vec<ColumnValues>) -> Result<Self, ColumnarError> {
        let schema = table.schema();
        if columns.len() != schema.columns.len() {
            return Err(ColumnarError::malformed(
                "table columns",
                format!(
                    "{} has {} columns, expected {}",
                    table.name(),
                    columns.len(),
                    schema.columns.len()
                ),
            ));
        }
        let row_count = columns.first().map_or(0, ColumnValues::len);
        if row_count > i32::MAX as usize {
            return Err(ColumnarError::LimitExceeded {
                context: "Parquet row count",
                value: row_count as u64,
                maximum: i32::MAX as u64,
            });
        }
        for (column, expected) in columns.iter().zip(schema.columns) {
            if column.len() != row_count {
                return Err(ColumnarError::malformed(
                    "table columns",
                    format!(
                        "{}.{} has {} rows, expected {row_count}",
                        table.name(),
                        expected.name,
                        column.len()
                    ),
                ));
            }
            if column.physical_type() != expected.physical_type {
                return Err(ColumnarError::malformed(
                    "table columns",
                    format!(
                        "{}.{} has physical type {:?}, expected {:?}",
                        table.name(),
                        expected.name,
                        column.physical_type(),
                        expected.physical_type
                    ),
                ));
            }
            if expected.repetition == Repetition::Required && column.null_count() != 0 {
                return Err(ColumnarError::malformed(
                    "table columns",
                    format!(
                        "required column {}.{} contains null",
                        table.name(),
                        expected.name
                    ),
                ));
            }
        }
        Ok(Self {
            table,
            columns,
            row_count,
        })
    }
}

#[derive(Debug)]
struct EncodedColumn {
    physical_type: i32,
    encodings: Vec<i32>,
    path: &'static str,
    codec: i32,
    num_values: i64,
    total_uncompressed_size: i64,
    total_compressed_size: i64,
    data_page_offset: i64,
}

pub(crate) fn write_table(
    data: &TableData,
    compression: Compression,
) -> Result<Vec<u8>, ColumnarError> {
    validate_table_data(data)?;
    let mut out = PARQUET_MAGIC.to_vec();
    let mut columns = Vec::with_capacity(data.columns.len());

    if data.row_count > 0 {
        for (values, schema) in data.columns.iter().zip(data.table.schema().columns) {
            let data_page_offset = i64::try_from(out.len()).map_err(|_| {
                ColumnarError::limit("Parquet file offset", out.len(), i64::MAX as usize)
            })?;
            let encoded = encode_column(values, schema, data.row_count, compression)?;
            out.extend_from_slice(&encoded.bytes);
            columns.push(EncodedColumn {
                physical_type: parquet_physical_type(schema.physical_type),
                encodings: if schema.repetition == Repetition::Optional {
                    vec![ENCODING_PLAIN, ENCODING_RLE]
                } else {
                    vec![ENCODING_PLAIN]
                },
                path: schema.name,
                codec: compression.parquet_id(),
                num_values: data.row_count as i64,
                total_uncompressed_size: encoded.total_uncompressed_size,
                total_compressed_size: encoded.total_compressed_size,
                data_page_offset,
            });
        }
    }

    let footer = encode_file_metadata(data, &columns);
    let footer_len = u32::try_from(footer.len()).map_err(|_| ColumnarError::LimitExceeded {
        context: "Parquet footer length",
        value: footer.len() as u64,
        maximum: u64::from(u32::MAX),
    })?;
    out.extend_from_slice(&footer);
    out.extend_from_slice(&footer_len.to_le_bytes());
    out.extend_from_slice(PARQUET_MAGIC);
    Ok(out)
}

fn validate_table_data(data: &TableData) -> Result<(), ColumnarError> {
    let rebuilt = TableData::new(data.table, data.columns.clone())?;
    if rebuilt.row_count != data.row_count {
        return Err(ColumnarError::malformed(
            "table row count",
            format!(
                "stored row count {} disagrees with columns {}",
                data.row_count, rebuilt.row_count
            ),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct EncodedPage {
    bytes: Vec<u8>,
    total_uncompressed_size: i64,
    total_compressed_size: i64,
}

fn encode_column(
    values: &ColumnValues,
    schema: &ColumnSchema,
    row_count: usize,
    compression: Compression,
) -> Result<EncodedPage, ColumnarError> {
    let definitions = if schema.repetition == Repetition::Optional {
        encode_definition_levels(values.definition_levels())
    } else {
        Vec::new()
    };
    let plain = values.encode_plain()?;
    let compressed_values = match compression {
        Compression::Uncompressed => plain.clone(),
        Compression::Zstd => compress_slice_to_vec(&plain, CompressionLevel::Level(3)),
    };
    let uncompressed_page_size = checked_i32_len(
        definitions.len().checked_add(plain.len()).ok_or_else(|| {
            ColumnarError::malformed("Parquet page size", "length addition overflows usize")
        })?,
        "Parquet uncompressed page size",
    )?;
    let compressed_page_size = checked_i32_len(
        definitions
            .len()
            .checked_add(compressed_values.len())
            .ok_or_else(|| {
                ColumnarError::malformed("Parquet page size", "length addition overflows usize")
            })?,
        "Parquet compressed page size",
    )?;
    let definition_len = checked_i32_len(definitions.len(), "definition-level byte length")?;
    let row_count = i32::try_from(row_count).expect("TableData bounded row count");
    let null_count = i32::try_from(values.null_count()).expect("null count <= row count");
    let header = CompactWriter::root(|writer| {
        writer.i32_field(1, PAGE_TYPE_DATA_V2);
        writer.i32_field(2, uncompressed_page_size);
        writer.i32_field(3, compressed_page_size);
        writer.struct_field(8, |writer| {
            writer.i32_field(1, row_count);
            writer.i32_field(2, null_count);
            writer.i32_field(3, row_count);
            writer.i32_field(4, ENCODING_PLAIN);
            writer.i32_field(5, definition_len);
            writer.i32_field(6, 0);
            writer.bool_field(7, compression == Compression::Zstd);
        });
    });
    let mut bytes = Vec::with_capacity(header.len() + definitions.len() + compressed_values.len());
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&definitions);
    bytes.extend_from_slice(&compressed_values);
    let total_uncompressed_size = i64::try_from(header.len()).expect("header length fits i64")
        + i64::from(uncompressed_page_size);
    let total_compressed_size = i64::try_from(bytes.len()).map_err(|_| {
        ColumnarError::limit("column chunk byte length", bytes.len(), i64::MAX as usize)
    })?;
    Ok(EncodedPage {
        bytes,
        total_uncompressed_size,
        total_compressed_size,
    })
}

fn encode_definition_levels(levels: impl Iterator<Item = bool>) -> Vec<u8> {
    let mut levels = levels.peekable();
    let mut out = Vec::new();
    while let Some(level) = levels.next() {
        let mut run_len = 1u64;
        while levels.next_if_eq(&level).is_some() {
            run_len += 1;
        }
        write_varint(&mut out, run_len << 1);
        out.push(u8::from(level));
    }
    out
}

fn encode_file_metadata(data: &TableData, columns: &[EncodedColumn]) -> Vec<u8> {
    let schema_nodes: Vec<Option<&ColumnSchema>> = std::iter::once(None)
        .chain(data.table.schema().columns.iter().map(Some))
        .collect();
    let row_groups = if data.row_count == 0 {
        &[][..]
    } else {
        &[()][..]
    };
    let metadata = [
        (META_SCHEMA_VERSION, SCHEMA_VERSION),
        (META_TABLE, data.table.name()),
    ];

    CompactWriter::root(|writer| {
        writer.i32_field(1, FILE_VERSION);
        writer.list_struct_field(2, &schema_nodes, |writer, node| match node {
            None => {
                writer.string_field(4, "schema");
                writer.i32_field(5, data.table.schema().columns.len() as i32);
            }
            Some(column) => write_schema_element(writer, column),
        });
        writer.i64_field(3, data.row_count as i64);
        writer.list_struct_field(4, row_groups, |writer, ()| {
            writer.list_struct_field(1, columns, write_column_chunk);
            writer.i64_field(
                2,
                columns
                    .iter()
                    .map(|column| column.total_uncompressed_size)
                    .sum(),
            );
            writer.i64_field(3, data.row_count as i64);
            writer.i64_field(5, 4);
            writer.i64_field(
                6,
                columns
                    .iter()
                    .map(|column| column.total_compressed_size)
                    .sum(),
            );
        });
        writer.list_struct_field(5, &metadata, |writer, (key, value)| {
            writer.string_field(1, key);
            writer.string_field(2, value);
        });
    })
}

fn write_schema_element(writer: &mut CompactWriter, column: &ColumnSchema) {
    writer.i32_field(1, parquet_physical_type(column.physical_type));
    writer.i32_field(
        3,
        match column.repetition {
            Repetition::Required => REPETITION_REQUIRED,
            Repetition::Optional => REPETITION_OPTIONAL,
        },
    );
    writer.string_field(4, column.name);
    if column.utf8 {
        writer.i32_field(6, CONVERTED_TYPE_UTF8);
        writer.struct_field(10, |writer| {
            writer.struct_field(1, |_| {});
        });
    }
}

fn write_column_chunk(writer: &mut CompactWriter, column: &EncodedColumn) {
    writer.i64_field(2, 0);
    writer.struct_field(3, |writer| {
        writer.i32_field(1, column.physical_type);
        writer.list_i32_field(2, &column.encodings);
        writer.list_string_field(3, &[column.path]);
        writer.i32_field(4, column.codec);
        writer.i64_field(5, column.num_values);
        writer.i64_field(6, column.total_uncompressed_size);
        writer.i64_field(7, column.total_compressed_size);
        writer.i64_field(9, column.data_page_offset);
    });
}

const fn parquet_physical_type(physical_type: PhysicalType) -> i32 {
    match physical_type {
        PhysicalType::Int64 => PARQUET_TYPE_INT64,
        PhysicalType::ByteArray => PARQUET_TYPE_BYTE_ARRAY,
    }
}

fn checked_i32_len(len: usize, context: &'static str) -> Result<i32, ColumnarError> {
    i32::try_from(len).map_err(|_| ColumnarError::LimitExceeded {
        context,
        value: len as u64,
        maximum: i32::MAX as u64,
    })
}

// ── Metadata reader ────────────────────────────────────────────────────────

#[derive(Debug)]
struct ParsedSchemaElement {
    physical_type: Option<i32>,
    repetition: Option<i32>,
    name: String,
    num_children: Option<i32>,
    converted_type: Option<i32>,
    logical_utf8: bool,
}

#[derive(Debug)]
struct ParsedColumnMeta {
    physical_type: i32,
    encodings: Vec<i32>,
    path: Vec<String>,
    codec: i32,
    num_values: i64,
    total_uncompressed_size: i64,
    total_compressed_size: i64,
    data_page_offset: i64,
}

#[derive(Debug)]
struct ParsedColumnChunk {
    file_offset: i64,
    metadata: ParsedColumnMeta,
}

#[derive(Debug)]
struct ParsedRowGroup {
    columns: Vec<ParsedColumnChunk>,
    total_byte_size: i64,
    num_rows: i64,
    file_offset: Option<i64>,
    total_compressed_size: Option<i64>,
}

#[derive(Debug)]
struct ParsedFileMeta {
    version: i32,
    schema: Vec<ParsedSchemaElement>,
    num_rows: i64,
    row_groups: Vec<ParsedRowGroup>,
    metadata: BTreeMap<String, String>,
}

#[derive(Debug)]
struct ParsedDataPageV2 {
    num_values: i32,
    num_nulls: i32,
    num_rows: i32,
    encoding: i32,
    definition_levels_byte_length: i32,
    repetition_levels_byte_length: i32,
    is_compressed: bool,
}

#[derive(Debug)]
struct ParsedPageHeader {
    page_type: i32,
    uncompressed_page_size: i32,
    compressed_page_size: i32,
    data_v2: ParsedDataPageV2,
    header_len: usize,
}

pub(crate) fn read_table(bytes: &[u8], table: Table) -> Result<TableData, ColumnarError> {
    let (footer_start, footer) = split_parquet_file(bytes)?;
    let metadata = parse_file_metadata(footer)?;
    validate_file_metadata(&metadata, table)?;
    let row_count = nonnegative_usize(metadata.num_rows, "Parquet file row count")?;

    if row_count == 0 {
        if footer_start != PARQUET_MAGIC.len() {
            return Err(ColumnarError::malformed(
                "empty Parquet table",
                "zero-row file contains page bytes",
            ));
        }
        let columns = table
            .schema()
            .columns
            .iter()
            .map(|column| match column.physical_type {
                PhysicalType::Int64 => ColumnValues::Int64(Vec::new()),
                PhysicalType::ByteArray => ColumnValues::ByteArray(Vec::new()),
            })
            .collect();
        return TableData::new(table, columns);
    }

    let row_group = &metadata.row_groups[0];
    let mut cursor = PARQUET_MAGIC.len();
    let mut decoded = Vec::with_capacity(row_group.columns.len());
    let mut uncompressed_total = 0i64;
    let mut compressed_total = 0i64;
    let mut observed_compression = None;
    for ((chunk, schema), expected_index) in row_group
        .columns
        .iter()
        .zip(table.schema().columns)
        .zip(0usize..)
    {
        let column = &chunk.metadata;
        let offset = nonnegative_usize(column.data_page_offset, "data page offset")?;
        if offset != cursor {
            return Err(ColumnarError::malformed(
                "column chunk layout",
                format!(
                    "column {expected_index} starts at {offset}, expected contiguous offset {cursor}"
                ),
            ));
        }
        let compression = Compression::from_parquet_id(column.codec)?;
        if let Some(previous) = observed_compression {
            if previous != compression {
                return Err(ColumnarError::malformed(
                    "column chunk compression",
                    "one table mixes compression codecs",
                ));
            }
        } else {
            observed_compression = Some(compression);
        }
        let page_region = bytes
            .get(offset..footer_start)
            .ok_or_else(|| ColumnarError::truncated("column page", offset, footer_start))?;
        let header = parse_page_header(page_region)?;
        let body_len = nonnegative_usize(
            i64::from(header.compressed_page_size),
            "compressed page size",
        )?;
        let page_len = header.header_len.checked_add(body_len).ok_or_else(|| {
            ColumnarError::malformed("column page", "page length overflows usize")
        })?;
        let page = page_region
            .get(..page_len)
            .ok_or_else(|| ColumnarError::truncated("column page", page_len, page_region.len()))?;
        decoded.push(decode_page(page, &header, schema, row_count, compression)?);

        let computed_uncompressed = i64::try_from(header.header_len)
            .expect("header length fits i64")
            + i64::from(header.uncompressed_page_size);
        if computed_uncompressed != column.total_uncompressed_size {
            return Err(ColumnarError::malformed(
                "column metadata",
                format!(
                    "{}.{} uncompressed size {} disagrees with page {computed_uncompressed}",
                    table.name(),
                    schema.name,
                    column.total_uncompressed_size
                ),
            ));
        }
        let computed_compressed = i64::try_from(page_len)
            .map_err(|_| ColumnarError::limit("column page length", page_len, i64::MAX as usize))?;
        if computed_compressed != column.total_compressed_size {
            return Err(ColumnarError::malformed(
                "column metadata",
                format!(
                    "{}.{} compressed size {} disagrees with page {computed_compressed}",
                    table.name(),
                    schema.name,
                    column.total_compressed_size
                ),
            ));
        }
        uncompressed_total += computed_uncompressed;
        compressed_total += computed_compressed;
        cursor = cursor.checked_add(page_len).ok_or_else(|| {
            ColumnarError::malformed("column chunk layout", "offset overflows usize")
        })?;
    }
    if cursor != footer_start {
        return Err(ColumnarError::malformed(
            "column chunk layout",
            format!("pages end at {cursor}, footer begins at {footer_start}"),
        ));
    }
    if row_group.total_byte_size != uncompressed_total {
        return Err(ColumnarError::malformed(
            "row group metadata",
            "total_byte_size disagrees with column chunks",
        ));
    }
    if row_group.total_compressed_size != Some(compressed_total) {
        return Err(ColumnarError::malformed(
            "row group metadata",
            "total_compressed_size disagrees with column chunks",
        ));
    }
    TableData::new(table, decoded)
}

fn split_parquet_file(bytes: &[u8]) -> Result<(usize, &[u8]), ColumnarError> {
    if bytes.len() < 12 {
        return Err(ColumnarError::truncated("Parquet file", 12, bytes.len()));
    }
    if bytes.get(..4) != Some(PARQUET_MAGIC) || bytes.get(bytes.len() - 4..) != Some(PARQUET_MAGIC)
    {
        return Err(ColumnarError::malformed(
            "Parquet file",
            "missing PAR1 magic",
        ));
    }
    let footer_len = u32::from_le_bytes(
        bytes[bytes.len() - 8..bytes.len() - 4]
            .try_into()
            .expect("four-byte footer length"),
    ) as usize;
    let footer_start = bytes
        .len()
        .checked_sub(8)
        .and_then(|end| end.checked_sub(footer_len))
        .ok_or_else(|| {
            ColumnarError::malformed("Parquet footer", "declared length exceeds file")
        })?;
    if footer_start < PARQUET_MAGIC.len() {
        return Err(ColumnarError::malformed(
            "Parquet footer",
            "footer overlaps leading magic",
        ));
    }
    Ok((footer_start, &bytes[footer_start..bytes.len() - 8]))
}

fn parse_file_metadata(bytes: &[u8]) -> Result<ParsedFileMeta, ColumnarError> {
    let mut reader = CompactReader::new(bytes);
    let mut state = StructState::default();
    let mut version = None;
    let mut schema = None;
    let mut num_rows = None;
    let mut row_groups = None;
    let mut metadata = None;
    while let Some(field) = reader.next_field(&mut state)? {
        match field.id {
            1 => set_once(&mut version, reader.read_i32(field)?, "file version")?,
            2 => {
                let values = parse_struct_list(&mut reader, field, parse_schema_element)?;
                set_once(&mut schema, values, "file schema")?;
            }
            3 => set_once(&mut num_rows, reader.read_i64(field)?, "file row count")?,
            4 => {
                let values = parse_struct_list(&mut reader, field, parse_row_group)?;
                set_once(&mut row_groups, values, "file row groups")?;
            }
            5 => {
                let pairs = parse_struct_list(&mut reader, field, parse_key_value)?;
                let mut map = BTreeMap::new();
                for (key, value) in pairs {
                    if map.insert(key.clone(), value).is_some() {
                        return Err(ColumnarError::malformed(
                            "file key/value metadata",
                            format!("duplicate key {key}"),
                        ));
                    }
                }
                set_once(&mut metadata, map, "file key/value metadata")?;
            }
            _ => reader.skip_field(field)?,
        }
    }
    if !reader.is_finished() {
        return Err(ColumnarError::malformed(
            "Parquet footer",
            "trailing compact bytes",
        ));
    }
    Ok(ParsedFileMeta {
        version: required(version, "file version")?,
        schema: required(schema, "file schema")?,
        num_rows: required(num_rows, "file row count")?,
        row_groups: required(row_groups, "file row groups")?,
        metadata: required(metadata, "file key/value metadata")?,
    })
}

fn parse_schema_element(
    reader: &mut CompactReader<'_>,
    state: &mut StructState,
) -> Result<ParsedSchemaElement, ColumnarError> {
    let mut physical_type = None;
    let mut repetition = None;
    let mut name = None;
    let mut num_children = None;
    let mut converted_type = None;
    let mut logical_utf8 = None;
    while let Some(field) = reader.next_field(state)? {
        match field.id {
            1 => set_once(
                &mut physical_type,
                reader.read_i32(field)?,
                "schema physical type",
            )?,
            3 => set_once(
                &mut repetition,
                reader.read_i32(field)?,
                "schema repetition",
            )?,
            4 => set_once(
                &mut name,
                reader.read_string(field)?.to_owned(),
                "schema name",
            )?,
            5 => set_once(
                &mut num_children,
                reader.read_i32(field)?,
                "schema child count",
            )?,
            6 => set_once(
                &mut converted_type,
                reader.read_i32(field)?,
                "schema converted type",
            )?,
            10 => set_once(
                &mut logical_utf8,
                parse_logical_type(reader, field)?,
                "schema logical type",
            )?,
            _ => reader.skip_field(field)?,
        }
    }
    Ok(ParsedSchemaElement {
        physical_type,
        repetition,
        name: required(name, "schema name")?,
        num_children,
        converted_type,
        logical_utf8: logical_utf8.unwrap_or(false),
    })
}

fn parse_logical_type(
    reader: &mut CompactReader<'_>,
    field: CompactField,
) -> Result<bool, ColumnarError> {
    let mut logical = CompactReader::expect_struct(field)?;
    let mut utf8 = false;
    while let Some(field) = reader.next_field(&mut logical)? {
        if field.id == 1 {
            if utf8 {
                return Err(ColumnarError::malformed(
                    "logical type",
                    "duplicate STRING member",
                ));
            }
            let mut string_type = CompactReader::expect_struct(field)?;
            if let Some(inner) = reader.next_field(&mut string_type)? {
                reader.skip_field(inner)?;
                while let Some(inner) = reader.next_field(&mut string_type)? {
                    reader.skip_field(inner)?;
                }
                return Err(ColumnarError::malformed(
                    "logical STRING type",
                    "STRING marker struct is not empty",
                ));
            }
            utf8 = true;
        } else {
            reader.skip_field(field)?;
            return Err(ColumnarError::Unsupported {
                context: "Parquet logical type field",
                value: i64::from(field.id),
            });
        }
    }
    Ok(utf8)
}

fn parse_row_group(
    reader: &mut CompactReader<'_>,
    state: &mut StructState,
) -> Result<ParsedRowGroup, ColumnarError> {
    let mut columns = None;
    let mut total_byte_size = None;
    let mut num_rows = None;
    let mut file_offset = None;
    let mut total_compressed_size = None;
    while let Some(field) = reader.next_field(state)? {
        match field.id {
            1 => set_once(
                &mut columns,
                parse_struct_list(reader, field, parse_column_chunk)?,
                "row group columns",
            )?,
            2 => set_once(
                &mut total_byte_size,
                reader.read_i64(field)?,
                "row group byte size",
            )?,
            3 => set_once(
                &mut num_rows,
                reader.read_i64(field)?,
                "row group row count",
            )?,
            5 => set_once(
                &mut file_offset,
                reader.read_i64(field)?,
                "row group file offset",
            )?,
            6 => set_once(
                &mut total_compressed_size,
                reader.read_i64(field)?,
                "row group compressed size",
            )?,
            _ => reader.skip_field(field)?,
        }
    }
    Ok(ParsedRowGroup {
        columns: required(columns, "row group columns")?,
        total_byte_size: required(total_byte_size, "row group byte size")?,
        num_rows: required(num_rows, "row group row count")?,
        file_offset,
        total_compressed_size,
    })
}

fn parse_column_chunk(
    reader: &mut CompactReader<'_>,
    state: &mut StructState,
) -> Result<ParsedColumnChunk, ColumnarError> {
    let mut file_offset = None;
    let mut metadata = None;
    while let Some(field) = reader.next_field(state)? {
        match field.id {
            2 => set_once(
                &mut file_offset,
                reader.read_i64(field)?,
                "column chunk file offset",
            )?,
            3 => {
                let mut nested = CompactReader::expect_struct(field)?;
                set_once(
                    &mut metadata,
                    parse_column_metadata(reader, &mut nested)?,
                    "column metadata",
                )?;
            }
            _ => reader.skip_field(field)?,
        }
    }
    Ok(ParsedColumnChunk {
        file_offset: required(file_offset, "column chunk file offset")?,
        metadata: required(metadata, "column metadata")?,
    })
}

fn parse_column_metadata(
    reader: &mut CompactReader<'_>,
    state: &mut StructState,
) -> Result<ParsedColumnMeta, ColumnarError> {
    let mut physical_type = None;
    let mut encodings = None;
    let mut path = None;
    let mut codec = None;
    let mut num_values = None;
    let mut total_uncompressed_size = None;
    let mut total_compressed_size = None;
    let mut data_page_offset = None;
    while let Some(field) = reader.next_field(state)? {
        match field.id {
            1 => set_once(
                &mut physical_type,
                reader.read_i32(field)?,
                "column physical type",
            )?,
            2 => set_once(
                &mut encodings,
                reader.read_list_i32(field)?,
                "column encodings",
            )?,
            3 => set_once(&mut path, reader.read_list_strings(field)?, "column path")?,
            4 => set_once(
                &mut codec,
                reader.read_i32(field)?,
                "column compression codec",
            )?,
            5 => set_once(
                &mut num_values,
                reader.read_i64(field)?,
                "column value count",
            )?,
            6 => set_once(
                &mut total_uncompressed_size,
                reader.read_i64(field)?,
                "column uncompressed size",
            )?,
            7 => set_once(
                &mut total_compressed_size,
                reader.read_i64(field)?,
                "column compressed size",
            )?,
            9 => set_once(
                &mut data_page_offset,
                reader.read_i64(field)?,
                "column data page offset",
            )?,
            _ => reader.skip_field(field)?,
        }
    }
    Ok(ParsedColumnMeta {
        physical_type: required(physical_type, "column physical type")?,
        encodings: required(encodings, "column encodings")?,
        path: required(path, "column path")?,
        codec: required(codec, "column compression codec")?,
        num_values: required(num_values, "column value count")?,
        total_uncompressed_size: required(total_uncompressed_size, "column uncompressed size")?,
        total_compressed_size: required(total_compressed_size, "column compressed size")?,
        data_page_offset: required(data_page_offset, "column data page offset")?,
    })
}

fn parse_key_value(
    reader: &mut CompactReader<'_>,
    state: &mut StructState,
) -> Result<(String, String), ColumnarError> {
    let mut key = None;
    let mut value = None;
    while let Some(field) = reader.next_field(state)? {
        match field.id {
            1 => set_once(
                &mut key,
                reader.read_string(field)?.to_owned(),
                "metadata key",
            )?,
            2 => set_once(
                &mut value,
                reader.read_string(field)?.to_owned(),
                "metadata value",
            )?,
            _ => reader.skip_field(field)?,
        }
    }
    Ok((
        required(key, "metadata key")?,
        required(value, "metadata value")?,
    ))
}

fn parse_struct_list<T>(
    reader: &mut CompactReader<'_>,
    field: CompactField,
    mut parse: impl FnMut(&mut CompactReader<'_>, &mut StructState) -> Result<T, ColumnarError>,
) -> Result<Vec<T>, ColumnarError> {
    let (len, element_type) = reader.read_list_header(field)?;
    if element_type != TYPE_STRUCT {
        return Err(ColumnarError::malformed(
            "compact struct list",
            format!("element type is {element_type}"),
        ));
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        let mut state = StructState::default();
        values.push(parse(reader, &mut state)?);
    }
    Ok(values)
}

fn validate_file_metadata(metadata: &ParsedFileMeta, table: Table) -> Result<(), ColumnarError> {
    if metadata.version != FILE_VERSION {
        return Err(ColumnarError::Unsupported {
            context: "Parquet file version",
            value: i64::from(metadata.version),
        });
    }
    let expected_metadata = BTreeMap::from([
        (META_SCHEMA_VERSION.to_owned(), SCHEMA_VERSION.to_owned()),
        (META_TABLE.to_owned(), table.name().to_owned()),
    ]);
    if metadata.metadata != expected_metadata {
        return Err(ColumnarError::malformed(
            "Parquet key/value metadata",
            format!(
                "observed {:?}, expected {:?}",
                metadata.metadata, expected_metadata
            ),
        ));
    }
    let expected_schema = table.schema();
    if metadata.schema.len() != expected_schema.columns.len() + 1 {
        return Err(ColumnarError::malformed(
            "Parquet schema",
            format!(
                "has {} nodes, expected {}",
                metadata.schema.len(),
                expected_schema.columns.len() + 1
            ),
        ));
    }
    let root = &metadata.schema[0];
    if root.physical_type.is_some()
        || root.repetition.is_some()
        || root.name != "schema"
        || root.num_children != Some(expected_schema.columns.len() as i32)
        || root.converted_type.is_some()
        || root.logical_utf8
    {
        return Err(ColumnarError::malformed(
            "Parquet schema root",
            "root shape differs from the fixed flat schema",
        ));
    }
    for (observed, expected) in metadata.schema[1..].iter().zip(expected_schema.columns) {
        let expected_repetition = match expected.repetition {
            Repetition::Required => REPETITION_REQUIRED,
            Repetition::Optional => REPETITION_OPTIONAL,
        };
        if observed.physical_type != Some(parquet_physical_type(expected.physical_type))
            || observed.repetition != Some(expected_repetition)
            || observed.name != expected.name
            || observed.num_children.is_some()
            || observed.converted_type != expected.utf8.then_some(CONVERTED_TYPE_UTF8)
            || observed.logical_utf8 != expected.utf8
        {
            return Err(ColumnarError::malformed(
                "Parquet schema leaf",
                format!("column {} differs from the fixed schema", expected.name),
            ));
        }
    }

    let row_count = nonnegative_usize(metadata.num_rows, "Parquet file row count")?;
    if row_count > i32::MAX as usize {
        return Err(ColumnarError::LimitExceeded {
            context: "Parquet file row count",
            value: row_count as u64,
            maximum: i32::MAX as u64,
        });
    }
    if row_count == 0 {
        if !metadata.row_groups.is_empty() {
            return Err(ColumnarError::malformed(
                "Parquet row groups",
                "zero-row table must have no row groups",
            ));
        }
        return Ok(());
    }
    if metadata.row_groups.len() != 1 {
        return Err(ColumnarError::malformed(
            "Parquet row groups",
            format!("has {}, expected one", metadata.row_groups.len()),
        ));
    }
    let row_group = &metadata.row_groups[0];
    if row_group.num_rows != metadata.num_rows
        || row_group.file_offset != Some(PARQUET_MAGIC.len() as i64)
        || row_group.total_compressed_size.is_none()
        || row_group.columns.len() != expected_schema.columns.len()
    {
        return Err(ColumnarError::malformed(
            "Parquet row group",
            "row count, offset, size, or column count differs from the fixed profile",
        ));
    }
    for (column, expected) in row_group.columns.iter().zip(expected_schema.columns) {
        let metadata = &column.metadata;
        let expected_encodings = if expected.repetition == Repetition::Optional {
            vec![ENCODING_PLAIN, ENCODING_RLE]
        } else {
            vec![ENCODING_PLAIN]
        };
        if column.file_offset != 0
            || metadata.physical_type != parquet_physical_type(expected.physical_type)
            || metadata.encodings != expected_encodings
            || metadata.path != [expected.name]
            || metadata.num_values != row_group.num_rows
            || metadata.total_uncompressed_size <= 0
            || metadata.total_compressed_size <= 0
        {
            return Err(ColumnarError::malformed(
                "Parquet column metadata",
                format!("column {} differs from the fixed profile", expected.name),
            ));
        }
        Compression::from_parquet_id(metadata.codec)?;
    }
    Ok(())
}

fn parse_page_header(bytes: &[u8]) -> Result<ParsedPageHeader, ColumnarError> {
    let mut reader = CompactReader::new(bytes);
    let mut state = StructState::default();
    let mut page_type = None;
    let mut uncompressed_page_size = None;
    let mut compressed_page_size = None;
    let mut data_v2 = None;
    while let Some(field) = reader.next_field(&mut state)? {
        match field.id {
            1 => set_once(&mut page_type, reader.read_i32(field)?, "page type")?,
            2 => set_once(
                &mut uncompressed_page_size,
                reader.read_i32(field)?,
                "uncompressed page size",
            )?,
            3 => set_once(
                &mut compressed_page_size,
                reader.read_i32(field)?,
                "compressed page size",
            )?,
            8 => {
                let mut nested = CompactReader::expect_struct(field)?;
                set_once(
                    &mut data_v2,
                    parse_data_page_v2(&mut reader, &mut nested)?,
                    "data page v2 header",
                )?;
            }
            _ => reader.skip_field(field)?,
        }
    }
    Ok(ParsedPageHeader {
        page_type: required(page_type, "page type")?,
        uncompressed_page_size: required(uncompressed_page_size, "uncompressed page size")?,
        compressed_page_size: required(compressed_page_size, "compressed page size")?,
        data_v2: required(data_v2, "data page v2 header")?,
        header_len: reader.position(),
    })
}

fn parse_data_page_v2(
    reader: &mut CompactReader<'_>,
    state: &mut StructState,
) -> Result<ParsedDataPageV2, ColumnarError> {
    let mut num_values = None;
    let mut num_nulls = None;
    let mut num_rows = None;
    let mut encoding = None;
    let mut definition_levels_byte_length = None;
    let mut repetition_levels_byte_length = None;
    let mut is_compressed = None;
    while let Some(field) = reader.next_field(state)? {
        match field.id {
            1 => set_once(
                &mut num_values,
                reader.read_i32(field)?,
                "data page value count",
            )?,
            2 => set_once(
                &mut num_nulls,
                reader.read_i32(field)?,
                "data page null count",
            )?,
            3 => set_once(
                &mut num_rows,
                reader.read_i32(field)?,
                "data page row count",
            )?,
            4 => set_once(&mut encoding, reader.read_i32(field)?, "data page encoding")?,
            5 => set_once(
                &mut definition_levels_byte_length,
                reader.read_i32(field)?,
                "definition-level byte length",
            )?,
            6 => set_once(
                &mut repetition_levels_byte_length,
                reader.read_i32(field)?,
                "repetition-level byte length",
            )?,
            7 => set_once(
                &mut is_compressed,
                reader.read_bool(field)?,
                "data page compression flag",
            )?,
            _ => reader.skip_field(field)?,
        }
    }
    Ok(ParsedDataPageV2 {
        num_values: required(num_values, "data page value count")?,
        num_nulls: required(num_nulls, "data page null count")?,
        num_rows: required(num_rows, "data page row count")?,
        encoding: required(encoding, "data page encoding")?,
        definition_levels_byte_length: required(
            definition_levels_byte_length,
            "definition-level byte length",
        )?,
        repetition_levels_byte_length: required(
            repetition_levels_byte_length,
            "repetition-level byte length",
        )?,
        is_compressed: is_compressed.unwrap_or(true),
    })
}

fn decode_page(
    page: &[u8],
    header: &ParsedPageHeader,
    schema: &ColumnSchema,
    row_count: usize,
    compression: Compression,
) -> Result<ColumnValues, ColumnarError> {
    if header.page_type != PAGE_TYPE_DATA_V2 {
        return Err(ColumnarError::Unsupported {
            context: "Parquet page type",
            value: i64::from(header.page_type),
        });
    }
    let data = &header.data_v2;
    let expected_rows = i32::try_from(row_count).expect("row count bounded by metadata check");
    if data.num_values != expected_rows
        || data.num_rows != expected_rows
        || data.num_nulls < 0
        || data.num_nulls > expected_rows
        || data.encoding != ENCODING_PLAIN
        || data.repetition_levels_byte_length != 0
        || data.is_compressed != (compression == Compression::Zstd)
    {
        return Err(ColumnarError::malformed(
            "Data Page V2 header",
            "counts, encoding, repetition levels, or compression flag differ from profile",
        ));
    }
    let definition_len = nonnegative_usize(
        i64::from(data.definition_levels_byte_length),
        "definition-level byte length",
    )?;
    if (schema.repetition == Repetition::Required && definition_len != 0)
        || (schema.repetition == Repetition::Optional && definition_len == 0)
    {
        return Err(ColumnarError::malformed(
            "definition levels",
            "presence disagrees with schema nullability",
        ));
    }
    let body = page
        .get(header.header_len..)
        .ok_or_else(|| ColumnarError::truncated("page body", header.header_len, page.len()))?;
    let definitions_bytes = body
        .get(..definition_len)
        .ok_or_else(|| ColumnarError::truncated("definition levels", definition_len, body.len()))?;
    let encoded_values = &body[definition_len..];
    let uncompressed_body = nonnegative_usize(
        i64::from(header.uncompressed_page_size),
        "uncompressed page size",
    )?;
    let plain_len = uncompressed_body
        .checked_sub(definition_len)
        .ok_or_else(|| {
            ColumnarError::malformed(
                "uncompressed page size",
                "smaller than definition-level section",
            )
        })?;
    let plain = match compression {
        Compression::Uncompressed => {
            if encoded_values.len() != plain_len {
                return Err(ColumnarError::malformed(
                    "uncompressed value body",
                    format!("has {} bytes, expected {plain_len}", encoded_values.len()),
                ));
            }
            encoded_values.to_vec()
        }
        Compression::Zstd => {
            let mut plain = vec![0; plain_len];
            let written = FrameDecoder::new()
                .decode_all(encoded_values, &mut plain)
                .map_err(|error| {
                    ColumnarError::malformed("Zstandard page body", error.to_string())
                })?;
            if written != plain_len {
                return Err(ColumnarError::malformed(
                    "Zstandard page body",
                    format!("decoded {written} bytes, expected {plain_len}"),
                ));
            }
            plain
        }
    };
    let definitions = if schema.repetition == Repetition::Optional {
        decode_definition_levels(definitions_bytes, row_count)?
    } else {
        vec![true; row_count]
    };
    let observed_nulls = definitions.iter().filter(|present| !**present).count();
    if observed_nulls != data.num_nulls as usize {
        return Err(ColumnarError::malformed(
            "definition levels",
            format!(
                "encode {observed_nulls} nulls, header declares {}",
                data.num_nulls
            ),
        ));
    }
    decode_plain(&plain, schema.physical_type, &definitions)
}

fn decode_definition_levels(bytes: &[u8], row_count: usize) -> Result<Vec<bool>, ColumnarError> {
    let mut pos = 0usize;
    let mut levels = Vec::with_capacity(row_count);
    while levels.len() < row_count {
        let header = read_varint(bytes, &mut pos)
            .map_err(|error| ColumnarError::malformed("definition-level RLE", error.to_string()))?;
        if header & 1 != 0 {
            return Err(ColumnarError::Unsupported {
                context: "definition-level bit-packed run",
                value: header as i64,
            });
        }
        let run_len = usize::try_from(header >> 1).map_err(|_| {
            ColumnarError::malformed("definition-level RLE", "run length exceeds usize")
        })?;
        if run_len == 0 || run_len > row_count - levels.len() {
            return Err(ColumnarError::malformed(
                "definition-level RLE",
                "zero or overlong run",
            ));
        }
        let level = *bytes.get(pos).ok_or_else(|| {
            ColumnarError::truncated("definition-level RLE value", pos + 1, bytes.len())
        })?;
        pos += 1;
        if level > 1 {
            return Err(ColumnarError::malformed(
                "definition-level RLE value",
                format!("level is {level}, expected 0 or 1"),
            ));
        }
        levels.extend(std::iter::repeat_n(level == 1, run_len));
    }
    if pos != bytes.len() {
        return Err(ColumnarError::malformed(
            "definition levels",
            "trailing bytes after final run",
        ));
    }
    Ok(levels)
}

fn decode_plain(
    bytes: &[u8],
    physical_type: PhysicalType,
    definitions: &[bool],
) -> Result<ColumnValues, ColumnarError> {
    let mut pos = 0usize;
    match physical_type {
        PhysicalType::Int64 => {
            let mut values = Vec::with_capacity(definitions.len());
            for &present in definitions {
                if !present {
                    values.push(None);
                    continue;
                }
                let end = pos.checked_add(8).ok_or_else(|| {
                    ColumnarError::malformed("PLAIN INT64", "offset overflows usize")
                })?;
                let raw = bytes
                    .get(pos..end)
                    .ok_or_else(|| ColumnarError::truncated("PLAIN INT64", end, bytes.len()))?;
                values.push(Some(i64::from_le_bytes(
                    raw.try_into().expect("eight-byte INT64"),
                )));
                pos = end;
            }
            if pos != bytes.len() {
                return Err(ColumnarError::malformed(
                    "PLAIN INT64",
                    "trailing bytes after values",
                ));
            }
            Ok(ColumnValues::Int64(values))
        }
        PhysicalType::ByteArray => {
            let mut values = Vec::with_capacity(definitions.len());
            for &present in definitions {
                if !present {
                    values.push(None);
                    continue;
                }
                let length_end = pos.checked_add(4).ok_or_else(|| {
                    ColumnarError::malformed("PLAIN BYTE_ARRAY", "offset overflows usize")
                })?;
                let raw_len = bytes.get(pos..length_end).ok_or_else(|| {
                    ColumnarError::truncated("PLAIN BYTE_ARRAY length", length_end, bytes.len())
                })?;
                let len =
                    i32::from_le_bytes(raw_len.try_into().expect("four-byte BYTE_ARRAY length"));
                if len < 0 {
                    return Err(ColumnarError::malformed(
                        "PLAIN BYTE_ARRAY length",
                        "negative length",
                    ));
                }
                pos = length_end;
                let end = pos.checked_add(len as usize).ok_or_else(|| {
                    ColumnarError::malformed("PLAIN BYTE_ARRAY", "offset overflows usize")
                })?;
                let value = bytes.get(pos..end).ok_or_else(|| {
                    ColumnarError::truncated("PLAIN BYTE_ARRAY value", end, bytes.len())
                })?;
                values.push(Some(value.to_vec()));
                pos = end;
            }
            if pos != bytes.len() {
                return Err(ColumnarError::malformed(
                    "PLAIN BYTE_ARRAY",
                    "trailing bytes after values",
                ));
            }
            Ok(ColumnValues::ByteArray(values))
        }
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, context: &'static str) -> Result<(), ColumnarError> {
    if slot.replace(value).is_some() {
        Err(ColumnarError::malformed(context, "duplicate field"))
    } else {
        Ok(())
    }
}

fn required<T>(slot: Option<T>, context: &'static str) -> Result<T, ColumnarError> {
    slot.ok_or_else(|| ColumnarError::malformed(context, "required field is absent"))
}

fn nonnegative_usize(value: i64, context: &'static str) -> Result<usize, ColumnarError> {
    usize::try_from(value)
        .map_err(|_| ColumnarError::malformed(context, "value is negative or exceeds usize"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quads() -> TableData {
        TableData::new(
            Table::Quads,
            vec![
                ColumnValues::int64(vec![Some(1), Some(4), Some(7)]),
                ColumnValues::int64(vec![Some(2), Some(5), Some(8)]),
                ColumnValues::int64(vec![Some(3), Some(6), Some(9)]),
                ColumnValues::int64(vec![None, Some(10), None]),
            ],
        )
        .unwrap()
    }

    #[test]
    fn both_compression_modes_round_trip_and_are_deterministic() {
        let data = quads();
        for compression in [Compression::Uncompressed, Compression::Zstd] {
            let first = write_table(&data, compression).unwrap();
            let second = write_table(&data, compression).unwrap();
            assert_eq!(first, second);
            assert_eq!(read_table(&first, Table::Quads).unwrap(), data);
        }
    }

    #[test]
    fn empty_table_has_all_schema_but_no_pages() {
        let data = TableData::new(
            Table::Blobs,
            vec![
                ColumnValues::bytes(Vec::new()),
                ColumnValues::bytes(Vec::new()),
            ],
        )
        .unwrap();
        let bytes = write_table(&data, Compression::Zstd).unwrap();
        assert_eq!(read_table(&bytes, Table::Blobs).unwrap(), data);
        let (footer_start, _) = split_parquet_file(&bytes).unwrap();
        assert_eq!(footer_start, 4);
    }

    #[test]
    fn required_null_and_mismatched_lengths_are_rejected() {
        assert!(
            TableData::new(
                Table::Quads,
                vec![
                    ColumnValues::int64(vec![None]),
                    ColumnValues::int64(vec![Some(1)]),
                    ColumnValues::int64(vec![Some(2)]),
                    ColumnValues::int64(vec![None]),
                ],
            )
            .is_err()
        );
        assert!(
            TableData::new(
                Table::Quads,
                vec![
                    ColumnValues::int64(vec![Some(0)]),
                    ColumnValues::int64(Vec::new()),
                    ColumnValues::int64(vec![Some(0)]),
                    ColumnValues::int64(vec![None]),
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_table_and_corruption_fail_closed() {
        let mut bytes = write_table(&quads(), Compression::Uncompressed).unwrap();
        assert!(read_table(&bytes, Table::Annotations).is_err());

        bytes[0] ^= 0xff;
        assert!(matches!(
            read_table(&bytes, Table::Quads),
            Err(ColumnarError::Malformed { .. })
        ));
    }

    #[test]
    fn definition_rle_rejects_bitpacked_and_trailing_data() {
        assert!(matches!(
            decode_definition_levels(&[3, 0], 8),
            Err(ColumnarError::Unsupported { .. })
        ));
        let mut encoded = encode_definition_levels([true, true, false].into_iter());
        encoded.push(0);
        assert!(decode_definition_levels(&encoded, 3).is_err());
    }
}
