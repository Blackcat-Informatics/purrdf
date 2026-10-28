// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller coordinates through the complete SHACL and SPARQL-RL rule engines.

#[path = "support/terms.rs"]
mod terms;

use std::sync::Arc;

use purrdf_core::{RdfDatasetBuilder, TripleDataset};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::rules::{RuleLimit, RuleOptions, infer};
use purrdf_shapes::sparql::{AmbientContext, replace_ambient_context};
use purrdf_shapes::srl::{self, InferOptions, SrlError};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_testkit::harness::without_host_clock_or_entropy;
use purrdf_xsd::temporal::{DateTime, parse_datetime};
use terms::example_org as iri;

fn expected_time(now: &DateTime) -> Term {
    Term::Literal(Literal::new_typed_literal(
        now.canonical_lexical(),
        NamedNode::from(purrdf_xsd::XsdDatatype::DateTime.iri()),
    ))
}

fn shacl_queries_keep_time_and_distinct_reproducible_seeds_across_focus_nodes() {
    let shapes = parse_shapes(
        r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ; sh:rule ex:r .
        ex:r a sh:SPARQLRule ; sh:runOnce true ; sh:construct """
            PREFIX ex: <http://example.org/>
            CONSTRUCT { $this ex:time ?time ; ex:random ?random ; ex:uuid ?uuid }
            WHERE {
                BIND(NOW() AS ?time)
                BIND(RAND() AS ?random)
                BIND(UUID() AS ?uuid)
            }
        """ ."#,
        None,
    )
    .unwrap();
    let empty = RdfDatasetBuilder::new().freeze().unwrap();
    let data = ShaclData::new(Arc::clone(&empty), empty, None);
    let lexical = format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096));
    let now = parse_datetime(&lexical).unwrap();
    let expected = expected_time(&now);
    let options = RuleOptions::default().with_now(now).with_entropy_seed(7);
    let run = |options: &RuleOptions| {
        without_host_clock_or_entropy(|| infer(&data, &shapes, options))
            .unwrap()
            .inferred()
            .to_vec()
    };
    let first = run(&options);
    assert_eq!(first.len(), 6);
    assert_eq!(first, run(&options));
    for focus in ["a", "b"] {
        assert!(first.contains(&[iri(focus), iri("time"), expected.clone()]));
    }
    let uuids: Vec<_> = first
        .iter()
        .filter(|[_, predicate, _]| *predicate == iri("uuid"))
        .map(|[_, _, object]| object)
        .collect();
    assert_eq!(uuids.len(), 2);
    assert_ne!(uuids[0], uuids[1]);
    let changed_seed = run(&options.clone().with_entropy_seed(8));
    for [subject, predicate, object] in &first {
        let changed = changed_seed
            .iter()
            .find(|[s, p, _]| s == subject && p == predicate)
            .unwrap();
        if *predicate == iri("time") {
            assert_eq!(*object, changed[2]);
        } else {
            assert_ne!(*object, changed[2]);
        }
    }
    let later = parse_datetime("2025-01-01T00:00:00Z").unwrap();
    let later_value = expected_time(&later);
    let changed_time = run(&options.clone().with_now(later));
    for [subject, predicate, object] in &first {
        let changed = changed_time
            .iter()
            .find(|[s, p, _]| s == subject && p == predicate)
            .unwrap();
        if *predicate == iri("time") {
            assert_eq!(changed[2], later_value);
            assert_ne!(*object, changed[2]);
        } else {
            assert_eq!(*object, changed[2]);
        }
    }
    assert_eq!(first, run(&options));
    assert!(replace_ambient_context(AmbientContext::default()).is_idle());
}

fn parsed_rl_keeps_coordinates_on_both_carriers_and_restores_after_refusal() {
    let document = srl::parse_and_check(
        "PREFIX : <http://example.org/>
         RULE { :s :time ?time } WHERE { SET(?time := NOW()) }",
        None,
    )
    .unwrap();
    let source = RdfDatasetBuilder::new().freeze().unwrap();
    let now = parse_datetime("1969-12-31T23:59:58.125Z").unwrap();
    let expected = [[iri("s"), iri("time"), expected_time(&now)]];
    let options = InferOptions::default().with_now(now).with_entropy_seed(7);
    let run = |options: &InferOptions| {
        without_host_clock_or_entropy(|| srl::infer(&document, &source, options))
    };
    assert_eq!(run(&options).unwrap().inferred(), expected);
    let refusal = run(&options.clone().with_max_stored_facts(0)).unwrap_err();
    assert!(matches!(
        refusal,
        SrlError::LimitExceeded(limit) if limit.limit() == RuleLimit::StoredFacts
    ));
    assert!(replace_ambient_context(AmbientContext::default()).is_idle());
    assert_eq!(run(&options).unwrap().inferred(), expected);
    let compact = TripleDataset::try_from(Arc::try_unwrap(source).unwrap()).unwrap();
    let read = compact.read().unwrap();
    let actual =
        without_host_clock_or_entropy(|| srl::infer_triples(&document, &read, &options)).unwrap();
    assert_eq!(actual.inferred(), expected);
    assert!(replace_ambient_context(AmbientContext::default()).is_idle());
}

/// The shared host dispatchers preserve both supplied inputs even with native
/// clock and entropy probes sealed, including the selected XPath scope.
fn shared_rule_dispatch_preserves_supplied_inputs_without_host_defaults() {
    let shapes = parse_shapes(
        r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetNode ex:s ; sh:rule [
          a sh:SPARQLRule ; sh:runOnce true ; sh:construct """
          CONSTRUCT { <http://example.org/s> <http://example.org/time> ?time }
          WHERE { BIND(NOW() AS ?time) FILTER(REGEX("ok", "^ok$")) } """ ] ."#,
        None,
    )
    .unwrap();
    let document = srl::parse_and_check(
        "PREFIX : <http://example.org/> RULE { :s :time ?time }
         WHERE { SET(?time := NOW()) FILTER(REGEX(\"ok\", \"^ok$\")) }",
        None,
    )
    .unwrap();
    let data = RdfDatasetBuilder::new().freeze().unwrap();
    let now = parse_datetime("1969-12-31T23:59:58.123456789012345678901+05:30").unwrap();
    let expected = [[iri("s"), iri("time"), expected_time(&now)]];
    let options = RuleOptions::default().with_now(now).with_entropy_seed(0);
    for source in [
        purrdf_shapes::RuleSource::Shapes(&shapes),
        purrdf_shapes::RuleSource::Srl(&document),
    ] {
        let ordinary = without_host_clock_or_entropy(|| {
            purrdf_shapes::run_rules_with_options(source, &data, &options)
        })
        .unwrap();
        assert_eq!(ordinary.inferred(), expected);
        let xpath = without_host_clock_or_entropy(|| {
            purrdf_shapes::xpath::run_rules_with_options(
                source,
                &data,
                &options,
                purrdf_core::xsd_regex::xpath::Profile::Xpath31,
                purrdf_core::xsd_regex::xpath::Limits::new(),
            )
        })
        .unwrap();
        assert_eq!(xpath.inferred(), expected);
    }
}

purrdf_testkit::harness_main!(
    shacl_queries_keep_time_and_distinct_reproducible_seeds_across_focus_nodes,
    parsed_rl_keeps_coordinates_on_both_carriers_and_restores_after_refusal,
    shared_rule_dispatch_preserves_supplied_inputs_without_host_defaults,
);
