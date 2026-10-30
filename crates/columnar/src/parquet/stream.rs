// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed canonical rows, grouped at the physical page representation boundary.

use super::{
    ColumnSchema, ColumnarError, Compression, ENCODING_PLAIN, ENCODING_RLE, EncodedColumn,
    PARQUET_MAGIC, PhysicalType, Presence, Repetition, Table, checked_i32_len,
    encode_definition_levels, encode_file_metadata, encode_page, parquet_physical_type,
    parse_page_header, table_budget_overflow,
};
use crate::ProjectionCell;
use purrdf_core::SmallVec;

pub(super) struct EncodedGroup {
    pub(super) columns: Vec<EncodedColumn>,
    pub(super) row_count: i64,
    pub(super) offset: i64,
}

/// Exact current PLAIN length and maximal definition-level RLE run sizes.
#[derive(Clone, Copy, Default)]
struct Measure {
    plain: u64,
    definitions: u64,
    run: u64,
    last: Option<bool>,
}

fn run_bytes(run: u64) -> u64 {
    // RLE stores the run length shifted left once, followed by the level byte.
    u64::from((64 - (run << 1).leading_zeros()).div_ceil(7)) + 1
}

impl Measure {
    fn append(
        self,
        cell: ProjectionCell<'_>,
        schema: &ColumnSchema,
    ) -> Result<Self, ColumnarError> {
        let present = !matches!(cell, ProjectionCell::Null);
        let bytes = match (cell, schema.physical_type) {
            (ProjectionCell::Int64(_), PhysicalType::Int64) => 8,
            (ProjectionCell::Bytes(bytes), PhysicalType::ByteArray) => {
                checked_i32_len(bytes.len(), "PLAIN BYTE_ARRAY length")?;
                bytes.len() as u64 + 4
            }
            (ProjectionCell::Null, _) if schema.repetition == Repetition::Optional => 0,
            _ => {
                return Err(ColumnarError::malformed(
                    "canonical row",
                    "cell type or required presence",
                ));
            }
        };
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
            .is_some_and(|size| size <= limit)
    }
}

pub(crate) fn write_rows<'a>(
    table: Table,
    rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
    compression: Compression,
) -> Result<Vec<u8>, ColumnarError> {
    write_rows_bounded(table, rows, compression, i32::MAX as u64)
}

fn write_rows_bounded<'a>(
    table: Table,
    rows: impl IntoIterator<Item = Result<SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
    compression: Compression,
    page_limit: u64,
) -> Result<Vec<u8>, ColumnarError> {
    let mut output = PARQUET_MAGIC.to_vec();
    let mut groups = SmallVec::new();
    let mut pending: SmallVec<[_; 1]> = SmallVec::new();
    let column_count = table.schema().columns.len();
    let mut sizes = [Measure::default(); 11];
    let mut next = sizes;
    let mut row_count = 0i64;
    for row in rows {
        let row = row?;
        if row.len() != column_count {
            return Err(ColumnarError::malformed("canonical row", "column count"));
        }
        let measure = |sizes: &[Measure], next: &mut [Measure]| -> Result<(), ColumnarError> {
            for (((&cell, schema), &size), next) in
                row.iter().zip(table.schema().columns).zip(sizes).zip(next)
            {
                *next = size.append(cell, schema)?;
            }
            Ok(())
        };
        measure(&sizes[..column_count], &mut next[..column_count])?;
        if pending.len() == i32::MAX as usize
            || next[..column_count]
                .iter()
                .any(|size| !size.fits(page_limit))
        {
            emit(
                table,
                &pending,
                compression,
                page_limit,
                &mut output,
                &mut groups,
            )?;
            pending.clear();
            sizes.fill(Measure::default());
            measure(&sizes[..column_count], &mut next[..column_count])?;
        }
        if next[..column_count]
            .iter()
            .any(|size| !size.fits(page_limit))
        {
            return Err(ColumnarError::LimitExceeded {
                context: "single row uncompressed Parquet page size",
                value: next[..column_count]
                    .iter()
                    .map(|size| size.plain + size.definitions)
                    .max()
                    .unwrap_or(0),
                maximum: page_limit,
            });
        }
        std::mem::swap(&mut sizes, &mut next);
        pending.push(row);
        row_count = row_count.checked_add(1).ok_or_else(table_budget_overflow)?;
    }
    emit(
        table,
        &pending,
        compression,
        page_limit,
        &mut output,
        &mut groups,
    )?;
    finish_file(table, row_count, &groups, output)
}

