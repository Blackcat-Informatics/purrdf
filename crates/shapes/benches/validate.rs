// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Baseline benchmark for the SHACL Core validator (acceleration, Phase 0).
//!
//! Sweeps the whole committed conformance corpus through
//! [`purrdf_shapes::engine::validate_graphs`] — parse data + shapes, resolve focus
//! nodes, run every constraint. This is the end-to-end number Phase 2 (regex /
//! subclass-closure / SPARQL caching) and Phase 4 (focus-node `rayon`) move.
//!
//! `shacl_focus_closed` is the per-value-node constraint path: a `sh:closed` shape
//! with several property shapes over many focus nodes, whose values are checked
//! by `sh:minLength`/`sh:maxLength`/`sh:nodeKind`/`sh:languageIn` — the arms that
//! now read the interned value node's borrowed surface (length, kind, language)
//! and the closed-permitted set's borrowed keys instead of materializing terms.
//!
//! `shacl_focus_unique_values_for` is the one cross-focus Core component,
//! `sh:uniqueValuesFor`, over a growing conforming target set: each focus node's
//! verdict depends on every other target node, and the grouping of the target set
//! by value tuple is built once per validation, so the per-focus-node cost is
//! expected to stay flat across the sweep rather than grow with it.
//!
//! `shacl_focus_target_where` is `sh:targetWhere` over a graph holding as many
//! unrelated nodes as targets, in its two resolutions: `narrowed`, where the where
//! shape's `sh:class` bounds the candidates to the class's instances, and
//! `full_scan`, the same condition behind a one-member `sh:or` the narrowing does
//! not look inside, which checks every node of the graph against the shape.
//!
//! `shacl_focus_computed_values` is a property shape's value nodes in the three
//! forms SHACL 1.2 Core gives them, over one conforming focus population:
//! `asserted`, reached by the path alone; `values`, the output of a `sh:values`
//! node expression evaluated at each focus node; and `default`, a
//! `sh:defaultValue` constant added where the path and `sh:values` produce
//! nothing. The `asserted` row is the control every shape without either term
//! pays, and the two computed rows show what an expression evaluation per focus
//! node costs beside it.
//!
//! `shacl_change_path_contrast` is the conforming-versus-violating pair over ONE
//! dataset and ONE binding: the change path materializes a focus node only where a
//! result is built, so a conforming request should cost a constant whatever the
//! focus count while a violating one pays per violation. Reporting only the
//! conforming half would be satisfied by a validator that had stopped validating,
//! so the violating half asserts its result count.
//!
//! Its conforming probe reads flat up to 512 focus nodes and then steps at 4,096.
//! That step is not a growth term and it is not this crate: 4,096 is above
//! `crate::parallel::PARALLEL_MIN_FOCUS_NODES`, and the shape carries
//! `sh:pattern`, whose `regex::Regex::is_match` borrows a scratch cache from a
//! thread-sharded pool inside the `regex` crate — a worker that finds its shard
//! empty builds one. `crates/shapes/tests/change_path_alloc.rs` traced that
//! residual allocation by allocation and holds `sh:pattern` out of its two exact
//! equality assertions for exactly this reason, while
//! `pattern_change_path_allocation_has_no_growth_term_below_the_parallel_threshold`
//! measures the same shape below the threshold and pins a slope of exactly zero.
//!
//! Every group here is **report-only**: nothing in this file asserts a threshold,
//! a ratio or a comparison against a baseline. The allocation invariants these
//! groups illustrate are executed as contracts in
//! `crates/shapes/tests/change_path_alloc.rs`.
//!
//! # The probe lines, and why this target uses both measurement modes
//!
//! Interleaved with the criterion groups are `println!` probe lines a human
//! reads when comparing two runs. They are measured with the workspace's shared
//! counting allocator, and this is the one target that needs both of its
//! windows in one process:
//!
//! * the validation, preparation, pattern and rule probes wrap code that fans
//!   focus nodes out over `rayon` above `PARALLEL_MIN_FOCUS_NODES`, so they use
//!   a [`WholeProcessWindow`]; a per-thread window would miss every worker and
//!   report the parallel sizes as the cheapest in the sweep;
//! * the schema-import, LinkML-import and slot-emission probes are
//!   single-threaded, so they use a [`CurrentThreadWindow`], which keeps their
//!   figures free of whatever criterion's own machinery is doing elsewhere.
//!
//! The whole-process ledger is armed only inside its windows, so the timed
//! criterion measurements outside them pay one relaxed load per allocation.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf::loss::LossLedger;
use purrdf::{DatasetView, GraphMatch, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, WholeProcessWindow};
use purrdf_shapes::engine::{
    __prepared_class_membership_view, FocusId, PreparedValidator, parse_shapes, validate_graphs,
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

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const IMPORT_CLASSES: usize = 128;
const IMPORT_PROPERTIES_PER_CLASS: usize = 8;
const LINKML: &str = "https://w3id.org/linkml/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const LINKML_EMIT_SIZES: &[usize] = &[32, 1_024, 60_000];
const CORE_FOCUS_SIZES: &[usize] = &[512, 1_024, 2_048, 3_000, 100_000, 1_000_000];
const CLOSED_FOCUS_SIZES: &[usize] = &[512, 4_096, 65_536];
const SPARQL_FOCUS_SIZES: &[usize] = &[64, 512, 4_096];
const UNIQUE_VALUES_FOCUS_SIZES: &[usize] = &[512, 4_096, 65_536];
const COMPUTED_VALUES_FOCUS_SIZES: &[usize] = &[512, 4_096, 65_536];
const TARGET_WHERE_FOCUS_SIZES: &[usize] = &[512, 4_096, 65_536];
const REALTIME_FOCUS_SIZES: &[usize] = &[1, 8, 64, 512, 4_096];
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const BENCH_EX: &str = "https://example.org/shacl-bench/";
const CLASS_DEPTH: usize = 40;
const MEMBERSHIP_DATASET_FOCUS_NODES: usize = 100_000;
const MEMBERSHIP_PATTERN_FOCUS_NODES: usize = 4_096;
const MEMBERSHIP_RULE_FOCUS_NODES: usize = 64;
/// How many conforming focus nodes the change-path contrast's dataset holds.
///
/// The same scale as `shacl_focus_realtime`'s: the claim being illustrated is that
/// the change path's per-request cost is independent of the graph it sits on, and
/// illustrating it over a small graph would illustrate nothing.
const CONTRAST_DATASET_FOCUS_NODES: usize = 1_000_000;

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
                let report = validate_graphs(data, shapes, None)
                    .unwrap_or_else(|e| panic!("validation failed for {name}: {e:?}"));
                std::hint::black_box(report);
            }
        });
    });
    group.finish();
}

