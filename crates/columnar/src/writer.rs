// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Generic RDF-to-columnar projection.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{
    ContentStore, DatasetView, LossLedger, QuadIds, RdfTextDirection, TermRef, TermValue,
};

use crate::error::ColumnarError;
use crate::files::ParquetFiles;
use crate::parquet::{ColumnValues, Compression, TableData, write_table};
use crate::schema::Table;

type QuadRow = (i64, i64, i64, Option<i64>);
type ReifierRow = (i64, i64, i64, i64, Option<i64>);

/// The result of a complete RDF-to-Parquet conversion.
///
/// Columnar projection is lossless, so a successful conversion currently
/// returns an empty runtime loss ledger. The ledger is still computed and
/// carried on every call so downstream conversion pipelines have one uniform
/// observability contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnarWrite {
    /// All five Parquet files, including valid zero-row files.
    pub files: ParquetFiles,
    /// Losses observed while projecting the dataset.
    pub losses: LossLedger,
}

/// Project any RDF 1.2 [`DatasetView`] and [`ContentStore`] into five Parquet files.
///
/// Term ids are reassigned from the canonical [`TermValue`] order, so output is
/// independent of backend-local ids and ingest order. `compression` is a runtime
/// encoding choice; both modes carry identical RDF semantics.
///
/// # Errors
///
/// Returns [`ColumnarError`] if the source view violates the `DatasetView`
/// structural contract, a table exceeds the Parquet profile's bounds, the blob
/// store fails integrity verification, or encoding fails.
pub fn write<D: DatasetView>(
    view: &D,
    blobs: &ContentStore,
    compression: Compression,
) -> Result<ColumnarWrite, ColumnarError> {
    blobs
        .verify_all()
        .map_err(|error| ColumnarError::malformed("content store", error.to_string()))?;

    let source = SourceRows::collect(view);
    let mut resolver = Resolver::new(view);
    source.resolve_all(&mut resolver)?;
    let dictionary = Dictionary::build(&resolver, &source.named_graphs)?;

    let tables = [
        build_terms_table(&dictionary)?,
        build_quads_table(&source, &resolver, &dictionary)?,
        build_reifiers_table(&source, &resolver, &dictionary)?,
        build_annotations_table(&source, &resolver, &dictionary)?,
        build_blobs_table(blobs)?,
    ];
    let mut encoded = Vec::with_capacity(Table::ALL.len());
    for table in &tables {
        encoded.push(write_table(table, compression)?);
    }
    let files: [Vec<u8>; 5] = encoded.try_into().map_err(|_| {
        ColumnarError::malformed("table set", "writer did not produce exactly five files")
    })?;

    Ok(ColumnarWrite {
        files: ParquetFiles::from_array(files),
        losses: LossLedger::new(),
    })
}

#[derive(Debug)]
struct SourceRows<Id> {
    quads: Vec<QuadIds<Id>>,
    reifiers: Vec<QuadIds<Id>>,
    annotations: Vec<QuadIds<Id>>,
    named_graphs: Vec<Id>,
}

impl<Id: purrdf_core::ViewTermId> SourceRows<Id> {
    fn collect<D>(view: &D) -> Self
    where
        D: DatasetView<Id = Id>,
    {
        let quads: Vec<_> = view.quads().collect();
        let reifiers: Vec<_> = view.reifier_quads().collect();
        let annotations: Vec<_> = view.annotation_quads().collect();
        let mut named_graphs: BTreeSet<_> = view.named_graphs().collect();
        named_graphs.extend(quads.iter().filter_map(|quad| quad.g));
        named_graphs.extend(reifiers.iter().filter_map(|quad| quad.g));
        named_graphs.extend(annotations.iter().filter_map(|quad| quad.g));
        Self {
            quads,
            reifiers,
            annotations,
            named_graphs: named_graphs.into_iter().collect(),
        }
    }

