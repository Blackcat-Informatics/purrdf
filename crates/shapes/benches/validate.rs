// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Baseline benchmark for the SHACL Core validator (acceleration, Phase 0).
//!
//! Sweeps the whole committed conformance corpus through
//! [`purrdf_shapes::engine::validate_graphs`] — parse data + shapes, resolve focus
//! nodes, run every constraint. This is the end-to-end number Phase 2 (regex /
//! subclass-closure / SPARQL caching) and Phase 4 (focus-node `rayon`) move.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf::loss::LossLedger;
use purrdf::{DatasetView, GraphMatch, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_shapes::engine::{
    __prepared_class_membership_view, PreparedValidator, parse_shapes, validate_graphs,
    validate_projected_dataset, validate_projected_dataset_with_focus_filter,
};
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::rules::entail_dataset;
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::{
    LinkmlConfig, LinkmlDocument, Namespaces, SchemaDatatypeMap, SchemaImportConfig, emit_linkml,
    import_json_schema, import_linkml,
};
use serde_json::{Map, Value, json};

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    static ALLOCATED_BYTES: Cell<u64> = const { Cell::new(0) };
}

static VALIDATION_COUNTING: AtomicBool = AtomicBool::new(false);
static VALIDATION_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static VALIDATION_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

// SAFETY: every operation forwards the original pointer/layout to the system
// allocator; thread-local counters are observational only.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + layout.size() as u64));
        if VALIDATION_COUNTING.load(Ordering::Relaxed) {
            VALIDATION_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            VALIDATION_ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        ALLOCATED_BYTES.with(|bytes| bytes.set(bytes.get() + new_size as u64));
        if VALIDATION_COUNTING.load(Ordering::Relaxed) {
            VALIDATION_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            VALIDATION_ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const IMPORT_CLASSES: usize = 128;
const IMPORT_PROPERTIES_PER_CLASS: usize = 8;
const LINKML: &str = "https://w3id.org/linkml/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const LINKML_EMIT_SIZES: &[usize] = &[32, 1_024, 60_000];
const CORE_FOCUS_SIZES: &[usize] = &[512, 1_024, 2_048, 3_000, 100_000, 1_000_000];
const SPARQL_FOCUS_SIZES: &[usize] = &[64, 512, 4_096];
const REALTIME_FOCUS_SIZES: &[usize] = &[1, 8, 64, 512, 4_096];
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const BENCH_EX: &str = "https://example.org/shacl-bench/";
const CLASS_DEPTH: usize = 40;
const MEMBERSHIP_DATASET_FOCUS_NODES: usize = 100_000;
const MEMBERSHIP_PATTERN_FOCUS_NODES: usize = 4_096;
const MEMBERSHIP_RULE_FOCUS_NODES: usize = 64;

struct ValidationFixture {
    dataset: Arc<RdfDataset>,
    shapes: Shapes,
    focus_nodes: usize,
}

#[derive(Debug, Clone, Copy)]
enum MembershipVariant {
    Identity,
    Direct,
    Deep,
}

impl MembershipVariant {
    const ALL: [Self; 3] = [Self::Identity, Self::Direct, Self::Deep];

    const fn label(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Direct => "direct",
            Self::Deep => "deep_40",
        }
    }

    const fn has_hierarchy(self) -> bool {
        matches!(self, Self::Direct | Self::Deep)
    }

    const fn visible_types_per_subject(self) -> usize {
        match self {
            Self::Identity | Self::Direct => 1,
            Self::Deep => CLASS_DEPTH,
        }
    }
}

struct MembershipFixture {
    dataset: Arc<RdfDataset>,
    shapes: Arc<Shapes>,
    focus_ids: Vec<TermId>,
    rdf_type: TermId,
    root_class: TermId,
    focus_nodes: usize,
    variant: MembershipVariant,
}

struct ValidationCountGuard;

impl ValidationCountGuard {
    fn start() -> Self {
        VALIDATION_ALLOCATIONS.store(0, Ordering::Relaxed);
        VALIDATION_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        VALIDATION_COUNTING.store(true, Ordering::Release);
        Self
    }
}

impl Drop for ValidationCountGuard {
    fn drop(&mut self) {
        VALIDATION_COUNTING.store(false, Ordering::Release);
    }
}

/// Read every `corpus/<case>/{data.nt, shapes.ttl}` pair, sorted by case name.
fn corpus_cases() -> Vec<(String, String, String)> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/corpus"));
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read corpus dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let data = fs::read_to_string(p.join("data.nt"))
                .unwrap_or_else(|e| panic!("{name}: data.nt: {e}"));
            let shapes = fs::read_to_string(p.join("shapes.ttl"))
                .unwrap_or_else(|e| panic!("{name}: shapes.ttl: {e}"));
            (name, data, shapes)
        })
        .collect()
}

