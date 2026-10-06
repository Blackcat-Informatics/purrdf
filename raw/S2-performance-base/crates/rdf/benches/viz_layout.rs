// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Renderer-neutral RDF 1.2 projection, scene, and layout benchmark.

use purrdf_core::ir::TermValue;
use purrdf_rdf::viz::{
    VizGraphInput, VizInputAnnotation, VizInputQuad, VizInputReifier, VizInputStatement,
    VizLayoutOptions, VizSpec, build_scene, layout_scene, project_graph_input,
};
use purrdf_testkit::bench::{Bench, bench_group, bench_main, black_box};

#[path = "../tests/support/viz_terms.rs"]
mod viz_terms;
use viz_terms::{EX, iri};

fn input() -> VizGraphInput {
    let mut quads = Vec::new();
    let mut reifiers = Vec::new();
    let mut annotations = Vec::new();
    for index in 0..120 {
        let source = format!("resource-{}", index % 40);
        let target = format!("resource-{}", (index * 7 + 3) % 40);
        let predicate = format!("{EX}relation-{}", index % 9);
        quads.push(VizInputQuad {
            subject: iri(&source),
            predicate: predicate.clone(),
            object: iri(&target),
            graph_name: Some(iri(&format!("graph-{}", index % 4))),
        });
        if index % 5 == 0 {
            let reifier = iri(&format!("claim-{index}"));
            reifiers.push(VizInputReifier {
                reifier: reifier.clone(),
                statement: VizInputStatement {
                    subject: iri(&source),
                    predicate,
                    object: iri(&target),
                },
                graph_name: Some(iri("claims")),
            });
            annotations.push(VizInputAnnotation {
                reifier,
                predicate: format!("{EX}confidence"),
                object: TermValue::Literal {
                    lexical_form: format!("0.{}", index % 10),
                    datatype: "http://www.w3.org/2001/XMLSchema#decimal".to_owned(),
                    language: None,
                    direction: None,
                },
                graph_name: Some(iri("provenance")),
            });
        }
    }
    VizGraphInput {
        quads,
        reifiers,
        annotations,
    }
}

fn benchmark(c: &mut Bench) {
    let input = input();
    let spec = VizSpec {
        max_statements: 500,
        max_terms: 1_500,
        ..VizSpec::default()
    };
    c.bench_function("viz_project_scene_layout_120", |b| {
        b.iter(|| {
            let projection = project_graph_input(black_box(&input), black_box(&spec))
                .expect("benchmark projection");
            let scene =
                build_scene(black_box(&projection), black_box(&spec)).expect("benchmark scene");
            layout_scene(black_box(&scene), black_box(&VizLayoutOptions::default()))
                .expect("benchmark layout")
        });
    });
}

bench_group!(benches, benchmark);
bench_main!(benches);
