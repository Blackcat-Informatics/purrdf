// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generic RDF-to-columnar projection.

use purrdf_core::{TermBox, TermGuard as _};
use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::{
    ContentStore, DatasetView, LossLedger, QuadIds, RdfTextDirection, TermRef, TermValue,
};

use crate::ProjectionCell;
use crate::column::Int64Column;
use crate::error::{ColumnarError, ColumnarWriteError, ProjectionReason};
use crate::files::ParquetFiles;
use crate::parquet::{ColumnValues, Compression, TableData, write_rows};
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
/// Returns the original source read failure, or a native [`ColumnarError`] if
/// the view violates the `DatasetView` structural contract, a table exceeds the
/// Parquet profile's bounds, blob verification fails, or encoding fails.
pub fn write<D: DatasetView>(
    view: &D,
    blobs: &ContentStore,
    compression: Compression,
) -> Result<ColumnarWrite, ColumnarWriteError<D::ReadError>> {
    view.checked_read(|view| {
        let prepared = prepare(view, blobs)?;
        let mut files = std::array::from_fn(|_| Vec::new());
        for (file, table) in files.iter_mut().zip(Table::ALL) {
            *file = write_rows(table, prepared.rows(table), compression)?;
        }
        Ok(ColumnarWrite {
            files: ParquetFiles::from_array(files),
            losses: LossLedger::default(),
        })
    })
    .map_err(ColumnarWriteError::SourceRead)?
}

/// Project an RDF view into the authoritative five-table schema.
///
/// This is the same canonical projection the Parquet writer uses. Relational
/// hosts insert these rows directly rather than assigning their own term ids.
/// The eager projection retains all rows; physical addressability still applies.
/// File writes consume the same canonical rows without materializing these columns.
///
/// # Errors
///
/// Preserves the original source read failure. Codec failures retain malformed
/// RDF structure, corrupt blob contents and native v1 profile bounds.
pub fn project<D: DatasetView>(
    view: &D,
    blobs: &ContentStore,
) -> Result<crate::projection::ColumnarProjection, ColumnarWriteError<D::ReadError>> {
    view.checked_read(|view| {
        let prepared = prepare(view, blobs)?;
        let mut tables = Vec::new();
        for table in Table::ALL {
            tables.push(materialize(table, prepared.rows(table))?);
        }
        Ok(crate::projection::ColumnarProjection::from_tables(
            tables.try_into().expect("five schema tables"),
        ))
    })
    .map_err(ColumnarWriteError::SourceRead)?
}