fn core_focus_fixture(focus_nodes: usize) -> ValidationFixture {
    core_focus_dataset(focus_nodes, 0)
}

/// The Core focus fixture, optionally carrying a disjoint VIOLATING population.
///
/// `violating` subjects are targeted by the same shape and carry `ex:value` and
/// `ex:member` but no `ex:label`, so each trips that property shape's
/// `sh:minCount` exactly once and contributes exactly one result. They live in the
/// same dataset as the conforming ones deliberately: the contrast group varies
/// *conformance* and nothing else — same graph, same binding, same shapes — so a
/// difference between its two rows cannot be a difference in dataset size,
/// interning or target resolution.
fn core_focus_dataset(focus_nodes: usize, violating: usize) -> ValidationFixture {
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

    for index in 0..violating {
        let focus = builder.intern_iri(&format!("{BENCH_EX}unlabelled-item{index}"));
        let value = builder.intern_literal(RdfLiteral::typed(index.to_string(), XSD_INTEGER));
        builder.push_quad(focus, rdf_type, focus_classes[CLASS_DEPTH - 1], None);
        builder.push_quad(focus, value_predicate, value, None);
        builder.push_quad(focus, member_predicate, member, None);
    }

    let dataset = builder.freeze().expect("Core focus fixture must freeze");
    let shapes = parse_shapes(
        &format!(
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
        ),
        None,
    )
    .expect("Core focus shapes must parse");
    ValidationFixture {
        dataset,
        shapes,
        focus_nodes,
    }
}

