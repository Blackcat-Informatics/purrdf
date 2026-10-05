// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A premise IRI is the IRI the premise document was read under, so an `owl:imports`
//! object — an absolute IRI once parsed — can only ever equal an absolute one. Every
//! entailment service on the shared boundary reaches [`premise_import_map`], so each one
//! refuses a premise IRI the workspace IRI parser does not read as absolute, naming it,
//! and each one still accepts the neighbouring absolute IRI.

use purrdf_validate::regime::consistency_to_string;
use purrdf_validate::{
    MaterializeLimits, certain_answers_to_string, check_premise_iris, graph_entails_to_string,
    materialize_to_nquads_string_with, premise_import_map,
};

const ONTOLOGY: &str = "http://example.org/onto";
const DOC: &str = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
const PATTERN: &str = "?s <http://example.org/p> ?o .\n";

/// Every refused spelling: not an IRI at all, a relative reference, and the empty string.
const REFUSED: [&str; 4] = ["::bad", "lib", "../lib", ""];

/// Every service that takes `premise_iris`, run over `premise` with nothing imported.
fn every_service(premise: &str) -> [(&'static str, Result<(), String>); 5] {
    let limits = MaterializeLimits::default();
    let premise_iris = [premise];
    [
        (
            "premise_import_map",
            premise_import_map(&[], &premise_iris).map(drop),
        ),
        (
            "materialize_to_nquads_string_with",
            materialize_to_nquads_string_with("rdfs", DOC, "", &[], &premise_iris, &limits)
                .map(drop),
        ),
        (
            "consistency_to_string",
            consistency_to_string(DOC, &[], &premise_iris, 0, 0).map(drop),
        ),
        (
            "certain_answers_to_string",
            certain_answers_to_string("rdfs", DOC, PATTERN, &[], &premise_iris, &limits).map(drop),
        ),
        (
            "graph_entails_to_string",
            graph_entails_to_string("rdfs", DOC, DOC, &[], &premise_iris, &limits).map(drop),
        ),
    ]
}

#[test]
fn a_premise_iri_that_is_not_absolute_is_refused_by_every_service() {
    for premise in REFUSED {
        for (service, outcome) in every_service(premise) {
            let error = outcome.expect_err(&format!("{service} accepted premise IRI {premise:?}"));
            assert!(
                error.contains("premise IRI") && error.contains(&format!("{premise:?}")),
                "{service}: {premise:?}: {error}"
            );
        }
    }
}

#[test]
fn an_absolute_premise_iri_is_accepted_by_every_service() {
    for (service, outcome) in every_service(ONTOLOGY) {
        outcome.unwrap_or_else(|error| panic!("{service} refused {ONTOLOGY:?}: {error}"));
    }
    // And an import of the premise's own IRI still resolves in place, which is what the
    // argument exists for.
    let map = premise_import_map(&[], &[ONTOLOGY]).expect("an absolute premise IRI");
    assert!(map.is_loaded(ONTOLOGY));
}

#[test]
fn the_presentation_nests_the_iri_parser_condition() {
    for (premise, detail) in [
        ("::bad", None),
        ("http://example.org/%zz", Some("iri-bad-percent-encoding")),
        ("lib", Some("iri-not-absolute-by-grammar.absent")),
        ("../lib", Some("iri-not-absolute-by-grammar.absent")),
        ("", Some("iri-empty")),
    ] {
        let error = check_premise_iris(&[ONTOLOGY, premise]).expect_err(premise);
        assert_eq!(error.iri(), premise);
        let presentation = error.presentation();
        assert_eq!(presentation.message_id(), "premise-iri-not-absolute");
        // The message every host renders IS the presentation's English, and it is the
        // string the shared boundary refuses with.
        assert_eq!(presentation.english(), error.to_string());
        assert_eq!(
            premise_import_map(&[], &[premise]).expect_err(premise),
            error.to_string()
        );
        let nested = presentation.detail().expect("the IRI parser's condition");
        assert_eq!(
            nested.message_id(),
            error.cause().presentation().message_id()
        );
        assert!(
            nested.message_id().starts_with("iri-"),
            "{premise:?}: {nested:?}"
        );
        if let Some(detail) = detail {
            assert_eq!(nested.message_id(), detail, "{premise:?}");
        }
        assert!(
            error.to_string().contains(error.cause().diagnostic_code()),
            "{error}"
        );
    }
}

#[test]
fn every_absolute_premise_iri_passes_the_check() {
    check_premise_iris(&[]).expect("no premise IRI");
    check_premise_iris(&[ONTOLOGY, "urn:example:onto", "http://example.org/onto#v1"])
        .expect("absolute premise IRIs");
}

#[test]
fn premise_iris_are_judged_before_the_import_table() {
    // Both arguments are bad; the premise IRI is the one reported, on every host.
    let error = premise_import_map(&[("lib", DOC)], &["::bad"]).expect_err("two bad arguments");
    assert!(error.contains("premise IRI"), "{error}");
    // The neighbour: with the premise IRI fixed, the import table's own refusal surfaces.
    let error = premise_import_map(&[("lib", DOC)], &[ONTOLOGY]).expect_err("a relative key");
    assert!(error.contains("import key <"), "{error}");
}
