// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Report-only scale instrument for ontology-aware schema compilation.

use std::fmt::Write as _;

use criterion::{Criterion, black_box, criterion_main};
use purrdf_shapes::json_schema::{
    Namespaces, SchemaCompileRequest, SchemaSurfaceMode, compile_schema,
};
use purrdf_shapes::shapes::{Shapes, from_dataset};

const PREFIXES: &str = r"
@prefix ex: <https://example.org/schema-bench/> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

struct Fixture {
    shapes: Shapes,
    ontology: std::sync::Arc<purrdf::RdfDataset>,
    namespaces: Namespaces,
}

#[derive(Clone, Copy)]
enum Density {
    Sparse,
    Dense,
}

fn fixture(
    classes: usize,
    properties: usize,
    density: Density,
    shape_every_class: bool,
) -> Fixture {
    let mut shapes_turtle = String::from(PREFIXES);
    if shape_every_class {
        for class in 0..classes {
            let _ = writeln!(
                shapes_turtle,
                "ex:Shape{class:04} a sh:NodeShape ; sh:targetClass ex:Class{class:04} ."
            );
        }
    }
    let shapes_dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&shapes_turtle)
        .expect("benchmark shapes Turtle");
    let shapes = from_dataset(&shapes_dataset).expect("benchmark shapes graph");

    let mut ontology_turtle = String::from(PREFIXES);
    for class in 0..classes {
        let _ = writeln!(ontology_turtle, "ex:Class{class:04} a owl:Class .");
    }
    for property in 0..properties {
        match density {
            Density::Sparse => {
                let domain = property % classes;
                let _ = writeln!(
                    ontology_turtle,
                    "ex:property{property:04} a owl:DatatypeProperty ; rdfs:domain ex:Class{domain:04} ; rdfs:range xsd:string ."
                );
            }
            Density::Dense => {
                let _ = writeln!(
                    ontology_turtle,
                    "ex:property{property:04} a owl:DatatypeProperty ; rdfs:range xsd:string ."
                );
            }
        }
    }
    let ontology = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&ontology_turtle)
        .expect("benchmark ontology Turtle");
    let namespaces = Namespaces::new(
        "ex",
        &[(
            "ex".to_owned(),
            "https://example.org/schema-bench/".to_owned(),
        )],
    )
    .expect("benchmark namespaces");
    Fixture {
        shapes,
        ontology,
        namespaces,
    }
}

fn bench_schema_surface(c: &mut Criterion) {
    let shaped = fixture(128, 128, Density::Sparse, true);
    let sparse = fixture(256, 256, Density::Sparse, false);
    let dense = fixture(128, 256, Density::Dense, false);
    let mut group = c.benchmark_group("shacl_schema_surface");
    group.sample_size(10);

    group.bench_function("shaped_only_128_classes_128_properties", |bencher| {
        bencher.iter(|| {
            let request = SchemaCompileRequest::new(
                &shaped.shapes,
                &shaped.namespaces,
                shaped.ontology.as_ref(),
                SchemaSurfaceMode::ShapedOnly,
            );
            black_box(compile_schema(&request).expect("shaped-only compilation"));
        });
    });
    group.bench_function("ontology_sparse_256_classes_256_properties", |bencher| {
        bencher.iter(|| {
            let request = SchemaCompileRequest::new(
                &sparse.shapes,
                &sparse.namespaces,
                sparse.ontology.as_ref(),
                SchemaSurfaceMode::OntologyComplete,
            );
            black_box(compile_schema(&request).expect("sparse ontology compilation"));
        });
    });
    group.bench_function("ontology_dense_128_classes_256_properties", |bencher| {
        bencher.iter(|| {
            let request = SchemaCompileRequest::new(
                &dense.shapes,
                &dense.namespaces,
                dense.ontology.as_ref(),
                SchemaSurfaceMode::OntologyComplete,
            );
            black_box(compile_schema(&request).expect("dense ontology compilation"));
        });
    });
    group.finish();
}

/// Run the schema-surface benchmark group.
pub fn benches() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_schema_surface(&mut criterion);
}

criterion_main!(benches);
