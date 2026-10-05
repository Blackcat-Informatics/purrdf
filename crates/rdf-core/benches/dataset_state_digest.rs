// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only native construction costs for the complete-state identity.
//! Fixtures are built before timing; no speedup or timing threshold is asserted.

use std::sync::Arc;

use purrdf_core::{BlankScope, DatasetStateDigest, RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

fn cases() -> Vec<(&'static str, Arc<RdfDataset>)> {
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let mut ground = RdfDatasetBuilder::new();
    let predicate = ground.intern_iri("http://example.org/p");
    let object = ground.intern_literal(RdfLiteral::simple("exact ground value"));
    for index in 0..64 {
        let subject = ground.intern_iri(&format!("http://example.org/s{index}"));
        ground.push_quad(subject, predicate, object, None);
    }
    let mut asymmetric = RdfDatasetBuilder::new();
    let predicate = asymmetric.intern_iri("http://example.org/p");
    let mut previous = asymmetric.intern_blank("b0", BlankScope::DEFAULT);
    for index in 1..32 {
        let next = asymmetric.intern_blank(&format!("b{index}"), BlankScope::DEFAULT);
        asymmetric.push_quad(previous, predicate, next, None);
        previous = next;
    }
    let mut symmetric = RdfDatasetBuilder::new();
    for index in 0..32 {
        let name = symmetric.intern_blank(&format!("g{index}"), BlankScope::DEFAULT);
        symmetric.declare_named_graph(name);
    }
    let mut composite = RdfDatasetBuilder::new();
    let subject = composite.intern_blank("shared", BlankScope::DEFAULT);
    let predicate = composite.intern_iri("http://example.org/p");
    let object = composite.intern_literal(RdfLiteral::typed(
        "[ _:shared, [ _:shared ], \"text\" ]",
        purrdf_cdt::CDT_LIST,
    ));
    composite.push_quad(subject, predicate, object, Some(subject));
    vec![
        ("empty_default", empty),
        ("ground_64_rows", ground.freeze().unwrap()),
        ("asymmetric_32_blanks", asymmetric.freeze().unwrap()),
        ("interchangeable_32_graphs", symmetric.freeze().unwrap()),
        ("cdt_shared_graph", composite.freeze().unwrap()),
    ]
}

fn bench_state(c: &mut Bench) {
    let mut group = c.benchmark_group("dataset_state_digest");
    for (name, dataset) in cases() {
        DatasetStateDigest::from_view(&dataset).expect("every measured fixture is admitted");
        group.bench_function(name, |bencher| {
            bencher.iter(|| black_box(DatasetStateDigest::from_view(black_box(&dataset)).unwrap()));
        });
    }
    group.finish();
}

bench_group!(benches, bench_state);
bench_main!(benches);
