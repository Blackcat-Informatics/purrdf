// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical CSVW table group carrying an exact RDF 1.2 dataset.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::sync::Arc;

use csv::{ReaderBuilder, StringRecord, Terminator, Writer, WriterBuilder};
use purrdf_core::{
    BlankScope, DatasetView, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId,
};
use serde::Serialize;

use super::super::util::canonical_json_bounded;
use super::super::{
    ProjectionDirection, ProjectionError, ProjectionLimits, ProjectionPackage, ProjectionTerm,
    stable_identifier,
};
use super::CsvwConfig;

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ExactQuad {
    subject: ProjectionTerm,
    predicate: ProjectionTerm,
    object: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ExactReifier {
    reifier: ProjectionTerm,
    statement: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ExactAnnotation {
    reifier: ProjectionTerm,
    predicate: ProjectionTerm,
    object: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExactDataset {
    named_graphs: BTreeSet<ProjectionTerm>,
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
    if required_artifact(package, METADATA_PATH)? != expected_metadata {
        return Err(ProjectionError::syntax(
            "exact CSVW metadata is outside the canonical PurRDF profile",
        )
        .at_path(METADATA_PATH));
    }

    let mut budget = RecordBudget::new(config.max_records());
    let term_records = read_records(
        required_artifact(package, TERMS_PATH)?,
        &TERM_HEADER,
        TERMS_PATH,
        &mut budget,
    )?;
    let (terms, named_graphs) = decode_terms(&term_records, config)?;
    let quads = decode_quads(
        &read_records(
            required_artifact(package, QUADS_PATH)?,
            &QUAD_HEADER,
            QUADS_PATH,
            &mut budget,
        )?,
        &terms,
        config,
    )?;
    let reifiers = decode_reifiers(
        &read_records(
            required_artifact(package, REIFIERS_PATH)?,
            &REIFIER_HEADER,
            REIFIERS_PATH,
            &mut budget,
        )?,
        &terms,
        config,
    )?;
    let annotations = decode_annotations(
        &read_records(
            required_artifact(package, ANNOTATIONS_PATH)?,
            &ANNOTATION_HEADER,
            ANNOTATIONS_PATH,
            &mut budget,
        )?,
        &terms,
        config,
    )?;
    let exact = ExactDataset {
        named_graphs,
        quads,
        reifiers,
        annotations,
    };
    let dataset = lift_exact(&exact)?;
    let canonical = write_exact(&exact, config)?;
    if !package.artifacts().eq(canonical.artifacts()) {
        return Err(ProjectionError::syntax(
            "exact CSVW package is valid but not in canonical PurRDF form",
        ));
    }
    Ok(CsvwExactReadOutcome {
        dataset,
        loss_ledger: LossLedger::new(),
    })
}

fn collect_dataset<D: DatasetView>(
    view: &D,
    config: &CsvwConfig,
) -> Result<ExactDataset, ProjectionError> {
    let mut budget = RecordBudget::new(config.max_records());
    let mut cache = BTreeMap::new();
    let mut named_graphs = BTreeSet::new();
    for graph in view.named_graphs() {
        budget.consume("named graph")?;
        let graph = resolve_term(view, graph, config.limits(), &mut cache)?;
        require_graph_name(&graph, "named graph declaration")?;
        if !named_graphs.insert(graph) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate named graph declaration",
            ));
        }
    }

    let mut quads = BTreeSet::new();
    for quad in view.quads() {
        budget.consume("quad")?;
        let row = ExactQuad {
            subject: resolve_term(view, quad.s, config.limits(), &mut cache)?,
            predicate: resolve_term(view, quad.p, config.limits(), &mut cache)?,
            object: resolve_term(view, quad.o, config.limits(), &mut cache)?,
            graph: quad
                .g
                .map(|id| resolve_term(view, id, config.limits(), &mut cache))
                .transpose()?,
        };
        require_statement_positions(&row.subject, &row.predicate, row.graph.as_ref(), "quad")?;
        if let Some(graph) = &row.graph {
            named_graphs.insert(graph.clone());
        }
        if !quads.insert(row) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate RDF quad",
            ));
        }
    }

    let expected_reifies = ProjectionTerm::Iri {
        value: RDF_REIFIES.to_owned(),
    };
    let mut reifiers = BTreeSet::new();
    for quad in view.reifier_quads() {
        budget.consume("reifier")?;
        let predicate = resolve_term(view, quad.p, config.limits(), &mut cache)?;
        if predicate != expected_reifies {
            return Err(ProjectionError::integrity(
                "dataset view exposed a reifier row without rdf:reifies",
            ));
        }
        let row = ExactReifier {
            reifier: resolve_term(view, quad.s, config.limits(), &mut cache)?,
            statement: resolve_term(view, quad.o, config.limits(), &mut cache)?,
            graph: quad
                .g
                .map(|id| resolve_term(view, id, config.limits(), &mut cache))
                .transpose()?,
        };
        require_resource(&row.reifier, "reifier subject")?;
        if !matches!(row.statement, ProjectionTerm::Triple { .. }) {
            return Err(ProjectionError::integrity(
                "rdf:reifies object is not a triple term",
            ));
        }
        if let Some(graph) = &row.graph {
            require_graph_name(graph, "reifier graph")?;
            named_graphs.insert(graph.clone());
        }
        if !reifiers.insert(row) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate reifier binding",
            ));
        }
    }

    let mut annotations = BTreeSet::new();
    for quad in view.annotation_quads() {
        budget.consume("annotation")?;
        let row = ExactAnnotation {
            reifier: resolve_term(view, quad.s, config.limits(), &mut cache)?,
            predicate: resolve_term(view, quad.p, config.limits(), &mut cache)?,
            object: resolve_term(view, quad.o, config.limits(), &mut cache)?,
            graph: quad
                .g
                .map(|id| resolve_term(view, id, config.limits(), &mut cache))
                .transpose()?,
        };
        require_statement_positions(
            &row.reifier,
            &row.predicate,
            row.graph.as_ref(),
            "annotation",
        )?;
        if let Some(graph) = &row.graph {
            named_graphs.insert(graph.clone());
        }
        if !annotations.insert(row) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate annotation",
            ));
        }
    }
    Ok(ExactDataset {
        named_graphs,
        quads,
        reifiers,
        annotations,
    })
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
    package.insert(
        TERMS_PATH,
        write_terms(&catalog, &dataset.named_graphs, config)?,
    )?;
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
    by_term: BTreeMap<ProjectionTerm, String>,
    by_id: BTreeMap<String, ProjectionTerm>,
}

