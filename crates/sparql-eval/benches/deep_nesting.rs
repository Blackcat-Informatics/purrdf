// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Parsing and evaluating requests nested many levels deep, per nesting construct.
//!
//! Constructs, each written `depth` levels deep over a four-subject dataset:
//! - `filter_exists` — `?s <p> ?o FILTER EXISTS { … }` nested inside itself;
//! - `optional_spine` — `depth` sibling `OPTIONAL { … }` groups, no textual nesting
//!   but an algebra spine `depth` levels tall;
//! - `lateral` — `?s <p> ?o LATERAL { … }` nested inside itself;
//! - `brackets` — `((…(?o)…))` in a `FILTER`;
//! - `not_chain` — `!!…!(?o = 1)` in a `FILTER`;
//! - `path_groups` — `((…(<p>)…))` as a property path;
//! - `triple_terms` — `isTRIPLE(<<( <s> <p> <<( … ?o )>> )>>)`, an expression triple
//!   term nested in the object position.
//!
//! `parse/<construct>/<depth>` is the parser alone (`SparqlParser::parse_query`, the
//! returned tree dropped inside the sample); `evaluate/<construct>/<depth>` is a whole
//! cold request through [`NativeSparqlEngine::query`] with a fresh engine per sample —
//! parse, admission and evaluation.
//!
//! # Which depths run
//!
//! [`DEPTHS`] is the list every construct is measured at; extending it is the one
//! line a deeper evaluator needs. How deep a request may nest today is the stack of
//! the thread answering it, so before sampling, each construct's deepest answering
//! level on the bench thread is found by bisection up to [`PROBE_CEILING`] — a
//! refusal there is the typed stack diagnostic, never an abort. A listed depth past
//! that limit is skipped with a note on stderr, never silently. The limit itself is
//! measured too, as `<construct>/bench_thread_limit`, at fifteen sixteenths of the
//! probed depth: the timed frames sit a little deeper in the harness than the probe
//! does, and the margin keeps that difference from turning the case into a refusal.
//! The probed depth is printed on stderr; the case id stays fixed so reports compare
//! across revisions.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench deep_nesting` (the
//! `make bench` lane) — excluded from `make check`. No timing is asserted.

use std::hint::black_box;
use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, SparqlEngine, SparqlRequest,
    SparqlResult,
};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{EvalError, NativeSparqlEngine};
use purrdf_testkit::bench::{Bench, bench_group, bench_main};

const EX: &str = "http://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The depths every construct is measured at, where the bench thread's stack holds
/// them.
const DEPTHS: &[usize] = &[16, 64, 256];

/// The deepest level the limit probe tries. A construct that still answers here is
/// measured here.
const PROBE_CEILING: usize = 20_000;

/// `<s1>` … `<s4>` each `<p>` their number, and only `<s1>` also `<q>` it.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for n in 1..=4 {
        let s = builder.intern_iri(&format!("{EX}s{n}"));
        let o = builder.intern_literal(RdfLiteral::typed(n.to_string(), XSD_INTEGER));
        builder.push_quad(s, p, o, None);
        if n == 1 {
            builder.push_quad(s, q, o, None);
        }
    }
    builder.freeze().expect("the deep-nesting dataset")
}

/// `open` written `n` times around `core`, closed by `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), close.repeat(n))
}

/// One nesting construct: its bench id and its query text at a depth.
struct Construct {
    name: &'static str,
    text: fn(usize) -> String,
}

/// Every construct. Each answers at least one row at every depth it answers at, so
/// an evaluation that silently emptied would fail the pre-sampling check.
fn constructs() -> [Construct; 7] {
    [
        Construct {
            name: "filter_exists",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ {} }}",
                    nested(
                        &format!("?s <{EX}p> ?o FILTER EXISTS {{ "),
                        &format!("?s <{EX}q> ?z"),
                        " }",
                        n
                    )
                )
            },
        },
        Construct {
            name: "optional_spine",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o {} }}",
                    format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(n)
                )
            },
        },
        Construct {
            name: "lateral",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ {} }}",
                    nested(
                        &format!("?s <{EX}p> ?o LATERAL {{ "),
                        &format!("?s <{EX}q> ?z"),
                        " }",
                        n
                    )
                )
            },
        },
        Construct {
            name: "brackets",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
                    nested("(", "?o", ")", n)
                )
            },
        },
        Construct {
            name: "not_chain",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}(?o = 1)) }}",
                    "!".repeat(n)
                )
            },
        },
        Construct {
            name: "path_groups",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s {} ?o FILTER(?o = 2) }}",
                    nested("(", &format!("<{EX}p>"), ")", n)
                )
            },
        },
        Construct {
            name: "triple_terms",
            text: |n| {
                format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER(isTRIPLE({})) }}",
                    nested(&format!("<<( <{EX}s> <{EX}p> "), "?o", " )>>", n)
                )
            },
        },
    ]
}