fn bench_validate(c: &mut Criterion) {
    let cases = corpus_cases();

    let mut group = c.benchmark_group("shacl_validate");
    group.bench_function("corpus_all", |b| {
        b.iter(|| {
            for (name, data, shapes) in &cases {
                // Panic (don't silently skip) on a validation failure: a swallowed
                // error would run instantly and report a false speedup (gemini review).
                let report = validate_graphs(data, shapes)
                    .unwrap_or_else(|e| panic!("validation failed for {name}: {e:?}"));
                std::hint::black_box(report);
            }
        });
    });
    group.finish();
}

fn core_focus_fixture(focus_nodes: usize) -> ValidationFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let subclass = builder.intern_iri(RDFS_SUBCLASS_OF);
    let label_predicate = builder.intern_iri(&format!("{BENCH_EX}label"));
    let value_predicate = builder.intern_iri(&format!("{BENCH_EX}value"));
    let member_predicate = builder.intern_iri(&format!("{BENCH_EX}member"));

    let focus_classes: Vec<_> = (0..CLASS_DEPTH)
        .map(|index| builder.intern_iri(&format!("{BENCH_EX}FocusClass{index}")))
        .collect();
    let value_classes: Vec<_> = (0..CLASS_DEPTH)
        .map(|index| builder.intern_iri(&format!("{BENCH_EX}ValueClass{index}")))
        .collect();
    for index in 1..CLASS_DEPTH {
        builder.push_quad(
            focus_classes[index],
            subclass,
            focus_classes[index - 1],
            None,
        );
        builder.push_quad(
            value_classes[index],
            subclass,
            value_classes[index - 1],
            None,
        );
    }

    let member = builder.intern_iri(&format!("{BENCH_EX}shared-member"));
    builder.push_quad(member, rdf_type, value_classes[CLASS_DEPTH - 1], None);

    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}item{index}"));
        let label = builder.intern_literal(RdfLiteral::simple(format!("item-{index}")));
        let value = builder.intern_literal(RdfLiteral::typed(index.to_string(), XSD_INTEGER));
        builder.push_quad(focus, rdf_type, focus_classes[CLASS_DEPTH - 1], None);
        builder.push_quad(focus, label_predicate, label, None);
        builder.push_quad(focus, value_predicate, value, None);
        builder.push_quad(focus, member_predicate, member, None);
    }

    let dataset = builder.freeze().expect("Core focus fixture must freeze");
    let shapes = parse_shapes(&format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:WholeBundleShape a sh:NodeShape ;
    sh:targetClass ex:FocusClass0 ;
    sh:property [
        sh:path ex:label ;
        sh:minCount 1 ;
        sh:pattern "^item-[0-9]+$" ;
    ] ;
    sh:property [
        sh:path ex:value ;
        sh:datatype xsd:integer ;
    ] ;
    sh:property [
        sh:path ex:member ;
        sh:class ex:ValueClass0 ;
    ] .
"#
    ))
    .expect("Core focus shapes must parse");
    ValidationFixture {
        dataset,
        shapes,
        focus_nodes,
    }
}

fn sparql_focus_fixture(focus_nodes: usize) -> ValidationFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let person = builder.intern_iri(&format!("{BENCH_EX}Person"));
    let amount_predicate = builder.intern_iri(&format!("{BENCH_EX}amount"));
    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}sparql-item{index}"));
        let amount = builder.intern_literal(RdfLiteral::typed("1", XSD_INTEGER));
        builder.push_quad(focus, rdf_type, person, None);
        builder.push_quad(focus, amount_predicate, amount, None);
    }
    let dataset = builder.freeze().expect("SPARQL focus fixture must freeze");
    let shapes = parse_shapes(&format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:tripled a sh:SPARQLFunction ;
    sh:parameter [ sh:path ex:x ; sh:datatype xsd:integer ] ;
    sh:returnType xsd:integer ;
    sh:select "SELECT ((?x * 3) AS ?result) WHERE {{}}" .

ex:WholeBundleSparqlShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        sh:select "SELECT $this WHERE {{ $this <{BENCH_EX}amount> ?amount . FILTER(<{BENCH_EX}tripled>(?amount) > 100) }}" ;
    ] .
