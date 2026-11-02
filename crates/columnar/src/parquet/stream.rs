// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical rows, bounded physical pages and forward-only file publication.

use core::convert::Infallible;
use std::ops::Range;

use purrdf_core::SmallVec;

use super::encoding::{self, Dictionary};
use super::pruning;
use super::{
    ColumnSchema, ColumnarError, Compression, ENCODING_PLAIN, ENCODING_RLE, EncodedColumn,
    PARQUET_MAGIC, PhysicalType, Presence, Repetition, Table, checked_i32_len,
    encode_definition_levels, encode_file_metadata, encode_page, parquet_physical_type,
    table_budget_overflow,
};
use crate::{ColumnarSinkError, EncodingPolicy, ProjectionCell, TableWriteReport, WriteOptions};

pub(super) struct EncodedGroup {
    pub(super) columns: Vec<EncodedColumn>,
    pub(super) row_count: i64,
    pub(super) offset: i64,
}

#[derive(Default)]
struct PendingColumn {
    plain: Vec<u8>,
    presence: Presence,
    // These are row-group offsets, not signed 32-bit page-size fields.
    byte_ends: SmallVec<[usize; 1]>,
}

impl PendingColumn {
    fn clear(&mut self) {
        self.plain.clear();
        self.presence.clear();
        self.byte_ends.clear();
    }

    fn append(&mut self, cell: ProjectionCell<'_>, schema: &ColumnSchema) {
        if schema.repetition == Repetition::Optional {
            self.presence.push(!matches!(cell, ProjectionCell::Null));
        }
        match cell {
            ProjectionCell::Null => {}
            ProjectionCell::Int64(value) => self.plain.extend_from_slice(&value.to_le_bytes()),
            ProjectionCell::Bytes(bytes) => {
                // Every source row is checked before any of its cells mutate buffers.
                self.plain
                    .extend_from_slice(&(bytes.len() as i32).to_le_bytes());
                self.plain.extend_from_slice(bytes);
                self.byte_ends.push(self.plain.len());
            }
        }
    }

    fn plain_range(&self, values: Range<usize>, schema: &ColumnSchema) -> &[u8] {
        let endpoint = |value: usize| match schema.physical_type {
            PhysicalType::Int64 => value * 8,
            PhysicalType::ByteArray => value.checked_sub(1).map_or(0, |last| self.byte_ends[last]),
        };
        &self.plain[endpoint(values.start)..endpoint(values.end)]
    }
}

#[derive(Clone, Copy, Default)]
struct Measure {
    plain: u64,
    definitions: u64,
    run: u64,
    last: Option<bool>,
}

fn run_bytes(run: u64) -> u64 {
    u64::from((64 - (run << 1).leading_zeros()).div_ceil(7)) + 1
}

impl Measure {
    fn append(
        self,
        cell: ProjectionCell<'_>,
        schema: &ColumnSchema,
    ) -> Result<Self, ColumnarError> {
        let bytes = match (cell, schema.physical_type) {
            (ProjectionCell::Int64(_), PhysicalType::Int64) => 8,
            (ProjectionCell::Bytes(bytes), PhysicalType::ByteArray) => {
                checked_i32_len(bytes.len(), "PLAIN BYTE_ARRAY length")?;
                bytes.len() as u64 + 4
            }
            (ProjectionCell::Null, _) if schema.repetition == Repetition::Optional => 0,
            _ => {
                return Err(ColumnarError::Canonical(crate::CanonicalReason::Cell {
                    column: schema.name,
                    actual: match cell {
                        ProjectionCell::Null => None,
                        ProjectionCell::Int64(_) => Some(PhysicalType::Int64),
                        ProjectionCell::Bytes(_) => Some(PhysicalType::ByteArray),
                    },
                    expected: schema.physical_type,
                    repetition: schema.repetition,
                }));
            }
        };
        self.extend(!matches!(cell, ProjectionCell::Null), bytes, schema)
    }

    fn extend(
        self,
        present: bool,
        bytes: u64,
        schema: &ColumnSchema,
    ) -> Result<Self, ColumnarError> {
        let mut next = self;
        next.plain = next
            .plain
            .checked_add(bytes)
            .ok_or_else(table_budget_overflow)?;
        if schema.repetition == Repetition::Optional {
            if self.last == Some(present) {
                next.run += 1;
                next.definitions = self.definitions - run_bytes(self.run) + run_bytes(next.run);
            } else {
                next.run = 1;
                next.definitions += run_bytes(1);
            }
            next.last = Some(present);
        }
        Ok(next)
    }

    fn fits(self, limit: u64) -> bool {
        self.plain
            .checked_add(self.definitions)
            .is_some_and(|bytes| bytes <= limit)
    }
}

struct Output<'a, F> {
    table: Table,
    offset: u64,
    pages: u64,
    emit: &'a mut F,
}

/// The same page transition drives pending-group accounting and final emission.
/// Each new optional page starts its own RLE runs; a run cannot be amortized
/// across pages when deciding whether the next row fits the group target.
#[derive(Clone, Copy, Default)]
struct PageMeasure {
    size: Measure,
    rows: u32,
}

