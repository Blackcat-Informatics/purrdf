// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]

//! Graph, tabular, and research-object carrier benchmarks over deterministic fixed datasets.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_rdf::{
    CsvwConfig, CsvwContext, CsvwDatatype, CsvwMode, CsvwTermsCardinality, CsvwTermsColumn,
    CsvwTermsConfig, CsvwTermsGraphSelection, CsvwTermsIdentityColumn, CsvwTermsLimits,
    CsvwTermsSelector, CsvwTermsTable, CsvwTermsValueMode, CsvwVocabulary, LiftProfile, LpgConfig,
    LpgExecutionLimits, LpgIriSelection, LpgNamedGraphSelection, LpgPackageProjection, LpgProgress,
    LpgScope, LpgStreamProjection, OboGraphsConfig, OboGraphsVocabulary, OboMetadataRoles,
    OboOwlRoles, OboRdfRoles, ProjectionArtifactSink, ProjectionConfig, ProjectionError,
    ProjectionLimits, ProjectionProfile, ProjectionTerm, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    ResearchObjectConfig, SkosClassRoles, SkosConfig, SkosDocumentationRoles, SkosGraphSelection,
    SkosLabelRoles, SkosRelationRoles, SkosSourceRoles, SkosTargetRoles, lift_archive,
    parse_dataset, project_archive, project_csvw_exact, project_csvw_terms, project_lpg,
    project_lpg_csv, project_lpg_csv_to_sink, project_lpg_cypher, project_lpg_cypher_to_sink,
    project_lpg_graphml, project_lpg_graphml_to_sink, project_neo4j_csv, project_neo4j_csv_to_sink,
    project_obo_graphs, project_research_object, project_skos, read_csvw_exact, read_lpg_csv,
    read_lpg_cypher, read_lpg_graphml, read_neo4j_csv, write_lpg_csv, write_lpg_cypher,
    write_lpg_graphml, write_neo4j_csv,
};

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static ALLOCATED_BYTES: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: every operation forwards the original pointer/layout to the system
// allocator; thread-local counters are observational only.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + layout.size() as u64));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + new_size as u64));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Debug, Default)]
struct DiscardSink {
    artifacts: usize,
    bytes: usize,
    committed: bool,
}

impl ProjectionArtifactSink for DiscardSink {
    fn begin_package(&mut self) -> Result<(), ProjectionError> {
        *self = Self::default();
        Ok(())
    }

    fn begin_artifact(&mut self, _path: &str) -> Result<(), ProjectionError> {
        self.artifacts = self
            .artifacts
            .checked_add(1)
            .ok_or_else(|| ProjectionError::integrity("benchmark artifact count overflow"))?;
        Ok(())
    }

    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ProjectionError> {
        self.bytes = self
            .bytes
            .checked_add(chunk.len())
            .ok_or_else(|| ProjectionError::integrity("benchmark byte count overflow"))?;
        Ok(())
    }

    fn finish_artifact(&mut self) -> Result<(), ProjectionError> {
        Ok(())
    }

    fn commit_package(&mut self) -> Result<(), ProjectionError> {
        self.committed = true;
        Ok(())
    }

    fn abort_package(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy)]
enum LpgCarrier {
    GenericCsv,
    Neo4jCsv,
    OpenCypher,
    Graphml,
}

impl LpgCarrier {
    const ALL: [Self; 4] = [
        Self::GenericCsv,
        Self::Neo4jCsv,
        Self::OpenCypher,
        Self::Graphml,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::GenericCsv => "generic_csv",
            Self::Neo4jCsv => "neo4j_csv",
            Self::OpenCypher => "open_cypher",
            Self::Graphml => "graphml",
        }
    }

    fn materialize(self, dataset: &RdfDataset, config: &LpgConfig) -> LpgPackageProjection {
        match self {
            Self::GenericCsv => project_lpg_csv(dataset, config),
            Self::Neo4jCsv => project_neo4j_csv(dataset, config),
            Self::OpenCypher => project_lpg_cypher(dataset, config),
            Self::Graphml => project_lpg_graphml(dataset, config),
        }
        .expect("materialized LPG carrier")
    }

    fn stream(
        self,
        dataset: &RdfDataset,
        config: &LpgConfig,
    ) -> (LpgStreamProjection, DiscardSink) {
        let mut sink = DiscardSink::default();
        let mut observer = |_progress: &LpgProgress| Ok(());
        let outcome = match self {
            Self::GenericCsv => project_lpg_csv_to_sink(dataset, config, &mut sink, &mut observer),
            Self::Neo4jCsv => project_neo4j_csv_to_sink(dataset, config, &mut sink, &mut observer),
            Self::OpenCypher => {
                project_lpg_cypher_to_sink(dataset, config, &mut sink, &mut observer)
            }
            Self::Graphml => project_lpg_graphml_to_sink(dataset, config, &mut sink, &mut observer),
        }
        .expect("streamed LPG carrier");
        assert!(sink.committed);
        (outcome, sink)
    }
}