    fn resolve_all<D>(&self, resolver: &mut Resolver<'_, D>) -> Result<(), ColumnarError>
    where
        D: DatasetView<Id = Id>,
    {
        for quad in &self.quads {
            resolve_quad(resolver, quad, true)?;
        }
        for quad in &self.reifiers {
            resolver.resolve(quad.s)?;
            resolver.resolve(quad.o)?;
            resolve_graph(resolver, quad.g)?;
        }
        for quad in &self.annotations {
            resolve_quad(resolver, quad, true)?;
        }
        for &graph in &self.named_graphs {
            resolver.resolve(graph)?;
        }
        Ok(())
    }
}

fn resolve_quad<D: DatasetView>(
    resolver: &mut Resolver<'_, D>,
    quad: &QuadIds<D::Id>,
    include_predicate: bool,
) -> Result<(), ColumnarError> {
    resolver.resolve(quad.s)?;
    if include_predicate {
        resolver.resolve(quad.p)?;
    }
    resolver.resolve(quad.o)?;
    resolve_graph(resolver, quad.g)
}

fn resolve_graph<D: DatasetView>(
    resolver: &mut Resolver<'_, D>,
    graph: Option<D::Id>,
) -> Result<(), ColumnarError> {
    if let Some(graph) = graph {
        resolver.resolve(graph)?;
    }
    Ok(())
}

struct Resolver<'a, D: DatasetView> {
    view: &'a D,
    values: BTreeMap<D::Id, TermValue>,
    active: BTreeSet<D::Id>,
}

impl<'a, D: DatasetView> Resolver<'a, D> {
    fn new(view: &'a D) -> Self {
        Self {
            view,
            values: BTreeMap::new(),
            active: BTreeSet::new(),
        }
    }

    fn resolve(&mut self, id: D::Id) -> Result<TermValue, ColumnarError> {
        if let Some(value) = self.values.get(&id) {
            return Ok(value.clone());
        }
        if !self.active.insert(id) {
            return Err(ColumnarError::malformed(
                "term graph",
                "cyclic triple-term reference",
            ));
        }

        let value = match self.view.resolve(id) {
            TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
            TermRef::Blank { label, scope } => TermValue::Blank {
                label: label.to_owned(),
                scope,
            },
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype = self.resolve(datatype)?;
                let TermValue::Iri(datatype) = datatype else {
                    return Err(ColumnarError::malformed(
                        "literal datatype",
                        "datatype id does not resolve to an IRI",
                    ));
                };
                TermValue::Literal {
                    lexical_form: lexical.to_owned(),
                    datatype,
                    language: language.map(str::to_owned),
                    direction,
                }
            }
            TermRef::Triple { s, p, o } => TermValue::Triple {
                s: Box::new(self.resolve(s)?),
                p: Box::new(self.resolve(p)?),
                o: Box::new(self.resolve(o)?),
            },
        };
        self.active.remove(&id);
        self.values.insert(id, value.clone());
        Ok(value)
    }

    fn get(&self, id: D::Id) -> Result<&TermValue, ColumnarError> {
        self.values
            .get(&id)
            .ok_or_else(|| ColumnarError::malformed("term closure", "source id was not resolved"))
    }
}

struct Dictionary {
    terms: Vec<TermValue>,
    ids: BTreeMap<TermValue, i64>,
    named_graphs: BTreeSet<TermValue>,
}

impl Dictionary {
    fn build<D: DatasetView>(
        resolver: &Resolver<'_, D>,
        named_graph_ids: &[D::Id],
    ) -> Result<Self, ColumnarError> {
        let terms: Vec<_> = resolver
            .values
            .values()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if terms.len() > i32::MAX as usize {
            return Err(ColumnarError::limit(
                "term dictionary",
                terms.len(),
                i32::MAX as usize,
            ));
        }
        let mut ids = BTreeMap::new();
        for (index, term) in terms.iter().enumerate() {
            let id = i64::try_from(index)
                .map_err(|_| ColumnarError::limit("term id", index, i64::MAX as usize))?;
            ids.insert(term.clone(), id);
        }
        let named_graphs = named_graph_ids
            .iter()
            .map(|&id| resolver.get(id).cloned())
            .collect::<Result<_, _>>()?;
        Ok(Self {
            terms,
            ids,
            named_graphs,
        })
    }