/// A `sh:closed` node shape with five simple-predicate property shapes carrying
/// `sh:minLength`/`sh:maxLength`/`sh:nodeKind`/`sh:languageIn`/`sh:datatype`,
/// over `focus_nodes` conforming subjects. Every subject also carries `rdf:type`,
/// which the shape admits only through `sh:ignoredProperties`, so the closed
/// permitted-set probe runs on every outgoing triple of every focus node.
fn closed_focus_fixture(focus_nodes: usize) -> ValidationFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let closed_class = builder.intern_iri(&format!("{BENCH_EX}ClosedClass"));
    let label_predicate = builder.intern_iri(&format!("{BENCH_EX}label"));
    let name_predicate = builder.intern_iri(&format!("{BENCH_EX}name"));
    let code_predicate = builder.intern_iri(&format!("{BENCH_EX}code"));
    let value_predicate = builder.intern_iri(&format!("{BENCH_EX}value"));
    let member_predicate = builder.intern_iri(&format!("{BENCH_EX}member"));
    let member = builder.intern_iri(&format!("{BENCH_EX}closed-member"));

    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}closed-item{index}"));
        let label = builder.intern_literal(RdfLiteral::simple(format!("item-{index}")));
        let name = builder.intern_literal(RdfLiteral::language_tagged(
            format!("name {index}"),
            if index % 2 == 0 { "en" } else { "fr-CA" },
        ));
        let code = builder.intern_literal(RdfLiteral::simple(format!("CODE-{index:05}")));
        let value = builder.intern_literal(RdfLiteral::typed(index.to_string(), XSD_INTEGER));
        builder.push_quad(focus, rdf_type, closed_class, None);
        builder.push_quad(focus, label_predicate, label, None);
        builder.push_quad(focus, name_predicate, name, None);
        builder.push_quad(focus, code_predicate, code, None);
        builder.push_quad(focus, value_predicate, value, None);
        builder.push_quad(focus, member_predicate, member, None);
    }

    let dataset = builder.freeze().expect("closed focus fixture must freeze");
    let shapes = parse_shapes(
        &format!(
            r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix ex: <{BENCH_EX}> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:ClosedBundleShape a sh:NodeShape ;
    sh:targetClass ex:ClosedClass ;
    sh:closed true ;
    sh:ignoredProperties ( rdf:type ) ;
    sh:property [
        sh:path ex:label ;
        sh:minCount 1 ;
        sh:nodeKind sh:Literal ;
        sh:minLength 3 ;
        sh:maxLength 32 ;
    ] ;
    sh:property [
        sh:path ex:name ;
        sh:nodeKind sh:Literal ;
        sh:languageIn ( "en" "fr" ) ;
    ] ;
    sh:property [
        sh:path ex:code ;
        sh:minLength 5 ;
        sh:maxLength 16 ;
    ] ;
    sh:property [
        sh:path ex:value ;
        sh:datatype xsd:integer ;
        sh:nodeKind sh:Literal ;
    ] ;
    sh:property [
        sh:path ex:member ;
        sh:nodeKind sh:IRI ;
        sh:maxLength 64 ;
    ] .
"#
        ),
        None,
    )
    .expect("closed focus shapes must parse");
    ValidationFixture {
        dataset,
        shapes,
        focus_nodes,
    }
}

/// A `sh:uniqueValuesFor ( ex:notation ex:scheme )` node shape over
/// `focus_nodes` conforming subjects, each with its own notation in one of 16
/// schemes — the cross-focus component, whose verdict for a focus node depends on
/// every other target node. The grouping of the target set is built once per
/// validation, so the cost should grow linearly in the focus count, not
/// quadratically.
fn unique_values_focus_fixture(focus_nodes: usize) -> ValidationFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let concept = builder.intern_iri(&format!("{BENCH_EX}Concept"));
    let notation_predicate = builder.intern_iri(&format!("{BENCH_EX}notation"));
    let scheme_predicate = builder.intern_iri(&format!("{BENCH_EX}scheme"));
    let schemes: Vec<TermId> = (0..16)
        .map(|scheme| builder.intern_iri(&format!("{BENCH_EX}scheme{scheme}")))
        .collect();
    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}unique-item{index}"));
        let notation = builder.intern_literal(RdfLiteral::simple(format!("N-{index}")));
        builder.push_quad(focus, rdf_type, concept, None);
        builder.push_quad(focus, notation_predicate, notation, None);
        builder.push_quad(
            focus,
            scheme_predicate,
            schemes[index % schemes.len()],
            None,
        );
    }
    let dataset = builder
        .freeze()
        .expect("uniqueValuesFor focus fixture must freeze");
    let shapes = parse_shapes(
        &format!(
            r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .

ex:UniqueNotationShape a sh:NodeShape ;
    sh:targetClass ex:Concept ;
    sh:uniqueValuesFor ( ex:notation ex:scheme ) .
"
        ),
        None,
    )
    .expect("uniqueValuesFor focus shapes must parse");
    ValidationFixture {
        dataset,
        shapes,
        focus_nodes,
    }
}