const EX: &str = "https://example.org/bench/";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const OBO: &str = "http://www.geneontology.org/formats/oboInOwl#";
const SKOS_SOURCE: &str = "https://example.org/bench/source-skos#";
const SKOS_TARGET: &str = "http://www.w3.org/2004/02/skos/core#";
const ROWS: usize = 200;
const LARGE_GRAPHS: usize = 20;
const LARGE_ROWS_PER_GRAPH: usize = 200;
const LARGE_QUADS: usize = LARGE_GRAPHS * LARGE_ROWS_PER_GRAPH * 3;
const RESEARCH_SOURCE: &[u8] =
    include_bytes!("../tests/fixtures/research-objects/carrier/shared.ttl");
const RESEARCH_CONFIGS: [(ProjectionProfile, LiftProfile, &[u8]); 5] = [
    (
        ProjectionProfile::Croissant11,
        LiftProfile::Croissant11,
        include_bytes!("../tests/fixtures/research-objects/carrier/croissant-1.1.json"),
    ),
    (
        ProjectionProfile::RoCrate13,
        LiftProfile::RoCrate13,
        include_bytes!("../tests/fixtures/research-objects/carrier/ro-crate-1.3.json"),
    ),
    (
        ProjectionProfile::DataCite46,
        LiftProfile::DataCite46,
        include_bytes!("../tests/fixtures/research-objects/carrier/datacite-4.6.json"),
    ),
    (
        ProjectionProfile::Dcat3,
        LiftProfile::Dcat3,
        include_bytes!("../tests/fixtures/research-objects/carrier/dcat-3.json"),
    ),
    (
        ProjectionProfile::FrictionlessDataPackage1,
        LiftProfile::FrictionlessDataPackage1,
        include_bytes!(
            "../tests/fixtures/research-objects/carrier/frictionless-data-package-1.json"
        ),
    ),
];

fn limits() -> ProjectionLimits {
    ProjectionLimits::new(64, 16_000_000, 64_000_000, 72_000_000, 16).expect("limits")
}

fn lpg_limits() -> ProjectionLimits {
    ProjectionLimits::new(64, 128_000_000, 512_000_000, 576_000_000, 16).expect("LPG limits")
}

fn push_iri(builder: &mut RdfDatasetBuilder, subject: &str, predicate: &str, object: &str) {
    let subject = builder.intern_iri(subject);
    let predicate = builder.intern_iri(predicate);
    let object = builder.intern_iri(object);
    builder.push_quad(subject, predicate, object, None);
}

fn push_literal(builder: &mut RdfDatasetBuilder, subject: &str, predicate: &str, value: &str) {
    let subject = builder.intern_iri(subject);
    let predicate = builder.intern_iri(predicate);
    let object = builder.intern_literal(RdfLiteral::simple(value));
    builder.push_quad(subject, predicate, object, None);
}

fn graph_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for index in 0..ROWS {
        let subject = format!("{EX}node-{index}");
        let object = format!("{EX}node-{}", (index * 17 + 3) % ROWS);
        push_iri(
            &mut builder,
            &subject,
            &format!("{EX}type"),
            &format!("{EX}Class-{}", index % 8),
        );
        push_iri(
            &mut builder,
            &subject,
            &format!("{EX}relation-{}", index % 11),
            &object,
        );
        push_literal(
            &mut builder,
            &subject,
            &format!("{EX}label"),
            &format!("Node {index}"),
        );
    }
    builder.freeze().expect("graph dataset")
}

