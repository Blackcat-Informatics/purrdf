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
//! - `h_distinct_dept`    — SELECT DISTINCT collapsing 30k rows to 200 keys, the
//!   entry-API dedup path in `modifier.rs`.
//! - `i_construct_blank_free` — a `CONSTRUCT` template with NO blank-node position,
//!   over 30k rows each carrying a **data** blank object (`ex:note`). Exercises
//!   `construct.rs::instantiate`'s fast path: `MintTracker::minted` can never become
//!   non-empty for this template, so `track_minted`/`track_minted_predicate` are
//!   skipped outright rather than run only to record dead-weight `data` labels.
//! - `j_construct_blank_bearing` — the SAME `WHERE`, but the template mints a fresh
//!   blank node per row, so the tracked path (§16.2 freshness bookkeeping) runs for
//!   real. Comparing this against `i_construct_blank_free` at equal row/data-blank
//!   volume is what makes the fast path's savings visible in the report.
//! - `k_property_function_join` — a 30k-row graph arm driving a **property-function**
//!   call into a host-injected 50-row relation: one `bf` invocation per driving row,
//!   which is the per-row dispatch path (argument evaluation, cursor open, filtered
//!   scan, row bind) beside the ordinary joins above.
//! - `l_single_group_aggregate` — the WITHIN-group chunked partial aggregation shape:
//!   NO `GROUP BY` at all, so `GROUP_CONCAT`/`MAX` fold the whole 30k-row `age`
//!   relation as ONE implicit group. `e_group_aggregate` above has 200 groups of
//!   ~150 rows each, which the ACROSS-groups fork (`eval_group`'s per-group
//!   `par_chunk_try_map_init`) already parallelizes; a single group never gives that
//!   fork more than one unit of work, so THIS case is the one
//!   `crate::parallel::par_chunk_reduce_init` (wired into
//!   `crate::modifier::eval_aggregate`'s phase 2) exists for. Report-only, like every
//!   other case here: no speedup is asserted, this documents the curve honestly —
//!   `GROUP_CONCAT`'s string-building work scales with total output size regardless
//!   of chunking, and `MAX`'s comparison work is cheap per item either way, so the
//!   wall-clock win this case shows (if any, on a given machine) comes entirely from
//!   spreading that fold's `step`/`combine` calls across rayon workers rather than
//!   from doing less work.
//! - `m_arithmetic_dense_filter` — a `FILTER` with several `+`/`-`/`*`/`/` operators
//!   chained over the DATASET-bound `?age` variable (never a literal constant — see
//!   the case's own doc comment for why). A **catastrophe tripwire only**: see
//!   `value_dispatch` below for the bench that can actually resolve the dispatch
//!   layer's cost.
//!
//! A second, separate criterion group — `value_dispatch` — isolates the value-space
//! operator dispatch (`value_add`/`value_sub`) from operand extraction, at ns
//! resolution; see its own doc comment for why `m_arithmetic_dense_filter` above
//! cannot do this.
//!
//! Report-only, `cargo bench -p purrdf-sparql-eval --bench query_eval` (the
//! `make bench` lane) — excluded from `make check`. Timings are not asserted;
//! this target documents relative cost, it does not gate it.

use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest,
    SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    MemoryRelation, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
};

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
/// - `ex:note _:note{i}`                          (30k rows — a DATA-carried blank
///   object per person, one distinct blank per row; feeds the `i_construct_blank_free`
///   / `j_construct_blank_bearing` CONSTRUCT cases)
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
    let p_note = b.intern_iri(&format!("{EX}note"));

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

        let note = b.intern_blank(&format!("note{i}"), BlankScope::DEFAULT);
        b.push_quad(s, p_note, note, None);
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

/// (h) DISTINCT over a heavily duplicated key: 30k joined rows collapse to the
/// 200 distinct `ex:dept` values, so the `dedup` entry-API path (`modifier.rs`)
/// does 30k hash-map probes with ~29.8k `Entry::Occupied` hits and only 200
/// `Entry::Vacant` inserts — the regime the single-owner `entry()` rewrite
/// (replacing a separate `contains_key` probe + `insert`) targets.
const Q_H: &str = "\
PREFIX ex: <https://example.org/>
SELECT DISTINCT ?d WHERE {
  ?p ex:dept ?d .
}";

/// (i) `CONSTRUCT` with a BLANK-FREE template over 30k rows that each carry a
/// DATA blank (`ex:note`) in object position. The template mints nothing —
/// `?p`/`?n` are both plain variables — so `construct.rs::instantiate` takes the
/// untracked fast path: `MintTracker::minted` can never become non-empty for
/// this template, so no label is ever inserted into either tracker set.
const Q_I: &str = "\
PREFIX ex: <https://example.org/>
CONSTRUCT { ?p ex:related ?n } WHERE {
  ?p ex:note ?n .
}";

