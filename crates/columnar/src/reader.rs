// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict five-table columnar-to-RDF reconstruction.

use purrdf_core::TermBox;
use std::collections::BTreeMap;
use std::str;
use std::sync::Arc;

use purrdf_core::langtag::is_identity_folded;
use purrdf_core::{
    BlankScope, ContentDigest, ContentStore, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection, TermFactory as _, TermValue,
};

use crate::column::Int64Column;
use crate::error::{ColumnarError, ReaderReason};
use crate::files::ParquetFiles;
use crate::parquet::{ColumnValues, TableData, bounded_row_capacity, read_table};
use crate::schema::Table;

type QuadRow = (usize, usize, usize, Option<usize>);
type ReifierRow = (usize, usize, usize, usize, Option<usize>);

/// The result of a complete Parquet-to-RDF conversion.
#[derive(Debug, Clone)]
pub struct ColumnarRead {
    /// Reconstructed, structurally validated RDF 1.2 dataset.
    pub dataset: Arc<RdfDataset>,
    /// Reconstructed and digest-verified content-addressed payloads.
    pub blobs: ContentStore,
    /// Losses observed while reconstructing the dataset.
    pub losses: LossLedger,
}

/// Read PurRDF's exact five-table Parquet profile back into RDF and blob values.
///
/// This is deliberately not a general Parquet reader. In addition to validating
/// the physical file profile, it enforces the canonical dictionary and row order,
/// rejects dangling or contradictory references, verifies every blob digest, and
/// runs the reconstructed dataset through the kernel's structural validator.
///
/// # Errors
///
/// Returns [`ColumnarError`] for malformed Parquet, schema/profile drift, safety
/// budget exhaustion, invalid UTF-8 or RDF term records, non-canonical ordering,
/// invalid references, blob digest mismatches, or RDF positional violations.
pub fn read(files: &ParquetFiles) -> Result<ColumnarRead, ColumnarError> {
    let dictionary = {
        let terms = read_table(files.get(Table::Terms), Table::Terms)?;
        Dictionary::decode(&terms)?
    };
    let quad_rows = {
        let quads = read_table(files.get(Table::Quads), Table::Quads)?;
        decode_quad_rows(&quads, &dictionary.named_graphs)?
    };
    let reifier_rows = {
        let reifiers = read_table(files.get(Table::Reifiers), Table::Reifiers)?;
        decode_reifier_rows(&reifiers, &dictionary.named_graphs)?
    };
    let annotation_rows = {
        let annotations = read_table(files.get(Table::Annotations), Table::Annotations)?;
        decode_quad_rows(&annotations, &dictionary.named_graphs)?
    };
    let content_store = {
        let blobs = read_table(files.get(Table::Blobs), Table::Blobs)?;
        decode_blobs(blobs)?
    };
    let dataset = reconstruct_dataset(&dictionary, &quad_rows, &reifier_rows, &annotation_rows)?;

    Ok(ColumnarRead {
        dataset,
        blobs: content_store,
        losses: LossLedger::default(),
    })
}

#[derive(Debug, Clone)]
enum TermRecord {
    Iri(String),
    Literal {
        lexical: String,
        datatype: usize,
        language: Option<String>,
        direction: Option<RdfTextDirection>,
    },
    Blank {
        label: String,
        scope: BlankScope,
    },
    Triple {
        s: usize,
        p: usize,
        o: usize,
    },
}

struct Dictionary {
    values: Vec<TermValue>,
    named_graphs: Vec<bool>,
    value_ids: BTreeMap<TermValue, usize>,
}

impl Dictionary {
    fn decode(data: &TableData) -> Result<Self, ColumnarError> {
        let records = parse_term_records(data)?;
        let values = resolve_term_records(&records)?;
        ensure_strict_order(&values, "term dictionary order")?;

        let mut named_column = int_column(data, 10)?.rows();
        let mut named_graphs = Vec::with_capacity(bounded_row_capacity(values.len()));
        for (row, value) in values.iter().enumerate() {
            let flag = required_i64(named_column.next_row(), row, "terms.named_graph")?;
            let named = match flag {
                0 => false,
                1 => true,
                _ => {
                    return Err(ColumnarError::reader_malformed(
                        "terms.named_graph",
                        ReaderReason::NamedGraphFlag { row, flag },
                    ));
                }
            };
            if named && !matches!(value, TermValue::Iri(_) | TermValue::Blank { .. }) {
                return Err(ColumnarError::reader_malformed(
                    "terms.named_graph",
                    ReaderReason::GraphName { row },
                ));
            }
            named_graphs.push(named);
        }
        let value_ids = values
            .iter()
            .cloned()
            .enumerate()
            .map(|(id, value)| (value, id))
            .collect();
        Ok(Self {
            values,
            named_graphs,
            value_ids,
        })
    }
}

/// One term row's nullable `INT64` cells, read from each column's rows in step.
#[derive(Debug, Clone, Copy)]
struct TermInts {
    datatype: Option<i64>,
    direction: Option<i64>,
    scope: Option<i64>,
    triple_s: Option<i64>,
    triple_p: Option<i64>,
    triple_o: Option<i64>,
}

