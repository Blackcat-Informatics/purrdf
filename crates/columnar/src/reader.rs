// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict five-table columnar-to-RDF reconstruction.

use purrdf_core::TermBox;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::str;
use std::sync::Arc;

use purrdf_core::langtag::is_identity_folded;
use purrdf_core::{
    BlankScope, ContentDigest, ContentStore, LossLedger, Nested, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, RdfTextDirection, TermId, TermValue, try_fold_nested,
};

use crate::column::Int64Column;
use crate::error::ColumnarError;
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
        decode_blobs(&blobs)?
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
                    return Err(ColumnarError::malformed(
                        "terms.named_graph",
                        format!("row {row} has flag {flag}, expected 0 or 1"),
                    ));
                }
            };
            if named && !matches!(value, TermValue::Iri(_) | TermValue::Blank { .. }) {
                return Err(ColumnarError::malformed(
                    "terms.named_graph",
                    format!("row {row} marks a literal or triple term as a graph name"),
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

/// One term row's nullable INT64 cells, read from each column's rows in step.
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
            return Err(ColumnarError::malformed(
                "terms.id",
                format!("row {row} has id {id}; ids must be dense and zero-based"),
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
    let scope = u32::try_from(scope).map_err(|_| {
        ColumnarError::malformed(
            "terms.scope",
            format!("row {row} has out-of-range scope {scope}"),
        )
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
        .ok_or_else(|| ColumnarError::malformed("term dictionary", "unresolved term record"))
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
            return Err(ColumnarError::malformed(
                "term dictionary",
                "cyclic triple-term reference",
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
                    return Err(ColumnarError::malformed(
                        "terms.datatype",
                        format!("literal row {index} references a non-IRI datatype"),
                    ));
                };
                if let Some(language) = &language {
                    if language.is_empty() || !is_identity_folded(language) {
                        return Err(ColumnarError::malformed(
                            "terms.lang",
                            format!("literal row {index} has a non-canonical language tag"),
                        ));
                    }
                    let expected = RdfLiteral::language_datatype_iri(direction);
                    if datatype != expected {
                        return Err(ColumnarError::malformed(
                            "terms.datatype",
                            format!("language literal row {index} must use {expected}"),
                        ));
                    }
                } else if direction.is_some() {
                    return Err(ColumnarError::malformed(
                        "terms.direction",
                        format!("literal row {index} has a direction without a language tag"),
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

fn decode_blobs(data: &TableData) -> Result<ContentStore, ColumnarError> {
    let digests = bytes_column(data, 0)?;
    let payloads = bytes_column(data, 1)?;
    let mut previous = None;
    let mut store = ContentStore::new();
    for row in 0..data.row_count {
        let digest_bytes = required_bytes(digests, row, "blobs.digest")?;
        let digest_text = str::from_utf8(digest_bytes).map_err(|_| {
            ColumnarError::malformed("blobs.digest", format!("row {row} is not UTF-8"))
        })?;
        let digest = ContentDigest::from_hex(digest_text).ok_or_else(|| {
            ColumnarError::malformed(
                "blobs.digest",
                format!("row {row} is not a 64-digit SHA-256 hex value"),
            )
        })?;
        if digest_text != digest.to_hex() {
            return Err(ColumnarError::malformed(
                "blobs.digest",
                format!("row {row} is not canonical lowercase hexadecimal"),
            ));
        }
        if previous.is_some_and(|value| value >= digest) {
            return Err(ColumnarError::malformed(
                "blob row order",
                "digests are not strictly increasing",
            ));
        }
        let payload = required_bytes(payloads, row, "blobs.bytes")?.to_vec();
        store.insert_checked(digest, payload).map_err(|error| {
            ColumnarError::malformed("blobs.bytes", format!("row {row}: {error}"))
        })?;
        previous = Some(digest);
    }
    Ok(store)
}

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
        .map(|value| intern_value(&mut builder, value))
        .collect();
    for (index, &named) in dictionary.named_graphs.iter().enumerate() {
        if named {
            builder.declare_named_graph(ids[index]);
        }
    }
    for &(s, p, o, g) in quads {
        builder.push_quad(ids[s], ids[p], ids[o], g.map(|id| ids[id]));
    }
    for &(reifier, s, p, o, g) in reifiers {
        let triple_value = TermValue::Triple {
            s: TermBox::new(dictionary.values[s].clone()),
            p: TermBox::new(dictionary.values[p].clone()),
            o: TermBox::new(dictionary.values[o].clone()),
        };
        let triple = dictionary.value_ids.get(&triple_value).ok_or_else(|| {
            ColumnarError::malformed(
                "reifier binding",
                "component tuple has no corresponding triple term",
            )
        })?;
        builder.push_reifier_in_graph(ids[reifier], ids[*triple], g.map(|id| ids[id]));
    }
    for &(reifier, predicate, value, graph) in annotations {
        builder.push_annotation_in_graph(
            ids[reifier],
            ids[predicate],
            ids[value],
            graph.map(|id| ids[id]),
        );
    }
    builder
        .freeze()
        .map_err(|error| ColumnarError::malformed("RDF reconstruction", error.to_string()))
}

/// Intern `value` into `builder`, a triple term over [`try_fold_nested`]'s work list:
/// its subject, predicate and object, each fully before the next, then the triple.
fn intern_value(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
    let interned = try_fold_nested(
        value,
        builder,
        |builder, value| {
            Ok::<_, Infallible>(Nested::Leaf(match value {
                TermValue::Iri(iri) => builder.intern_iri(iri),
                TermValue::Blank { label, scope } => builder.intern_blank(label, *scope),
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => builder.intern_literal(RdfLiteral {
                    lexical_form: lexical_form.clone(),
                    datatype: Some(datatype.clone()),
                    language: language.clone(),
                    direction: *direction,
                }),
                TermValue::Triple { s, p, o } => return Ok(Nested::Triple(&**s, &**p, &**o)),
            }))
        },
        |builder, _, s, p, o| Ok(builder.intern_triple(s, p, o)),
    );
    match interned {
        Ok(id) => id,
    }
}

fn int_column(data: &TableData, index: usize) -> Result<&Int64Column, ColumnarError> {
    match data.columns.get(index) {
        Some(ColumnValues::Int64(values)) => Ok(values),
        _ => Err(ColumnarError::malformed(
            "table column",
            format!("{}.{} is not INT64", data.table.name(), index),
        )),
    }
}

fn bytes_column(data: &TableData, index: usize) -> Result<&[Option<Vec<u8>>], ColumnarError> {
    match data.columns.get(index) {
        Some(ColumnValues::ByteArray(values)) => Ok(values),
        _ => Err(ColumnarError::malformed(
            "table column",
            format!("{}.{} is not BYTE_ARRAY", data.table.name(), index),
        )),
    }
}

fn required_i64(
    value: Option<i64>,
    row: usize,
    context: &'static str,
) -> Result<i64, ColumnarError> {
    value.ok_or_else(|| {
        ColumnarError::malformed(context, format!("required value is null at row {row}"))
    })
}

fn required_bytes<'a>(
    column: &'a [Option<Vec<u8>>],
    row: usize,
    context: &'static str,
) -> Result<&'a [u8], ColumnarError> {
    column[row].as_deref().ok_or_else(|| {
        ColumnarError::malformed(context, format!("required value is null at row {row}"))
    })
}

fn required_utf8(
    column: &[Option<Vec<u8>>],
    row: usize,
    context: &'static str,
) -> Result<String, ColumnarError> {
    let value = required_bytes(column, row, context)?;
    str::from_utf8(value)
        .map(str::to_owned)
        .map_err(|_| ColumnarError::malformed(context, format!("row {row} is not valid UTF-8")))
}

fn optional_utf8(
    column: &[Option<Vec<u8>>],
    row: usize,
    context: &'static str,
) -> Result<Option<String>, ColumnarError> {
    column[row]
        .as_deref()
        .map(|value| {
            str::from_utf8(value).map(str::to_owned).map_err(|_| {
                ColumnarError::malformed(context, format!("row {row} is not valid UTF-8"))
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
    let id = usize::try_from(value).map_err(|_| {
        ColumnarError::malformed(context, format!("row {row} has negative term id {value}"))
    })?;
    if id >= term_count {
        return Err(ColumnarError::malformed(
            context,
            format!("row {row} references term {id}, but dictionary has {term_count} rows"),
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
    let graph = usize::try_from(value).map_err(|_| {
        ColumnarError::malformed(
            "graph reference",
            format!("row {row} has negative id {value}"),
        )
    })?;
    if graph >= named_graphs.len() {
        return Err(ColumnarError::malformed(
            "graph reference",
            format!("row {row} references missing term {graph}"),
        ));
    }
    if !named_graphs[graph] {
        return Err(ColumnarError::malformed(
            "graph reference",
            format!("row {row} references term {graph} not marked as a named graph"),
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
        return Err(ColumnarError::malformed(
            context,
            format!("value must be null for term kind at row {row}"),
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
        return Err(ColumnarError::malformed(
            context,
            format!("value must be null for term kind at row {row}"),
        ));
    }
    Ok(())
}

fn ensure_strict_order<T: Ord>(values: &[T], context: &'static str) -> Result<(), ColumnarError> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ColumnarError::malformed(
            context,
            "rows are not strictly increasing",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_core::{
        BlankScope, DatasetView, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
        datasets_isomorphic,
    };

    use super::*;
    use crate::parquet::{Compression, write_table};
    use crate::writer::write;

    fn fixture() -> (Arc<RdfDataset>, ContentStore) {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("subject", BlankScope(3));
        let predicate = builder.intern_iri("https://example.org/p");
        let object = builder.intern_literal(RdfLiteral {
            lexical_form: "bonjour".to_owned(),
            datatype: None,
            language: Some("fr".to_owned()),
            direction: Some(RdfTextDirection::Ltr),
        });
        let graph = builder.intern_iri("https://example.org/graph");
        let empty_graph = builder.intern_iri("https://example.org/empty");
        builder.declare_named_graph(empty_graph);
        builder.push_quad(subject, predicate, object, Some(graph));
        let triple = builder.intern_triple(subject, predicate, object);
        let reifier = builder.intern_blank("reifier", BlankScope(3));
        builder.push_reifier_in_graph(reifier, triple, Some(graph));
        builder.push_annotation_in_graph(reifier, predicate, object, None);

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
        assert!(datasets_isomorphic(&dataset, &decoded.dataset));
        assert_eq!(decoded.blobs, blobs);
        assert!(encoded.losses.is_empty());
        assert!(decoded.losses.is_empty());

        let graph_names: Vec<_> = decoded
            .dataset
            .named_graphs()
            .map(|id| match decoded.dataset.resolve(id) {
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
        let (dataset, blobs) = fixture();
        let encoded = write(&*dataset, &blobs, Compression::Uncompressed).unwrap();
        let mut array = encoded.files.into_array();
        let mut terms = read_table(&array[0], Table::Terms).unwrap();
        let ColumnValues::Int64(ids) = &mut terms.columns[0] else {
            panic!("terms.id is INT64");
        };
        ids.values_mut()[0] = 9;
        array[0] = write_table(&terms, Compression::Uncompressed).unwrap();
        assert!(matches!(
            read(&ParquetFiles::from_array(array)),
            Err(ColumnarError::Malformed { .. })
        ));
    }

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
            Err(ColumnarError::Malformed { .. })
        ));
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The term-record resolution and the re-interning against their recursive
    //! references, and at a hundred thousand levels on a 128 KiB thread.

    use purrdf_core::langtag::is_identity_folded;
    use purrdf_core::{
        BlankScope, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermBox, TermId, TermValue,
    };

    use super::{
        ColumnarError, TermRecord, intern_value, resolve_term_record, resolve_term_records,
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
            return Err(ColumnarError::malformed(
                "term dictionary",
                "cyclic triple-term reference",
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
                    return Err(ColumnarError::malformed(
                        "terms.datatype",
                        format!("literal row {index} references a non-IRI datatype"),
                    ));
                };
                if let Some(language) = &language {
                    if language.is_empty() || !is_identity_folded(language) {
                        return Err(ColumnarError::malformed(
                            "terms.lang",
                            format!("literal row {index} has a non-canonical language tag"),
                        ));
                    }
                    let expected = RdfLiteral::language_datatype_iri(direction);
                    if datatype != expected {
                        return Err(ColumnarError::malformed(
                            "terms.datatype",
                            format!("language literal row {index} must use {expected}"),
                        ));
                    }
                } else if direction.is_some() {
                    return Err(ColumnarError::malformed(
                        "terms.direction",
                        format!("literal row {index} has a direction without a language tag"),
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

    fn reference_intern(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
        match value {
            TermValue::Triple { s, p, o } => {
                let s = reference_intern(builder, s);
                let p = reference_intern(builder, p);
                let o = reference_intern(builder, o);
                builder.intern_triple(s, p, o)
            }
            leaf => intern_value(builder, leaf),
        }
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
                        "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
                        "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString",
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
        let refused = resolve_term_records(&language_literal_table("en-GB"))
            .expect_err("an unfolded tag is not the canonical identity");
        assert!(
            refused.to_string().contains("non-canonical language tag"),
            "{refused}"
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

    /// Every generated term re-interns into a fresh builder as the recursive reference
    /// interns it: the same id, and the same next id after it.
    #[test]
    fn interning_agrees_with_its_recursive_reference_on_generated_terms() {
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::Any,
            );
            let (mut found, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            assert_eq!(
                intern_value(&mut found, &value),
                reference_intern(&mut expected, &value),
                "seed {seed}"
            );
            let sentinel = "http://example.org/sentinel";
            assert_eq!(
                found.intern_iri(sentinel),
                expected.intern_iri(sentinel),
                "seed {seed}"
            );
        }
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

    /// A triple term a hundred thousand levels deep re-interns on a thread whose whole
    /// stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_interns_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = purrdf_core::term_fixture::triple_chain(LEVELS);
            let mut builder = RdfDatasetBuilder::new();
            assert_eq!(intern_value(&mut builder, &value).index(), LEVELS + 2);
        })
        .expect("the thread starts");
    }
}
