// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The pre-binding rewrite's seed-only fast path against the expression walk it
//! skips, on the same queries and the same binary.
//!
//! A query that reads every pre-bound variable from the seeded row — no `GROUP BY`,
//! sub-`SELECT`, nested `FILTER`/`BIND`, `OPTIONAL`, `MINUS`, `LATERAL`, `SERVICE`,
//! `EXISTS` or property-function call — answers alike with the seed alone, so the
//! rewrite skips the walk there. Each `rewrite/*` pair times the rewrite of one
//! such query both ways: `fast` is the production rewrite, `walk` forces the walk.
//! The `execute/*` arms time the whole per-focus-node run a SHACL scalar call makes
//! (bind, rewrite, evaluate) through a prepared execution, the fast-path shape beside
//! a neighbour the walk must take. Report-only: nothing here asserts a threshold.

use purrdf_core::{RdfDatasetBuilder, TermValue};
use purrdf_sparql_algebra::{GroundTerm, Literal, NamedNode, Query, SparqlParser, Variable};
use purrdf_sparql_eval::fixture::shacl_prebinding_rewrite;
use purrdf_sparql_eval::{InternedOutcome, NativeSparqlEngine, QueryOptions};
use purrdf_testkit::bench::{Bench, bench_main};
use std::hint::black_box;

/// The single-row scalar probe a SHACL node-expression call runs per focus node.
const SCALAR: &str = "SELECT ((CONTAINS(STR(?a0), \"example\") && ?a1 > 0) AS ?result) WHERE {}";

/// A wider fast-path query: a joined triple pattern, a top-level `FILTER` and an
/// ordered projection, all reading the pre-bound values from the seeded row.
const JOINED: &str = "SELECT ?o ((STRLEN(STR(?o)) + ?a1) AS ?n) WHERE { ?a0 <http://example.org/p> ?o \
     FILTER(?a1 >= 0) } ORDER BY ?o";

/// A neighbour the walk must take: the pre-bound value is read inside `OPTIONAL`.
const NEEDS_WALK: &str = "SELECT ?o WHERE { ?a0 <http://example.org/p> ?o OPTIONAL { ?o \
     <http://example.org/q> ?x FILTER(?x = ?a1) } }";

fn probes() -> Vec<(Variable, GroundTerm)> {
    vec![
        (
            Variable::new("a0"),
            GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/s7")),
        ),
        (
            Variable::new("a1"),
            GroundTerm::Literal(Literal::new_typed(
                "3",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
            )),
        ),
    ]
}

fn parse(query: &str) -> Query {
    SparqlParser::new()
        .with_prebound_variables(["a0", "a1"])
        .parse_query(query)
        .expect("benchmark query parses")
}

fn rewrite(c: &mut Bench) {
    let mut group = c.benchmark_group("prebind_seed_fast_path");
    for (name, text) in [("scalar", SCALAR), ("joined", JOINED)] {
        let query = parse(text);
        for (arm, fast) in [("fast", true), ("walk", false)] {
            group.bench_function(format!("rewrite/{name}/{arm}"), |b| {
                b.iter(|| {
                    black_box(shacl_prebinding_rewrite(
                        black_box(query.clone()),
                        probes(),
                        fast,
                    ))
                });
            });
        }
    }
    group.finish();
}

fn execute(c: &mut Bench) {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..16 {
        let s = builder.intern_iri(&format!("http://example.org/s{index}"));
        for object in 0..4 {
            let o = builder.intern_iri(&format!("http://example.org/o{index}-{object}"));
            builder.push_quad(s, p, o, None);
        }
    }
    let data = builder.freeze().expect("valid benchmark dataset");
    let engine = NativeSparqlEngine::new();
    let mut group = c.benchmark_group("prebind_seed_fast_path");
    for (name, text) in [
        ("scalar", SCALAR),
        ("joined", JOINED),
        ("needs_walk", NEEDS_WALK),
    ] {
        let mut execution = engine
            .prepare_execution(text, None, &["a0", "a1"], QueryOptions::EMPTY)
            .expect("prepare execution");
        group.bench_function(format!("execute/{name}"), |b| {
            let mut index = 0_usize;
            b.iter(|| {
                index = (index + 1) % 16;
                execution
                    .bind(0, TermValue::Iri(format!("http://example.org/s{index}")))
                    .expect("bind");
                execution
                    .bind(
                        1,
                        TermValue::Literal {
                            lexical_form: "3".to_owned(),
                            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                            language: None,
                            direction: None,
                        },
                    )
                    .expect("bind");
                engine
                    .execute(
                        &mut execution,
                        black_box(&*data),
                        QueryOptions::EMPTY,
                        |outcome| black_box(matches!(outcome, InternedOutcome::Solutions(_))),
                    )
                    .expect("execute");
            });
        });
    }
    group.finish();
}

/// Run both groups with the harness's command-line configuration.
pub fn benches() {
    let mut criterion = Bench::default().configure_from_args();
    rewrite(&mut criterion);
    execute(&mut criterion);
}
bench_main!(benches);
