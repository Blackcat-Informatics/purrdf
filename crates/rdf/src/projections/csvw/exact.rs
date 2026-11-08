// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical CSVW table group carrying an exact RDF 1.2 dataset.

use crate::projections::util::{RecordBudget, csv_field, csv_row_path};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::sync::Arc;

use purrdf_core::csv::{Dialect, Reader, StringRecord, Writer};
use purrdf_core::{
    DatasetView, LossLedger, RdfDataset, RdfDatasetBuilder, RdfTextDirection, TermId, TermRef,
};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_lex::json::{Object, Value};

use super::super::native_term::{
    CapturedTerm, NativeCapture, canonical_row_json, canonical_term_json, check_depth,
};
use super::super::term::{validate_blank_label, validate_literal_identity};
use super::super::util::canonical_json_bounded;
use super::super::validate_absolute_iri;
use super::super::{ProjectionError, ProjectionLimits, ProjectionPackage, stable_identifier};
use super::CsvwConfig;
use purrdf_lex::json::record::ToJson;

use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;
const METADATA_PATH: &str = "csvw-metadata.json";
const TERMS_PATH: &str = "terms.csv";
const QUADS_PATH: &str = "quads.csv";
const REIFIERS_PATH: &str = "reifiers.csv";
const ANNOTATIONS_PATH: &str = "annotations.csv";

const TERM_HEADER: [&str; 11] = [
    "term_id",
    "kind",
    "value",
    "datatype_id",
    "language",
    "direction",
    "blank_scope",
    "subject_id",
    "predicate_id",
    "object_id",
    "named_graph",
];
const QUAD_HEADER: [&str; 5] = [
    "quad_id",
    "subject_id",
    "predicate_id",
    "object_id",
    "graph_id",
];
const REIFIER_HEADER: [&str; 4] = ["reifier_row_id", "reifier_id", "statement_id", "graph_id"];
const ANNOTATION_HEADER: [&str; 5] = [
    "annotation_id",
    "reifier_id",
    "predicate_id",
    "object_id",
    "graph_id",
];

/// Exact, lossless RDF 1.2 → CSVW result.
#[derive(Debug, Clone)]
pub struct CsvwExactProjection {
    /// Deterministic five-artifact CSVW package.
    pub package: ProjectionPackage,
    /// Always empty: the exact profile preserves the complete dataset model.
    pub loss_ledger: LossLedger,
}

/// Exact CSVW → RDF 1.2 result.
#[derive(Debug, Clone)]
pub struct CsvwExactReadOutcome {
    /// Reconstructed validated RDF 1.2 dataset.
    pub dataset: Arc<RdfDataset>,
    /// Always empty: no interpretation or semantic lowering occurs.
    pub loss_ledger: LossLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExactQuad {
    subject: TermId,
    predicate: TermId,
    object: TermId,
    graph: Option<TermId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExactReifier {
    reifier: TermId,
    statement: TermId,
    graph: Option<TermId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExactAnnotation {
    reifier: TermId,
    predicate: TermId,
    object: TermId,
    graph: Option<TermId>,
}

impl ExactQuad {
    fn fields(&self) -> [(&'static str, Option<TermId>); 4] {
        [
            ("subject", Some(self.subject)),
            ("predicate", Some(self.predicate)),
            ("object", Some(self.object)),
            ("graph", self.graph),
        ]
    }
}

impl ExactReifier {
    fn fields(&self) -> [(&'static str, Option<TermId>); 3] {
        [
            ("reifier", Some(self.reifier)),
            ("statement", Some(self.statement)),
            ("graph", self.graph),
        ]
    }
}

impl ExactAnnotation {
    fn fields(&self) -> [(&'static str, Option<TermId>); 4] {
        [
            ("reifier", Some(self.reifier)),
            ("predicate", Some(self.predicate)),
            ("object", Some(self.object)),
            ("graph", self.graph),
        ]
    }
}

#[derive(Debug, Clone)]
struct ExactDataset {
    native: Arc<RdfDataset>,
    named_graphs: BTreeSet<TermId>,
    quads: BTreeSet<ExactQuad>,
    reifiers: BTreeSet<ExactReifier>,
    annotations: BTreeSet<ExactAnnotation>,
}

/// Project any RDF dataset view into the canonical exact CSVW profile.
///
/// Term ids and row ids are hashes of canonical RDF values, so output is independent
/// of backend-local term ids and iteration order. Empty named graphs, recursive triple
/// terms, blank scope, literal language/direction/datatype, graph placement, reifier
/// bindings, and annotations are all first-class rows.
///
/// # Errors
///
/// Returns a typed term, dataset-integrity, CSV, package, or resource-limit failure.
pub fn project_csvw_exact<D: DatasetView>(
    view: &D,
    config: &CsvwConfig,
) -> Result<CsvwExactProjection, ProjectionError> {
    let dataset = collect_dataset(view, config)?;
    let package = write_exact(&dataset, config)?;
    Ok(CsvwExactProjection {
        package,
        loss_ledger: LossLedger::new(),
    })
}

/// Decode and strictly validate the canonical exact CSVW profile.
///
/// The reader validates the CSVW metadata contract, exact headers, every key and
/// foreign-key reference, recursive term structure, row ids, record limits, and the
/// reconstructed RDF dataset. It then re-encodes and requires byte identity, rejecting
/// non-canonical quoting, ordering, metadata, or extra artifacts.
///
/// # Errors
///
/// Returns a typed syntax, integrity, package, term, or resource-limit failure.
pub fn read_csvw_exact(
    package: &ProjectionPackage,
    config: &CsvwConfig,
) -> Result<CsvwExactReadOutcome, ProjectionError> {
    validate_package_bounds(package, config.limits())?;
    require_artifact_set(package)?;
    let expected_metadata = exact_metadata(config)?;
    if package.required(METADATA_PATH)? != expected_metadata {
        return Err(ProjectionError::syntax(
            "exact CSVW metadata is outside the canonical PurRDF profile",
        )
        .at_path(METADATA_PATH));
    }

    let mut budget = RecordBudget::new(config.max_records(), "CSVW");
    let term_records = read_records(
        package.required(TERMS_PATH)?,
        &TERM_HEADER,
        TERMS_PATH,
        &mut budget,
    )?;
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let DecodedTerms {
        builder,
        by_id: terms,
        named_graphs,
    } = decode_terms(&term_records, config, &mut memory)?;
    let quads = decode_quads(
        &read_records(
            package.required(QUADS_PATH)?,
            &QUAD_HEADER,
            QUADS_PATH,
            &mut budget,
        )?,
        &terms,
        &builder,
        config,
    )?;
    let reifiers = decode_reifiers(
        &read_records(
            package.required(REIFIERS_PATH)?,
            &REIFIER_HEADER,
            REIFIERS_PATH,
            &mut budget,
        )?,
        &terms,
        &builder,
        config,
    )?;
    let annotations = decode_annotations(
        &read_records(
            package.required(ANNOTATIONS_PATH)?,
            &ANNOTATION_HEADER,
            ANNOTATIONS_PATH,
            &mut budget,
        )?,
        &terms,
        &builder,
        config,
    )?;
    let exact = finish_exact(builder, named_graphs, quads, reifiers, annotations)?;
    let canonical = write_exact(&exact, config)?;
    if !package.artifacts().eq(canonical.artifacts()) {
        return Err(ProjectionError::syntax(
            "exact CSVW package is valid but not in canonical PurRDF form",
        ));
    }
    Ok(CsvwExactReadOutcome {
        dataset: Arc::clone(&exact.native),
        loss_ledger: LossLedger::new(),
    })
}

fn collect_dataset<D: DatasetView>(
    view: &D,
    config: &CsvwConfig,
) -> Result<ExactDataset, ProjectionError> {
    let mut budget = RecordBudget::new(config.max_records(), "CSVW");
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut terms = NativeCapture::new(view, config.limits().max_term_depth());
    view.checked_read(|_| ())
        .map_err(ProjectionError::source_read)?;
    let mut named_graphs = BTreeSet::new();
    for graph in view.named_graphs() {
        budget.consume(&[1], "named graph")?;
        let graph = terms.term(graph, &mut memory)?;
        require_resource(terms.builder.resolve(graph), "named graph declaration")?;
        if !named_graphs.insert(graph) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate named graph declaration",
            ));
        }
    }

    let mut quads = BTreeSet::new();
    for quad in view.quads() {
        budget.consume(&[1], "quad")?;
        let row = ExactQuad {
            subject: terms.term(quad.s, &mut memory)?,
            predicate: terms.term(quad.p, &mut memory)?,
            object: terms.term(quad.o, &mut memory)?,
            graph: quad.g.map(|id| terms.term(id, &mut memory)).transpose()?,
        };
        require_statement_positions(
            terms.builder.resolve(row.subject),
            terms.builder.resolve(row.predicate),
            row.graph.map(|id| terms.builder.resolve(id)),
            "quad",
        )?;
        if let Some(graph) = &row.graph {
            named_graphs.insert(*graph);
        }
        if !quads.insert(row) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate RDF quad",
            ));
        }
    }

