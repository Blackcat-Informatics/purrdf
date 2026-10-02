// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Time to refusal of runaway rule sets under the DEFAULT rule-evaluation limits
//! (`purrdf_shapes::RuleOptions`: 16,384 term-generating rounds, and a generated-term
//! budget of `max(65,536, 4 × N)`), and the completion time of the deep, terminating
//! neighbours the defaults must admit.
//!
//! * `refuse/fresh_iri` — the first-party `err-diverging-fresh-term` case: a SHACL
//!   SPARQL rule minting a strictly longer `ex:Counter` IRI every pass.
//! * `refuse/concat` — a SHACL SPARQL rule extending a string every pass.
//! * `refuse/counter` — a SHACL SPARQL rule stepping an unbounded counter: the round
//!   limit, reached one new term per round.
//! * `refuse/doubling` — two SHACL SPARQL rules extending every string by one of two
//!   letters: the generated-term budget, reached as the strings double.
//! * `refuse/srl_nest` — a SPARQL 1.2 RL rule nesting its matched triple term every
//!   round.
//! * `refuse/per_focus_counter` — a SHACL SHAPE rule minting one new focus node per
//!   iteration (a counter IRI per step): the round limit, reached with one execution per
//!   iteration because only the new focus node is executed.
//! * `complete/per_focus_chain/<n>` — a SHACL shape rule walking an `n`-link chain one
//!   link per iteration, each through a focus node the iteration before made an instance
//!   of its target class.
//! * `complete/depth/<n>` — a SHACL SPARQL rule counting depth along an `n`-edge chain:
//!   a counter bounded by its data.
//! * `complete/countdown/<n>` — a SHACL SPARQL rule counting down from `n` to 0.
//! * `complete/copy/<n>` — a non-recursive `sh:TripleRule` copying `n` triples: 70,000
//!   holds 140,000 facts, past the `wasm32` default stored-fact limit and inside the
//!   native one.
//! * `complete/closure/<n>` — the transitive closure of an `n`-edge chain as two global
//!   `sh:SPARQLRule`s, one of them linear-recursive: 1,000 edges infer 500,500 triples.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench rules_divergence` (the `make
//! bench` lane) — excluded from `make check`. No timing is asserted.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_rdf::RdfDataset;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{self, parse_shapes};
use purrdf_shapes::rules::{RuleOptions, infer};
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::srl::{self, InferOptions};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_testkit::bench::{Bench, BenchmarkId, bench_group, bench_main, black_box};

const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
";

const FRESH_IRI: &str =
    include_str!("../../../vectors/shacl/af/rules/err-diverging-fresh-term/input.ttl");

const CONCAT: &str = r#"ex:a ex:name "s" .
ex:grow a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?s ex:name ?m } WHERE { ?s ex:name ?n BIND (CONCAT(?n, 'x') AS ?m) }" ."#;

const COUNTER: &str = r#"ex:a ex:n 0 .
ex:count a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?s ex:n ?m } WHERE { ?s ex:n ?n BIND (?n + 1 AS ?m) }" ."#;

const DOUBLING: &str = r#"ex:s ex:name "" .
ex:a a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?s ex:name ?m } WHERE { ?s ex:name ?n BIND (CONCAT(?n, 'a') AS ?m) }" .
ex:b a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?s ex:name ?m } WHERE { ?s ex:name ?n BIND (CONCAT(?n, 'b') AS ?m) }" ."#;

const PER_FOCUS_COUNTER: &str = r#"ex:c1 a ex:C ; ex:n 1 .
ex:Counter a sh:NodeShape ; sh:targetClass ex:C ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """PREFIX ex: <http://example.org/ns#>
CONSTRUCT { ?next a ex:C ; ex:n ?m . $this ex:next ?next . }
WHERE { $this ex:n ?n . BIND (?n + 1 AS ?m)
        BIND (IRI(CONCAT("http://example.org/ns#c", STR(?m))) AS ?next) }""" ] ."#;

const PER_FOCUS_WALK: &str = r#"ex:Walk a sh:NodeShape ; sh:targetClass ex:B ;
  sh:rule [ a sh:SPARQLRule ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:q ?o . ?o a ex:B } WHERE { $this ex:p ?o }" ] .
"#;

/// Chain lengths for the per-focus walk.
const WALKS: &[usize] = &[300, 3_000];

