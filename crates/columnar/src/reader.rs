// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Strict five-table columnar-to-RDF reconstruction.

use std::collections::BTreeMap;
use std::str;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, ContentDigest, ContentStore, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfTextDirection, TermId, TermValue,
};

use crate::error::ColumnarError;
use crate::files::ParquetFiles;
use crate::parquet::{ColumnValues, TableData, read_table};
use crate::schema::Table;

type QuadRow = (usize, usize, usize, Option<usize>);
type ReifierRow = (usize, usize, usize, usize, Option<usize>);

const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

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
/// Returns [`ColumnarError`] for malformed Parquet, schema/profile drift, invalid
/// UTF-8 or RDF term records, non-canonical ordering, invalid references, blob
/// digest mismatches, or RDF positional violations.
pub fn read(files: &ParquetFiles) -> Result<ColumnarRead, ColumnarError> {
    let terms = read_table(files.get(Table::Terms), Table::Terms)?;
    let quads = read_table(files.get(Table::Quads), Table::Quads)?;
    let reifiers = read_table(files.get(Table::Reifiers), Table::Reifiers)?;
    let annotations = read_table(files.get(Table::Annotations), Table::Annotations)?;
    let blobs = read_table(files.get(Table::Blobs), Table::Blobs)?;

    let dictionary = Dictionary::decode(&terms)?;
    let quad_rows = decode_quad_rows(&quads, &dictionary.named_graphs)?;
    let reifier_rows = decode_reifier_rows(&reifiers, &dictionary.named_graphs)?;
    let annotation_rows = decode_quad_rows(&annotations, &dictionary.named_graphs)?;
    let content_store = decode_blobs(&blobs)?;
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

        let named_column = int_column(data, 10)?;
        let mut named_graphs = Vec::with_capacity(values.len());
        for (row, value) in values.iter().enumerate() {
            let flag = required_i64(named_column, row, "terms.named_graph")?;
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

fn parse_term_records(data: &TableData) -> Result<Vec<TermRecord>, ColumnarError> {
    let ids = int_column(data, 0)?;
    let kinds = int_column(data, 1)?;
    let lex = bytes_column(data, 2)?;
    let datatypes = int_column(data, 3)?;
    let languages = bytes_column(data, 4)?;
    let directions = int_column(data, 5)?;
    let scopes = int_column(data, 6)?;
    let triple_subjects = int_column(data, 7)?;
    let triple_predicates = int_column(data, 8)?;
    let triple_objects = int_column(data, 9)?;
    let mut records = Vec::with_capacity(data.row_count);

    for row in 0..data.row_count {
        let id = required_i64(ids, row, "terms.id")?;
        if id != row as i64 {
            return Err(ColumnarError::malformed(
                "terms.id",
                format!("row {row} has id {id}; ids must be dense and zero-based"),
            ));
        }
        let kind = required_i64(kinds, row, "terms.kind")?;
        records.push(match kind {
            0 => parse_iri_record(
                row,
                lex,
                datatypes,
                languages,
                directions,
                scopes,
                triple_subjects,
                triple_predicates,
                triple_objects,
            )?,
            1 => parse_literal_record(
                row,
                data.row_count,
                lex,
                datatypes,
                languages,
                directions,
                scopes,
                triple_subjects,
                triple_predicates,
                triple_objects,
            )?,
            2 => parse_blank_record(
                row,
                lex,
                datatypes,
                languages,
                directions,
                scopes,
                triple_subjects,
                triple_predicates,
                triple_objects,
            )?,
            3 => parse_triple_record(
                row,
                data.row_count,
                lex,
                datatypes,
                languages,
                directions,
                scopes,
                triple_subjects,
                triple_predicates,
                triple_objects,
            )?,
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

#[allow(clippy::too_many_arguments)]
fn parse_iri_record(
    row: usize,
    lex: &[Option<Vec<u8>>],
    datatypes: &[Option<i64>],
    languages: &[Option<Vec<u8>>],
    directions: &[Option<i64>],
    scopes: &[Option<i64>],
    triple_subjects: &[Option<i64>],
    triple_predicates: &[Option<i64>],
    triple_objects: &[Option<i64>],
) -> Result<TermRecord, ColumnarError> {
    ensure_null_i64(row, datatypes, "terms.datatype")?;
    ensure_null_bytes(row, languages, "terms.lang")?;
    ensure_null_i64(row, directions, "terms.direction")?;
    ensure_null_i64(row, scopes, "terms.scope")?;
    ensure_null_i64(row, triple_subjects, "terms.triple_s")?;
    ensure_null_i64(row, triple_predicates, "terms.triple_p")?;
    ensure_null_i64(row, triple_objects, "terms.triple_o")?;
    Ok(TermRecord::Iri(required_utf8(lex, row, "terms.lex")?))
}

#[allow(clippy::too_many_arguments)]
fn parse_literal_record(
    row: usize,
    term_count: usize,
    lex: &[Option<Vec<u8>>],
    datatypes: &[Option<i64>],
    languages: &[Option<Vec<u8>>],
    directions: &[Option<i64>],
    scopes: &[Option<i64>],
    triple_subjects: &[Option<i64>],
    triple_predicates: &[Option<i64>],
    triple_objects: &[Option<i64>],
) -> Result<TermRecord, ColumnarError> {
    ensure_null_i64(row, scopes, "terms.scope")?;
    ensure_null_i64(row, triple_subjects, "terms.triple_s")?;
    ensure_null_i64(row, triple_predicates, "terms.triple_p")?;
    ensure_null_i64(row, triple_objects, "terms.triple_o")?;
    let direction = match directions[row] {
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
        datatype: required_term_ref(datatypes, row, term_count, "terms.datatype")?,
        language: optional_utf8(languages, row, "terms.lang")?,
        direction,
    })
}

#[allow(clippy::too_many_arguments)]
fn parse_blank_record(
    row: usize,
    lex: &[Option<Vec<u8>>],
    datatypes: &[Option<i64>],
    languages: &[Option<Vec<u8>>],
    directions: &[Option<i64>],
    scopes: &[Option<i64>],
    triple_subjects: &[Option<i64>],
    triple_predicates: &[Option<i64>],
    triple_objects: &[Option<i64>],
) -> Result<TermRecord, ColumnarError> {
    ensure_null_i64(row, datatypes, "terms.datatype")?;
    ensure_null_bytes(row, languages, "terms.lang")?;
    ensure_null_i64(row, directions, "terms.direction")?;
    ensure_null_i64(row, triple_subjects, "terms.triple_s")?;
    ensure_null_i64(row, triple_predicates, "terms.triple_p")?;
    ensure_null_i64(row, triple_objects, "terms.triple_o")?;
    let scope = required_i64(scopes, row, "terms.scope")?;
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

#[allow(clippy::too_many_arguments)]
fn parse_triple_record(
    row: usize,
    term_count: usize,
    lex: &[Option<Vec<u8>>],
    datatypes: &[Option<i64>],
    languages: &[Option<Vec<u8>>],
    directions: &[Option<i64>],
    scopes: &[Option<i64>],
    triple_subjects: &[Option<i64>],
    triple_predicates: &[Option<i64>],
    triple_objects: &[Option<i64>],
) -> Result<TermRecord, ColumnarError> {
    ensure_null_bytes(row, lex, "terms.lex")?;
    ensure_null_i64(row, datatypes, "terms.datatype")?;
    ensure_null_bytes(row, languages, "terms.lang")?;
    ensure_null_i64(row, directions, "terms.direction")?;
    ensure_null_i64(row, scopes, "terms.scope")?;
    Ok(TermRecord::Triple {
        s: required_term_ref(triple_subjects, row, term_count, "terms.triple_s")?,
        p: required_term_ref(triple_predicates, row, term_count, "terms.triple_p")?,
        o: required_term_ref(triple_objects, row, term_count, "terms.triple_o")?,
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

fn resolve_term_record(
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
            let datatype = resolve_term_record(datatype, records, states, values)?;
            let TermValue::Iri(datatype) = datatype else {
                return Err(ColumnarError::malformed(
                    "terms.datatype",
                    format!("literal row {index} references a non-IRI datatype"),
                ));
            };
            if let Some(language) = &language {
                if language.is_empty() || language != &language.to_lowercase() {
                    return Err(ColumnarError::malformed(
                        "terms.lang",
                        format!("literal row {index} has a non-canonical language tag"),
                    ));
                }
                if datatype != RDF_LANG_STRING {
                    return Err(ColumnarError::malformed(
                        "terms.datatype",
                        format!("language literal row {index} does not use rdf:langString"),
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
            s: Box::new(resolve_term_record(s, records, states, values)?),
            p: Box::new(resolve_term_record(p, records, states, values)?),
            o: Box::new(resolve_term_record(o, records, states, values)?),
        },
    };
    states[index] = 2;
    values[index] = Some(value.clone());
    Ok(value)
}

fn decode_quad_rows(
    data: &TableData,
    named_graphs: &[bool],
) -> Result<Vec<QuadRow>, ColumnarError> {
    let first = int_column(data, 0)?;
    let second = int_column(data, 1)?;
    let third = int_column(data, 2)?;
    let graphs = int_column(data, 3)?;
    let mut rows = Vec::with_capacity(data.row_count);
    for row in 0..data.row_count {
        rows.push((
            required_term_ref(first, row, named_graphs.len(), "row first term")?,
            required_term_ref(second, row, named_graphs.len(), "row second term")?,
            required_term_ref(third, row, named_graphs.len(), "row third term")?,
            optional_graph_ref(graphs, row, named_graphs)?,
        ));
    }
    ensure_strict_order(&rows, "quad-like row order")?;
    Ok(rows)
}

fn decode_reifier_rows(
    data: &TableData,
    named_graphs: &[bool],
) -> Result<Vec<ReifierRow>, ColumnarError> {
    let reifiers = int_column(data, 0)?;
    let subjects = int_column(data, 1)?;
    let predicates = int_column(data, 2)?;
    let objects = int_column(data, 3)?;
    let graphs = int_column(data, 4)?;
    let mut rows = Vec::with_capacity(data.row_count);
    for row in 0..data.row_count {
        rows.push((
            required_term_ref(reifiers, row, named_graphs.len(), "reifiers.reifier")?,
            required_term_ref(subjects, row, named_graphs.len(), "reifiers.s")?,
            required_term_ref(predicates, row, named_graphs.len(), "reifiers.p")?,
            required_term_ref(objects, row, named_graphs.len(), "reifiers.o")?,
            optional_graph_ref(graphs, row, named_graphs)?,
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
            s: Box::new(dictionary.values[s].clone()),
            p: Box::new(dictionary.values[p].clone()),
            o: Box::new(dictionary.values[o].clone()),
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

fn intern_value(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
    match value {
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
        TermValue::Triple { s, p, o } => {
            let s = intern_value(builder, s);
            let p = intern_value(builder, p);
            let o = intern_value(builder, o);
            builder.intern_triple(s, p, o)
        }
    }
}

fn int_column(data: &TableData, index: usize) -> Result<&[Option<i64>], ColumnarError> {
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
    column: &[Option<i64>],
    row: usize,
    context: &'static str,
) -> Result<i64, ColumnarError> {
    column[row].ok_or_else(|| {
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
    column: &[Option<i64>],
    row: usize,
    term_count: usize,
    context: &'static str,
) -> Result<usize, ColumnarError> {
    let value = required_i64(column, row, context)?;
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
    column: &[Option<i64>],
    row: usize,
    named_graphs: &[bool],
) -> Result<Option<usize>, ColumnarError> {
    let Some(value) = column[row] else {
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
    column: &[Option<i64>],
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
        ids[0] = Some(9);
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
