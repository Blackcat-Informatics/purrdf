// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! The corpus is the shared deterministic keystone generator
//! (`purrdf_rdf::gts_fixtures`): RNG-free, and deliberately shaped to exercise
//! named graphs, scoped blank nodes, all three literal shapes, and the RDF 1.2
//! statement layer, so a highly compressible flat corpus cannot under-report
//! the envelope.
//!
//! The 32-bit boundary workload (logical IDs above `2^32` through bounded
//! local buffers) is deliberately absent: it lands with the B2 fallible
//! read-session seams it exists to exercise.

use std::sync::Arc;

use purrdf_core::{
    FallibleDatasetView, InMemoryPageProvider, PagedDataset, PagedQueryLimits, RdfDataset,
    RdfLookaside, ResourceDimension, SparqlRequest, ViewOperationStatus,
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

/// Runs one named workload at a profile's scale.
///
/// # Errors
///
/// A string naming the first failure; the probe is evidence tooling, and any
/// failure is a finding to report verbatim, never to degrade around.
pub fn run(workload: &str, profile: &Profile) -> Result<Vec<Metric>, String> {
    match workload {
        "roundtrip" => roundtrip(profile),
        "governed_query" => governed_query(profile),
        "shacl" => shacl(profile),
        "gts" => gts(profile),
        "pack_paged" => pack_paged(profile),
        other => Err(format!("unknown workload {other:?}")),
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
    metrics.push((
        "neighbor_answer_rows",
        neighbor
            .evidence()
            .consumed_in(ResourceDimension::AnswerRows),
    ));
    Ok(metrics)
}

/// Shapes for the keystone corpus: every subject of `ex:p` must carry at
/// least one `ex:q`. The generator's per-group `shared` blank node is a
/// subject of `ex:p` only, so the corpus yields exactly one violation per
/// group — the workload therefore exercises the violation-report path, and
/// `results` scales with the profile.
const SHAPES_TTL: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <https://example.org/> .

ex:ProbeShape a sh:NodeShape ;
  sh:targetSubjectsOf ex:p ;
  sh:property [ sh:path ex:q ; sh:minCount 1 ] .
";

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
        GovernedValidation::Complete { report, evidence } => Ok(vec![
            ("conforms", u64::from(report.conforms)),
            ("results", report.results.len() as u64),
            ("fuel", evidence.consumed_in(ResourceDimension::Fuel)),
            (
                "intermediate_cells",
                evidence.consumed_in(ResourceDimension::IntermediateCells),
            ),
        ]),
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
    if result.is_err() {
        return Err("bounded paged scan failed under the profile budget".to_owned());
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

    // Measure what a single-graph query actually materializes today. This is
    // a B2 audit datum, recorded rather than assumed: with no per-page graph
    // index in the sealed metadata, even a graph-selective pattern may touch
    // every page.
    let single_graph_query =
        "SELECT ?o WHERE { GRAPH <https://example.org/page/0> { ?s <https://example.org/p> ?o } }";
    let measured = paged.query_view(PagedQueryLimits::UNBOUNDED);
    let first = engine.query_governed_fallible_view(
        &measured,
        SparqlRequest {
            query: single_graph_query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    if first.is_err() {
        return Err("the single-graph query failed unbounded".to_owned());
    }
    let ViewOperationStatus::Ready { evidence } = measured.operation_status() else {
        return Err("unbounded single-graph query left the view failed".to_owned());
    };
    let touched = evidence.consumed_pages;
    metrics.push(("single_graph_pages_touched", touched));

    // Refusal pair at the measured boundary: one page fewer refuses, the
    // exact consumption completes. Both sides of the boundary are executed,
    // so a tightened budget cannot silently over-refuse.
    if touched > 1 {
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
    }
    let exact = paged.query_view(PagedQueryLimits::new(touched, profile.paged_max_bytes));
    let neighbor = engine.query_governed_fallible_view(
        &exact,
        SparqlRequest {
            query: single_graph_query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    if neighbor.is_err() {
        return Err(format!(
            "the single-graph query was refused at its own measured {touched}-page budget: over-refusal"
        ));
    }
    metrics.push(("paged_neighbor_ok", 1));
    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::{PROFILES, WORKLOADS, profile, run};

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