    let mut reifiers = BTreeSet::new();
    for quad in view.reifier_quads() {
        budget.consume(&[1], "reifier")?;
        let predicate = terms.term(quad.p, &mut memory)?;
        if !matches!(terms.builder.resolve(predicate), TermRef::Iri(RDF_REIFIES)) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a reifier row without rdf:reifies",
            ));
        }
        let row = ExactReifier {
            reifier: terms.term(quad.s, &mut memory)?,
            statement: terms.term(quad.o, &mut memory)?,
            graph: quad.g.map(|id| terms.term(id, &mut memory)).transpose()?,
        };
        require_resource(terms.builder.resolve(row.reifier), "reifier subject")?;
        if !matches!(terms.builder.resolve(row.statement), TermRef::Triple { .. }) {
            return Err(ProjectionError::integrity(
                "rdf:reifies object is not a triple term",
            ));
        }
        if let Some(graph) = &row.graph {
            require_resource(terms.builder.resolve(*graph), "reifier graph")?;
            named_graphs.insert(*graph);
        }
        if !reifiers.insert(row) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate reifier binding",
            ));
        }
    }

    let mut annotations = BTreeSet::new();
    for quad in view.annotation_quads() {
        budget.consume(&[1], "annotation")?;
        let row = ExactAnnotation {
            reifier: terms.term(quad.s, &mut memory)?,
            predicate: terms.term(quad.p, &mut memory)?,
            object: terms.term(quad.o, &mut memory)?,
            graph: quad.g.map(|id| terms.term(id, &mut memory)).transpose()?,
        };
        require_statement_positions(
            terms.builder.resolve(row.reifier),
            terms.builder.resolve(row.predicate),
            row.graph.map(|id| terms.builder.resolve(id)),
            "annotation",
        )?;
        if let Some(graph) = &row.graph {
            named_graphs.insert(*graph);
        }
        if !annotations.insert(row) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate annotation",
            ));
        }
    }
    view.checked_read(|_| ())
        .map_err(ProjectionError::source_read)?;
    finish_exact(terms.builder, named_graphs, quads, reifiers, annotations)
}

fn write_exact(
    dataset: &ExactDataset,
    config: &CsvwConfig,
) -> Result<ProjectionPackage, ProjectionError> {
    let catalog = TermCatalog::build(dataset, config)?;
    let record_count = catalog
        .len()
        .checked_add(dataset.quads.len())
        .and_then(|count| count.checked_add(dataset.reifiers.len()))
        .and_then(|count| count.checked_add(dataset.annotations.len()))
        .ok_or_else(|| ProjectionError::limit("exact CSVW record count overflow"))?;
    if record_count > config.max_records() {
        return Err(ProjectionError::limit(format!(
            "exact CSVW package exceeds the {}-record limit",
            config.max_records()
        )));
    }
    let mut package = ProjectionPackage::new(config.limits());
    package.insert(TERMS_PATH, write_terms(&catalog, dataset, config)?)?;
    package.insert(QUADS_PATH, write_quads(dataset, &catalog, config)?)?;
    package.insert(REIFIERS_PATH, write_reifiers(dataset, &catalog, config)?)?;
    package.insert(
        ANNOTATIONS_PATH,
        write_annotations(dataset, &catalog, config)?,
    )?;
    package.insert(METADATA_PATH, exact_metadata(config)?)?;
    Ok(package)
}

struct TermCatalog {
    by_term: BTreeMap<TermId, String>,
    by_id: BTreeMap<String, TermId>,
}

impl TermCatalog {
    fn build(dataset: &ExactDataset, config: &CsvwConfig) -> Result<Self, ProjectionError> {
        let mut terms = BTreeSet::new();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for graph in &dataset.named_graphs {
            collect_term(&dataset.native, *graph, &mut terms, &mut memory)?;
        }
        for quad in &dataset.quads {
            collect_term(&dataset.native, quad.subject, &mut terms, &mut memory)?;
            collect_term(&dataset.native, quad.predicate, &mut terms, &mut memory)?;
            collect_term(&dataset.native, quad.object, &mut terms, &mut memory)?;
            if let Some(graph) = &quad.graph {
                collect_term(&dataset.native, *graph, &mut terms, &mut memory)?;
            }
        }
        for row in &dataset.reifiers {
            collect_term(&dataset.native, row.reifier, &mut terms, &mut memory)?;
            collect_term(&dataset.native, row.statement, &mut terms, &mut memory)?;
            if let Some(graph) = &row.graph {
                collect_term(&dataset.native, *graph, &mut terms, &mut memory)?;
            }
        }
        for row in &dataset.annotations {
            collect_term(&dataset.native, row.reifier, &mut terms, &mut memory)?;
            collect_term(&dataset.native, row.predicate, &mut terms, &mut memory)?;
            collect_term(&dataset.native, row.object, &mut terms, &mut memory)?;
            if let Some(graph) = &row.graph {
                collect_term(&dataset.native, *graph, &mut terms, &mut memory)?;
            }
        }
        if terms.len() > config.max_records() {
            return Err(ProjectionError::limit(format!(
                "exact CSVW term table exceeds the {}-record limit",
                config.max_records()
            )));
        }
        let mut by_term = BTreeMap::new();
        let mut by_id = BTreeMap::new();
        for term in terms {
            let id = term_identifier(
                |id| dataset.native.as_ref().resolve(id),
                term,
                config.limits(),
            )?;
            if let Some(existing) = by_id.insert(id.clone(), term)
                && existing != term
            {
                return Err(ProjectionError::integrity(
                    "SHA-256 collision between distinct exact CSVW terms",
                ));
            }
            by_term.insert(term, id);
        }
        Ok(Self { by_term, by_id })
    }

    fn id(&self, term: TermId) -> Result<&str, ProjectionError> {
        self.by_term.get(&term).map(String::as_str).ok_or_else(|| {
            ProjectionError::integrity("exact CSVW row references an uncatalogued RDF term")
        })
    }

    fn len(&self) -> usize {
        self.by_term.len()
    }
}

