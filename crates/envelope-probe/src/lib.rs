// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The micro-hardware envelope workload set.
//!
//! This crate implements the capture side of the small-deployment validation
//! envelope (`docs/design/micro-hardware-envelope.md`): a fixed, deterministic
//! workload set run over the **public** PurRDF APIs, per named profile, so a
//! release can demonstrate that a constrained deployment class still fits its
//! pinned ceilings. Everything here is measurement, never a gate: pass
//! criteria are completion and memory; wall time is recorded evidence only.
//!
//! The library half is target-portable (the future wasm profile compiles this
//! same workload set); the binary half owns the counting allocator, timing,
//! and report emission. Workloads therefore return domain metrics — counts,
//! serialized sizes, governor evidence — and the caller wraps them with
//! allocator readings.
//!
//! The corpus is the deterministic keystone fixture generator
//! (`purrdf_rdf::gts_fixtures`): RNG-free, and deliberately shaped to exercise
//! named graphs, scoped blank nodes, all three literal shapes, and the RDF 1.2
//! statement layer, so a highly compressible flat corpus cannot under-report
//! the envelope.
//!
//! It is one of **two** deterministic corpus generators in this repository,
//! and the two own different regimes. The keystone fixture is small, fixed and
//! statement-layer-focused, which is what makes an envelope ceiling meaningful
//! on a constrained deployment: the workload has to be pinned before a memory
//! budget can mean anything. The scale profile (`purrdf-bench`,
//! `purrdf-scale-mixed-v1`) is the opposite regime — shardable,
//! anti-compressible and parameterized, built to keep a *capacity* claim at
//! scale honest against a corpus a single dictionary trick could flatter.
//! Neither substitutes for the other: an envelope measured over the scale
//! profile would not be an envelope, and a capacity claim measured over the
//! keystone fixture would be measured over a corpus designed to be small.
//!
//! The 32-bit boundary workload (logical IDs above `2^32` through bounded
//! local buffers) is absent because it has nothing to measure yet: it exercises
//! the fallible read-session seams, and it is meaningful only once those seams
//! carry the bounded local buffers it is built to stress.

use std::sync::Arc;

use purrdf_core::{
    DatasetView, FallibleDatasetView, GraphMatch, InMemoryPageProvider, PagedDataset,
    PagedQueryLimits, RdfDataset, RdfLookaside, ResourceDimension, SparqlRequest, SparqlResult,
    TermValue, ViewOperationStatus,
};
use purrdf_rdf::gts_fixtures::{keystone_base, keystone_contribution};
use purrdf_rdf::{
    NativeRdfFormat, SerializeGraph, SerializeOptions, StatementLayer, import_gts_events,
    parse_dataset, serialize_dataset_with,
};
use purrdf_shapes::engine::{self as shacl_engine, GovernedValidation};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryGovernors, QueryOptions};

/// One recorded quantity: a stable label and its value.
pub type Metric = (&'static str, u64);

/// A named envelope profile: the workload scale a deployment class is
/// validated at. Memory ceilings are enforced by the harness around the
/// process (cgroup / wasm linear-memory maximum), not here; the profile pins
/// the *work*, the harness pins the *budget*.
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    /// Stable profile name (`smoke`, `wasm-browser`, `sbc-32-small`, …).
    pub name: &'static str,
    /// Keystone row groups for the resident dataset (~7 quads per group).
    pub groups: usize,
    /// Number of pages in the paged-tier workload.
    pub paged_pages: usize,
    /// Contribution rows per page (~2 quads per row).
    pub paged_rows_per_page: usize,
    /// Page budget for the bounded paged query.
    pub paged_max_pages: u64,
    /// Byte budget for the bounded paged query.
    pub paged_max_bytes: u64,
}

