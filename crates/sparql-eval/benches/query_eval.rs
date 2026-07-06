// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! End-to-end SPARQL evaluation benchmark over a ~300k-quad synthetic dataset,
//! driven through [`NativeSparqlEngine`] (parse memoized by the plan cache, BGP
//! orders memoized by the engine's order cache — each sample measures evaluation,
//! not parsing or planning-from-cold).
//!
//! The dataset is a star-shaped "people" graph with skewed predicate cardinalities
//! (`knows` 90k, seven predicates at 30k each, `email` sparse at 3k) plus numeric,
//! language-tagged, and plain literals, and a binary `reportsTo` tree for the
//! transitive-path case. All IRIs are `example.org` fixtures (PurRDF mints no
//! vocabulary).
//!
//! Cases:
//! - `a_selective_join`   — 4-way star BGP join seeded by a sparse predicate.
//! - `b_scan_filter`      — unselective 2-way join + FILTER(REGEX && numeric `>`).
//! - `c_optional_heavy`   — three OPTIONALs (sparse hit, dense hit, multiplying).
//! - `d_union_4`          — UNION of four branches with mixed cardinalities.
//! - `e_group_aggregate`  — GROUP BY 200 keys with COUNT + AVG + MAX.
//! - `f_path_transitive`  — `reportsTo+` closure to the tree root (30k solutions).
//! - `g_order_by_limit`   — whole-relation ORDER BY (numeric DESC, tiebreak) + LIMIT.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench query_eval` (the
//! `make bench` lane) — excluded from `make check`.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult,
};
use purrdf_sparql_eval::NativeSparqlEngine;

/// Entity count. Each person contributes ~10 quads, so 30k people ≈ 303k quads
/// (within the 200k–500k target band).
const PEOPLE: usize = 30_000;

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const EX: &str = "https://example.org/";

/// Build the synthetic star-shaped people graph.
///
/// Per person `i` (0-based):
/// - `rdf:type ex:Person`                        (30k rows, one giant class)
/// - `ex:name "Name{i}"`                         (30k distinct plain literals)
/// - `ex:age  "18 + i % 60"^^xsd:integer`        (30k rows, 60 distinct values)
/// - `ex:label "Person {i}"@en|@de`              (30k lang-tagged literals)
/// - `ex:dept ex:dept{i % 200}`                  (30k rows, 200 objects — moderate skew)
/// - `ex:city ex:city{i % 50}`                   (30k rows, 50 objects — heavy skew)
/// - `ex:knows` ×3 (ring +1, +17, +97)           (90k rows — the hot predicate)
/// - `ex:email "p{i}@example.org"` for `i % 10 == 0` (3k rows — the sparse predicate)
/// - `ex:reportsTo ex:person{(i-1)/2}` for `i>0` (30k-1 rows — a binary tree, depth ~15)
fn people_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let rdf_type = b.intern_iri(RDF_TYPE);
    let person_class = b.intern_iri(&format!("{EX}Person"));
    let p_name = b.intern_iri(&format!("{EX}name"));
    let p_age = b.intern_iri(&format!("{EX}age"));
    let p_label = b.intern_iri(&format!("{EX}label"));
    let p_dept = b.intern_iri(&format!("{EX}dept"));
    let p_city = b.intern_iri(&format!("{EX}city"));
    let p_knows = b.intern_iri(&format!("{EX}knows"));
    let p_email = b.intern_iri(&format!("{EX}email"));
    let p_reports = b.intern_iri(&format!("{EX}reportsTo"));

    let people: Vec<_> = (0..PEOPLE)
        .map(|i| b.intern_iri(&format!("{EX}person{i}")))
        .collect();
    let depts: Vec<_> = (0..200)
        .map(|d| b.intern_iri(&format!("{EX}dept{d}")))
        .collect();
    let cities: Vec<_> = (0..50)
        .map(|c| b.intern_iri(&format!("{EX}city{c}")))
        .collect();

    for i in 0..PEOPLE {
        let s = people[i];
        b.push_quad(s, rdf_type, person_class, None);

        let name = b.intern_literal(RdfLiteral::simple(format!("Name{i}")));
        b.push_quad(s, p_name, name, None);

        let age = b.intern_literal(RdfLiteral::typed((18 + i % 60).to_string(), XSD_INTEGER));
        b.push_quad(s, p_age, age, None);

        let lang = if i % 2 == 0 { "en" } else { "de" };
        let label = b.intern_literal(RdfLiteral::language_tagged(format!("Person {i}"), lang));
        b.push_quad(s, p_label, label, None);

        b.push_quad(s, p_dept, depts[i % 200], None);
        b.push_quad(s, p_city, cities[i % 50], None);

        for step in [1usize, 17, 97] {
            b.push_quad(s, p_knows, people[(i + step) % PEOPLE], None);
        }

        if i % 10 == 0 {
            let email = b.intern_literal(RdfLiteral::simple(format!("p{i}@example.org")));
            b.push_quad(s, p_email, email, None);
        }

        if i > 0 {
            b.push_quad(s, p_reports, people[(i - 1) / 2], None);
        }
    }

    b.freeze().expect("freeze people dataset")
}

