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
        without_host_clock_or_entropy(|| infer(&data, &shapes, options)).unwrap()
    };
    let first_run = run(&options);
    let first = first_run.inferred();
    assert_eq!(first.len(), 6);
    let repeated = run(&options);
    assert_eq!(first, repeated.inferred());
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert_eq!(first_run.coordinates(), repeated.coordinates());
        assert_eq!(first_run.coordinates().entropy_seed(), Some(7));
        assert_eq!(
            expected_time(first_run.coordinates().now().unwrap()),
            expected
        );
    }
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert_ne!(first_run.coordinates(), changed_seed.coordinates());
        assert_eq!(first_run.contract_hash(), changed_seed.contract_hash());
    }
    for [subject, predicate, object] in first {
        let changed = changed_seed
            .inferred()
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
    #[cfg(not(target_arch = "wasm32"))]
    assert_ne!(first_run.coordinates(), changed_time.coordinates());
    for [subject, predicate, object] in first {
        let changed = changed_time
            .inferred()
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
    assert_eq!(first, run(&options).inferred());
    assert!(replace_ambient_context(AmbientContext::default()).is_idle());
    #[cfg(not(target_arch = "wasm32"))]
    {
        let receipt = first_run.coordinates().clone();
        let retained = first_run.clone();
        drop(first_run);
        drop(repeated);
        drop(changed_seed);
        drop(changed_time);
        drop(options);
        drop(data);
        drop(shapes);
        assert_eq!(retained.coordinates(), &receipt);
        assert_eq!(expected_time(receipt.now().unwrap()), expected);
    }
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
    let first = run(&options).unwrap();
    assert_eq!(first.inferred(), expected);
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert_eq!(first.coordinates().entropy_seed(), Some(7));
        assert_eq!(
            expected_time(first.coordinates().now().unwrap()),
            expected[0][2]
        );
    }
    let refusal = run(&options.clone().with_max_stored_facts(0)).unwrap_err();
    let SrlError::Execution(refusal) = refusal else {
        panic!("receipt-bearing refusal")
    };
    assert!(matches!(
        *refusal.cause,
        purrdf_shapes::rules::RulesError::LimitExceeded(limit) if limit.limit() == RuleLimit::StoredFacts
    ));
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[derive(Debug)]
        struct Cancelled;
        impl purrdf_core::StopSignal for Cancelled {
            fn stopped(&self) -> bool {
                true
            }
        }
        let stopped = srl::infer_until(&document, &source, &options, Some(&Cancelled)).unwrap_err();
        let SrlError::Execution(stopped) = stopped else {
            panic!("receipt-bearing cancellation")
        };
        assert!(matches!(
            *stopped.cause,
            purrdf_shapes::rules::RulesError::Stopped { .. }
        ));
    }
    assert!(replace_ambient_context(AmbientContext::default()).is_idle());
    let repeated = run(&options).unwrap();
    assert_eq!(repeated.inferred(), expected);
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(repeated.coordinates(), first.coordinates());
    let compact = TripleDataset::try_from(Arc::try_unwrap(source).unwrap()).unwrap();
    let read = compact.read().unwrap();
    let actual =
        without_host_clock_or_entropy(|| srl::infer_triples(&document, &read, &options)).unwrap();
    assert_eq!(actual.inferred(), expected);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let actual = actual.coordinates();
        let first = first.coordinates();
        assert!(
            actual
                .now()
                .unwrap()
                .same_representation(first.now().unwrap())
        );
        assert_eq!(actual.entropy_seed(), first.entropy_seed());
        assert_eq!(actual.program_identity(), first.program_identity());
        assert_eq!(actual.snapshot_identity(), first.snapshot_identity());
        assert_eq!(actual.executions(), first.executions());
        // Three-position and four-position clause stores have different compiled
        // contracts, even though their query contexts and inferred facts agree.
        assert_ne!(actual.execution_identity(), first.execution_identity());
    }
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

fn receipts_preserve_exact_time_representation_and_absent_inputs() {
    let document = srl::parse_and_check(
        "PREFIX : <http://example.org/> RULE { :s :time ?time } WHERE { SET(?time := NOW()) }",
        None,
    )
    .unwrap();
    let source = RdfDatasetBuilder::new().freeze().unwrap();
    let run = |lexical, seed| {
        let mut options = InferOptions::default();
        if let Some(lexical) = lexical {
            options = options.with_now(parse_datetime(lexical).unwrap());
        }
        if let Some(seed) = seed {
            options = options.with_entropy_seed(seed);
        }
        srl::infer(&document, &source, &options).unwrap()
    };
    let first = run(Some("2024-02-29T12:34:56.1250+05:30"), Some(7));
    let normalized = run(Some("2024-02-29T12:34:56.125+05:30"), Some(7));
    assert_eq!(first.coordinates(), normalized.coordinates());
    assert_eq!(first.inferred(), normalized.inferred());
    let same_instant = run(Some("2024-02-29T07:04:56.125Z"), Some(7));
    assert_ne!(first.coordinates(), same_instant.coordinates());
    assert_ne!(first.inferred(), same_instant.inferred());
    let no_zone = run(Some("2024-02-29T07:04:56.125"), Some(7));
    assert_ne!(same_instant.coordinates(), no_zone.coordinates());
    let time_only = run(Some("2024-02-29T12:34:56.125+05:30"), None);
    assert!(time_only.coordinates().entropy_seed().is_none());
    assert!(time_only.coordinates().now().is_some());
    let seed_only = run(None, Some(7));
    assert!(seed_only.coordinates().now().is_none());
    assert_eq!(seed_only.coordinates().entropy_seed(), Some(7));
    let defaults = run(None, None);
    assert!(defaults.coordinates().now().is_none());
    assert!(defaults.coordinates().entropy_seed().is_none());
}

purrdf_testkit::harness_main!(
    shacl_queries_keep_time_and_distinct_reproducible_seeds_across_focus_nodes,
    parsed_rl_keeps_coordinates_on_both_carriers_and_restores_after_refusal,
    shared_rule_dispatch_preserves_supplied_inputs_without_host_defaults,
    receipts_preserve_exact_time_representation_and_absent_inputs,
);