impl PageMeasure {
    fn append(
        self,
        present: bool,
        bytes: u64,
        schema: &ColumnSchema,
        options: &WriteOptions,
        physical_limit: u64,
    ) -> Result<(Self, bool, u64), ColumnarError> {
        let candidate = self.size.extend(present, bytes, schema)?;
        let starts_page = self.rows != 0
            && (self.rows >= options.page_rows
                || !candidate.fits(options.page_bytes.min(physical_limit)));
        let previous = if starts_page {
            Measure::default()
        } else {
            self.size
        };
        let next = if starts_page {
            previous.extend(present, bytes, schema)?
        } else {
            candidate
        };
        let added = (next.plain + next.definitions) - (previous.plain + previous.definitions);
        Ok((
            Self {
                size: next,
                rows: if starts_page { 1 } else { self.rows + 1 },
            },
            starts_page,
            added,
        ))
    }
}

#[derive(Clone, Copy, Default)]
struct GroupMeasure {
    page: PageMeasure,
    bytes: u64,
}

impl GroupMeasure {
    fn append(
        self,
        cell: ProjectionCell<'_>,
        schema: &ColumnSchema,
        options: &WriteOptions,
        physical_limit: u64,
    ) -> Result<Self, ColumnarError> {
        let cell_size = Measure::default().append(cell, schema)?;
        let (page, _, added) = self.page.append(
            !matches!(cell, ProjectionCell::Null),
            cell_size.plain,
            schema,
            options,
            physical_limit,
        )?;
        Ok(Self {
            page,
            bytes: self
                .bytes
                .checked_add(added)
                .ok_or_else(table_budget_overflow)?,
        })
    }
}

impl<F> Output<'_, F> {
    fn publish<S, E>(&mut self, bytes: &[u8]) -> Result<(), ColumnarSinkError<S, E>>
    where
        F: FnMut(Table, &[u8]) -> Result<(), E>,
    {
        let next = self
            .offset
            .checked_add(bytes.len() as u64)
            .filter(|&offset| i64::try_from(offset).is_ok())
            .ok_or_else(table_budget_overflow)?;
        (self.emit)(self.table, bytes).map_err(|cause| ColumnarSinkError::Sink {
            table: self.table,
            successful_bytes: self.offset,
            attempted_bytes: bytes.len(),
            cause,
        })?;
        self.offset = next;
        Ok(())
    }
}

pub(crate) fn write_rows<'a>(
    table: Table,
    rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
    compression: Compression,
) -> Result<Vec<u8>, ColumnarError> {
    let options = WriteOptions {
        compression,
        ..WriteOptions::default()
    };
    collect_rows(table, rows, &options, i32::MAX as u64)
}

fn collect_rows<'a>(
    table: Table,
    rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
    options: &WriteOptions,
    physical_limit: u64,
) -> Result<Vec<u8>, ColumnarError> {
    let mut bytes = Vec::new();
    write_rows_limited::<Infallible, Infallible>(
        table,
        rows,
        options,
        physical_limit,
        i32::MAX as u64,
        &mut |_, chunk| {
            bytes.extend_from_slice(chunk);
            Ok(())
        },
    )
    .map_err(|error| match error {
        ColumnarSinkError::Codec(cause) => cause,
        ColumnarSinkError::SourceRead(cause) | ColumnarSinkError::Sink { cause, .. } => {
            match cause {}
        }
    })?;
    Ok(bytes)
}

pub(crate) fn write_rows_to<'a, S, E>(
    table: Table,
    rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
    options: &WriteOptions,
    emit: &mut impl FnMut(Table, &[u8]) -> Result<(), E>,
) -> Result<TableWriteReport, ColumnarSinkError<S, E>> {
    write_rows_limited(table, rows, options, i32::MAX as u64, i32::MAX as u64, emit)
}

