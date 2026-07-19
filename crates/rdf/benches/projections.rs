// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]

//! Graph, tabular, and research-object carrier benchmarks over deterministic fixed datasets.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Arc;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_rdf::{
    CsvwConfig, CsvwContext, CsvwMode, CsvwVocabulary, LiftProfile, LpgConfig, LpgExecutionLimits,
    LpgScope, OboGraphsConfig, OboGraphsVocabulary, OboMetadataRoles, OboOwlRoles, OboRdfRoles,
    ProjectionConfig, ProjectionLimits, ProjectionProfile, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, ResearchObjectConfig, SkosClassRoles, SkosConfig, SkosDocumentationRoles,
    SkosGraphSelection, SkosLabelRoles, SkosRelationRoles, SkosSourceRoles, SkosTargetRoles,
    lift_archive, parse_dataset, project_archive, project_csvw_exact, project_lpg,
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

const EX: &str = "https://example.org/bench/";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const OBO: &str = "http://www.geneontology.org/formats/oboInOwl#";
const SKOS_SOURCE: &str = "https://example.org/bench/source-skos#";
const SKOS_TARGET: &str = "http://www.w3.org/2004/02/skos/core#";
const ROWS: usize = 200;
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

fn lpg_config() -> LpgConfig {
    LpgConfig::new(
        format!("{EX}type"),
        LpgScope::all(),
        limits(),
        LpgExecutionLimits::new(20_000, 20_000, 20_000, 20_000).expect("execution limits"),
    )
    .expect("LPG config")
}

fn csvw_config() -> CsvwConfig {
    CsvwConfig::new(
        format!("{EX}csvw-metadata"),
        CsvwContext::new(format!("{EX}csvw-context"), BTreeMap::default()).expect("context"),
        format!("{EX}csvw-group"),
        CsvwVocabulary::new("http://www.w3.org/ns/csvw#", RDF, RDFS, XSD).expect("CSVW vocabulary"),
        CsvwMode::Standard,
        limits(),
        20_000,
    )
    .expect("CSVW config")
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
    let obo_dataset = obo_dataset();
    let skos_dataset = skos_dataset();
    let lpg_config = lpg_config();
    let csvw_config = csvw_config();
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

    black_box(report_allocations("rdf_to_lpg", || {
        project_lpg(graph_dataset.as_ref(), &lpg_config).expect("LPG")
    }));
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