fn parse_term_records(data: &TableData) -> Result<Vec<TermRecord>, ColumnarError> {
    let mut ids = int_column(data, 0)?.rows();
    let mut kinds = int_column(data, 1)?.rows();
    let lex = bytes_column(data, 2)?;
    let mut datatypes = int_column(data, 3)?.rows();
    let languages = bytes_column(data, 4)?;
    let mut directions = int_column(data, 5)?.rows();
    let mut scopes = int_column(data, 6)?.rows();
    let mut triple_subjects = int_column(data, 7)?.rows();
    let mut triple_predicates = int_column(data, 8)?.rows();
    let mut triple_objects = int_column(data, 9)?.rows();
    let mut records = Vec::with_capacity(bounded_row_capacity(data.row_count));

    for row in 0..data.row_count {
        let id = ids.next_row();
        let kind = kinds.next_row();
        let ints = TermInts {
            datatype: datatypes.next_row(),
            direction: directions.next_row(),
            scope: scopes.next_row(),
            triple_s: triple_subjects.next_row(),
            triple_p: triple_predicates.next_row(),
            triple_o: triple_objects.next_row(),
        };
        let id = required_i64(id, row, "terms.id")?;
        if id != row as i64 {
            return Err(ColumnarError::reader_malformed(
                "terms.id",
                ReaderReason::DenseId { row, id },
            ));
        }
        let kind = required_i64(kind, row, "terms.kind")?;
        records.push(match kind {
            0 => parse_iri_record(row, lex, languages, ints)?,
            1 => parse_literal_record(row, data.row_count, lex, languages, ints)?,
            2 => parse_blank_record(row, lex, languages, ints)?,
            3 => parse_triple_record(row, data.row_count, lex, languages, ints)?,
            _ => {
                return Err(ColumnarError::Unsupported {
                    context: "terms.kind",
                    value: kind,
                });
            }
        });
    }
    Ok(records)
}

fn parse_iri_record(
    row: usize,
    lex: &[Option<Vec<u8>>],
    languages: &[Option<Vec<u8>>],
    ints: TermInts,
) -> Result<TermRecord, ColumnarError> {
    ensure_null_i64(row, ints.datatype, "terms.datatype")?;
    ensure_null_bytes(row, languages, "terms.lang")?;
    ensure_null_i64(row, ints.direction, "terms.direction")?;
    ensure_null_i64(row, ints.scope, "terms.scope")?;
    ensure_null_i64(row, ints.triple_s, "terms.triple_s")?;
    ensure_null_i64(row, ints.triple_p, "terms.triple_p")?;
    ensure_null_i64(row, ints.triple_o, "terms.triple_o")?;
    Ok(TermRecord::Iri(required_utf8(lex, row, "terms.lex")?))
}

fn parse_literal_record(
    row: usize,
    term_count: usize,
    lex: &[Option<Vec<u8>>],
    languages: &[Option<Vec<u8>>],
    ints: TermInts,
) -> Result<TermRecord, ColumnarError> {
    ensure_null_i64(row, ints.scope, "terms.scope")?;
    ensure_null_i64(row, ints.triple_s, "terms.triple_s")?;
    ensure_null_i64(row, ints.triple_p, "terms.triple_p")?;
    ensure_null_i64(row, ints.triple_o, "terms.triple_o")?;
    let direction = match ints.direction {
        None => None,
        Some(0) => Some(RdfTextDirection::Ltr),
        Some(1) => Some(RdfTextDirection::Rtl),
        Some(value) => {
            return Err(ColumnarError::Unsupported {
                context: "terms.direction",
                value,
            });
        }
    };
    Ok(TermRecord::Literal {
        lexical: required_utf8(lex, row, "terms.lex")?,
        datatype: required_term_ref(ints.datatype, row, term_count, "terms.datatype")?,
        language: optional_utf8(languages, row, "terms.lang")?,
        direction,
    })
}

fn parse_blank_record(
    row: usize,
    lex: &[Option<Vec<u8>>],
    languages: &[Option<Vec<u8>>],
    ints: TermInts,
) -> Result<TermRecord, ColumnarError> {
    ensure_null_i64(row, ints.datatype, "terms.datatype")?;
    ensure_null_bytes(row, languages, "terms.lang")?;
    ensure_null_i64(row, ints.direction, "terms.direction")?;
    ensure_null_i64(row, ints.triple_s, "terms.triple_s")?;
    ensure_null_i64(row, ints.triple_p, "terms.triple_p")?;
    ensure_null_i64(row, ints.triple_o, "terms.triple_o")?;
    let scope = required_i64(ints.scope, row, "terms.scope")?;
    let scope = u32::try_from(scope).map_err(|cause| {
        ColumnarError::reader_malformed("terms.scope", ReaderReason::Scope { row, scope, cause })
    })?;
    Ok(TermRecord::Blank {
        label: required_utf8(lex, row, "terms.lex")?,
        scope: BlankScope(scope),
    })
}

fn parse_triple_record(
    row: usize,
    term_count: usize,
    lex: &[Option<Vec<u8>>],
    languages: &[Option<Vec<u8>>],
    ints: TermInts,
) -> Result<TermRecord, ColumnarError> {
    ensure_null_bytes(row, lex, "terms.lex")?;
    ensure_null_i64(row, ints.datatype, "terms.datatype")?;
    ensure_null_bytes(row, languages, "terms.lang")?;
    ensure_null_i64(row, ints.direction, "terms.direction")?;
    ensure_null_i64(row, ints.scope, "terms.scope")?;
    Ok(TermRecord::Triple {
        s: required_term_ref(ints.triple_s, row, term_count, "terms.triple_s")?,
        p: required_term_ref(ints.triple_p, row, term_count, "terms.triple_p")?,
        o: required_term_ref(ints.triple_o, row, term_count, "terms.triple_o")?,
    })
}

fn resolve_term_records(records: &[TermRecord]) -> Result<Vec<TermValue>, ColumnarError> {
    let mut states = vec![0u8; records.len()];
    let mut values = vec![None; records.len()];
    for index in 0..records.len() {
        resolve_term_record(index, records, &mut states, &mut values)?;
    }
    values
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            ColumnarError::reader_malformed("term dictionary", ReaderReason::UnresolvedTerm)
        })
}