fn large_multigraph_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(&format!("{EX}type"));
    let relation = builder.intern_iri(&format!("{EX}relation"));
    let label = builder.intern_iri(&format!("{EX}label"));
    for graph_index in 0..LARGE_GRAPHS {
        let graph = builder.intern_iri(&format!("{EX}graph-{graph_index}"));
        for row in 0..LARGE_ROWS_PER_GRAPH {
            let subject = builder.intern_iri(&format!("{EX}g{graph_index}-node-{row}"));
            let object = builder.intern_iri(&format!(
                "{EX}g{graph_index}-node-{}",
                (row * 17 + 3) % LARGE_ROWS_PER_GRAPH
            ));
            let class = builder.intern_iri(&format!("{EX}Class-{}", row % 8));
            let text = builder.intern_literal(RdfLiteral::simple(format!(
                "Graph {graph_index} node {row}"
            )));
            builder.push_quad(subject, rdf_type, class, Some(graph));
            builder.push_quad(subject, relation, object, Some(graph));
            builder.push_quad(subject, label, text, Some(graph));
        }
    }
    let dataset = builder.freeze().expect("large multi-graph dataset");
    assert_eq!(dataset.quad_count(), LARGE_QUADS);
    dataset
}

fn obo_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    push_iri(
        &mut builder,
        &format!("{EX}ontology"),
        &format!("{RDF}type"),
        &format!("{OWL}Ontology"),
    );
    for index in 0..ROWS {
        let class = format!("{EX}class-{index}");
        push_iri(
            &mut builder,
            &class,
            &format!("{RDF}type"),
            &format!("{OWL}Class"),
        );
        push_literal(
            &mut builder,
            &class,
            &format!("{RDFS}label"),
            &format!("Class {index}"),
        );
        if index > 0 {
            push_iri(
                &mut builder,
                &class,
                &format!("{RDFS}subClassOf"),
                &format!("{EX}class-{}", index - 1),
            );
        }
    }
    builder.freeze().expect("OBO dataset")
}

fn skos_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let scheme = format!("{EX}scheme");
    push_iri(
        &mut builder,
        &scheme,
        &format!("{RDF}type"),
        &format!("{SKOS_SOURCE}ConceptScheme"),
    );
    for index in 0..ROWS {
        let concept = format!("{EX}concept-{index}");
        push_iri(
            &mut builder,
            &concept,
            &format!("{RDF}type"),
            &format!("{SKOS_SOURCE}Concept"),
        );
        push_iri(
            &mut builder,
            &concept,
            &format!("{SKOS_SOURCE}inScheme"),
            &scheme,
        );
        push_literal(
            &mut builder,
            &concept,
            &format!("{SKOS_SOURCE}prefLabel"),
            &format!("Concept {index}"),
        );
        if index > 0 {
            push_iri(
                &mut builder,
                &concept,
                &format!("{SKOS_SOURCE}broader"),
                &format!("{EX}concept-{}", index - 1),
            );
        }
    }
    builder.freeze().expect("SKOS dataset")
}

fn lpg_config_with_scope(scope: LpgScope) -> LpgConfig {
    LpgConfig::new(
        format!("{EX}type"),
        scope,
        lpg_limits(),
        LpgExecutionLimits::new(100_000, 100_000, 100_000, 100_000).expect("execution limits"),
    )
    .expect("LPG config")
}

fn lpg_config() -> LpgConfig {
    lpg_config_with_scope(LpgScope::all())
}

fn scoped_lpg_config() -> LpgConfig {
    lpg_config_with_scope(LpgScope::select(
        false,
        LpgNamedGraphSelection::only(
            [ProjectionTerm::Iri {
                value: format!("{EX}graph-0"),
            }],
            std::iter::empty(),
        ),
        LpgIriSelection::all(),
        LpgIriSelection::all(),
        LpgIriSelection::all(),
    ))
}

fn csvw_config() -> CsvwConfig {
    CsvwConfig::new(
        format!("{EX}csvw-metadata"),
        CsvwContext::new(format!("{EX}csvw-context"), BTreeMap::default()).expect("context"),
        format!("{EX}csvw-group"),
        CsvwVocabulary::new("http://www.w3.org/ns/csvw#", RDF, RDFS, XSD).expect("CSVW vocabulary"),
        CsvwMode::Standard,
        limits(),
        100_000,
    )
    .expect("CSVW config")
}

fn csvw_datatype(base: impl Into<String>) -> CsvwDatatype {
    CsvwDatatype {
        id: None,
        base: base.into(),
        format: None,
        length: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        min_inclusive: None,
        max_inclusive: None,
        min_exclusive: None,
        max_exclusive: None,
    }
}