"#
    ))
    .expect("SPARQL focus shapes must parse");
    ValidationFixture {
        dataset,
        shapes,
        focus_nodes,
    }
}

fn membership_fixture(focus_nodes: usize, variant: MembershipVariant) -> MembershipFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let subclass = builder.intern_iri(RDFS_SUBCLASS_OF);
    let classes: Vec<_> = (0..CLASS_DEPTH)
        .map(|index| builder.intern_iri(&format!("{BENCH_EX}MembershipClass{index}")))
        .collect();
    if variant.has_hierarchy() {
        for index in 1..CLASS_DEPTH {
            builder.push_quad(classes[index], subclass, classes[index - 1], None);
        }
    }
    let asserted_class = match variant {
        MembershipVariant::Identity | MembershipVariant::Direct => classes[0],
        MembershipVariant::Deep => classes[CLASS_DEPTH - 1],
    };
    let retained_focus = focus_nodes.min(
        *REALTIME_FOCUS_SIZES
            .last()
            .expect("realtime sizes are non-empty"),
    );
    let mut focus_ids = Vec::with_capacity(retained_focus);
    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}membership-item{index}"));
        builder.push_quad(focus, rdf_type, asserted_class, None);
        if focus_ids.len() < retained_focus {
            focus_ids.push(focus);
        }
    }

    let dataset = builder
        .freeze()
        .expect("class-membership fixture must freeze");
    let shapes = parse_shapes(&format!(
        r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .

ex:MembershipShape a sh:NodeShape ;
    sh:targetClass ex:MembershipClass0 ;
    sh:nodeKind sh:IRI .
"
    ))
    .expect("class-membership shapes must parse");
    MembershipFixture {
        dataset,
        shapes: Arc::new(shapes),
        focus_ids,
        rdf_type,
        root_class: classes[0],
        focus_nodes,
        variant,
    }
}

fn prepare_membership_fixture(fixture: &MembershipFixture) -> PreparedValidator {
    PreparedValidator::from_projected_dataset(
        Arc::clone(&fixture.dataset),
        Arc::clone(&fixture.shapes),
    )
    .expect("class-membership benchmark preparation must succeed")
}

fn assert_membership_dimensions(fixture: &MembershipFixture, dimensions: [usize; 6]) {
    match fixture.variant {
        MembershipVariant::Identity | MembershipVariant::Direct => {
            assert_eq!(dimensions, [0; 6], "non-deriving fixtures retain no index");
        }
        MembershipVariant::Deep => {
            assert_eq!(
                dimensions,
                [
                    1,
                    fixture.focus_nodes,
                    CLASS_DEPTH - 1,
                    CLASS_DEPTH - 1,
                    CLASS_DEPTH - 1,
                    fixture.focus_nodes * (CLASS_DEPTH - 1),
                ],
                "the compact index must not store one row per virtual membership"
            );
        }
    }
}

fn validate_fixture(fixture: &ValidationFixture) {
    let report = validate_projected_dataset(Arc::clone(&fixture.dataset), &fixture.shapes)
        .expect("benchmark validation must not error");
    assert!(report.conforms, "benchmark fixture must conform");
    black_box(report);
}

fn print_validation_probe(label: &str, fixture: &ValidationFixture) {
    validate_fixture(fixture);
    let guard = ValidationCountGuard::start();
    let started = Instant::now();
    validate_fixture(fixture);
    let elapsed = started.elapsed();
    drop(guard);
    println!(
        "[shacl_focus_validation] case={label} focus_nodes={} quads={} terms={} threads={} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.focus_nodes,
        fixture.dataset.quad_count(),
        fixture.dataset.term_count(),
        rayon::current_num_threads(),
        elapsed.as_nanos(),
        VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
        VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
    );
}

fn bench_focus_core(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_focus_core");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    for &focus_nodes in CORE_FOCUS_SIZES {
        let fixture = core_focus_fixture(focus_nodes);
        let probe = Once::new();
        group.throughput(Throughput::Elements(focus_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(focus_nodes),
            &fixture,
            move |bencher, fixture| {
                probe.call_once(|| print_validation_probe("core", fixture));
                bencher.iter(|| validate_fixture(black_box(fixture)));
            },
        );
    }
    group.finish();
}

fn bench_focus_sparql(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_focus_sparql");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    for &focus_nodes in SPARQL_FOCUS_SIZES {
        let fixture = sparql_focus_fixture(focus_nodes);
        let probe = Once::new();
        group.throughput(Throughput::Elements(focus_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(focus_nodes),
            &fixture,
            move |bencher, fixture| {
                probe.call_once(|| print_validation_probe("sparql_function", fixture));
                bencher.iter(|| validate_fixture(black_box(fixture)));
            },
        );
    }
    group.finish();
}