/// Resolve term record `index`, memoised in `values` with `states` marking the records
/// being resolved (`1`) and resolved (`2`).
///
/// The walk runs over a work list of records being resolved: a record is entered by
/// first consulting `values`, then refusing a record already being resolved (a
/// cycle); a literal's datatype record is resolved fully before the literal is
/// checked, and a triple term's subject, predicate and object records each fully, in
/// that order, before the triple is assembled. Every record is memoised as soon as its
/// value exists, and the first refusal ends the walk.
fn resolve_term_record(
    index: usize,
    records: &[TermRecord],
    states: &mut [u8],
    values: &mut [Option<TermValue>],
) -> Result<TermValue, ColumnarError> {
    /// A record being resolved, and the values its dependencies resolved to so far.
    struct Frame {
        index: usize,
        record: TermRecord,
        resolved: Vec<TermValue>,
    }
    /// The records `record` depends on, in resolution order.
    fn dependencies(record: &TermRecord) -> ([usize; 3], usize) {
        match *record {
            TermRecord::Literal { datatype, .. } => ([datatype, 0, 0], 1),
            TermRecord::Triple { s, p, o } => ([s, p, o], 3),
            TermRecord::Iri(_) | TermRecord::Blank { .. } => ([0; 3], 0),
        }
    }
    /// Enter record `index`: its memoised value, or a new frame for it.
    fn enter(
        index: usize,
        records: &[TermRecord],
        states: &mut [u8],
        values: &[Option<TermValue>],
    ) -> Result<Result<TermValue, Frame>, ColumnarError> {
        if let Some(value) = &values[index] {
            return Ok(Ok(value.clone()));
        }
        if states[index] == 1 {
            return Err(ColumnarError::reader_malformed(
                "term dictionary",
                ReaderReason::CyclicTriple,
            ));
        }
        states[index] = 1;
        Ok(Err(Frame {
            index,
            record: records[index].clone(),
            resolved: Vec::new(),
        }))
    }
    let mut frames = match enter(index, records, states, values)? {
        Ok(value) => return Ok(value),
        Err(frame) => vec![frame],
    };
    loop {
        let frame = frames.last_mut().expect("a record is being resolved");
        let (dependencies, count) = dependencies(&frame.record);
        if frame.resolved.len() < count {
            match enter(dependencies[frame.resolved.len()], records, states, values)? {
                Ok(value) => frame.resolved.push(value),
                Err(inner) => frames.push(inner),
            }
            continue;
        }
        let Frame {
            index,
            record,
            resolved,
        } = frames.pop().expect("a record is being resolved");
        let mut resolved = resolved.into_iter();
        let value = match record {
            TermRecord::Iri(iri) => TermValue::Iri(iri),
            TermRecord::Literal {
                lexical,
                datatype: _,
                language,
                direction,
            } => {
                let datatype = resolved.next().expect("a literal's datatype is resolved");
                let TermValue::Iri(datatype) = datatype else {
                    return Err(ColumnarError::reader_malformed(
                        "terms.datatype",
                        ReaderReason::LiteralDatatype { row: index },
                    ));
                };
                if let Some(tag) = &language {
                    if tag.is_empty() || !is_identity_folded(tag) {
                        return Err(ColumnarError::reader_malformed(
                            "terms.lang",
                            ReaderReason::LanguageTag {
                                row: index,
                                language: language.expect("language was present"),
                            },
                        ));
                    }
                    let expected = RdfLiteral::language_datatype_iri(direction);
                    if datatype != expected {
                        return Err(ColumnarError::reader_malformed(
                            "terms.datatype",
                            ReaderReason::LanguageDatatype {
                                row: index,
                                actual: datatype,
                                expected,
                            },
                        ));
                    }
                } else if direction.is_some() {
                    return Err(ColumnarError::reader_malformed(
                        "terms.direction",
                        ReaderReason::DirectionWithoutLanguage { row: index },
                    ));
                }
                TermValue::Literal {
                    lexical_form: lexical,
                    datatype,
                    language,
                    direction,
                }
            }
            TermRecord::Blank { label, scope } => TermValue::Blank { label, scope },
            TermRecord::Triple { .. } => TermValue::Triple {
                s: TermBox::new(resolved.next().expect("a triple's subject is resolved")),
                p: TermBox::new(resolved.next().expect("a triple's predicate is resolved")),
                o: TermBox::new(resolved.next().expect("a triple's object is resolved")),
            },
        };
        states[index] = 2;
        values[index] = Some(value.clone());
        match frames.last_mut() {
            Some(parent) => parent.resolved.push(value),
            None => return Ok(value),
        }
    }
}

fn decode_quad_rows(
    data: &TableData,
    named_graphs: &[bool],
) -> Result<Vec<QuadRow>, ColumnarError> {
    let mut first = int_column(data, 0)?.rows();
    let mut second = int_column(data, 1)?.rows();
    let mut third = int_column(data, 2)?.rows();
    let mut graphs = int_column(data, 3)?.rows();
    let mut rows = Vec::with_capacity(bounded_row_capacity(data.row_count));
    for row in 0..data.row_count {
        let (a, b, c, g) = (
            first.next_row(),
            second.next_row(),
            third.next_row(),
            graphs.next_row(),
        );
        rows.push((
            required_term_ref(a, row, named_graphs.len(), "row first term")?,
            required_term_ref(b, row, named_graphs.len(), "row second term")?,
            required_term_ref(c, row, named_graphs.len(), "row third term")?,
            optional_graph_ref(g, row, named_graphs)?,
        ));
    }
    ensure_strict_order(&rows, "quad-like row order")?;
    Ok(rows)
}