/// The named profiles of the envelope proposal, plus `smoke` for tests and
/// quick verification runs. Scales follow the proposal's dataset column;
/// ceilings ratchet downward as captures land.
pub const PROFILES: &[Profile] = &[
    Profile {
        name: "smoke",
        groups: 64,
        paged_pages: 4,
        paged_rows_per_page: 64,
        paged_max_pages: 4,
        paged_max_bytes: 8 << 20,
    },
    Profile {
        name: "wasm-browser",
        groups: 14_300,
        paged_pages: 8,
        paged_rows_per_page: 2_000,
        paged_max_pages: 8,
        paged_max_bytes: 64 << 20,
    },
    Profile {
        name: "sbc-32-small",
        groups: 14_300,
        paged_pages: 32,
        paged_rows_per_page: 15_625,
        paged_max_pages: 32,
        paged_max_bytes: 128 << 20,
    },
    Profile {
        name: "sbc-64",
        groups: 143_000,
        paged_pages: 64,
        paged_rows_per_page: 78_125,
        paged_max_pages: 64,
        paged_max_bytes: 512 << 20,
    },
    Profile {
        name: "edge-gateway",
        groups: 1_430_000,
        paged_pages: 64,
        paged_rows_per_page: 78_125,
        paged_max_pages: 64,
        paged_max_bytes: 1 << 30,
    },
];

/// Looks a profile up by name.
#[must_use]
pub fn profile(name: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.name == name)
}

/// The workload names, in run order.
pub const WORKLOADS: &[&str] = &["roundtrip", "governed_query", "shacl", "gts", "pack_paged"];

/// Exactly which metrics each workload owes, by name.
///
/// A measurement that is emitted only when some condition holds stops being
/// measured the moment that condition stops holding — and the conditions worth
/// guarding on are usually the ones the system falsifies by working. The absence
/// that results is invisible: a reader sees a report with one fewer entry and no
/// statement that anything was skipped. The roster turns that absence into a
/// refusal, so a metric can only leave this crate by being deleted from here too,
/// deliberately and in the diff.
const EXPECTED_METRICS: &[(&str, &[&str])] = &[
    (
        "roundtrip",
        &[
            "term_count",
            "rdf_rows",
            "payload_bytes",
            "nquads_bytes",
            "trig_bytes",
            "jsonld_bytes",
        ],
    ),
    (
        "governed_query",
        &[
            "join_fuel",
            "join_intermediate_cells",
            "join_answer_rows",
            "join_scratch_bytes",
            "truncation_tripped",
            "neighbor_answer_rows",
        ],
    ),
    (
        "shacl",
        &["conforms", "results", "fuel", "intermediate_cells"],
    ),
    ("gts", &["gts_bytes", "rdf_rows"]),
    (
        "pack_paged",
        &[
            "pack_bytes",
            "pack_join_fuel",
            "paged_pages",
            "paged_page_bytes",
            "paged_consumed_pages",
            "paged_consumed_bytes",
            "single_graph_pages_predicted",
            "single_graph_pages_touched",
            "single_graph_rows",
            "single_graph_retained_pages",
            "paged_budget_tripped",
            "paged_neighbor_ok",
        ],
    ),
];

/// The metric names a workload owes, or `None` when it has no roster.
fn expected_metrics(workload: &str) -> Option<&'static [&'static str]> {
    EXPECTED_METRICS
        .iter()
        .find(|(name, _)| *name == workload)
        .map(|(_, metrics)| *metrics)
}

/// Checks an emitted metric list against the workload's roster, in both
/// directions: a missing metric is a measurement that quietly stopped happening,
/// an unlisted one is a roster that quietly stopped describing the workload.
fn check_metric_roster(workload: &str, metrics: &[Metric]) -> Result<(), String> {
    let expected = expected_metrics(workload)
        .ok_or_else(|| format!("workload {workload:?} declares no expected metrics"))?;
    for name in expected {
        if !metrics.iter().any(|(emitted, _)| emitted == name) {
            return Err(format!(
                "workload {workload} did not emit {name}: a measurement that stops being taken \
                 deletes the evidence it exists to produce"
            ));
        }
    }
    for (emitted, _) in metrics {
        if !expected.contains(emitted) {
            return Err(format!(
                "workload {workload} emitted {emitted}, which it does not declare"
            ));
        }
    }
    if metrics.len() != expected.len() {
        return Err(format!(
            "workload {workload} emitted {} metrics against {} declared: a name is repeated",
            metrics.len(),
            expected.len()
        ));
    }
    Ok(())
}