fn csvw_terms_config(graph_selection: CsvwTermsGraphSelection) -> CsvwTermsConfig {
    let iri = || csvw_datatype(format!("{XSD}anyURI"));
    let string = || csvw_datatype(format!("{XSD}string"));
    let column = |name: &str, predicate: String, mode: CsvwTermsValueMode| {
        CsvwTermsColumn::new(
            name,
            BTreeMap::new(),
            predicate,
            mode,
            CsvwTermsCardinality::One,
            true,
        )
        .expect("terms column")
    };
    let table = CsvwTermsTable::new(
        "resources",
        format!("{EX}catalog/resources.csv"),
        "resources.csv",
        CsvwTermsSelector::new(
            None,
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::from([EX.to_owned()]),
        )
        .expect("terms selector"),
        CsvwTermsIdentityColumn::new(
            "iri",
            BTreeMap::from([(String::new(), vec!["IRI".to_owned()])]),
            iri(),
        )
        .expect("identity column"),
        vec![
            column(
                "kind",
                format!("{EX}type"),
                CsvwTermsValueMode::iri(iri()).expect("IRI mode"),
            ),
            column(
                "relation",
                format!("{EX}relation"),
                CsvwTermsValueMode::iri(iri()).expect("IRI mode"),
            ),
            column(
                "label",
                format!("{EX}label"),
                CsvwTermsValueMode::literal(string(), None, None).expect("literal mode"),
            ),
        ],
    )
    .expect("terms table");
    CsvwTermsConfig::new(
        csvw_config(),
        "csvw-metadata.json",
        graph_selection,
        vec![table],
        CsvwTermsLimits::new(5_000, 20_000, 8).expect("terms limits"),
    )
    .expect("terms config")
}

fn scoped_csvw_terms_config() -> CsvwTermsConfig {
    csvw_terms_config(
        CsvwTermsGraphSelection::include(false, BTreeSet::from([format!("{EX}graph-0")]))
            .expect("one-graph scope"),
    )
}

fn obo_config() -> OboGraphsConfig {
    let rdf = OboRdfRoles::new(
        format!("{RDF}type"),
        format!("{RDF}reifies"),
        format!("{RDF}first"),
        format!("{RDF}rest"),
        format!("{RDF}nil"),
        format!("{XSD}string"),
        format!("{XSD}boolean"),
    )
    .expect("RDF roles");
    let owl = OboOwlRoles::new(
        format!("{RDFS}label"),
        format!("{RDFS}comment"),
        format!("{RDFS}subClassOf"),
        format!("{RDFS}subPropertyOf"),
        format!("{RDFS}domain"),
        format!("{RDFS}range"),
        format!("{OWL}Ontology"),
        format!("{OWL}Class"),
        format!("{OWL}NamedIndividual"),
        format!("{OWL}ObjectProperty"),
        format!("{OWL}AnnotationProperty"),
        format!("{OWL}DatatypeProperty"),
        format!("{OWL}equivalentClass"),
        format!("{OWL}intersectionOf"),
        format!("{OWL}Restriction"),
        format!("{OWL}onProperty"),
        format!("{OWL}someValuesFrom"),
        format!("{OWL}allValuesFrom"),
        format!("{OWL}propertyChainAxiom"),
        format!("{OWL}deprecated"),
    )
    .expect("OWL roles");
    let metadata = OboMetadataRoles::new(
        format!("{EX}definition"),
        format!("{OBO}hasExactSynonym"),
        format!("{OBO}hasBroadSynonym"),
        format!("{OBO}hasNarrowSynonym"),
        format!("{OBO}hasRelatedSynonym"),
        format!("{OBO}hasSynonymType"),
        format!("{OBO}hasDbXref"),
        format!("{OBO}inSubset"),
        format!("{OWL}versionInfo"),
    )
    .expect("OBO metadata roles");
    OboGraphsConfig::new(
        format!("{EX}ontology"),
        OboGraphsVocabulary::new(rdf, owl, metadata).expect("OBO vocabulary"),
        limits(),
        20_000,
    )
    .expect("OBO config")
}

fn skos_class_roles(prefix: &str) -> SkosClassRoles {
    SkosClassRoles::new(
        format!("{RDF}type"),
        format!("{prefix}Concept"),
        format!("{prefix}ConceptScheme"),
    )
    .expect("SKOS classes")
}

