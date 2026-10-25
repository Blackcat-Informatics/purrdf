// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Narrow, deterministic Parquet Data Page V2 encoder/decoder.

use std::collections::BTreeMap;

use purrdf_core::ir::pack::bits::{read_varint, write_varint};
use structured_zstd::decoding::FrameDecoder;
use structured_zstd::encoding::{CompressionLevel, compress_slice_to_vec};

use std::borrow::Cow;

use crate::MetadataReason;
use crate::column::{Int64Column, Presence};
mod profile;
use crate::compact::{CompactField, CompactReader, CompactWriter, StructState, TYPE_STRUCT};
use crate::{
    ColumnSchema, ColumnarError, ConstructionReason, PageBodyReason, PhysicalType, Repetition,
    Table, ValueBodyReason,
};
pub use profile::{ColumnProfile, MetadataProfile, PageProfile, RowGroupProfile, SchemaProfile};

const PARQUET_MAGIC: &[u8; 4] = b"PAR1";
const FILE_VERSION: i32 = 1;
const SCHEMA_VERSION: &str = "1";
const META_SCHEMA_VERSION: &str = "purrdf.columnar.schema-version";
const META_TABLE: &str = "purrdf.columnar.table";

fn expected_metadata(table: Table) -> [(&'static str, &'static str); 2] {
    [
        (META_SCHEMA_VERSION, SCHEMA_VERSION),
        (META_TABLE, table.name()),
    ]
}

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

// The fixed profile keeps one decoded table resident at a time. Bound both the
// column slots and their uncompressed value bytes so hostile metadata cannot
// turn a small compressed input into an allocator-sized request.
const MAX_DECODED_TABLE_BYTES: usize = 256 * 1024 * 1024;
const MAX_INITIAL_ROW_CAPACITY: usize = 65_536;
// What the decoded-table budget charges each `INT64` row: a value word and a
// presence word. The charge is part of the profile's accept/refuse boundary, so
// it is stated here rather than read off the in-memory layout, which packs
// presence into a bitmap and would otherwise move the boundary with it.
const INT64_ROW_BUDGET_BYTES: usize = 2 * size_of::<i64>();

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
    Int64(Int64Column),
    ByteArray(Vec<Option<Vec<u8>>>),
}

impl ColumnValues {
    pub(crate) fn int64(values: Int64Column) -> Self {
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
            Self::Int64(values) => values.null_count(),
            Self::ByteArray(values) => values.iter().filter(|value| value.is_none()).count(),
        }
    }

    /// The column's definition levels: the `INT64` column's own bitmap, or one
    /// built from the BYTE_ARRAY rows.
    fn presence(&self) -> Cow<'_, Presence> {
        match self {
            Self::Int64(values) => Cow::Borrowed(values.presence()),
            Self::ByteArray(values) => Cow::Owned(values.iter().map(Option::is_some).collect()),
        }
    }

    fn encode_plain(&self) -> Result<Vec<u8>, ColumnarError> {
        match self {
            Self::Int64(values) => Ok(encode_int64_plain(values.values())),
            Self::ByteArray(values) => {
                let mut out = Vec::new();
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
                Ok(out)
            }
        }
    }

    fn plain_len(&self) -> Result<usize, ColumnarError> {
        match self {
            Self::Int64(values) => values
                .values()
                .len()
                .checked_mul(size_of::<i64>())
                .ok_or_else(table_budget_overflow),
            Self::ByteArray(values) => values.iter().flatten().try_fold(0usize, |total, value| {
                total
                    .checked_add(size_of::<i32>())
                    .and_then(|total| total.checked_add(value.len()))
                    .ok_or_else(table_budget_overflow)
            }),
        }
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
        let row_count = validate_table_columns(table, &columns)?;
        validate_decoded_table_budget(table, row_count, plain_columns_len(&columns)?)?;
        Ok(Self {
            table,
            columns,
            row_count,
        })
    }
}