fn collect_term(
    dataset: &RdfDataset,
    term: TermId,
    terms: &mut BTreeSet<TermId>,
    memory: &mut Memory<'_, Resident>,
) -> Result<(), ProjectionError> {
    let mut work = Vec::new();
    let result = (|| {
        memory.push(&mut work, term)?;
        while let Some(term) = work.pop() {
            if !terms.insert(term) {
                continue;
            }
            match dataset.resolve(term) {
                TermRef::Literal { datatype, .. } => memory.push(&mut work, datatype)?,
                TermRef::Triple { s, p, o } => {
                    for id in [o, p, s] {
                        memory.push(&mut work, id)?;
                    }
                }
                TermRef::Iri(_) | TermRef::Blank { .. } => {}
            }
        }
        Ok(())
    })();
    let refund = memory.release_vec(work).map_err(ProjectionError::from);
    result.and(refund)
}

fn write_terms(
    catalog: &TermCatalog,
    dataset: &ExactDataset,
    config: &CsvwConfig,
) -> Result<Vec<u8>, ProjectionError> {
    let mut rows = Vec::with_capacity(catalog.by_id.len());
    for (id, term) in &catalog.by_id {
        let named = dataset.named_graphs.contains(term).to_string();
        let row = match dataset.native.as_ref().resolve(*term) {
            TermRef::Iri(value) => vec![
                id.clone(),
                "iri".to_owned(),
                value.to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                named,
            ],
            TermRef::Blank { label, scope } => vec![
                id.clone(),
                "blank".to_owned(),
                label.to_owned(),
                String::new(),
                String::new(),
                String::new(),
                scope.ordinal().to_string(),
                String::new(),
                String::new(),
                String::new(),
                named,
            ],
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => vec![
                id.clone(),
                "literal".to_owned(),
                lexical.to_owned(),
                catalog.id(datatype)?.to_owned(),
                language.unwrap_or_default().to_owned(),
                direction.map_or_else(String::new, |value| value.as_str().to_owned()),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                named,
            ],
            TermRef::Triple { s, p, o } => vec![
                id.clone(),
                "triple".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                catalog.id(s)?.to_owned(),
                catalog.id(p)?.to_owned(),
                catalog.id(o)?.to_owned(),
                named,
            ],
        };
        rows.push(row);
    }
    write_csv(TERMS_PATH, &TERM_HEADER, rows, config.limits())
}

fn write_quads(
    dataset: &ExactDataset,
    catalog: &TermCatalog,
    config: &CsvwConfig,
) -> Result<Vec<u8>, ProjectionError> {
    let mut rows = BTreeMap::new();
    for quad in &dataset.quads {
        let id = row_identifier(
            "CsvwQuad",
            |id| dataset.native.as_ref().resolve(id),
            &quad.fields(),
            config.limits(),
        )?;
        let row = vec![
            id.clone(),
            catalog.id(quad.subject)?.to_owned(),
            catalog.id(quad.predicate)?.to_owned(),
            catalog.id(quad.object)?.to_owned(),
            quad.graph
                .as_ref()
                .map(|term| catalog.id(*term).map(str::to_owned))
                .transpose()?
                .unwrap_or_default(),
        ];
        if rows.insert(id, row).is_some() {
            return Err(ProjectionError::integrity(
                "duplicate or colliding exact CSVW quad row id",
            ));
        }
    }
    write_csv(
        QUADS_PATH,
        &QUAD_HEADER,
        rows.into_values(),
        config.limits(),
    )
}

fn write_reifiers(
    dataset: &ExactDataset,
    catalog: &TermCatalog,
    config: &CsvwConfig,
) -> Result<Vec<u8>, ProjectionError> {
    let mut rows = BTreeMap::new();
    for reifier in &dataset.reifiers {
        let id = row_identifier(
            "CsvwReifier",
            |id| dataset.native.as_ref().resolve(id),
            &reifier.fields(),
            config.limits(),
        )?;
        let row = vec![
            id.clone(),
            catalog.id(reifier.reifier)?.to_owned(),
            catalog.id(reifier.statement)?.to_owned(),
            reifier
                .graph
                .as_ref()
                .map(|term| catalog.id(*term).map(str::to_owned))
                .transpose()?
                .unwrap_or_default(),
        ];
        if rows.insert(id, row).is_some() {
            return Err(ProjectionError::integrity(
                "duplicate or colliding exact CSVW reifier row id",
            ));
        }
    }
    write_csv(
        REIFIERS_PATH,
        &REIFIER_HEADER,
        rows.into_values(),
        config.limits(),
    )
}

fn write_annotations(
    dataset: &ExactDataset,
    catalog: &TermCatalog,
    config: &CsvwConfig,
) -> Result<Vec<u8>, ProjectionError> {
    let mut rows = BTreeMap::new();
    for annotation in &dataset.annotations {
        let id = row_identifier(
            "CsvwAnnotation",
            |id| dataset.native.as_ref().resolve(id),
            &annotation.fields(),
            config.limits(),
        )?;
        let row = vec![
            id.clone(),
            catalog.id(annotation.reifier)?.to_owned(),
            catalog.id(annotation.predicate)?.to_owned(),
            catalog.id(annotation.object)?.to_owned(),
            annotation
                .graph
                .as_ref()
                .map(|term| catalog.id(*term).map(str::to_owned))
                .transpose()?
                .unwrap_or_default(),
        ];
        if rows.insert(id, row).is_some() {
            return Err(ProjectionError::integrity(
                "duplicate or colliding exact CSVW annotation row id",
            ));
        }
    }
    write_csv(
        ANNOTATIONS_PATH,
        &ANNOTATION_HEADER,
        rows.into_values(),
        config.limits(),
    )
}

struct ExactMetadata<'a> {
    context: (String, LocalContext<'a>),
    id: &'a str,
    tables: Vec<TableMetadata>,
}

struct LocalContext<'a> {
    base: &'a str,
}

struct TableMetadata {
    url: &'static str,
    table_schema: SchemaMetadata,
}

struct SchemaMetadata {
    columns: Vec<ColumnMetadata>,
    primary_key: &'static str,
    foreign_keys: Vec<ForeignKeyMetadata>,
}

struct ColumnMetadata {
    name: &'static str,
    titles: &'static str,
    datatype: &'static str,
    required: bool,
    null: Option<&'static str>,
}

struct ForeignKeyMetadata {
    column_reference: &'static str,
    reference: ForeignKeyReference,
}

struct ForeignKeyReference {
    resource: &'static str,
    column_reference: &'static str,
}

impl ToJson for ExactMetadata<'_> {
    /// `{"@context": [context, {"@base": …}], "@id": …, "tables": […]}`.
    fn to_json(&self) -> Value {
        let (context, local) = &self.context;
        Value::Object(
            Object::new()
                .with(
                    "@context",
                    Value::Array(vec![
                        Value::from(context.as_str()),
                        Value::Object(Object::new().with("@base", local.base)),
                    ]),
                )
                .with("@id", self.id)
                .with("tables", self.tables.to_json()),
        )
    }
}

purrdf_lex::json_record!(impl ToJson for TableMetadata {
    "url" => url,
    "tableSchema" => table_schema,
});

impl ToJson for SchemaMetadata {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("columns", self.columns.to_json())
            .with("primaryKey", self.primary_key);
        if !self.foreign_keys.is_empty() {
            object.insert("foreignKeys", self.foreign_keys.to_json());
        }
        Value::Object(object)
    }
}