fn write_rows_limited<'a, S, E>(
    table: Table,
    rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
    options: &WriteOptions,
    physical_limit: u64,
    indexed_limit: u64,
    emit: &mut impl FnMut(Table, &[u8]) -> Result<(), E>,
) -> Result<TableWriteReport, ColumnarSinkError<S, E>> {
    options.validate()?;
    let mut output = Output {
        table,
        offset: 0,
        pages: 0,
        emit,
    };
    output.publish(PARQUET_MAGIC)?;
    let mut groups = Vec::new();
    let column_count = table.schema().columns.len();
    let mut pending: SmallVec<[PendingColumn; 11]> = (0..column_count)
        .map(|_| PendingColumn::default())
        .collect();
    let mut pending_rows = 0usize;
    let mut sizes = [GroupMeasure::default(); 11];
    let mut row_count = 0i64;
    for row in rows {
        let row = row?;
        if row.len() != column_count {
            return Err(
                ColumnarError::Canonical(crate::CanonicalReason::ColumnCount {
                    actual: row.len(),
                    expected: column_count,
                })
                .into(),
            );
        }
        for (&cell, schema) in row.iter().zip(table.schema().columns) {
            let single = Measure::default().append(cell, schema)?;
            if !single.fits(physical_limit) {
                return Err(ColumnarError::LimitExceeded {
                    context: "single row uncompressed Parquet page size",
                    value: single.plain + single.definitions,
                    maximum: physical_limit,
                }
                .into());
            }
        }
        let mut next = sizes;
        for ((next, &cell), schema) in next.iter_mut().zip(&row).zip(table.schema().columns) {
            *next = next.append(cell, schema, options, physical_limit)?;
        }
        let aggregate = next[..column_count].iter().try_fold(0u64, |total, size| {
            total
                .checked_add(size.bytes)
                .ok_or_else(table_budget_overflow)
        })?;
        if pending_rows != 0
            && (pending_rows as u64 >= options.row_group_rows
                || aggregate > options.row_group_bytes)
        {
            groups.push(emit_group(
                table,
                &pending,
                pending_rows,
                options,
                physical_limit,
                indexed_limit,
                &mut output,
            )?);
            for column in &mut pending {
                column.clear();
            }
            pending_rows = 0;
            next.fill(GroupMeasure::default());
            for ((next, &cell), schema) in next.iter_mut().zip(&row).zip(table.schema().columns) {
                *next = next.append(cell, schema, options, physical_limit)?;
            }
        }
        sizes = next;
        for ((column, &cell), schema) in pending.iter_mut().zip(&row).zip(table.schema().columns) {
            column.append(cell, schema);
        }
        pending_rows += 1;
        row_count = row_count.checked_add(1).ok_or_else(table_budget_overflow)?;
    }
    if pending_rows != 0 {
        groups.push(emit_group(
            table,
            &pending,
            pending_rows,
            options,
            physical_limit,
            indexed_limit,
            &mut output,
        )?);
    }
    drop(pending);
    let footer = encode_file_metadata(table, row_count, &groups);
    let length = u32::try_from(footer.len()).map_err(|_| {
        ColumnarError::limit("Parquet footer length", footer.len(), u32::MAX as usize)
    })?;
    output.publish(&footer)?;
    output.publish(&length.to_le_bytes())?;
    output.publish(PARQUET_MAGIC)?;
    Ok(TableWriteReport {
        bytes: output.offset,
        rows: row_count as u64,
        row_groups: groups.len() as u64,
        data_pages: output.pages,
    })
}

#[derive(Clone)]
struct PageRange {
    rows: Range<usize>,
    values: Range<usize>,
}

#[derive(Clone, Copy)]
struct ChunkEncoding<'a> {
    compression: Compression,
    policy: EncodingPolicy,
    dictionary: Option<&'a Dictionary>,
    physical_limit: u64,
    indexed_limit: u64,
    selected: Option<i32>,
}

type BuiltPage = (Vec<u8>, super::EncodedPage, i32, pruning::Statistics);

#[derive(Default)]
struct PageRecords {
    encodings: Vec<i32>,
    pages: Vec<pruning::Page>,
}