fn finish_file(
    table: Table,
    row_count: i64,
    groups: &[EncodedGroup],
    mut output: Vec<u8>,
) -> Result<Vec<u8>, ColumnarError> {
    let footer = encode_file_metadata(table, row_count, groups);
    let length = u32::try_from(footer.len()).map_err(|_| {
        ColumnarError::limit("Parquet footer length", footer.len(), u32::MAX as usize)
    })?;
    output.reserve_exact(footer.len() + 8);
    output.extend_from_slice(&footer);
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(PARQUET_MAGIC);
    Ok(output)
}

/// Compression can expand a page. Only an observed physical size failure
/// subdivides a candidate; there is no row-count tuning threshold.
fn emit(
    table: Table,
    rows: &[SmallVec<[ProjectionCell<'_>; 11]>],
    compression: Compression,
    page_limit: u64,
    output: &mut Vec<u8>,
    groups: &mut SmallVec<[EncodedGroup; 1]>,
) -> Result<(), ColumnarError> {
    if rows.is_empty() {
        return Ok(());
    }
    let start = output.len();
    let mut columns = Vec::with_capacity(table.schema().columns.len());
    for (index, schema) in table.schema().columns.iter().enumerate() {
        let mut plain = Vec::new();
        let mut presence = Presence::default();
        for row in rows {
            if schema.repetition == Repetition::Optional {
                presence.push(!matches!(row[index], ProjectionCell::Null));
            }
            match row[index] {
                ProjectionCell::Null => {}
                ProjectionCell::Int64(value) => plain.extend_from_slice(&value.to_le_bytes()),
                ProjectionCell::Bytes(bytes) => {
                    let length = checked_i32_len(bytes.len(), "PLAIN BYTE_ARRAY length")?;
                    plain.extend_from_slice(&length.to_le_bytes());
                    plain.extend_from_slice(bytes);
                }
            }
        }
        let definitions = if schema.repetition == Repetition::Optional {
            encode_definition_levels(&presence)
        } else {
            Vec::new()
        };
        let page_start = output.len();
        let encoded = match encode_page(
            plain,
            &definitions,
            rows.len(),
            if schema.repetition == Repetition::Optional {
                rows.len() - presence.count()
            } else {
                0
            },
            compression,
            output,
        ) {
            Ok(encoded) => encoded,
            Err(ColumnarError::LimitExceeded {
                context: "Parquet compressed page size",
                value,
                ..
            }) => {
                output.truncate(start);
                return subdivide(table, rows, compression, page_limit, value, output, groups);
            }
            Err(error) => return Err(error),
        };
        let header = parse_page_header(&output[page_start..])?;
        if header.compressed_page_size as u64 > page_limit {
            output.truncate(start);
            return subdivide(
                table,
                rows,
                compression,
                page_limit,
                header.compressed_page_size as u64,
                output,
                groups,
            );
        }
        i64::try_from(output.len()).map_err(|_| table_budget_overflow())?;
        columns.push(EncodedColumn {
            physical_type: parquet_physical_type(schema.physical_type),
            encodings: if schema.repetition == Repetition::Optional {
                &[ENCODING_PLAIN, ENCODING_RLE]
            } else {
                &[ENCODING_PLAIN]
            },
            path: schema.name,
            codec: compression.parquet_id(),
            num_values: rows.len() as i64,
            total_uncompressed_size: encoded.total_uncompressed_size,
            total_compressed_size: encoded.total_compressed_size,
            data_page_offset: i64::try_from(page_start).map_err(|_| table_budget_overflow())?,
        });
    }
    groups.push(EncodedGroup {
        columns,
        row_count: rows.len() as i64,
        offset: i64::try_from(start).map_err(|_| table_budget_overflow())?,
    });
    Ok(())
}

fn subdivide(
    table: Table,
    rows: &[SmallVec<[ProjectionCell<'_>; 11]>],
    compression: Compression,
    limit: u64,
    observed: u64,
    output: &mut Vec<u8>,
    groups: &mut SmallVec<[EncodedGroup; 1]>,
) -> Result<(), ColumnarError> {
    if rows.len() == 1 {
        return Err(ColumnarError::LimitExceeded {
            context: "single row compressed Parquet page size",
            value: observed,
            maximum: limit,
        });
    }
    let (left, right) = rows.split_at(rows.len() / 2);
    emit(table, left, compression, limit, output, groups)?;
    emit(table, right, compression, limit, output, groups)
}

#[cfg(test)]
mod tests {
    use super::super::{TableReader, read_table};
    use super::*;
    use purrdf_core::{ContentStore, RdfDatasetBuilder, RdfLiteral};

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
        // Four required-width cells fit one row per physical page at this bound.
        let bytes = write_rows_bounded(Table::Quads, rows(), Compression::Uncompressed, 8).unwrap();
        assert_eq!(
            bytes,
            write_rows_bounded(Table::Quads, rows(), Compression::Uncompressed, 8).unwrap()
        );
        let mut reader = TableReader::open(&bytes, Table::Quads).unwrap();
        let mut groups = 0;
        while let Some(group) = reader.next_group().unwrap() {
            assert_eq!(group.row_count, 1);
            groups += 1;
        }
        assert_eq!(groups, 1_025);

        // A later column expands under Zstd after the first column was already
        // appended. The real emitter must roll it back and encode two valid groups.
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
            write_rows_bounded(Table::Blobs, compressed_rows(), Compression::Zstd, 136).unwrap();
        let mut reader = TableReader::open(&bytes, Table::Blobs).unwrap();
        assert_eq!(reader.next_group().unwrap().unwrap().row_count, 1);
        assert_eq!(reader.next_group().unwrap().unwrap().row_count, 1);
        assert!(reader.next_group().unwrap().is_none());
        assert!(
            matches!(write_rows_bounded(Table::Blobs, compressed_rows().take(1), Compression::Zstd, 68), Err(ColumnarError::LimitExceeded {context: "single row compressed Parquet page size", value, maximum: 68}) if value > 68)
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
                *file = write_rows_bounded(table, data.rows().map(Ok), compression, 128).unwrap();
            }
            let files = crate::ParquetFiles::from_array(files);
            let restored = crate::read(&files).unwrap();
            assert_eq!(
                crate::write(restored.dataset.as_ref(), &restored.blobs, compression)
                    .unwrap()
                    .files,
                original.files
            );

            // Both physical groups are valid, but duplicating their boundary row
            // must fail semantic ordering across the released group boundary.
            let data = read_table(files.get(Table::Quads), Table::Quads).unwrap();
            let row = data.rows().next().unwrap();
            let mut duplicate = PARQUET_MAGIC.to_vec();
            let mut groups = SmallVec::new();
            for _ in 0..2 {
                emit(
                    Table::Quads,
                    std::slice::from_ref(&row),
                    compression,
                    i32::MAX as u64,
                    &mut duplicate,
                    &mut groups,
                )
                .unwrap();
            }
            let duplicate = finish_file(Table::Quads, 2, &groups, duplicate).unwrap();
            let mut damaged = files.into_array();
            damaged[1] = duplicate;
            assert!(crate::read(&crate::ParquetFiles::from_array(damaged)).is_err());
        }
    }
}
