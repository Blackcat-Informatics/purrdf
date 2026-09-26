// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only instrument for the JSON-LD instance projection.
//!
//! Two data shapes of equal node count: one with no RDF list — where the list
//! index finds no `rdf:rest rdf:nil` and builds nothing — and one whose every
//! node carries a short list, which pays the reference census and the
//! backward walk from each tail once per projection.

use std::fmt::Write as _;

use criterion::{Criterion, black_box, criterion_main};
use purrdf_shapes::instance::project_graph;
use purrdf_shapes::json_schema::Namespaces;

const PREFIXES: &str = r"
@prefix ex: <https://example.org/instance-bench/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

fn dataset(nodes: usize, lists: bool) -> std::sync::Arc<purrdf::RdfDataset> {
    let mut turtle = String::from(PREFIXES);
    for node in 0..nodes {
        let _ = write!(
            turtle,
            "ex:n{node:05} a ex:Item ; ex:name \"item {node}\" ; ex:count {node} ; \
             ex:ratio \"0.{node}\"^^xsd:decimal ; ex:next ex:n{:05}",
            (node + 1) % nodes
        );
        if lists {
            let _ = write!(turtle, " ; ex:members ( {node} \"a\" ex:n{node:05} )");
        }
        turtle.push_str(" .\n");
    }
    purrdf_shapes::text_ingest::parse_turtle_to_dataset(&turtle, None)
        .expect("benchmark data Turtle")
}

fn bench_instance_projection(criterion: &mut Criterion) {
    let namespaces = Namespaces::new(
        "ex",
        &[(
            "ex".to_owned(),
            "https://example.org/instance-bench/".to_owned(),
        )],
    )
    .expect("benchmark namespaces");
    let plain = dataset(2_000, false);
    let listed = dataset(2_000, true);
    let mut group = criterion.benchmark_group("instance_projection");
    group.bench_function("no_lists_2000_nodes", |bencher| {
        bencher.iter(|| black_box(project_graph(&plain, &namespaces)));
    });
    group.bench_function("one_list_per_node_2000_nodes", |bencher| {
        bencher.iter(|| black_box(project_graph(&listed, &namespaces)));
    });
    group.finish();
}

/// Run the instance-projection benchmark group.
pub fn benches() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_instance_projection(&mut criterion);
}

criterion_main!(benches);