/// A property shape over `focus_nodes` conforming rectangles whose `ex:area`
/// value nodes come from `form`: `asserted` (each rectangle carries `ex:area`),
/// `values` (each carries `ex:width`, and `sh:values [ sh:path ex:width ]`
/// computes the area from it) or `default` (none carries either, and
/// `sh:defaultValue 1` supplies it). `sh:minCount 1` and `sh:datatype
/// xsd:integer` hold in every form, so the report is empty and every row does the
/// same constraint work over one value node per focus node.
fn computed_values_focus_fixture(focus_nodes: usize, form: &str) -> ValidationFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let rectangle = builder.intern_iri(&format!("{BENCH_EX}Rectangle"));
    let area = builder.intern_iri(&format!("{BENCH_EX}area"));
    let width = builder.intern_iri(&format!("{BENCH_EX}width"));
    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}rectangle{index}"));
        builder.push_quad(focus, rdf_type, rectangle, None);
        let value =
            builder.intern_literal(RdfLiteral::typed((index % 97 + 1).to_string(), XSD_INTEGER));
        match form {
            "asserted" => builder.push_quad(focus, area, value, None),
            "values" => builder.push_quad(focus, width, value, None),
            _ => {}
        }
    }
    let dataset = builder
        .freeze()
        .expect("computed-values focus fixture must freeze");
    let computed = match form {
        "asserted" => "",
        "values" => "sh:values [ sh:path ex:width ] ;",
        _ => "sh:defaultValue 1 ;",
    };
    let shapes = parse_shapes(
        &format!(
            r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <{BENCH_EX}> .

ex:RectangleShape a sh:NodeShape ;
    sh:targetClass ex:Rectangle ;
    sh:property [ sh:path ex:area ; {computed} sh:minCount 1 ; sh:datatype xsd:integer ] .
"
        ),
        None,
    )
    .expect("computed-values focus shapes must parse");
    ValidationFixture {
        dataset,
        shapes,
        focus_nodes,
    }
}

