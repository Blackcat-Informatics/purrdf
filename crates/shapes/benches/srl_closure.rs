// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! SPARQL 1.2 RL transitive closure: the recursive rule every rules engine is judged
//! by, through the public text API.
//!
//! * `parse_and_check` — the §7 grammar, §4.2 well-formedness and §4.4 stratification
//!   of the two-rule closure program, independent of data size.
//! * `infer/<n>` — `srl::infer` over a chain of `n` `:link` edges, whose closure holds
//!   `n (n + 1) / 2` `:connected` triples; the per-rule semi-naive delta is what keeps a
//!   round from rejoining the whole closure.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench srl_closure` (the `make bench`
//! lane) — excluded from `make check`. No timing is asserted.

use std::fmt::Write as _;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf::RdfDataset;
use purrdf_shapes::srl::{self, InferOptions};

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

fn bench(c: &mut Criterion) {
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

criterion_group!(benches, bench);
criterion_main!(benches);