fn validate_prepared_ids(prepared: &PreparedValidator, focus_ids: &[TermId]) {
    let report = prepared
        .validate_focus_node_ids(focus_ids)
        .expect("prepared benchmark validation must not error");
    assert!(report.conforms, "prepared benchmark fixture must conform");
    black_box(report);
}

fn print_realtime_probe(prepared: &PreparedValidator, focus_ids: &[TermId]) {
    validate_prepared_ids(prepared, focus_ids);
    let guard = ValidationCountGuard::start();
    let started = Instant::now();
    validate_prepared_ids(prepared, focus_ids);
    let elapsed = started.elapsed();
    drop(guard);
    println!(
        "[shacl_focus_realtime] requested_focus_nodes={} elapsed_ns={} allocations={} allocated_bytes={}",
        focus_ids.len(),
        elapsed.as_nanos(),
        VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
        VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
    );
}

fn bench_focus_realtime(c: &mut Criterion) {
    const DATASET_FOCUS_NODES: usize = 1_000_000;

    let fixture = core_focus_fixture(DATASET_FOCUS_NODES);
    let preparation_guard = ValidationCountGuard::start();
    let preparation_started = Instant::now();
    let prepared = PreparedValidator::from_projected_dataset(
        Arc::clone(&fixture.dataset),
        Arc::new(fixture.shapes.clone()),
    )
    .expect("realtime benchmark preparation must succeed");
    let preparation_elapsed = preparation_started.elapsed();
    drop(preparation_guard);
    println!(
        "[shacl_focus_prepare] dataset_focus_nodes={DATASET_FOCUS_NODES} elapsed_ns={} allocations={} allocated_bytes={}",
        preparation_elapsed.as_nanos(),
        VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
        VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
    );
    let all_focus_ids: Vec<_> = (0..*REALTIME_FOCUS_SIZES.last().expect("non-empty sizes"))
        .map(|index| {
            fixture
                .dataset
                .term_id_by_iri(&format!("{BENCH_EX}item{index}"))
                .expect("benchmark focus must be interned")
        })
        .collect();

    let mut group = c.benchmark_group("shacl_focus_realtime");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    let legacy_focus =
        purrdf_shapes::term::NamedNode::new_unchecked(format!("{BENCH_EX}item0")).into_term();
    group.throughput(Throughput::Elements(1));
    group.bench_function(BenchmarkId::new("compat_filter", 1), |bencher| {
        bencher.iter(|| {
            let report = validate_projected_dataset_with_focus_filter(
                Arc::clone(black_box(&fixture.dataset)),
                black_box(&fixture.shapes),
                |_, focus| focus == &legacy_focus,
            )
            .expect("compatibility filter benchmark must not error");
            assert!(report.conforms, "benchmark fixture must conform");
            black_box(report);
        });
    });

    for &focus_nodes in REALTIME_FOCUS_SIZES {
        let focus_ids = &all_focus_ids[..focus_nodes];
        let probe = Once::new();
        let prepared_ref = &prepared;
        group.throughput(Throughput::Elements(focus_nodes as u64));
        group.bench_with_input(
            BenchmarkId::new("prepared_ids", focus_nodes),
            focus_ids,
            move |bencher, focus_ids| {
                probe.call_once(|| print_realtime_probe(prepared_ref, focus_ids));
                bencher
                    .iter(|| validate_prepared_ids(black_box(prepared_ref), black_box(focus_ids)));
            },
        );
    }
    group.finish();
}

fn print_membership_preparation_probe(fixture: &MembershipFixture) -> PreparedValidator {
    let guard = ValidationCountGuard::start();
    let started = Instant::now();
    let prepared = prepare_membership_fixture(fixture);
    let elapsed = started.elapsed();
    drop(guard);
    let dimensions = prepared.__class_membership_dimensions();
    assert_membership_dimensions(fixture, dimensions);
    validate_prepared_ids(&prepared, &fixture.focus_ids[..1]);
    println!(
        "[shacl_subclass_prepare] variant={} dataset_focus_nodes={} quads={} terms={} class_depth={CLASS_DEPTH} indexed_typed_classes={} indexed_subject_ids={} ancestor_ids={} superclass_entries={} source_class_ids={} virtual_row_upper_bound={} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.variant.label(),
        fixture.focus_nodes,
        fixture.dataset.quad_count(),
        fixture.dataset.term_count(),
        dimensions[0],
        dimensions[1],
        dimensions[2],
        dimensions[3],
        dimensions[4],
        dimensions[5],
        elapsed.as_nanos(),
        VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
        VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
    );
    prepared
}