fn skos_label_roles(prefix: &str) -> SkosLabelRoles {
    SkosLabelRoles::new(
        format!("{prefix}prefLabel"),
        format!("{prefix}altLabel"),
        format!("{prefix}hiddenLabel"),
        format!("{prefix}notation"),
    )
    .expect("SKOS labels")
}

fn skos_documentation_roles(prefix: &str) -> SkosDocumentationRoles {
    SkosDocumentationRoles::new(
        format!("{prefix}note"),
        format!("{prefix}changeNote"),
        format!("{prefix}definition"),
        format!("{prefix}editorialNote"),
        format!("{prefix}example"),
        format!("{prefix}historyNote"),
        format!("{prefix}scopeNote"),
    )
    .expect("SKOS documentation")
}

fn skos_relation_roles(prefix: &str) -> SkosRelationRoles {
    SkosRelationRoles::new(
        format!("{prefix}broader"),
        format!("{prefix}narrower"),
        format!("{prefix}related"),
        format!("{prefix}closeMatch"),
        format!("{prefix}exactMatch"),
        format!("{prefix}broadMatch"),
        format!("{prefix}narrowMatch"),
        format!("{prefix}relatedMatch"),
        format!("{prefix}inScheme"),
        format!("{prefix}hasTopConcept"),
        format!("{prefix}topConceptOf"),
    )
    .expect("SKOS relations")
}

fn skos_config() -> SkosConfig {
    let source = SkosSourceRoles::new(
        skos_class_roles(SKOS_SOURCE),
        skos_label_roles(SKOS_SOURCE),
        skos_documentation_roles(SKOS_SOURCE),
        skos_relation_roles(SKOS_SOURCE),
    )
    .expect("source roles");
    let target = SkosTargetRoles::new(
        skos_class_roles(SKOS_TARGET),
        skos_label_roles(SKOS_TARGET),
        skos_documentation_roles(SKOS_TARGET),
        skos_relation_roles(SKOS_TARGET),
    )
    .expect("target roles");
    SkosConfig::new(
        source,
        target,
        format!("{EX}scheme"),
        SkosGraphSelection::DefaultGraph,
        limits(),
        20_000,
    )
    .expect("SKOS config")
}

fn research_common(config: &ProjectionConfig) -> &ResearchObjectConfig {
    match config {
        ProjectionConfig::Croissant11(config) => config.common(),
        ProjectionConfig::RoCrate13(config) => config.common(),
        ProjectionConfig::DataCite46(config) => config.common(),
        ProjectionConfig::Dcat3(config) => config.common(),
        ProjectionConfig::FrictionlessDataPackage1(config) => config.common(),
        _ => panic!("research-object benchmark received a non-research profile"),
    }
}

fn allocation_snapshot() -> (u64, u64) {
    (ALLOCATIONS.with(Cell::get), ALLOCATED_BYTES.with(Cell::get))
}

fn report_allocations<T>(label: &str, operation: impl FnOnce() -> T) -> T {
    let before = allocation_snapshot();
    let result = operation();
    let after = allocation_snapshot();
    println!(
        "[projections] {label:24} allocations={:>7} allocated_bytes={:>10}",
        after.0 - before.0,
        after.1 - before.1
    );
    result
}