impl ToJson for ColumnMetadata {
    fn to_json(&self) -> Value {
        let mut object = Object::new()
            .with("name", self.name)
            .with("titles", self.titles)
            .with("datatype", self.datatype)
            .with("required", self.required);
        if let Some(null) = self.null {
            object.insert("null", null);
        }
        Value::Object(object)
    }
}

impl ToJson for ForeignKeyMetadata {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("columnReference", self.column_reference)
                .with(
                    "reference",
                    Object::new()
                        .with("resource", self.reference.resource)
                        .with("columnReference", self.reference.column_reference),
                ),
        )
    }
}

fn exact_metadata(config: &CsvwConfig) -> Result<Vec<u8>, ProjectionError> {
    let tables = vec![
        table_metadata(
            TERMS_PATH,
            "term_id",
            &TERM_HEADER,
            &[
                ("datatype_id", TERMS_PATH),
                ("subject_id", TERMS_PATH),
                ("predicate_id", TERMS_PATH),
                ("object_id", TERMS_PATH),
            ],
        ),
        table_metadata(
            QUADS_PATH,
            "quad_id",
            &QUAD_HEADER,
            &[
                ("subject_id", TERMS_PATH),
                ("predicate_id", TERMS_PATH),
                ("object_id", TERMS_PATH),
                ("graph_id", TERMS_PATH),
            ],
        ),
        table_metadata(
            REIFIERS_PATH,
            "reifier_row_id",
            &REIFIER_HEADER,
            &[
                ("reifier_id", TERMS_PATH),
                ("statement_id", TERMS_PATH),
                ("graph_id", TERMS_PATH),
            ],
        ),
        table_metadata(
            ANNOTATIONS_PATH,
            "annotation_id",
            &ANNOTATION_HEADER,
            &[
                ("reifier_id", TERMS_PATH),
                ("predicate_id", TERMS_PATH),
                ("object_id", TERMS_PATH),
                ("graph_id", TERMS_PATH),
            ],
        ),
    ];
    canonical_json_bounded(
        &ExactMetadata {
            context: (
                config.context_iri().to_owned(),
                LocalContext {
                    base: config.metadata_base_iri(),
                },
            ),
            id: config.table_group_iri(),
            tables,
        },
        config.limits(),
        "exact CSVW metadata",
    )
}

fn table_metadata(
    url: &'static str,
    primary_key: &'static str,
    header: &[&'static str],
    foreign_keys: &[(&'static str, &'static str)],
) -> TableMetadata {
    let columns = header
        .iter()
        .map(|name| {
            let datatype = match *name {
                "blank_scope" => "integer",
                "named_graph" => "boolean",
                _ => "string",
            };
            let required = matches!(
                *name,
                "term_id"
                    | "kind"
                    | "named_graph"
                    | "quad_id"
                    | "subject_id"
                    | "predicate_id"
                    | "object_id"
                    | "reifier_row_id"
                    | "reifier_id"
                    | "statement_id"
                    | "annotation_id"
            ) && *name != "graph_id";
            ColumnMetadata {
                name,
                titles: name,
                datatype,
                required,
                null: (!required).then_some(""),
            }
        })
        .collect();
    let foreign_keys = foreign_keys
        .iter()
        .map(|(column, resource)| ForeignKeyMetadata {
            column_reference: column,
            reference: ForeignKeyReference {
                resource,
                column_reference: "term_id",
            },
        })
        .collect();
    TableMetadata {
        url,
        table_schema: SchemaMetadata {
            columns,
            primary_key,
            foreign_keys,
        },
    }
}

#[derive(Debug, Clone)]
struct RawTermRow {
    id: String,
    kind: String,
    value: String,
    datatype_id: String,
    language: String,
    direction: String,
    blank_scope: String,
    subject_id: String,
    predicate_id: String,
    object_id: String,
    named_graph: bool,
    /// Traversal color lives in the buffered declaration, never a second set.
    active: std::cell::Cell<bool>,
}

struct DecodedTerms {
    builder: RdfDatasetBuilder,
    by_id: BTreeMap<String, TermId>,
    named_graphs: BTreeSet<TermId>,
}

fn decode_terms(
    records: &[StringRecord],
    config: &CsvwConfig,
    memory: &mut Memory<'_, Resident>,
) -> Result<DecodedTerms, ProjectionError> {
    let mut raw = BTreeMap::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = csv_row_path(TERMS_PATH, index);
        let id = csv_field(record, 0, &path)?.to_owned();
        require_nonempty(&id, "term_id", &path)?;
        require_strict_order(previous.as_deref(), &id, &path)?;
        previous = Some(id.clone());
        let named_graph = match csv_field(record, 10, &path)? {
            "true" => true,
            "false" => false,
            _ => {
                return Err(ProjectionError::syntax(
                    "named_graph must be the canonical boolean true or false",
                )
                .at_path(path));
            }
        };
        let row = RawTermRow {
            id: id.clone(),
            kind: csv_field(record, 1, &path)?.to_owned(),
            value: csv_field(record, 2, &path)?.to_owned(),
            datatype_id: csv_field(record, 3, &path)?.to_owned(),
            language: csv_field(record, 4, &path)?.to_owned(),
            direction: csv_field(record, 5, &path)?.to_owned(),
            blank_scope: csv_field(record, 6, &path)?.to_owned(),
            subject_id: csv_field(record, 7, &path)?.to_owned(),
            predicate_id: csv_field(record, 8, &path)?.to_owned(),
            object_id: csv_field(record, 9, &path)?.to_owned(),
            named_graph,
            active: std::cell::Cell::new(false),
        };
        if raw.insert(id, row).is_some() {
            return Err(ProjectionError::integrity("duplicate exact CSVW term id").at_path(path));
        }
    }
    let mut terms = BTreeMap::new();
    let mut builder = RdfDatasetBuilder::new();
    for id in raw.keys() {
        resolve_term_row(id, &raw, &mut terms, config, &mut builder, memory)?;
    }
    let mut named_graphs = BTreeSet::new();
    for row in raw.values().filter(|row| row.named_graph) {
        let term = terms
            .get(&row.id)
            .ok_or_else(|| ProjectionError::integrity("resolved term disappeared"))?;
        require_resource(builder.resolve(term.id), "named_graph term")?;
        named_graphs.insert(term.id);
    }
    Ok(DecodedTerms {
        builder,
        by_id: terms.into_iter().map(|(id, term)| (id, term.id)).collect(),
        named_graphs,
    })
}

fn resolve_term_row<'a>(
    id: &'a str,
    raw: &'a BTreeMap<String, RawTermRow>,
    resolved: &mut BTreeMap<String, CapturedTerm>,
    config: &CsvwConfig,
    builder: &mut RdfDatasetBuilder,
    memory: &mut Memory<'_, Resident>,
) -> Result<(), ProjectionError> {
    let mut work = Vec::new();
    let result = (|| {
        memory.push(&mut work, (id, 0usize, false))?;
        while let Some((id, depth, finish)) = work.pop() {
            if !finish && let Some(term) = resolved.get(id) {
                check_depth(term.height, depth, config.limits().max_term_depth())?;
                continue;
            }
            let row = raw.get(id).ok_or_else(|| {
                ProjectionError::integrity("exact CSVW foreign key references an unknown term")
                    .at_path(TERMS_PATH)
            })?;
            if finish {
                let term = intern_term_row(row, raw, resolved, builder, memory)?;
                check_depth(term.height, depth, config.limits().max_term_depth())?;
                if term_identifier(|id| builder.resolve(id), term.id, config.limits())? != row.id {
                    return Err(ProjectionError::integrity(
                        "exact CSVW term id does not match its canonical RDF value",
                    )
                    .at_path(TERMS_PATH));
                }
                row.active.set(false);
                resolved.insert(id.to_owned(), term);
                continue;
            }
            if row.active.replace(true) {
                return Err(ProjectionError::integrity(
                    "cycle in exact CSVW recursive term references",
                )
                .at_path(TERMS_PATH));
            }
            memory.push(&mut work, (id, depth, true))?;
            match row.kind.as_str() {
                "triple" => {
                    check_depth(1, depth, config.limits().max_term_depth())?;
                    let next = depth
                        .checked_add(1)
                        .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                    for child in [&row.object_id, &row.predicate_id, &row.subject_id] {
                        require_nonempty(child, "triple component", TERMS_PATH)?;
                        memory.push(&mut work, (child.as_str(), next, false))?;
                    }
                }
                "literal" => {
                    require_nonempty(&row.datatype_id, "literal datatype_id", TERMS_PATH)?;
                    memory.push(&mut work, (&row.datatype_id, depth, false))?;
                }
                _ => {}
            }
        }
        Ok(())
    })();
    let refund = memory.release_vec(work).map_err(ProjectionError::from);
    result.and(refund)
}

