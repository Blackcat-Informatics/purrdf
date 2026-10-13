// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native dated-bundle proofs through public source, binding and restore doors.

#![cfg(not(target_arch = "wasm32"))]

#[path = "support/turtle.rs"]
mod turtle;

use std::sync::Arc;

use purrdf_core::xsd_regex::xpath::{Error, Limits, Profile, Resource};
use purrdf_rdf::DatasetMut;
use purrdf_rdf::ir::{MutableDataset, ViewLimits};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_shapes::engine::{self, PreparedShapes, ValidationOptions};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::report::CompleteValidationError;
use purrdf_shapes::rules::{RuleOptions, infer_complete};
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::xpath::XPathValidationError;
use purrdf_shapes::{AdmissionReason, QueryPurpose, ShaclProfile, ShapesImports};

const PREFIXES: &str =
    "@prefix ex: <http://example.org/> . @prefix sh: <http://www.w3.org/ns/shacl#> .";
const PROFILES: [ShaclProfile; 2] = [ShaclProfile::REC_20170720, ShaclProfile::WD_20260918];
const BACKREFERENCE: &str = r#"
    ex:S a sh:NodeShape; sh:targetNode ex:n;
        sh:property [ sh:path ex:p; sh:pattern "(a)\\1" ] .
"#;

fn parsed(
    text: &str,
    options: &ValidationOptions,
) -> Result<Arc<purrdf_shapes::shapes::Shapes>, CompleteValidationError> {
    engine::parse_shapes_with_options(
        &format!("{PREFIXES}{text}"),
        None,
        None,
        None,
        &ShapesImports::new(),
        options,
    )
    .map(Arc::new)
}

#[test]
fn each_dated_bundle_reaches_free_complete_and_every_binding_door() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    for profile in PROFILES {
        let options = ValidationOptions::default().with_profile(profile);
        let shapes = parsed(BACKREFERENCE, &options).unwrap();
        assert!(engine::validate_dataset(&data, &shapes).unwrap().conforms);
        let prepared = PreparedShapes::new(Arc::clone(&shapes));
        let projected = engine::project_dataset(&data).unwrap();
        let mut mutation = MutableDataset::new(Arc::clone(&data));
        for quad in turtle::data(PREFIXES, "ex:n ex:extra 1 .").flat_default_graph_quads() {
            mutation.insert(quad).unwrap();
        }
        let delta = Arc::new(mutation.snapshot_view().unwrap());
        let bindings = [
            prepared.bind(ShaclData::new(
                Arc::clone(&projected),
                Arc::clone(&projected),
                None,
            )),
            prepared.bind_dataset(&data),
            prepared.bind_shared_dataset(Arc::clone(&data)),
            prepared.bind_view(Arc::new(ShaclDatasetView::project(Arc::clone(&data)))),
            prepared.bind_shared_dataset_with_shapes_graph(
                Arc::clone(&data),
                Some("http://example.org/graph"),
                ViewLimits::default(),
            ),
            prepared.bind_delta_with_shapes_graph(Arc::clone(&delta), None, ViewLimits::default()),
            prepared.bind_projected_dataset(Arc::clone(&projected)),
            prepared.bind_projected_dataset_with_shapes_graph(
                projected,
                Some("http://example.org/graph"),
            ),
        ];
        let node = Term::NamedNode(NamedNode::new_unchecked("http://example.org/n"));
        for binding in bindings {
            let binding = binding.unwrap();
            assert_eq!(binding.profile(), profile);
            assert!(binding.validate().unwrap().conforms);
            assert!(
                binding
                    .validate_focus_nodes(std::slice::from_ref(&node))
                    .unwrap()
                    .conforms
            );
            assert!(
                binding
                    .validate_focus_node_ids(&[binding.term_id(&node).unwrap()])
                    .unwrap()
                    .conforms
            );
        }
        let complete = prepared
            .bind_complete_shared_dataset(Arc::clone(&data))
            .unwrap();
        let report = complete.validate().unwrap();
        assert_eq!(report.profile(), profile);
        assert!(report.legacy().conforms);
        assert!(
            complete
                .validate_focus_nodes(std::slice::from_ref(&node))
                .unwrap()
                .legacy()
                .conforms
        );
        assert!(
            complete
                .validate_focus_node_ids(&[complete.legacy_binding().term_id(&node).unwrap()])
                .unwrap()
                .legacy()
                .conforms
        );
        let changed = prepared
            .bind_delta_with_shapes_graph(Arc::clone(&delta), None, ViewLimits::default())
            .unwrap();
        assert!(
            engine::validate_change(&changed, &delta)
                .unwrap()
                .report
                .conforms
        );
    }
}