fn benchmark(c: &mut Criterion) {
    let graph_dataset = graph_dataset();
    let large_graph_dataset = large_multigraph_dataset();
    let obo_dataset = obo_dataset();
    let skos_dataset = skos_dataset();
    let lpg_config = lpg_config();
    let scoped_lpg_config = scoped_lpg_config();
    let csvw_config = csvw_config();
    let csvw_terms_all_config = csvw_terms_config(CsvwTermsGraphSelection::All);
    let csvw_terms_scoped_config = scoped_csvw_terms_config();
    let obo_config = obo_config();
    let skos_config = skos_config();
    let research_dataset =
        parse_dataset(RESEARCH_SOURCE, "text/turtle", None).expect("research-object dataset");
    let research_configs: Vec<_> = RESEARCH_CONFIGS
        .iter()
        .map(|&(profile, lift, bytes)| {
            (
                profile,
                lift,
                ProjectionConfig::from_json(bytes).expect("research-object config"),
            )
        })
        .collect();
    let lpg = project_lpg(graph_dataset.as_ref(), &lpg_config).expect("LPG projection");
    let generic = write_lpg_csv(&lpg.graph, &lpg_config).expect("generic CSV");
    let neo4j = write_neo4j_csv(&lpg.graph, &lpg_config).expect("Neo4j CSV");
    let cypher = write_lpg_cypher(&lpg.graph, &lpg_config).expect("openCypher");
    let graphml = write_lpg_graphml(&lpg.graph, &lpg_config).expect("GraphML");
    let csvw = project_csvw_exact(graph_dataset.as_ref(), &csvw_config).expect("CSVW");
    let research_archives: Vec<_> = research_configs
        .iter()
        .map(|(profile, _, config)| {
            project_archive(research_dataset.as_ref(), *profile, config)
                .expect("research-object projection")
        })
        .collect();

    // Warm all paths before taking one-shot allocation deltas.
    let _ = read_lpg_csv(&generic, &lpg_config).expect("generic read");
    let _ = read_neo4j_csv(&neo4j, &lpg_config).expect("Neo4j read");
    let _ = read_lpg_cypher(&cypher, &lpg_config).expect("Cypher read");
    let _ = read_lpg_graphml(&graphml, &lpg_config).expect("GraphML read");
    let _ = read_csvw_exact(&csvw.package, &csvw_config).expect("CSVW read");
    let _ = project_obo_graphs(obo_dataset.as_ref(), &obo_config).expect("OBO Graphs");
    let _ = project_skos(skos_dataset.as_ref(), &skos_config).expect("SKOS");
    let _ = project_research_object(
        research_dataset.as_ref(),
        research_configs[0].0.as_str(),
        research_common(&research_configs[0].2),
    )
    .expect("research-object model");
    for ((_, lift, config), archive) in research_configs.iter().zip(&research_archives) {
        let _ = lift_archive(&archive.archive, *lift, config).expect("research-object lift");
    }
    let _ = project_lpg(large_graph_dataset.as_ref(), &lpg_config).expect("large all-scope LPG");
    let _ = project_lpg(large_graph_dataset.as_ref(), &scoped_lpg_config)
        .expect("large selective-scope LPG");
    let large_csvw_exact =
        project_csvw_exact(large_graph_dataset.as_ref(), &csvw_config).expect("large exact CSVW");
    let large_csvw_terms_all =
        project_csvw_terms(large_graph_dataset.as_ref(), &csvw_terms_all_config)
            .expect("large all-graph terms CSVW");
    let large_csvw_terms_scoped =
        project_csvw_terms(large_graph_dataset.as_ref(), &csvw_terms_scoped_config)
            .expect("large scoped terms CSVW");
    assert_eq!(
        large_csvw_terms_all.report.rows,
        LARGE_GRAPHS * LARGE_ROWS_PER_GRAPH
    );
    assert_eq!(large_csvw_terms_scoped.report.rows, LARGE_ROWS_PER_GRAPH);
    println!(
        "[projections] csvw_large_bodies exact={} terms_all={} terms_one_graph={}",
        large_csvw_exact.package.total_bytes(),
        large_csvw_terms_all.package.total_bytes(),
        large_csvw_terms_scoped.package.total_bytes()
    );
    for carrier in LpgCarrier::ALL {
        let _ = carrier.materialize(large_graph_dataset.as_ref(), &lpg_config);
        let _ = carrier.stream(large_graph_dataset.as_ref(), &lpg_config);
    }

    black_box(report_allocations("rdf_to_lpg", || {
        project_lpg(graph_dataset.as_ref(), &lpg_config).expect("LPG")
    }));
    black_box(report_allocations("lpg_large_all_scope", || {
        project_lpg(large_graph_dataset.as_ref(), &lpg_config).expect("large all-scope LPG")
    }));
    black_box(report_allocations("lpg_large_one_graph", || {
        project_lpg(large_graph_dataset.as_ref(), &scoped_lpg_config)
            .expect("large selective-scope LPG")
    }));
    for carrier in LpgCarrier::ALL {
        black_box(report_allocations(
            &format!("{}_package", carrier.name()),
            || carrier.materialize(large_graph_dataset.as_ref(), &lpg_config),
        ));
        black_box(report_allocations(
            &format!("{}_sink", carrier.name()),
            || carrier.stream(large_graph_dataset.as_ref(), &lpg_config),
        ));
    }
    black_box(report_allocations("lpg_generic_write", || {
        write_lpg_csv(&lpg.graph, &lpg_config).expect("generic write")
    }));
    black_box(report_allocations("lpg_generic_read", || {
        read_lpg_csv(&generic, &lpg_config).expect("generic read")
    }));
    black_box(report_allocations("csvw_exact_write", || {
        project_csvw_exact(graph_dataset.as_ref(), &csvw_config).expect("CSVW write")
    }));
    black_box(report_allocations("csvw_exact_read", || {
        read_csvw_exact(&csvw.package, &csvw_config).expect("CSVW read")
    }));
    black_box(report_allocations("csvw_large_exact", || {
        project_csvw_exact(large_graph_dataset.as_ref(), &csvw_config).expect("large exact CSVW")
    }));
    black_box(report_allocations("csvw_terms_all", || {
        project_csvw_terms(large_graph_dataset.as_ref(), &csvw_terms_all_config)
            .expect("large all-graph terms CSVW")
    }));
    black_box(report_allocations("csvw_terms_one_graph", || {
        project_csvw_terms(large_graph_dataset.as_ref(), &csvw_terms_scoped_config)
            .expect("large scoped terms CSVW")
    }));
    black_box(report_allocations("obo_graphs_write", || {
        project_obo_graphs(obo_dataset.as_ref(), &obo_config).expect("OBO write")
    }));
    black_box(report_allocations("skos_write", || {
        project_skos(skos_dataset.as_ref(), &skos_config).expect("SKOS write")
    }));
    black_box(report_allocations("research_common_model", || {
        project_research_object(
            research_dataset.as_ref(),
            research_configs[0].0.as_str(),
            research_common(&research_configs[0].2),
        )
        .expect("research-object model")
    }));
    for ((profile, lift, config), archive) in research_configs.iter().zip(&research_archives) {
        black_box(report_allocations(
            &format!("{}_write", profile.as_str()),
            || {
                project_archive(research_dataset.as_ref(), *profile, config)
                    .expect("research-object write")
            },
        ));
        black_box(report_allocations(
            &format!("{}_read", profile.as_str()),
            || lift_archive(&archive.archive, *lift, config).expect("research-object read"),
        ));
    }

    {
        let mut mapping = c.benchmark_group("projection_mapping");
        mapping.throughput(Throughput::Elements(graph_dataset.quad_count() as u64));
        mapping.bench_function("rdf_to_lpg_600_quads", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_lpg(black_box(graph_dataset.as_ref()), black_box(&lpg_config))
                        .expect("LPG"),
                );
            });
        });
        mapping.finish();
    }

    {
        let mut scope = c.benchmark_group("lpg_scope_mapping");
        scope.throughput(Throughput::Elements(LARGE_QUADS as u64));
        scope.bench_function("all_20_graphs_12000_quads", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_lpg(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&lpg_config),
                    )
                    .expect("large all-scope LPG"),
                );
            });
        });
        scope.bench_function("one_graph_12000_scanned_600_selected", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_lpg(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&scoped_lpg_config),
                    )
                    .expect("large selective-scope LPG"),
                );
            });
        });
        scope.finish();
    }

    {
        let mut carriers = c.benchmark_group("lpg_package_vs_sink");
        carriers.throughput(Throughput::Elements(LARGE_QUADS as u64));
        for carrier in LpgCarrier::ALL {
            carriers.bench_function(format!("{}_package", carrier.name()), |bencher| {
                bencher.iter(|| {
                    black_box(carrier.materialize(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&lpg_config),
                    ));
                });
            });
            carriers.bench_function(format!("{}_sink", carrier.name()), |bencher| {
                bencher.iter(|| {
                    black_box(carrier.stream(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&lpg_config),
                    ));
                });
            });
        }
        carriers.finish();
    }

    {
        let mut carriers = c.benchmark_group("lpg_carriers");
        carriers.throughput(Throughput::Elements(
            (lpg.graph.nodes.len() + lpg.graph.edges.len()) as u64,
        ));
        carriers.bench_function("generic_csv_write", |bencher| {
            bencher.iter(|| {
                black_box(write_lpg_csv(black_box(&lpg.graph), &lpg_config).expect("write"))
            });
        });
        carriers.bench_function("generic_csv_read", |bencher| {
            bencher
                .iter(|| black_box(read_lpg_csv(black_box(&generic), &lpg_config).expect("read")));
        });
        carriers.bench_function("neo4j_csv_write", |bencher| {
            bencher.iter(|| {
                black_box(write_neo4j_csv(black_box(&lpg.graph), &lpg_config).expect("write"))
            });
        });
        carriers.bench_function("neo4j_csv_read", |bencher| {
            bencher
                .iter(|| black_box(read_neo4j_csv(black_box(&neo4j), &lpg_config).expect("read")));
        });
        carriers.bench_function("open_cypher_write", |bencher| {
            bencher.iter(|| {
                black_box(write_lpg_cypher(black_box(&lpg.graph), &lpg_config).expect("write"))
            });
        });
        carriers.bench_function("open_cypher_read", |bencher| {
            bencher.iter(|| {
                black_box(read_lpg_cypher(black_box(&cypher), &lpg_config).expect("read"))
            });
        });
        carriers.bench_function("graphml_write", |bencher| {
            bencher.iter(|| {
                black_box(write_lpg_graphml(black_box(&lpg.graph), &lpg_config).expect("write"))
            });
        });
        carriers.bench_function("graphml_read", |bencher| {
            bencher.iter(|| {
                black_box(read_lpg_graphml(black_box(&graphml), &lpg_config).expect("read"))
            });
        });
        carriers.finish();
    }

    {
        let mut exact = c.benchmark_group("csvw_exact");
        exact.throughput(Throughput::Elements(graph_dataset.quad_count() as u64));
        exact.bench_function("write_600_quads", |bencher| {
            bencher.iter(|| {
                black_box(project_csvw_exact(graph_dataset.as_ref(), &csvw_config).expect("write"));
            });
        });
        exact.bench_function("read_600_quads", |bencher| {
            bencher.iter(|| {
                black_box(read_csvw_exact(black_box(&csvw.package), &csvw_config).expect("read"));
            });
        });
        exact.finish();
    }

    {
        let mut scope = c.benchmark_group("csvw_terms_scope");
        scope.throughput(Throughput::Elements(LARGE_QUADS as u64));
        scope.bench_function("exact_20_graphs_12000_quads", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_csvw_exact(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&csvw_config),
                    )
                    .expect("large exact CSVW"),
                );
            });
        });
        scope.bench_function("terms_all_12000_scanned_4000_rows", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_csvw_terms(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&csvw_terms_all_config),
                    )
                    .expect("large all-graph terms CSVW"),
                );
            });
        });
        scope.bench_function("terms_one_graph_12000_scanned_200_rows", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_csvw_terms(
                        black_box(large_graph_dataset.as_ref()),
                        black_box(&csvw_terms_scoped_config),
                    )
                    .expect("large scoped terms CSVW"),
                );
            });
        });
        scope.finish();
    }

    {
        let mut views = c.benchmark_group("projection_views");
        views.throughput(Throughput::Elements(obo_dataset.quad_count() as u64));
        views.bench_function("obo_graphs_600_quads", |bencher| {
            bencher.iter(|| {
                black_box(project_obo_graphs(obo_dataset.as_ref(), &obo_config).expect("OBO"));
            });
        });
        views.throughput(Throughput::Elements(skos_dataset.quad_count() as u64));
        views.bench_function("skos_800_quads", |bencher| {
            bencher.iter(|| {
                black_box(project_skos(skos_dataset.as_ref(), &skos_config).expect("SKOS"));
            });
        });
        views.finish();
    }

    {
        let mut research = c.benchmark_group("research_object_carriers");
        research.throughput(Throughput::Elements(research_dataset.quad_count() as u64));
        research.bench_function("common_model", |bencher| {
            bencher.iter(|| {
                black_box(
                    project_research_object(
                        black_box(research_dataset.as_ref()),
                        black_box(research_configs[0].0.as_str()),
                        black_box(research_common(&research_configs[0].2)),
                    )
                    .expect("research-object model"),
                );
            });
        });
        for ((profile, lift, config), archive) in research_configs.iter().zip(&research_archives) {
            research.bench_function(format!("{}_write", profile.as_str()), |bencher| {
                bencher.iter(|| {
                    black_box(
                        project_archive(
                            black_box(research_dataset.as_ref()),
                            *profile,
                            black_box(config),
                        )
                        .expect("research-object write"),
                    );
                });
            });
            research.bench_function(format!("{}_read", profile.as_str()), |bencher| {
                bencher.iter(|| {
                    black_box(
                        lift_archive(black_box(&archive.archive), *lift, black_box(config))
                            .expect("research-object read"),
                    );
                });
            });
        }
        research.finish();
    }
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