fn intern_term_row(
    row: &RawTermRow,
    raw: &BTreeMap<String, RawTermRow>,
    resolved: &BTreeMap<String, CapturedTerm>,
    builder: &mut RdfDatasetBuilder,
    memory: &mut Memory<'_, Resident>,
) -> Result<CapturedTerm, ProjectionError> {
    let dependency = |id: &str| {
        resolved.get(id).copied().ok_or_else(|| {
            ProjectionError::integrity("exact CSVW continuation has no resolved dependency")
                .at_path(TERMS_PATH)
        })
    };
    let mut height = 0;
    let term = match row.kind.as_str() {
        "iri" => {
            require_empty_fields(
                row,
                &[
                    "datatype_id",
                    "language",
                    "direction",
                    "blank_scope",
                    "subject_id",
                    "predicate_id",
                    "object_id",
                ],
            )?;
            validate_absolute_iri(&row.value, "term IRI")
                .map_err(|error| ProjectionError::term(error.message()))?;
            builder.intern_iri_with_memory(&row.value, memory)?
        }
        "blank" => {
            require_nonempty(&row.value, "blank label", TERMS_PATH)?;
            if !row.datatype_id.is_empty()
                || !row.language.is_empty()
                || !row.direction.is_empty()
                || !row.subject_id.is_empty()
                || !row.predicate_id.is_empty()
                || !row.object_id.is_empty()
            {
                return Err(ProjectionError::integrity(
                    "blank term has fields belonging to another term kind",
                )
                .at_path(TERMS_PATH));
            }
            let scope = row.blank_scope.parse::<u32>().map_err(|error| {
                ProjectionError::syntax(format!("invalid blank_scope: {error}")).at_path(TERMS_PATH)
            })?;
            validate_blank_label(&row.value)?;
            builder.intern_blank_with_memory(&row.value, purrdf_core::BlankScope(scope), memory)?
        }
        "literal" => {
            if !row.blank_scope.is_empty()
                || !row.subject_id.is_empty()
                || !row.predicate_id.is_empty()
                || !row.object_id.is_empty()
                || row.named_graph
            {
                return Err(ProjectionError::integrity(
                    "literal term has fields belonging to another term kind",
                )
                .at_path(TERMS_PATH));
            }
            require_nonempty(&row.datatype_id, "literal datatype_id", TERMS_PATH)?;
            let datatype = dependency(&row.datatype_id)?;
            let TermRef::Iri(_) = builder.resolve(datatype.id) else {
                return Err(ProjectionError::integrity(
                    "literal datatype_id does not identify an IRI term",
                )
                .at_path(TERMS_PATH));
            };
            let direction = if row.direction.is_empty() {
                None
            } else {
                let direction =
                    RdfTextDirection::from_str_token(&row.direction).ok_or_else(|| {
                        ProjectionError::syntax("literal direction must be empty, ltr, or rtl")
                            .at_path(TERMS_PATH)
                    })?;
                Some(direction)
            };
            let datatype = &raw
                .get(&row.datatype_id)
                .ok_or_else(|| ProjectionError::integrity("resolved datatype row disappeared"))?
                .value;
            let language = (!row.language.is_empty()).then_some(row.language.as_str());
            validate_literal_identity(datatype, language, direction)?;
            builder.intern_literal_parts_with_memory(
                &row.value,
                Some(datatype),
                language,
                direction,
                memory,
            )?
        }
        "triple" => {
            if !row.value.is_empty()
                || !row.datatype_id.is_empty()
                || !row.language.is_empty()
                || !row.direction.is_empty()
                || !row.blank_scope.is_empty()
                || row.named_graph
            {
                return Err(ProjectionError::integrity(
                    "triple term has fields belonging to another term kind",
                )
                .at_path(TERMS_PATH));
            }
            for (name, value) in [
                ("subject_id", &row.subject_id),
                ("predicate_id", &row.predicate_id),
                ("object_id", &row.object_id),
            ] {
                require_nonempty(value, name, TERMS_PATH)?;
            }
            let (subject, predicate, object) = (
                dependency(&row.subject_id)?,
                dependency(&row.predicate_id)?,
                dependency(&row.object_id)?,
            );
            require_resource(builder.resolve(subject.id), "triple-term subject")?;
            if !matches!(builder.resolve(predicate.id), TermRef::Iri(_)) {
                return Err(
                    ProjectionError::term("triple-term predicate must be an IRI")
                        .at_path(TERMS_PATH),
                );
            }
            height = subject
                .height
                .max(object.height)
                .checked_add(1)
                .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
            builder.intern_triple_with_memory(subject.id, predicate.id, object.id, memory)?
        }
        other => {
            return Err(
                ProjectionError::syntax(format!("unknown exact CSVW term kind {other:?}"))
                    .at_path(TERMS_PATH),
            );
        }
    };
    Ok(CapturedTerm { id: term, height })
}

fn decode_quads(
    records: &[StringRecord],
    terms: &BTreeMap<String, TermId>,
    builder: &RdfDatasetBuilder,
    config: &CsvwConfig,
) -> Result<BTreeSet<ExactQuad>, ProjectionError> {
    let mut rows = BTreeSet::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = csv_row_path(QUADS_PATH, index);
        let id = csv_field(record, 0, &path)?;
        require_nonempty(id, "quad_id", &path)?;
        require_strict_order(previous.as_deref(), id, &path)?;
        previous = Some(id.to_owned());
        let row = ExactQuad {
            subject: referenced_term(terms, csv_field(record, 1, &path)?, "subject_id", &path)?,
            predicate: referenced_term(terms, csv_field(record, 2, &path)?, "predicate_id", &path)?,
            object: referenced_term(terms, csv_field(record, 3, &path)?, "object_id", &path)?,
            graph: optional_term(terms, csv_field(record, 4, &path)?, "graph_id", &path)?,
        };
        require_statement_positions(
            builder.resolve(row.subject),
            builder.resolve(row.predicate),
            row.graph.map(|id| builder.resolve(id)),
            "quad",
        )?;
        if row_identifier(
            "CsvwQuad",
            |id| builder.resolve(id),
            &row.fields(),
            config.limits(),
        )? != id
        {
            return Err(ProjectionError::integrity(
                "exact CSVW quad id does not match its canonical row",
            )
            .at_path(path));
        }
        if !rows.insert(row) {
            return Err(ProjectionError::integrity("duplicate exact CSVW quad").at_path(path));
        }
    }
    Ok(rows)
}

