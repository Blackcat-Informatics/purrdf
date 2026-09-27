// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL 1.2 Core §6.7.1.4: "Implementations that do perform such checks (e.g., when the
//! shapes graph is installed in the system, or before or during the validation) SHOULD use
//! the property sh:shapesGraphWellFormed to inform the consumer of the validation report
//! about this fact."
//!
//! Every report a validation produces states it. It is `true` for a shapes graph that met
//! every syntax rule, and `false` for the one ill-formedness the approved W3C suite requires
//! a validation of: an empty `sh:in` or `sh:xone` list (`in-minListLength`,
//! `xone-minListLength`). The neighbour of each `false` row differs only in the list having
//! a member, and states `true`.

use std::sync::Arc;

use purrdf_shapes::engine::{PreparedShapes, parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::report::{ConformanceDisallows, ValidationReport};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
    @prefix sh: <http://www.w3.org/ns/shacl#> .\n";

const WELL_FORMED_TRUE: &str = "<http://www.w3.org/ns/shacl#shapesGraphWellFormed> \
    \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>";
const WELL_FORMED_FALSE: &str = "<http://www.w3.org/ns/shacl#shapesGraphWellFormed> \
    \"false\"^^<http://www.w3.org/2001/XMLSchema#boolean>";

/// The report of validating `ex:a` against `constraint`, through the free function and a
/// prepared binding, which must agree.
fn report(constraint: &str) -> ValidationReport {
    let shapes = parse_shapes(
        &format!("{PREFIXES}ex:S a sh:NodeShape ; sh:targetNode ex:a ; {constraint} .\n"),
        None,
    )
    .expect("the shapes graph loads");
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}ex:a ex:p ex:b .\n"), None)
        .expect("data parses");
    let free = validate_dataset_with_shapes_graph(&data, &shapes, None).expect("validates");
    let prepared = PreparedShapes::new(Arc::new(shapes))
        .bind_dataset(&data)
        .expect("binds")
        .validate()
        .expect("validates");
    assert_eq!(
        free.shapes_graph_well_formed, prepared.shapes_graph_well_formed,
        "both routes state the same"
    );
    free
}

#[test]
fn every_report_states_whether_the_shapes_graph_is_well_formed() {
    for (constraint, well_formed) in [
        ("sh:in ( ex:a )", true),
        ("sh:in ( )", false),
        ("sh:xone ( [ sh:nodeKind sh:IRI ] )", true),
        ("sh:xone ( )", false),
        ("sh:nodeKind sh:IRI", true),
    ] {
        let report = report(constraint);
        assert_eq!(
            report.shapes_graph_well_formed,
            Some(well_formed),
            "{constraint}"
        );
        let text = report.to_ntriples();
        let (stated, absent) = if well_formed {
            (WELL_FORMED_TRUE, WELL_FORMED_FALSE)
        } else {
            (WELL_FORMED_FALSE, WELL_FORMED_TRUE)
        };
        assert!(text.contains(stated), "{constraint}: {text}");
        assert!(!text.contains(absent), "{constraint}: {text}");
    }
}

/// A report assembled from results rather than produced by a validation claims nothing.
#[test]
fn a_report_no_validation_produced_states_nothing() {
    let report = ValidationReport::from_results(Vec::new(), ConformanceDisallows::default());
    assert_eq!(report.shapes_graph_well_formed, None);
    assert!(!report.to_ntriples().contains("shapesGraphWellFormed"));
}