/// Runs one named workload at a profile's scale.
///
/// # Errors
///
/// A string naming the first failure; the probe is evidence tooling, and any
/// failure is a finding to report verbatim, never to degrade around. Emitting
/// fewer (or other) metrics than the workload declares is itself such a failure.
pub fn run(workload: &str, profile: &Profile) -> Result<Vec<Metric>, String> {
    let metrics = match workload {
        "roundtrip" => roundtrip(profile),
        "governed_query" => governed_query(profile),
        "shacl" => shacl(profile),
        "gts" => gts(profile),
        "pack_paged" => pack_paged(profile),
        other => return Err(format!("unknown workload {other:?}")),
    }?;
    check_metric_roster(workload, &metrics)?;
    Ok(metrics)
}

/// A SELECT answer normalized for comparison: the projection, plus the rows in a
/// canonical order, so two executions are compared as answer *sets* rather than
/// as whatever order each happened to produce.
#[derive(PartialEq, Eq)]
struct SolutionSet {
    /// The projected variable names, in projection order.
    variables: Vec<String>,
    /// The solution rows, sorted.
    rows: Vec<Vec<Option<TermValue>>>,
}

/// Normalizes a SELECT result, refusing anything that is not a solution sequence.
fn solution_set(result: SparqlResult, label: &str) -> Result<SolutionSet, String> {
    match result {
        SparqlResult::Solutions {
            variables,
            mut rows,
            ..
        } => {
            rows.sort_unstable();
            Ok(SolutionSet { variables, rows })
        }
        SparqlResult::Graph(_) | SparqlResult::Boolean(_) => {
            Err(format!("{label} produced no solution sequence to count"))
        }
    }
}

/// The dataset-bearing formats of the round-trip workload. Turtle is absent
/// on purpose: it is a triples syntax, so a dataset round-trip through it is
/// not identity and belongs to a lossiness workload, not this one.
const ROUNDTRIP_FORMATS: &[(NativeRdfFormat, &str, &str)] = &[
    (NativeRdfFormat::NQuads, "application/n-quads", "nquads"),
    (NativeRdfFormat::TriG, "application/trig", "trig"),
    (NativeRdfFormat::JsonLd, "application/ld+json", "jsonld"),
];

fn roundtrip(profile: &Profile) -> Result<Vec<Metric>, String> {
    let dataset = keystone_base(profile.groups);
    let rows = dataset.rdf_row_count() as u64;
    let mut metrics = vec![
        ("term_count", dataset.term_count() as u64),
        ("rdf_rows", rows),
        ("payload_bytes", dataset.rdf_payload_bytes() as u64),
    ];
    for (format, media_type, label) in ROUNDTRIP_FORMATS {
        let options = SerializeOptions {
            selection: SerializeGraph::Dataset,
            statement_layer: StatementLayer::PerFormatCapability,
            jsonld_options: None,
        };
        let outcome = serialize_dataset_with(&*dataset, *format, None, &options)
            .map_err(|d| format!("{label} serialize: {d}"))?;
        let dropped = (outcome.statement_rows_dropped
            + outcome.directional_literals_dropped
            + outcome.named_graph_rows_dropped) as u64;
        let reparsed = parse_dataset(&outcome.bytes, media_type, None)
            .map_err(|d| format!("{label} reparse: {d}"))?;
        let back = reparsed.rdf_row_count() as u64;
        if back + dropped != rows {
            return Err(format!(
                "{label} round-trip lost rows: {rows} out, {back} back, {dropped} declared dropped"
            ));
        }
        metrics.push(match *label {
            "nquads" => ("nquads_bytes", outcome.bytes.len() as u64),
            "trig" => ("trig_bytes", outcome.bytes.len() as u64),
            _ => ("jsonld_bytes", outcome.bytes.len() as u64),
        });
    }
    Ok(metrics)
}

