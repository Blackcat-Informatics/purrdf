// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Task 7 comprehensive determinism gate: forcing the parallel fork-join path
//! (`parallel::force_parallel_for_test(true)`) vs forcing the sequential path
//! (`force_parallel_for_test(false)`) must yield **byte-identical** results for
//! every parallelized node (BGP/joins, MINUS, FILTER, UNION, GROUP BY, BIND) —
//! not just value-equal, but identical row order, since [`crate::parallel`]'s
//! whole contract is order-stability under fork-join.
//!
//! This module lives inside the crate (`#[cfg(test)]`, gated in `lib.rs`) rather
//! than under `tests/` because [`crate::parallel::force_parallel_for_test`] is
//! `pub(crate)` — an external integration-test binary cannot see it.
//!
//! Two things are proven here:
//!
//! 1. [`parallel_paths_are_byte_identical_across_the_corpus`] — a broad query
//!    corpus, each query run twice against the *same* [`NativeSparqlEngine`] +
//!    dataset (so the plan cache and BGP order cache are identical across the
//!    two runs; the *only* difference is the forced threshold), asserting the
//!    two [`SparqlResult`]s match exactly.
//! 2. [`default_pool_runs_a_genuinely_parallel_query`] — one query, run under
//!    the real (unforced) rayon global pool, whose hot BGP scan exceeds
//!    [`crate::parallel::PARALLEL_MIN_ROWS`] — proving the parallel path
//!    actually compiles and schedules under a live multi-thread pool, not just
//!    under the test-only force seam.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult,
    canonicalize,
};

use crate::engine::NativeSparqlEngine;
use crate::parallel::{PARALLEL_MIN_ROWS, force_parallel_for_test};

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const EX: &str = "https://example.org/";

/// Entity count for this gate's dataset. Small enough to build/evaluate the
/// whole corpus twice (forced-parallel + forced-sequential) quickly in `cargo
/// test`, but large enough that the hot predicates (`knows` at 3x, `name`,
/// `age`, `dept`, `city`, ...) comfortably exceed [`PARALLEL_MIN_ROWS`] (1024),
/// so the forced-parallel run genuinely exercises rayon's `par_iter` branch
/// (see [`gate_dataset_crosses_the_parallel_threshold`]) rather than the
/// trivial one-chunk case.
const PEOPLE: usize = 5_000;

/// Build the same star-shaped "people" dataset shape as
/// `benches/query_eval.rs`'s `people_dataset` (predicate list, skew, and the
/// binary `reportsTo` tree), scaled down to [`PEOPLE`] for test speed. Kept as
/// a separate copy (not shared with the bench) because the bench lives in a
/// different crate target (`benches/`) with no access to this crate's
/// `pub(crate)` force seam.
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

    b.freeze()
        .expect("freeze parallel-determinism-gate people dataset")
}

/// Sanity check that this gate's dataset is not a toy: the `knows` predicate
/// (the hot join/BGP-scan predicate exercised by most of the corpus) has
/// `3 * PEOPLE` rows, which must clear [`PARALLEL_MIN_ROWS`] — otherwise the
/// forced-parallel runs below would still take the `rayon::par_iter` branch
/// (the force seam makes that unconditional), but this test documents *why*
/// the dataset size was chosen: even an unforced run over this predicate
/// would genuinely parallelize.
#[test]
fn gate_dataset_crosses_the_parallel_threshold() {
    let hot_predicate_rows = 3 * PEOPLE;
    assert!(
        hot_predicate_rows > PARALLEL_MIN_ROWS,
        "the gate dataset's hot `knows` predicate ({hot_predicate_rows} rows) must exceed \
         PARALLEL_MIN_ROWS ({PARALLEL_MIN_ROWS}) for the corpus below to exercise a genuine \
         parallel workload"
    );
    let scan_rows = PEOPLE;
    assert!(
        scan_rows > PARALLEL_MIN_ROWS,
        "the gate dataset's per-person scan predicates ({scan_rows} rows) must exceed \
         PARALLEL_MIN_ROWS ({PARALLEL_MIN_ROWS})"
    );
}