fn emit_group<S, E, F>(
    table: Table,
    pending: &[PendingColumn],
    rows: usize,
    options: &WriteOptions,
    physical_limit: u64,
    indexed_limit: u64,
    output: &mut Output<'_, F>,
) -> Result<EncodedGroup, ColumnarSinkError<S, E>>
where
    F: FnMut(Table, &[u8]) -> Result<(), E>,
{
    let offset = output.offset as i64;
    let mut columns = Vec::with_capacity(pending.len());
    for (ordinal, (column, schema)) in pending.iter().zip(table.schema().columns).enumerate() {
        let (compression, policy) = options.column(table, ordinal);
        let choice = ChunkEncoding {
            compression,
            policy,
            dictionary: None,
            physical_limit,
            indexed_limit,
            selected: None,
        };
        let mut ranges = Vec::new();
        let mut total_uncompressed_size = 0i64;
        let mut total_compressed_size = 0i64;
        let mut first_row = 0;
        let mut first_value = 0;
        let mut value = 0;
        let mut size = PageMeasure::default();
        for row in 0..rows {
            let present = schema.repetition == Repetition::Required || column.presence.get(row);
            let bytes = if present {
                column.plain_range(value..value + 1, schema).len() as u64
            } else {
                0
            };
            let (next, starts_page, _) =
                size.append(present, bytes, schema, options, physical_limit)?;
            if starts_page {
                ranges.push(PageRange {
                    rows: first_row..row,
                    values: first_value..value,
                });
                first_row = row;
                first_value = value;
            }
            size = next;
            value += usize::from(present);
        }
        ranges.push(PageRange {
            rows: first_row..rows,
            values: first_value..value,
        });
        let dictionary = if value >= 2
            && matches!(
                policy,
                EncodingPolicy::Adaptive | EncodingPolicy::Dictionary
            ) {
            Dictionary::build(
                &column.plain,
                schema.physical_type,
                options.dictionary_max_entries as usize,
                options.dictionary_max_bytes as usize,
            )?
        } else {
            None
        }
        .filter(|dictionary| {
            // With one page and no repetitions, dictionary PLAIN bytes are
            // identical to the ordinary PLAIN payload. A dictionary adds its
            // own header and index body without removing any payload bytes.
            ranges.len() != 1 || dictionary.len() != value
        });
        let mut dictionary_page = Vec::new();
        let mut dictionary_sizes = None;
        let mut ordinary_encodings = Vec::new();
        let mut single_page = None;
        if let Some(dictionary) = &dictionary {
            match super::encode_dictionary(dictionary, compression, &mut dictionary_page) {
                Ok(sizes) => {
                    let header = super::parse_page_header(&dictionary_page)?;
                    if header.compressed_page_size as u64 <= physical_limit
                        && header.uncompressed_page_size as u64 <= physical_limit
                    {
                        let mut ordinary_cost = 0u64;
                        let mut dictionary_cost = sizes.total_compressed_size as u64;
                        let mut fits = true;
                        for range in &ranges {
                            let ordinary = match build_page(column, schema, range, choice) {
                                Ok(page) => page,
                                Err(ColumnarError::LimitExceeded { .. }) => {
                                    fits = false;
                                    break;
                                }
                                Err(error) => return Err(error.into()),
                            };
                            ordinary_cost = ordinary_cost
                                .checked_add(ordinary.0.len() as u64)
                                .ok_or_else(table_budget_overflow)?;
                            ordinary_encodings.push(ordinary.2);
                            // One cached page keeps output storage bounded. A
                            // multi-page chunk retains only codec decisions.
                            if ranges.len() == 1 {
                                single_page = Some(ordinary);
                            }
                            match build_page(
                                column,
                                schema,
                                range,
                                ChunkEncoding {
                                    dictionary: Some(dictionary),
                                    ..choice
                                },
                            ) {
                                Ok(candidate) => {
                                    let header = super::parse_page_header(&candidate.0)?;
                                    fits &= header.compressed_page_size as u64 <= physical_limit
                                        && header.uncompressed_page_size as u64 <= physical_limit
                                        && candidate.0.len() as u64 <= indexed_limit;
                                    dictionary_cost = dictionary_cost
                                        .checked_add(candidate.0.len() as u64)
                                        .ok_or_else(table_budget_overflow)?;
                                }
                                Err(ColumnarError::LimitExceeded { .. }) => {
                                    fits = false;
                                    break;
                                }
                                Err(error) => return Err(error.into()),
                            }
                        }
                        if fits && dictionary_cost < ordinary_cost {
                            dictionary_sizes = Some(sizes);
                        }
                    }
                }
                Err(ColumnarError::LimitExceeded { .. }) => {}
                Err(error) => return Err(error.into()),
            }
        }
        if dictionary_sizes.is_some() {
            single_page = None;
        }
        let dictionary_page_offset = if let Some(sizes) = dictionary_sizes {
            let offset = output.offset as i64;
            output.publish(&dictionary_page)?;
            total_uncompressed_size = sizes.total_uncompressed_size;
            total_compressed_size = sizes.total_compressed_size;
            Some(offset)
        } else {
            None
        };
        drop(dictionary_page);
        let dictionary = dictionary.filter(|_| dictionary_page_offset.is_some());
        let data_page_offset = output.offset as i64;
        let mut records = PageRecords::default();
        if dictionary.is_some() {
            records.encodings.push(ENCODING_PLAIN);
        }
        if schema.repetition == Repetition::Optional {
            records.encodings.push(ENCODING_RLE);
        }
        for (index, range) in ranges.into_iter().enumerate() {
            let (uncompressed, compressed) = emit_page(
                column,
                schema,
                &range,
                ChunkEncoding {
                    dictionary: dictionary.as_ref(),
                    selected: ordinary_encodings.get(index).copied(),
                    ..choice
                },
                output,
                &mut records,
                single_page.take(),
            )?;
            total_uncompressed_size = total_uncompressed_size
                .checked_add(uncompressed)
                .ok_or_else(table_budget_overflow)?;
            total_compressed_size = total_compressed_size
                .checked_add(compressed)
                .ok_or_else(table_budget_overflow)?;
        }
        records.encodings.sort_unstable();
        records.encodings.dedup();
        columns.push(EncodedColumn {
            physical_type: parquet_physical_type(schema.physical_type),
            encodings: records.encodings,
            path: schema.name,
            codec: compression.parquet_id(),
            num_values: rows as i64,
            total_uncompressed_size,
            total_compressed_size,
            data_page_offset,
            dictionary_page_offset,
            statistics: pruning::Statistics::collect(
                &column.plain,
                schema.physical_type,
                rows - value,
                schema.utf8,
            )?,
            pages: records.pages,
            column_index: None,
            offset_index: None,
            bloom: None,
        });
    }
    for (ordinal, column) in columns.iter_mut().enumerate() {
        let pages = core::mem::take(&mut column.pages);
        if let Some(options) = options.bloom(table, ordinal) {
            let bytes = pruning::bloom(
                &pending[ordinal].plain,
                table.schema().columns[ordinal].physical_type,
                rows - column.statistics.nulls as usize,
                options,
            )?;
            column.bloom = Some(publish_auxiliary(&bytes, output)?);
        }
        column.column_index = Some(publish_auxiliary(&pruning::column_index(&pages), output)?);
        column.offset_index = Some(publish_auxiliary(&pruning::offset_index(&pages), output)?);
    }
    Ok(EncodedGroup {
        columns,
        row_count: rows as i64,
        offset,
    })
}

