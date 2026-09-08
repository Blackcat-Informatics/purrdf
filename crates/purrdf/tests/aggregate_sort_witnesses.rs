// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Aggregate sort inputs participate in the public reasoning witness boundary.

use std::sync::Arc;

use purrdf::entail::materialize_combined;
use purrdf::sparql::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};
use purrdf::{
    ClosureRelations, GovernedEntailment, QueryEntailment, RdfDataset, SparqlRequest, SparqlResult,
    TermValue, parse_dataset, query_with_entailment, query_with_entailment_governed,
};

const ONTOLOGY: &str = r"@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:A a owl:Class; rdfs:subClassOf [ a owl:Restriction; owl:onProperty :r; owl:someValuesFrom :B ] .
:B a owl:Class .
:a1 a :A; :r :named1 .
:a2 a :A; :r :named2 .
";

fn ontology() -> Arc<RdfDataset> {
    parse_dataset(ONTOLOGY.as_bytes(), "text/turtle", None).expect("ontology")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn evaluate(dataset: &RdfDataset, query: &str, mode: QueryEntailment<'_>) -> SparqlResult {
    query_with_entailment(
        &NativeSparqlEngine::new(),
        dataset,
        request(query),
        mode,
        QueryOptions::EMPTY,
        &ClosureRelations::NONE,
    )
    .expect("entailed query")
    .0
}

fn governed(dataset: &RdfDataset, query: &str) -> SparqlResult {
    let GovernedEntailment::Answered {
        outcome: GovernedOutcome::Complete { result, .. },
        ..
    } = query_with_entailment_governed(
        &NativeSparqlEngine::new(),
        dataset,
        request(query),
        QueryEntailment::OwlDirect,
        QueryOptions::EMPTY,
        &ClosureRelations::NONE,
        &QueryGovernors::UNBOUNDED,
    )
    .expect("governed query")
    else {
        panic!("an unbounded query must complete")
    };
    result
}

fn bindings(result: SparqlResult, variable: &str) -> Vec<Vec<Option<TermValue>>> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("SELECT result")
    };
    assert_eq!(variables, [variable]);
    rows
}

#[test]
fn aggregate_sort_witnesses_cannot_change_returned_lists() {
    let dataset = ontology();
    let combined = materialize_combined(&dataset, &[])
        .expect("Horn ontology")
        .expect("combined approach");
    assert!(
        !combined.surrogates.is_empty(),
        "the fixture mints witnesses"
    );
    for (sort, [first, second]) in [
        ("?y", ["a1", "a2"]),
        ("STR(?y)", ["a1", "a2"]),
        ("IF(isBlank(?y), \"0\", STR(?y))", ["a1", "a2"]),
        ("DESC(?y)", ["a2", "a1"]),
        ("DESC(STR(?y))", ["a2", "a1"]),
    ] {
        let query = format!(
            "SELECT (FOLD(?x ORDER BY {sort} ?x) AS ?list) \
             WHERE {{ ?x <http://example.org/r> ?y }}"
        );
        let expected = vec![vec![Some(TermValue::typed_literal(
            format!("[<http://example.org/{first}>,<http://example.org/{second}>]"),
            "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
        ))]];
        assert_eq!(
            bindings(evaluate(&dataset, &query, QueryEntailment::Simple), "list"),
            expected,
            "asserted named targets: {sort}"
        );
        assert_ne!(
            bindings(
                evaluate(&combined.dataset, &query, QueryEntailment::Simple),
                "list"
            ),
            expected,
            "raw witnesses must affect the list or this fixture is vacuous: {sort}"
        );
        assert_eq!(
            bindings(
                evaluate(&dataset, &query, QueryEntailment::OwlDirect),
                "list"
            ),
            expected,
            "unguarded aggregate sort key: {sort}"
        );
        assert_eq!(
            bindings(governed(&dataset, &query), "list"),
            expected,
            "{sort}"
        );
    }
}

#[test]
fn outer_order_by_keeps_witnesses_existential() {
    let dataset = ontology();
    let query = "SELECT DISTINCT ?x WHERE { ?x <http://example.org/r> ?y . \
                 ?y a <http://example.org/B> } ORDER BY STR(?y) ?x";
    assert_eq!(
        bindings(evaluate(&dataset, query, QueryEntailment::Simple), "x"),
        Vec::<Vec<Option<TermValue>>>::new()
    );
    let expected: Vec<_> = ["a1", "a2"]
        .map(|name| vec![Some(TermValue::iri(format!("http://example.org/{name}")))])
        .into();
    assert_eq!(
        bindings(evaluate(&dataset, query, QueryEntailment::OwlDirect), "x"),
        expected
    );
    assert_eq!(bindings(governed(&dataset, query), "x"), expected);
}