/// (j) The same `WHERE` as (i), but the template MINTS a fresh blank node per
/// row instead of carrying the data blank through. Same row/data-blank volume
/// as (i), but every row now runs the full §16.2 freshness bookkeeping
/// (`track_minted` populates `MintTracker::minted`, and the eventual
/// `freshness_remap` check actually has something to intersect).
const Q_J: &str = "\
PREFIX ex: <https://example.org/>
CONSTRUCT { ?p ex:related _:x } WHERE {
  ?p ex:note ?n .
}";

/// (k) A **property-function** join: the 30k-row `ex:city` arm drives a call into a
/// host-injected relation, which is invoked once per driving row with its subject
/// position bound (`bf`) and answers from a 50-row in-memory table no index sized.
/// This is the per-row dispatch path — argument evaluation, cursor open, filtered
/// scan, row bind — laid beside the ordinary joins above.
const Q_K: &str = "\
PREFIX ex: <https://example.org/>
PREFIX rel: <https://example.org/rel/>
SELECT ?p ?region WHERE {
  ?p ex:city ?c .
  ?c rel:cityRegion ?region .
}";

/// (l) A single implicit group (no `GROUP BY`): `GROUP_CONCAT`/`MAX` fold the
/// whole 30k-row `age` relation as ONE group — the shape within-group chunked
/// partial aggregation targets (see the module docs' case list).
const Q_L: &str = "\
PREFIX ex: <https://example.org/>
SELECT (GROUP_CONCAT(?a; separator=\",\") AS ?ages) (MAX(?a) AS ?max) WHERE {
  ?p ex:age ?a .
}";

/// (m) Arithmetic-dense `FILTER` over a **dataset-bound** variable (`?age`), never
/// a literal constant: `const_atom` memoization (`expr.rs`) caches a literal
/// operand's parsed `XsdValue` per AST node on its first evaluation, so a FILTER
/// built from literal constants would measure the (memoized) constant-folding
/// path rather than the per-row extraction + dispatch path production traffic
/// actually takes.
///
/// **Catastrophe tripwire only — not a resolution instrument.** Cost model from
/// the code, per `+`/`-`/`*`/`/` over two dataset-bound operands: operand
/// extraction (lexical + datatype-IRI lookup, a full lexical re-parse, an intern
/// probe) is the dominant cost; the `value_*` family dispatch this issue adds is
/// one compare-and-branch on an in-register discriminant — under 0.1% of one
/// evaluation. This row, run whole-query with `sample_size(10)` on a possibly
/// contended host, has percent-level sample variance, so it can resolve a
/// 10-30%-class effect (e.g. accidental dynamic dispatch, a lost monomorphization)
/// but the dispatch-layer delta itself sits below this row's noise floor and
/// this row must NOT be read as measuring it — `value_dispatch` below is the
/// bench built to resolve that at ns granularity.
const Q_M: &str = "\
PREFIX ex: <https://example.org/>
SELECT ?p WHERE {
  ?p ex:age ?a .
  FILTER(((?a + 3) * 2 - (?a - 1)) / 2 > 20)
}";

/// The namespace the benchmark host configures for its one relation.
const REL_NS: &str = "https://example.org/rel/";

/// The relation case (k) calls: each of the 50 synthetic cities to one of 5 regions.
fn city_regions() -> PropertyFunctionRegistry {
    let rows = (0..50)
        .map(|c| {
            vec![
                TermValue::iri(format!("{EX}city{c}")),
                TermValue::iri(format!("{EX}region{}", c % 5)),
            ]
        })
        .collect();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        format!("{REL_NS}cityRegion"),
        Arc::new(MemoryRelation::new(1, 1, rows).expect("every row is two values wide")),
    );
    registry
}

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
    ("h_distinct_dept", Q_H, 200),
    ("i_construct_blank_free", Q_I, PEOPLE),
    ("j_construct_blank_bearing", Q_J, PEOPLE),
    ("l_single_group_aggregate", Q_L, 1),
    ("m_arithmetic_dense_filter", Q_M, 1),
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
    count(result)
}

/// [`run`] with a property-function registry injected — the entry case (k) needs, and
/// the only difference between the two paths.
fn run_with_relations(
    engine: &NativeSparqlEngine,
    ds: &Arc<RdfDataset>,
    query: &str,
    relations: &PropertyFunctionRegistry,
) -> usize {
    let result = engine
        .query_with_options_view(
            &**ds,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: relations,
                ..QueryOptions::EMPTY
            },
        )
        .expect("query evaluates");
    count(result)
}