fn publish_auxiliary<S, E, F>(
    bytes: &[u8],
    output: &mut Output<'_, F>,
) -> Result<pruning::Location, ColumnarSinkError<S, E>>
where
    F: FnMut(Table, &[u8]) -> Result<(), E>,
{
    let location = pruning::Location {
        offset: output.offset as i64,
        bytes: checked_i32_len(bytes.len(), "Parquet auxiliary metadata size")?,
    };
    output.publish(bytes)?;
    Ok(location)
}

fn build_page(
    column: &PendingColumn,
    schema: &ColumnSchema,
    range: &PageRange,
    choice: ChunkEncoding<'_>,
) -> Result<BuiltPage, ColumnarError> {
    let definitions = if schema.repetition == Repetition::Optional {
        let presence = range
            .rows
            .clone()
            .map(|row| column.presence.get(row))
            .collect();
        encode_definition_levels(&presence)
    } else {
        Vec::new()
    };
    let plain = column.plain_range(range.values.clone(), schema);
    let statistics = pruning::Statistics::collect(
        plain,
        schema.physical_type,
        range.rows.len() - range.values.len(),
        schema.utf8,
    )?;
    let encode = |body: &[u8], id, page: &mut Vec<u8>| {
        encode_page(
            body,
            &definitions,
            range.rows.len(),
            range.rows.len() - range.values.len(),
            super::PageEncoding {
                compression: choice.compression,
                values: id,
                statistics: Some(&statistics),
            },
            page,
        )
    };
    if let Some(dictionary) = choice.dictionary {
        let body = dictionary.encode_indexes(plain)?;
        let mut page = Vec::new();
        let sizes = encode(&body, encoding::RLE_DICTIONARY, &mut page)?;
        return Ok((page, sizes, encoding::RLE_DICTIONARY, statistics));
    }
    if let Some(id) = choice.selected {
        let mut page = Vec::new();
        let sizes = if id == ENCODING_PLAIN {
            encode(plain, id, &mut page)?
        } else {
            let body = encoding::encode_delta(id, plain, schema.physical_type)?;
            encode(&body, id, &mut page)?
        };
        return Ok((page, sizes, id, statistics));
    }
    let mut page = Vec::new();
    let mut sizes = encode(plain, ENCODING_PLAIN, &mut page)?;
    let mut selected = ENCODING_PLAIN;
    if matches!(
        choice.policy,
        EncodingPolicy::Adaptive | EncodingPolicy::Delta
    ) {
        let candidates: &[i32] = match schema.physical_type {
            PhysicalType::Int64 => &[encoding::DELTA_BINARY_PACKED],
            PhysicalType::ByteArray => &[
                encoding::DELTA_LENGTH_BYTE_ARRAY,
                encoding::DELTA_BYTE_ARRAY,
            ],
        };
        for &id in candidates {
            let body = match encoding::encode_delta(id, plain, schema.physical_type) {
                Ok(body) => body,
                Err(ColumnarError::LimitExceeded { .. }) => continue,
                Err(error) => return Err(error),
            };
            let mut candidate = Vec::new();
            if body.len() as u64 + definitions.len() as u64 > choice.physical_limit {
                continue;
            }
            let candidate_sizes = match encode(&body, id, &mut candidate) {
                Ok(sizes) => sizes,
                Err(ColumnarError::LimitExceeded { .. }) => continue,
                Err(error) => return Err(error),
            };
            if candidate.len() < page.len() {
                page = candidate;
                sizes = candidate_sizes;
                selected = id;
            }
        }
    }
    Ok((page, sizes, selected, statistics))
}