#[test]
fn incompatible_override_is_refused_before_document_parsing_and_has_a_valid_neighbor() {
    let options = ValidationOptions::default()
        .with_profile(ShaclProfile::REC_20170720)
        .with_xpath_regex(Profile::Xpath31, Limits::new());
    assert!(
        matches!(parsed("invalid Turtle", &options), Err(CompleteValidationError::XPathProfile(conflict))
        if conflict.required() == Profile::Xpath20 && conflict.requested() == Profile::Xpath31)
    );
    parsed(
        BACKREFERENCE,
        &options.with_xpath_regex(Profile::Xpath20, Limits::new()),
    )
    .unwrap();
}

#[test]
fn current_source_preflight_applies_without_targets_and_does_not_reuse_another_law() {
    let text = r#"ex:S a sh:NodeShape; sh:deactivated true; sh:pattern "(?:a)" ."#;
    let legacy = PreparedShapes::new(Arc::new(turtle::loads(PREFIXES, text)));
    legacy
        .bind_shared_dataset(turtle::data(PREFIXES, ""))
        .unwrap()
        .validate()
        .unwrap();
    let draft = legacy.with_validation_options(
        ValidationOptions::default().with_profile(ShaclProfile::WD_20260918),
    );
    draft
        .bind_complete_shared_dataset(turtle::data(PREFIXES, ""))
        .unwrap()
        .validate()
        .unwrap();
    let rec = draft.clone().with_validation_options(
        ValidationOptions::default().with_profile(ShaclProfile::REC_20170720),
    );
    assert!(
        matches!(rec.bind_complete_shared_dataset(turtle::data(PREFIXES, "")),
        Err(CompleteValidationError::XPath(error)) if matches!(*error, XPathValidationError::Pattern(Error::Syntax { .. })))
    );
    draft
        .bind_complete_shared_dataset(turtle::data(PREFIXES, ""))
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn dated_quoted_flag_admission_does_not_inherit_a_warm_other_law() {
    let source = r#"
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:pattern "a.b"; sh:flags "q" ] .
    "#;
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a.b", "axb" ."#);
    let prepared = PreparedShapes::new(Arc::new(turtle::loads(PREFIXES, source)));
    for profile in [
        ShaclProfile::WD_20260918,
        ShaclProfile::REC_20170720,
        ShaclProfile::WD_20260918,
    ] {
        let selected = prepared
            .clone()
            .with_validation_options(ValidationOptions::default().with_profile(profile));
        let outcome = selected.bind_complete_shared_dataset(Arc::clone(&data));
        if profile == ShaclProfile::REC_20170720 {
            assert!(matches!(outcome, Err(CompleteValidationError::XPath(error))
            if matches!(*error, XPathValidationError::Pattern(Error::Flags {
                flag: 'q', profile: Profile::Xpath20, ..
            }))));
        } else {
            let report = outcome.unwrap().validate().unwrap();
            assert_eq!(report.results().len(), 1);
            assert_eq!(report.profile(), profile);
        }
        let neighbor = parsed(
            &source.replace("; sh:flags \"q\"", ""),
            &ValidationOptions::default().with_profile(profile),
        )
        .unwrap();
        assert!(
            engine::validate_complete_dataset(Arc::clone(&data), neighbor)
                .unwrap()
                .legacy()
                .conforms
        );
    }
}

