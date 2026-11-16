// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE class-targeted validation workload both bench targets read.
//!
//! Many `sh:targetClass` node shapes, each closed and carrying two property
//! shapes, over subjects spread evenly across the classes. Every subject
//! conforms. This is the shape of workload where a validator that plans a
//! shape's constraints again for every focus node pays per subject what it
//! should pay once per shape: the cost under measurement is the per-focus
//! overhead, not the constraint arms. Public API only, so the identical file
//! measures a base revision.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_rdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_shapes::engine::{parse_shapes, validate_projected_dataset};
use purrdf_shapes::shapes::Shapes;

const EX: &str = "https://example.org/class-targets/";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// A generated dataset, its shapes graph and the number of subjects in it.
pub(crate) struct ClassTargets {
    pub(crate) dataset: Arc<RdfDataset>,
    pub(crate) shapes: Shapes,
    pub(crate) subjects: usize,
}

/// `subjects` conforming subjects over `classes` class-targeted closed shapes.
pub(crate) fn fixture(subjects: usize, classes: usize) -> ClassTargets {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE).unwrap();
    let label = builder.intern_iri(&format!("{EX}label")).unwrap();
    let value = builder.intern_iri(&format!("{EX}value")).unwrap();
    let class_ids: Vec<_> = (0..classes)
        .map(|class| builder.intern_iri(&format!("{EX}Class{class}")).unwrap())
        .collect();
    for subject in 0..subjects {
        let focus = builder
            .intern_iri(&format!("{EX}subject{subject}"))
            .unwrap();
        let text = builder
            .intern_literal(RdfLiteral::simple(format!("subject-{subject}")))
            .unwrap();
        let number = builder
            .intern_literal(RdfLiteral::typed(subject.to_string(), XSD_INTEGER))
            .unwrap();
        builder
            .push_quad(focus, rdf_type, class_ids[subject % classes], None)
            .unwrap();
        builder.push_quad(focus, label, text, None).unwrap();
        builder.push_quad(focus, value, number, None).unwrap();
    }
    let dataset = builder.freeze().expect("class target fixture must freeze");

    let mut turtle = format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
         @prefix ex: <{EX}> .\n"
    );
    for class in 0..classes {
        write!(
            turtle,
            "ex:Shape{class} a sh:NodeShape ;\n\
             \x20   sh:targetClass ex:Class{class} ;\n\
             \x20   sh:closed true ;\n\
             \x20   sh:ignoredProperties ( rdf:type ) ;\n\
             \x20   sh:property [ sh:path ex:label ; sh:minCount 1 ; sh:datatype xsd:string ] ;\n\
             \x20   sh:property [ sh:path ex:value ; sh:datatype xsd:integer ] .\n"
        )
        .unwrap();
    }
    let shapes = parse_shapes(&turtle, None).expect("class target shapes must parse");
    ClassTargets {
        dataset,
        shapes,
        subjects,
    }
}

/// Validate every subject and return how many results there were (none).
pub(crate) fn validate(fixture: &ClassTargets) -> usize {
    let report = validate_projected_dataset(Arc::clone(&fixture.dataset), &fixture.shapes)
        .expect("class target validation must not error");
    assert!(report.conforms, "class target fixture must conform");
    report.results.len()
}