fn prepare<'a, D: DatasetView>(
    view: &D,
    blobs: &'a ContentStore,
) -> Result<Prepared<'a>, ColumnarWriteError<D::ReadError>> {
    blobs.verify_all().map_err(ColumnarError::ContentStore)?;
    let source = SourceRows::collect(view);
    let mut resolver = Resolver::new(view);
    source.resolve_all(&mut resolver)?;
    let dictionary = Dictionary::build(&resolver, &source.named_graphs)?;
    let quads = quad_rows(&source.quads, &resolver, &dictionary)?;
    let annotations = quad_rows(&source.annotations, &resolver, &dictionary)?;
    let reifiers = reifier_rows(&source, &resolver, &dictionary)?;
    let mut blob_rows: Vec<_> = blobs.iter().collect();
    blob_rows.sort_unstable_by_key(|(digest, _)| **digest);
    let blobs = blob_rows
        .into_iter()
        .map(|(digest, payload)| (digest.to_hex(), payload.as_slice()))
        .collect();
    Ok(Prepared {
        dictionary,
        quads,
        reifiers,
        annotations,
        blobs,
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

    /// Resolve every original RDF stream and graph through the same term memo.
    fn resolve_all<D>(
        &self,
        resolver: &mut Resolver<'_, D>,
    ) -> Result<(), ColumnarWriteError<D::ReadError>>
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

/// Resolve original positions in subject, predicate, object and graph order.
fn resolve_quad<D: DatasetView>(
    resolver: &mut Resolver<'_, D>,
    quad: &QuadIds<D::Id>,
    include_predicate: bool,
) -> Result<(), ColumnarWriteError<D::ReadError>> {
    resolver.resolve(quad.s)?;
    if include_predicate {
        resolver.resolve(quad.p)?;
    }
    resolver.resolve(quad.o)?;
    resolve_graph(resolver, quad.g)
}

/// Resolve a named graph without inventing a term for the default graph.
fn resolve_graph<D: DatasetView>(
    resolver: &mut Resolver<'_, D>,
    graph: Option<D::Id>,
) -> Result<(), ColumnarWriteError<D::ReadError>> {
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

    /// Resolve source term `id`, memoised in `values` with `active` holding the
    /// terms being resolved.
    ///
    /// The walk runs over a work list of terms being resolved: a term is entered by
    /// first consulting `values`, then refusing a term already being resolved (a
    /// cycle); a literal's datatype is resolved fully before the literal is checked,
    /// and a triple term's subject, predicate and object each fully, in that order,
    /// before the triple is assembled. Every term is memoised as soon as its value
    /// exists, and the first refusal ends the walk.
    fn resolve(&mut self, id: D::Id) -> Result<TermValue, ColumnarWriteError<D::ReadError>> {
        /// A term being resolved, the terms it depends on in resolution order, and
        /// the values they resolved to so far.
        struct Frame<I, G> {
            id: I,
            guard: G,
            dependencies: ([I; 3], usize),
            resolved: Vec<TermValue>,
        }
        type EnterResult<I, G, E> = Result<Result<TermValue, Frame<I, G>>, ColumnarWriteError<E>>;
        let view = self.view;
        let enter =
            |this: &mut Self, id: D::Id| -> EnterResult<D::Id, D::TermGuard<'a>, D::ReadError> {
                if let Some(value) = this.values.get(&id) {
                    return Ok(Ok(value.clone()));
                }
                if !this.active.insert(id) {
                    return Err(ColumnarError::projection(
                        "term graph",
                        ProjectionReason::CyclicTriple,
                    )
                    .into());
                }
                let guard = view.resolve(id).map_err(ColumnarWriteError::SourceRead)?;
                let dependencies = match guard.term() {
                    TermRef::Literal { datatype, .. } => ([datatype, datatype, datatype], 1),
                    TermRef::Triple { s, p, o } => ([s, p, o], 3),
                    TermRef::Iri(_) | TermRef::Blank { .. } => ([id, id, id], 0),
                };
                Ok(Err(Frame {
                    id,
                    guard,
                    dependencies,
                    resolved: Vec::new(),
                }))
            };
        let mut frames = match enter(self, id)? {
            Ok(value) => return Ok(value),
            Err(frame) => vec![frame],
        };
        loop {
            let frame = frames.last_mut().expect("a term is being resolved");
            let (dependencies, count) = frame.dependencies;
            if frame.resolved.len() < count {
                match enter(self, dependencies[frame.resolved.len()])? {
                    Ok(value) => frame.resolved.push(value),
                    Err(inner) => frames.push(inner),
                }
                continue;
            }
            let Frame {
                id,
                guard,
                resolved,
                ..
            } = frames.pop().expect("a term is being resolved");
            let mut resolved = resolved.into_iter();
            let value = match guard.term() {
                TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
                TermRef::Blank { label, scope } => TermValue::Blank {
                    label: label.to_owned(),
                    scope,
                },
                TermRef::Literal {
                    lexical,
                    language,
                    direction,
                    ..
                } => {
                    // A completed literal frame has resolved its one datatype dependency.
                    let actual = resolved.next().expect("a literal's datatype is resolved");
                    let TermValue::Iri(datatype) = actual else {
                        return Err(ColumnarError::projection(
                            "literal datatype",
                            ProjectionReason::DatatypeNotIri { actual },
                        )
                        .into());
                    };
                    TermValue::Literal {
                        lexical_form: lexical.to_owned(),
                        datatype,
                        language: language.map(str::to_owned),
                        direction,
                    }
                }
                TermRef::Triple { .. } => TermValue::Triple {
                    s: TermBox::new(resolved.next().expect("a triple's subject is resolved")),
                    p: TermBox::new(resolved.next().expect("a triple's predicate is resolved")),
                    o: TermBox::new(resolved.next().expect("a triple's object is resolved")),
                },
            };
            self.active.remove(&id);
            self.values.insert(id, value.clone());
            match frames.last_mut() {
                Some(parent) => parent.resolved.push(value),
                None => return Ok(value),
            }
        }
    }

    fn get(&self, id: D::Id) -> Result<&TermValue, ColumnarError> {
        self.values.get(&id).ok_or_else(|| {
            ColumnarError::projection("term closure", ProjectionReason::UnresolvedSource)
        })
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
            ColumnarError::projection("term reference", ProjectionReason::MissingTerm)
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

struct Prepared<'a> {
    dictionary: Dictionary,
    quads: BTreeSet<QuadRow>,
    reifiers: BTreeSet<ReifierRow>,
    annotations: BTreeSet<QuadRow>,
    blobs: Vec<(String, &'a [u8])>,
}

impl Prepared<'_> {
    fn rows(
        &self,
        table: Table,
    ) -> Box<
        dyn Iterator<Item = Result<purrdf_core::SmallVec<[ProjectionCell<'_>; 11]>, ColumnarError>>
            + '_,
    > {
        use ProjectionCell::{Bytes, Int64};
        match table {
            Table::Terms => Box::new(
                self.dictionary
                    .terms
                    .iter()
                    .enumerate()
                    .map(|(id, term)| term_row(&self.dictionary, id, term)),
            ),
            Table::Quads | Table::Annotations => {
                let rows = if table == Table::Quads {
                    &self.quads
                } else {
                    &self.annotations
                };
                Box::new(rows.iter().map(|&(s, p, o, g)| {
                    Ok(purrdf_core::smallvec![
                        Int64(s),
                        Int64(p),
                        Int64(o),
                        optional(g)
                    ])
                }))
            }
            Table::Reifiers => Box::new(self.reifiers.iter().map(|&(r, s, p, o, g)| {
                Ok(purrdf_core::smallvec![
                    Int64(r),
                    Int64(s),
                    Int64(p),
                    Int64(o),
                    optional(g)
                ])
            })),
            Table::Blobs => Box::new(self.blobs.iter().map(|(digest, payload)| {
                Ok(purrdf_core::smallvec![
                    Bytes(digest.as_bytes()),
                    Bytes(payload)
                ])
            })),
        }
    }
}

fn optional(value: Option<i64>) -> ProjectionCell<'static> {
    value.map_or(ProjectionCell::Null, ProjectionCell::Int64)
}

fn term_row<'a>(
    dictionary: &Dictionary,
    index: usize,
    term: &'a TermValue,
) -> Result<purrdf_core::SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError> {
    use ProjectionCell::{Bytes, Int64, Null};
    let mut row: purrdf_core::SmallVec<[_; 11]> = purrdf_core::smallvec![Null; 11];
    row[0] = Int64(
        i64::try_from(index)
            .map_err(|_| ColumnarError::limit("term id", index, i64::MAX as usize))?,
    );
    row[10] = Int64(i64::from(dictionary.named_graphs.contains(term)));
    match term {
        TermValue::Iri(iri) => {
            row[1] = Int64(0);
            row[2] = Bytes(iri.as_bytes());
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            row[1] = Int64(1);
            row[2] = Bytes(lexical_form.as_bytes());
            row[3] = Int64(dictionary.id(&TermValue::Iri(datatype.clone()))?);
            row[4] = language
                .as_ref()
                .map_or(Null, |value| Bytes(value.as_bytes()));
            row[5] = optional(direction.map(direction_id));
        }
        TermValue::Blank { label, scope } => {
            row[1] = Int64(2);
            row[2] = Bytes(label.as_bytes());
            row[6] = Int64(i64::from(scope.ordinal()));
        }
        TermValue::Triple { s, p, o } => {
            row[1] = Int64(3);
            row[7] = Int64(dictionary.id(s)?);
            row[8] = Int64(dictionary.id(p)?);
            row[9] = Int64(dictionary.id(o)?);
        }
    }
    Ok(row)
}

const fn direction_id(direction: RdfTextDirection) -> i64 {
    match direction {
        RdfTextDirection::Ltr => 0,
        RdfTextDirection::Rtl => 1,
    }
}

fn reifier_rows<D: DatasetView>(
    source: &SourceRows<D::Id>,
    resolver: &Resolver<'_, D>,
    dictionary: &Dictionary,
) -> Result<BTreeSet<ReifierRow>, ColumnarError> {
    let mut reifiers = BTreeSet::new();
    for quad in &source.reifiers {
        let TermValue::Triple { s, p, o } = resolver.get(quad.o)? else {
            return Err(ColumnarError::projection(
                "reifier binding",
                ProjectionReason::ReifierObject,
            ));
        };
        reifiers.insert((
            dictionary.source_id(resolver, quad.s)?,
            dictionary.id(s)?,
            dictionary.id(p)?,
            dictionary.id(o)?,
            dictionary.graph_id(resolver, quad.g)?,
        ));
    }
    Ok(reifiers)
}

fn quad_rows<D: DatasetView>(
    source: &[QuadIds<D::Id>],
    resolver: &Resolver<'_, D>,
    dictionary: &Dictionary,
) -> Result<BTreeSet<QuadRow>, ColumnarError> {
    source
        .iter()
        .map(|quad| {
            Ok((
                dictionary.source_id(resolver, quad.s)?,
                dictionary.source_id(resolver, quad.p)?,
                dictionary.source_id(resolver, quad.o)?,
                dictionary.graph_id(resolver, quad.g)?,
            ))
        })
        .collect()
}

/// The explicitly eager relational API owns cells; the file path above borrows them.
fn materialize<'a>(
    table: Table,
    rows: impl Iterator<Item = Result<purrdf_core::SmallVec<[ProjectionCell<'a>; 11]>, ColumnarError>>,
) -> Result<TableData, ColumnarError> {
    let mut columns: Vec<_> = table
        .schema()
        .columns
        .iter()
        .map(|column| match column.physical_type {
            crate::PhysicalType::Int64 => ColumnValues::int64(Int64Column::default()),
            crate::PhysicalType::ByteArray => ColumnValues::bytes(Vec::new()),
        })
        .collect();
    for row in rows {
        for (column, cell) in columns.iter_mut().zip(row?) {
            match (column, cell) {
                (ColumnValues::Int64(values), ProjectionCell::Int64(value)) => {
                    values.push(Some(value));
                }
                (ColumnValues::Int64(values), ProjectionCell::Null) => values.push(None),
                (ColumnValues::ByteArray(values), ProjectionCell::Bytes(value)) => {
                    values.push(Some(value.to_vec()));
                }
                (ColumnValues::ByteArray(values), ProjectionCell::Null) => values.push(None),
                _ => unreachable!("canonical rows match the schema"),
            }
        }
    }
    TableData::new(table, columns)
}

#[cfg(test)]
mod tests {
    use purrdf_core::{
        BlankScope, ContentStore, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral,
        RdfTextDirection,
    };

    use super::*;
    use crate::parquet::read_table;

    fn fixture() -> (std::sync::Arc<RdfDataset>, ContentStore) {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("subject", BlankScope(7)).unwrap();
        let predicate = builder.intern_iri("https://example.org/p").unwrap();
        let object = builder
            .intern_literal(RdfLiteral::typed("42", "https://example.org/integer"))
            .unwrap();
        let directional = builder
            .intern_literal(RdfLiteral {
                lexical_form: "مرحبا".to_owned(),
                datatype: None,
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
            .unwrap();
        let graph = builder.intern_iri("https://example.org/graph").unwrap();
        let empty_graph = builder.intern_iri("https://example.org/empty").unwrap();
        builder.declare_named_graph(empty_graph).unwrap();
        builder
            .push_quad(subject, predicate, object, Some(graph))
            .unwrap();

        let triple = builder
            .intern_triple(subject, predicate, directional)
            .unwrap();
        let reifier = builder.intern_blank("reifier", BlankScope(7)).unwrap();
        builder
            .push_reifier_in_graph(reifier, triple, Some(graph))
            .unwrap();
        builder
            .push_annotation_in_graph(reifier, predicate, object, None)
            .unwrap();

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
                .rows()
                .filter(|value| *value == Some(1))
                .count(),
            2
        );

        // Refuse real invariant failures after all source/workspace setup. These
        // static grammar paths must not allocate a diagnostic string.
        let id = dataset.as_ref().quads().next().unwrap().s;
        let mut cyclic = Resolver::new(&*dataset);
        cyclic.active.insert(id);
        let empty = Resolver::new(&*dataset);
        let dictionary = Dictionary {
            terms: Vec::new(),
            ids: BTreeMap::new(),
            named_graphs: BTreeSet::new(),
        };
        let missing = TermValue::Iri("https://example.org/missing".to_owned());
        let mut nontriple = Resolver::new(&*dataset);
        nontriple.values.insert(id, missing.clone());
        let reifier = SourceRows {
            quads: Vec::new(),
            reifiers: vec![QuadIds {
                s: id,
                p: id,
                o: id,
                g: None,
            }],
            annotations: Vec::new(),
            named_graphs: Vec::new(),
        };
        for (case, code) in [
            "columnar-projection-cyclic-triple",
            "columnar-projection-unresolved-source",
            "columnar-projection-missing-term",
            "columnar-projection-reifier-object",
        ]
        .into_iter()
        .enumerate()
        {
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let error: ColumnarWriteError<core::convert::Infallible> = match case {
                0 => cyclic.resolve(id).unwrap_err(),
                1 => empty.get(id).unwrap_err().into(),
                2 => dictionary.id(&missing).unwrap_err().into(),
                3 => reifier_rows(&reifier, &nontriple, &dictionary)
                    .unwrap_err()
                    .into(),
                _ => unreachable!("each fixture names one production refusal"),
            };
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            let captured = purrdf_testkit::error_capture::Capture::read(&error);
            assert_eq!(captured.child("cause").child("reason").code, code);
        }
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

    #[test]
    fn pack_view_and_native_dataset_emit_identical_files() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("subject", BlankScope(9)).unwrap();
        let predicate = builder.intern_iri("https://example.org/p").unwrap();
        let object = builder
            .intern_literal(RdfLiteral::simple("object"))
            .unwrap();
        let graph = builder.intern_iri("https://example.org/graph").unwrap();
        builder
            .push_quad(subject, predicate, object, Some(graph))
            .unwrap();
        let triple = builder.intern_triple(subject, predicate, object).unwrap();
        let reifier = builder.intern_iri("https://example.org/reifier").unwrap();
        builder
            .push_reifier_in_graph(reifier, triple, Some(graph))
            .unwrap();
        builder.push_annotation(reifier, predicate, object).unwrap();
        let dataset = builder.freeze().unwrap();
        let blobs = ContentStore::new();
        let pack_bytes = PackBuilder::build_bytes(&dataset).unwrap();
        let pack = PackView::from_bytes(&pack_bytes).unwrap();
        let native = write(&*dataset, &blobs, Compression::Zstd).unwrap();
        let packed = write(&pack, &blobs, Compression::Zstd).unwrap();
        assert_eq!(packed, native);
    }

    #[test]
    fn term_and_quad_ingest_order_do_not_change_bytes() {
        fn build(reverse: bool) -> std::sync::Arc<RdfDataset> {
            let mut builder = RdfDatasetBuilder::new();
            let (subject, second_subject, predicate, object, graph, reifier) = if reverse {
                let reifier = builder.intern_iri("https://example.org/reifier").unwrap();
                let graph = builder.intern_iri("https://example.org/graph").unwrap();
                let object = builder
                    .intern_literal(RdfLiteral::simple("object"))
                    .unwrap();
                let predicate = builder.intern_iri("https://example.org/p").unwrap();
                let second_subject = builder.intern_iri("https://example.org/second").unwrap();
                let subject = builder.intern_blank("subject", BlankScope(4)).unwrap();
                (subject, second_subject, predicate, object, graph, reifier)
            } else {
                let subject = builder.intern_blank("subject", BlankScope(4)).unwrap();
                let second_subject = builder.intern_iri("https://example.org/second").unwrap();
                let predicate = builder.intern_iri("https://example.org/p").unwrap();
                let object = builder
                    .intern_literal(RdfLiteral::simple("object"))
                    .unwrap();
                let graph = builder.intern_iri("https://example.org/graph").unwrap();
                let reifier = builder.intern_iri("https://example.org/reifier").unwrap();
                (subject, second_subject, predicate, object, graph, reifier)
            };
            let rows = [
                (subject, predicate, object, None),
                (second_subject, predicate, subject, Some(graph)),
            ];
            for &(s, p, o, g) in if reverse {
                rows.iter().rev().collect::<Vec<_>>()
            } else {
                rows.iter().collect()
            } {
                builder.push_quad(s, p, o, g).unwrap();
            }
            let triple = builder.intern_triple(subject, predicate, object).unwrap();
            builder
                .push_reifier_in_graph(reifier, triple, Some(graph))
                .unwrap();
            builder.push_annotation(reifier, predicate, object).unwrap();
            builder.freeze().unwrap()
        }

        let blobs = ContentStore::new();
        for compression in [Compression::Uncompressed, Compression::Zstd] {
            let forward = write(&*build(false), &blobs, compression).unwrap();
            let reverse = write(&*build(true), &blobs, compression).unwrap();
            assert_eq!(forward, reverse);
        }
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The source-term resolution against its recursive reference.

    use purrdf_core::backend::TermFactory as _;
    use purrdf_core::{
        DatasetView, RdfDataset, RdfDatasetBuilder, TermBox, TermGuard as _, TermRef, TermValue,
    };

    use super::{ColumnarError, ColumnarWriteError, ProjectionReason, Resolver};

    /// The recursive oracle for original term order and source refusals.
    fn reference<D: DatasetView>(
        resolver: &mut Resolver<'_, D>,
        id: D::Id,
    ) -> Result<TermValue, ColumnarWriteError<D::ReadError>> {
        if let Some(value) = resolver.values.get(&id) {
            return Ok(value.clone());
        }
        if !resolver.active.insert(id) {
            return Err(
                ColumnarError::projection("term graph", ProjectionReason::CyclicTriple).into(),
            );
        }
        let view = resolver.view;
        let guard = view.resolve(id).map_err(ColumnarWriteError::SourceRead)?;
        let value = match guard.term() {
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
                let actual = reference(resolver, datatype)?;
                let TermValue::Iri(datatype) = actual else {
                    return Err(ColumnarError::projection(
                        "literal datatype",
                        ProjectionReason::DatatypeNotIri { actual },
                    )
                    .into());
                };
                TermValue::Literal {
                    lexical_form: lexical.to_owned(),
                    datatype,
                    language: language.map(str::to_owned),
                    direction,
                }
            }
            TermRef::Triple { s, p, o } => TermValue::Triple {
                s: TermBox::new(reference(resolver, s)?),
                p: TermBox::new(reference(resolver, p)?),
                o: TermBox::new(reference(resolver, o)?),
            },
        };
        resolver.active.remove(&id);
        resolver.values.insert(id, value.clone());
        Ok(value)
    }

    /// Every term of a dataset holding a generated term resolves — each twice, the second
    /// time through the memo — exactly as the recursive reference resolves it, leaving the
    /// same memo.
    #[test]
    fn resolution_agrees_with_its_recursive_reference_on_generated_terms() {
        for seed in 0..300_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::WellFormed,
            );
            let mut builder = RdfDatasetBuilder::new();
            let object = builder.intern_value(&value).unwrap();
            let holder = builder.intern_iri("http://example.org/holder").unwrap();
            builder.push_quad(holder, holder, object, None).unwrap();
            let dataset: std::sync::Arc<RdfDataset> =
                builder.freeze().expect("a generated term freezes");
            let (mut found, mut expected) = (Resolver::new(&*dataset), Resolver::new(&*dataset));
            for quad in dataset.quads() {
                for id in [quad.s, quad.p, quad.o, quad.o] {
                    assert_eq!(
                        format!("{:?}", found.resolve(id)),
                        format!("{:?}", reference(&mut expected, id)),
                        "seed {seed}"
                    );
                }
            }
            assert_eq!(found.values, expected.values, "seed {seed}");
        }
    }
}