/// The size of one result, whatever shape it has.
fn count(result: SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        SparqlResult::Graph(graph) => graph.quad_count(),
        SparqlResult::Boolean(_) => 0,
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

    // Case (k) runs through the registry-carrying entry, so it warms up and benches
    // beside the others rather than inside their table.
    let relations = city_regions();
    let rows = run_with_relations(&engine, &ds, Q_K, &relations);
    assert!(
        rows >= PEOPLE,
        "case k_property_function_join returned {rows} rows (< {PEOPLE}) — the benchmark \
         would be a no-op"
    );

    let mut group = c.benchmark_group("query_eval");
    // Whole-dataset evaluations run tens of milliseconds; keep sampling light so
    // the full mix (and `--profile-time` runs under `perf`) stays tractable.
    group.sample_size(10);
    for &(label, query, _) in CASES {
        group.bench_function(label, |bencher| {
            bencher.iter(|| criterion::black_box(run(&engine, &ds, query)));
        });
    }
    group.bench_function("k_property_function_join", |bencher| {
        bencher.iter(|| criterion::black_box(run_with_relations(&engine, &ds, Q_K, &relations)));
    });
    group.finish();
}

/// Isolates the `value_*` operator dispatch layer (`purrdf_xsd::ops`) from
/// operand extraction, at ns resolution — the resolution `m_arithmetic_dense_filter`
/// above cannot reach (its own doc comment states why).
///
/// Operand pairs are parsed **once, outside the timed loop** — parsing cost must
/// not appear inside a dispatch-layer measurement. `int_plus_int_numeric_add`
/// is the CONTROL: it calls [`purrdf_xsd::numeric_add`] directly, on the exact
/// same pre-parsed operands `int_plus_int_value_add` feeds to
/// [`purrdf_xsd::value_add`]. `value_add`'s numeric arm does one discriminant
/// range-test and then calls `numeric_add` unchanged, so the
/// `int_plus_int_value_add` minus `int_plus_int_numeric_add` difference **is**
/// the dispatch layer's own cost — nothing else can be structurally different
/// between the two rows.
///
/// `black_box` on the **inputs**, not just the outputs, is mandatory: this
/// workspace's release profile is `lto = "fat"` + `codegen-units = 1`, and a
/// constant, un-blackboxed operand's `XsdValue` discriminant would const-fold
/// at compile time — the loop would then measure a compile-time constant, not
/// a call.
fn bench_value_dispatch(c: &mut Criterion) {
    use purrdf_xsd::{XsdDatatype, numeric_add, parse, value_add, value_sub};

    let int_a = parse("17", XsdDatatype::Integer).expect("parse int_a");
    let int_b = parse("25", XsdDatatype::Integer).expect("parse int_b");
    let dec_a = parse("17.5", XsdDatatype::Decimal).expect("parse dec_a");
    let dec_b = parse("25.25", XsdDatatype::Decimal).expect("parse dec_b");
    let mixed_int = parse("17", XsdDatatype::Integer).expect("parse mixed_int");
    let mixed_dec = parse("25.25", XsdDatatype::Decimal).expect("parse mixed_dec");
    let dt_a = parse("2024-03-10T00:00:00Z", XsdDatatype::DateTime).expect("parse dt_a");
    let dt_b = parse("2024-03-01T00:00:00Z", XsdDatatype::DateTime).expect("parse dt_b");

    let mut group = c.benchmark_group("value_dispatch");
    group.bench_function("int_plus_int_value_add", |bencher| {
        bencher.iter(|| {
            criterion::black_box(value_add(
                criterion::black_box(&int_a),
                criterion::black_box(&int_b),
            ))
        });
    });
    group.bench_function("int_plus_int_numeric_add_control", |bencher| {
        bencher.iter(|| {
            criterion::black_box(numeric_add(
                criterion::black_box(&int_a),
                criterion::black_box(&int_b),
            ))
        });
    });
    group.bench_function("dec_plus_dec_value_add", |bencher| {
        bencher.iter(|| {
            criterion::black_box(value_add(
                criterion::black_box(&dec_a),
                criterion::black_box(&dec_b),
            ))
        });
    });
    group.bench_function("int_plus_dec_value_add", |bencher| {
        bencher.iter(|| {
            criterion::black_box(value_add(
                criterion::black_box(&mixed_int),
                criterion::black_box(&mixed_dec),
            ))
        });
    });
    group.bench_function("datetime_minus_datetime_value_sub", |bencher| {
        bencher.iter(|| {
            criterion::black_box(value_sub(
                criterion::black_box(&dt_a),
                criterion::black_box(&dt_b),
            ))
        });
    });
    group.finish();
}

criterion_group!(benches, bench_query_eval, bench_value_dispatch);
criterion_main!(benches);
