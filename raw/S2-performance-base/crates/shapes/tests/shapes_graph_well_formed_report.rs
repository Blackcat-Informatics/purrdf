// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL 1.2 Core §6.7.1.4: "Implementations that do perform such checks (e.g., when the
//! shapes graph is installed in the system, or before or during the validation) SHOULD use
//! the property sh:shapesGraphWellFormed to inform the consumer of the validation report
//! about this fact."
//!
//! Every report a validation produces states it, and states `true`: the loader refuses a
//! shapes graph that breaks a syntax rule it enforces. Appendix A's `in-minListLength` and
//! `xone-minListLength` ("Each such list SHOULD have at least one member") are a MANDATORY
//! DIAGNOSTIC instead (maintainer decision, `docs/design/should-audit.md`): the graph is
//! well-formed, validation proceeds as the approved W3C tests `core/node/in-002`, `in-003`,
//! `xone-002` and `xone-003` require, and `lint` always reports each empty list by rule id.
//! The lint rows below are graded against the W3C test files themselves, each beside a
//! neighbour whose list has a member.

use std::sync::Arc;

use purrdf_shapes::engine::{PreparedShapes, parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::imports::ShapesImports;
use purrdf_shapes::lint::{LintReport, lint};
use purrdf_shapes::report::{ConformanceDisallows, ValidationReport};
use purrdf_shapes::term::Term;
use purrdf_shapes::text_ingest::{parse_turtle_document, parse_turtle_to_dataset};

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

/// Every row states `true`, the empty-list rows included: an empty `sh:in` or `sh:xone`
/// list is a lint diagnostic, not an ill-formedness. The empty rows are observed to be
/// validated (the approved `in-002` / `xone-002` answer: the focus node violates the empty
/// list), and their non-empty neighbours to conform, so the rows cannot pass by the
/// validation having been skipped.
#[test]
fn every_report_states_the_shapes_graph_well_formed() {
    for (constraint, conforms) in [
        ("sh:in ( ex:a )", true),
        ("sh:in ( )", false),
        ("sh:xone ( [ sh:nodeKind sh:IRI ] )", true),
        ("sh:xone ( )", false),
        ("sh:nodeKind sh:IRI", true),
    ] {
        let report = report(constraint);
        assert_eq!(report.shapes_graph_well_formed, Some(true), "{constraint}");
        assert_eq!(report.conforms, conforms, "{constraint}");
        let text = report.to_ntriples();
        assert!(text.contains(WELL_FORMED_TRUE), "{constraint}: {text}");
        assert!(!text.contains(WELL_FORMED_FALSE), "{constraint}: {text}");
    }
}

/// A W3C test file's text, linted as its own shapes graph (every test there names `<>`
/// as its shapes graph).
fn lint_text(text: &str) -> LintReport {
    let document = parse_turtle_document(text, Some("http://example.org/w3c-test"))
        .expect("the test file parses");
    lint(
        &document.dataset,
        &document.prefixes,
        None,
        None,
        &ShapesImports::new(),
    )
    .expect("lints")
}

/// The W3C test file at `path`, relative to the SHACL 1.2 suite's `tests/` directory.
fn w3c_test(path: &str) -> String {
    let file = format!(
        "{}/../../vectors/shacl12/tests/{path}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&file).unwrap_or_else(|error| panic!("{file}: {error}"))
}

/// `lint` over the approved W3C test `core/node/in-002` (and `xone-002`) lists the empty
/// list as a finding naming its rule id and shape, and the report is not clean. The
/// neighbour is the same file with one member in the list: it carries no diagnostic, and
/// its finding count is exactly one lower, so the diagnostic is the only difference.
#[test]
fn lint_always_reports_an_empty_in_or_xone_list_by_rule_id() {
    for (path, shape, rule, empty, filled) in [
        (
            "core/node/in-002.ttl",
            "TestShape",
            "in-minListLength",
            "sh:in () ;",
            "sh:in ( ex:Instance ) ;",
        ),
        (
            "core/node/xone-002.ttl",
            "TestXoneUnsatisfiableShape",
            "xone-minListLength",
            "sh:xone () ;",
            "sh:xone ( [ sh:nodeKind sh:IRI ] ) ;",
        ),
    ] {
        let text = w3c_test(path);
        assert_eq!(text.matches(empty).count(), 1, "{path}: the fixture moved");
        let flagged = lint_text(&text);
        let shape_iri = format!("http://example.com/ns#{shape}");
        assert_eq!(
            flagged
                .diagnostics()
                .iter()
                .map(|diagnostic| (diagnostic.rule, diagnostic.shape.clone()))
                .collect::<Vec<_>>(),
            vec![(rule, Term::NamedNode(shape_iri.as_str().into()))],
            "{path}: {}",
            flagged.render()
        );
        assert!(
            flagged.load_error().is_none(),
            "{path}: {}",
            flagged.render()
        );
        assert!(!flagged.is_clean(), "{path}: {}", flagged.render());
        let rendered = flagged.render();
        assert!(
            rendered.contains(&format!("diagnostics 1\ndiagnostic {rule} <{shape_iri}>\n")),
            "{path}: {rendered}"
        );
        // shacl-shacl.ttl's own warning on the empty list is the same defect: listed,
        // marked with the rule that states it, and not counted twice.
        let warned: Vec<_> = flagged
            .shacl_shacl()
            .iter()
            .filter(|result| result.diagnosed.is_some())
            .collect();
        assert_eq!(warned.len(), 1, "{path}: {rendered}");
        assert_eq!(warned[0].diagnosed, Some(rule), "{path}: {rendered}");
        assert!(
            rendered.contains(&format!(" diagnosed {rule}\n")),
            "{path}: {rendered}"
        );
        assert_eq!(flagged.findings(), 1, "{path}: {rendered}");

        let neighbour = lint_text(&text.replace(empty, filled));
        assert_eq!(
            neighbour.diagnostics(),
            [],
            "{path}: {}",
            neighbour.render()
        );
        assert!(
            neighbour.render().contains("diagnostics 0\n"),
            "{path}: {}",
            neighbour.render()
        );
        assert!(
            !neighbour.render().contains(rule),
            "{path}: {}",
            neighbour.render()
        );
        assert_eq!(
            neighbour.findings() + 1,
            flagged.findings(),
            "{path}: the diagnostic is the one finding the neighbour lacks: {}",
            flagged.render()
        );
    }
}

/// A report assembled from results rather than produced by a validation claims nothing.
#[test]
fn a_report_no_validation_produced_states_nothing() {
    let report = ValidationReport::from_results(Vec::new(), ConformanceDisallows::default());
    assert_eq!(report.shapes_graph_well_formed, None);
    assert!(!report.to_ntriples().contains("shapesGraphWellFormed"));
}

/// Maintainer decision, "Every run reports it": the approved W3C tests `core/node/in-002`
/// and `xone-002`, VALIDATED — each file is its own data and shapes graph — carry the
/// mandatory diagnostic on the report, beside the results and never among them. The
/// verdict and the results are the approved ones (the focus node violates the empty
/// list), the report graph is byte-identical to the same report with no diagnostic (SHACL
/// defines no report term for it and PurRDF mints none), and a whole validation, a
/// prepared binding, a candidate check and a prepared product restore all agree. The
/// neighbour whose list has a member carries none, so the diagnostic is about the list.
#[test]
fn every_validation_reports_the_mandatory_diagnostic_beside_its_results() {
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    for (path, shape, rule, empty, filled) in [
        (
            "core/node/in-002.ttl",
            "TestShape",
            "in-minListLength",
            "sh:in () ;",
            "sh:in ( ex:Instance ) ;",
        ),
        (
            "core/node/xone-002.ttl",
            "TestXoneUnsatisfiableShape",
            "xone-minListLength",
            "sh:xone () ;",
            "sh:xone ( [ sh:nodeKind sh:IRI ] ) ;",
        ),
    ] {
        let run = |text: &str| {
            let base = Some("http://example.org/w3c-test");
            let shapes = parse_shapes(text, base).expect("the test file loads");
            let data = parse_turtle_to_dataset(text, base).expect("the test file parses");
            let whole = validate_dataset_with_shapes_graph(&data, &shapes, None).expect("valid");
            let prepared = PreparedShapes::new(Arc::new(shapes));
            let bound = prepared.bind_dataset(&data).expect("binds");
            let again = bound.validate().expect("validates");
            assert_eq!(again.diagnostics, whole.diagnostics, "{path}");
            assert_eq!(again.to_ntriples(), whole.to_ntriples(), "{path}");
            let focus = Term::NamedNode("http://example.com/ns#Instance".into());
            let candidate = bound
                .validate_focus_nodes(&[focus])
                .expect("a candidate check");
            assert_eq!(candidate.diagnostics, whole.diagnostics, "{path}");
            let product = prepared.to_product(&ShapesProfile::CORE).expect("encodes");
            let restored = ShapesProduct::open(&product)
                .expect("opens")
                .admit(&ShapesProfile::CORE, &HostBindings::empty())
                .expect("admits")
                .bind_dataset(&data)
                .expect("binds")
                .validate()
                .expect("validates");
            assert_eq!(restored.diagnostics, whole.diagnostics, "{path}");
            whole
        };
        let text = w3c_test(path);
        let flagged = run(&text);
        let shape_term = Term::NamedNode(format!("http://example.com/ns#{shape}").as_str().into());
        assert_eq!(
            flagged
                .diagnostics
                .iter()
                .map(|diagnostic| (diagnostic.rule, diagnostic.shape.clone()))
                .collect::<Vec<_>>(),
            vec![(rule, shape_term)],
            "{path}"
        );
        assert_eq!(
            flagged.diagnostics[0].to_string(),
            format!("{rule} <http://example.com/ns#{shape}>")
        );
        assert!(!flagged.conforms, "{path}: the approved verdict");
        assert_eq!(flagged.results.len(), 1, "{path}");
        let without = flagged.clone().with_diagnostics(Vec::new());
        assert_eq!(
            flagged.to_ntriples(),
            without.to_ntriples(),
            "{path}: the report graph carries no diagnostic"
        );

        let neighbour = run(&text.replace(empty, filled));
        assert!(neighbour.diagnostics.is_empty(), "{path}");
    }
}
