// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ONE shapes graph and data graph both prepared-product bench targets read.
//!
//! The timed target and the allocation target measure the same phases of the same
//! pipeline, and the only way their two sets of numbers describe one workload is
//! for both to construct that workload from the same builders. Two transcriptions
//! of "the same" fixture that drifted by a property shape would leave a reader
//! comparing a latency taken over one graph against a byte count taken over
//! another, with nothing in either log saying so.
//!
//! Everything here is under `example.org`: PurRDF mints no vocabulary IRIs, and a
//! benchmark fixture is no more entitled to invent one than a release build is.
//!
//! # Why the fixture is shaped like this
//!
//! The phases being measured have different sensitivities, so a fixture that
//! exercised only one of them would make the other numbers meaningless:
//!
//! * [`CLASS_COUNT`] node shapes, each with [`PROPERTY_COUNT`] property shapes,
//!   so the AST section, the shape index and the class catalog all have real
//!   population rather than a single entry;
//! * a `sh:targetSubjectsOf` shape and a `sh:targetObjectsOf` shape alongside the
//!   `sh:targetClass` ones, so the target tables are populated by more than one
//!   kind — target resolution is per-binding work, and a fixture with one target
//!   kind would understate it;
//! * [`FOCUS_PER_CLASS`] conforming focus nodes per class in the data graph, so
//!   binding and evaluation have something to do and are not measuring an empty
//!   traversal;
//! * exactly one VIOLATING focus node per class, so [`VIOLATION_COUNT`] results
//!   are produced. A fixture where everything conformed would let an
//!   `eval/validate` figure that reached no constraint at all — a restore that
//!   silently lost every shape, say — look indistinguishable from one that
//!   checked all of them, and the reader of the log would have no way to tell.
//!   Both bench targets check the count before reporting anything.
//!
//! Nothing here consults a clock, a source of randomness, the filesystem or the
//! environment, so every figure either target reports is a figure the same
//! revision reproduces.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_shapes::product::ShapesProfile;
use purrdf_shapes::report::ValidationReport;

/// The fixture's namespace. Caller-supplied, and `example.org` by rule.
const NS: &str = "http://example.org/bench#";

/// How many `sh:targetClass` node shapes the shapes graph declares.
const CLASS_COUNT: usize = 16;

/// How many property shapes each of those node shapes carries.
const PROPERTY_COUNT: usize = 4;

/// How many conforming focus nodes the data graph holds for each class.
const FOCUS_PER_CLASS: usize = 32;

/// How many results validating [`data`] against the shapes graph must produce.
///
/// One `sh:minCount` violation per class, from the one focus node per class that
/// is missing `ex:p0`. Checked by both bench targets: see [`assert_non_vacuous`].
pub(crate) const VIOLATION_COUNT: usize = CLASS_COUNT;

/// The shapes graph, as Turtle source text.
///
/// Built rather than pasted so the three scale constants above are the single
/// place the fixture's size is stated, and a reader who changes one cannot leave
/// the prose describing the other.
pub(crate) fn shapes_source() -> String {
    let mut out = String::with_capacity(8 * 1024);
    out.push_str("@prefix sh:  <http://www.w3.org/ns/shacl#> .\n");
    out.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
    writeln!(out, "@prefix ex:  <{NS}> .\n").expect("writing to a String cannot fail");

    for class in 0..CLASS_COUNT {
        writeln!(out, "ex:Shape{class} a sh:NodeShape ;").expect("writing to a String cannot fail");
        writeln!(out, "    sh:targetClass ex:Class{class} ;")
            .expect("writing to a String cannot fail");
        for property in 0..PROPERTY_COUNT {
            let terminator = if property + 1 == PROPERTY_COUNT {
                "."
            } else {
                ";"
            };
            writeln!(
                out,
                "    sh:property [ sh:path ex:p{property} ; sh:minCount 1 ; sh:maxCount 4 ; \
                 sh:datatype xsd:string ] {terminator}"
            )
            .expect("writing to a String cannot fail");
        }
        out.push('\n');
    }

    out.push_str("ex:SubjectsShape a sh:NodeShape ;\n");
    out.push_str("    sh:targetSubjectsOf ex:linksTo ;\n");
    out.push_str("    sh:property [ sh:path ex:p0 ; sh:minCount 1 ] .\n\n");
    out.push_str("ex:ObjectsShape a sh:NodeShape ;\n");
    out.push_str("    sh:targetObjectsOf ex:linksTo ;\n");
    out.push_str("    sh:nodeKind sh:IRI .\n");
    out
}