fn decode_reifiers(
    records: &[StringRecord],
    terms: &BTreeMap<String, TermId>,
    builder: &RdfDatasetBuilder,
    config: &CsvwConfig,
) -> Result<BTreeSet<ExactReifier>, ProjectionError> {
    let mut rows = BTreeSet::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = csv_row_path(REIFIERS_PATH, index);
        let id = csv_field(record, 0, &path)?;
        require_nonempty(id, "reifier_row_id", &path)?;
        require_strict_order(previous.as_deref(), id, &path)?;
        previous = Some(id.to_owned());
        let row = ExactReifier {
            reifier: referenced_term(terms, csv_field(record, 1, &path)?, "reifier_id", &path)?,
            statement: referenced_term(terms, csv_field(record, 2, &path)?, "statement_id", &path)?,
            graph: optional_term(terms, csv_field(record, 3, &path)?, "graph_id", &path)?,
        };
        require_resource(builder.resolve(row.reifier), "reifier subject")?;
        if !matches!(builder.resolve(row.statement), TermRef::Triple { .. }) {
            return Err(ProjectionError::integrity(
                "exact CSVW reifier statement_id is not a triple term",
            )
            .at_path(path));
        }
        if let Some(graph) = &row.graph {
            require_resource(builder.resolve(*graph), "reifier graph")?;
        }
        if row_identifier(
            "CsvwReifier",
            |id| builder.resolve(id),
            &row.fields(),
            config.limits(),
        )? != id
        {
            return Err(ProjectionError::integrity(
                "exact CSVW reifier row id does not match its canonical row",
            )
            .at_path(path));
        }
        if !rows.insert(row) {
            return Err(ProjectionError::integrity("duplicate exact CSVW reifier").at_path(path));
        }
    }
    Ok(rows)
}

fn decode_annotations(
    records: &[StringRecord],
    terms: &BTreeMap<String, TermId>,
    builder: &RdfDatasetBuilder,
    config: &CsvwConfig,
) -> Result<BTreeSet<ExactAnnotation>, ProjectionError> {
    let mut rows = BTreeSet::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = csv_row_path(ANNOTATIONS_PATH, index);
        let id = csv_field(record, 0, &path)?;
        require_nonempty(id, "annotation_id", &path)?;
        require_strict_order(previous.as_deref(), id, &path)?;
        previous = Some(id.to_owned());
        let row = ExactAnnotation {
            reifier: referenced_term(terms, csv_field(record, 1, &path)?, "reifier_id", &path)?,
            predicate: referenced_term(terms, csv_field(record, 2, &path)?, "predicate_id", &path)?,
            object: referenced_term(terms, csv_field(record, 3, &path)?, "object_id", &path)?,
            graph: optional_term(terms, csv_field(record, 4, &path)?, "graph_id", &path)?,
        };
        require_statement_positions(
            builder.resolve(row.reifier),
            builder.resolve(row.predicate),
            row.graph.map(|id| builder.resolve(id)),
            "annotation",
        )?;
        if row_identifier(
            "CsvwAnnotation",
            |id| builder.resolve(id),
            &row.fields(),
            config.limits(),
        )? != id
        {
            return Err(ProjectionError::integrity(
                "exact CSVW annotation id does not match its canonical row",
            )
            .at_path(path));
        }
        if !rows.insert(row) {
            return Err(
                ProjectionError::integrity("duplicate exact CSVW annotation").at_path(path),
            );
        }
    }
    Ok(rows)
}

fn finish_exact(
    mut builder: RdfDatasetBuilder,
    named_graphs: BTreeSet<TermId>,
    quads: BTreeSet<ExactQuad>,
    reifiers: BTreeSet<ExactReifier>,
    annotations: BTreeSet<ExactAnnotation>,
) -> Result<ExactDataset, ProjectionError> {
    for graph in &named_graphs {
        builder.declare_named_graph(*graph)?;
    }
    for quad in &quads {
        builder.push_quad(quad.subject, quad.predicate, quad.object, quad.graph)?;
    }
    for row in &reifiers {
        builder.push_reifier_in_graph(row.reifier, row.statement, row.graph)?;
    }
    for row in &annotations {
        builder.push_annotation_in_graph(row.reifier, row.predicate, row.object, row.graph)?;
    }
    Ok(ExactDataset {
        native: builder.freeze().map_err(ProjectionError::from)?,
        named_graphs,
        quads,
        reifiers,
        annotations,
    })
}

fn require_statement_positions(
    subject: TermRef<'_>,
    predicate: TermRef<'_>,
    graph: Option<TermRef<'_>>,
    description: &str,
) -> Result<(), ProjectionError> {
    require_resource(subject, &format!("{description} subject"))?;
    if !matches!(predicate, TermRef::Iri(_)) {
        return Err(ProjectionError::integrity(format!(
            "{description} predicate is not an IRI"
        )));
    }
    if let Some(graph) = graph {
        require_resource(graph, &format!("{description} graph"))?;
    }
    Ok(())
}

fn require_resource(term: TermRef<'_>, description: &str) -> Result<(), ProjectionError> {
    if !matches!(term, TermRef::Iri(_) | TermRef::Blank { .. }) {
        return Err(ProjectionError::integrity(format!(
            "{description} must be an IRI or blank node"
        )));
    }
    Ok(())
}

fn term_identifier<'a>(
    resolve: impl Fn(TermId) -> TermRef<'a> + Copy,
    term: TermId,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    stable_identifier(
        "CsvwTerm",
        &canonical_term_json(resolve, term, limits.max_artifact_bytes())?,
    )
}

fn row_identifier<'a>(
    prefix: &str,
    resolve: impl Fn(TermId) -> TermRef<'a> + Copy,
    fields: &[(&str, Option<TermId>)],
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    stable_identifier(
        prefix,
        &canonical_row_json(resolve, fields, limits.max_artifact_bytes())?,
    )
}

struct LimitedCsvBytes {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl LimitedCsvBytes {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            exceeded: false,
        }
    }
}