impl TermCatalog {
    fn build(dataset: &ExactDataset, config: &CsvwConfig) -> Result<Self, ProjectionError> {
        let mut terms = BTreeSet::new();
        for graph in &dataset.named_graphs {
            collect_term(graph, &mut terms);
        }
        for quad in &dataset.quads {
            collect_term(&quad.subject, &mut terms);
            collect_term(&quad.predicate, &mut terms);
            collect_term(&quad.object, &mut terms);
            if let Some(graph) = &quad.graph {
                collect_term(graph, &mut terms);
            }
        }
        for row in &dataset.reifiers {
            collect_term(&row.reifier, &mut terms);
            collect_term(&row.statement, &mut terms);
            if let Some(graph) = &row.graph {
                collect_term(graph, &mut terms);
            }
        }
        for row in &dataset.annotations {
            collect_term(&row.reifier, &mut terms);
            collect_term(&row.predicate, &mut terms);
            collect_term(&row.object, &mut terms);
            if let Some(graph) = &row.graph {
                collect_term(graph, &mut terms);
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
            term.validate(config.limits())?;
            let id = term_identifier(&term, config.limits())?;
            if let Some(existing) = by_id.insert(id.clone(), term.clone())
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

    fn id(&self, term: &ProjectionTerm) -> Result<&str, ProjectionError> {
        self.by_term.get(term).map(String::as_str).ok_or_else(|| {
            ProjectionError::integrity("exact CSVW row references an uncatalogued RDF term")
        })
    }

    fn len(&self) -> usize {
        self.by_term.len()
    }
}

fn collect_term(term: &ProjectionTerm, terms: &mut BTreeSet<ProjectionTerm>) {
    if !terms.insert(term.clone()) {
        return;
    }
    match term {
        ProjectionTerm::Literal { datatype, .. } => {
            terms.insert(ProjectionTerm::Iri {
                value: datatype.clone(),
            });
        }
        ProjectionTerm::Triple {
            subject,
            predicate,
            object,
        } => {
            collect_term(subject, terms);
            collect_term(predicate, terms);
            collect_term(object, terms);
        }
        ProjectionTerm::Iri { .. } | ProjectionTerm::Blank { .. } => {}
    }
}

fn write_terms(
    catalog: &TermCatalog,
    named_graphs: &BTreeSet<ProjectionTerm>,
    config: &CsvwConfig,
) -> Result<Vec<u8>, ProjectionError> {
    let mut rows = Vec::with_capacity(catalog.by_id.len());
    for (id, term) in &catalog.by_id {
        let named = named_graphs.contains(term).to_string();
        let row = match term {
            ProjectionTerm::Iri { value } => vec![
                id.clone(),
                "iri".to_owned(),
                value.clone(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                named,
            ],
            ProjectionTerm::Blank { label, scope } => vec![
                id.clone(),
                "blank".to_owned(),
                label.clone(),
                String::new(),
                String::new(),
                String::new(),
                scope.to_string(),
                String::new(),
                String::new(),
                String::new(),
                named,
            ],
            ProjectionTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => vec![
                id.clone(),
                "literal".to_owned(),
                lexical.clone(),
                catalog
                    .id(&ProjectionTerm::Iri {
                        value: datatype.clone(),
                    })?
                    .to_owned(),
                language.clone().unwrap_or_default(),
                direction.map_or_else(String::new, |value| match value {
                    ProjectionDirection::Ltr => "ltr".to_owned(),
                    ProjectionDirection::Rtl => "rtl".to_owned(),
                }),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                named,
            ],
            ProjectionTerm::Triple {
                subject,
                predicate,
                object,
            } => vec![
                id.clone(),
                "triple".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                catalog.id(subject)?.to_owned(),
                catalog.id(predicate)?.to_owned(),
                catalog.id(object)?.to_owned(),
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
        let id = row_identifier("CsvwQuad", quad, config.limits(), "exact CSVW quad")?;
        let row = vec![
            id.clone(),
            catalog.id(&quad.subject)?.to_owned(),
            catalog.id(&quad.predicate)?.to_owned(),
            catalog.id(&quad.object)?.to_owned(),
            quad.graph
                .as_ref()
                .map(|term| catalog.id(term).map(str::to_owned))
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
            reifier,
            config.limits(),
            "exact CSVW reifier",
        )?;
        let row = vec![
            id.clone(),
            catalog.id(&reifier.reifier)?.to_owned(),
            catalog.id(&reifier.statement)?.to_owned(),
            reifier
                .graph
                .as_ref()
                .map(|term| catalog.id(term).map(str::to_owned))
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
            annotation,
            config.limits(),
            "exact CSVW annotation",
        )?;
        let row = vec![
            id.clone(),
            catalog.id(&annotation.reifier)?.to_owned(),
            catalog.id(&annotation.predicate)?.to_owned(),
            catalog.id(&annotation.object)?.to_owned(),
            annotation
                .graph
                .as_ref()
                .map(|term| catalog.id(term).map(str::to_owned))
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

#[derive(Serialize)]
struct ExactMetadata<'a> {
    #[serde(rename = "@context")]
    context: (String, LocalContext<'a>),
    #[serde(rename = "@id")]
    id: &'a str,
    tables: Vec<TableMetadata>,
}

#[derive(Serialize)]
struct LocalContext<'a> {
    #[serde(rename = "@base")]
    base: &'a str,
}

#[derive(Serialize)]
struct TableMetadata {
    url: &'static str,
    #[serde(rename = "tableSchema")]
    table_schema: SchemaMetadata,
}

#[derive(Serialize)]
struct SchemaMetadata {
    columns: Vec<ColumnMetadata>,
    #[serde(rename = "primaryKey")]
    primary_key: &'static str,
    #[serde(rename = "foreignKeys", skip_serializing_if = "Vec::is_empty")]
    foreign_keys: Vec<ForeignKeyMetadata>,
}

#[derive(Serialize)]
struct ColumnMetadata {
    name: &'static str,
    titles: &'static str,
    datatype: &'static str,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    null: Option<&'static str>,
}

#[derive(Serialize)]
struct ForeignKeyMetadata {
    #[serde(rename = "columnReference")]
    column_reference: &'static str,
    reference: ForeignKeyReference,
}

#[derive(Serialize)]
struct ForeignKeyReference {
    resource: &'static str,
    #[serde(rename = "columnReference")]
    column_reference: &'static str,
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
}

fn decode_terms(
    records: &[StringRecord],
    config: &CsvwConfig,
) -> Result<(BTreeMap<String, ProjectionTerm>, BTreeSet<ProjectionTerm>), ProjectionError> {
    let mut raw = BTreeMap::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = row_path(TERMS_PATH, index);
        let id = field(record, 0, &path)?.to_owned();
        require_nonempty(&id, "term_id", &path)?;
        require_strict_order(previous.as_deref(), &id, &path)?;
        previous = Some(id.clone());
        let named_graph = match field(record, 10, &path)? {
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
            kind: field(record, 1, &path)?.to_owned(),
            value: field(record, 2, &path)?.to_owned(),
            datatype_id: field(record, 3, &path)?.to_owned(),
            language: field(record, 4, &path)?.to_owned(),
            direction: field(record, 5, &path)?.to_owned(),
            blank_scope: field(record, 6, &path)?.to_owned(),
            subject_id: field(record, 7, &path)?.to_owned(),
            predicate_id: field(record, 8, &path)?.to_owned(),
            object_id: field(record, 9, &path)?.to_owned(),
            named_graph,
        };
        if raw.insert(id, row).is_some() {
            return Err(ProjectionError::integrity("duplicate exact CSVW term id").at_path(path));
        }
    }
    let mut terms = BTreeMap::new();
    let mut active = BTreeSet::new();
    for id in raw.keys() {
        resolve_term_row(id, &raw, &mut terms, &mut active, config, 0)?;
    }
    let mut named_graphs = BTreeSet::new();
    for row in raw.values().filter(|row| row.named_graph) {
        let term = terms
            .get(&row.id)
            .ok_or_else(|| ProjectionError::integrity("resolved term disappeared"))?;
        require_graph_name(term, "named_graph term")?;
        named_graphs.insert(term.clone());
    }
    Ok((terms, named_graphs))
}

fn resolve_term_row(
    id: &str,
    raw: &BTreeMap<String, RawTermRow>,
    resolved: &mut BTreeMap<String, ProjectionTerm>,
    active: &mut BTreeSet<String>,
    config: &CsvwConfig,
    depth: usize,
) -> Result<ProjectionTerm, ProjectionError> {
    if let Some(term) = resolved.get(id) {
        return Ok(term.clone());
    }
    if depth > config.limits().max_term_depth() {
        return Err(ProjectionError::limit(
            "exact CSVW triple term exceeds configured recursion depth",
        )
        .at_path(TERMS_PATH));
    }
    if !active.insert(id.to_owned()) {
        return Err(
            ProjectionError::integrity("cycle in exact CSVW recursive term references")
                .at_path(TERMS_PATH),
        );
    }
    let row = raw.get(id).ok_or_else(|| {
        ProjectionError::integrity("exact CSVW foreign key references an unknown term")
            .at_path(TERMS_PATH)
    })?;
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
            ProjectionTerm::Iri {
                value: row.value.clone(),
            }
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
            ProjectionTerm::Blank {
                label: row.value.clone(),
                scope,
            }
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
            let datatype =
                resolve_term_row(&row.datatype_id, raw, resolved, active, config, depth + 1)?;
            let ProjectionTerm::Iri { value: datatype } = datatype else {
                return Err(ProjectionError::integrity(
                    "literal datatype_id does not identify an IRI term",
                )
                .at_path(TERMS_PATH));
            };
            let direction = match row.direction.as_str() {
                "" => None,
                "ltr" => Some(ProjectionDirection::Ltr),
                "rtl" => Some(ProjectionDirection::Rtl),
                _ => {
                    return Err(ProjectionError::syntax(
                        "literal direction must be empty, ltr, or rtl",
                    )
                    .at_path(TERMS_PATH));
                }
            };
            ProjectionTerm::Literal {
                lexical: row.value.clone(),
                datatype,
                language: (!row.language.is_empty()).then(|| row.language.clone()),
                direction,
            }
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
            ProjectionTerm::Triple {
                subject: Box::new(resolve_term_row(
                    &row.subject_id,
                    raw,
                    resolved,
                    active,
                    config,
                    depth + 1,
                )?),
                predicate: Box::new(resolve_term_row(
                    &row.predicate_id,
                    raw,
                    resolved,
                    active,
                    config,
                    depth + 1,
                )?),
                object: Box::new(resolve_term_row(
                    &row.object_id,
                    raw,
                    resolved,
                    active,
                    config,
                    depth + 1,
                )?),
            }
        }
        other => {
            return Err(
                ProjectionError::syntax(format!("unknown exact CSVW term kind {other:?}"))
                    .at_path(TERMS_PATH),
            );
        }
    };
    term.validate(config.limits())?;
    if term_identifier(&term, config.limits())? != row.id {
        return Err(ProjectionError::integrity(
            "exact CSVW term id does not match its canonical RDF value",
        )
        .at_path(TERMS_PATH));
    }
    active.remove(id);
    resolved.insert(id.to_owned(), term.clone());
    Ok(term)
}

fn decode_quads(
    records: &[StringRecord],
    terms: &BTreeMap<String, ProjectionTerm>,
    config: &CsvwConfig,
) -> Result<BTreeSet<ExactQuad>, ProjectionError> {
    let mut rows = BTreeSet::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = row_path(QUADS_PATH, index);
        let id = field(record, 0, &path)?;
        require_nonempty(id, "quad_id", &path)?;
        require_strict_order(previous.as_deref(), id, &path)?;
        previous = Some(id.to_owned());
        let row = ExactQuad {
            subject: referenced_term(terms, field(record, 1, &path)?, "subject_id", &path)?,
            predicate: referenced_term(terms, field(record, 2, &path)?, "predicate_id", &path)?,
            object: referenced_term(terms, field(record, 3, &path)?, "object_id", &path)?,
            graph: optional_term(terms, field(record, 4, &path)?, "graph_id", &path)?,
        };
        require_statement_positions(&row.subject, &row.predicate, row.graph.as_ref(), "quad")?;
        if row_identifier("CsvwQuad", &row, config.limits(), "exact CSVW quad")? != id {
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
    terms: &BTreeMap<String, ProjectionTerm>,
    config: &CsvwConfig,
) -> Result<BTreeSet<ExactReifier>, ProjectionError> {
    let mut rows = BTreeSet::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = row_path(REIFIERS_PATH, index);
        let id = field(record, 0, &path)?;
        require_nonempty(id, "reifier_row_id", &path)?;
        require_strict_order(previous.as_deref(), id, &path)?;
        previous = Some(id.to_owned());
        let row = ExactReifier {
            reifier: referenced_term(terms, field(record, 1, &path)?, "reifier_id", &path)?,
            statement: referenced_term(terms, field(record, 2, &path)?, "statement_id", &path)?,
            graph: optional_term(terms, field(record, 3, &path)?, "graph_id", &path)?,
        };
        require_resource(&row.reifier, "reifier subject")?;
        if !matches!(row.statement, ProjectionTerm::Triple { .. }) {
            return Err(ProjectionError::integrity(
                "exact CSVW reifier statement_id is not a triple term",
            )
            .at_path(path));
        }
        if let Some(graph) = &row.graph {
            require_graph_name(graph, "reifier graph")?;
        }
        if row_identifier("CsvwReifier", &row, config.limits(), "exact CSVW reifier")? != id {
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
    terms: &BTreeMap<String, ProjectionTerm>,
    config: &CsvwConfig,
) -> Result<BTreeSet<ExactAnnotation>, ProjectionError> {
    let mut rows = BTreeSet::new();
    let mut previous = None;
    for (index, record) in records.iter().enumerate() {
        let path = row_path(ANNOTATIONS_PATH, index);
        let id = field(record, 0, &path)?;
        require_nonempty(id, "annotation_id", &path)?;
        require_strict_order(previous.as_deref(), id, &path)?;
        previous = Some(id.to_owned());
        let row = ExactAnnotation {
            reifier: referenced_term(terms, field(record, 1, &path)?, "reifier_id", &path)?,
            predicate: referenced_term(terms, field(record, 2, &path)?, "predicate_id", &path)?,
            object: referenced_term(terms, field(record, 3, &path)?, "object_id", &path)?,
            graph: optional_term(terms, field(record, 4, &path)?, "graph_id", &path)?,
        };
        require_statement_positions(
            &row.reifier,
            &row.predicate,
            row.graph.as_ref(),
            "annotation",
        )?;
        if row_identifier(
            "CsvwAnnotation",
            &row,
            config.limits(),
            "exact CSVW annotation",
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

fn lift_exact(dataset: &ExactDataset) -> Result<Arc<RdfDataset>, ProjectionError> {
    let mut builder = RdfDatasetBuilder::new();
    for graph in &dataset.named_graphs {
        let graph = intern_term(&mut builder, graph)?;
        builder.declare_named_graph(graph);
    }
    for quad in &dataset.quads {
        let subject = intern_term(&mut builder, &quad.subject)?;
        let ProjectionTerm::Iri { value: predicate } = &quad.predicate else {
            return Err(ProjectionError::integrity(
                "exact CSVW quad predicate is not an IRI",
            ));
        };
        let predicate = builder.intern_iri(predicate);
        let object = intern_term(&mut builder, &quad.object)?;
        let graph = quad
            .graph
            .as_ref()
            .map(|term| intern_term(&mut builder, term))
            .transpose()?;
        builder.push_quad(subject, predicate, object, graph);
    }
    for row in &dataset.reifiers {
        let reifier = intern_term(&mut builder, &row.reifier)?;
        let statement = intern_term(&mut builder, &row.statement)?;
        let graph = row
            .graph
            .as_ref()
            .map(|term| intern_term(&mut builder, term))
            .transpose()?;
        builder.push_reifier_in_graph(reifier, statement, graph);
    }
    for row in &dataset.annotations {
        let reifier = intern_term(&mut builder, &row.reifier)?;
        let ProjectionTerm::Iri { value: predicate } = &row.predicate else {
            return Err(ProjectionError::integrity(
                "exact CSVW annotation predicate is not an IRI",
            ));
        };
        let predicate = builder.intern_iri(predicate);
        let object = intern_term(&mut builder, &row.object)?;
        let graph = row
            .graph
            .as_ref()
            .map(|term| intern_term(&mut builder, term))
            .transpose()?;
        builder.push_annotation_in_graph(reifier, predicate, object, graph);
    }
    builder.freeze().map_err(|error| {
        ProjectionError::integrity(format!(
            "exact CSVW rows reconstructed an invalid RDF dataset: {error}"
        ))
    })
}

fn intern_term(
    builder: &mut RdfDatasetBuilder,
    term: &ProjectionTerm,
) -> Result<TermId, ProjectionError> {
    Ok(match term {
        ProjectionTerm::Iri { value } => builder.intern_iri(value),
        ProjectionTerm::Blank { label, scope } => builder.intern_blank(label, BlankScope(*scope)),
        ProjectionTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => builder.intern_literal(RdfLiteral {
            lexical_form: lexical.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: direction.map(Into::into),
        }),
        ProjectionTerm::Triple {
            subject,
            predicate,
            object,
        } => {
            let subject = intern_term(builder, subject)?;
            let ProjectionTerm::Iri { value: predicate } = predicate.as_ref() else {
                return Err(ProjectionError::integrity(
                    "exact CSVW triple predicate is not an IRI",
                ));
            };
            let predicate = builder.intern_iri(predicate);
            let object = intern_term(builder, object)?;
            builder.intern_triple(subject, predicate, object)
        }
    })
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    limits: ProjectionLimits,
    cache: &mut BTreeMap<D::Id, ProjectionTerm>,
) -> Result<ProjectionTerm, ProjectionError> {
    if let Some(term) = cache.get(&id) {
        return Ok(term.clone());
    }
    let term = ProjectionTerm::from_view(view, id, limits)?;
    cache.insert(id, term.clone());
    Ok(term)
}

fn require_statement_positions(
    subject: &ProjectionTerm,
    predicate: &ProjectionTerm,
    graph: Option<&ProjectionTerm>,
    description: &str,
) -> Result<(), ProjectionError> {
    require_resource(subject, &format!("{description} subject"))?;
    if !matches!(predicate, ProjectionTerm::Iri { .. }) {
        return Err(ProjectionError::integrity(format!(
            "{description} predicate is not an IRI"
        )));
    }
    if let Some(graph) = graph {
        require_graph_name(graph, &format!("{description} graph"))?;
    }
    Ok(())
}

fn require_resource(term: &ProjectionTerm, description: &str) -> Result<(), ProjectionError> {
    if matches!(term, ProjectionTerm::Literal { .. }) {
        return Err(ProjectionError::integrity(format!(
            "{description} must not be a literal"
        )));
    }
    Ok(())
}

fn require_graph_name(term: &ProjectionTerm, description: &str) -> Result<(), ProjectionError> {
    if !matches!(
        term,
        ProjectionTerm::Iri { .. } | ProjectionTerm::Blank { .. }
    ) {
        return Err(ProjectionError::integrity(format!(
            "{description} must be an IRI or blank node"
        )));
    }
    Ok(())
}

fn term_identifier(
    term: &ProjectionTerm,
    limits: ProjectionLimits,
) -> Result<String, ProjectionError> {
    stable_identifier("CsvwTerm", &term.to_canonical_json(limits)?)
}

fn row_identifier<T: Serialize>(
    prefix: &str,
    row: &T,
    limits: ProjectionLimits,
    description: &str,
) -> Result<String, ProjectionError> {
    stable_identifier(prefix, &canonical_json_bounded(row, limits, description)?)
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
    let mut writer = WriterBuilder::new()
        .terminator(Terminator::Any(b'\n'))
        .from_writer(sink);
    write_csv_record(&mut writer, header, path)?;
    for row in rows {
        write_csv_record(&mut writer, row, path)?;
    }
    writer
        .flush()
        .map_err(|error| csv_write_error(error, path))?;
    let sink = writer.into_inner().map_err(|error| {
        let error = error.into_error();
        csv_write_error(error, path)
    })?;
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
    T: AsRef<[u8]>,
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
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .from_reader(bytes);
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
        budget.consume(path)?;
        output.push(record.map_err(|error| {
            ProjectionError::syntax(format!("read CSV row: {error}")).at_path(path)
        })?);
    }
    Ok(output)
}

struct RecordBudget {
    used: usize,
    maximum: usize,
}

impl RecordBudget {
    const fn new(maximum: usize) -> Self {
        Self { used: 0, maximum }
    }

    fn consume(&mut self, description: &str) -> Result<(), ProjectionError> {
        self.used = self
            .used
            .checked_add(1)
            .ok_or_else(|| ProjectionError::limit("CSVW record count overflow"))?;
        if self.used > self.maximum {
            return Err(ProjectionError::limit(format!(
                "{description} rows exceed the configured {}-record CSVW limit",
                self.maximum
            )));
        }
        Ok(())
    }
}

fn field<'a>(
    record: &'a StringRecord,
    index: usize,
    path: &str,
) -> Result<&'a str, ProjectionError> {
    record.get(index).ok_or_else(|| {
        ProjectionError::syntax(format!("CSV row is missing field {index}")).at_path(path)
    })
}

fn row_path(path: &str, zero_index: usize) -> String {
    format!("{path}:{}", zero_index + 2)
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
    terms: &BTreeMap<String, ProjectionTerm>,
    id: &str,
    field_name: &str,
    path: &str,
) -> Result<ProjectionTerm, ProjectionError> {
    require_nonempty(id, field_name, path)?;
    terms.get(id).cloned().ok_or_else(|| {
        ProjectionError::integrity(format!(
            "exact CSVW {field_name} references an unknown term id"
        ))
        .at_path(path)
    })
}

fn optional_term(
    terms: &BTreeMap<String, ProjectionTerm>,
    id: &str,
    field_name: &str,
    path: &str,
) -> Result<Option<ProjectionTerm>, ProjectionError> {
    if id.is_empty() {
        Ok(None)
    } else {
        referenced_term(terms, id, field_name, path).map(Some)
    }
}

fn required_artifact<'a>(
    package: &'a ProjectionPackage,
    path: &str,
) -> Result<&'a [u8], ProjectionError> {
    package
        .get(path)
        .ok_or_else(|| ProjectionError::package("required artifact is missing").at_path(path))
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
        PackBuilder, PackView, RdfDatasetBuilder, RdfTextDirection, datasets_isomorphic,
    };

    use super::*;
    use crate::ProjectionErrorKind;

    fn test_config(max_records: usize) -> CsvwConfig {
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
            ProjectionLimits::new(16, 4_000_000, 16_000_000, 20_000_000, 16).expect("limits"),
            max_records,
        )
        .expect("config")
    }

    fn fixture() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject");
        let predicate = builder.intern_iri("http://example.org/predicate");
        let graph = builder.intern_blank("graph", BlankScope(9));
        let literal = builder.intern_literal(RdfLiteral {
            lexical_form: "مرحبا,\nquoted \" value".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        builder.push_quad(subject, predicate, literal, Some(graph));
        let quoted = builder.intern_triple(subject, predicate, literal);
        let nested = builder.intern_triple(quoted, predicate, subject);
        builder.push_quad(subject, predicate, nested, None);
        let reifier = builder.intern_blank("reifier", BlankScope(7));
        builder.push_reifier_in_graph(reifier, quoted, Some(graph));
        let annotation_predicate = builder.intern_iri("http://example.org/confidence");
        let annotation_object = builder.intern_iri("http://example.org/high");
        builder.push_annotation_in_graph(
            reifier,
            annotation_predicate,
            annotation_object,
            Some(graph),
        );
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
        let config = test_config(10_000);
        let projected = project_csvw_exact(dataset.as_ref(), &config).expect("project");
        assert!(projected.loss_ledger.is_empty());
        assert_eq!(projected.package.len(), 5);
        let metadata: serde_json::Value = serde_json::from_slice(
            projected
                .package
                .get(METADATA_PATH)
                .expect("metadata artifact"),
        )
        .expect("metadata JSON");
        assert_eq!(metadata["tables"].as_array().expect("tables").len(), 4);

        let decoded = read_csvw_exact(&projected.package, &config).expect("read");
        assert!(decoded.loss_ledger.is_empty());
        assert!(datasets_isomorphic(&dataset, &decoded.dataset));
        let rewritten = project_csvw_exact(decoded.dataset.as_ref(), &config).expect("rewrite");
        assert!(same_artifacts(&projected.package, &rewritten.package));

        let pack = PackBuilder::build_bytes(&dataset).expect("pack");
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

        let error = read_csvw_exact(&projected.package, &test_config(1)).expect_err("record limit");
        assert_eq!(error.kind(), ProjectionErrorKind::ResourceLimit);
    }

    #[test]
    fn exact_csvw_preserves_explicit_empty_named_graphs() {
        let mut builder = RdfDatasetBuilder::new();
        let iri = builder.intern_iri("http://example.org/empty-iri-graph");
        let blank = builder.intern_blank("empty-blank-graph", BlankScope(12));
        builder.declare_named_graph(iri);
        builder.declare_named_graph(blank);
        let dataset = builder.freeze().expect("empty named graphs");
        let config = test_config(100);
        let projected = project_csvw_exact(dataset.as_ref(), &config).expect("project");
        let decoded = read_csvw_exact(&projected.package, &config).expect("read");
        assert!(datasets_isomorphic(&dataset, &decoded.dataset));
        assert_eq!(decoded.dataset.named_graphs().count(), 2);
    }
}