    fn id(&self, value: &TermValue) -> Result<i64, ColumnarError> {
        self.ids.get(value).copied().ok_or_else(|| {
            ColumnarError::malformed("term reference", "value is absent from dictionary")
        })
    }

    fn source_id<D: DatasetView>(
        &self,
        resolver: &Resolver<'_, D>,
        id: D::Id,
    ) -> Result<i64, ColumnarError> {
        self.id(resolver.get(id)?)
    }

    fn graph_id<D: DatasetView>(
        &self,
        resolver: &Resolver<'_, D>,
        id: Option<D::Id>,
    ) -> Result<Option<i64>, ColumnarError> {
        id.map(|id| self.source_id(resolver, id)).transpose()
    }
}

fn build_terms_table(dictionary: &Dictionary) -> Result<TableData, ColumnarError> {
    let capacity = dictionary.terms.len();
    let mut ids = Vec::with_capacity(capacity);
    let mut kinds = Vec::with_capacity(capacity);
    let mut lex = Vec::with_capacity(capacity);
    let mut datatypes = Vec::with_capacity(capacity);
    let mut languages = Vec::with_capacity(capacity);
    let mut directions = Vec::with_capacity(capacity);
    let mut scopes = Vec::with_capacity(capacity);
    let mut triple_subjects = Vec::with_capacity(capacity);
    let mut triple_predicates = Vec::with_capacity(capacity);
    let mut triple_objects = Vec::with_capacity(capacity);
    let mut named_graphs = Vec::with_capacity(capacity);

    for (index, term) in dictionary.terms.iter().enumerate() {
        ids.push(Some(index as i64));
        append_term_columns(
            dictionary,
            term,
            &mut kinds,
            &mut lex,
            &mut datatypes,
            &mut languages,
            &mut directions,
            &mut scopes,
            &mut triple_subjects,
            &mut triple_predicates,
            &mut triple_objects,
        )?;
        named_graphs.push(Some(i64::from(dictionary.named_graphs.contains(term))));
    }

    TableData::new(
        Table::Terms,
        vec![
            ColumnValues::int64(ids),
            ColumnValues::int64(kinds),
            ColumnValues::bytes(lex),
            ColumnValues::int64(datatypes),
            ColumnValues::bytes(languages),
            ColumnValues::int64(directions),
            ColumnValues::int64(scopes),
            ColumnValues::int64(triple_subjects),
            ColumnValues::int64(triple_predicates),
            ColumnValues::int64(triple_objects),
            ColumnValues::int64(named_graphs),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
fn append_term_columns(
    dictionary: &Dictionary,
    term: &TermValue,
    kinds: &mut Vec<Option<i64>>,
    lex: &mut Vec<Option<Vec<u8>>>,
    datatypes: &mut Vec<Option<i64>>,
    languages: &mut Vec<Option<Vec<u8>>>,
    directions: &mut Vec<Option<i64>>,
    scopes: &mut Vec<Option<i64>>,
    triple_subjects: &mut Vec<Option<i64>>,
    triple_predicates: &mut Vec<Option<i64>>,
    triple_objects: &mut Vec<Option<i64>>,
) -> Result<(), ColumnarError> {
    let mut row = TermColumns::default();
    match term {
        TermValue::Iri(iri) => {
            row.kind = 0;
            row.lex = Some(iri.as_bytes().to_vec());
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            row.kind = 1;
            row.lex = Some(lexical_form.as_bytes().to_vec());
            row.datatype = Some(dictionary.id(&TermValue::Iri(datatype.clone()))?);
            row.language = language.as_ref().map(|value| value.as_bytes().to_vec());
            row.direction = direction.map(direction_id);
        }
        TermValue::Blank { label, scope } => {
            row.kind = 2;
            row.lex = Some(label.as_bytes().to_vec());
            row.scope = Some(i64::from(scope.ordinal()));
        }
        TermValue::Triple { s, p, o } => {
            row.kind = 3;
            row.triple_s = Some(dictionary.id(s)?);
            row.triple_p = Some(dictionary.id(p)?);
            row.triple_o = Some(dictionary.id(o)?);
        }
    }
    kinds.push(Some(row.kind));
    lex.push(row.lex);
    datatypes.push(row.datatype);
    languages.push(row.language);
    directions.push(row.direction);
    scopes.push(row.scope);
    triple_subjects.push(row.triple_s);
    triple_predicates.push(row.triple_p);
    triple_objects.push(row.triple_o);
    Ok(())
}

#[derive(Default)]
struct TermColumns {
    kind: i64,
    lex: Option<Vec<u8>>,
    datatype: Option<i64>,
    language: Option<Vec<u8>>,
    direction: Option<i64>,
    scope: Option<i64>,
    triple_s: Option<i64>,
    triple_p: Option<i64>,
    triple_o: Option<i64>,
}

const fn direction_id(direction: RdfTextDirection) -> i64 {
    match direction {
        RdfTextDirection::Ltr => 0,
        RdfTextDirection::Rtl => 1,
    }
}

fn build_quads_table<D: DatasetView>(
    source: &SourceRows<D::Id>,
    resolver: &Resolver<'_, D>,
    dictionary: &Dictionary,
) -> Result<TableData, ColumnarError> {
    let rows = source
        .quads
        .iter()
        .map(|quad| {
            Ok((
                dictionary.source_id(resolver, quad.s)?,
                dictionary.source_id(resolver, quad.p)?,
                dictionary.source_id(resolver, quad.o)?,
                dictionary.graph_id(resolver, quad.g)?,
            ))
        })
        .collect::<Result<BTreeSet<_>, ColumnarError>>()?;
    table_from_quad_rows(Table::Quads, rows)
}

fn build_reifiers_table<D: DatasetView>(
    source: &SourceRows<D::Id>,
    resolver: &Resolver<'_, D>,
    dictionary: &Dictionary,
) -> Result<TableData, ColumnarError> {
    let mut rows = BTreeSet::new();
    for quad in &source.reifiers {
        let TermValue::Triple { s, p, o } = resolver.get(quad.o)? else {
            return Err(ColumnarError::malformed(
                "reifier binding",
                "object is not a triple term",
            ));
        };
        rows.insert((
            dictionary.source_id(resolver, quad.s)?,
            dictionary.id(s)?,
            dictionary.id(p)?,
            dictionary.id(o)?,
            dictionary.graph_id(resolver, quad.g)?,
        ));
    }
    table_from_reifier_rows(rows)
}

fn build_annotations_table<D: DatasetView>(
    source: &SourceRows<D::Id>,
    resolver: &Resolver<'_, D>,
    dictionary: &Dictionary,
) -> Result<TableData, ColumnarError> {
    let rows = source
        .annotations
        .iter()
        .map(|quad| {
            Ok((
                dictionary.source_id(resolver, quad.s)?,
                dictionary.source_id(resolver, quad.p)?,
                dictionary.source_id(resolver, quad.o)?,
                dictionary.graph_id(resolver, quad.g)?,
            ))
        })
        .collect::<Result<BTreeSet<_>, ColumnarError>>()?;
    table_from_quad_rows(Table::Annotations, rows)
}

fn table_from_quad_rows(table: Table, rows: BTreeSet<QuadRow>) -> Result<TableData, ColumnarError> {
    let mut first = Vec::with_capacity(rows.len());
    let mut second = Vec::with_capacity(rows.len());
    let mut third = Vec::with_capacity(rows.len());
    let mut graphs = Vec::with_capacity(rows.len());
    for (a, b, c, graph) in rows {
        first.push(Some(a));
        second.push(Some(b));
        third.push(Some(c));
        graphs.push(graph);
    }
    TableData::new(
        table,
        vec![
            ColumnValues::int64(first),
            ColumnValues::int64(second),
            ColumnValues::int64(third),
            ColumnValues::int64(graphs),
        ],
    )
}

fn table_from_reifier_rows(rows: BTreeSet<ReifierRow>) -> Result<TableData, ColumnarError> {
    let mut reifiers = Vec::with_capacity(rows.len());
    let mut subjects = Vec::with_capacity(rows.len());
    let mut predicates = Vec::with_capacity(rows.len());
    let mut objects = Vec::with_capacity(rows.len());
    let mut graphs = Vec::with_capacity(rows.len());
    for (reifier, subject, predicate, object, graph) in rows {
        reifiers.push(Some(reifier));
        subjects.push(Some(subject));
        predicates.push(Some(predicate));
        objects.push(Some(object));
        graphs.push(graph);
    }
    TableData::new(
        Table::Reifiers,
        vec![
            ColumnValues::int64(reifiers),
            ColumnValues::int64(subjects),
            ColumnValues::int64(predicates),
            ColumnValues::int64(objects),
            ColumnValues::int64(graphs),
        ],
    )
}

fn build_blobs_table(blobs: &ContentStore) -> Result<TableData, ColumnarError> {
    let mut rows: Vec<_> = blobs.iter().collect();
    rows.sort_unstable_by_key(|(digest, _)| **digest);
    let mut digests = Vec::with_capacity(rows.len());
    let mut bytes = Vec::with_capacity(rows.len());
    for (digest, payload) in rows {
        digests.push(Some(digest.to_hex().into_bytes()));
        bytes.push(Some(payload.clone()));
    }
    TableData::new(
        Table::Blobs,
        vec![ColumnValues::bytes(digests), ColumnValues::bytes(bytes)],
    )
}

#[cfg(test)]
mod tests {
    use purrdf_core::{
        BlankScope, ContentStore, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
    };

    use super::*;
    use crate::parquet::read_table;

    fn fixture() -> (std::sync::Arc<RdfDataset>, ContentStore) {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("subject", BlankScope(7));
        let predicate = builder.intern_iri("https://example.org/p");
        let object = builder.intern_literal(RdfLiteral::typed("42", "https://example.org/integer"));
        let directional = builder.intern_literal(RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        let graph = builder.intern_iri("https://example.org/graph");
        let empty_graph = builder.intern_iri("https://example.org/empty");
        builder.declare_named_graph(empty_graph);
        builder.push_quad(subject, predicate, object, Some(graph));

        let triple = builder.intern_triple(subject, predicate, directional);
        let reifier = builder.intern_blank("reifier", BlankScope(7));
        builder.push_reifier_in_graph(reifier, triple, Some(graph));
        builder.push_annotation_in_graph(reifier, predicate, object, None);

        let mut blobs = ContentStore::new();
        blobs.insert(b"columnar payload".to_vec());
        (builder.freeze().unwrap(), blobs)
    }

    #[test]
    fn writer_projects_all_rdf_layers_and_is_deterministic() {
        let (dataset, blobs) = fixture();
        let first = write(&*dataset, &blobs, Compression::Zstd).unwrap();
        let second = write(&*dataset, &blobs, Compression::Zstd).unwrap();
        assert_eq!(first, second);
        assert!(first.losses.is_empty());
        assert_eq!(first.files.iter().len(), Table::ALL.len());

        let terms = read_table(first.files.get(Table::Terms), Table::Terms).unwrap();
        let quads = read_table(first.files.get(Table::Quads), Table::Quads).unwrap();
        let reifiers = read_table(first.files.get(Table::Reifiers), Table::Reifiers).unwrap();
        let annotations =
            read_table(first.files.get(Table::Annotations), Table::Annotations).unwrap();
        let blob_rows = read_table(first.files.get(Table::Blobs), Table::Blobs).unwrap();
        assert!(terms.row_count >= 10);
        assert_eq!(quads.row_count, 1);
        assert_eq!(reifiers.row_count, 1);
        assert_eq!(annotations.row_count, 1);
        assert_eq!(blob_rows.row_count, 1);

        let ColumnValues::Int64(named_graphs) = &terms.columns[10] else {
            panic!("named_graph must be INT64");
        };
        assert_eq!(
            named_graphs
                .iter()
                .filter(|value| **value == Some(1))
                .count(),
            2
        );
    }

    #[test]
    fn empty_input_still_writes_five_valid_zero_row_files() {
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let written = write(&*dataset, &ContentStore::new(), Compression::Uncompressed).unwrap();
        for (table, bytes) in written.files.iter() {
            assert_eq!(read_table(bytes, table).unwrap().row_count, 0);
        }
        assert!(written.losses.is_empty());
    }
}