impl io::Write for LimitedCsvBytes {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(buffer.len())
            .is_none_or(|length| length > self.limit)
        {
            self.exceeded = true;
            return Err(io::Error::other("exact CSVW artifact limit exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_csv<I>(
    path: &str,
    header: &[&str],
    rows: I,
    limits: ProjectionLimits,
) -> Result<Vec<u8>, ProjectionError>
where
    I: IntoIterator<Item = Vec<String>>,
{
    let sink = LimitedCsvBytes::new(limits.max_artifact_bytes());
    let mut writer = Writer::new(Dialect::RFC4180, sink);
    write_csv_record(&mut writer, header, path)?;
    for row in rows {
        write_csv_record(&mut writer, row, path)?;
    }
    writer
        .flush()
        .map_err(|error| csv_write_error(error, path))?;
    let sink = writer
        .into_inner()
        .map_err(|error| csv_write_error(error, path))?;
    if sink.exceeded {
        return Err(ProjectionError::limit(
            "exact CSVW CSV exceeds the configured artifact byte limit",
        )
        .at_path(path));
    }
    Ok(sink.bytes)
}

fn write_csv_record<I, T>(
    writer: &mut Writer<LimitedCsvBytes>,
    record: I,
    path: &str,
) -> Result<(), ProjectionError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    writer
        .write_record(record)
        .map_err(|error| csv_write_error(error, path))
}

fn csv_write_error(error: impl std::fmt::Display, path: &str) -> ProjectionError {
    ProjectionError::limit(format!("write exact CSVW CSV: {error}")).at_path(path)
}

fn read_records(
    bytes: &[u8],
    expected_header: &[&str],
    path: &str,
    budget: &mut RecordBudget,
) -> Result<Vec<StringRecord>, ProjectionError> {
    let mut reader = Reader::new(Dialect::RFC4180, bytes);
    let mut records = reader.records();
    let header = records
        .next()
        .ok_or_else(|| ProjectionError::syntax("CSV artifact is empty").at_path(path))?
        .map_err(|error| {
            ProjectionError::syntax(format!("read CSV header: {error}")).at_path(path)
        })?;
    if !header.iter().eq(expected_header.iter().copied()) {
        return Err(ProjectionError::syntax(format!(
            "CSV header does not match the exact profile; expected {expected_header:?}"
        ))
        .at_path(path));
    }
    let mut output = Vec::new();
    for record in records {
        budget.consume(&[1], path)?;
        output.push(record.map_err(|error| {
            ProjectionError::syntax(format!("read CSV row: {error}")).at_path(path)
        })?);
    }
    Ok(output)
}

fn require_nonempty(value: &str, field: &str, path: &str) -> Result<(), ProjectionError> {
    if value.is_empty() {
        Err(
            ProjectionError::integrity(format!("exact CSVW {field} must not be empty"))
                .at_path(path),
        )
    } else {
        Ok(())
    }
}

fn require_strict_order(
    previous: Option<&str>,
    current: &str,
    path: &str,
) -> Result<(), ProjectionError> {
    if previous.is_some_and(|value| value >= current) {
        return Err(ProjectionError::integrity(
            "exact CSVW primary keys must be unique and strictly ordered",
        )
        .at_path(path));
    }
    Ok(())
}

fn require_empty_fields(row: &RawTermRow, names: &[&str]) -> Result<(), ProjectionError> {
    for name in names {
        let value = match *name {
            "datatype_id" => &row.datatype_id,
            "language" => &row.language,
            "direction" => &row.direction,
            "blank_scope" => &row.blank_scope,
            "subject_id" => &row.subject_id,
            "predicate_id" => &row.predicate_id,
            "object_id" => &row.object_id,
            _ => {
                return Err(ProjectionError::integrity(
                    "internal exact CSVW field-name mismatch",
                ));
            }
        };
        if !value.is_empty() {
            return Err(ProjectionError::integrity(format!(
                "exact CSVW {} term unexpectedly sets {name}",
                row.kind
            ))
            .at_path(TERMS_PATH));
        }
    }
    Ok(())
}

fn referenced_term(
    terms: &BTreeMap<String, TermId>,
    id: &str,
    field_name: &str,
    path: &str,
) -> Result<TermId, ProjectionError> {
    require_nonempty(id, field_name, path)?;
    terms.get(id).copied().ok_or_else(|| {
        ProjectionError::integrity(format!(
            "exact CSVW {field_name} references an unknown term id"
        ))
        .at_path(path)
    })
}

fn optional_term(
    terms: &BTreeMap<String, TermId>,
    id: &str,
    field_name: &str,
    path: &str,
) -> Result<Option<TermId>, ProjectionError> {
    if id.is_empty() {
        Ok(None)
    } else {
        referenced_term(terms, id, field_name, path).map(Some)
    }
}

fn require_artifact_set(package: &ProjectionPackage) -> Result<(), ProjectionError> {
    let expected = BTreeSet::from([
        ANNOTATIONS_PATH,
        METADATA_PATH,
        QUADS_PATH,
        REIFIERS_PATH,
        TERMS_PATH,
    ]);
    let actual: BTreeSet<_> = package.artifacts().map(|(path, _)| path).collect();
    if actual != expected {
        return Err(ProjectionError::package(format!(
            "exact CSVW artifact set mismatch; expected {expected:?}, found {actual:?}"
        )));
    }
    Ok(())
}

fn validate_package_bounds(
    package: &ProjectionPackage,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    if package.len() > limits.max_artifacts() {
        return Err(ProjectionError::limit(
            "exact CSVW package exceeds the configured artifact count",
        ));
    }
    if package.total_bytes() > limits.max_total_bytes()
        || package.archive_bytes() > limits.max_archive_bytes()
    {
        return Err(ProjectionError::limit(
            "exact CSVW package exceeds total or archive byte limits",
        ));
    }
    for (path, bytes) in package.artifacts() {
        if bytes.len() > limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(
                "exact CSVW artifact exceeds the configured byte limit",
            )
            .at_path(path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use purrdf_core::{
        BlankScope, PackBuilder, PackView, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
        datasets_isomorphic,
    };

    use super::*;
    use crate::ProjectionErrorKind;

    fn test_config(max_records: usize) -> CsvwConfig {
        test_config_at_depth(max_records, 16)
    }

    fn test_config_at_depth(max_records: usize, depth: usize) -> CsvwConfig {
        CsvwConfig::new(
            "http://example.org/csvw/",
            super::super::CsvwContext::new("http://www.w3.org/ns/csvw", BTreeMap::new())
                .expect("context"),
            "http://example.org/csvw/dataset",
            super::super::CsvwVocabulary::new(
                "http://www.w3.org/ns/csvw#",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "http://www.w3.org/2000/01/rdf-schema#",
                "http://www.w3.org/2001/XMLSchema#",
            )
            .expect("vocabulary"),
            super::super::CsvwMode::Standard,
            ProjectionLimits::new(16, 4_000_000, 16_000_000, 20_000_000, depth).expect("limits"),
            max_records,
        )
        .expect("config")
    }

    fn fixture() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject").unwrap();
        let predicate = builder.intern_iri("http://example.org/predicate").unwrap();
        let graph = builder.intern_blank("graph", BlankScope(9)).unwrap();
        let literal = builder
            .intern_literal(RdfLiteral {
                lexical_form: "مرحبا,\nquoted \" value".to_owned(),
                datatype: None,
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
            .unwrap();
        builder
            .push_quad(subject, predicate, literal, Some(graph))
            .unwrap();
        let quoted = builder.intern_triple(subject, predicate, literal).unwrap();
        let nested = builder.intern_triple(subject, predicate, quoted).unwrap();
        builder.push_quad(subject, predicate, nested, None).unwrap();
        let reifier = builder.intern_blank("reifier", BlankScope(7)).unwrap();
        builder
            .push_reifier_in_graph(reifier, quoted, Some(graph))
            .unwrap();
        let annotation_predicate = builder.intern_iri("http://example.org/confidence").unwrap();
        let annotation_object = builder.intern_iri("http://example.org/high").unwrap();
        builder
            .push_annotation_in_graph(
                reifier,
                annotation_predicate,
                annotation_object,
                Some(graph),
            )
            .unwrap();
        builder.freeze().expect("fixture")
    }

    fn same_artifacts(left: &ProjectionPackage, right: &ProjectionPackage) -> bool {
        left.artifacts().eq(right.artifacts())
    }

    fn replace_artifact(
        package: &ProjectionPackage,
        path: &str,
        replacement: &[u8],
    ) -> ProjectionPackage {
        ProjectionPackage::from_artifacts(
            package.limits(),
            package.artifacts().map(|(candidate, bytes)| {
                (
                    candidate.to_owned(),
                    if candidate == path {
                        replacement.to_vec()
                    } else {
                        bytes.to_vec()
                    },
                )
            }),
        )
        .expect("replacement package")
    }

    #[test]
    fn exact_csvw_is_backend_independent_byte_stable_and_lossless() {
        let dataset = fixture();
        // This fixture has two triple nodes on its longest path: the root is
        // depth zero, so max depth one must round-trip its leaf as well.
        let config = test_config_at_depth(10_000, 1);
        let native = collect_dataset(dataset.as_ref(), &config).expect("native projection model");
        let catalog = TermCatalog::build(&native, &config).expect("reachable term catalog");
        for term in catalog.by_term.keys() {
            let legacy =
                crate::ProjectionTerm::from_view(native.native.as_ref(), *term, config.limits())
                    .expect("bounded legacy identity oracle");
            assert_eq!(
                canonical_term_json(
                    |id| native.native.as_ref().resolve(id),
                    *term,
                    config.limits().max_artifact_bytes()
                )
                .expect("native term JSON"),
                legacy
                    .to_canonical_json(config.limits())
                    .expect("published term law")
            );
        }
        for fields in native
            .quads
            .iter()
            .map(|row| row.fields().to_vec())
            .chain(native.reifiers.iter().map(|row| row.fields().to_vec()))
            .chain(native.annotations.iter().map(|row| row.fields().to_vec()))
        {
            let mut legacy = Object::new();
            for (name, term) in &fields {
                legacy.insert(
                    *name,
                    term.map(|id| {
                        crate::ProjectionTerm::from_view(
                            native.native.as_ref(),
                            id,
                            config.limits(),
                        )
                        .expect("bounded legacy row term")
                    })
                    .to_json(),
                );
            }
            assert_eq!(
                canonical_row_json(
                    |id| native.native.as_ref().resolve(id),
                    &fields,
                    config.limits().max_artifact_bytes()
                )
                .expect("native row JSON"),
                canonical_json_bounded(&Value::Object(legacy), config.limits(), "legacy exact row")
                    .expect("published row identity")
            );
        }
        let projected = project_csvw_exact(dataset.as_ref(), &config).expect("project");
        assert!(projected.loss_ledger.is_empty());
        assert_eq!(projected.package.len(), 5);
        let metadata = purrdf_lex::json::read(
            std::str::from_utf8(
                projected
                    .package
                    .get(METADATA_PATH)
                    .expect("metadata artifact"),
            )
            .expect("metadata UTF-8"),
        )
        .expect("metadata JSON");
        assert_eq!(metadata["tables"].as_array().expect("tables").len(), 4);

        let decoded = read_csvw_exact(&projected.package, &config).expect("read");
        assert!(decoded.loss_ledger.is_empty());
        assert!(
            datasets_isomorphic(
                &dataset,
                &decoded.dataset,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap()
        );
        let rewritten = project_csvw_exact(decoded.dataset.as_ref(), &config).expect("rewrite");
        assert!(same_artifacts(&projected.package, &rewritten.package));

        let pack = PackBuilder::build_bytes(
            &dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .expect("pack");
        let view = PackView::from_bytes(&pack).expect("view");
        let packed = project_csvw_exact(&view, &config).expect("pack projection");
        for ((left_path, left), (right_path, right)) in projected
            .package
            .artifacts()
            .zip(packed.package.artifacts())
        {
            assert_eq!(left_path, right_path);
            assert_eq!(left, right, "backend drift in {left_path}");
        }
        let repeated = project_csvw_exact(dataset.as_ref(), &config).expect("repeat");
        assert!(same_artifacts(&projected.package, &repeated.package));

        let mut over = RdfDatasetBuilder::new();
        let subject = over.intern_iri("http://example.org/s").expect("subject");
        let predicate = over.intern_iri("http://example.org/p").expect("predicate");
        let mut object = subject;
        for _ in 0..3 {
            object = over
                .intern_triple(subject, predicate, object)
                .expect("triple");
        }
        over.push_quad(subject, predicate, object, None)
            .expect("quad");
        let over = over.freeze().expect("valid deeper native source");
        assert_eq!(
            project_csvw_exact(over.as_ref(), &config)
                .expect_err("one triple beyond caller depth")
                .kind(),
            ProjectionErrorKind::ResourceLimit
        );
    }

    #[test]
    fn exact_csvw_rejects_metadata_key_and_record_corruption() {
        let config = test_config(10_000);
        let projected = project_csvw_exact(fixture().as_ref(), &config).expect("project");

        let mut metadata = projected
            .package
            .get(METADATA_PATH)
            .expect("metadata")
            .to_vec();
        metadata.push(b'\n');
        assert!(
            read_csvw_exact(
                &replace_artifact(&projected.package, METADATA_PATH, &metadata),
                &config
            )
            .is_err()
        );

        let quads =
            std::str::from_utf8(projected.package.get(QUADS_PATH).expect("quads")).expect("UTF-8");
        let corrupt = quads.replacen("CsvwTerm_", "Missing__", 1).into_bytes();
        assert!(
            read_csvw_exact(
                &replace_artifact(&projected.package, QUADS_PATH, &corrupt),
                &config
            )
            .is_err()
        );

        let terms = projected.package.get(TERMS_PATH).expect("terms");
        let mut duplicate = terms.to_vec();
        let first_row = terms
            .split(|byte| *byte == b'\n')
            .nth(1)
            .expect("first term row");
        duplicate.extend_from_slice(first_row);
        duplicate.push(b'\n');
        assert!(
            read_csvw_exact(
                &replace_artifact(&projected.package, TERMS_PATH, &duplicate),
                &config
            )
            .is_err()
        );

        // Every declared term is validated, but an unreferenced term is outside
        // the emitted closure and must fail strict canonical re-encoding.
        let orphan = crate::ProjectionTerm::Iri {
            value: "http://example.org/orphan".to_owned(),
        };
        let orphan_id = stable_identifier(
            "CsvwTerm",
            &orphan
                .to_canonical_json(config.limits())
                .expect("orphan identity"),
        )
        .expect("orphan id");
        let mut budget = RecordBudget::new(10_000, "CSVW");
        let mut rows: Vec<Vec<String>> = read_records(terms, &TERM_HEADER, TERMS_PATH, &mut budget)
            .expect("canonical term rows")
            .into_iter()
            .map(|record| record.iter().map(str::to_owned).collect())
            .collect();
        rows.push(vec![
            orphan_id,
            "iri".to_owned(),
            "http://example.org/orphan".to_owned(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "false".to_owned(),
        ]);
        rows.sort_by(|a, b| a[0].cmp(&b[0]));
        let orphan_terms =
            write_csv(TERMS_PATH, &TERM_HEADER, rows, config.limits()).expect("orphan term table");
        let error = read_csvw_exact(
            &replace_artifact(&projected.package, TERMS_PATH, &orphan_terms),
            &config,
        )
        .expect_err("orphan declaration is not canonical");
        assert_eq!(error.kind(), ProjectionErrorKind::Syntax);

        let error = read_csvw_exact(&projected.package, &test_config(1)).expect_err("record limit");
        assert_eq!(error.kind(), ProjectionErrorKind::ResourceLimit);
    }

    #[test]
    fn exact_csvw_preserves_explicit_empty_named_graphs() {
        let mut builder = RdfDatasetBuilder::new();
        let iri = builder
            .intern_iri("http://example.org/empty-iri-graph")
            .unwrap();
        let blank = builder
            .intern_blank("empty-blank-graph", BlankScope(12))
            .unwrap();
        builder.declare_named_graph(iri).unwrap();
        builder.declare_named_graph(blank).unwrap();
        let dataset = builder.freeze().expect("empty named graphs");
        let config = test_config(100);
        let projected = project_csvw_exact(dataset.as_ref(), &config).expect("project");
        let decoded = read_csvw_exact(&projected.package, &config).expect("read");
        assert!(
            datasets_isomorphic(
                &dataset,
                &decoded.dataset,
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .unwrap()
        );
        assert_eq!(decoded.dataset.named_graphs().count(), 2);
    }
}
