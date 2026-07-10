// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! RDF 1.2 visualization projection and deterministic SVG export benchmark.
//!
//! Report-only, `cargo bench -p purrdf-rdf --bench viz_projection`. The fixture
//! exercises asserted triples, quoted-only statements, reifiers, annotations,
//! and named graph contexts through the graph-like API used by non-IR callers.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_rdf::TermValue;
use purrdf_rdf::viz::{
    VizGraphInput, VizInputAnnotation, VizInputQuad, VizInputReifier, VizInputStatement, VizSpec,
    VizSvgOptions, project_graph_input, render_graph_input_svg,
};

const ROWS: usize = 2_000;
const EX: &str = "https://example.org/";

fn iri(local: impl std::fmt::Display) -> TermValue {
    TermValue::Iri(format!("{EX}{local}"))
}

fn fixture(rows: usize) -> VizGraphInput {
    let mut input = VizGraphInput::default();
    for idx in 0..rows {
        let subject = iri(format_args!("s{idx}"));
        let object = iri(format_args!("o{}", idx % 256));
        let predicate = format!("{EX}p{}", idx % 16);
        let graph_name = Some(iri(format_args!("g{}", idx % 8)));
        input.quads.push(VizInputQuad {
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: object.clone(),
            graph_name,
        });
        if idx % 5 == 0 {
            let reifier = iri(format_args!("claim{idx}"));
            input.reifiers.push(VizInputReifier {
                reifier: reifier.clone(),
                statement: VizInputStatement {
                    subject,
                    predicate: predicate.clone(),
                    object: object.clone(),
                },
                graph_name: Some(iri("claims")),
            });
            input.annotations.push(VizInputAnnotation {
                reifier,
                predicate: format!("{EX}confidence"),
                object: TermValue::Literal {
                    lexical_form: "0.8".to_owned(),
                    datatype: "http://www.w3.org/2001/XMLSchema#decimal".to_owned(),
                    language: None,
                    direction: None,
                },
                graph_name: Some(iri("provenance")),
            });
        }
    }
    input
}

fn bench_projection(c: &mut Criterion) {
    let input = fixture(ROWS);
    let spec = VizSpec {
        max_statements: ROWS + ROWS / 5,
        max_terms: ROWS * 4,
        ..VizSpec::default()
    };
    let mut group = c.benchmark_group("viz_projection");
    group.throughput(Throughput::Elements(ROWS as u64));
    group.bench_function("graph_input_2k", |bencher| {
        bencher.iter(|| {
            let projection =
                project_graph_input(black_box(&input), black_box(&spec)).expect("project");
            black_box(projection);
        });
    });
    group.finish();
}

fn bench_svg_export(c: &mut Criterion) {
    let input = fixture(ROWS);
    let spec = VizSpec {
        max_statements: ROWS + ROWS / 5,
        max_terms: ROWS * 4,
        ..VizSpec::default()
    };
    let options = VizSvgOptions::default();
    let mut group = c.benchmark_group("viz_svg_export");
    group.sample_size(10);
    group.throughput(Throughput::Elements(ROWS as u64));
    group.bench_function("graph_input_2k", |bencher| {
        bencher.iter(|| {
            let document =
                render_graph_input_svg(black_box(&input), black_box(&spec), black_box(&options))
                    .expect("render svg");
            black_box(document);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_projection, bench_svg_export);
criterion_main!(benches);