/// Answer `query` cold through a fresh engine: its row count, or the diagnostic.
fn answer(data: &Arc<RdfDataset>, query: &str) -> Result<usize, RdfDiagnostic> {
    let result = NativeSparqlEngine::new().query(
        data,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
    )?;
    Ok(match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Boolean(value) => usize::from(value),
    })
}

/// Whether `diagnostic` is the evaluator's stack refusal.
fn is_stack_refusal(diagnostic: &RdfDiagnostic) -> bool {
    diagnostic.code == EvalError::STACK_EXHAUSTED_CODE
}

/// Whether `construct` answers `depth` levels deep on this thread. A refusal that is
/// not the stack's is a real failure and panics; a stack refusal is `false`.
fn answers(data: &Arc<RdfDataset>, construct: &Construct, depth: usize) -> bool {
    match answer(data, &(construct.text)(depth)) {
        Ok(rows) => {
            assert!(
                rows > 0,
                "{} {depth} deep answered no rows — the benchmark would be a no-op",
                construct.name
            );
            true
        }
        Err(refused) => {
            assert!(
                is_stack_refusal(&refused),
                "{} {depth} deep failed with something other than the stack refusal: \
                 {refused:?}",
                construct.name
            );
            false
        }
    }
}

/// The deepest level `construct` answers at on this thread, by bisection up to
/// [`PROBE_CEILING`].
fn deepest(data: &Arc<RdfDataset>, construct: &Construct) -> usize {
    assert!(
        answers(data, construct, 1),
        "{} answers one level deep",
        construct.name
    );
    if answers(data, construct, PROBE_CEILING) {
        return PROBE_CEILING;
    }
    let (mut deepest, mut refused) = (1_usize, PROBE_CEILING);
    while refused - deepest > 1 {
        let mid = deepest.midpoint(refused);
        if answers(data, construct, mid) {
            deepest = mid;
        } else {
            refused = mid;
        }
    }
    deepest
}

/// Measure `construct` at `depth`, parse alone and whole cold request.
fn bench_at(
    group: &mut purrdf_testkit::bench::BenchmarkGroup<'_>,
    data: &Arc<RdfDataset>,
    construct: &Construct,
    depth: usize,
    id: &str,
) {
    let query = (construct.text)(depth);
    assert!(
        answers(data, construct, depth),
        "{} {depth} deep answers before it is sampled",
        construct.name
    );
    group.bench_function(format!("parse/{}/{id}", construct.name), |b| {
        b.iter(|| {
            black_box(
                SparqlParser::new()
                    .parse_query(black_box(&query))
                    .expect("the deep request parses"),
            )
        });
    });
    group.bench_function(format!("evaluate/{}/{id}", construct.name), |b| {
        b.iter(|| black_box(answer(data, black_box(&query)).expect("the deep request answers")));
    });
}

fn bench_deep_nesting(c: &mut Bench) {
    let data = dataset();
    let mut group = c.benchmark_group("deep_nesting");
    group.sample_size(10);
    for construct in constructs() {
        let limit = deepest(&data, &construct);
        eprintln!(
            "deep_nesting: {} answers {limit} levels deep on the bench thread",
            construct.name
        );
        for &depth in DEPTHS {
            if depth > limit {
                eprintln!(
                    "deep_nesting: {} skipped at {depth}: the bench thread's stack holds {limit}",
                    construct.name
                );
                continue;
            }
            bench_at(&mut group, &data, &construct, depth, &depth.to_string());
        }
        let margin = limit - limit / 16;
        bench_at(&mut group, &data, &construct, margin, "bench_thread_limit");
    }
    group.finish();
}

bench_group!(benches, bench_deep_nesting);
bench_main!(benches);