#[test]
fn current_limits_admit_warm_programs_and_actual_target_failures_remain_native_typed() {
    let text = r#"
        ex:S a sh:NodeShape; sh:target [ a sh:SPARQLTarget; sh:select '''
            SELECT ?this WHERE { ?this <http://example.org/p> ?v FILTER(REGEX(?v, "a")) }
        ''' ]; sh:class ex:Missing .
    "#;
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    let options = ValidationOptions::default().with_profile(ShaclProfile::WD_20260918);
    let prepared = PreparedShapes::new(parsed(text, &options).unwrap());
    let good = prepared
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap()
        .validate()
        .unwrap();
    assert_eq!(good.results().len(), 1);
    let limits = Limits::new().with(Resource::MatchSteps, 0);
    let low = prepared
        .clone()
        .with_validation_options(options.with_xpath_regex(Profile::Xpath31, limits));
    assert!(
        matches!(low.bind_complete_shared_dataset(Arc::clone(&data)),
        Err(CompleteValidationError::XPath(error)) if matches!(*error, XPathValidationError::Query(ref diagnostic)
            if diagnostic.code == Resource::MatchSteps.code()))
    );
    assert!(
        matches!(prepared.clone().with_xpath_regex(Profile::Xpath31, limits).bind_shared_dataset(Arc::clone(&data)),
        Err(XPathValidationError::Query(diagnostic)) if diagnostic.code == Resource::MatchSteps.code())
    );
    assert!(
        matches!(prepared.clone().with_xpath_regex(Profile::Xpath20, Limits::new()).bind_shared_dataset(Arc::clone(&data)),
        Err(XPathValidationError::Complete(error)) if matches!(*error, CompleteValidationError::XPathProfile(_)))
    );
    assert_eq!(
        prepared
            .bind_complete_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .results()
            .len(),
        1
    );
}

#[test]
fn legacy_native_wrapper_keeps_its_law_over_shape_options() {
    let options = ValidationOptions::default().with_xpath_regex(Profile::Xpath20, Limits::new());
    let prepared = PreparedShapes::new(
        parsed(
            r#"
                ex:S a sh:NodeShape; sh:targetNode ex:n;
                    sh:property [ sh:path ex:p; sh:pattern "(?:a)" ] .
            "#,
            &options,
        )
        .unwrap(),
    );
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    assert!(
        !prepared
            .bind_shared_dataset(Arc::clone(&data))
            .unwrap()
            .validate()
            .unwrap()
            .conforms,
        "a direct explicit XPath 2.0 request keeps its own law"
    );
    let wrapper = prepared
        .with_xpath_regex(Profile::Xpath31, Limits::new())
        .bind_shared_dataset(data)
        .unwrap();
    assert_eq!(wrapper.selection(), (Profile::Xpath31, Limits::new()));
    assert!(
        wrapper.validate().unwrap().conforms,
        "the wrapper's XPath 3.1 selection admits the noncapturing group"
    );
}

#[test]
fn legacy_native_wrapper_keeps_current_limits_over_warmed_shape_options() {
    let options = ValidationOptions::default().with_xpath_regex(Profile::Xpath20, Limits::new());
    let prepared = PreparedShapes::new(
        parsed(
            r#"
                ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C .
                ex:C sh:select 'SELECT $this WHERE { FILTER(REGEX("a", "a")) }' .
            "#,
            &options,
        )
        .unwrap(),
    );
    let data = turtle::data(PREFIXES, "");
    let high = prepared
        .clone()
        .with_xpath_regex(Profile::Xpath31, Limits::new())
        .bind_shared_dataset(Arc::clone(&data))
        .unwrap();
    assert_eq!(high.validate().unwrap().results.len(), 1);
    let low_limits = Limits::new().with(Resource::MatchSteps, 0);
    let low = prepared
        .with_xpath_regex(Profile::Xpath31, low_limits)
        .bind_shared_dataset(data)
        .unwrap();
    assert_eq!(low.selection(), (Profile::Xpath31, low_limits));
    assert!(
        matches!(low.validate(), Err(XPathValidationError::Query(diagnostic))
        if diagnostic.code == Resource::MatchSteps.code())
    );
    assert_eq!(high.validate().unwrap().results.len(), 1);
}