fn decode_reifier_rows(
    data: &TableData,
    named_graphs: &[bool],
) -> Result<Vec<ReifierRow>, ColumnarError> {
    let mut reifiers = int_column(data, 0)?.rows();
    let mut subjects = int_column(data, 1)?.rows();
    let mut predicates = int_column(data, 2)?.rows();
    let mut objects = int_column(data, 3)?.rows();
    let mut graphs = int_column(data, 4)?.rows();
    let mut rows = Vec::with_capacity(bounded_row_capacity(data.row_count));
    for row in 0..data.row_count {
        let (r, s, p, o, g) = (
            reifiers.next_row(),
            subjects.next_row(),
            predicates.next_row(),
            objects.next_row(),
            graphs.next_row(),
        );
        rows.push((
            required_term_ref(r, row, named_graphs.len(), "reifiers.reifier")?,
            required_term_ref(s, row, named_graphs.len(), "reifiers.s")?,
            required_term_ref(p, row, named_graphs.len(), "reifiers.p")?,
            required_term_ref(o, row, named_graphs.len(), "reifiers.o")?,
            optional_graph_ref(g, row, named_graphs)?,
        ));
    }
    ensure_strict_order(&rows, "reifier row order")?;
    Ok(rows)
}

/// Verify every decoded payload, retaining its original digest cause and row.
fn decode_blobs(mut data: TableData) -> Result<ContentStore, ColumnarError> {
    // Preserve column-refusal order before splitting the independently owned
    // columns. The decoded payload moves into its final store after validation.
    bytes_column(&data, 0)?;
    bytes_column(&data, 1)?;
    let (digest_columns, payload_columns) = data.columns.split_at_mut(1);
    let ColumnValues::ByteArray(digests) = &mut digest_columns[0] else {
        unreachable!("digest column was checked above")
    };
    let ColumnValues::ByteArray(payloads) = &mut payload_columns[0] else {
        unreachable!("payload column was checked above")
    };
    let mut previous = None;
    let mut store = ContentStore::new();
    for row in 0..data.row_count {
        let digest_bytes = required_bytes(digests, row, "blobs.digest")?;
        let digest_text = match str::from_utf8(digest_bytes) {
            Ok(text) => text,
            Err(cause) => {
                let bytes = digests[row].take().expect("required digest is present");
                return Err(ColumnarError::reader_malformed(
                    "blobs.digest",
                    ReaderReason::DigestUtf8 { row, cause, bytes },
                ));
            }
        };
        let Some(digest) = ContentDigest::from_hex(digest_text) else {
            let bytes = digests[row].take().expect("required digest is present");
            return Err(ColumnarError::reader_malformed(
                "blobs.digest",
                ReaderReason::DigestShape { row, bytes },
            ));
        };
        // Parsing established exactly 64 ASCII hexadecimal digits.
        if digest_text.bytes().any(|byte| byte.is_ascii_uppercase()) {
            let bytes = digests[row].take().expect("required digest is present");
            return Err(ColumnarError::reader_malformed(
                "blobs.digest",
                ReaderReason::DigestCase { row, bytes },
            ));
        }
        if previous.is_some_and(|value| value >= digest) {
            return Err(ColumnarError::reader_malformed(
                "blob row order",
                ReaderReason::DigestOrder,
            ));
        }
        required_bytes(payloads, row, "blobs.bytes")?;
        let payload = payloads[row].take().expect("required payload is present");
        store
            .insert_checked(digest, payload)
            .map_err(|cause| ColumnarError::BlobContent { row, cause })?;
        previous = Some(digest);
    }
    Ok(store)
}

/// Freeze native rows without extracting or rendering the kernel diagnostic.
fn reconstruct_dataset(
    dictionary: &Dictionary,
    quads: &[QuadRow],
    reifiers: &[ReifierRow],
    annotations: &[QuadRow],
) -> Result<Arc<RdfDataset>, ColumnarError> {
    let mut builder = RdfDatasetBuilder::new();
    let ids: Vec<_> = dictionary
        .values
        .iter()
        .map(|value| builder.intern_value(value))
        .collect::<Result<_, _>>()?;
    for (index, &named) in dictionary.named_graphs.iter().enumerate() {
        if named {
            builder.declare_named_graph(ids[index])?;
        }
    }
    for &(s, p, o, g) in quads {
        builder.push_quad(ids[s], ids[p], ids[o], g.map(|id| ids[id]))?;
    }
    for &(reifier, s, p, o, g) in reifiers {
        let triple_value = TermValue::Triple {
            s: TermBox::new(dictionary.values[s].clone()),
            p: TermBox::new(dictionary.values[p].clone()),
            o: TermBox::new(dictionary.values[o].clone()),
        };
        let triple = dictionary.value_ids.get(&triple_value).ok_or_else(|| {
            ColumnarError::reader_malformed("reifier binding", ReaderReason::MissingTriple)
        })?;
        builder.push_reifier_in_graph(ids[reifier], ids[*triple], g.map(|id| ids[id]))?;
    }
    for &(reifier, predicate, value, graph) in annotations {
        builder.push_annotation_in_graph(
            ids[reifier],
            ids[predicate],
            ids[value],
            graph.map(|id| ids[id]),
        )?;
    }
    builder.freeze().map_err(ColumnarError::Reconstruction)
}