/// A `sh:targetWhere` fixture: `focus_nodes` concepts, each with a notation, and
/// as many unrelated nodes beside them, so a where target that scans every node of
/// the graph has twice the candidates one that narrows to the class does.
///
/// `narrowed` selects the where shape: `[ sh:class ex:Concept ]`, whose class
/// bounds the candidates to its instances, or the same condition wrapped in a
/// one-member `sh:or`, which means the same thing and which the narrowing does not
/// look inside — the full scan over every node of the graph.
fn target_where_focus_fixture(focus_nodes: usize, narrowed: bool) -> ValidationFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let concept = builder.intern_iri(&format!("{BENCH_EX}Concept"));
    let notation_predicate = builder.intern_iri(&format!("{BENCH_EX}notation"));
    let other_predicate = builder.intern_iri(&format!("{BENCH_EX}other"));
    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{BENCH_EX}where-item{index}"));
        let notation = builder.intern_literal(RdfLiteral::simple(format!("N-{index}")));
        builder.push_quad(focus, rdf_type, concept, None);
        builder.push_quad(focus, notation_predicate, notation, None);
        let unrelated = builder.intern_iri(&format!("{BENCH_EX}where-other{index}"));
        builder.push_quad(unrelated, other_predicate, focus, None);
    }
    let dataset = builder
        .freeze()
        .expect("targetWhere focus fixture must freeze");
    let condition = if narrowed {
        "[ sh:class ex:Concept ]"
    } else {
        "[ sh:or ( [ sh:class ex:Concept ] ) ]"
    };
    let shapes = parse_shapes(
        &format!(
            r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .

ex:ConceptShape a sh:NodeShape ;
    sh:targetWhere {condition} ;
    sh:property [ sh:path ex:notation ; sh:minCount 1 ] .
"
        ),
        None,
    )
    .expect("targetWhere focus shapes must parse");
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
    ), None)
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
    let shapes = parse_shapes(
        &format!(
            r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{BENCH_EX}> .

ex:MembershipShape a sh:NodeShape ;
    sh:targetClass ex:MembershipClass0 ;
    sh:nodeKind sh:IRI .
"
        ),
        None,
    )
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
    let window = WholeProcessWindow::open();
    let started = Instant::now();
    validate_fixture(fixture);
    let elapsed = started.elapsed();
    let measured = window.close();
    println!(
        "[shacl_focus_validation] case={label} focus_nodes={} quads={} terms={} threads={} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.focus_nodes,
        fixture.dataset.quad_count(),
        fixture.dataset.term_count(),
        rayon::current_num_threads(),
        elapsed.as_nanos(),
        measured.allocations,
        measured.requested_bytes,
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

fn bench_focus_closed(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_focus_closed");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    for &focus_nodes in CLOSED_FOCUS_SIZES {
        let fixture = closed_focus_fixture(focus_nodes);
        let probe = Once::new();
        group.throughput(Throughput::Elements(focus_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(focus_nodes),
            &fixture,
            move |bencher, fixture| {
                probe.call_once(|| print_validation_probe("closed", fixture));
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

fn bench_focus_unique_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_focus_unique_values_for");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    for &focus_nodes in UNIQUE_VALUES_FOCUS_SIZES {
        let fixture = unique_values_focus_fixture(focus_nodes);
        let probe = Once::new();
        group.throughput(Throughput::Elements(focus_nodes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(focus_nodes),
            &fixture,
            move |bencher, fixture| {
                probe.call_once(|| print_validation_probe("unique_values_for", fixture));
                bencher.iter(|| validate_fixture(black_box(fixture)));
            },
        );
    }
    group.finish();
}

fn bench_focus_target_where(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_focus_target_where");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    for &focus_nodes in TARGET_WHERE_FOCUS_SIZES {
        for (label, narrowed) in [("narrowed", true), ("full_scan", false)] {
            let fixture = target_where_focus_fixture(focus_nodes, narrowed);
            let probe = Once::new();
            group.throughput(Throughput::Elements(focus_nodes as u64));
            group.bench_with_input(
                BenchmarkId::new(label, focus_nodes),
                &fixture,
                move |bencher, fixture| {
                    probe.call_once(|| {
                        print_validation_probe(&format!("target_where_{label}"), fixture);
                    });
                    bencher.iter(|| validate_fixture(black_box(fixture)));
                },
            );
        }
    }
    group.finish();
}

fn bench_focus_computed_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("shacl_focus_computed_values");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    for &focus_nodes in COMPUTED_VALUES_FOCUS_SIZES {
        for form in ["asserted", "values", "default"] {
            let fixture = computed_values_focus_fixture(focus_nodes, form);
            let probe = Once::new();
            group.throughput(Throughput::Elements(focus_nodes as u64));
            group.bench_with_input(
                BenchmarkId::new(form, focus_nodes),
                &fixture,
                move |bencher, fixture| {
                    probe.call_once(|| {
                        print_validation_probe(&format!("computed_values_{form}"), fixture);
                    });
                    bencher.iter(|| validate_fixture(black_box(fixture)));
                },
            );
        }
    }
    group.finish();
}

/// Mint one focus id against the binding that will validate it.
///
/// The change path takes ids stamped with their binding, so a benchmark resolves
/// them through the binding rather than through the dataset beside it. Every call
/// happens while a fixture is being assembled, outside every timed or measured
/// region.
fn focus_id(prepared: &PreparedValidator, iri: &str) -> FocusId {
    prepared
        .term_id(&purrdf_shapes::term::NamedNode::new_unchecked(iri).into_term())
        .unwrap_or_else(|| panic!("benchmark focus {iri} must be interned"))
}

/// The first `count` class-membership focus nodes, minted against `prepared`.
fn membership_focus_ids(prepared: &PreparedValidator, count: usize) -> Vec<FocusId> {
    (0..count)
        .map(|index| focus_id(prepared, &format!("{BENCH_EX}membership-item{index}")))
        .collect()
}

fn validate_prepared_ids(prepared: &PreparedValidator, focus_ids: &[FocusId]) {
    let report = prepared
        .validate_focus_node_ids(focus_ids)
        .expect("prepared benchmark validation must not error");
    assert!(report.conforms, "prepared benchmark fixture must conform");
    black_box(report);
}

fn print_realtime_probe(prepared: &PreparedValidator, focus_ids: &[FocusId]) {
    validate_prepared_ids(prepared, focus_ids);
    let window = WholeProcessWindow::open();
    let started = Instant::now();
    validate_prepared_ids(prepared, focus_ids);
    let elapsed = started.elapsed();
    let measured = window.close();
    println!(
        "[shacl_focus_realtime] requested_focus_nodes={} elapsed_ns={} allocations={} allocated_bytes={}",
        focus_ids.len(),
        elapsed.as_nanos(),
        measured.allocations,
        measured.requested_bytes,
    );
}

fn bench_focus_realtime(c: &mut Criterion) {
    const DATASET_FOCUS_NODES: usize = 1_000_000;

    let fixture = core_focus_fixture(DATASET_FOCUS_NODES);
    let preparation_window = WholeProcessWindow::open();
    let preparation_started = Instant::now();
    let prepared = PreparedValidator::from_projected_dataset(
        Arc::clone(&fixture.dataset),
        Arc::new(fixture.shapes.clone()),
    )
    .expect("realtime benchmark preparation must succeed");
    let preparation_elapsed = preparation_started.elapsed();
    let measured = preparation_window.close();
    println!(
        "[shacl_focus_prepare] dataset_focus_nodes={DATASET_FOCUS_NODES} elapsed_ns={} allocations={} allocated_bytes={}",
        preparation_elapsed.as_nanos(),
        measured.allocations,
        measured.requested_bytes,
    );
    // Minted BY the binding: a focus id names the binding it belongs to, so it
    // cannot be resolved from the dataset beside it.
    let all_focus_ids: Vec<FocusId> = (0..*REALTIME_FOCUS_SIZES.last().expect("non-empty sizes"))
        .map(|index| focus_id(&prepared, &format!("{BENCH_EX}item{index}")))
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

/// One binding over one graph, with a conforming and a violating focus
/// population addressable separately.
struct ContrastFixture {
    prepared: PreparedValidator,
    conforming_ids: Vec<FocusId>,
    violating_ids: Vec<FocusId>,
    dataset_focus_nodes: usize,
}

/// Build the contrast fixture: one dataset, one preparation, two focus
/// populations differing only in whether they satisfy the shape.
fn contrast_fixture() -> ContrastFixture {
    let violating = *REALTIME_FOCUS_SIZES
        .last()
        .expect("realtime sizes are non-empty");
    let fixture = core_focus_dataset(CONTRAST_DATASET_FOCUS_NODES, violating);
    let prepared = PreparedValidator::from_projected_dataset(
        Arc::clone(&fixture.dataset),
        Arc::new(fixture.shapes.clone()),
    )
    .expect("contrast benchmark preparation must succeed");
    let conforming_ids = (0..violating)
        .map(|index| focus_id(&prepared, &format!("{BENCH_EX}item{index}")))
        .collect();
    let violating_ids = (0..violating)
        .map(|index| focus_id(&prepared, &format!("{BENCH_EX}unlabelled-item{index}")))
        .collect();
    ContrastFixture {
        prepared,
        conforming_ids,
        violating_ids,
        dataset_focus_nodes: fixture.focus_nodes,
    }
}

/// Validate a conforming focus set through the change path; answer its result
/// count, which must be zero.
fn validate_conforming_ids(prepared: &PreparedValidator, focus_ids: &[FocusId]) -> usize {
    let report = prepared
        .validate_focus_node_ids(focus_ids)
        .expect("conforming contrast validation must not error");
    assert!(report.conforms, "the conforming contrast row must conform");
    let results = report.results.len();
    black_box(report);
    results
}

/// Validate a violating focus set through the same path; answer its result count,
/// which must be one per focus node.
///
/// The count is asserted, not merely reported: a cheap row that had stopped
/// producing results would otherwise be indistinguishable from a cheap row that
/// still checked everything, and the whole contrast rests on the violating side
/// really doing the work.
fn validate_violating_ids(prepared: &PreparedValidator, focus_ids: &[FocusId]) -> usize {
    let report = prepared
        .validate_focus_node_ids(focus_ids)
        .expect("violating contrast validation must not error");
    assert!(
        !report.conforms,
        "the violating contrast row must not conform"
    );
    assert_eq!(
        report.results.len(),
        focus_ids.len(),
        "every violating contrast focus node must contribute exactly one result"
    );
    let results = report.results.len();
    black_box(report);
    results
}

fn print_contrast_probe(
    fixture: &ContrastFixture,
    conformance: &str,
    focus_ids: &[FocusId],
    run: fn(&PreparedValidator, &[FocusId]) -> usize,
) {
    run(&fixture.prepared, focus_ids);
    let window = WholeProcessWindow::open();
    let started = Instant::now();
    let results = run(&fixture.prepared, focus_ids);
    let elapsed = started.elapsed();
    let measured = window.close();
    println!(
        "[shacl_change_path_contrast] stage=2 conformance={conformance} dataset_focus_nodes={} requested_focus_nodes={} results={results} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.dataset_focus_nodes,
        focus_ids.len(),
        elapsed.as_nanos(),
        measured.allocations,
        measured.requested_bytes,
    );
}

/// The conforming-versus-violating contrast, which is what deferred
/// materialization buys.
///
/// The change path materializes a focus node only where a result is built, so a
/// conforming graph should pay a constant no matter how many focus nodes the
/// change touched, while a violating one pays per violation. Both halves of that
/// sentence are measurable and neither is worth much alone: the conforming row on
/// its own is satisfied perfectly by a validator that stopped validating, and the
/// violating row on its own says nothing about the common case. They are reported
/// side by side, over one dataset and one binding, so the only thing that differs
/// between two rows at the same size is whether the focus nodes conform.
///
/// Report-only, like every other group in this file: no threshold, ratio or
/// baseline is asserted here. The zero-growth claim itself is an executable
/// contract in `crates/shapes/tests/change_path_alloc.rs`, which is where it
/// belongs — a bench that gated on it would be a gate on a machine, not on the
/// code.
fn bench_change_path_contrast(c: &mut Criterion) {
    let fixture = contrast_fixture();

    let mut group = c.benchmark_group("shacl_change_path_contrast");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    for &focus_nodes in REALTIME_FOCUS_SIZES {
        for (conformance, ids, run) in [
            (
                "conforming",
                &fixture.conforming_ids[..focus_nodes],
                validate_conforming_ids as fn(&PreparedValidator, &[FocusId]) -> usize,
            ),
            (
                "violating",
                &fixture.violating_ids[..focus_nodes],
                validate_violating_ids as fn(&PreparedValidator, &[FocusId]) -> usize,
            ),
        ] {
            let probe = Once::new();
            let fixture_ref = &fixture;
            group.throughput(Throughput::Elements(focus_nodes as u64));
            group.bench_with_input(
                BenchmarkId::new(conformance, focus_nodes),
                ids,
                move |bencher, focus_ids| {
                    probe.call_once(|| {
                        print_contrast_probe(fixture_ref, conformance, focus_ids, run);
                    });
                    bencher.iter(|| {
                        black_box(run(black_box(&fixture_ref.prepared), black_box(focus_ids)));
                    });
                },
            );
        }
    }
    group.finish();
}

fn print_membership_preparation_probe(fixture: &MembershipFixture) -> PreparedValidator {
    let window = WholeProcessWindow::open();
    let started = Instant::now();
    let prepared = prepare_membership_fixture(fixture);
    let elapsed = started.elapsed();
    let measured = window.close();
    let dimensions = prepared.__class_membership_dimensions();
    assert_membership_dimensions(fixture, dimensions);
    validate_prepared_ids(&prepared, &membership_focus_ids(&prepared, 1));
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
        measured.allocations,
        measured.requested_bytes,
    );
    prepared
}

fn print_membership_realtime_probe(
    fixture: &MembershipFixture,
    prepared: &PreparedValidator,
    focus_ids: &[FocusId],
) {
    validate_prepared_ids(prepared, focus_ids);
    let window = WholeProcessWindow::open();
    let started = Instant::now();
    validate_prepared_ids(prepared, focus_ids);
    let elapsed = started.elapsed();
    let measured = window.close();
    println!(
        "[shacl_subclass_realtime] variant={} dataset_focus_nodes={} requested_focus_nodes={} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.variant.label(),
        fixture.focus_nodes,
        focus_ids.len(),
        elapsed.as_nanos(),
        measured.allocations,
        measured.requested_bytes,
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
        let membership_ids = membership_focus_ids(&prepared, fixture.focus_ids.len());

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
            let focus_ids = &membership_ids[..focus_nodes];
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
    let window = WholeProcessWindow::open();
    let started = Instant::now();
    let rows = membership_pattern_count(view, subject, Some(fixture.rdf_type), object);
    let elapsed = started.elapsed();
    let measured = window.close();
    assert_eq!(rows, expected_rows);
    println!(
        "[shacl_subclass_pattern] variant={} pattern={pattern} dataset_focus_nodes={} result_rows={rows} elapsed_ns={} allocations={} allocated_bytes={}",
        fixture.variant.label(),
        fixture.focus_nodes,
        elapsed.as_nanos(),
        measured.allocations,
        measured.requested_bytes,
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
    ), None)
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
        let window = WholeProcessWindow::open();
        let started = Instant::now();
        run_membership_rules(&fixture, &shapes);
        let elapsed = started.elapsed();
        let measured = window.close();
        println!(
            "[shacl_subclass_rule_rounds] variant={} focus_nodes={} rounds=2 index_builds_per_round=1 elapsed_ns={} allocations={} allocated_bytes={}",
            variant.label(),
            fixture.focus_nodes,
            elapsed.as_nanos(),
            measured.allocations,
            measured.requested_bytes,
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

fn linkml_import_fixture(config: &SchemaImportConfig) -> LinkmlDocument {
    let imported = import_json_schema(&schema_import_fixture(), config)
        .expect("benchmark source schema imports");
    let compiled = purrdf_shapes::json_schema::compile(&imported.shapes, config.namespaces())
        .expect("schema compilation");
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

    let window = CurrentThreadWindow::open();
    let observed = import_json_schema(&schema, &config).expect("allocation probe imports");
    let measured = window.close();
    assert_eq!(observed.shapes.node_shapes.len(), IMPORT_CLASSES);
    println!(
        "[shacl_schema_import] classes={IMPORT_CLASSES} properties={} allocations={} allocated_bytes={}",
        IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS,
        measured.allocations,
        measured.requested_bytes
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

    let window = CurrentThreadWindow::open();
    let observed = import_linkml(&document, &config).expect("allocation probe imports");
    let measured = window.close();
    assert_eq!(observed.shapes.node_shapes.len(), expected_shapes);
    println!(
        "[shacl_linkml_import] source_classes={IMPORT_CLASSES} source_properties={} imported_shapes={expected_shapes} allocations={} allocated_bytes={}",
        IMPORT_CLASSES * IMPORT_PROPERTIES_PER_CLASS,
        measured.allocations,
        measured.requested_bytes
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

            let window = CurrentThreadWindow::open();
            let observed = emit_linkml(&compiled, &config).expect("allocation probe emits");
            let measured = window.close();
            assert_output(&observed);
            println!(
                "[linkml_slot_emission] mode={} slots={slots} renames={expected_renames} collisions={expected_collisions} allocations={} allocated_bytes={}",
                mode.label(),
                measured.allocations,
                measured.requested_bytes
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
    bench_focus_closed,
    bench_focus_sparql,
    bench_focus_unique_values,
    bench_focus_target_where,
    bench_focus_computed_values,
    bench_focus_realtime,
    bench_change_path_contrast,
    bench_subclass_membership,
    bench_subclass_patterns,
    bench_subclass_rule_rounds,
    bench_schema_import,
    bench_linkml_import,
    bench_linkml_slot_emission
);
criterion_main!(benches);