fn print_membership_realtime_probe(
    fixture: &MembershipFixture,
    prepared: &PreparedValidator,
    focus_ids: &[TermId],
) {
    validate_prepared_ids(prepared, focus_ids);
    let guard = ValidationCountGuard::start();
    let started = Instant::now();
    validate_prepared_ids(prepared, focus_ids);
    let elapsed = started.elapsed();
    drop(guard);
    println!(
        "[shacl_subclass_realtime] variant={} dataset_focus_nodes={} requested_focus_nodes={} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.variant.label(),
        fixture.focus_nodes,
        focus_ids.len(),
        elapsed.as_nanos(),
        VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
        VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
    );
}

fn bench_subclass_membership(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_subclass_membership");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for variant in MembershipVariant::ALL {
        let fixture = membership_fixture(MEMBERSHIP_DATASET_FOCUS_NODES, variant);
        let prepared = print_membership_preparation_probe(&fixture);

        group.throughput(Throughput::Elements(fixture.focus_nodes as u64));
        group.bench_with_input(
            BenchmarkId::new(format!("{}/prepare", variant.label()), fixture.focus_nodes),
            &fixture,
            |bencher, fixture| {
                bencher.iter(|| {
                    black_box(prepare_membership_fixture(black_box(fixture)));
                });
            },
        );

        for &focus_nodes in REALTIME_FOCUS_SIZES {
            let focus_ids = &fixture.focus_ids[..focus_nodes];
            let probe = Once::new();
            group.throughput(Throughput::Elements(focus_nodes as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{}/prepared_ids", variant.label()), focus_nodes),
                focus_ids,
                |bencher, focus_ids| {
                    probe.call_once(|| {
                        print_membership_realtime_probe(&fixture, &prepared, focus_ids);
                    });
                    bencher.iter(|| {
                        validate_prepared_ids(black_box(&prepared), black_box(focus_ids));
                    });
                },
            );
        }
    }
    group.finish();
}

fn membership_pattern_count<D>(
    view: &D,
    subject: Option<TermId>,
    predicate: Option<TermId>,
    object: Option<TermId>,
) -> usize
where
    D: DatasetView<Id = TermId> + Sync,
{
    view.quads_for_pattern(subject, predicate, object, GraphMatch::Default)
        .count()
}

fn print_membership_pattern_probe<D>(
    fixture: &MembershipFixture,
    view: &D,
    pattern: &str,
    subject: Option<TermId>,
    object: Option<TermId>,
    expected_rows: usize,
) where
    D: DatasetView<Id = TermId> + Sync,
{
    assert_eq!(
        membership_pattern_count(view, subject, Some(fixture.rdf_type), object),
        expected_rows
    );
    let guard = ValidationCountGuard::start();
    let started = Instant::now();
    let rows = membership_pattern_count(view, subject, Some(fixture.rdf_type), object);
    let elapsed = started.elapsed();
    drop(guard);
    assert_eq!(rows, expected_rows);
    println!(
        "[shacl_subclass_pattern] variant={} pattern={pattern} dataset_focus_nodes={} result_rows={rows} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.variant.label(),
        fixture.focus_nodes,
        elapsed.as_nanos(),
        VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
        VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
    );
}

fn bench_subclass_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_subclass_patterns");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for variant in MembershipVariant::ALL {
        let fixture = membership_fixture(MEMBERSHIP_PATTERN_FOCUS_NODES, variant);
        let prepared = prepare_membership_fixture(&fixture);
        assert_membership_dimensions(&fixture, prepared.__class_membership_dimensions());
        let view = __prepared_class_membership_view(Arc::clone(&fixture.dataset));
        let subject = fixture.focus_ids[0];
        let visible_types = variant.visible_types_per_subject();
        let patterns = [
            ("bound", Some(subject), Some(fixture.root_class), 1usize),
            (
                "object_bound",
                None,
                Some(fixture.root_class),
                fixture.focus_nodes,
            ),
            ("subject_bound", Some(subject), None, visible_types),
            (
                "fully_variable",
                None,
                None,
                fixture.focus_nodes * visible_types,
            ),
        ];

        for (pattern, pattern_subject, pattern_object, expected_rows) in patterns {
            print_membership_pattern_probe(
                &fixture,
                &view,
                pattern,
                pattern_subject,
                pattern_object,
                expected_rows,
            );
            group.throughput(Throughput::Elements(expected_rows as u64));
            group.bench_function(
                BenchmarkId::new(
                    format!("{}/{pattern}", variant.label()),
                    fixture.focus_nodes,
                ),
                |bencher| {
                    bencher.iter(|| {
                        let rows = membership_pattern_count(
                            black_box(&view),
                            black_box(pattern_subject),
                            black_box(Some(fixture.rdf_type)),
                            black_box(pattern_object),
                        );
                        assert_eq!(rows, expected_rows);
                        black_box(rows);
                    });
                },
            );
        }
    }
    group.finish();
}