fn int_column(data: &TableData, index: usize) -> Result<&Int64Column, ColumnarError> {
    match data.columns.get(index) {
        Some(ColumnValues::Int64(values)) => Ok(values),
        _ => Err(ColumnarError::reader_malformed(
            "table column",
            ReaderReason::ColumnType {
                table: data.table,
                column: index,
                expected: crate::PhysicalType::Int64,
            },
        )),
    }
}

fn bytes_column(data: &TableData, index: usize) -> Result<&[Option<Vec<u8>>], ColumnarError> {
    match data.columns.get(index) {
        Some(ColumnValues::ByteArray(values)) => Ok(values),
        _ => Err(ColumnarError::reader_malformed(
            "table column",
            ReaderReason::ColumnType {
                table: data.table,
                column: index,
                expected: crate::PhysicalType::ByteArray,
            },
        )),
    }
}

fn required_i64(
    value: Option<i64>,
    row: usize,
    context: &'static str,
) -> Result<i64, ColumnarError> {
    value.ok_or_else(|| ColumnarError::reader_malformed(context, ReaderReason::Null { row }))
}

fn required_bytes<'a>(
    column: &'a [Option<Vec<u8>>],
    row: usize,
    context: &'static str,
) -> Result<&'a [u8], ColumnarError> {
    column[row]
        .as_deref()
        .ok_or_else(|| ColumnarError::reader_malformed(context, ReaderReason::Null { row }))
}

fn required_utf8(
    column: &[Option<Vec<u8>>],
    row: usize,
    context: &'static str,
) -> Result<String, ColumnarError> {
    let value = required_bytes(column, row, context)?;
    str::from_utf8(value).map(str::to_owned).map_err(|cause| {
        ColumnarError::reader_malformed(context, ReaderReason::RowUtf8 { row, cause })
    })
}

fn optional_utf8(
    column: &[Option<Vec<u8>>],
    row: usize,
    context: &'static str,
) -> Result<Option<String>, ColumnarError> {
    column[row]
        .as_deref()
        .map(|value| {
            str::from_utf8(value).map(str::to_owned).map_err(|cause| {
                ColumnarError::reader_malformed(context, ReaderReason::RowUtf8 { row, cause })
            })
        })
        .transpose()
}

fn required_term_ref(
    value: Option<i64>,
    row: usize,
    term_count: usize,
    context: &'static str,
) -> Result<usize, ColumnarError> {
    let value = required_i64(value, row, context)?;
    let id = usize::try_from(value).map_err(|cause| {
        ColumnarError::reader_malformed(context, ReaderReason::TermId { row, value, cause })
    })?;
    if id >= term_count {
        return Err(ColumnarError::reader_malformed(
            context,
            ReaderReason::TermReference {
                row,
                id,
                term_count,
            },
        ));
    }
    Ok(id)
}

fn optional_graph_ref(
    value: Option<i64>,
    row: usize,
    named_graphs: &[bool],
) -> Result<Option<usize>, ColumnarError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let graph = usize::try_from(value).map_err(|cause| {
        ColumnarError::reader_malformed(
            "graph reference",
            ReaderReason::GraphId { row, value, cause },
        )
    })?;
    if graph >= named_graphs.len() {
        return Err(ColumnarError::reader_malformed(
            "graph reference",
            ReaderReason::MissingGraph {
                row,
                id: graph,
                term_count: named_graphs.len(),
            },
        ));
    }
    if !named_graphs[graph] {
        return Err(ColumnarError::reader_malformed(
            "graph reference",
            ReaderReason::UndeclaredGraph { row, id: graph },
        ));
    }
    Ok(Some(graph))
}

fn ensure_null_i64(
    row: usize,
    value: Option<i64>,
    context: &'static str,
) -> Result<(), ColumnarError> {
    if value.is_some() {
        return Err(ColumnarError::reader_malformed(
            context,
            ReaderReason::ExpectedNull { row },
        ));
    }
    Ok(())
}

fn ensure_null_bytes(
    row: usize,
    column: &[Option<Vec<u8>>],
    context: &'static str,
) -> Result<(), ColumnarError> {
    if column[row].is_some() {
        return Err(ColumnarError::reader_malformed(
            context,
            ReaderReason::ExpectedNull { row },
        ));
    }
    Ok(())
}

