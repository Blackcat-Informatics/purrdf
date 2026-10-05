// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only scale instrument for ontology-aware schema compilation.

use std::fmt::Write as _;

use purrdf_shapes::json_schema::{
    Namespaces, SchemaCompileRequest, SchemaSurfaceMode, compile_schema,
};
use purrdf_shapes::shapes::{Shapes, from_dataset};
use purrdf_testkit::bench::{Bench, bench_main, black_box};

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
    ontology: std::sync::Arc<purrdf_rdf::RdfDataset>,
    namespaces: Namespaces,
}

#[derive(Clone, Copy)]
enum Density {
    Sparse,
    Dense,
    /// Four shared, domainless object properties, each class a subclass of
    /// one target and restricted by an existential on each of the four: the
    /// shape whose provenance once grew with the square of the class count.
    SharedRestricted,
    /// Domainless properties plus four anonymous superclass expressions per class:
    /// an existential, a universal, a maximum cardinality and a disjunction of
    /// two minimums, each over the class's own properties.
    Restricted,
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
    let shapes_dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&shapes_turtle, None)
        .expect("benchmark shapes Turtle");
    let shapes = from_dataset(&shapes_dataset).expect("benchmark shapes graph");

    let mut ontology_turtle = String::from(PREFIXES);
    for class in 0..classes {
        let _ = writeln!(ontology_turtle, "ex:Class{class:04} a owl:Class .");
    }
    if matches!(density, Density::SharedRestricted) {
        let _ = writeln!(ontology_turtle, "ex:Target a owl:Class .");
        for class in 0..classes {
            let _ = write!(
                ontology_turtle,
                "ex:Class{class:04} rdfs:subClassOf ex:Target"
            );
            for property in 0..properties {
                let _ = write!(
                    ontology_turtle,
                    " , [ a owl:Restriction ; owl:onProperty ex:property{property:04} ; owl:someValuesFrom ex:Target ]"
                );
            }
            let _ = writeln!(ontology_turtle, " .");
        }
    }
    if matches!(density, Density::Restricted) {
        for class in 0..classes {
            let first = class % properties;
            let second = (class + 1) % properties;
            let _ = writeln!(
                ontology_turtle,
                "ex:Class{class:04} rdfs:subClassOf \
                 [ a owl:Restriction ; owl:onProperty ex:property{first:04} ; owl:someValuesFrom xsd:string ] , \
                 [ a owl:Restriction ; owl:onProperty ex:property{second:04} ; owl:allValuesFrom xsd:string ] , \
                 [ a owl:Restriction ; owl:onProperty ex:property{first:04} ; owl:maxCardinality 2 ] , \
                 [ a owl:Class ; owl:unionOf ( \
                     [ a owl:Restriction ; owl:onProperty ex:property{first:04} ; owl:minCardinality 1 ] \
                     [ a owl:Restriction ; owl:onProperty ex:property{second:04} ; owl:minCardinality 1 ] ) ] ."
            );
        }
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
            Density::Dense | Density::Restricted => {
                let _ = writeln!(
                    ontology_turtle,
                    "ex:property{property:04} a owl:DatatypeProperty ; rdfs:range xsd:string ."
                );
            }
            Density::SharedRestricted => {
                let _ = writeln!(
                    ontology_turtle,
                    "ex:property{property:04} a owl:ObjectProperty ."
                );
            }
        }
    }
    let ontology = purrdf_shapes::text_ingest::parse_turtle_to_dataset(&ontology_turtle, None)
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

fn bench_schema_surface(c: &mut Bench) {
    let shaped = fixture(128, 128, Density::Sparse, true);
    let sparse = fixture(256, 256, Density::Sparse, false);
    let dense = fixture(128, 256, Density::Dense, false);
    let restricted = fixture(128, 256, Density::Restricted, false);
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
    group.bench_function(
        "ontology_restricted_128_classes_256_properties",
        |bencher| {
            bencher.iter(|| {
                let request = SchemaCompileRequest::new(
                    &restricted.shapes,
                    &restricted.namespaces,
                    restricted.ontology.as_ref(),
                    SchemaSurfaceMode::OntologyComplete,
                );
                black_box(compile_schema(&request).expect("restricted ontology compilation"));
            });
        },
    );
    group.bench_function(
        "ontology_shared_restrictions_1000_classes_4_properties",
        |bencher| {
            // Built here, untimed, so that only this lane pays for its
            // thousand-class fixture.
            let shared = fixture(1_000, 4, Density::SharedRestricted, false);
            bencher.iter(|| {
                let request = SchemaCompileRequest::new(
                    &shared.shapes,
                    &shared.namespaces,
                    shared.ontology.as_ref(),
                    SchemaSurfaceMode::OntologyComplete,
                );
                black_box(compile_schema(&request).expect("shared restriction compilation"));
            });
        },
    );
    group.finish();
}

/// Run the schema-surface benchmark group.
pub fn benches() {
    let mut criterion = Bench::default().configure_from_args();
    bench_schema_surface(&mut criterion);
}

bench_main!(benches);