fn membership_rule_shapes() -> Shapes {
    parse_shapes(&format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .

ex:MembershipRuleShape a sh:NodeShape ;
    sh:targetClass ex:MembershipClass0 ;
    sh:rule [
        a sh:SPARQLRule ;
        sh:construct "CONSTRUCT {{ $this ex:marked ex:yes }} WHERE {{ $this a <{BENCH_EX}MembershipClass0> }}" ;
    ] .
"#
    ))
    .expect("class-membership rule shapes must parse")
}

fn run_membership_rules(fixture: &MembershipFixture, shapes: &Shapes) {
    let output = entail_dataset(fixture.dataset.as_ref(), shapes)
        .expect("class-membership rule benchmark must entail");
    assert_eq!(
        output.quad_count(),
        fixture.dataset.quad_count() + fixture.focus_nodes,
        "every direct or derived root-class instance must receive one rule result"
    );
    black_box(output);
}

fn bench_subclass_rule_rounds(c: &mut Criterion) {
    let shapes = membership_rule_shapes();
    let mut group = c.benchmark_group("shacl_subclass_rule_rounds");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.throughput(Throughput::Elements(MEMBERSHIP_RULE_FOCUS_NODES as u64));

    for variant in MembershipVariant::ALL {
        let fixture = membership_fixture(MEMBERSHIP_RULE_FOCUS_NODES, variant);
        run_membership_rules(&fixture, &shapes);
        let guard = ValidationCountGuard::start();
        let started = Instant::now();
        run_membership_rules(&fixture, &shapes);
        let elapsed = started.elapsed();
        drop(guard);
        println!(
            "[shacl_subclass_rule_rounds] variant={} focus_nodes={} rounds=2 index_builds_per_round=1 elapsed_ns={} allocations={} allocated_bytes={}",
            variant.label(),
            fixture.focus_nodes,
            elapsed.as_nanos(),
            VALIDATION_ALLOCATIONS.load(Ordering::Relaxed),
            VALIDATION_ALLOCATED_BYTES.load(Ordering::Relaxed),
        );
        group.bench_with_input(
            BenchmarkId::from_parameter(variant.label()),
            &fixture,
            |bencher, fixture| {
                bencher.iter(|| run_membership_rules(black_box(fixture), black_box(&shapes)));
            },
        );
    }
    group.finish();
}

fn schema_import_config() -> SchemaImportConfig {
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/bench/".to_owned())],
    )
    .expect("benchmark namespace configuration");
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )
    .expect("benchmark datatype configuration");
    SchemaImportConfig::new(namespaces, datatypes)
}

fn schema_import_fixture() -> String {
    let mut definitions = Map::new();
    for class_idx in 0..IMPORT_CLASSES {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for property_idx in 0..IMPORT_PROPERTIES_PER_CLASS {
            let key = format!("ex:field{property_idx}");
            let schema = match property_idx {
                0 => json!({ "type": "string", "minLength": 1, "maxLength": 96 }),
                1 => json!({ "type": "integer", "minimum": 0, "maximum": 1_000_000 }),
                2 => json!({ "type": "number", "minimum": 0, "maximum": 1_000_000 }),
                3 => json!({ "type": "boolean" }),
                4 => json!({ "type": "string", "pattern": "^[A-Za-z0-9_-]+$" }),
                5 => json!({ "enum": ["open", "closed", "pending"] }),
                6 => json!({
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1,
                    "maxItems": 8,
                    "uniqueItems": true
                }),
                7 => json!({
                    "$ref": format!(
                        "#/$defs/Class{:03}",
                        (class_idx + IMPORT_CLASSES - 1) % IMPORT_CLASSES
                    )
                }),
                _ => unreachable!("fixed eight-property fixture"),
            };
            properties.insert(key.clone(), schema);
            if property_idx < IMPORT_PROPERTIES_PER_CLASS / 2 {
                required.push(Value::String(key));
            }
        }
        definitions.insert(
            format!("Class{class_idx:03}"),
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": properties,
                "required": required
            }),
        );
    }
    serde_json::to_string(&json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": definitions
    }))
    .expect("benchmark schema serializes")
}