fn validate_table_columns(table: Table, columns: &[ColumnValues]) -> Result<usize, ColumnarError> {
    let schema = table.schema();
    if columns.len() != schema.columns.len() {
        return Err(ColumnarError::construction(
            "table columns",
            ConstructionReason::ColumnCount {
                table,
                actual: columns.len(),
                expected: schema.columns.len(),
            },
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
            return Err(ColumnarError::construction(
                "table columns",
                ConstructionReason::ColumnRows {
                    table,
                    column: expected.name,
                    actual: column.len(),
                    expected: row_count,
                },
            ));
        }
        if column.physical_type() != expected.physical_type {
            return Err(ColumnarError::construction(
                "table columns",
                ConstructionReason::ColumnType {
                    table,
                    column: expected.name,
                    actual: column.physical_type(),
                    expected: expected.physical_type,
                },
            ));
        }
        if expected.repetition == Repetition::Required && column.null_count() != 0 {
            return Err(ColumnarError::construction(
                "table columns",
                ConstructionReason::RequiredColumn {
                    table,
                    column: expected.name,
                    nulls: column.null_count(),
                },
            ));
        }
    }
    Ok(row_count)
}

fn plain_columns_len(columns: &[ColumnValues]) -> Result<usize, ColumnarError> {
    columns.iter().try_fold(0usize, |total, column| {
        total
            .checked_add(column.plain_len()?)
            .ok_or_else(table_budget_overflow)
    })
}

fn validate_decoded_table_budget(
    table: Table,
    row_count: usize,
    uncompressed_bytes: usize,
) -> Result<(), ColumnarError> {
    let bytes_per_row = table
        .schema()
        .columns
        .iter()
        .try_fold(0usize, |total, column| {
            let slot = match column.physical_type {
                PhysicalType::Int64 => INT64_ROW_BUDGET_BYTES,
                PhysicalType::ByteArray => size_of::<Option<Vec<u8>>>(),
            };
            total.checked_add(slot).ok_or_else(table_budget_overflow)
        })?;
    let decoded_bytes = bytes_per_row
        .checked_mul(row_count)
        .and_then(|slots| slots.checked_add(uncompressed_bytes))
        .ok_or_else(table_budget_overflow)?;
    if decoded_bytes > MAX_DECODED_TABLE_BYTES {
        return Err(ColumnarError::LimitExceeded {
            context: "decoded Parquet table bytes",
            value: decoded_bytes as u64,
            maximum: MAX_DECODED_TABLE_BYTES as u64,
        });
    }
    Ok(())
}

fn table_budget_overflow() -> ColumnarError {
    ColumnarError::LimitExceeded {
        context: "decoded Parquet table bytes",
        value: u64::MAX,
        maximum: MAX_DECODED_TABLE_BYTES as u64,
    }
}

pub(crate) const fn bounded_row_capacity(row_count: usize) -> usize {
    if row_count < MAX_INITIAL_ROW_CAPACITY {
        row_count
    } else {
        MAX_INITIAL_ROW_CAPACITY
    }
}

#[derive(Debug)]
struct EncodedColumn {
    physical_type: i32,
    encodings: &'static [i32],
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
            let encoded = encode_column(values, schema, data.row_count, compression, &mut out)?;
            columns.push(EncodedColumn {
                physical_type: parquet_physical_type(schema.physical_type),
                encodings: if schema.repetition == Repetition::Optional {
                    &[ENCODING_PLAIN, ENCODING_RLE]
                } else {
                    &[ENCODING_PLAIN]
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

    let total_uncompressed = columns.iter().try_fold(0i64, |total, column| {
        total
            .checked_add(column.total_uncompressed_size)
            .ok_or_else(|| {
                ColumnarError::construction(
                    "row group metadata",
                    ConstructionReason::UncompressedTotalOverflow {
                        total,
                        size: column.total_uncompressed_size,
                    },
                )
            })
    })?;
    validate_decoded_table_budget(
        data.table,
        data.row_count,
        nonnegative_usize(total_uncompressed, "row group uncompressed byte size")?,
    )?;

    let footer = encode_file_metadata(data, &columns);
    let footer_len = u32::try_from(footer.len()).map_err(|_| ColumnarError::LimitExceeded {
        context: "Parquet footer length",
        value: footer.len() as u64,
        maximum: u64::from(u32::MAX),
    })?;
    // The footer and trailer finish the file; geometric growth here would
    // retain up to another page's capacity without any later write to use it.
    out.reserve_exact(footer.len() + size_of::<u32>() + PARQUET_MAGIC.len());
    out.extend_from_slice(&footer);
    out.extend_from_slice(&footer_len.to_le_bytes());
    out.extend_from_slice(PARQUET_MAGIC);
    Ok(out)
}

fn validate_table_data(data: &TableData) -> Result<(), ColumnarError> {
    let row_count = validate_table_columns(data.table, &data.columns)?;
    validate_decoded_table_budget(data.table, row_count, plain_columns_len(&data.columns)?)?;
    if row_count != data.row_count {
        return Err(ColumnarError::construction(
            "table row count",
            ConstructionReason::RowCount {
                stored: data.row_count,
                actual: row_count,
            },
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct EncodedPage {
    total_uncompressed_size: i64,
    total_compressed_size: i64,
}

fn encode_column(
    values: &ColumnValues,
    schema: &ColumnSchema,
    row_count: usize,
    compression: Compression,
    out: &mut Vec<u8>,
) -> Result<EncodedPage, ColumnarError> {
    let definitions = if schema.repetition == Repetition::Optional {
        encode_definition_levels(&values.presence())
    } else {
        Vec::new()
    };
    let plain = values.encode_plain()?;
    let plain_len = plain.len();
    let compressed_values = match compression {
        Compression::Uncompressed => plain,
        Compression::Zstd => compress_slice_to_vec(&plain, CompressionLevel::Level(3)),
    };
    let uncompressed_page_size = checked_i32_len(
        definitions.len().checked_add(plain_len).ok_or_else(|| {
            ColumnarError::construction(
                "Parquet page size",
                ConstructionReason::PageSizeOverflow {
                    definitions: definitions.len(),
                    values: plain_len,
                },
            )
        })?,
        "Parquet uncompressed page size",
    )?;
    let compressed_page_size = checked_i32_len(
        definitions
            .len()
            .checked_add(compressed_values.len())
            .ok_or_else(|| {
                ColumnarError::construction(
                    "Parquet page size",
                    ConstructionReason::PageSizeOverflow {
                        definitions: definitions.len(),
                        values: compressed_values.len(),
                    },
                )
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
    let page_len = header.len() + definitions.len() + compressed_values.len();
    let total_uncompressed_size = i64::try_from(header.len()).expect("header length fits i64")
        + i64::from(uncompressed_page_size);
    let total_compressed_size = i64::try_from(page_len).map_err(|_| {
        ColumnarError::limit("column chunk byte length", page_len, i64::MAX as usize)
    })?;
    out.reserve(page_len);
    out.extend_from_slice(&header);
    out.extend_from_slice(&definitions);
    out.extend_from_slice(&compressed_values);
    Ok(EncodedPage {
        total_uncompressed_size,
        total_compressed_size,
    })
}

/// The RLE runs of the definition levels: each maximal run of equal presence
/// as its length (shifted left one, the RLE-run tag bit clear) then its level.
fn encode_definition_levels(presence: &Presence) -> Vec<u8> {
    let mut out = Vec::new();
    for (present, run_len) in presence.runs() {
        write_varint(&mut out, (run_len as u64) << 1);
        out.push(u8::from(present));
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
        writer.list_i32_field(2, column.encodings);
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSchemaElement {
    physical_type: Option<i32>,
    repetition: Option<i32>,
    name: String,
    num_children: Option<i32>,
    converted_type: Option<i32>,
    logical_utf8: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedColumnChunk {
    file_offset: i64,
    metadata: ParsedColumnMeta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRowGroup {
    columns: Vec<ParsedColumnChunk>,
    total_byte_size: i64,
    num_rows: i64,
    file_offset: Option<i64>,
    total_compressed_size: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFileMeta {
    version: i32,
    schema: Vec<ParsedSchemaElement>,
    num_rows: i64,
    row_groups: Vec<ParsedRowGroup>,
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedDataPageV2 {
    num_values: i32,
    num_nulls: i32,
    num_rows: i32,
    encoding: i32,
    definition_levels_byte_length: i32,
    repetition_levels_byte_length: i32,
    is_compressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedPageHeader {
    page_type: i32,
    uncompressed_page_size: i32,
    compressed_page_size: i32,
    data_v2: ParsedDataPageV2,
    header_len: usize,
}

pub(crate) fn read_table(bytes: &[u8], table: Table) -> Result<TableData, ColumnarError> {
    let (footer_start, footer) = split_parquet_file(bytes)?;
    let metadata = validate_file_metadata(parse_file_metadata(footer)?, table)?;
    let row_count = nonnegative_usize(metadata.num_rows, "Parquet file row count")?;

    if row_count == 0 {
        if footer_start != PARQUET_MAGIC.len() {
            return Err(ColumnarError::metadata(
                "empty Parquet table",
                MetadataReason::EmptyPages { footer_start },
            ));
        }
        let columns = table
            .schema()
            .columns
            .iter()
            .map(|column| match column.physical_type {
                PhysicalType::Int64 => ColumnValues::Int64(Int64Column::default()),
                PhysicalType::ByteArray => ColumnValues::ByteArray(Vec::new()),
            })
            .collect();
        return TableData::new(table, columns);
    }

    let row_group = &metadata.row_groups[0];
    let declared_uncompressed_total =
        row_group.columns.iter().try_fold(0i64, |total, column| {
            total
                .checked_add(column.metadata.total_uncompressed_size)
                .ok_or_else(|| {
                    ColumnarError::metadata(
                        "row group metadata",
                        MetadataReason::UncompressedTotalOverflow {
                            total,
                            size: column.metadata.total_uncompressed_size,
                        },
                    )
                })
        })?;
    if row_group.total_byte_size != declared_uncompressed_total {
        return Err(ColumnarError::metadata(
            "row group metadata",
            MetadataReason::UncompressedTotal {
                declared: row_group.total_byte_size,
                actual: declared_uncompressed_total,
            },
        ));
    }
    validate_decoded_table_budget(
        table,
        row_count,
        nonnegative_usize(
            declared_uncompressed_total,
            "row group uncompressed byte size",
        )?,
    )?;

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
            return Err(ColumnarError::metadata(
                "column chunk layout",
                MetadataReason::ColumnOffset {
                    column: expected_index,
                    actual: offset,
                    expected: cursor,
                },
            ));
        }
        let compression = Compression::from_parquet_id(column.codec)?;
        if let Some(previous) = observed_compression {
            if previous != compression {
                return Err(ColumnarError::metadata(
                    "column chunk compression",
                    MetadataReason::MixedCompression {
                        previous,
                        actual: compression,
                    },
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
            ColumnarError::metadata(
                "column page",
                MetadataReason::PageLengthOverflow {
                    header: header.header_len,
                    body: body_len,
                },
            )
        })?;
        let page = page_region
            .get(..page_len)
            .ok_or_else(|| ColumnarError::truncated("column page", page_len, page_region.len()))?;

        let computed_uncompressed = i64::try_from(header.header_len)
            .expect("header length fits i64")
            + i64::from(header.uncompressed_page_size);
        if computed_uncompressed != column.total_uncompressed_size {
            return Err(ColumnarError::metadata(
                "column metadata",
                MetadataReason::UncompressedColumnSize {
                    table,
                    column: schema.name,
                    declared: column.total_uncompressed_size,
                    actual: computed_uncompressed,
                },
            ));
        }
        let computed_compressed = i64::try_from(page_len)
            .map_err(|_| ColumnarError::limit("column page length", page_len, i64::MAX as usize))?;
        if computed_compressed != column.total_compressed_size {
            return Err(ColumnarError::metadata(
                "column metadata",
                MetadataReason::CompressedColumnSize {
                    table,
                    column: schema.name,
                    declared: column.total_compressed_size,
                    actual: computed_compressed,
                },
            ));
        }
        // Validate every metadata cross-reference before allocating the decoded
        // column or entering the decompressor.
        decoded.push(decode_page(page, &header, schema, row_count, compression)?);
        uncompressed_total += computed_uncompressed;
        compressed_total += computed_compressed;
        cursor = cursor.checked_add(page_len).ok_or_else(|| {
            ColumnarError::metadata(
                "column chunk layout",
                MetadataReason::OffsetOverflow {
                    offset: cursor,
                    length: page_len,
                },
            )
        })?;
    }
    if cursor != footer_start {
        return Err(ColumnarError::metadata(
            "column chunk layout",
            MetadataReason::PageEnd {
                actual: cursor,
                expected: footer_start,
            },
        ));
    }
    if declared_uncompressed_total != uncompressed_total {
        return Err(ColumnarError::metadata(
            "row group metadata",
            MetadataReason::UncompressedTotal {
                declared: declared_uncompressed_total,
                actual: uncompressed_total,
            },
        ));
    }
    if row_group.total_compressed_size != Some(compressed_total) {
        return Err(ColumnarError::metadata(
            "row group metadata",
            MetadataReason::CompressedTotal {
                declared: row_group.total_compressed_size,
                actual: compressed_total,
            },
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
        return Err(ColumnarError::metadata(
            "Parquet file",
            MetadataReason::Magic,
        ));
    }
    let footer_len = u32::from_le_bytes(
        *bytes[bytes.len() - 8..]
            .first_chunk()
            .expect("four-byte footer length"),
    ) as usize;
    let footer_start = bytes
        .len()
        .checked_sub(8)
        .and_then(|end| end.checked_sub(footer_len))
        .ok_or_else(|| {
            ColumnarError::metadata(
                "Parquet footer",
                MetadataReason::FooterLength {
                    length: footer_len,
                    file_length: bytes.len(),
                },
            )
        })?;
    if footer_start < PARQUET_MAGIC.len() {
        return Err(ColumnarError::metadata(
            "Parquet footer",
            MetadataReason::FooterOverlap {
                start: footer_start,
            },
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
                    match map.entry(key) {
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(value);
                        }
                        std::collections::btree_map::Entry::Occupied(entry) => {
                            let (key, _) = entry.remove_entry();
                            return Err(ColumnarError::metadata(
                                "file key/value metadata",
                                MetadataReason::DuplicateKey { key },
                            ));
                        }
                    }
                }
                set_once(&mut metadata, map, "file key/value metadata")?;
            }
            _ => reader.skip_field(field)?,
        }
    }
    if !reader.is_finished() {
        return Err(ColumnarError::metadata(
            "Parquet footer",
            MetadataReason::TrailingCompact {
                offset: reader.position(),
                length: bytes.len(),
            },
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
                return Err(ColumnarError::metadata(
                    "logical type",
                    MetadataReason::DuplicateString,
                ));
            }
            let mut string_type = CompactReader::expect_struct(field)?;
            if let Some(inner) = reader.next_field(&mut string_type)? {
                reader.skip_field(inner)?;
                while let Some(inner) = reader.next_field(&mut string_type)? {
                    reader.skip_field(inner)?;
                }
                return Err(ColumnarError::metadata(
                    "logical STRING type",
                    MetadataReason::NonemptyString,
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
        return Err(ColumnarError::compact(
            "compact struct list",
            crate::CompactReason::ElementType {
                actual: element_type,
                expected: TYPE_STRUCT,
            },
        ));
    }
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        let mut state = StructState::default();
        values.push(parse(reader, &mut state)?);
    }
    Ok(values)
}

fn validate_file_metadata(
    mut metadata: ParsedFileMeta,
    table: Table,
) -> Result<ParsedFileMeta, ColumnarError> {
    if metadata.version != FILE_VERSION {
        return Err(ColumnarError::Unsupported {
            context: "Parquet file version",
            value: i64::from(metadata.version),
        });
    }
    let expected_metadata = expected_metadata(table);
    if !metadata
        .metadata
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .eq(expected_metadata)
    {
        return Err(ColumnarError::metadata(
            "Parquet key/value metadata",
            MetadataReason::Metadata {
                profile: MetadataProfile {
                    observed: metadata.metadata,
                    table,
                },
            },
        ));
    }
    let expected_schema = table.schema();
    if metadata.schema.len() != expected_schema.columns.len() + 1 {
        return Err(ColumnarError::metadata(
            "Parquet schema",
            MetadataReason::SchemaNodes {
                actual: metadata.schema.len(),
                expected: expected_schema.columns.len() + 1,
            },
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
        let observed = SchemaProfile {
            nodes: metadata.schema,
            selected: 0,
        };
        return Err(ColumnarError::metadata(
            "Parquet schema root",
            MetadataReason::SchemaRoot {
                observed,
                expected_children: expected_schema.columns.len(),
            },
        ));
    }
    for (index, expected) in expected_schema.columns.iter().enumerate() {
        let observed = &metadata.schema[index + 1];
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
            let observed = SchemaProfile {
                nodes: metadata.schema,
                selected: index + 1,
            };
            return Err(ColumnarError::metadata(
                "Parquet schema leaf",
                MetadataReason::SchemaLeaf {
                    observed,
                    expected: *expected,
                },
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
            return Err(ColumnarError::metadata(
                "Parquet row groups",
                MetadataReason::EmptyRowGroups {
                    actual: metadata.row_groups.len(),
                },
            ));
        }
        return Ok(metadata);
    }
    if metadata.row_groups.len() != 1 {
        return Err(ColumnarError::metadata(
            "Parquet row groups",
            MetadataReason::RowGroups {
                actual: metadata.row_groups.len(),
            },
        ));
    }
    let row_group = &metadata.row_groups[0];
    if row_group.num_rows != metadata.num_rows
        || row_group.file_offset != Some(PARQUET_MAGIC.len() as i64)
        || row_group.total_compressed_size.is_none()
        || row_group.columns.len() != expected_schema.columns.len()
    {
        let observed = RowGroupProfile(metadata.row_groups);
        return Err(ColumnarError::metadata(
            "Parquet row group",
            MetadataReason::RowGroup {
                observed,
                expected_rows: metadata.num_rows,
                expected_offset: PARQUET_MAGIC.len() as i64,
                expected_columns: expected_schema.columns.len(),
            },
        ));
    }
    let row_group = &mut metadata.row_groups[0];
    for (index, expected) in expected_schema.columns.iter().enumerate() {
        let column = &row_group.columns[index];
        let column_metadata = &column.metadata;
        let expected_encodings: &[i32] = if expected.repetition == Repetition::Optional {
            &[ENCODING_PLAIN, ENCODING_RLE]
        } else {
            &[ENCODING_PLAIN]
        };
        if column.file_offset != 0
            || column_metadata.physical_type != parquet_physical_type(expected.physical_type)
            || column_metadata.encodings != expected_encodings
            || column_metadata.path != [expected.name]
            || column_metadata.num_values != row_group.num_rows
            || column_metadata.total_uncompressed_size <= 0
            || column_metadata.total_compressed_size <= 0
        {
            let observed = ColumnProfile {
                columns: std::mem::take(&mut row_group.columns),
                selected: index,
            };
            return Err(ColumnarError::metadata(
                "Parquet column metadata",
                MetadataReason::Column {
                    observed,
                    expected: *expected,
                    expected_rows: row_group.num_rows,
                },
            ));
        }
        Compression::from_parquet_id(column_metadata.codec)?;
    }
    Ok(metadata)
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
    // Metadata validation bounds the row count to a signed 32-bit integer.
    let expected_rows = i32::try_from(row_count).expect("row count bounded by metadata check");
    if data.num_values != expected_rows
        || data.num_rows != expected_rows
        || data.num_nulls < 0
        || data.num_nulls > expected_rows
        || data.encoding != ENCODING_PLAIN
        || data.repetition_levels_byte_length != 0
        || data.is_compressed != (compression == Compression::Zstd)
    {
        return Err(ColumnarError::metadata(
            "Data Page V2 header",
            MetadataReason::Page {
                observed: PageProfile(*header),
                expected_rows: row_count,
                compression,
            },
        ));
    }
    let definition_len = nonnegative_usize(
        i64::from(data.definition_levels_byte_length),
        "definition-level byte length",
    )?;
    if (schema.repetition == Repetition::Required && definition_len != 0)
        || (schema.repetition == Repetition::Optional && definition_len == 0)
    {
        return Err(ColumnarError::page_body(
            "definition levels",
            PageBodyReason::DefinitionPresence {
                length: definition_len,
                repetition: schema.repetition,
            },
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
            ColumnarError::page_body(
                "uncompressed page size",
                PageBodyReason::DefinitionSize {
                    page: uncompressed_body,
                    definitions: definition_len,
                },
            )
        })?;
    if plain_len > MAX_DECODED_TABLE_BYTES {
        return Err(ColumnarError::LimitExceeded {
            context: "uncompressed Parquet page value bytes",
            value: plain_len as u64,
            maximum: MAX_DECODED_TABLE_BYTES as u64,
        });
    }
    let plain = match compression {
        Compression::Uncompressed => {
            if encoded_values.len() != plain_len {
                return Err(ColumnarError::page_body(
                    "uncompressed value body",
                    PageBodyReason::PlainSize {
                        actual: encoded_values.len(),
                        expected: plain_len,
                    },
                ));
            }
            Cow::Borrowed(encoded_values)
        }
        Compression::Zstd => {
            let mut plain = vec![0; plain_len];
            let written = FrameDecoder::new()
                .decode_all(encoded_values, &mut plain)
                .map_err(|error| {
                    ColumnarError::malformed("Zstandard page body", error.to_string())
                })?;
            if written != plain_len {
                return Err(ColumnarError::page_body(
                    "Zstandard page body",
                    PageBodyReason::DecodedSize {
                        actual: written,
                        expected: plain_len,
                    },
                ));
            }
            Cow::Owned(plain)
        }
    };
    let presence = if schema.repetition == Repetition::Optional {
        decode_definition_levels(definitions_bytes, row_count)?
    } else {
        Presence::full(row_count)
    };
    let present = presence.count();
    let observed_nulls = row_count - present;
    if observed_nulls != data.num_nulls as usize {
        return Err(ColumnarError::page_body(
            "definition levels",
            PageBodyReason::NullCount {
                actual: observed_nulls,
                declared: data.num_nulls,
            },
        ));
    }
    decode_plain(&plain, schema.physical_type, presence, present)
}

fn decode_definition_levels(bytes: &[u8], row_count: usize) -> Result<Presence, ColumnarError> {
    let mut pos = 0usize;
    let mut levels = Presence::with_capacity(bounded_row_capacity(row_count));
    while levels.len() < row_count {
        let offset = pos;
        let header = read_varint(bytes, &mut pos).map_err(|cause| {
            ColumnarError::value_body(
                "definition-level RLE",
                ValueBodyReason::Varint { offset, cause },
            )
        })?;
        if header & 1 != 0 {
            return Err(ColumnarError::value_body(
                "definition-level bit-packed run",
                ValueBodyReason::UnsupportedBitPackedRun { header },
            ));
        }
        let length = header >> 1;
        let run_len = usize::try_from(length).map_err(|cause| {
            ColumnarError::value_body(
                "definition-level RLE",
                ValueBodyReason::RunLength { length, cause },
            )
        })?;
        if run_len == 0 || run_len > row_count - levels.len() {
            return Err(ColumnarError::value_body(
                "definition-level RLE",
                ValueBodyReason::InvalidRun {
                    length: run_len,
                    remaining: row_count - levels.len(),
                },
            ));
        }
        let level = *bytes.get(pos).ok_or_else(|| {
            ColumnarError::truncated("definition-level RLE value", pos + 1, bytes.len())
        })?;
        pos += 1;
        if level > 1 {
            return Err(ColumnarError::value_body(
                "definition-level RLE value",
                ValueBodyReason::DefinitionLevel { level },
            ));
        }
        levels.push_run(level == 1, run_len);
    }
    if pos != bytes.len() {
        return Err(ColumnarError::value_body(
            "definition levels",
            ValueBodyReason::TrailingLevels {
                offset: pos,
                available: bytes.len(),
            },
        ));
    }
    Ok(levels)
}

/// PLAIN `INT64` value bytes: each present value as eight little-endian bytes,
/// in row order. Nulls contribute nothing, so the body is exactly the column's
/// dense value vector in little-endian order.
///
/// One counted copy with no branch: each value's `to_le_bytes` is the identity
/// on a little-endian target, so the loop is a plain word copy, which LLVM
/// lowers to one `memcpy` call (`memory.copy` on `wasm32`); on a big-endian
/// target it is a byte swap per word.
/// The collect writes into exactly sized storage, so no byte is zeroed first,
/// and `into_flattened` reinterprets the words as bytes without copying.
///
/// Out of line so the PLAIN loop is its own compiled function rather than a
/// fragment of the column encoder it is called from once per column.
#[inline(never)]
pub(crate) fn encode_int64_plain(values: &[i64]) -> Vec<u8> {
    values
        .iter()
        .map(|value| value.to_le_bytes())
        .collect::<Vec<[u8; size_of::<i64>()]>>()
        .into_flattened()
}

/// The column a PLAIN `INT64` value body holds under `presence`, whose
/// `present` rows hold values, or the error the one-value-at-a-time decode
/// reports.
///
/// The count of present rows decides the body's length exactly, so the length
/// checks happen once, before any value is read: a short body is
/// [`ColumnarError::Truncated`] at the end offset of the first value that does
/// not fit, and a long one is the trailing-bytes [`ColumnarError::ValueBody`],
/// the two answers the per-value loop gives at the value it fails on. Then the
/// body, which is the dense value vector in little-endian bytes, is read as one
/// counted copy (`from_le_bytes` is the identity on a little-endian target, so
/// LLVM lowers it to one `memcpy` call, `memory.copy` on `wasm32`; a big-endian
/// target swaps each word). The present rows were counted by the caller, a
/// popcount over the bitmap, before the page's null count was checked.
///
/// Out of line for the reason [`encode_int64_plain`] is.
#[inline(never)]
pub(crate) fn decode_int64_plain(
    bytes: &[u8],
    presence: Presence,
    present: usize,
) -> Result<Int64Column, ColumnarError> {
    const WIDTH: usize = size_of::<i64>();
    let needed = present.checked_mul(WIDTH);
    if needed.is_none_or(|needed| bytes.len() < needed) {
        // The per-value loop reads whole words from offset 0 and stops at the
        // first one that runs past the body: the word after the last whole one.
        let end = (bytes.len() / WIDTH)
            .checked_add(1)
            .and_then(|words| words.checked_mul(WIDTH))
            .ok_or_else(|| {
                ColumnarError::value_body(
                    "PLAIN INT64",
                    ValueBodyReason::OffsetOverflow {
                        offset: bytes.len() / WIDTH * WIDTH,
                        length: WIDTH,
                    },
                )
            })?;
        return Err(ColumnarError::truncated("PLAIN INT64", end, bytes.len()));
    }
    if needed != Some(bytes.len()) {
        return Err(ColumnarError::value_body(
            "PLAIN INT64",
            ValueBodyReason::TrailingValues {
                // The short-body branch above has established this product fits.
                offset: present * WIDTH,
                available: bytes.len(),
            },
        ));
    }
    let (words, _) = bytes.as_chunks::<WIDTH>();
    let values = words.iter().map(|word| i64::from_le_bytes(*word)).collect();
    Ok(Int64Column::from_parts(presence, values))
}

/// The rows of a PLAIN value body under `presence`, whose `present` rows
/// hold values.
fn decode_plain(
    bytes: &[u8],
    physical_type: PhysicalType,
    presence: Presence,
    present: usize,
) -> Result<ColumnValues, ColumnarError> {
    match physical_type {
        PhysicalType::Int64 => {
            decode_int64_plain(bytes, presence, present).map(ColumnValues::Int64)
        }
        PhysicalType::ByteArray => {
            let mut pos = 0usize;
            let mut values = Vec::with_capacity(bounded_row_capacity(presence.len()));
            for present in presence.iter() {
                if !present {
                    values.push(None);
                    continue;
                }
                let length_end = pos.checked_add(4).ok_or_else(|| {
                    ColumnarError::value_body(
                        "PLAIN BYTE_ARRAY",
                        ValueBodyReason::OffsetOverflow {
                            offset: pos,
                            length: 4,
                        },
                    )
                })?;
                let raw_len = bytes.get(pos..length_end).ok_or_else(|| {
                    ColumnarError::truncated("PLAIN BYTE_ARRAY length", length_end, bytes.len())
                })?;
                // The checked range above has exactly four bytes.
                let len =
                    i32::from_le_bytes(raw_len.try_into().expect("four-byte BYTE_ARRAY length"));
                if len < 0 {
                    return Err(ColumnarError::value_body(
                        "PLAIN BYTE_ARRAY length",
                        ValueBodyReason::NegativeLength { length: len },
                    ));
                }
                pos = length_end;
                let end = pos.checked_add(len as usize).ok_or_else(|| {
                    ColumnarError::value_body(
                        "PLAIN BYTE_ARRAY",
                        ValueBodyReason::OffsetOverflow {
                            offset: pos,
                            length: len as usize,
                        },
                    )
                })?;
                let value = bytes.get(pos..end).ok_or_else(|| {
                    ColumnarError::truncated("PLAIN BYTE_ARRAY value", end, bytes.len())
                })?;
                values.push(Some(value.to_vec()));
                pos = end;
            }
            if pos != bytes.len() {
                return Err(ColumnarError::value_body(
                    "PLAIN BYTE_ARRAY",
                    ValueBodyReason::TrailingValues {
                        offset: pos,
                        available: bytes.len(),
                    },
                ));
            }
            Ok(ColumnValues::ByteArray(values))
        }
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, context: &'static str) -> Result<(), ColumnarError> {
    if slot.replace(value).is_some() {
        Err(ColumnarError::metadata(
            context,
            MetadataReason::DuplicateField,
        ))
    } else {
        Ok(())
    }
}

fn required<T>(slot: Option<T>, context: &'static str) -> Result<T, ColumnarError> {
    slot.ok_or_else(|| ColumnarError::metadata(context, MetadataReason::AbsentField))
}

fn nonnegative_usize(value: i64, context: &'static str) -> Result<usize, ColumnarError> {
    usize::try_from(value).map_err(|cause| {
        ColumnarError::metadata(context, MetadataReason::Nonnegative { value, cause })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};

    #[global_allocator]
    static GLOBAL: CountingAllocator = CountingAllocator;

    /// Decode real writer output while retaining independently editable headers.
    fn optional_page(compression: Compression) -> (Vec<u8>, ParsedPageHeader) {
        let mut page = Vec::new();
        encode_column(
            &ints(&[Some(7), None]),
            &Table::Quads.schema().columns[3],
            2,
            compression,
            &mut page,
        )
        .unwrap();
        let header = parse_page_header(&page).unwrap();
        (page, header)
    }

    /// Native inspection and cloning retain original fields without allocating.
    fn assert_page_refusal(error: ColumnarError, expected: PageBodyReason, message: &str) {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let outer_events = Vec::with_capacity(4);
        let reason_events = Vec::with_capacity(4);
        let window = CurrentThreadWindow::open();
        let copy = error.clone();
        drop(error);
        let outer = Capture::read_with_buffer(&copy, outer_events);
        let ErrorValue::Cause(cause) = outer.field("reason") else {
            panic!("the original page-body reason must remain borrowed");
        };
        let fields = Capture::read_with_buffer(cause, reason_events);
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
        assert_eq!(
            (measured.retained_bytes, measured.peak_working_bytes),
            (0, 0)
        );
        assert_eq!(outer.code, "columnar-malformed");
        let ColumnarError::PageBody { reason, .. } = &copy else {
            panic!("the native page-body variant must survive cloning");
        };
        assert_eq!(reason, &expected);
        assert!(std::ptr::eq(
            std::error::Error::source(&copy)
                .unwrap()
                .downcast_ref::<PageBodyReason>()
                .unwrap(),
            reason
        ));
        assert_eq!(copy.to_string(), message);
        assert_eq!(
            fields.code,
            match expected {
                PageBodyReason::DefinitionPresence { .. } => "columnar-definition-presence",
                PageBodyReason::DefinitionSize { .. } => "columnar-definition-size",
                PageBodyReason::PlainSize { .. } => "columnar-plain-size",
                PageBodyReason::DecodedSize { .. } => "columnar-decoded-size",
                PageBodyReason::NullCount { .. } => "columnar-null-count",
            }
        );
        match expected {
            PageBodyReason::DefinitionPresence { length, repetition } => {
                assert!(
                    matches!(fields.field("length"), ErrorValue::ByteCount(value) if value == length as u128)
                );
                assert!(
                    matches!(fields.field("repetition"), ErrorValue::Code(value) if value == match repetition {
                        Repetition::Required => "required",
                        Repetition::Optional => "optional",
                    })
                );
            }
            PageBodyReason::DefinitionSize { page, definitions } => {
                assert!(
                    matches!(fields.field("page"), ErrorValue::ByteCount(value) if value == page as u128)
                );
                assert!(
                    matches!(fields.field("definitions"), ErrorValue::ByteCount(value) if value == definitions as u128)
                );
            }
            PageBodyReason::PlainSize { actual, expected }
            | PageBodyReason::DecodedSize { actual, expected } => {
                assert!(
                    matches!(fields.field("actual"), ErrorValue::ByteCount(value) if value == actual as u128)
                );
                assert!(
                    matches!(fields.field("expected"), ErrorValue::ByteCount(value) if value == expected as u128)
                );
            }
            PageBodyReason::NullCount { actual, declared } => {
                assert!(
                    matches!(fields.field("actual"), ErrorValue::Unsigned(value) if value == actual as u128)
                );
                assert!(
                    matches!(fields.field("declared"), ErrorValue::Signed(value) if value == i128::from(declared))
                );
            }
        }
    }

    /// Five actual page refusals preserve operands, neighbors and fault order.
    #[test]
    fn native_page_body_refusals_preserve_fields_and_precedence() {
        let schema = &Table::Quads.schema().columns[3];
        let (page, mut header) = optional_page(Compression::Uncompressed);
        assert_eq!(
            decode_page(&page, &header, schema, 2, Compression::Uncompressed).unwrap(),
            ints(&[Some(7), None])
        );
        for (definitions, size, reason, message) in [
            (
                0,
                12,
                PageBodyReason::DefinitionPresence {
                    length: 0,
                    repetition: Repetition::Optional,
                },
                "columnar definition levels is malformed: presence disagrees with schema nullability",
            ),
            (
                4,
                3,
                PageBodyReason::DefinitionSize {
                    page: 3,
                    definitions: 4,
                },
                "columnar uncompressed page size is malformed: smaller than definition-level section",
            ),
            (
                4,
                13,
                PageBodyReason::PlainSize {
                    actual: 8,
                    expected: 9,
                },
                "columnar uncompressed value body is malformed: has 8 bytes, expected 9",
            ),
        ] {
            header.data_v2.definition_levels_byte_length = definitions;
            header.uncompressed_page_size = size;
            let window = CurrentThreadWindow::open();
            let error =
                decode_page(&page, &header, schema, 2, Compression::Uncompressed).unwrap_err();
            let measured = window.close();
            assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
            assert_eq!(
                (measured.retained_bytes, measured.peak_working_bytes),
                (0, 0)
            );
            assert_page_refusal(error, reason, message);
        }

        let (page, mut header) = optional_page(Compression::Zstd);
        assert_eq!(
            decode_page(&page, &header, schema, 2, Compression::Zstd).unwrap(),
            ints(&[Some(7), None])
        );
        header.uncompressed_page_size += 1;
        let error = decode_page(&page, &header, schema, 2, Compression::Zstd).unwrap_err();
        drop(page);
        assert_page_refusal(
            error,
            PageBodyReason::DecodedSize {
                actual: 8,
                expected: 9,
            },
            "columnar Zstandard page body is malformed: decoded 8 bytes, expected 9",
        );

        let (mut page, mut header) = optional_page(Compression::Uncompressed);
        // Two present runs leave too few PLAIN values; null disagreement wins.
        page[header.header_len + 3] = 1;
        let window = CurrentThreadWindow::open();
        let error = decode_page(&page, &header, schema, 2, Compression::Uncompressed).unwrap_err();
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (1, 8));
        assert_eq!(
            (measured.retained_bytes, measured.peak_working_bytes),
            (0, 8)
        );
        assert_page_refusal(
            error,
            PageBodyReason::NullCount {
                actual: 0,
                declared: 1,
            },
            "columnar definition levels is malformed: encode 0 nulls, header declares 1",
        );
        header.data_v2.num_nulls = 0;
        assert_eq!(
            decode_page(&page, &header, schema, 2, Compression::Uncompressed),
            Err(ColumnarError::truncated("PLAIN INT64", 16, 8))
        );
        // A malformed run wins even when both the null count and PLAIN disagree.
        header.data_v2.num_nulls = 1;
        page[header.header_len] = 3;
        assert!(matches!(
            decode_page(&page, &header, schema, 2, Compression::Uncompressed),
            Err(ColumnarError::ValueBody {
                reason: ValueBodyReason::UnsupportedBitPackedRun { header: 3 },
                ..
            })
        ));
    }

    /// Edit a page count at its parsed location without changing file geometry.
    fn change_page_count(
        files: &crate::ParquetFiles,
        table: Table,
        column: usize,
        field_id: i16,
        value: i32,
    ) -> crate::ParquetFiles {
        let mut array = files.clone().into_array();
        let file = &mut array[crate::files::table_index(table)];
        let (_, footer) = split_parquet_file(file).unwrap();
        let metadata = parse_file_metadata(footer).unwrap();
        let offset = usize::try_from(
            metadata.row_groups[0].columns[column]
                .metadata
                .data_page_offset,
        )
        .unwrap();
        let range = {
            let mut reader = CompactReader::new(&file[offset..]);
            let mut root = StructState::default();
            let mut range = None;
            while let Some(field) = reader.next_field(&mut root).unwrap() {
                if field.id == 8 {
                    let mut data = CompactReader::expect_struct(field).unwrap();
                    while let Some(field) = reader.next_field(&mut data).unwrap() {
                        if field.id == field_id {
                            let start = reader.position();
                            reader.read_i32(field).unwrap();
                            range = Some(start..reader.position());
                            break;
                        }
                        reader.skip_field(field).unwrap();
                    }
                    break;
                }
                reader.skip_field(field).unwrap();
            }
            range.unwrap()
        };
        let mut encoded = Vec::new();
        write_varint(
            &mut encoded,
            purrdf_core::ir::pack::bits::zigzag_encode(i64::from(value)),
        );
        assert_eq!(
            range.len(),
            encoded.len(),
            "the edit must preserve physical sizes"
        );
        file[offset + range.start..offset + range.end].copy_from_slice(&encoded);
        crate::ParquetFiles::from_array(array)
    }

    /// Public readers retain body refusals after both-codec source disposal.
    #[test]
    fn public_page_body_refusals_survive_file_disposal() {
        let mut builder = purrdf_core::RdfDatasetBuilder::new();
        let id = builder.intern_iri("https://example.org/page").unwrap();
        builder.push_quad(id, id, id, None).unwrap();
        let dataset = builder.freeze().unwrap();
        let mut blobs = purrdf_core::ContentStore::new();
        blobs.insert(b"original native page body".to_vec());
        for compression in [Compression::Uncompressed, Compression::Zstd] {
            let files = crate::write(&dataset, &blobs, compression).unwrap().files;
            assert_eq!(crate::read(&files).unwrap().dataset.quad_count(), 1);
            for (table, column, field, value, expected, message) in [
                (
                    Table::Blobs,
                    0,
                    5,
                    2,
                    PageBodyReason::DefinitionPresence {
                        length: 2,
                        repetition: Repetition::Required,
                    },
                    "columnar definition levels is malformed: presence disagrees with schema nullability",
                ),
                (
                    Table::Quads,
                    3,
                    2,
                    0,
                    PageBodyReason::NullCount {
                        actual: 1,
                        declared: 0,
                    },
                    "columnar definition levels is malformed: encode 1 nulls, header declares 0",
                ),
            ] {
                let changed = change_page_count(&files, table, column, field, value);
                let error = crate::read(&changed).unwrap_err();
                drop(changed);
                assert_page_refusal(error, expected, message);
            }
        }
    }

    #[test]
    fn blob_page_write_allocation_growth() {
        for (payload_len, bounds) in [
            (1024, [(25, 4382, 1314, 2318), (37, 31_779, 287, 21_229)]),
            (
                1024 * 1024,
                [
                    (25, 3_147_054, 1_048_878, 2_097_426),
                    (39, 2_696_968, 329, 2_294_063),
                ],
            ),
        ] {
            let data = TableData::new(
                Table::Blobs,
                vec![
                    ColumnValues::bytes(vec![Some(vec![0; 32])]),
                    ColumnValues::bytes(vec![Some(vec![b'x'; payload_len])]),
                ],
            )
            .unwrap();
            for (compression, (allocations, requested, retained, peak)) in
                [Compression::Uncompressed, Compression::Zstd]
                    .into_iter()
                    .zip(bounds)
            {
                drop(write_table(&data, compression).unwrap());
                let window = CurrentThreadWindow::open();
                let bytes = write_table(&data, compression).unwrap();
                let measured = window.close();
                eprintln!("{payload_len}/{compression:?}: {measured:?}");
                assert!(measured.allocations <= allocations);
                assert!(measured.requested_bytes <= requested);
                assert!(measured.retained_bytes <= retained);
                assert!(measured.peak_working_bytes <= peak);
                assert_eq!(read_table(&bytes, Table::Blobs).unwrap(), data);
            }
        }
    }

    #[test]
    fn parsed_profile_validation_borrows_all_expected_fields() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::{Capture, Event};

        for table in Table::ALL {
            let columns = table
                .schema()
                .columns
                .iter()
                .map(|column| match column.physical_type {
                    PhysicalType::Int64 => ints(&[Some(0)]),
                    PhysicalType::ByteArray => ColumnValues::bytes(vec![Some(b"v".to_vec())]),
                })
                .collect();
            let data = TableData::new(table, columns).unwrap();
            for compression in [Compression::Uncompressed, Compression::Zstd] {
                let bytes = write_table(&data, compression).unwrap();
                let (_, footer) = split_parquet_file(&bytes).unwrap();
                let metadata = parse_file_metadata(footer).unwrap();
                let window = CurrentThreadWindow::open();
                let result = validate_file_metadata(metadata, table);
                let measured = window.close();
                result.unwrap();
                assert_eq!(measured.allocations, 0, "{table:?}/{compression:?}");
                assert_eq!(measured.requested_bytes, 0, "{table:?}/{compression:?}");
                assert_eq!(read_table(&bytes, table).unwrap(), data);
            }
        }

        for case in 0..5 {
            let bytes = write_table(&quads(), Compression::Uncompressed).unwrap();
            let (_, footer) = split_parquet_file(&bytes).unwrap();
            let mut metadata = parse_file_metadata(footer).unwrap();
            let pointer = match case {
                0 => {
                    let value = metadata.metadata.get_mut(META_TABLE).unwrap();
                    value.push('_');
                    value.as_ptr()
                }
                1 | 2 => {
                    let node = &mut metadata.schema[case - 1];
                    node.name.push('_');
                    node.name.as_ptr()
                }
                3 => {
                    metadata.row_groups[0].file_offset = Some(i64::MAX);
                    metadata.row_groups[0].columns[0].metadata.path[0].as_ptr()
                }
                4 => {
                    let path = &mut metadata.row_groups[0].columns[0].metadata.path[0];
                    path.push('_');
                    path.as_ptr()
                }
                _ => unreachable!("five original metadata owner kinds"),
            };
            let expected_message = (case == 0).then(|| {
                format!(
                    "columnar Parquet key/value metadata is malformed: observed {:?}, expected {:?}",
                    metadata.metadata,
                    BTreeMap::from(expected_metadata(Table::Quads))
                )
            });
            let window = CurrentThreadWindow::open();
            let error = validate_file_metadata(metadata, Table::Quads).unwrap_err();
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            drop(bytes);
            let reason = Capture::read(&error).child("reason");
            let text = match case {
                0 => {
                    assert_eq!(error.to_string(), expected_message.unwrap());
                    let pairs = reason.child("profile").records("observed");
                    let pair = pairs
                        .iter()
                        .find(|pair| matches!(pair.field("key"), ErrorValue::Text(META_TABLE)))
                        .unwrap();
                    pair.field("value")
                }
                1 | 2 => reason.child("observed").child("original").field("name"),
                3 | 4 => {
                    let original = reason.child("observed").child("original");
                    let column = if case == 3 {
                        assert!(
                            matches!(original.field("file_offset"), ErrorValue::Signed(value) if value == i128::from(i64::MAX))
                        );
                        original.records("columns").remove(0)
                    } else {
                        original
                    };
                    column
                        .child("metadata")
                        .events
                        .iter()
                        .find_map(|event| match event {
                            Event::Value(None, ErrorValue::Text(path)) => {
                                Some(ErrorValue::Text(path))
                            }
                            _ => None,
                        })
                        .unwrap()
                }
                _ => unreachable!("five original metadata owner kinds"),
            };
            assert!(matches!(text, ErrorValue::Text(text) if text.as_ptr() == pointer));
        }

        let bytes = write_table(&quads(), Compression::Uncompressed).unwrap();
        let mut header = parse_page_header(&bytes[4..]).unwrap();
        header.data_v2.num_nulls = -1;
        header.data_v2.encoding = -1;
        let window = CurrentThreadWindow::open();
        let error = decode_page(
            &[],
            &header,
            &Table::Quads.schema().columns[0],
            3,
            Compression::Uncompressed,
        )
        .unwrap_err();
        let measured = window.close();
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.requested_bytes, 0);
        drop(bytes);
        let reason = Capture::read(&error).child("reason");
        assert_eq!(reason.code, "columnar-parquet-page");
        let data = reason.child("observed").child("original").child("data_v2");
        assert!(matches!(data.field("num_nulls"), ErrorValue::Signed(-1)));
        assert!(matches!(data.field("encoding"), ErrorValue::Signed(-1)));
        let error = nonnegative_usize(-1, "original signed count").unwrap_err();
        let reason = Capture::read(&error).child("reason");
        assert!(matches!(reason.field("value"), ErrorValue::Signed(-1)));
        assert_eq!(reason.child("cause").code, "integer-conversion-range");
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .source()
                .unwrap()
                .is::<std::num::TryFromIntError>()
        );
    }

    fn ints(rows: &[Option<i64>]) -> ColumnValues {
        ColumnValues::int64(rows.iter().copied().collect())
    }

    fn quads() -> TableData {
        TableData::new(
            Table::Quads,
            vec![
                ints(&[Some(1), Some(4), Some(7)]),
                ints(&[Some(2), Some(5), Some(8)]),
                ints(&[Some(3), Some(6), Some(9)]),
                ints(&[None, Some(10), None]),
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

    /// Original shape operands survive input disposal and allocation-free inspection.
    #[test]
    fn required_null_and_mismatched_lengths_are_rejected() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let valid = quads();
        TableData::new(valid.table, valid.columns.clone()).unwrap();
        for compression in [Compression::Uncompressed, Compression::Zstd] {
            let bytes = write_table(&valid, compression).unwrap();
            assert_eq!(read_table(&bytes, Table::Quads).unwrap(), valid);
        }
        let cases = [
            (
                vec![ints(&[None])],
                ConstructionReason::ColumnCount {
                    table: Table::Quads,
                    actual: 1,
                    expected: 4,
                },
                "columnar table columns is malformed: quads has 1 columns, expected 4",
                "columnar-parquet-column-count",
            ),
            (
                vec![
                    ints(&[Some(0)]),
                    ColumnValues::bytes(Vec::new()),
                    ints(&[Some(0)]),
                    ints(&[None]),
                ],
                ConstructionReason::ColumnRows {
                    table: Table::Quads,
                    column: "p",
                    actual: 0,
                    expected: 1,
                },
                "columnar table columns is malformed: quads.p has 0 rows, expected 1",
                "columnar-parquet-column-rows",
            ),
            (
                vec![
                    ColumnValues::bytes(vec![None]),
                    ints(&[Some(1)]),
                    ints(&[Some(2)]),
                    ints(&[None]),
                ],
                ConstructionReason::ColumnType {
                    table: Table::Quads,
                    column: "s",
                    actual: PhysicalType::ByteArray,
                    expected: PhysicalType::Int64,
                },
                "columnar table columns is malformed: quads.s has physical type ByteArray, expected Int64",
                "columnar-parquet-column-type",
            ),
            (
                vec![ints(&[None]), ints(&[]), ints(&[Some(2)]), ints(&[None])],
                ConstructionReason::RequiredColumn {
                    table: Table::Quads,
                    column: "s",
                    nulls: 1,
                },
                "columnar table columns is malformed: required column quads.s contains null",
                "columnar-parquet-required-column",
            ),
        ];
        for (columns, expected, message, code) in cases {
            let outer_events = Vec::with_capacity(4);
            let reason_events = Vec::with_capacity(16);
            let window = CurrentThreadWindow::open();
            // Each case also violates a later check: the first fault stays stable.
            let error = TableData::new(Table::Quads, columns).unwrap_err();
            let copy = error.clone();
            drop(error);
            let outer = Capture::read_with_buffer(&copy, outer_events);
            let ErrorValue::Cause(cause) = outer.field("reason") else {
                panic!("construction operands must stay borrowed");
            };
            let fields = Capture::read_with_buffer(cause, reason_events);
            let measured = window.close();
            assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
            assert_eq!(outer.code, "columnar-malformed");
            assert!(matches!(
                outer.field("context"),
                ErrorValue::Code("table columns")
            ));
            assert_eq!(fields.code, code);
            assert!(matches!(fields.field("table"), ErrorValue::Code("quads")));
            match expected {
                ConstructionReason::ColumnCount { .. } => {
                    assert!(matches!(fields.field("actual"), ErrorValue::Unsigned(1)));
                    assert!(matches!(fields.field("expected"), ErrorValue::Unsigned(4)));
                }
                ConstructionReason::ColumnRows { .. } => {
                    assert!(matches!(fields.field("column"), ErrorValue::Code("p")));
                    assert!(matches!(fields.field("actual"), ErrorValue::Unsigned(0)));
                    assert!(matches!(fields.field("expected"), ErrorValue::Unsigned(1)));
                }
                ConstructionReason::ColumnType { .. } => {
                    assert!(matches!(fields.field("column"), ErrorValue::Code("s")));
                    assert_eq!(fields.child("actual").code, "columnar-physical-byte-array");
                    assert_eq!(fields.child("expected").code, "columnar-physical-int64");
                }
                ConstructionReason::RequiredColumn { .. } => {
                    assert!(matches!(fields.field("column"), ErrorValue::Code("s")));
                    assert!(matches!(fields.field("nulls"), ErrorValue::Unsigned(1)));
                }
                _ => unreachable!("only reachable construction refusals are selected"),
            }
            let ColumnarError::Construction { reason, .. } = &copy else {
                panic!("the native construction variant must survive cloning");
            };
            assert_eq!(reason, &expected);
            assert!(std::ptr::eq(
                std::error::Error::source(&copy)
                    .unwrap()
                    .downcast_ref::<ConstructionReason>()
                    .unwrap(),
                reason,
            ));
            assert_eq!(copy.to_string(), message);
        }

        let mut stale = valid;
        stale.row_count += 1;
        let outer_events = Vec::with_capacity(4);
        let reason_events = Vec::with_capacity(4);
        let window = CurrentThreadWindow::open();
        let error = write_table(&stale, Compression::Zstd).unwrap_err();
        drop(stale);
        let copy = error.clone();
        drop(error);
        let outer = Capture::read_with_buffer(&copy, outer_events);
        let ErrorValue::Cause(cause) = outer.field("reason") else {
            panic!("stored row-count operands must stay borrowed");
        };
        let fields = Capture::read_with_buffer(cause, reason_events);
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
        assert_eq!(outer.code, "columnar-malformed");
        assert!(matches!(
            outer.field("context"),
            ErrorValue::Code("table row count")
        ));
        assert_eq!(fields.code, "columnar-parquet-row-count");
        assert!(matches!(fields.field("stored"), ErrorValue::Unsigned(4)));
        assert!(matches!(fields.field("actual"), ErrorValue::Unsigned(3)));
        assert_eq!(
            std::error::Error::source(&copy)
                .unwrap()
                .downcast_ref::<ConstructionReason>(),
            Some(&ConstructionReason::RowCount {
                stored: 4,
                actual: 3
            })
        );
        assert_eq!(
            copy.to_string(),
            "columnar table row count is malformed: stored row count 4 disagrees with columns 3"
        );
    }

    #[test]
    fn wrong_table_and_corruption_fail_closed() {
        let mut bytes = write_table(&quads(), Compression::Uncompressed).unwrap();
        assert!(read_table(&bytes, Table::Annotations).is_err());

        bytes[0] ^= 0xff;
        assert!(matches!(
            read_table(&bytes, Table::Quads),
            Err(ColumnarError::Metadata {
                reason: MetadataReason::Magic,
                ..
            })
        ));
    }

    #[test]
    fn definition_rle_rejects_bitpacked_and_trailing_data() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        assert!(matches!(
            decode_definition_levels(&[3, 0], 8),
            Err(ColumnarError::ValueBody {
                reason: ValueBodyReason::UnsupportedBitPackedRun { header: 3 },
                ..
            })
        ));
        let mut encoded = encode_definition_levels(&[true, true, false].into_iter().collect());
        encoded.push(0);
        assert_eq!(
            decode_definition_levels(&encoded, 3),
            Err(ColumnarError::value_body(
                "definition levels",
                ValueBodyReason::TrailingLevels {
                    offset: 4,
                    available: 5,
                },
            ))
        );

        let mut header = Vec::new();
        write_varint(&mut header, u64::MAX);
        let error = decode_definition_levels(&header, 8).unwrap_err();
        drop(header);
        let reason = Capture::read(&error).child("reason");
        assert!(
            matches!(reason.field("header"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert_eq!(
            error.to_string(),
            "columnar definition-level bit-packed run value -1 is unsupported"
        );
        assert_eq!(
            decode_definition_levels(&[16, 1], 8).unwrap(),
            Presence::full(8)
        );

        for (bytes, expected) in [
            (
                &[2, 1, 0x80][..],
                ValueBodyReason::Varint {
                    offset: 2,
                    cause: purrdf_core::ir::pack::bits::PackBitsError::Truncated {
                        needed: 4,
                        found: 3,
                    },
                },
            ),
            (
                &[0][..],
                ValueBodyReason::InvalidRun {
                    length: 0,
                    remaining: 8,
                },
            ),
            (
                &[18][..],
                ValueBodyReason::InvalidRun {
                    length: 9,
                    remaining: 8,
                },
            ),
            (&[16, 2][..], ValueBodyReason::DefinitionLevel { level: 2 }),
        ] {
            let window = CurrentThreadWindow::open();
            let error = decode_definition_levels(bytes, 8).unwrap_err();
            let measured = window.close();
            // Only the existing eight-row presence bitmap may allocate.
            assert_eq!((measured.allocations, measured.requested_bytes), (1, 8));
            assert_eq!(
                (measured.retained_bytes, measured.peak_working_bytes),
                (0, 8)
            );
            assert!(
                matches!(error, ColumnarError::ValueBody { ref reason, .. } if reason == &expected)
            );
            if let ValueBodyReason::Varint { cause, .. } = expected {
                let native = std::error::Error::source(&error).unwrap();
                assert_eq!(
                    native
                        .source()
                        .unwrap()
                        .downcast_ref::<purrdf_core::ir::pack::bits::PackBitsError>(),
                    Some(&cause)
                );
            }
        }
    }

    /// Original signed lengths and trailing coordinates survive value decoding.
    #[test]
    fn byte_array_refusals_preserve_lengths_and_trailing_coordinates() {
        for length in [-1_i32, i32::MIN] {
            let bytes = length.to_le_bytes();
            let presence = Presence::full(1);
            let window = CurrentThreadWindow::open();
            let error = decode_plain(&bytes, PhysicalType::ByteArray, presence, 1).unwrap_err();
            let measured = window.close();
            assert_eq!(
                (measured.allocations, measured.requested_bytes),
                (1, size_of::<Option<Vec<u8>>>() as u64)
            );
            // Refusal also releases the input bitmap allocated before the window.
            assert_eq!(measured.retained_bytes, -(size_of::<u64>() as i64));
            assert_eq!(
                error,
                ColumnarError::value_body(
                    "PLAIN BYTE_ARRAY length",
                    ValueBodyReason::NegativeLength { length },
                )
            );
            assert_eq!(
                error.to_string(),
                "columnar PLAIN BYTE_ARRAY length is malformed: negative length"
            );
        }
        let body = [1, 0, 0, 0, b'x', 9];
        assert_eq!(
            decode_plain(&body, PhysicalType::ByteArray, Presence::full(1), 1),
            Err(ColumnarError::value_body(
                "PLAIN BYTE_ARRAY",
                ValueBodyReason::TrailingValues {
                    offset: 5,
                    available: 6
                },
            ))
        );
        assert_eq!(
            decode_plain(&body[..5], PhysicalType::ByteArray, Presence::full(1), 1).unwrap(),
            ColumnValues::bytes(vec![Some(vec![b'x'])])
        );
        // An impossible total byte count still reports the first missing word.
        assert_eq!(
            decode_int64_plain(&[], Presence::default(), usize::MAX),
            Err(ColumnarError::truncated("PLAIN INT64", 8, 0))
        );
    }

    #[test]
    fn decoded_table_budget_rejects_adversarial_row_counts() {
        assert!(matches!(
            validate_decoded_table_budget(Table::Quads, i32::MAX as usize, 0),
            Err(ColumnarError::LimitExceeded {
                context: "decoded Parquet table bytes",
                ..
            })
        ));
        assert_eq!(bounded_row_capacity(usize::MAX), 65_536);
    }

    #[test]
    fn oversized_zstd_output_is_rejected_before_allocation() {
        let oversized = MAX_DECODED_TABLE_BYTES + 1;
        let header = ParsedPageHeader {
            page_type: PAGE_TYPE_DATA_V2,
            uncompressed_page_size: i32::try_from(oversized).unwrap(),
            compressed_page_size: 0,
            data_v2: ParsedDataPageV2 {
                num_values: 1,
                num_nulls: 0,
                num_rows: 1,
                encoding: ENCODING_PLAIN,
                definition_levels_byte_length: 0,
                repetition_levels_byte_length: 0,
                is_compressed: true,
            },
            header_len: 0,
        };
        assert!(matches!(
            decode_page(
                &[],
                &header,
                &Table::Quads.schema().columns[0],
                1,
                Compression::Zstd,
            ),
            Err(ColumnarError::LimitExceeded {
                context: "uncompressed Parquet page value bytes",
                ..
            })
        ));
    }

    /// PLAIN `INT64` encode as it was written: one value at a time, nulls
    /// skipped. Kept as the byte oracle of the chunked encode.
    fn reference_encode_int64_plain(values: &[Option<i64>]) -> Vec<u8> {
        let mut out = Vec::new();
        for value in values.iter().flatten() {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// PLAIN `INT64` decode as it was written: one row at a time, each present
    /// row reading the next eight bytes, then the trailing-bytes check. Kept as
    /// the oracle of the chunked decode, errors included.
    fn reference_decode_int64_plain(
        bytes: &[u8],
        definitions: &[bool],
    ) -> Result<Vec<Option<i64>>, ColumnarError> {
        let mut pos = 0usize;
        let mut values = Vec::with_capacity(bounded_row_capacity(definitions.len()));
        for &present in definitions {
            if !present {
                values.push(None);
                continue;
            }
            let end = pos.checked_add(8).ok_or_else(|| {
                ColumnarError::value_body(
                    "PLAIN INT64",
                    ValueBodyReason::OffsetOverflow {
                        offset: pos,
                        length: 8,
                    },
                )
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
            return Err(ColumnarError::value_body(
                "PLAIN INT64",
                ValueBodyReason::TrailingValues {
                    offset: pos,
                    available: bytes.len(),
                },
            ));
        }
        Ok(values)
    }

    use purrdf_testkit::rng::splitmix64_next as mix;

    /// A value whose eight bytes all differ, so a word written to the wrong
    /// slot, or with its bytes in the wrong order, changes the output.
    fn int64(state: &mut u64) -> i64 {
        mix(state).cast_signed()
    }

    /// The null patterns the codec must treat alike: all null, no null,
    /// alternating (both phases), runs of random length, and random density.
    fn int64_columns(len: usize, state: &mut u64) -> Vec<Vec<Option<i64>>> {
        let mut columns = vec![
            vec![None; len],
            (0..len).map(|_| Some(int64(state))).collect(),
            (0..len)
                .map(|i| (i % 2 == 0).then(|| int64(state)))
                .collect(),
            (0..len)
                .map(|i| (i % 2 == 1).then(|| int64(state)))
                .collect(),
        ];
        let mut runs = Vec::with_capacity(len);
        let mut present = true;
        while runs.len() < len {
            let run = 1 + (mix(state) % 19) as usize;
            for _ in 0..run.min(len - runs.len()) {
                runs.push(present.then(|| int64(state)));
            }
            present = !present;
        }
        columns.push(runs);
        for density in [1_u64, 3, 7] {
            columns.push(
                (0..len)
                    .map(|_| (mix(state) % 8 < density).then(|| int64(state)))
                    .collect(),
            );
        }
        // A column that is full but for one null, with the null at every
        // position of a short column (so every chunk offset breaks a run) and
        // at one seeded position of a long one.
        let positions: Vec<usize> = if len <= 70 {
            (0..len).collect()
        } else {
            vec![(mix(state) as usize) % len]
        };
        for at in positions {
            columns.push((0..len).map(|i| (i != at).then(|| int64(state))).collect());
        }
        columns
    }

    fn definitions(values: &[Option<i64>]) -> Vec<bool> {
        values.iter().map(Option::is_some).collect()
    }

    /// The split-layout decode under `definitions`, as `decode_page` calls it.
    fn decode_split(bytes: &[u8], definitions: &[bool]) -> Result<Vec<Option<i64>>, ColumnarError> {
        let presence: Presence = definitions.iter().copied().collect();
        let present = presence.count();
        decode_int64_plain(bytes, presence, present).map(|column| column.rows().collect())
    }

    /// Definition levels as they were written: one row at a time, a run
    /// extended while the next row's level matches. Kept as the byte oracle of
    /// the bitmap-run encode.
    fn reference_encode_definition_levels(levels: &[bool]) -> Vec<u8> {
        let mut levels = levels.iter().copied().peekable();
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

    const LENGTHS: [usize; 4] = [256, 1_000, 4_099, 65_541];

    /// The split-layout encode writes the bytes the one-value-at-a-time encode
    /// wrote over the `Option<i64>` rows, for every null pattern at every length
    /// 0 through 70 and at larger lengths; the definition levels are the bytes
    /// the one-row-at-a-time run encode wrote; and the decode reads both back
    /// to the rows the old decode read.
    #[test]
    fn int64_plain_matches_the_one_value_codec() {
        let mut state = 0x0C01_u64;
        for len in (0..=70).chain(LENGTHS) {
            for values in int64_columns(len, &mut state) {
                let column: Int64Column = values.iter().copied().collect();
                let bytes = encode_int64_plain(column.values());
                assert_eq!(
                    bytes,
                    reference_encode_int64_plain(&values),
                    "len {len}: {values:?}"
                );
                assert_eq!(ints(&values).encode_plain().unwrap(), bytes);
                let definitions = definitions(&values);
                assert_eq!(
                    encode_definition_levels(column.presence()),
                    reference_encode_definition_levels(&definitions),
                    "len {len}"
                );
                let levels = reference_encode_definition_levels(&definitions);
                assert_eq!(
                    decode_definition_levels(&levels, len).unwrap(),
                    *column.presence()
                );
                let decoded = decode_split(&bytes, &definitions);
                assert_eq!(decoded, reference_decode_int64_plain(&bytes, &definitions));
                assert_eq!(decoded.unwrap(), values, "len {len}");
                let present = column.presence().count();
                assert_eq!(
                    decode_plain(
                        &bytes,
                        PhysicalType::Int64,
                        column.presence().clone(),
                        present
                    )
                    .unwrap(),
                    ints(&values)
                );
            }
        }
    }

    /// A body that is short or long by any amount is refused with the error the
    /// old decode gave: the same variant, context and offsets.
    #[test]
    fn int64_plain_refuses_bad_bodies_as_the_one_value_decode_did() {
        let mut state = 0xBAD_u64;
        for len in (0..=70).chain([256, 1_000]) {
            for values in int64_columns(len, &mut state) {
                let definitions = definitions(&values);
                let bytes = reference_encode_int64_plain(&values);
                let mut bodies: Vec<Vec<u8>> = Vec::new();
                for cut in 1..=bytes.len().min(25) {
                    bodies.push(bytes[..bytes.len() - cut].to_vec());
                }
                bodies.push(Vec::new());
                for extra in 1..=17 {
                    let mut long = bytes.clone();
                    long.extend(std::iter::repeat_n(0xA5, extra));
                    bodies.push(long);
                }
                for body in bodies {
                    let got = decode_split(&body, &definitions);
                    let want = reference_decode_int64_plain(&body, &definitions);
                    assert_eq!(got, want, "len {len}, body {} bytes", body.len());
                    if body.len() != bytes.len() {
                        assert!(want.is_err(), "len {len}: a {}-byte body", body.len());
                    }
                }
            }
        }
    }

    /// The refusals' neighbour: a body of exactly the counted length decodes,
    /// including the empty body of an all-null or empty column.
    #[test]
    fn int64_plain_accepts_the_exact_length_neighbour() {
        for definitions in [vec![], vec![false; 9], vec![true; 9], vec![false, true]] {
            let present = definitions.iter().filter(|present| **present).count();
            let body: Vec<u8> = (0..present * 8).map(|i| i as u8).collect();
            let decoded = decode_split(&body, &definitions).unwrap();
            assert_eq!(
                decoded,
                reference_decode_int64_plain(&body, &definitions).unwrap()
            );
            assert_eq!(decoded.len(), definitions.len());
        }
        assert_eq!(
            decode_split(&[0; 7], &[true]),
            Err(ColumnarError::truncated("PLAIN INT64", 8, 7))
        );
        assert_eq!(
            decode_split(&[0; 17], &[true, false, true]),
            Err(ColumnarError::value_body(
                "PLAIN INT64",
                ValueBodyReason::TrailingValues {
                    offset: 16,
                    available: 17,
                }
            ))
        );
    }
}