/// Parse [`shapes_source`] and derive the reusable preparation: the cold path a
/// prepared product exists to replace.
///
/// A function rather than a cached value, because "cold" is the whole point — the
/// callers that time this need a fresh parse and a fresh class catalog on every
/// call.
pub(crate) fn prepared(source: &str) -> PreparedShapes {
    let shapes = parse_shapes(source, None).expect("the bench shapes graph parses");
    PreparedShapes::new(Arc::new(shapes))
}

/// Encode a preparation as a prepared product under the only profile this build
/// implements.
pub(crate) fn encode(prepared: &PreparedShapes) -> Vec<u8> {
    prepared
        .to_product(&ShapesProfile::CORE)
        .expect("the bench shapes graph is representable as a product")
}

/// The data graph the binding and evaluation phases run against.
///
/// Deterministic in both directions: every conforming focus node conforms on
/// every run, and exactly [`VIOLATION_COUNT`] focus nodes fail, so the report the
/// evaluation phase builds is the same size every time and the figures are not
/// moved around by how much violation text a particular iteration rendered.
pub(crate) fn data() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let links_to = builder.intern_iri(&format!("{NS}linksTo"));
    let properties: Vec<_> = (0..PROPERTY_COUNT)
        .map(|property| builder.intern_iri(&format!("{NS}p{property}")))
        .collect();

    for class in 0..CLASS_COUNT {
        let class_iri = builder.intern_iri(&format!("{NS}Class{class}"));
        for focus in 0..FOCUS_PER_CLASS {
            let subject = builder.intern_iri(&format!("{NS}node-{class}-{focus}"));
            builder.push_quad(subject, rdf_type, class_iri, None);
            for (property, predicate) in properties.iter().enumerate() {
                let value = builder.intern_literal(RdfLiteral::simple(format!(
                    "value-{class}-{focus}-{property}"
                )));
                builder.push_quad(subject, *predicate, value, None);
            }
            // One link per focus node, so the subjects-of and objects-of targets
            // resolve to a real population rather than an empty one.
            let peer = builder.intern_iri(&format!(
                "{NS}node-{class}-{}",
                (focus + 1) % FOCUS_PER_CLASS
            ));
            builder.push_quad(subject, links_to, peer, None);
        }

        // One focus node per class that is missing `ex:p0` and so trips that
        // property shape's `sh:minCount`. It is deliberately outside the
        // `ex:linksTo` chain, so it is targeted by its class shape only and
        // contributes exactly one result.
        let violating = builder.intern_iri(&format!("{NS}missing-p0-{class}"));
        builder.push_quad(violating, rdf_type, class_iri, None);
        for (property, predicate) in properties.iter().enumerate().skip(1) {
            let value =
                builder.intern_literal(RdfLiteral::simple(format!("value-{class}-bad-{property}")));
            builder.push_quad(violating, *predicate, value, None);
        }
    }
    builder.freeze().expect("freeze the bench data graph")
}

/// Refuse a report that could not distinguish real evaluation from none at all.
///
/// Called once, outside every measured region, by both bench targets. An
/// `eval/validate` line taken over a run that reached no constraint would be a
/// number with nothing behind it, and nothing in the log would say so.
pub(crate) fn assert_non_vacuous(report: &ValidationReport) {
    assert!(
        !report.conforms,
        "the bench data graph must not conform: its {VIOLATION_COUNT} violating focus nodes are \
         what prove the evaluation phase reached the constraints at all"
    );
    assert_eq!(
        report.results.len(),
        VIOLATION_COUNT,
        "the bench data graph must produce exactly {VIOLATION_COUNT} results, one per class; a \
         different count means the fixture and the shapes graph have drifted apart and the \
         reported figures describe a workload nobody specified"
    );
}