const COUNTDOWN_RULE: &str = r#"ex:count a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?s ex:n ?m } WHERE { ?s ex:n ?n FILTER (?n > 0) BIND (?n - 1 AS ?m) }" ."#;

/// Countdown starts for the deep terminating neighbour.
const COUNTDOWNS: &[usize] = &[300, 10_000];

const DEPTH_RULE: &str = r#"ex:depth a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?y ex:depth ?e } WHERE { ?x ex:depth ?d . ?x ex:next ?y BIND (?d + 1 AS ?e) }" ."#;

/// Chain lengths for the data-bounded neighbour.
const DEPTHS: &[usize] = &[300, 400];

/// The shapes graph and the rule-evaluation data of one self-contained document.
fn load(ttl: &str) -> (Shapes, ShaclData) {
    let shapes = parse_shapes(ttl, None).expect("shapes parse");
    let dataset: Arc<RdfDataset> = parse_turtle_to_dataset(ttl, None).expect("data parses");
    let projected = engine::project_dataset(dataset.as_ref()).expect("projects");
    (
        shapes,
        ShaclData::new(Arc::clone(&projected), projected, None),
    )
}

/// Time to refusal: every input is loaded before the group opens, so the group lives
/// only across its measurements.
fn bench_refuse(c: &mut Bench) {
    let loaded: Vec<(&str, (Shapes, ShaclData))> = [
        ("fresh_iri", FRESH_IRI.to_owned()),
        ("concat", format!("{PREFIXES}{CONCAT}")),
        ("counter", format!("{PREFIXES}{COUNTER}")),
        ("doubling", format!("{PREFIXES}{DOUBLING}")),
        (
            "per_focus_counter",
            format!("{PREFIXES}{PER_FOCUS_COUNTER}"),
        ),
    ]
    .into_iter()
    .map(|(name, ttl)| (name, load(&ttl)))
    .collect();
    let document = srl::parse_and_check(
        "PREFIX : <http://example.org/>\nRULE { ?x :p <<( ?x :p ?y )>> } WHERE { ?x :p ?y }",
        None,
    )
    .expect("checks");
    let base = purrdf_rdf::parse_dataset(
        b"<http://example.org/a> <http://example.org/p> <http://example.org/b> .",
        "text/turtle",
        None,
    )
    .expect("parses");

    let mut refuse = c.benchmark_group("rules_divergence/refuse");
    refuse.sample_size(10);
    for (name, (shapes, data)) in &loaded {
        refuse.bench_function(*name, |b| {
            b.iter(|| {
                infer(black_box(data), shapes, &RuleOptions::default()).expect_err("passes a limit")
            });
        });
    }
    refuse.bench_function("srl_nest", |b| {
        b.iter(|| {
            srl::infer(&document, black_box(&base), &InferOptions::default())
                .expect_err("passes a limit")
        });
    });
    refuse.finish();
}

/// Completion of a data-bounded counter.
fn bench_complete(c: &mut Bench) {
    let loaded: Vec<(usize, (Shapes, ShaclData))> = DEPTHS
        .iter()
        .map(|&length| {
            let mut ttl = format!("{PREFIXES}{DEPTH_RULE}\nex:n0 ex:depth 0 .\n");
            for index in 0..length {
                writeln!(ttl, "ex:n{index} ex:next ex:n{} .", index + 1).expect("write to String");
            }
            (length, load(&ttl))
        })
        .collect();

    let mut complete = c.benchmark_group("rules_divergence/complete/depth");
    complete.sample_size(10);
    for (length, (shapes, data)) in &loaded {
        let length = *length;
        complete.bench_with_input(BenchmarkId::from_parameter(length), data, |b, data| {
            b.iter(|| {
                let inference = infer(black_box(data), shapes, &RuleOptions::default())
                    .expect("a data-bounded counter completes");
                assert_eq!(inference.inferred().len(), length);
                inference
            });
        });
    }
    complete.finish();
}

/// Completion of a countdown bounded by a constant.
fn bench_countdown(c: &mut Bench) {
    let countdowns: Vec<(usize, (Shapes, ShaclData))> = COUNTDOWNS
        .iter()
        .map(|&start| {
            (
                start,
                load(&format!(
                    "{PREFIXES}{COUNTDOWN_RULE}\nex:a ex:n {start} .\n"
                )),
            )
        })
        .collect();
    let mut countdown = c.benchmark_group("rules_divergence/complete/countdown");
    countdown.sample_size(10);
    for (start, (shapes, data)) in &countdowns {
        let start = *start;
        countdown.bench_with_input(BenchmarkId::from_parameter(start), data, |b, data| {
            b.iter(|| {
                let inference = infer(black_box(data), shapes, &RuleOptions::default())
                    .expect("a countdown completes");
                assert_eq!(inference.inferred().len(), start);
                inference
            });
        });
    }
    countdown.finish();
}