/// The join query every profile measures: touches every group through both
/// predicates, so its intermediate volume scales with the corpus.
const JOIN_QUERY: &str = "SELECT ?s ?o WHERE { \
     ?s <https://example.org/p> ?o . ?s <https://example.org/q> \"bare\" }";

/// The selective neighbor: one bound subject, one pattern. Under the same
/// tiny budget that refuses the join, this must still complete — the
/// over-refusal discipline applied to budgets.
const SELECTIVE_QUERY: &str =
    "SELECT ?o WHERE { <https://example.org/s7> <https://example.org/p> ?o }";

/// The budget that must refuse the join at every profile scale while
/// admitting the selective neighbor.
const TRUNCATION_CELLS: u64 = 16;

fn governed_query(profile: &Profile) -> Result<Vec<Metric>, String> {
    let dataset = keystone_base(profile.groups);
    let engine = NativeSparqlEngine::new();

    let request = SparqlRequest {
        query: JOIN_QUERY,
        base_iri: None,
        substitutions: &[],
    };
    let outcome = engine
        .query_governed(
            &dataset,
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .map_err(|d| format!("metered join: {d}"))?;
    let evidence = outcome.evidence().clone();
    if !outcome.is_complete() {
        return Err("metered join did not complete".to_owned());
    }
    let mut metrics = vec![
        ("join_fuel", evidence.consumed_in(ResourceDimension::Fuel)),
        (
            "join_intermediate_cells",
            evidence.consumed_in(ResourceDimension::IntermediateCells),
        ),
        (
            "join_answer_rows",
            evidence.consumed_in(ResourceDimension::AnswerRows),
        ),
        (
            "join_scratch_bytes",
            evidence.consumed_in(ResourceDimension::ScratchBytes),
        ),
    ];

    let tiny = QueryGovernors::METERED.with_max_intermediate_cells(TRUNCATION_CELLS);
    let refused = engine
        .query_governed(
            &dataset,
            SparqlRequest {
                query: JOIN_QUERY,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &tiny,
        )
        .map_err(|d| format!("truncation join: {d}"))?;
    if refused.is_complete() {
        return Err(format!(
            "the join completed under a {TRUNCATION_CELLS}-cell budget; the truncation case no longer truncates"
        ));
    }
    metrics.push(("truncation_tripped", 1));

    let neighbor = engine
        .query_governed(
            &dataset,
            SparqlRequest {
                query: SELECTIVE_QUERY,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &tiny,
        )
        .map_err(|d| format!("selective neighbor: {d}"))?;
    if !neighbor.is_complete() {
        return Err(format!(
            "the selective neighbor was refused under the {TRUNCATION_CELLS}-cell budget: over-refusal"
        ));
    }
    let neighbor_rows = neighbor
        .evidence()
        .consumed_in(ResourceDimension::AnswerRows);
    if neighbor_rows == 0 {
        return Err(format!(
            "the selective neighbor completed under the {TRUNCATION_CELLS}-cell budget with no \
             rows: a query that answers nothing evidences no admission"
        ));
    }
    metrics.push(("neighbor_answer_rows", neighbor_rows));
    Ok(metrics)
}

/// Shapes for the keystone corpus, deliberately covering BOTH validation paths.
///
/// `ex:ProbeShape` is the core-constraint path: every subject of `ex:p` must
/// carry at least one `ex:q`. The generator's per-group `shared` blank node is a
/// subject of `ex:p` only, so the corpus yields exactly one violation per group.
///
/// `ex:ProbeSparqlShape` states the same rule as a SHACL-SPARQL constraint. It
/// exists because governors bound the SPARQL paths ONLY — core constraint
/// evaluation reads the IR directly and spends no evaluator budget, so a shapes
/// graph with no SHACL-SPARQL in it validates under any budget, including a zero
/// one. With core constraints alone the workload's `fuel` and
/// `intermediate_cells` were structurally zero: two measurements that measured
/// nothing, reported alongside real ones. Running one SPARQL query per focus node
/// charges the governor, so those metrics now carry evidence.
///
/// Both shapes flag the same nodes, so `results` counts each violating node twice
/// and still scales with the profile.
const SHAPES_TTL: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <https://example.org/> .

ex:ProbeShape a sh:NodeShape ;
  sh:targetSubjectsOf ex:p ;
  sh:property [ sh:path ex:q ; sh:minCount 1 ] .

ex:ProbeSparqlShape a sh:NodeShape ;
  sh:targetSubjectsOf ex:p ;
  sh:sparql [
    sh:select "SELECT $this WHERE { $this <https://example.org/p> ?o FILTER NOT EXISTS { $this <https://example.org/q> ?q } }" ;
  ] .
"#;

fn shacl(profile: &Profile) -> Result<Vec<Metric>, String> {
    let dataset = keystone_base(profile.groups);
    let shapes = shacl_engine::parse_shapes(SHAPES_TTL, Some("https://example.org/shapes"))
        .map_err(|e| format!("parse_shapes: {e}"))?;
    let governed = shacl_engine::validate_dataset_with_governors(
        &dataset,
        &shapes,
        None,
        &QueryGovernors::METERED,
    )
    .map_err(|e| format!("validate: {e}"))?;
    match governed {
        GovernedValidation::Complete { report, evidence } => {
            let fuel = evidence.consumed_in(ResourceDimension::Fuel);
            let cells = evidence.consumed_in(ResourceDimension::IntermediateCells);
            // Governors bound the SPARQL paths only, so a shapes graph carrying no
            // SHACL-SPARQL charges nothing and reports a budget it never spent. The
            // shapes above include a SPARQL constraint precisely so these two read
            // something; refuse rather than publish a measurement that measures
            // nothing next to ones that do.
            if fuel == 0 || cells == 0 {
                return Err(format!(
                    "metered validation reported {} fuel and {cells} intermediate cells over \
                     {} results: a governor that charges nothing evidences no bound, so the \
                     shapes must exercise a SPARQL path",
                    fuel,
                    report.results.len()
                ));
            }
            Ok(vec![
                ("conforms", u64::from(report.conforms)),
                ("results", report.results.len() as u64),
                ("fuel", fuel),
                ("intermediate_cells", cells),
            ])
        }
        GovernedValidation::BudgetExhausted { tripped, .. } => Err(format!(
            "metered validation tripped {}: METERED must not bound",
            tripped.label()
        )),
    }
}

fn gts(profile: &Profile) -> Result<Vec<Metric>, String> {
    let dataset = keystone_base(profile.groups);
    let rows = dataset.rdf_row_count() as u64;
    let bytes = purrdf_rdf::gts_write::to_gts(&dataset, &RdfLookaside::default(), "purrdf")
        .map_err(|d| format!("gts write: {d}"))?;
    let bundle = import_gts_events(&bytes).map_err(|d| format!("gts read: {d}"))?;
    let back = bundle.dataset.rdf_row_count() as u64;
    if back != rows {
        return Err(format!("gts round-trip lost rows: {rows} out, {back} back"));
    }
    Ok(vec![("gts_bytes", bytes.len() as u64), ("rdf_rows", rows)])
}

/// A query touching every page of the paged tier (each page holds one graph).
const PAGED_SCAN_QUERY: &str = "SELECT ?g ?o WHERE { GRAPH ?g { ?s <https://example.org/p> ?o } }";

fn pack_paged(profile: &Profile) -> Result<Vec<Metric>, String> {
    // Pack leg: build, reopen zero-copy, query through the pack view.
    let dataset = keystone_base(profile.groups);
    let pack_bytes =
        purrdf_core::PackBuilder::build_bytes(&dataset).map_err(|e| format!("pack build: {e}"))?;
    let view =
        purrdf_core::PackView::from_bytes(&pack_bytes).map_err(|e| format!("pack open: {e}"))?;
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(JOIN_QUERY, None)
        .map_err(|d| format!("prepare: {d}"))?;
    let outcome = engine
        .query_prepared_governed_view(
            &view,
            &prepared,
            &[],
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .map_err(|d| format!("pack query: {d}"))?;
    if !outcome.is_complete() {
        return Err("metered pack query did not complete".to_owned());
    }
    let mut metrics = vec![
        ("pack_bytes", pack_bytes.len() as u64),
        (
            "pack_join_fuel",
            outcome.evidence().consumed_in(ResourceDimension::Fuel),
        ),
    ];
    drop(outcome);
    drop(view);
    drop(pack_bytes);
    drop(dataset);

    // Paged leg: disjoint per-page graphs and term spaces, byte charges from
    // each page's own payload accounting.
    let pages: Vec<(Arc<RdfDataset>, u64)> = (0..profile.paged_pages)
        .map(|index| {
            let graph = format!("https://example.org/page/{index}");
            let page = keystone_contribution(&graph, index, profile.paged_rows_per_page);
            let bytes = page.rdf_payload_bytes() as u64;
            (page, bytes)
        })
        .collect();
    let total_page_bytes: u64 = pages.iter().map(|(_, b)| *b).sum();
    let provider = Arc::new(InMemoryPageProvider::with_byte_lengths(
        pages,
        purrdf_core::PageGeneration(1),
    ));
    let paged = PagedDataset::from_provider(provider).map_err(|e| format!("paged seal: {e}"))?;
    metrics.push(("paged_pages", profile.paged_pages as u64));
    metrics.push(("paged_page_bytes", total_page_bytes));

    let bounded = paged.query_view(PagedQueryLimits::new(
        profile.paged_max_pages,
        profile.paged_max_bytes,
    ));
    let request = SparqlRequest {
        query: PAGED_SCAN_QUERY,
        base_iri: None,
        substitutions: &[],
    };
    let result = engine.query_governed_fallible_view(
        &bounded,
        request,
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    if let Err(diagnostic) = result {
        return Err(format!(
            "bounded paged scan failed under the profile budget: {diagnostic}"
        ));
    }
    match bounded.operation_status() {
        ViewOperationStatus::Ready { evidence } => {
            metrics.push(("paged_consumed_pages", evidence.consumed_pages));
            metrics.push(("paged_consumed_bytes", evidence.consumed_bytes));
        }
        ViewOperationStatus::Failed { .. } => {
            return Err("paged view reported failure after a complete scan".to_owned());
        }
    }

    // A graph-selective query's page footprint, measured rather than assumed, and
    // paired with the footprint the sealed metadata PREDICTS. Recording only the
    // measured count would make pruning indistinguishable from a query that simply
    // found nothing; asserting the two against each other makes the narrowing
    // positively evidenced.
    let single_graph_query =
        "SELECT ?o WHERE { GRAPH <https://example.org/page/0> { ?s <https://example.org/p> ?o } }";
    let predicate_id = paged
        .term_id_by_value(&TermValue::iri("https://example.org/p"))
        .ok_or_else(|| "the single-graph query's predicate is not interned".to_owned())?;
    let graph_id = paged
        .term_id_by_value(&TermValue::iri("https://example.org/page/0"))
        .ok_or_else(|| "the single-graph query's graph is not interned".to_owned())?;
    let predicted = paged
        .pages_for_pattern(None, Some(predicate_id), None, GraphMatch::Named(graph_id))
        .len() as u64;
    metrics.push(("single_graph_pages_predicted", predicted));
    let measured = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let first = engine
        .query_governed_fallible_view(
            &measured,
            SparqlRequest {
                query: single_graph_query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .map_err(|diagnostic| format!("the single-graph query failed unbounded: {diagnostic}"))?;
    let (answer, _) = first.into_parts();
    let answers = solution_set(answer, "the unbounded single-graph query")?;
    let ViewOperationStatus::Ready { evidence } = measured.operation_status() else {
        return Err("unbounded single-graph query left the view failed".to_owned());
    };
    let touched = evidence.consumed_pages;
    metrics.push(("single_graph_pages_touched", touched));
    if predicted != touched {
        return Err(format!(
            "the sealed metadata predicted {predicted} pages for the single-graph query but it \
             consumed {touched}"
        ));
    }

    // The row axis of the same guarantee. A page count alone reads identically whether
    // narrowing kept the owning page's answers or pruned them to nothing, so the count is
    // asserted against the shape of the fixture that produced the page: each contribution
    // emits exactly one `ex:p` row per contribution row, wholly inside its own graph, and
    // the query binds exactly that pattern in exactly that graph.
    let expected_rows = profile.paged_rows_per_page as u64;
    let rows = answers.rows.len() as u64;
    if rows == 0 {
        return Err(format!(
            "the single-graph query returned no rows while touching {touched} page(s): a page \
             count cannot tell narrowing apart from a query that found nothing"
        ));
    }
    if rows != expected_rows {
        return Err(format!(
            "the single-graph query returned {rows} rows where its one page holds \
             {expected_rows} matching rows"
        ));
    }
    metrics.push(("single_graph_rows", rows));

    // Graph-scoped eviction, exercised on the shipped surface rather than described.
    // `retain_graph` drops every page the sealed per-stream postings prove holds nothing
    // in the graph, so it is the eviction counterpart of the prediction above: the same
    // metadata that says which pages a query WILL touch says which pages may be released
    // without losing a row. Two legs are asserted here — the eviction happened, and it
    // changed no answer — because either alone is satisfied by doing nothing: retaining
    // every page preserves the answer, and dropping the owning page evicts plenty. The
    // third leg, that it materializes no page, is a claim about the sealed postings
    // rather than about this query, and is pinned by name in the paged backend's own
    // tests.
    let retained = paged.retain_graph(graph_id);
    let retained_pages = retained.page_count() as u64;
    if retained_pages >= profile.paged_pages as u64 {
        return Err(format!(
            "retaining the single-graph pages kept {retained_pages} of {} pages: an eviction \
             that evicts nothing evidences no graph-scoped retention",
            profile.paged_pages
        ));
    }
    let retained_view = retained.query_view(PagedQueryLimits::UNBOUNDED);
    let retained_answer = engine
        .query_governed_fallible_view(
            &retained_view,
            SparqlRequest {
                query: single_graph_query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .map_err(|diagnostic| {
            format!("the single-graph query failed after graph-scoped eviction: {diagnostic}")
        })?;
    let (retained_answer, _) = retained_answer.into_parts();
    let retained_answers = solution_set(retained_answer, "the single-graph query after eviction")?;
    if retained_answers != answers {
        return Err(format!(
            "graph-scoped eviction changed the single-graph answer: {} rows before against {} \
             after, and the two answer sets are not identical",
            answers.rows.len(),
            retained_answers.rows.len()
        ));
    }
    metrics.push(("single_graph_retained_pages", retained_pages));

    // Refusal pair at the measured boundary: one page fewer refuses, the exact
    // consumption completes. Both sides are executed, so a tightened budget cannot
    // silently over-refuse. A zero-page footprint would leave nothing to starve, and it
    // is refused rather than skipped: the pair going quiet exactly when the narrowing
    // starts working would delete the evidence it exists to produce.
    if touched == 0 {
        return Err(format!(
            "the single-graph query returned {rows} rows while consuming no pages, so the \
             page-budget boundary has nothing to exercise"
        ));
    }
    let starved = paged.query_view(PagedQueryLimits::new(touched - 1, profile.paged_max_bytes));
    let refused = engine.query_governed_fallible_view(
        &starved,
        SparqlRequest {
            query: single_graph_query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    if refused.is_ok() {
        return Err(format!(
            "the single-graph query completed under a {}-page budget after consuming {touched}",
            touched - 1
        ));
    }
    metrics.push(("paged_budget_tripped", 1));

    let exact = paged.query_view(PagedQueryLimits::new(touched, profile.paged_max_bytes));
    let neighbor = engine
        .query_governed_fallible_view(
            &exact,
            SparqlRequest {
                query: single_graph_query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .map_err(|diagnostic| {
            format!(
                "the single-graph query was refused at its own measured {touched}-page budget: \
                 over-refusal ({diagnostic})"
            )
        })?;
    // Narrowing is allowed to change how many pages a query touches. It is not allowed to
    // change the answer, so the exact-budget re-run is compared row for row against the
    // unbounded one rather than merely counted.
    let (narrowed_answer, _) = neighbor.into_parts();
    let narrowed = solution_set(narrowed_answer, "the exact-budget single-graph query")?;
    if narrowed != answers {
        return Err(format!(
            "the exact {touched}-page budget changed the single-graph answer: {} rows unbounded \
             against {} narrowed, and the two answer sets are not identical",
            answers.rows.len(),
            narrowed.rows.len()
        ));
    }
    metrics.push(("paged_neighbor_ok", 1));
    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::{
        EXPECTED_METRICS, Metric, PROFILES, WORKLOADS, check_metric_roster, expected_metrics,
        profile, run,
    };

    /// Every `WORKLOADS` entry owns a non-empty, duplicate-free `EXPECTED_METRICS`
    /// roster, and every roster names a workload that is actually run — a stray
    /// roster entry would silently stop tracking anything.
    #[test]
    fn every_workload_declares_a_distinct_metric_roster() {
        for workload in WORKLOADS {
            let declared = expected_metrics(workload)
                .unwrap_or_else(|| panic!("workload {workload} declares no metrics"));
            assert!(!declared.is_empty(), "{workload} declares an empty roster");
            let mut names = declared.to_vec();
            names.sort_unstable();
            names.dedup();
            assert_eq!(
                names.len(),
                declared.len(),
                "{workload} declares a name twice"
            );
        }
        for (workload, _) in EXPECTED_METRICS {
            assert!(
                WORKLOADS.contains(workload),
                "{workload} is declared but never run"
            );
        }
    }

    /// `check_metric_roster` accepts the full declared roster and refuses all three
    /// ways it can drift from it: a metric dropped from the emitted list, a stray
    /// metric the roster never declared, and a metric repeated under the same name.
    #[test]
    fn the_roster_refuses_a_dropped_or_stray_metric() {
        let declared = expected_metrics("pack_paged").expect("pack_paged declares metrics");
        let full: Vec<Metric> = declared.iter().map(|name| (*name, 0)).collect();
        check_metric_roster("pack_paged", &full).expect("the full roster passes");

        let dropped = &full[..full.len() - 1];
        assert!(
            check_metric_roster("pack_paged", dropped).is_err(),
            "a metric that stopped being emitted passed unnoticed"
        );

        let mut stray = full.clone();
        stray.push(("undeclared_metric", 0));
        assert!(
            check_metric_roster("pack_paged", &stray).is_err(),
            "an undeclared metric passed unnoticed"
        );

        let mut repeated = full.clone();
        repeated.push(full[0]);
        assert!(
            check_metric_roster("pack_paged", &repeated).is_err(),
            "a repeated metric name passed unnoticed"
        );
    }

    #[test]
    fn smoke_profile_runs_every_workload() {
        let smoke = profile("smoke").expect("smoke profile exists");
        for workload in WORKLOADS {
            let metrics =
                run(workload, smoke).unwrap_or_else(|e| panic!("workload {workload} failed: {e}"));
            assert!(!metrics.is_empty(), "{workload} reported no metrics");
        }
    }

    #[test]
    fn profiles_are_unique_and_named() {
        let mut names: Vec<&str> = PROFILES.iter().map(|p| p.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PROFILES.len());
        assert!(profile("no-such-profile").is_none());
    }
}