fn emit_page<S, E, F>(
    column: &PendingColumn,
    schema: &ColumnSchema,
    range: &PageRange,
    choice: ChunkEncoding<'_>,
    output: &mut Output<'_, F>,
    records: &mut PageRecords,
    cached: Option<BuiltPage>,
) -> Result<(i64, i64), ColumnarSinkError<S, E>>
where
    F: FnMut(Table, &[u8]) -> Result<(), E>,
{
    let encoded = match cached {
        Some(page) => Ok(page),
        None => build_page(column, schema, range, choice),
    };
    let (context, observed, maximum) = match encoded {
        Ok((page, encoded, id, statistics)) => {
            let header = super::parse_page_header(&page)?;
            if header.compressed_page_size as u64 <= choice.physical_limit
                && header.uncompressed_page_size as u64 <= choice.physical_limit
                && page.len() as u64 <= choice.indexed_limit
            {
                let location = pruning::Page {
                    offset: output.offset as i64,
                    bytes: checked_i32_len(page.len(), "indexed Parquet page size")?,
                    first_row: range.rows.start as i64,
                    stats: statistics,
                };
                output.publish(&page)?;
                records.encodings.push(id);
                records.pages.push(location);
                output.pages += 1;
                return Ok((
                    encoded.total_uncompressed_size,
                    encoded.total_compressed_size,
                ));
            }
            if page.len() as u64 > choice.indexed_limit {
                (
                    "single row indexed Parquet page size",
                    page.len() as u64,
                    choice.indexed_limit,
                )
            } else {
                (
                    "single row compressed Parquet page size",
                    (header.compressed_page_size as u64).max(header.uncompressed_page_size as u64),
                    choice.physical_limit,
                )
            }
        }
        Err(ColumnarError::LimitExceeded {
            context: "Parquet compressed page size",
            value,
            ..
        }) => (
            "single row compressed Parquet page size",
            value,
            choice.physical_limit,
        ),
        Err(error) => return Err(error.into()),
    };
    if range.rows.len() == 1 {
        return Err(ColumnarError::LimitExceeded {
            context,
            value: observed,
            maximum,
        }
        .into());
    }
    // No candidate bytes reached the sink, so accepted pages need no rollback.
    let middle = range.rows.start + range.rows.len() / 2;
    let values_middle = range.values.start
        + if schema.repetition == Repetition::Optional {
            (range.rows.start..middle)
                .filter(|&row| column.presence.get(row))
                .count()
        } else {
            middle - range.rows.start
        };
    let left = emit_page(
        column,
        schema,
        &PageRange {
            rows: range.rows.start..middle,
            values: range.values.start..values_middle,
        },
        ChunkEncoding {
            selected: None,
            ..choice
        },
        output,
        records,
        None,
    )?;
    let right = emit_page(
        column,
        schema,
        &PageRange {
            rows: middle..range.rows.end,
            values: values_middle..range.values.end,
        },
        ChunkEncoding {
            selected: None,
            ..choice
        },
        output,
        records,
        None,
    )?;
    Ok((
        left.0
            .checked_add(right.0)
            .ok_or_else(table_budget_overflow)?,
        left.1
            .checked_add(right.1)
            .ok_or_else(table_budget_overflow)?,
    ))
}
#[cfg(test)]
mod tests {
    use super::super::{TableReader, read_table};
    use super::*;
    use purrdf_core::{ContentStore, DatasetView, RdfDatasetBuilder, RdfLiteral};