#[test]
fn both_bundles_retain_constant_dynamic_and_replacement_query_laws_on_warm_requests() {
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    for select in [
        r#"SELECT $this WHERE { FILTER(REGEX("aa", "(a)\\1")) }"#,
        r#"SELECT $this WHERE { $this <http://example.org/p> ?value . BIND("(a)\\1" AS ?pattern) FILTER(REGEX(?value, ?pattern)) }"#,
        r#"SELECT $this WHERE { FILTER(REPLACE("aa", "(a)\\1", "$1X") = "aX") }"#,
    ] {
        let literal = purrdf_lex::literal_escape::escape(
            select,
            purrdf_lex::literal_escape::Carrier::Canonical,
        );
        let text = format!(
            "ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql ex:C . \
             ex:C sh:select \"{literal}\" ."
        );
        let original = PreparedShapes::new(Arc::new(turtle::loads(PREFIXES, &text)));
        for profile in PROFILES {
            let options = ValidationOptions::default().with_profile(profile);
            let prepared = original.clone().with_validation_options(options.clone());
            let bound = prepared
                .bind_complete_shared_dataset(Arc::clone(&data))
                .unwrap();
            for _ in 0..2 {
                let report = bound.validate().unwrap();
                assert_eq!(report.profile(), profile);
                assert_eq!(report.results().len(), 1, "{select}");
                assert_eq!(
                    report.results().next().unwrap().source_constraint(),
                    Some(&Term::NamedNode(NamedNode::new_unchecked(
                        "http://example.org/C"
                    )))
                );
            }
            for resource in [Resource::ProgramNodes, Resource::MatchSteps] {
                let low =
                    prepared
                        .clone()
                        .with_validation_options(options.clone().with_xpath_regex(
                            profile.xpath_profile().unwrap(),
                            Limits::new().with(resource, 0),
                        ));
                let refused = low
                    .bind_complete_shared_dataset(Arc::clone(&data))
                    .unwrap()
                    .validate();
                assert!(
                    matches!(refused,
                    Err(CompleteValidationError::XPath(error))
                        if matches!(*error, XPathValidationError::Query(ref diagnostic)
                            if diagnostic.code == resource.code())),
                    "{select}: {resource:?}"
                );
                assert_eq!(bound.validate().unwrap().results().len(), 1);
            }
        }
    }
}

#[test]
fn dated_governors_cover_target_binding_and_outrank_native_query_refusals() {
    let text = r#"
        ex:S a sh:NodeShape; sh:target [ a sh:SPARQLTarget; sh:select '''
            SELECT ?this WHERE { ?this <http://example.org/p> ?value FILTER(REGEX(?value, "a")) }
        ''' ]; sh:sparql [ sh:select 'SELECT $this WHERE {}' ] .
    "#;
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "a" ."#);
    let zero = purrdf_sparql_eval::QueryGovernors::UNBOUNDED.with_fuel(0);
    for profile in PROFILES {
        let options = ValidationOptions::default().with_profile(profile);
        let prepared = parsed(text, &options).unwrap();
        let low = PreparedShapes::new(Arc::clone(&prepared)).with_validation_options(
            options.with_xpath_regex(
                profile.xpath_profile().unwrap(),
                Limits::new().with(Resource::MatchSteps, 0),
            ),
        );
        for _ in 0..2 {
            let error = engine::validate_complete_dataset_with_governors(
                Arc::clone(&data),
                Arc::clone(low.shapes()),
                &zero,
            )
            .unwrap_err();
            let CompleteValidationError::Resource(refusal) = error else {
                panic!("the actual target governor must remain authoritative: {error:?}");
            };
            assert!(matches!(
                refusal.tripped(),
                purrdf_core::TrippedGovernor::Budget {
                    dimension: purrdf_core::ResourceDimension::Fuel,
                    ..
                }
            ));
            let report = engine::validate_complete_dataset_with_governors(
                Arc::clone(&data),
                Arc::clone(&prepared),
                &purrdf_sparql_eval::QueryGovernors::UNBOUNDED,
            )
            .unwrap();
            assert_eq!(report.results().len(), 1);
        }
        assert!(matches!(engine::validate_complete_dataset_with_governors(
            Arc::clone(&data), Arc::clone(low.shapes()),
            &purrdf_sparql_eval::QueryGovernors::UNBOUNDED),
            Err(CompleteValidationError::XPath(error))
                if matches!(*error, XPathValidationError::Query(ref diagnostic)
                    if diagnostic.code == Resource::MatchSteps.code())));
    }
}

