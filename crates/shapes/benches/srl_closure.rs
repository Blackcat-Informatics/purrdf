// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! SPARQL 1.2 RL transitive closure: the recursive rule every rules engine is judged
//! by, through the public text API.
//!
//! * `parse_and_check` — the §7 grammar, §4.2 well-formedness and §4.4 stratification
//!   of the two-rule closure program, independent of data size.
//! * `infer/<n>` — `srl::infer` over a chain of `n` `:link` edges, whose closure holds
//!   `n (n + 1) / 2` `:connected` triples; the per-rule semi-naive delta is what keeps a
//!   round from rejoining the whole closure.
//! * `infer_linear/<n>` — the LINEAR closure (`?x :connected ?y . ?y :link ?z`), which
//!   takes one round per edge. Its `:link` atom gains no row after the first round, so a
//!   round skips the decomposition anchored at it rather than enumerating the whole
//!   closure first; 1,000 edges (500,500 triples) is past the `wasm32` default
//!   stored-fact limit and inside the native one.
//!
//! * `shacl_per_focus/<n>` — the same linear closure as a SHACL SHAPE rule, executed once
//!   per focus node (every node with a `:link`): each iteration re-executes it only for
//!   the focus nodes whose `:connected` triples the iteration before extended, not for
//!   every focus node.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench srl_closure` (the `make bench`
//! lane) — excluded from `make check`. No timing is asserted.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{self, parse_shapes};
use purrdf_shapes::rules::{RuleOptions, infer};
use purrdf_shapes::srl::{self, InferOptions};
use purrdf_testkit::bench::{Bench, BenchmarkId, Throughput, bench_group, bench_main, black_box};

const RULES: &str = "PREFIX : <https://example.org/srl-bench/>
RULE { ?x :connected ?y } WHERE { ?x :link ?y }
RULE { ?x :connected ?z } WHERE { ?x :connected ?y . ?y :connected ?z }
";

/// Chain lengths.
const CHAINS: &[usize] = &[16, 64, 128];

/// A chain `n0 :link n1 :link … n{len}`.
fn chain(len: usize) -> Arc<RdfDataset> {
    let mut text = String::from("@prefix : <https://example.org/srl-bench/> .\n");
    for index in 0..len {
        writeln!(text, ":n{index} :link :n{} .", index + 1).expect("write to String");
    }
    purrdf::parse_dataset(text.as_bytes(), "text/turtle", None).expect("the chain parses")
}

fn bench(c: &mut Bench) {
    c.bench_function("srl_closure/parse_and_check", |b| {
        b.iter(|| srl::parse_and_check(black_box(RULES), None).expect("checks"));
    });
    let document = srl::parse_and_check(RULES, None).expect("checks");
    let mut group = c.benchmark_group("srl_closure/infer");
    group.sample_size(10);
    for &len in CHAINS {
        let data = chain(len);
        group.throughput(Throughput::Elements((len * (len + 1) / 2) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &data, |b, data| {
            b.iter(|| {
                let inference = srl::infer(&document, black_box(data), &InferOptions::default())
                    .expect("evaluates");
                assert_eq!(inference.inferred().len(), len * (len + 1) / 2);
                inference
            });
        });
    }
    group.finish();
}

const LINEAR_RULES: &str = "PREFIX : <https://example.org/srl-bench/>
RULE { ?x :connected ?y } WHERE { ?x :link ?y }
RULE { ?x :connected ?z } WHERE { ?x :connected ?y . ?y :link ?z }
";

/// Chain lengths for the linear closure.
const LINEAR_CHAINS: &[usize] = &[128, 1_000];

fn bench_linear(c: &mut Bench) {
    let document = srl::parse_and_check(LINEAR_RULES, None).expect("checks");
    let mut group = c.benchmark_group("srl_closure/infer_linear");
    group.sample_size(10);
    for &len in LINEAR_CHAINS {
        let data = chain(len);
        group.throughput(Throughput::Elements((len * (len + 1) / 2) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &data, |b, data| {
            b.iter(|| {
                let inference = srl::infer(&document, black_box(data), &InferOptions::default())
                    .expect("evaluates under the default limits");
                assert_eq!(inference.inferred().len(), len * (len + 1) / 2);
                inference
            });
        });
    }
    group.finish();
}

const PER_FOCUS_RULES: &str = r#"@prefix : <https://example.org/srl-bench/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
:Closure a sh:NodeShape ; sh:targetSubjectsOf :link ;
  sh:rule [ a sh:SPARQLRule ; sh:construct
    "PREFIX : <https://example.org/srl-bench/> CONSTRUCT { $this :connected ?y } WHERE { $this :link ?y }" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:construct
    "PREFIX : <https://example.org/srl-bench/> CONSTRUCT { $this :connected ?z } WHERE { $this :connected ?y . ?y :link ?z }" ] .
"#;

/// Chain lengths for the per-focus closure.
const PER_FOCUS_CHAINS: &[usize] = &[64, 256];

fn bench_per_focus(c: &mut Bench) {
    let shapes = parse_shapes(PER_FOCUS_RULES, None).expect("the shapes parse");
    let mut group = c.benchmark_group("srl_closure/shacl_per_focus");
    group.sample_size(10);
    for &len in PER_FOCUS_CHAINS {
        let projected = engine::project_dataset(chain(len).as_ref()).expect("projects");
        let data = ShaclData::new(Arc::clone(&projected), projected, None);
        group.throughput(Throughput::Elements((len * (len + 1) / 2) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(len), &data, |b, data| {
            b.iter(|| {
                let inference = infer(black_box(data), &shapes, &RuleOptions::default())
                    .expect("evaluates under the default limits");
                assert_eq!(inference.inferred().len(), len * (len + 1) / 2);
                inference
            });
        });
    }
    group.finish();
}

bench_group!(benches, bench, bench_linear, bench_per_focus);
bench_main!(benches);