fn allocation_snapshot() -> (u64, u64) {
    (ALLOCATIONS.with(Cell::get), ALLOCATED_BYTES.with(Cell::get))
}

fn linkml_import_fixture(config: &SchemaImportConfig) -> LinkmlDocument {
    let imported = import_json_schema(&schema_import_fixture(), config)
        .expect("benchmark source schema imports");
    let compiled = purrdf_shapes::json_schema::compile(&imported.shapes, config.namespaces());
    let linkml_config = LinkmlConfig::new(
        "https://example.org/bench/schema",
        "BenchSchema",
        "Representative LinkML import benchmark fixture.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/bench/".to_owned()),
            ("linkml".to_owned(), LINKML.to_owned()),
        ]),
    )
    .expect("benchmark LinkML configuration");
    emit_linkml(&compiled, &linkml_config)
        .expect("benchmark LinkML fixture emits")
        .document
}

fn bench_schema_import(c: &mut Criterion) {
    let schema = schema_import_fixture();
    let config = schema_import_config();

    let warm = import_json_schema(&schema, &config).expect("benchmark schema imports");
    assert_eq!(warm.shapes.node_shapes.len(), IMPORT_CLASSES);
    drop(warm);

    let before = allocation_snapshot();
    let observed = import_json_schema(&schema, &config).expect("allocation probe imports");
    let after = allocation_snapshot();
    assert_eq!(observed.shapes.node_shapes.len(), IMPORT_CLASSES);
    println!(
        "[shacl_schema_import] classes={IMPORT_CLASSES} properties={} allocations={} allocated_bytes={}",
        IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS,
        after.0 - before.0,
        after.1 - before.1
    );
    black_box(observed);

    let mut group = c.benchmark_group("shacl_schema_import");
    group.sample_size(20);
    group.throughput(Throughput::Elements(
        u64::try_from(IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS).expect("fixture size fits u64"),
    ));
    group.bench_function("json_schema_128_classes_1024_properties", |bencher| {
        bencher.iter(|| {
            let imported = import_json_schema(black_box(&schema), black_box(&config))
                .expect("benchmark schema imports");
            assert_eq!(imported.shapes.node_shapes.len(), IMPORT_CLASSES);
            black_box(imported);
        });
    });
    group.finish();
}

fn bench_linkml_import(c: &mut Criterion) {
    let config = schema_import_config();
    let document = linkml_import_fixture(&config);
    let expected_shapes = document
        .as_value()
        .get("classes")
        .and_then(Value::as_object)
        .map(Map::len)
        .expect("benchmark LinkML fixture has classes");

    let warm = import_linkml(&document, &config).expect("benchmark LinkML imports");
    assert_eq!(warm.shapes.node_shapes.len(), expected_shapes);
    drop(warm);

    let before = allocation_snapshot();
    let observed = import_linkml(&document, &config).expect("allocation probe imports");
    let after = allocation_snapshot();
    assert_eq!(observed.shapes.node_shapes.len(), expected_shapes);
    println!(
        "[shacl_linkml_import] source_classes={IMPORT_CLASSES} source_properties={} imported_shapes={expected_shapes} allocations={} allocated_bytes={}",
        IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS,
        after.0 - before.0,
        after.1 - before.1
    );
    black_box(observed);

    let mut group = c.benchmark_group("shacl_linkml_import");
    group.sample_size(20);
    group.throughput(Throughput::Elements(
        u64::try_from(IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS).expect("fixture size fits u64"),
    ));
    group.bench_function("from_128_class_1024_property_schema", |bencher| {
        bencher.iter(|| {
            let imported = import_linkml(black_box(&document), black_box(&config))
                .expect("benchmark LinkML imports");
            assert_eq!(imported.shapes.node_shapes.len(), expected_shapes);
            black_box(imported);
        });
    });
    group.finish();
}

#[derive(Debug, Clone, Copy)]
enum SlotEmissionMode {
    Safe,
    Rename,
    Collision,
}

impl SlotEmissionMode {
    const ALL: [Self; 3] = [Self::Safe, Self::Rename, Self::Collision];