#[test]
fn canonical_focus_errors_keep_the_actual_typed_cause_across_input_orders() {
    let text = r"
        ex:S a sh:NodeShape; sh:targetSubjectsOf ex:p; sh:sparql ex:C .
        ex:C sh:select '''SELECT $this ?failure WHERE {
            BIND(IF($this = <http://example.org/a>, true, REGEX('a', 'a')) AS ?failure)
        }''' .
    ";
    // The larger fixture crosses the production scheduler's 1,024-focus floor;
    // this local pool makes its actual indexed worker path independent of the host.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    for size in [2, 1_025] {
        pool.install(|| {
            let mut rows = String::new();
            let mut requested = Vec::new();
            for index in 1..size {
                use std::fmt::Write as _;
                writeln!(rows, "ex:n{index:04} ex:p \"a\" .").unwrap();
                requested.push(Term::NamedNode(NamedNode::new_unchecked(format!(
                    "http://example.org/n{index:04}"
                ))));
            }
            rows.push_str("ex:a ex:p \"a\" .");
            requested.push(Term::NamedNode(NamedNode::new_unchecked(
                "http://example.org/a",
            )));
            let data = turtle::data(PREFIXES, &rows);
            for profile in PROFILES {
                let options = ValidationOptions::default()
                    .with_profile(profile)
                    .with_xpath_regex(
                        profile.xpath_profile().unwrap(),
                        Limits::new().with(Resource::MatchSteps, 0),
                    );
                for semantic_first in [true, false] {
                    let source = if semantic_first {
                        text.to_owned()
                    } else {
                        text.replace("true, REGEX('a', 'a')", "REGEX('a', 'a'), true")
                    };
                    let binding = PreparedShapes::new(parsed(&source, &options).unwrap())
                        .bind_complete_shared_dataset(Arc::clone(&data))
                        .unwrap();
                    for _ in 0..2 {
                        for outcome in
                            [binding.validate(), binding.validate_focus_nodes(&requested)]
                        {
                            if semantic_first {
                                assert!(
                                    matches!(outcome, Err(CompleteValidationError::Semantic(_))),
                                    "the earlier semantic failure owns this operation"
                                );
                            } else {
                                assert!(
                                    matches!(outcome, Err(CompleteValidationError::XPath(error))
                                if matches!(*error, XPathValidationError::Query(ref diagnostic)
                                    if diagnostic.code == Resource::MatchSteps.code())),
                                    "the earlier native failure owns this operation"
                                );
                            }
                        }
                        requested.reverse();
                    }
                }
            }
        });
    }
}

#[test]
fn dated_some_value_still_suppresses_an_ordinary_semantic_candidate_failure() {
    let text = r"
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:someValue [
                sh:or ( [ sh:class ex:Duck ] [ sh:sparql ex:C ] )
            ] ] .
        ex:C sh:select 'SELECT $this ?failure WHERE { BIND(true AS ?failure) }' .
    ";
    let options = ValidationOptions::default().with_profile(ShaclProfile::WD_20260918);
    let prepared = PreparedShapes::new(parsed(text, &options).unwrap());
    let failed = prepared
        .bind_complete_shared_dataset(turtle::data(PREFIXES, "ex:n ex:p ex:aBad ."))
        .unwrap();
    let error = failed.validate().unwrap_err();
    let CompleteValidationError::Semantic(failure) = error else {
        panic!("the single candidate has an ordinary semantic failure: {error:?}")
    };
    assert_eq!(failure.focus().to_string(), "<http://example.org/aBad>");
    assert_eq!(
        failure.source_constraint().unwrap().to_string(),
        "<http://example.org/C>"
    );
    let neighbour = prepared
        .bind_complete_shared_dataset(turtle::data(
            PREFIXES,
            "ex:n ex:p ex:aBad, ex:zGood . ex:zGood a ex:Duck .",
        ))
        .unwrap();
    for _ in 0..2 {
        let report = neighbour.validate().unwrap();
        assert!(report.legacy().conforms);
        assert_eq!(report.results().len(), 0);
    }
}