fn ensure_strict_order<T: Ord>(values: &[T], context: &'static str) -> Result<(), ColumnarError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ColumnarError::reader_malformed(
            context,
            ReaderReason::RowOrder,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn decoded_blob_transfer_keeps_the_original_payload_without_size_growth() {
        let mut previous = None;
        for length in [1_024, 1_048_576] {
            let payload = vec![b'x'; length];
            let original = payload.as_ptr();
            let digest = ContentDigest::of(&payload);
            let data = TableData::new(
                Table::Blobs,
                vec![
                    ColumnValues::bytes(vec![Some(digest.to_hex().into_bytes())]),
                    ColumnValues::bytes(vec![Some(payload)]),
                ],
            )
            .unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = decode_blobs(data);
            let measured = window.close();
            let store = result.unwrap();
            let retained = store.get(&digest).unwrap();
            assert_eq!(retained.as_ptr(), original);
            assert_eq!(retained.len(), length);
            assert!(retained.iter().all(|byte| *byte == b'x'));
            assert!(measured.requested_bytes < length as u64);
            let traffic = (measured.allocations, measured.requested_bytes);
            eprintln!(
                "blob bytes={length}: allocations={}, requested={}",
                traffic.0, traffic.1
            );
            if let Some(previous) = previous {
                assert_eq!(
                    traffic, previous,
                    "payload length must not add allocator traffic"
                );
            }
            previous = Some(traffic);
        }
    }

    use purrdf_core::{
        BlankScope, DatasetView, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
        datasets_isomorphic,
    };

    use super::*;
    use crate::parquet::{Compression, write_table};
    use crate::writer::write;

    fn fixture() -> (Arc<RdfDataset>, ContentStore) {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("subject", BlankScope(3)).unwrap();
        let predicate = builder.intern_iri("https://example.org/p").unwrap();
        let object = builder
            .intern_literal(RdfLiteral {
                lexical_form: "bonjour".to_owned(),
                datatype: None,
                language: Some("fr".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            })
            .unwrap();
        let graph = builder.intern_iri("https://example.org/graph").unwrap();
        let empty_graph = builder.intern_iri("https://example.org/empty").unwrap();
        builder.declare_named_graph(empty_graph).unwrap();
        builder
            .push_quad(subject, predicate, object, Some(graph))
            .unwrap();
        let triple = builder.intern_triple(subject, predicate, object).unwrap();
        let reifier = builder.intern_blank("reifier", BlankScope(3)).unwrap();
        builder
            .push_reifier_in_graph(reifier, triple, Some(graph))
            .unwrap();
        builder
            .push_annotation_in_graph(reifier, predicate, object, None)
            .unwrap();

        let mut blobs = ContentStore::new();
        blobs.insert(b"first".to_vec());
        blobs.insert(b"second".to_vec());
        (builder.freeze().unwrap(), blobs)
    }

    #[test]
    fn full_conversion_round_trips_and_rewrites_identically() {
        let (dataset, blobs) = fixture();
        let encoded = write(&*dataset, &blobs, Compression::Zstd).unwrap();
        let decoded = read(&encoded.files).unwrap();
        assert!(datasets_isomorphic(&dataset, &decoded.dataset).unwrap());
        assert_eq!(decoded.blobs, blobs);
        assert!(encoded.losses.is_empty());
        assert!(decoded.losses.is_empty());

        let graph_names: Vec<_> = decoded
            .dataset
            .named_graphs()
            .map(|id| match decoded.dataset.as_ref().resolve(id) {
                purrdf_core::TermRef::Iri(iri) => iri.to_owned(),
                _ => panic!("fixture graph names are IRIs"),
            })
            .collect();
        assert_eq!(graph_names.len(), 2);
        assert!(graph_names.iter().any(|iri| iri.ends_with("/empty")));

        let rewritten = write(&*decoded.dataset, &decoded.blobs, Compression::Zstd).unwrap();
        assert_eq!(rewritten.files, encoded.files);
    }

    #[test]
    fn semantic_reader_rejects_non_dense_term_ids() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let (dataset, blobs) = fixture();
        let encoded = write(&*dataset, &blobs, Compression::Uncompressed).unwrap();
        read(&encoded.files).unwrap();
        let mut array = encoded.files.into_array();
        let mut terms = read_table(&array[0], Table::Terms).unwrap();
        let ColumnValues::Int64(ids) = &mut terms.columns[0] else {
            panic!("terms.id is INT64");
        };
        ids.values_mut()[0] = i64::MAX;
        let ColumnValues::Int64(kinds) = &mut terms.columns[1] else {
            panic!("terms.kind is INT64");
        };
        kinds.values_mut()[0] = i64::MAX;
        array[0] = write_table(&terms, Compression::Uncompressed).unwrap();
        let files = ParquetFiles::from_array(array);
        let error = read(&files).unwrap_err();
        drop(files);
        drop(terms);
        let captured = Capture::read(&error);
        assert!(matches!(
            captured.field("context"),
            ErrorValue::Code("terms.id")
        ));
        let reason = captured.child("reason");
        assert_eq!(
            reason.code, "columnar-dense-id",
            "id refusal precedes the invalid kind"
        );
        assert!(matches!(reason.field("row"), ErrorValue::Unsigned(0)));
        assert!(
            matches!(reason.field("id"), ErrorValue::Signed(value) if value == i128::from(i64::MAX))
        );

        // Mutate decoded columns, so physical certification cannot mask the
        // original signed scope/reference or the reader's own refusal order.
        for (column, value, code, field) in [
            (6, i64::MAX, "columnar-scope", "scope"),
            (3, -1, "columnar-term-id", "value"),
            (3, i64::MAX, "columnar-term-reference", "id"),
            (10, i64::MAX, "columnar-named-graph-flag", "flag"),
        ] {
            let mut array = write(&*dataset, &blobs, Compression::Uncompressed)
                .unwrap()
                .files
                .into_array();
            let mut terms = read_table(&array[0], Table::Terms).unwrap();
            let ColumnValues::Int64(values) = &mut terms.columns[column] else {
                panic!("the selected column is INT64");
            };
            let row = values.rows().position(|value| value.is_some()).unwrap();
            values.values_mut()[0] = value;
            array[0] = write_table(&terms, Compression::Uncompressed).unwrap();
            let files = ParquetFiles::from_array(array);
            let error = read(&files).unwrap_err();
            drop(files);
            drop(terms);
            let captured = Capture::read(&error).child("reason");
            assert_eq!(captured.code, code);
            assert!(
                matches!(captured.field("row"), ErrorValue::Unsigned(actual) if actual == row as u128)
            );
            if field == "id" {
                assert!(
                    matches!(captured.field(field), ErrorValue::Unsigned(actual) if actual == value as u128)
                );
            } else {
                assert!(
                    matches!(captured.field(field), ErrorValue::Signed(actual) if actual == i128::from(value))
                );
            }
            if matches!(code, "columnar-scope" | "columnar-term-id") {
                assert_eq!(captured.child("cause").code, "integer-conversion-range");
                let reason = std::error::Error::source(&error).unwrap();
                assert!(reason.source().unwrap().is::<std::num::TryFromIntError>());
            }
        }
    }

    /// Reject changed payloads through their original row-bearing integrity cause.
    #[test]
    fn semantic_reader_rehashes_blob_payloads() {
        let (dataset, blobs) = fixture();
        let encoded = write(&*dataset, &blobs, Compression::Uncompressed).unwrap();
        let mut array = encoded.files.into_array();
        let mut blob_table = read_table(&array[4], Table::Blobs).unwrap();
        let ColumnValues::ByteArray(payloads) = &mut blob_table.columns[1] else {
            panic!("blobs.bytes is BYTE_ARRAY");
        };
        payloads[0].as_mut().unwrap().push(0xff);
        array[4] = write_table(&blob_table, Compression::Uncompressed).unwrap();
        assert!(matches!(
            read(&ParquetFiles::from_array(array)),
            Err(ColumnarError::BlobContent { row: 0, .. })
        ));

        let digest = ContentDigest::of(b"digest case neighbor").to_hex();
        for (bytes, code) in [
            (vec![0xff; 64], "columnar-digest-utf8"),
            (vec![b'g'; 64], "columnar-digest-shape"),
            (
                digest.to_ascii_uppercase().into_bytes(),
                "columnar-digest-case",
            ),
        ] {
            let pointer = bytes.as_ptr();
            let expected = bytes.clone();
            let data = TableData::new(
                Table::Blobs,
                vec![
                    ColumnValues::bytes(vec![Some(bytes)]),
                    ColumnValues::bytes(vec![Some(Vec::new())]),
                ],
            )
            .unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let error = decode_blobs(data).unwrap_err();
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            let reason = purrdf_testkit::error_capture::Capture::read(&error).child("reason");
            assert_eq!(reason.code, code);
            assert!(
                matches!(reason.field("bytes"), purrdf_hash::error::ErrorValue::Bytes(bytes) if bytes == expected && bytes.as_ptr() == pointer)
            );
            if code == "columnar-digest-utf8" {
                assert_eq!(reason.child("cause").code, "std-utf8");
                let reason = std::error::Error::source(&error).unwrap();
                assert!(reason.source().unwrap().is::<str::Utf8Error>());
            }
        }
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The term-record resolution against its recursive reference, and deep on a
    //! 128 KiB thread. Re-interning a resolved value is the builder's own
    //! `TermFactory::intern_value`, tested beside it in `purrdf-core`.

    use purrdf_core::langtag::is_identity_folded;
    use purrdf_core::{BlankScope, RdfLiteral, RdfTextDirection, TermBox, TermValue};

    use super::{
        ColumnarError, ReaderReason, TermRecord, resolve_term_record, resolve_term_records,
    };

    fn reference_record(
        index: usize,
        records: &[TermRecord],
        states: &mut [u8],
        values: &mut [Option<TermValue>],
    ) -> Result<TermValue, ColumnarError> {
        if let Some(value) = &values[index] {
            return Ok(value.clone());
        }
        if states[index] == 1 {
            return Err(ColumnarError::reader_malformed(
                "term dictionary",
                ReaderReason::CyclicTriple,
            ));
        }
        states[index] = 1;
        let record = records[index].clone();
        let value = match record {
            TermRecord::Iri(iri) => TermValue::Iri(iri),
            TermRecord::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype = reference_record(datatype, records, states, values)?;
                let TermValue::Iri(datatype) = datatype else {
                    return Err(ColumnarError::reader_malformed(
                        "terms.datatype",
                        ReaderReason::LiteralDatatype { row: index },
                    ));
                };
                if let Some(tag) = &language {
                    if tag.is_empty() || !is_identity_folded(tag) {
                        return Err(ColumnarError::reader_malformed(
                            "terms.lang",
                            ReaderReason::LanguageTag {
                                row: index,
                                language: language.expect("language was present"),
                            },
                        ));
                    }
                    let expected = RdfLiteral::language_datatype_iri(direction);
                    if datatype != expected {
                        return Err(ColumnarError::reader_malformed(
                            "terms.datatype",
                            ReaderReason::LanguageDatatype {
                                row: index,
                                actual: datatype,
                                expected,
                            },
                        ));
                    }
                } else if direction.is_some() {
                    return Err(ColumnarError::reader_malformed(
                        "terms.direction",
                        ReaderReason::DirectionWithoutLanguage { row: index },
                    ));
                }
                TermValue::Literal {
                    lexical_form: lexical,
                    datatype,
                    language,
                    direction,
                }
            }
            TermRecord::Blank { label, scope } => TermValue::Blank { label, scope },
            TermRecord::Triple { s, p, o } => TermValue::Triple {
                s: TermBox::new(reference_record(s, records, states, values)?),
                p: TermBox::new(reference_record(p, records, states, values)?),
                o: TermBox::new(reference_record(o, records, states, values)?),
            },
        };
        states[index] = 2;
        values[index] = Some(value.clone());
        Ok(value)
    }

    /// A generated term-record table: records naming any record — cycles included — and
    /// literals whose datatype, tag and direction are sometimes inconsistent, so every
    /// refusal is met.
    fn generated(seed: u64) -> Vec<TermRecord> {
        let mut rng = purrdf_testkit::rng::SplitMix64::new(seed);
        let len = 1 + rng.below_usize(9);
        (0..len)
            .map(|index| match rng.below_usize(6) {
                0 => TermRecord::Iri(
                    [
                        "http://example.org/i",
                        purrdf_core::vocab::rdf::LANG_STRING,
                        purrdf_core::vocab::rdf::DIR_LANG_STRING,
                    ][index % 3]
                        .to_owned(),
                ),
                1 => TermRecord::Blank {
                    label: "b".to_owned(),
                    scope: BlankScope::DEFAULT,
                },
                2 | 3 => TermRecord::Literal {
                    lexical: "x".to_owned(),
                    datatype: rng.below_usize(len),
                    language: [None, Some("en"), Some("EN")][rng.below_usize(3)].map(str::to_owned),
                    direction: [None, Some(RdfTextDirection::Ltr)][rng.below_usize(2)],
                },
                _ => TermRecord::Triple {
                    s: rng.below_usize(len),
                    p: rng.below_usize(len),
                    o: rng.below_usize(len),
                },
            })
            .collect()
    }

    /// Every generated record table resolves — every record, in order — to exactly the
    /// values, the memo and the refusal the recursive reference reaches.
    /// A literal row naming `rdf:langString` with the given tag.
    fn language_literal_table(tag: &str) -> Vec<TermRecord> {
        vec![
            TermRecord::Iri(purrdf_core::vocab::rdf::LANG_STRING.to_owned()),
            TermRecord::Literal {
                lexical: "chat".to_owned(),
                datatype: 0,
                language: Some(tag.to_owned()),
                direction: None,
            },
        ]
    }

    #[test]
    fn a_language_tag_with_an_ascii_uppercase_letter_is_refused_as_non_canonical() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let records = language_literal_table("en-GB");
        let refused = resolve_term_records(&records)
            .expect_err("an unfolded tag is not the canonical identity");
        drop(records);
        let reason = Capture::read(&refused).child("reason");
        assert!(matches!(
            reason.field("language"),
            ErrorValue::Text("en-GB")
        ));
        assert!(
            refused.to_string().contains("non-canonical language tag"),
            "{refused}"
        );

        let mut records = language_literal_table("en-gb");
        records[0] = TermRecord::Iri("https://example.org/original-datatype".to_owned());
        let refused = resolve_term_records(&records).unwrap_err();
        drop(records);
        let reason = Capture::read(&refused).child("reason");
        assert_eq!(reason.code, "columnar-language-datatype");
        assert!(matches!(
            reason.field("actual"),
            ErrorValue::Iri("https://example.org/original-datatype")
        ));
        assert!(
            matches!(reason.field("expected"), ErrorValue::Iri(iri) if iri == purrdf_core::vocab::rdf::LANG_STRING)
        );
    }

    #[test]
    fn an_identity_folded_language_tag_resolves() {
        let values = resolve_term_records(&language_literal_table("en-gb"))
            .expect("a folded tag is canonical");
        assert_eq!(
            values[1],
            TermValue::Literal {
                lexical_form: "chat".to_owned(),
                datatype: purrdf_core::vocab::rdf::LANG_STRING.to_owned(),
                language: Some("en-gb".to_owned()),
                direction: None,
            }
        );
    }

    #[test]
    fn resolution_agrees_with_its_recursive_reference_on_generated_tables() {
        let (mut resolved, mut refused) = (0, 0);
        for seed in 0..500_u64 {
            let records = generated(seed);
            let n = records.len();
            let (mut states, mut values) = (vec![0u8; n], vec![None; n]);
            let (mut expected_states, mut expected_values) = (vec![0u8; n], vec![None; n]);
            let mut table_refused = false;
            for index in 0..n {
                let found = resolve_term_record(index, &records, &mut states, &mut values);
                let expected: Result<TermValue, ColumnarError> =
                    reference_record(index, &records, &mut expected_states, &mut expected_values);
                assert_eq!(
                    format!("{found:?}"),
                    format!("{expected:?}"),
                    "seed {seed}, record {index}"
                );
                if found.is_err() {
                    refused += 1;
                    table_refused = true;
                    break;
                }
                resolved += 1;
                assert_eq!(values, expected_values, "seed {seed}, record {index}");
            }
            // The whole table resolves exactly when every record does, to every value.
            match resolve_term_records(&records) {
                Ok(whole) => {
                    assert!(!table_refused, "seed {seed}");
                    let expected: Vec<TermValue> = expected_values
                        .into_iter()
                        .map(|value| value.expect("resolved"))
                        .collect();
                    assert_eq!(whole, expected, "seed {seed}");
                }
                Err(_) => assert!(table_refused, "seed {seed}"),
            }
        }
        assert!(resolved > 0, "some generated record resolves");
        assert!(refused > 0, "some generated table is refused");
    }

    /// A record chain a thousand triple terms deep resolves on a thread whose whole
    /// stack is 128 KiB — more levels than a recursion could take there. Every record is
    /// memoised as its whole value, so a chain costs memory with the square of its depth
    /// and is kept to a thousand levels.
    #[test]
    fn a_deep_record_chain_resolves_on_a_128_kib_thread() {
        const LEVELS: usize = 1_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let mut records = vec![
                TermRecord::Iri("http://example.org/s".to_owned()),
                TermRecord::Iri("http://example.org/p".to_owned()),
                TermRecord::Iri("http://example.org/o".to_owned()),
            ];
            for level in 0..LEVELS {
                records.push(TermRecord::Triple {
                    s: 0,
                    p: 1,
                    o: level + 2,
                });
            }
            let root = records.len() - 1;
            let (mut states, mut values) = (vec![0u8; records.len()], vec![None; records.len()]);
            let value = resolve_term_record(root, &records, &mut states, &mut values)
                .expect("the chain resolves");
            assert_eq!(value, purrdf_core::term_fixture::triple_chain(LEVELS));
        })
        .expect("the thread starts");
    }
}