/// Copy workload sizes: the second passes the `wasm32` default stored-fact limit.
const COPIES: &[usize] = &[10_000, 70_000];

/// Chain lengths for the transitive closure.
const CLOSURE_CHAINS: &[usize] = &[150, 1_000];

const COPY_RULE: &str = "ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:q ;
            sh:object [ sh:path ex:p ] ] .
";

const CLOSURE_RULES: &str = r#"ex:base a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?x ex:connected ?y } WHERE { ?x ex:link ?y }" .
ex:step a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?x ex:connected ?z } WHERE { ?x ex:connected ?y . ?y ex:link ?z }" .
"#;

/// Completion of the copy workloads the native default limits admit.
fn bench_copy(c: &mut Bench) {
    let copies: Vec<(usize, (Shapes, ShaclData))> = COPIES
        .iter()
        .map(|&count| {
            let mut ttl = format!("{PREFIXES}{COPY_RULE}");
            for index in 0..count {
                writeln!(ttl, "ex:s{index} ex:p ex:o{index} .").expect("write to String");
            }
            (count, load(&ttl))
        })
        .collect();
    let mut copy = c.benchmark_group("rules_divergence/complete/copy");
    copy.sample_size(10);
    for (count, (shapes, data)) in &copies {
        let count = *count;
        copy.bench_with_input(BenchmarkId::from_parameter(count), data, |b, data| {
            b.iter(|| {
                let inference = infer(black_box(data), shapes, &RuleOptions::default())
                    .expect("a copy completes under the default limits");
                assert_eq!(inference.inferred().len(), count);
                inference
            });
        });
    }
    copy.finish();
}

/// Completion of the transitive closure the native default limits admit.
fn bench_closure(c: &mut Bench) {
    let closures: Vec<(usize, (Shapes, ShaclData))> = CLOSURE_CHAINS
        .iter()
        .map(|&length| {
            let mut ttl = format!("{PREFIXES}{CLOSURE_RULES}");
            for index in 0..length {
                writeln!(ttl, "ex:n{index} ex:link ex:n{} .", index + 1).expect("write to String");
            }
            (length, load(&ttl))
        })
        .collect();
    let mut closure = c.benchmark_group("rules_divergence/complete/closure");
    closure.sample_size(10);
    for (length, (shapes, data)) in &closures {
        let length = *length;
        closure.bench_with_input(BenchmarkId::from_parameter(length), data, |b, data| {
            b.iter(|| {
                let inference = infer(black_box(data), shapes, &RuleOptions::default())
                    .expect("a closure completes under the default limits");
                assert_eq!(inference.inferred().len(), length * (length + 1) / 2);
                inference
            });
        });
    }
    closure.finish();
}

/// Completion of a per-focus walk discovering one focus node per iteration.
fn bench_per_focus_walk(c: &mut Bench) {
    let walks: Vec<(usize, (Shapes, ShaclData))> = WALKS
        .iter()
        .map(|&length| {
            let mut ttl = format!("{PREFIXES}{PER_FOCUS_WALK}\nex:n0 a ex:B .\n");
            for index in 0..length {
                writeln!(ttl, "ex:n{index} ex:p ex:n{} .", index + 1).expect("write to String");
            }
            (length, load(&ttl))
        })
        .collect();
    let mut walk = c.benchmark_group("rules_divergence/complete/per_focus_chain");
    walk.sample_size(10);
    for (length, (shapes, data)) in &walks {
        let length = *length;
        walk.bench_with_input(BenchmarkId::from_parameter(length), data, |b, data| {
            b.iter(|| {
                let inference = infer(black_box(data), shapes, &RuleOptions::default())
                    .expect("a walk bounded by its chain completes");
                assert_eq!(inference.inferred().len(), 2 * length);
                inference
            });
        });
    }
    walk.finish();
}

bench_group!(
    benches,
    bench_refuse,
    bench_per_focus_walk,
    bench_complete,
    bench_countdown,
    bench_copy,
    bench_closure
);
bench_main!(benches);