#[test]
fn existential_conformance_does_not_discard_a_reached_function_policy_refusal() {
    let text = r#"
        ex:f a sh:SPARQLFunction; sh:parameter [ sh:path ex:p ];
            sh:select 'SELECT ?result WHERE { VALUES ?p { "x" } BIND(?p AS ?result) }' .
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:someValue [ sh:sparql [ sh:select '''
                SELECT $this WHERE {
                    FILTER(IF($this = <http://example.org/a>,
                              <http://example.org/f>("x"), "ok") = "x")
                }
            ''' ] ] ] .
    "#;
    let profile = ShaclProfile::WD_20260918;
    let options = ValidationOptions::default().with_profile(profile);
    let data = turtle::data(PREFIXES, "ex:n ex:p ex:a, ex:z .");
    let preparation = PreparedShapes::new(parsed(text, &options).unwrap());
    let binding = preparation
        .bind_complete_shared_dataset(Arc::clone(&data))
        .unwrap();
    for _ in 0..2 {
        match binding
            .validate()
            .expect_err("a reached function policy refusal is authoritative")
        {
            CompleteValidationError::Admission(refusal) => {
                assert_eq!(refusal.profile(), profile);
                assert_eq!(refusal.purpose(), QueryPurpose::Function);
                assert_eq!(refusal.reason(), AdmissionReason::Values);
            }
            error => panic!("the typed admission refusal must survive: {error:?}"),
        }
    }
    let dormant = text.replace("$this = <http://example.org/a>", "false");
    assert!(
        PreparedShapes::new(parsed(&dormant, &options).unwrap())
            .bind_complete_shared_dataset(data)
            .unwrap()
            .validate()
            .unwrap()
            .legacy()
            .conforms,
        "an uninvoked function creates no policy refusal"
    );
    let ordered = r#"
        ex:f a sh:SPARQLFunction; sh:parameter [ sh:path ex:p ];
            sh:select 'SELECT ?result WHERE { VALUES ?p { "x" } BIND(?p AS ?result) }' .
        ex:S a sh:NodeShape; sh:targetNode ex:n;
            sh:property [ sh:path ex:p; sh:someValue [ sh:sparql [ sh:select '''
                SELECT $this ?failure WHERE {
                    { FILTER($this = <http://example.org/a>) BIND(true AS ?failure) }
                    UNION
                    { FILTER($this = <http://example.org/b>)
                      FILTER(<http://example.org/f>("x") = "x") }
                }
            ''' ] ] ] .
    "#;
    let binding = PreparedShapes::new(parsed(ordered, &options).unwrap())
        .bind_complete_shared_dataset(turtle::data(PREFIXES, "ex:n ex:p ex:a, ex:b, ex:z ."))
        .unwrap();
    for _ in 0..2 {
        let error = binding.validate().unwrap_err();
        let CompleteValidationError::Admission(refusal) = error else {
            panic!("a suppressed semantic candidate cannot mask policy refusal: {error:?}")
        };
        assert_eq!(refusal.profile(), profile);
        assert_eq!(refusal.purpose(), QueryPurpose::Function);
        assert_eq!(refusal.reason(), AdmissionReason::Values);
    }
}

#[test]
fn a_reached_udf_admission_capsule_wins_its_paired_native_query_diagnostic() {
    let text = r#"
        ex:f a sh:SPARQLFunction; sh:parameter [ sh:path ex:p ];
            sh:select 'SELECT ?result WHERE { VALUES ?p { "x" } BIND(?p AS ?result) }' .
        ex:S a sh:NodeShape; sh:targetNode ex:n; sh:sparql [ sh:select '''
            SELECT $this WHERE { FILTER(ex:f("x") = "x") }
        ''' ] .
    "#;
    for profile in PROFILES {
        let options = ValidationOptions::default().with_profile(profile);
        let prepared = PreparedShapes::new(parsed(text, &options).unwrap());
        let complete = prepared
            .bind_complete_shared_dataset(turtle::data(PREFIXES, ""))
            .unwrap();
        match complete.validate().unwrap_err() {
            CompleteValidationError::Admission(refusal) => {
                assert_eq!(refusal.profile(), profile);
                assert_eq!(refusal.purpose(), QueryPurpose::Function);
                assert_eq!(refusal.reason(), AdmissionReason::Values);
            }
            error => panic!("the actual UDF capsule must remain authoritative: {error:?}"),
        }
        let law = profile.xpath_profile().unwrap();
        let native = prepared
            .with_xpath_regex(law, Limits::new())
            .bind_shared_dataset(turtle::data(PREFIXES, ""))
            .unwrap();
        assert!(
            matches!(native.validate(), Err(XPathValidationError::Complete(error))
            if matches!(*error, CompleteValidationError::Admission(_)))
        );
        let dormant = text.replace("ex:f(\"x\")", "IF(false, ex:f(\"x\"), \"x\")");
        let report = PreparedShapes::new(parsed(&dormant, &options).unwrap())
            .bind_complete_shared_dataset(turtle::data(PREFIXES, ""))
            .unwrap()
            .validate()
            .unwrap();
        assert_eq!(
            report.results().len(),
            1,
            "a short-circuited call is not invoked"
        );
    }
}

