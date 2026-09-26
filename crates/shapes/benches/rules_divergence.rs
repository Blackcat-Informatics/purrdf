// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Time to refusal of divergent rule sets under the default term-generating horizon
//! (`purrdf_shapes::rules::TermGeneratingLimit::Horizon`), and the completion time of
//! the convergent neighbour the horizon must admit.
//!
//! * `refuse/fresh_iri` — the first-party `err-diverging-fresh-term` case: a SHACL
//!   SPARQL rule minting a strictly longer `ex:Counter` IRI every pass.
//! * `refuse/concat` — a SHACL SPARQL rule extending a string every pass.
//! * `refuse/counter` — a SHACL SPARQL rule stepping an unbounded counter.
//! * `refuse/srl_nest` — a SPARQL 1.2 RL rule nesting its matched triple term every
//!   round.
//! * `complete/depth/<n>` — a SHACL SPARQL rule counting depth along an `n`-edge chain:
//!   a counter bounded by its data, deeper than the horizon's 256-round floor.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench rules_divergence` (the `make
//! bench` lane) — excluded from `make check`. No timing is asserted.

use std::fmt::Write as _;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use purrdf::RdfDataset;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{self, parse_shapes};
use purrdf_shapes::rules::{RuleOptions, infer};
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::srl::{self, InferOptions};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

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

const DEPTH_RULE: &str = r#"ex:depth a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?y ex:depth ?e } WHERE { ?x ex:depth ?d . ?x ex:next ?y BIND (?d + 1 AS ?e) }" ."#;

/// Chain lengths for the convergent neighbour, each past the 256-round floor.
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
fn bench_refuse(c: &mut Criterion) {
    let loaded: Vec<(&str, (Shapes, ShaclData))> = [
        ("fresh_iri", FRESH_IRI.to_owned()),
        ("concat", format!("{PREFIXES}{CONCAT}")),
        ("counter", format!("{PREFIXES}{COUNTER}")),
    ]
    .into_iter()
    .map(|(name, ttl)| (name, load(&ttl)))
    .collect();
    let document = srl::parse_and_check(
        "PREFIX : <http://example.org/>\nRULE { ?x :p <<( ?x :p ?y )>> } WHERE { ?x :p ?y }",
        None,
    )
    .expect("checks");
    let base = purrdf::parse_dataset(
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
                infer(black_box(data), shapes, &RuleOptions::default()).expect_err("diverges")
            });
        });
    }
    refuse.bench_function("srl_nest", |b| {
        b.iter(|| {
            srl::infer(&document, black_box(&base), &InferOptions::default()).expect_err("diverges")
        });
    });
    refuse.finish();
}

/// Completion of the convergent neighbour: a data-bounded counter deeper than the floor.
fn bench_complete(c: &mut Criterion) {
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

criterion_group!(benches, bench_refuse, bench_complete);
criterion_main!(benches);