/// (name, query text) corpus covering every node [`crate::parallel`] wires
/// up (BGP/joins, MINUS, FILTER, UNION, GROUP BY, BIND) plus the 7
/// `benches/query_eval.rs` shapes verbatim (kept in lock-step with that
/// bench's cases, since Task 7's bench evidence and this determinism gate
/// should watch the exact same query mix).
const CORPUS: &[(&str, &str)] = &[
    // -- BGP joins ---------------------------------------------------------
    (
        "join_single_var_selective",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?n ?a WHERE {
  ?p ex:email ?e .
  ?p ex:name ?n .
  ?p ex:age ?a .
  ?p ex:city ex:city20 .
}",
    ),
    (
        "join_multi_var_star",
        // Two SHARED variables (?d, ?c) join independently-bound ?p/?q pairs
        // (not linked via `knows`): every 25-person dept group guarantees a
        // non-empty match, exercising the multi-variable join path.
        "PREFIX ex: <https://example.org/>
SELECT ?p ?q ?qn WHERE {
  ?p ex:dept ?d .
  ?q ex:dept ?d .
  ?p ex:city ?c .
  ?q ex:city ?c .
  ?q ex:name ?qn .
  FILTER(?p != ?q)
}",
    ),
    (
        "join_unselective_scan_filter",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?n WHERE {
  ?p ex:name ?n .
  ?p ex:age ?a .
  FILTER(REGEX(?n, \"^Name1[0-9][0-9]2$\") && ?a > 40)
}",
    ),
    // -- OPTIONAL ------------------------------------------------------------
    (
        "optional_heavy_no_filter",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?e ?l ?f WHERE {
  ?p a ex:Person .
  OPTIONAL { ?p ex:email ?e }
  OPTIONAL { ?p ex:label ?l }
  OPTIONAL { ?p ex:knows ?f }
}",
    ),
    (
        "optional_with_inline_filter",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?a ?e WHERE {
  ?p ex:age ?a .
  OPTIONAL { ?p ex:email ?e . FILTER(?a > 30) }
}",
    ),
    // -- UNION ---------------------------------------------------------------
    (
        "union_4_overlap_and_disjoint",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?v WHERE {
  { ?p ex:name ?v }
  UNION { ?p ex:label ?v }
  UNION { ?p ex:email ?v }
  UNION { ?p ex:dept ?v }
}",
    ),
    (
        "union_with_bind_branches",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?tag WHERE {
  { ?p ex:email ?e . BIND(\"has-email\" AS ?tag) }
  UNION { ?p ex:age ?a . FILTER(?a > 50) BIND(\"senior\" AS ?tag) }
  UNION { ?p ex:city ex:city20 . BIND(\"city20\" AS ?tag) }
}",
    ),
    // -- FILTER (REGEX/numeric/EXISTS/NOT EXISTS) -----------------------------
    (
        "filter_regex_numeric",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?n ?a WHERE {
  ?p ex:name ?n .
  ?p ex:age ?a .
  FILTER(REGEX(?n, \"^Name[0-9]*0$\") && ?a >= 20)
}",
    ),
    (
        "filter_exists",
        "PREFIX ex: <https://example.org/>
SELECT ?p WHERE {
  ?p a ex:Person .
  FILTER EXISTS { ?p ex:email ?e }
}",
    ),
    (
        "filter_not_exists",
        "PREFIX ex: <https://example.org/>
SELECT ?p WHERE {
  ?p a ex:Person .
  FILTER NOT EXISTS { ?p ex:email ?e }
}",
    ),
    // -- MINUS -----------------------------------------------------------------
    (
        "minus_email_holders",
        "PREFIX ex: <https://example.org/>
SELECT ?p WHERE {
  ?p a ex:Person .
  MINUS { ?p ex:email ?e }
}",
    ),
    // -- BIND chains ------------------------------------------------------------
    (
        "bind_chain_numeric",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?a ?doubled ?bucket WHERE {
  ?p ex:age ?a .
  BIND(?a * 2 AS ?doubled)
  BIND(?doubled + 1 AS ?bucket)
}",
    ),
    (
        "bind_chain_string",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?n ?upper ?greeting WHERE {
  ?p ex:name ?n .
  BIND(UCASE(?n) AS ?upper)
  BIND(CONCAT(\"Hello, \", ?upper) AS ?greeting)
}",
    ),
    // -- GROUP BY / aggregates ----------------------------------------------------
    (
        "group_by_all_aggregates",
        "PREFIX ex: <https://example.org/>
SELECT ?d
  (COUNT(?p) AS ?n)
  (COUNT(DISTINCT ?a) AS ?distinctAges)
  (SUM(?a) AS ?sum)
  (AVG(?a) AS ?avg)
  (MIN(?a) AS ?min)
  (MAX(?a) AS ?max)
  (SAMPLE(?n2) AS ?sample)
  (GROUP_CONCAT(?n2; separator=\",\") AS ?names)
WHERE {
  ?p ex:dept ?d .
  ?p ex:age ?a .
  ?p ex:name ?n2 .
} GROUP BY ?d",
    ),
    // -- a SINGLE huge group (within-group chunked partial aggregation) -----------
    (
        "group_concat_single_huge_group",
        // No `GROUP BY` at all: the whole 5000-person input is ONE implicit
        // group (`crate::modifier::eval_group`'s single-implicit-group rule),
        // comfortably over `PARALLEL_MIN_ROWS` — the shape a per-group-only
        // fork could never parallelize (there is only one group), and
        // `GROUP_CONCAT` is `OrderDependent`, so this is the corpus case that
        // actually exercises `crate::parallel::par_chunk_reduce_init`'s
        // combine-in-chunk-order contract through the FULL engine (parse,
        // plan, evaluate), not just the direct `eval()` unit coverage in
        // `crate::modifier`'s own tests.
        "PREFIX ex: <https://example.org/>
SELECT (GROUP_CONCAT(?n; separator=\",\") AS ?names) (MAX(?a) AS ?maxAge) WHERE {
  ?p ex:name ?n .
  ?p ex:age ?a .
}",
    ),
    // -- ORDER BY + LIMIT ---------------------------------------------------------
    (
        "order_by_desc_limit",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?a WHERE {
  ?p ex:age ?a .
} ORDER BY DESC(?a) ?p LIMIT 10",
    ),
    // -- subquery / nested pattern -------------------------------------------------
    (
        "nested_subquery",
        "PREFIX ex: <https://example.org/>
SELECT ?d ?maxAge WHERE {
  ?p ex:dept ?d .
  {
    SELECT ?d2 (MAX(?a) AS ?maxAge) WHERE {
      ?q ex:dept ?d2 .
      ?q ex:age ?a .
    } GROUP BY ?d2
  }
  FILTER(?d = ?d2)
}",
    ),
    // -- ASK -------------------------------------------------------------------
    (
        "ask_email_exists",
        "PREFIX ex: <https://example.org/>
ASK { ?p ex:email ?e }",
    ),
    // -- CONSTRUCT ---------------------------------------------------------------
    (
        "construct_dept_edges",
        "PREFIX ex: <https://example.org/>
CONSTRUCT { ?p ex:inDept ?d } WHERE {
  ?p ex:dept ?d .
  ?p ex:age ?a .
  FILTER(?a > 55)
}",
    ),
    // -- the 7 benches/query_eval.rs shapes, verbatim -----------------------------
    (
        "bench_a_selective_join",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?n ?a WHERE {
  ?p ex:email ?e .
  ?p ex:name ?n .
  ?p ex:age ?a .
  ?p ex:city ex:city20 .
}",
    ),
    (
        "bench_b_scan_filter",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?n WHERE {
  ?p ex:name ?n .
  ?p ex:age ?a .
  FILTER(REGEX(?n, \"^Name1[0-9][0-9]2$\") && ?a > 40)
}",
    ),
    (
        "bench_c_optional_heavy",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?e ?l ?f WHERE {
  ?p a ex:Person .
  OPTIONAL { ?p ex:email ?e }
  OPTIONAL { ?p ex:label ?l }
  OPTIONAL { ?p ex:knows ?f }
}",
    ),
    (
        "bench_d_union_4",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?v WHERE {
  { ?p ex:name ?v } UNION { ?p ex:label ?v }
  UNION { ?p ex:email ?v } UNION { ?p ex:dept ?v }
}",
    ),
    (
        "bench_e_group_aggregate",
        "PREFIX ex: <https://example.org/>
SELECT ?d (COUNT(?p) AS ?n) (AVG(?a) AS ?avg) (MAX(?a) AS ?max) WHERE {
  ?p ex:dept ?d .
  ?p ex:age ?a .
} GROUP BY ?d",
    ),
    (
        "bench_f_path_transitive",
        "PREFIX ex: <https://example.org/>
SELECT ?e WHERE { ?e ex:reportsTo+ ex:person0 }",
    ),
    (
        "bench_g_order_by_limit",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?a WHERE {
  ?p ex:age ?a .
} ORDER BY DESC(?a) ?p LIMIT 10",
    ),
];

/// Evaluate `query` against `engine`/`ds` under the forced threshold `force`
/// (see [`force_parallel_for_test`]), returning the raw [`SparqlResult`]. The
/// guard is scoped to this call so the override never leaks into the next
/// query in the corpus loop.
fn eval_forced(
    engine: &NativeSparqlEngine,
    ds: &Arc<RdfDataset>,
    query: &str,
    force: bool,
) -> SparqlResult {
    let _guard = force_parallel_for_test(force);
    engine
        .query(
            ds,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .unwrap_or_else(|e| panic!("query evaluation failed: {e:?}\nquery:\n{query}"))
}

/// Materialize a [`SparqlResult`] into a form comparable with plain
/// `assert_eq!`/`PartialEq`, canonicalizing whichever payload the result
/// variant carries: SELECT compares `(variables, rows)` (rows in the exact
/// row order the engine returned — this is the whole point, since fork-join
/// order-stability is the property under test), ASK compares the bare bool,
/// and CONSTRUCT/DESCRIBE compare canonical N-Quads (RDFC-1.0 blank-node
/// canonicalization; `TermId`s from the two independently-evaluated queries
/// are never expected to match, only the abstract graph they denote).
#[derive(Debug, PartialEq)]
enum Comparable {
    Solutions {
        variables: Vec<String>,
        rows: Vec<Vec<Option<purrdf_core::TermValue>>>,
    },
    Boolean(bool),
    Graph {
        nquads: String,
    },
}

fn comparable(result: &SparqlResult) -> Comparable {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => Comparable::Solutions {
            variables: variables.clone(),
            rows: rows.clone(),
        },
        SparqlResult::Boolean(b) => Comparable::Boolean(*b),
        SparqlResult::Graph(ds) => Comparable::Graph {
            nquads: canonicalize(ds).nquads,
        },
    }
}

/// **The comprehensive determinism gate.** For every `(name, query)` in
/// [`CORPUS`], evaluate it twice against the *same* engine + dataset — once
/// forced parallel, once forced sequential — and assert the results are
/// byte-identical (same variables, same rows, same order; ASK bool equal;
/// CONSTRUCT canonical N-Quads equal). Same engine on both sides so the plan
/// cache and BGP order cache are identical across the two runs — the forced
/// threshold is the *only* variable.
#[test]
fn parallel_paths_are_byte_identical_across_the_corpus() {
    let ds = people_dataset();
    let engine = NativeSparqlEngine::new();

    for &(name, query) in CORPUS {
        let parallel = eval_forced(&engine, &ds, query, true);
        let sequential = eval_forced(&engine, &ds, query, false);

        let parallel_cmp = comparable(&parallel);
        let sequential_cmp = comparable(&sequential);

        assert_eq!(
            parallel_cmp, sequential_cmp,
            "query `{name}` diverged between forced-parallel and forced-sequential evaluation:\n{query}"
        );

        // Every case must do real work: an accidentally-empty result on both
        // sides would make the byte-identity assertion above vacuous.
        let is_nonempty = match &parallel {
            SparqlResult::Solutions { rows, .. } => !rows.is_empty(),
            SparqlResult::Boolean(_) => true,
            SparqlResult::Graph(ds) => ds.quad_count() > 0,
        };
        assert!(
            is_nonempty,
            "query `{name}` returned an empty/no-op result on both paths — \
             the determinism assertion above would be vacuous"
        );
    }
}

/// **Default-pool instantiation test** (deliberately NOT using the force
/// seam): evaluate a query whose hot BGP scan (`ex:knows`, `3 * PEOPLE` rows,
/// see [`gate_dataset_crosses_the_parallel_threshold`]) genuinely exceeds
/// [`PARALLEL_MIN_ROWS`] under the real, default rayon global pool. The
/// byte-identity test above proves order-stability between the two forced
/// paths; this test proves the parallel path actually *compiles and runs*
/// under live rayon scheduling (real `Send`/`Sync` bounds honored, no forced
/// override), by checking the returned rows are exactly what the dataset
/// construction guarantees.
#[test]
fn default_pool_runs_a_genuinely_parallel_query() {
    let ds = people_dataset();
    let engine = NativeSparqlEngine::new();

    let query = "PREFIX ex: <https://example.org/>
SELECT ?p ?f WHERE {
  ?p ex:knows ?f .
}";
    let result = engine
        .query(
            &ds,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("default-pool query evaluates");

    match result {
        SparqlResult::Solutions { rows, .. } => {
            assert_eq!(
                rows.len(),
                3 * PEOPLE,
                "expected exactly 3 * PEOPLE `knows` edges under the default rayon pool"
            );
        }
        other => panic!("expected Solutions, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The property-function corpus case
// ---------------------------------------------------------------------------

/// A `Stable` relation that MINTS a term absent from the dataset: `?p ex:tagOf ?tag`
/// pairs a person with a freshly-built literal derived from their IRI.
///
/// Minting is what makes this case load-bearing for the gate. A minted value is not in
/// the dataset, so its cell is a [`crate::scratch::SolutionTerm::Computed`] id into the
/// *evaluating context's* scratch table — and a forked worker has its own clone of that
/// table. If a worker's minted row escaped without being re-interned against the parent
/// ([`crate::parallel::portable_row`] / `reintern_minted_row`), the id would resolve to
/// a different value on the parallel path than on the sequential one. Nothing in the
/// dataset can make that divergence visible; only a relation that mints can.
#[derive(Debug)]
struct TagRelation {
    modes: [purrdf_core::binding_pattern::BindingPattern; 1],
}

impl TagRelation {
    fn new() -> Self {
        Self {
            modes: [crate::property_fn::PfArity::new(1, 1).all_free_mode()],
        }
    }
}

impl crate::property_fn::PropertyFunction for TagRelation {
    fn volatility(&self) -> crate::user_fn::Volatility {
        crate::user_fn::Volatility::Stable
    }

    fn arity(&self) -> crate::property_fn::PfArity {
        crate::property_fn::PfArity::new(1, 1)
    }

    fn modes(&self) -> &[purrdf_core::binding_pattern::BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: purrdf_core::binding_pattern::BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        args: &crate::property_fn::PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn crate::property_fn::PfCursor>, crate::EvalError> {
        let rows = match args.get(0) {
            Some(purrdf_core::TermValue::Iri(subject)) => {
                let tag = purrdf_core::TermValue::Literal {
                    lexical_form: format!("tag:{subject}"),
                    datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    language: None,
                    direction: None,
                };
                vec![vec![purrdf_core::TermValue::Iri(subject.clone()), tag]]
            }
            _ => Vec::new(),
        };
        Ok(Box::new(TagCursor { rows, next: 0 }))
    }
}

struct TagCursor {
    rows: Vec<crate::property_fn::PfRow>,
    next: usize,
}

impl crate::property_fn::PfCursor for TagCursor {
    fn next(&mut self) -> Result<Option<crate::property_fn::PfRow>, crate::EvalError> {
        let row = self.rows.get(self.next).cloned();
        self.next += 1;
        Ok(row)
    }
}

/// (name, query text) corpus for the property-function seam, evaluated with a registry
/// injected. Every case invokes a relation that mints, so the byte-identity assertion
/// below is about values that exist only in the evaluating context's scratch space.
const PF_CORPUS: &[(&str, &str)] = &[
    (
        "pf_minting_join",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?tag WHERE {
  ?p ex:city ex:city20 .
  ?p <https://example.org/pf/tagOf> ?tag .
}",
    ),
    (
        "pf_minting_union_branches",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?tag WHERE {
  { ?p ex:city ex:city20 . ?p <https://example.org/pf/tagOf> ?tag }
  UNION
  { ?p ex:city ex:city21 . ?p <https://example.org/pf/tagOf> ?tag }
}",
    ),
    (
        "pf_minting_written_before_its_data_pattern",
        // Textual order puts the call first; the prepare-time feasibility ordering
        // schedules the data pattern ahead of it, and the answer must be the same on
        // both scheduling paths.
        "PREFIX ex: <https://example.org/>
SELECT ?p ?tag WHERE {
  ?p <https://example.org/pf/tagOf> ?tag .
  ?p ex:city ex:city20 .
}",
    ),
    (
        "pf_minting_filtered_and_ordered",
        "PREFIX ex: <https://example.org/>
SELECT ?p ?tag WHERE {
  ?p ex:city ex:city20 .
  ?p <https://example.org/pf/tagOf> ?tag .
  FILTER(STRSTARTS(STR(?tag), \"tag:\"))
} ORDER BY ?tag LIMIT 25",
    ),
];

/// The determinism gate for a **term-minting property function**: every case in
/// [`PF_CORPUS`], run twice against the same engine and dataset with the same registry
/// — once forced parallel, once forced sequential — must be byte-identical.
#[test]
fn property_function_paths_are_byte_identical_across_the_corpus() {
    let ds = people_dataset();
    let engine = NativeSparqlEngine::new();
    let mut registry = crate::property_fn::PropertyFunctionRegistry::new();
    registry.register("https://example.org/pf/tagOf", Arc::new(TagRelation::new()));

    for &(name, query) in PF_CORPUS {
        let run = |force: bool| {
            let _guard = force_parallel_for_test(force);
            engine
                .query_with_property_functions(
                    &ds,
                    SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    &registry,
                )
                .unwrap_or_else(|e| panic!("query `{name}` failed: {e:?}\n{query}"))
        };
        let parallel = run(true);
        let sequential = run(false);
        assert_eq!(
            comparable(&parallel),
            comparable(&sequential),
            "query `{name}` diverged between forced-parallel and forced-sequential \
             evaluation:\n{query}"
        );
        let SparqlResult::Solutions { rows, .. } = &parallel else {
            panic!("query `{name}` must return solutions");
        };
        assert!(
            !rows.is_empty(),
            "query `{name}` returned no rows on both paths — the assertion above would \
             be vacuous"
        );
        assert!(
            rows.iter().all(|row| matches!(
                row.get(1),
                Some(Some(purrdf_core::TermValue::Literal { lexical_form, .. }))
                    if lexical_form.starts_with("tag:")
            )),
            "every row must carry the relation's MINTED tag, resolved to the value the \
             relation produced"
        );
    }
}

// ---------------------------------------------------------------------------
// Within-group chunked partial aggregation: a large single-group CUSTOM
// aggregate, through the FULL engine (not the direct `eval()` unit coverage
// in `crate::modifier`'s own tests).
// ---------------------------------------------------------------------------

/// A minimal `OrderDependent` custom aggregate: collects each row's argument
/// lexical form into an ordered list, joined by `;` at `finish`. `combine`
/// appends the LATER partial's items after the earlier one's, never
/// reordered — the exact shape `crate::agg_fn`'s own module docs use as the
/// canonical example of a fold whose answer depends on row order, so this is
/// the sharpest possible probe of
/// [`crate::parallel::par_chunk_reduce_init`]'s chunk-order `combine`
/// contract: a commutative custom aggregate could pass this gate by
/// accident (any merge order gives the same answer); this one cannot.
#[derive(Debug)]
struct ListCollectorAggregate;

struct ListCollectorAccumulator {
    items: Vec<String>,
}

impl crate::agg_fn::AggregateAccumulator for ListCollectorAccumulator {
    fn step(&mut self, args: &[purrdf_core::TermValue]) -> Result<(), crate::error::EvalError> {
        if let Some(purrdf_core::TermValue::Literal { lexical_form, .. }) = args.first() {
            self.items.push(lexical_form.clone());
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn crate::agg_fn::AggregateAccumulator>) {
        // No downcasting seam is required of the trait: finish the other
        // partial through the same public surface a caller has, and re-derive
        // its item list from that — the same pattern `crate::agg_fn`'s and
        // `crate::modifier`'s own test fixtures use.
        if let Ok(Some(purrdf_core::TermValue::Literal { lexical_form, .. })) = other.finish()
            && !lexical_form.is_empty()
        {
            self.items
                .extend(lexical_form.split(';').map(str::to_owned));
        }
    }

    fn finish(self: Box<Self>) -> Result<Option<purrdf_core::TermValue>, crate::error::EvalError> {
        Ok(Some(purrdf_core::TermValue::typed_literal(
            self.items.join(";"),
            "http://www.w3.org/2001/XMLSchema#string",
        )))
    }
}

impl crate::agg_fn::CustomAggregate for ListCollectorAggregate {
    fn arity(&self) -> crate::user_fn::Arity {
        crate::user_fn::Arity::Exact(1)
    }
    fn volatility(&self) -> crate::user_fn::Volatility {
        crate::user_fn::Volatility::Stable
    }
    fn algebraic_class(&self) -> crate::agg_fn::AlgebraicClass {
        crate::agg_fn::AlgebraicClass::OrderDependent
    }
    fn state_bound(&self) -> u64 {
        256
    }
    fn init(&self) -> Box<dyn crate::agg_fn::AggregateAccumulator> {
        Box::new(ListCollectorAccumulator { items: Vec::new() })
    }
}

/// A large single-group (no `GROUP BY`, 5000-row implicit group — well over
/// [`PARALLEL_MIN_ROWS`]) `OrderDependent` CUSTOM aggregate, run through the
/// full engine (`query_with_options_view`, exercising parse + plan-time
/// admission + evaluation, not just the direct `eval()` call
/// `crate::modifier`'s own within-group tests use), must agree byte-for-byte
/// between forced-parallel and forced-sequential.
#[test]
fn custom_aggregate_single_huge_group_parallel_and_sequential_agree() {
    let ds = people_dataset();
    let engine = NativeSparqlEngine::new();
    let mut registry = crate::agg_fn::AggregateRegistry::new();
    registry.register(
        "https://example.org/agg/listCollector",
        Arc::new(ListCollectorAggregate),
    );

    let query = "PREFIX ex: <https://example.org/>
SELECT (AGG(<https://example.org/agg/listCollector>, ?n) AS ?names) WHERE {
  ?p ex:name ?n .
}";

    let run = |force: bool| {
        let _guard = force_parallel_for_test(force);
        engine
            .query_with_options_view(
                &*ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                crate::engine::QueryOptions {
                    aggregates: Some(&registry),
                    ..crate::engine::QueryOptions::EMPTY
                },
            )
            .unwrap_or_else(|e| panic!("custom-aggregate query failed: {e:?}\n{query}"))
    };

    let parallel = run(true);
    let sequential = run(false);
    assert_eq!(
        comparable(&parallel),
        comparable(&sequential),
        "custom aggregate diverged between forced-parallel and forced-sequential evaluation"
    );

    let SparqlResult::Solutions { rows, .. } = &parallel else {
        panic!("expected solutions");
    };
    assert_eq!(rows.len(), 1, "one implicit group, no GROUP BY");
    let Some(Some(purrdf_core::TermValue::Literal { lexical_form, .. })) = rows[0].first() else {
        panic!(
            "expected a literal in the single result row, got {:?}",
            rows[0]
        );
    };
    assert_eq!(
        lexical_form.split(';').count(),
        PEOPLE,
        "every person's name must have been collected exactly once, in row order"
    );
}