#[test]
fn product_admit_rebuild_and_expectation_doors_reconstruct_the_current_bundle() {
    let preparation = PreparedShapes::new(Arc::new(turtle::loads(PREFIXES, BACKREFERENCE)));
    let bytes = preparation.to_product(&ShapesProfile::CORE).unwrap();
    let expected = *ShapesProduct::open(&bytes)
        .unwrap()
        .declared_identity()
        .digest();
    let data = turtle::data(PREFIXES, r#"ex:n ex:p "aa" ."#);
    for profile in PROFILES {
        let options = ValidationOptions::default().with_profile(profile);
        let host = HostBindings::empty();
        let restored = [
            ShapesProduct::open(&bytes).unwrap().admit_with_options(
                &ShapesProfile::CORE,
                &host,
                &options,
            ),
            ShapesProduct::open(&bytes)
                .unwrap()
                .admit_expecting_with_options(&ShapesProfile::CORE, &host, &expected, &options),
            ShapesProduct::open(&bytes).unwrap().rebuild_with_options(
                &ShapesProfile::CORE,
                &host,
                &options,
            ),
            ShapesProduct::open(&bytes)
                .unwrap()
                .rebuild_expecting_with_options(&ShapesProfile::CORE, &host, &expected, &options),
        ];
        for prepared in restored {
            let prepared = prepared.unwrap();
            assert_eq!(prepared.to_product(&ShapesProfile::CORE).unwrap(), bytes);
            let report = prepared
                .bind_complete_shared_dataset(Arc::clone(&data))
                .unwrap()
                .validate()
                .unwrap();
            assert_eq!(report.profile(), profile);
            assert!(report.legacy().conforms);
        }
    }
}

#[test]
fn public_global_rule_execution_audits_original_query_even_for_an_empty_join() {
    let text = r"
        ex:R a sh:SPARQLRule; sh:runOnce true; sh:construct '''
            CONSTRUCT { <http://example.org/n> <http://example.org/q> ?v }
            WHERE { ?s <http://example.org/missing> ?v MINUS { ?s <http://example.org/p> ?v } }
        ''' .
    ";
    for profile in PROFILES {
        let options = ValidationOptions::default().with_profile(profile);
        let shapes = parsed(text, &options).unwrap();
        for rows in ["", "ex:n ex:missing 1 ."] {
            let data = turtle::data(PREFIXES, rows);
            let holder = ShaclData::new(Arc::clone(&data), data, None);
            let outcome = infer_complete(&holder, &shapes, &RuleOptions::default());
            if profile == ShaclProfile::REC_20170720 {
                assert!(
                    matches!(&outcome,
                    Err(CompleteValidationError::Admission(refusal)) if refusal.reason() == AdmissionReason::Minus
                        && refusal.purpose() == QueryPurpose::GlobalConstructRule),
                    "the REC source audit must run even for an empty join: {outcome:?}"
                );
            } else {
                // The draft's pre-binding restrictions do not invent a focus
                // binding for a global rule with no declared parameters.
                assert_eq!(
                    outcome
                        .unwrap()
                        .dataset()
                        .owned_quads()
                        .filter(|quad| quad.predicate == "http://example.org/q")
                        .count(),
                    usize::from(!rows.is_empty())
                );
            }
        }
        let valid = parsed(
            &text.replace(" MINUS { ?s <http://example.org/p> ?v }", ""),
            &options,
        )
        .unwrap();
        let data = turtle::data(PREFIXES, "ex:n ex:missing 1 .");
        let holder = ShaclData::new(Arc::clone(&data), data, None);
        assert_eq!(
            infer_complete(&holder, &valid, &RuleOptions::default())
                .unwrap()
                .dataset()
                .owned_quads()
                .filter(|quad| quad.predicate == "http://example.org/q")
                .count(),
            1
        );
    }
}