/// (a) Selective 4-way star BGP join: the planner should seed on the sparse
/// `email` predicate (3k rows) or the bound-object `city20` pattern (600 rows),
/// then join the dense star arms. 300 result rows (people with `i ≡ 0 mod 10`
/// AND `i ≡ 20 mod 50`, i.e. `i ≡ 20 mod 100`).
const Q_A: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?p ?n ?a WHERE {
  ?p ex:email ?e .
  ?p ex:name ?n .
  ?p ex:age ?a .
  ?p ex:city ex:city20 .
}";

/// (b) Unselective scan + FILTER with REGEX and a numeric comparison: a 30k-row
/// 2-way join, then a per-row regex over the name and a value-space `>` over the
/// integer age.
const Q_B: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?p ?n WHERE {
  ?p ex:name ?n .
  ?p ex:age ?a .
  FILTER(REGEX(?n, \"^Name1[0-9][0-9]2$\") && ?a > 40)
}";

/// (c) OPTIONAL-heavy: a 30k-row base with a sparse OPTIONAL (email, 10% hit), a
/// dense OPTIONAL (label, 100% hit), and a multiplying OPTIONAL (knows, ×3).
const Q_C: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?p ?e ?l ?f WHERE {
  ?p a ex:Person .
  OPTIONAL { ?p ex:email ?e }
  OPTIONAL { ?p ex:label ?l }
  OPTIONAL { ?p ex:knows ?f }
}";

/// (d) UNION of four branches with mixed cardinalities (30k + 30k + 3k + 30k rows).
const Q_D: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?p ?v WHERE {
  { ?p ex:name ?v } UNION { ?p ex:label ?v }
  UNION { ?p ex:email ?v } UNION { ?p ex:dept ?v }
}";

/// (e) GROUP BY + aggregates: 30k joined rows into 200 department groups with
/// COUNT / AVG / MAX over the numeric ages.
const Q_E: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?d (COUNT(?p) AS ?n) (AVG(?a) AS ?avg) (MAX(?a) AS ?max) WHERE {
  ?p ex:dept ?d .
  ?p ex:age ?a .
} GROUP BY ?d";

/// (f) Transitive property path: everyone below the tree root via `reportsTo+`
/// (a 30k-solution closure over a depth-~15 binary tree).
const Q_F: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?e WHERE { ?e ex:reportsTo+ ex:person0 }";

/// (g) ORDER BY + LIMIT: whole-relation sort (numeric DESC with an entity
/// tiebreak) of 30k rows, then a top-10 slice.
const Q_G: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?p ?a WHERE {
  ?p ex:age ?a .
} ORDER BY DESC(?a) ?p LIMIT 10";

/// The full case list as `(criterion id, query text, minimum expected rows)`.
/// The row floor is a sanity check that every case does real work (an empty
/// result would silently benchmark a no-op plan).
const CASES: &[(&str, &str, usize)] = &[
    ("a_selective_join", Q_A, 1),
    ("b_scan_filter", Q_B, 1),
    ("c_optional_heavy", Q_C, PEOPLE),
    ("d_union_4", Q_D, 3 * PEOPLE),
    ("e_group_aggregate", Q_E, 200),
    ("f_path_transitive", Q_F, PEOPLE - 1),
    ("g_order_by_limit", Q_G, 10),
];

/// Run one query end-to-end through the engine, returning its solution count.
fn run(engine: &NativeSparqlEngine, ds: &Arc<RdfDataset>, query: &str) -> usize {
    let result = engine
        .query(
            ds,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("query evaluates");
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Boolean(_) | SparqlResult::Graph(_) => 0,
    }
}

fn bench_query_eval(c: &mut Criterion) {
    let ds = people_dataset();
    let engine = NativeSparqlEngine::new();

    // Sanity pass: every case must produce at least its row floor, and this warm-up
    // also populates the plan cache and the BGP order cache so the timed iterations
    // measure evaluation only.
    for &(label, query, min_rows) in CASES {
        let rows = run(&engine, &ds, query);
        assert!(
            rows >= min_rows,
            "case {label} returned {rows} rows (< {min_rows}) — the benchmark would be a no-op"
        );
    }

    let mut group = c.benchmark_group("query_eval");
    // Whole-dataset evaluations run tens of milliseconds; keep sampling light so
    // the full mix (and `--profile-time` runs under `perf`) stays tractable.
    group.sample_size(10);
    for &(label, query, _) in CASES {
        group.bench_function(label, |bencher| {
            bencher.iter(|| criterion::black_box(run(&engine, &ds, query)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_query_eval);
criterion_main!(benches);