    const fn label(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Rename => "rename",
            Self::Collision => "collision",
        }
    }
}

fn linkml_emit_fixture(
    slots: usize,
    mode: SlotEmissionMode,
) -> (CompiledSchema, LinkmlConfig, usize, usize) {
    assert!(slots > 0, "benchmark fixture requires slots");
    let mut properties = Map::new();
    let mut required = Vec::new();
    for index in 0..slots {
        let name = match mode {
            SlotEmissionMode::Safe => format!("ex:slot{index:05}"),
            SlotEmissionMode::Rename => format!("ex:slot/{index:05}"),
            SlotEmissionMode::Collision if index == 0 => "ex:collision_".to_owned(),
            SlotEmissionMode::Collision => {
                let scalar = u32::try_from(index).expect("benchmark index fits u32");
                let marker = char::from_u32(0x0f_0000 + scalar)
                    .expect("plane-15 private-use benchmark marker");
                format!("ex:collision{marker}")
            }
        };
        if index % 4 == 0 {
            required.push(Value::String(name.clone()));
        }
        properties.insert(
            name,
            json!({
                "type": "string",
                "pattern": "^[A-Z]"
            }),
        );
    }
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "Carrier": {
                "type": "object",
                "additionalProperties": false,
                "properties": properties,
                "required": required
            }
        }
    });
    let compiled = CompiledSchema {
        schema_json: format!(
            "{}\n",
            serde_json::to_string_pretty(&schema).expect("benchmark schema serializes")
        ),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    };
    let config = LinkmlConfig::new(
        "https://example.org/bench/linkml-emission",
        "LinkmlEmissionBench",
        "Matched safe, rename, and collision-heavy LinkML emission fixture.",
        "ex",
        BTreeMap::from([
            ("ex".to_owned(), "https://example.org/bench/".to_owned()),
            ("linkml".to_owned(), LINKML.to_owned()),
        ]),
    )
    .expect("benchmark LinkML configuration");
    let expected_renames = match mode {
        SlotEmissionMode::Safe => 0,
        SlotEmissionMode::Rename => slots,
        SlotEmissionMode::Collision => slots - 1,
    };
    let expected_collisions = match mode {
        SlotEmissionMode::Collision => slots - 1,
        SlotEmissionMode::Safe | SlotEmissionMode::Rename => 0,
    };
    (compiled, config, expected_renames, expected_collisions)
}

fn bench_linkml_slot_emission(c: &mut Criterion) {
    let mut group = c.benchmark_group("linkml_slot_emission");
    group.sample_size(10);
    for &slots in LINKML_EMIT_SIZES {
        for mode in SlotEmissionMode::ALL {
            let (compiled, config, expected_renames, expected_collisions) =
                linkml_emit_fixture(slots, mode);
            let assert_output = |output: &purrdf_shapes::LinkmlPackage| {
                assert_eq!(output.slot_renames.len(), expected_renames);
                assert_eq!(
                    output
                        .slot_renames
                        .iter()
                        .filter(|rename| rename.reasons.iter().any(|reason| {
                            *reason == purrdf_shapes::linkml::LinkmlSlotReason::Collision
                        }))
                        .count(),
                    expected_collisions
                );
            };

            let warm = emit_linkml(&compiled, &config).expect("benchmark fixture emits");
            assert_output(&warm);
            drop(warm);

            let before = allocation_snapshot();
            let observed = emit_linkml(&compiled, &config).expect("allocation probe emits");
            let after = allocation_snapshot();
            assert_output(&observed);
            println!(
                "[linkml_slot_emission] mode={} slots={slots} renames={expected_renames} collisions={expected_collisions} allocations={} allocated_bytes={}",
                mode.label(),
                after.0 - before.0,
                after.1 - before.1
            );
            black_box(observed);

            group.throughput(Throughput::Elements(
                u64::try_from(slots).expect("benchmark slot count fits u64"),
            ));
            group.bench_with_input(
                BenchmarkId::new(mode.label(), slots),
                &slots,
                |bencher, _| {
                    bencher.iter(|| {
                        let output = emit_linkml(black_box(&compiled), black_box(&config))
                            .expect("benchmark fixture emits");
                        assert_output(&output);
                        black_box(output);
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_validate,
    bench_focus_core,
    bench_focus_sparql,
    bench_focus_realtime,
    bench_subclass_membership,
    bench_subclass_patterns,
    bench_subclass_rule_rounds,
    bench_schema_import,
    bench_linkml_import,
    bench_linkml_slot_emission
);
criterion_main!(benches);
