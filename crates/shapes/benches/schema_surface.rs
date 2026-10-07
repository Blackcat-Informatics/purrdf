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
    /// A subclass chain, each class restricted by a minimum cardinality on the
    /// one shared object property: the class closure and the inherited
    /// restrictions grow with the square of the depth.
    Chain,
    /// A binary tree of classes (each a subclass of `(c - 1) / 2`), each
    /// restricted by an existential on each shared object property to a
    /// scattered target class: an ontology the size of the Gene Ontology.
    Tree,
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
    if matches!(density, Density::Chain) {
        for class in 1..classes {
            let _ = writeln!(
                ontology_turtle,
                "ex:Class{class:04} rdfs:subClassOf ex:Class{:04} .",
                class - 1
            );
        }
    }
    if matches!(density, Density::Chain | Density::Tree) {
        for class in 0..classes {
            if matches!(density, Density::Tree) && class > 0 {
                let _ = writeln!(
                    ontology_turtle,
                    "ex:Class{class:04} rdfs:subClassOf ex:Class{:04} .",
                    (class - 1) / 2
                );
            }
            for property in 0..properties {
                let constraint = match density {
                    Density::Chain => format!("owl:minCardinality {}", class % 5),
                    _ => format!(
                        "owl:someValuesFrom ex:Class{:04}",
                        (class * 7_919 + property * 104_729) % classes
                    ),
                };
                let _ = writeln!(
                    ontology_turtle,
                    "ex:Class{class:04} rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:property{property:04} ; {constraint} ] ."
                );
            }
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
            Density::SharedRestricted | Density::Chain | Density::Tree => {
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
            // Built here, untimed, so that only this lane pays for its fixture.
            let restricted = fixture(128, 256, Density::Restricted, false);
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
    // The large ontologies: each fixture is built inside its lane, untimed,
    // so that only that lane pays for it.
    for (lane, classes, properties, density) in [
        (
            "ontology_subclass_chain_4000_classes",
            4_000,
            1,
            Density::Chain,
        ),
        (
            "ontology_shared_restrictions_16000_classes_4_properties",
            16_000,
            4,
            Density::SharedRestricted,
        ),
        (
            "ontology_restricted_tree_50000_classes_2_properties",
            50_000,
            2,
            Density::Tree,
        ),
    ] {
        group.bench_function(lane, |bencher| {
            let large = fixture(classes, properties, density, false);
            bencher.iter(|| {
                let request = SchemaCompileRequest::new(
                    &large.shapes,
                    &large.namespaces,
                    large.ontology.as_ref(),
                    SchemaSurfaceMode::OntologyComplete,
                );
                black_box(compile_schema(&request).expect("large ontology compilation"));
            });
        });
    }
    group.finish();
}

/// Each language emitter over an ontology-complete schema whose classes
/// reference each other through inherited existentials: a 400-class tree, its
/// schema compiled once, untimed. The Pydantic emitter's negation audit once
/// re-walked every reference path, exponential in such a chain.
fn bench_ontology_emitters(c: &mut Bench) {
    let tree = fixture(400, 2, Density::Tree, false);
    let compiled = compile_schema(&SchemaCompileRequest::new(
        &tree.shapes,
        &tree.namespaces,
        tree.ontology.as_ref(),
        SchemaSurfaceMode::OntologyComplete,
    ))
    .expect("tree ontology compilation")
    .compiled;
    let graphql =
        purrdf_shapes::GraphqlConfig::new("Bench", "x", "y", "RdfValue").expect("GraphQL config");
    let typescript =
        purrdf_shapes::TypeScriptConfig::new("bench-types", "x", "y").expect("TypeScript config");
    let pydantic =
        purrdf_shapes::PydanticConfig::new("bench_models", "x", "y").expect("Pydantic config");
    let linkml = purrdf_shapes::LinkmlConfig::new(
        "https://example.org/schema-bench/generated",
        "Bench",
        "x",
        "ex",
        std::collections::BTreeMap::from([
            (
                "ex".to_owned(),
                "https://example.org/schema-bench/".to_owned(),
            ),
            ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
        ]),
    )
    .expect("LinkML config");
    let mut group = c.benchmark_group("ontology_schema_emitters");
    group.sample_size(10);
    group.bench_function("graphql_tree_400_classes", |bencher| {
        bencher
            .iter(|| black_box(purrdf_shapes::emit_graphql(&compiled, &graphql).expect("GraphQL")));
    });
    group.bench_function("typescript_tree_400_classes", |bencher| {
        bencher.iter(|| {
            black_box(purrdf_shapes::emit_typescript(&compiled, &typescript).expect("TypeScript"))
        });
    });
    group.bench_function("pydantic_tree_400_classes", |bencher| {
        bencher.iter(|| {
            black_box(purrdf_shapes::emit_pydantic(&compiled, &pydantic).expect("Pydantic"))
        });
    });
    group.bench_function("linkml_tree_400_classes", |bencher| {
        bencher.iter(|| black_box(purrdf_shapes::emit_linkml(&compiled, &linkml).expect("LinkML")));
    });
    group.finish();
}

/// LinkML import of the emitted package at doubling class counts, each class
/// the domain of at most one of `classes / 16` properties, so the package grows
/// linearly; each package is emitted once, untimed. The rows should grow
/// linearly too: resolving each `$ref`, and each slot or class name, once
/// scanned every definition, which made the import quadratic.
fn bench_linkml_import_scaling(c: &mut Bench) {
    let xsd = |local: &str| format!("http://www.w3.org/2001/XMLSchema#{local}");
    let linkml = purrdf_shapes::LinkmlConfig::new(
        "https://example.org/schema-bench/generated",
        "Bench",
        "x",
        "ex",
        std::collections::BTreeMap::from([
            (
                "ex".to_owned(),
                "https://example.org/schema-bench/".to_owned(),
            ),
            ("linkml".to_owned(), "https://w3id.org/linkml/".to_owned()),
        ]),
    )
    .expect("LinkML config");
    let mut group = c.benchmark_group("linkml_import_scaling");
    group.sample_size(10);
    for classes in [250, 500, 1_000] {
        let sparse = fixture(classes, classes / 16, Density::Sparse, false);
        let compiled = compile_schema(&SchemaCompileRequest::new(
            &sparse.shapes,
            &sparse.namespaces,
            sparse.ontology.as_ref(),
            SchemaSurfaceMode::OntologyComplete,
        ))
        .expect("sparse ontology compilation")
        .compiled;
        let package = purrdf_shapes::emit_linkml(&compiled, &linkml).expect("LinkML");
        let import = purrdf_shapes::SchemaImportConfig::new(
            sparse.namespaces,
            purrdf_shapes::SchemaDatatypeMap::new(
                xsd("string"),
                xsd("boolean"),
                xsd("integer"),
                xsd("decimal"),
                xsd("dateTime"),
                xsd("date"),
                xsd("time"),
                xsd("anyURI"),
            )
            .expect("datatype map"),
        );
        group.bench_function(
            format!("linkml_import_sparse_{classes}_classes"),
            |bencher| {
                bencher.iter(|| {
                    black_box(
                        purrdf_shapes::import_linkml_package(&package, &import)
                            .expect("LinkML import"),
                    )
                });
            },
        );
    }
    group.finish();
}

/// Run the schema-surface benchmark groups.
pub fn benches() {
    let mut criterion = Bench::default().configure_from_args();
    bench_schema_surface(&mut criterion);
    bench_ontology_emitters(&mut criterion);
    bench_linkml_import_scaling(&mut criterion);
}

bench_main!(benches);