    fn write_rows_bounded<'a>(
        table: Table,
        rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
        compression: Compression,
        physical_limit: u64,
        group_rows: u64,
    ) -> Result<Vec<u8>, ColumnarError> {
        collect_rows(
            table,
            rows,
            &WriteOptions {
                compression,
                page_bytes: physical_limit,
                row_group_rows: group_rows,
                ..WriteOptions::default()
            },
            physical_limit,
        )
    }

    #[test]
    fn small_rows_retain_column_sized_writer_scratch() {
        // Three dense integer columns and one null bitmap must not retain an
        // eleven-cell row allocation for every admitted quad.
        let rows = 16_384;
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let bytes = write_rows(
            Table::Quads,
            (0..rows).map(|id| {
                Ok(purrdf_core::smallvec![
                    ProjectionCell::Int64(id),
                    ProjectionCell::Int64(id),
                    ProjectionCell::Int64(id),
                    ProjectionCell::Null,
                ])
            }),
            Compression::Uncompressed,
        )
        .unwrap();
        let measured = window.close();
        let plain_bytes = rows * 3 * 8;
        // Includes pending PLAIN buffers, encoded output growth, bitmap and
        // footer scratch; the former row representation alone exceeds this.
        assert!(
            measured.peak_working_bytes <= plain_bytes * 8,
            "{measured:?}"
        );
        assert_eq!(
            read_table(&bytes, Table::Quads).unwrap().row_count,
            rows as usize
        );
    }

    #[test]
    fn physical_groups_have_no_chosen_table_or_metadata_count_ceiling() {
        let rows = || {
            (0..1_025).map(|id| {
                Ok(purrdf_core::smallvec![
                    ProjectionCell::Int64(id),
                    ProjectionCell::Int64(id),
                    ProjectionCell::Int64(id),
                    ProjectionCell::Null,
                ])
            })
        };
        // Three required integer cells fit one row per physical page at this bound.
        // Explicit one-row groups still exercise more than 1,024 footer records.
        let bytes =
            write_rows_bounded(Table::Quads, rows(), Compression::Uncompressed, 8, 1).unwrap();
        assert_eq!(
            bytes,
            write_rows_bounded(Table::Quads, rows(), Compression::Uncompressed, 8, 1).unwrap()
        );
        let mut reader = TableReader::open(&bytes, Table::Quads).unwrap();
        let mut groups = 0;
        while let Some(group) = reader.next_group().unwrap() {
            assert_eq!(group.row_count, 1);
            groups += 1;
        }
        assert_eq!(groups, 1_025);

        // Page locations include the header. Exact text extrema can make a
        // multirow page exceed that field even when its value body fits.
        let a = [b'a'; 64];
        let b = [b'b'; 64];
        let rows = || {
            [a.as_slice(), b.as_slice()].into_iter().map(|digest| {
                Ok(purrdf_core::smallvec![
                    ProjectionCell::Bytes(digest),
                    ProjectionCell::Bytes(b"x")
                ])
            })
        };
        let mut indexed = Vec::new();
        let report = write_rows_limited::<Infallible, Infallible>(
            Table::Blobs,
            rows(),
            &WriteOptions {
                encoding: EncodingPolicy::Plain,
                ..WriteOptions::default()
            },
            1024,
            240,
            &mut |_, bytes| {
                indexed.extend_from_slice(bytes);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(report.data_pages, 3);
        assert_eq!(
            read_table(&indexed, Table::Blobs)
                .unwrap()
                .rows()
                .collect::<Vec<_>>(),
            rows().collect::<Result<Vec<_>, _>>().unwrap()
        );
        let mut refused = Vec::new();
        assert!(matches!(
            write_rows_limited::<Infallible, Infallible>(
                Table::Blobs,
                rows().take(1),
                &WriteOptions {
                    encoding: EncodingPolicy::Plain,
                    ..WriteOptions::default()
                },
                1024,
                1,
                &mut |_, bytes| {
                    refused.extend_from_slice(bytes);
                    Ok(())
                },
            ),
            Err(ColumnarSinkError::Codec(ColumnarError::LimitExceeded {
                context: "single row indexed Parquet page size",
                maximum: 1,
                ..
            }))
        ));
        assert_eq!(refused, PARQUET_MAGIC);

        // Each optional column restarts its two-byte null run on every page.
        // One row is 24 PLAIN bytes plus 16 definition bytes, so six rows fit
        // 256 bytes and a seventh starts the next group, even though a run
        // measured across the whole group would incorrectly admit ten rows.
        let rows = (0..10).map(|id| {
            Ok(purrdf_core::smallvec![
                ProjectionCell::Int64(id),
                ProjectionCell::Int64(0),
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Null,
                ProjectionCell::Int64(0),
            ])
        });
        let bytes = collect_rows(
            Table::Terms,
            rows,
            &WriteOptions {
                page_rows: 1,
                page_bytes: 1024,
                row_group_bytes: 256,
                row_group_rows: 100,
                ..WriteOptions::default()
            },
            i32::MAX as u64,
        )
        .unwrap();
        let mut reader = TableReader::open(&bytes, Table::Terms).unwrap();
        assert_eq!(reader.next_group().unwrap().unwrap().row_count, 6);
        assert_eq!(reader.next_group().unwrap().unwrap().row_count, 4);
        assert!(reader.next_group().unwrap().is_none());

        // A later column expands under Zstd after the first column was emitted.
        // It splits into pages within the group, without retracting sink bytes.
        let first: Vec<u8> = (0..64).collect();
        let second: Vec<u8> = (128..192).collect();
        let compressed_rows = || {
            [&first, &second].into_iter().map(|payload| {
                Ok(purrdf_core::smallvec![
                    ProjectionCell::Bytes(b"a"),
                    ProjectionCell::Bytes(payload)
                ])
            })
        };
        let bytes =
            write_rows_bounded(Table::Blobs, compressed_rows(), Compression::Zstd, 136, 2).unwrap();
        let mut reader = TableReader::open(&bytes, Table::Blobs).unwrap();
        assert_eq!(reader.next_group().unwrap().unwrap().row_count, 2);
        assert!(reader.next_group().unwrap().is_none());
        assert!(
            matches!(write_rows_bounded(Table::Blobs, compressed_rows().take(1), Compression::Zstd, 68, 1), Err(ColumnarError::LimitExceeded {context: "single row compressed Parquet page size", value, maximum: 68}) if value > 68)
        );

        // Expansion splits inside both a present-byte run and a nullable
        // integer run. Empty present bytes retain their length prefix, and
        // the right child starts at its own dense value ordinal.
        let a: Vec<u8> = (0..64).collect();
        let b: Vec<u8> = (64..128).collect();
        let lex = [Some(&b""[..]), Some(a.as_slice()), Some(b.as_slice()), None];
        let datatype = [None, Some(7), Some(9), None];
        let rows: Vec<SmallVec<[_; 11]>> = (0..4)
            .map(|index| {
                purrdf_core::smallvec![
                    ProjectionCell::Int64(index as i64),
                    ProjectionCell::Int64(0),
                    lex[index].map_or(ProjectionCell::Null, ProjectionCell::Bytes),
                    datatype[index].map_or(ProjectionCell::Null, ProjectionCell::Int64),
                    ProjectionCell::Null,
                    ProjectionCell::Null,
                    ProjectionCell::Null,
                    ProjectionCell::Null,
                    ProjectionCell::Null,
                    ProjectionCell::Null,
                    ProjectionCell::Int64(0),
                ]
            })
            .collect();
        let bytes = write_rows_bounded(
            Table::Terms,
            rows.iter().cloned().map(Ok),
            Compression::Zstd,
            144,
            4,
        )
        .unwrap();
        let mut reader = TableReader::open(&bytes, Table::Terms).unwrap();
        assert_eq!(reader.next_group().unwrap().unwrap().row_count, 4);
        assert!(reader.next_group().unwrap().is_none());
        assert_eq!(
            read_table(&bytes, Table::Terms)
                .unwrap()
                .rows()
                .collect::<Vec<_>>(),
            rows
        );

        // Exercise the real planner with synthetic sizes, without allocating a
        // formerly forbidden table or a maximum-size physical page.
        let schema = &Table::Quads.schema().columns[0];
        let above_old_ceiling = Measure {
            plain: 257 * 1024 * 1024,
            ..Measure::default()
        };
        assert!(
            above_old_ceiling
                .append(ProjectionCell::Int64(0), schema)
                .unwrap()
                .fits(i32::MAX as u64)
        );
        let full = Measure {
            plain: i32::MAX as u64 - 8,
            ..Measure::default()
        };
        let full = full.append(ProjectionCell::Int64(0), schema).unwrap();
        assert!(full.fits(i32::MAX as u64));
        assert!(
            !full
                .append(ProjectionCell::Int64(0), schema)
                .unwrap()
                .fits(i32::MAX as u64)
        );
        let optional = &Table::Quads.schema().columns[3];
        let boundary = Measure {
            definitions: 2,
            run: 63,
            last: Some(false),
            ..Measure::default()
        };
        assert!(
            !boundary
                .append(ProjectionCell::Null, optional)
                .unwrap()
                .fits(2)
        );
    }

    #[test]
    fn grouped_reads_preserve_forward_terms_side_tables_and_global_order() {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("https://example.org/a").unwrap();
        let z = builder.intern_iri("https://example.org/z").unwrap();
        let inner = builder.intern_triple(z, a, z).unwrap();
        let outer = builder.intern_triple(a, a, inner).unwrap();
        let reifier = builder.intern_iri("https://example.org/r").unwrap();
        builder.push_reifier_in_graph(reifier, outer, None).unwrap();
        builder
            .push_annotation_in_graph(reifier, a, z, None)
            .unwrap();
        for index in 0..24 {
            let value = builder
                .intern_literal(RdfLiteral::typed(
                    index.to_string(),
                    "https://example.org/type",
                ))
                .unwrap();
            builder.push_quad(a, a, value, None).unwrap();
        }
        let dataset = builder.freeze().unwrap();
        let mut blobs = ContentStore::new();
        blobs.insert(b"original blob".to_vec());
        for compression in [Compression::Uncompressed, Compression::Zstd] {
            let original = crate::write(&dataset, &blobs, compression).unwrap();
            let mut files = std::array::from_fn(|_| Vec::new());
            for (file, table) in files.iter_mut().zip(Table::ALL) {
                let data = read_table(original.files.get(table), table).unwrap();
                *file =
                    write_rows_bounded(table, data.rows().map(Ok), compression, 128, 2).unwrap();
            }
            let files = crate::ParquetFiles::from_array(files);
            let restored = crate::read(&files).unwrap();
            assert_eq!(
                crate::write(restored.dataset.as_ref(), &restored.blobs, compression)
                    .unwrap()
                    .files,
                original.files
            );
            // The last statement spans physical groups and must keep its
            // original table-global ordinal through dictionary reconstruction.
            let evidence = crate::read_with_ordinals(
                &files,
                &[
                    crate::ColumnarOrdinal::Quad(23),
                    crate::ColumnarOrdinal::Term(0),
                ],
            )
            .unwrap();
            let Some(crate::ColumnarOrdinalValue::Quad(quad)) =
                evidence.get(crate::ColumnarOrdinal::Quad(23))
            else {
                panic!("requested statement")
            };
            let term_value = evidence.read().dataset.as_ref().term_value(quad.o);
            let expected = dataset
                .quads()
                .map(|quad| dataset.as_ref().term_value(quad.o))
                .max()
                .unwrap();
            assert_eq!(term_value, expected);
            assert!(evidence.get(crate::ColumnarOrdinal::Quad(0)).is_none());

            // Both physical groups are valid, but duplicating their boundary row
            // must fail semantic ordering across the released group boundary.
            let data = read_table(files.get(Table::Quads), Table::Quads).unwrap();
            let row = data.rows().next().unwrap();
            let duplicate = write_rows_bounded(
                Table::Quads,
                [Ok(row.clone()), Ok(row)],
                compression,
                i32::MAX as u64,
                1,
            )
            .unwrap();
            let mut damaged = files.into_array();
            damaged[1] = duplicate;
            let damaged = crate::ParquetFiles::from_array(damaged);
            assert!(crate::read(&damaged).is_err());
            assert!(
                crate::read_with_ordinals(&damaged, &[crate::ColumnarOrdinal::Quad(0)]).is_err()
            );
        }
    }
}
