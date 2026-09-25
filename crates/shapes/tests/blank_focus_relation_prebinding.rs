// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A SHACL focus node reaches a relation call BOUND wherever SHACL-SPARQL pre-binding
//! writes it — including the positions the ordinary pushdown does not reach — and a
//! blank-node focus node reaches it exactly as an IRI one does.
//!
//! SHACL pre-binds `$this` into every scope of a constraint body, including the three
//! the ordinary pushdown stops at: an `OPTIONAL`'s right arm, a sub-`SELECT`, and an
//! `EXISTS` nested below the pattern the seed is joined onto. An IRI focus node reaches a relation's argument
//! there as a written constant. A blank-node focus node cannot be written into the
//! query — a blank node in a query is an anonymous variable — so it has to reach the
//! call as the term it is, bound, by another door. Left free, a relation that serves
//! only a bound subject refuses, and the validation fails.
//!
//! The relation below serves ONLY a bound subject, and answers with the subject
//! itself, which the body then joins against the data graph. So every run tells apart
//! three outcomes: refused (the subject arrived free), the focus node's own verdict,
//! and another node's verdict. Each shape runs over four focus nodes — a violating and
//! a conforming IRI, and a violating and a conforming blank node. The oracle holds each
//! node by the term id the dataset gives it, so the two blank nodes are never merged
//! into one "blank" answer: each reported focus node must BE the violating node that
//! owns the reported status, the conforming blank node must report nothing, and each
//! relation invocation is counted against the exact node it was bound to. A conforming
//! blank focus node handed the violating blank's answer would be reported under its own
//! identity, and fail.
//!
//! Fixture IRIs are `example.org`; PurRDF mints none.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use purrdf::RdfDataset;
use purrdf_core::{TermId, TermValue};
use purrdf_shapes::engine::{parse_shapes, validate_dataset};
use purrdf_shapes::sparql::enter_property_function_scope;
use purrdf_sparql_eval::{
    BindingPattern, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, Volatility,
};

const EX: &str = "http://example.org/";

/// The relation IRI the constraint bodies call.
const REL_IRI: &str = "http://example.org/rel/self";

/// A `bf` relation that answers a bound subject with that subject, and refuses a free
/// one.
#[derive(Debug)]
struct SelfRelation {
    modes: [BindingPattern; 1],
    /// Every subject a bound invocation was handed, as the term itself — a blank node
    /// keeps its label and scope, so two blank nodes are never recorded as one.
    seen: Mutex<Vec<TermValue>>,
}

impl SelfRelation {
    fn seen(&self) -> Vec<TermValue> {
        self.seen.lock().expect("unpoisoned").clone()
    }
}

struct OneRow(Option<PfRow>);

impl PfCursor for OneRow {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.0.take())
    }
}

impl PropertyFunction for SelfRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let Some(subject) = args.get(0) else {
            return Err(EvalError::function(
                "the subject at position 0 is free; this relation is declared `bf` and cannot \
                 enumerate subjects"
                    .to_owned(),
            ));
        };
        self.seen.lock().expect("unpoisoned").push(subject.clone());
        Ok(Box::new(OneRow(Some(vec![
            subject.clone(),
            subject.clone(),
        ]))))
    }
}

fn relation() -> (Arc<PropertyFunctionRegistry>, Arc<SelfRelation>) {
    let relation = Arc::new(SelfRelation {
        modes: [BindingPattern::from_code("bf")],
        seen: Mutex::new(Vec::new()),
    });
    let mut registry = PropertyFunctionRegistry::new();
    let registered: Arc<dyn PropertyFunction> = Arc::<SelfRelation>::clone(&relation);
    registry.register(REL_IRI.to_owned(), registered);
    (Arc::new(registry), relation)
}

/// Four items. Each owns one `ex:status` literal; the two whose literal starts with
/// `bad` violate, and the literal says which of them it is.
fn items() -> Arc<RdfDataset> {
    let triples = format!(
        "<{EX}iri-bad> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Item> .\n\
         <{EX}iri-bad> <{EX}status> \"bad iri\" .\n\
         <{EX}iri-good> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Item> .\n\
         <{EX}iri-good> <{EX}status> \"fine iri\" .\n\
         _:bad <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Item> .\n\
         _:bad <{EX}status> \"bad blank\" .\n\
         _:good <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Item> .\n\
         _:good <{EX}status> \"fine blank\" .\n"
    );
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&triples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

/// The four fixture nodes, each by the term id the dataset holds it under.
struct Nodes {
    iri_bad: TermId,
    iri_good: TermId,
    blank_bad: TermId,
    blank_good: TermId,
}

impl Nodes {
    /// Find each node as the one subject that owns its status literal — so a blank node
    /// is identified by the data it carries, not by the label it was written with.
    fn of(dataset: &RdfDataset) -> Self {
        let status = dataset
            .term_id_by_iri(&format!("{EX}status"))
            .expect("the status predicate is in the dataset");
        let owner = |literal: &str| {
            let object = dataset
                .term_id_by_value(&TermValue::simple_literal(literal))
                .unwrap_or_else(|| panic!("the status {literal:?} is in the dataset"));
            let owners: Vec<TermId> = dataset
                .quads()
                .filter(|quad| quad.p == status && quad.o == object)
                .map(|quad| quad.s)
                .collect();
            assert_eq!(owners.len(), 1, "one node owns {literal:?}: {owners:?}");
            owners[0]
        };
        let nodes = Self {
            iri_bad: owner("bad iri"),
            iri_good: owner("fine iri"),
            blank_bad: owner("bad blank"),
            blank_good: owner("fine blank"),
        };
        for blank in [nodes.blank_bad, nodes.blank_good] {
            assert!(
                matches!(dataset.term_value(blank), TermValue::Blank { .. }),
                "{:?}",
                dataset.term_value(blank)
            );
        }
        assert_ne!(
            nodes.blank_bad, nodes.blank_good,
            "two distinct blank nodes"
        );
        nodes
    }
}

/// The dataset's own id for a term the validation or the relation handed back — the
/// identity the oracle compares, which keeps two blank nodes apart.
fn id_in(dataset: &RdfDataset, value: &TermValue, what: &str) -> TermId {
    dataset
        .term_id_by_value(value)
        .unwrap_or_else(|| panic!("{what} {value:?} is a node the dataset holds"))
}

/// One shape over every item, whose `sh:select` body is `body`.
fn shapes(body: &str) -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:ItemShape
    a sh:NodeShape ;
    sh:targetClass ex:Item ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "the item's status is bad" ;
        sh:select """{body}""" ;
    ] .
"#
    );
    parse_shapes(&turtle, None).expect("parse shapes")
}

/// The three positions the pushdown does not reach, each with a body that reports a
/// focus node whose relation-returned self owns a `bad` status, with that status as the
/// value.
fn bodies() -> [(&'static str, String); 3] {
    [
        (
            "an OPTIONAL arm",
            format!(
                "SELECT $this ?value WHERE {{ \
                   OPTIONAL {{ ( $this ) <{REL_IRI}> ( ?v ) . ?v <{EX}status> ?value }} \
                   FILTER(BOUND(?value) && STRSTARTS(STR(?value), \"bad\")) }}"
            ),
        ),
        (
            // SHACL requires a sub-SELECT to project every pre-bound variable (a shape
            // whose sub-SELECT does not is refused at load), so this one projects
            // `$this`. Joined beside a data atom, it is a separate scope the pushdown
            // does not enter (alone in the group it would BE the core, which it does).
            "a sub-SELECT",
            format!(
                "SELECT $this ?value WHERE {{ $this a <{EX}Item> . \
                   {{ SELECT $this ?value WHERE {{ ( $this ) <{REL_IRI}> ( ?v ) . \
                      ?v <{EX}status> ?value }} }} \
                   FILTER(STRSTARTS(STR(?value), \"bad\")) }}"
            ),
        ),
        (
            "an EXISTS below the core",
            format!(
                "SELECT $this ?value WHERE {{ $this <{EX}status> ?value . \
                   {{ BIND(1 AS ?one) FILTER EXISTS {{ ( $this ) <{REL_IRI}> ( ?v ) . \
                      ?v <{EX}status> ?s FILTER(STRSTARTS(STR(?s), \"bad\")) }} }} }}"
            ),
        ),
    ]
}

/// Validate the four items under `body` and hold the report and the relation's
/// invocations to the claim: every focus node — IRI or blank — reaches the relation
/// bound AS ITSELF, and exactly the two violating nodes are reported, each under its own
/// identity with its own status.
fn check(position: &str, body: &str) {
    let (registry, relation) = relation();
    let dataset = items();
    let nodes = Nodes::of(&dataset);
    let report = {
        let _relations = enter_property_function_scope(registry);
        validate_dataset(&dataset, &shapes(body))
            .unwrap_or_else(|error| panic!("{position}: the validation runs: {error}"))
    };
    assert!(!report.conforms, "{position}: {report:?}");
    let reported: BTreeSet<(TermId, String)> = report
        .results
        .iter()
        .map(|result| {
            (
                id_in(
                    &dataset,
                    &result.focus_node.to_term_value(),
                    "the focus node",
                ),
                result.value.as_ref().map_or_default(ToString::to_string),
            )
        })
        .collect();
    let expected: BTreeSet<(TermId, String)> = [
        (nodes.iri_bad, "\"bad iri\"".to_owned()),
        (nodes.blank_bad, "\"bad blank\"".to_owned()),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        reported, expected,
        "{position}: exactly the violating IRI and the violating blank node, each reported \
         as itself with the status it owns, and neither conforming node — {report:?}"
    );
    assert_eq!(report.results.len(), 2, "{position}: {report:?}");

    // Every focus node reached the relation bound — the relation refuses a free
    // subject, so every recorded invocation is a bound one — and each was handed to it
    // as ITSELF, as often as every other: the validation runs its query the same number
    // of times per focus node whatever its kind, so a blank focus node bound to the
    // other blank node would leave one count high and the other short.
    let seen = relation.seen();
    let mut bound: BTreeMap<TermId, usize> = BTreeMap::new();
    for subject in &seen {
        *bound
            .entry(id_in(&dataset, subject, "a bound subject"))
            .or_default() += 1;
    }
    let per_node = bound.get(&nodes.iri_bad).copied().unwrap_or_default();
    assert!(
        per_node > 0,
        "{position}: the IRI focus nodes reached it — {seen:?}"
    );
    let expected: BTreeMap<TermId, usize> = [
        nodes.iri_bad,
        nodes.iri_good,
        nodes.blank_bad,
        nodes.blank_good,
    ]
    .into_iter()
    .map(|node| (node, per_node))
    .collect();
    assert_eq!(
        bound, expected,
        "{position}: each focus node — both blank ones included, each as itself — was \
         handed to the relation bound as often as every other, and nothing else was — \
         {seen:?}"
    );
}

/// **An `OPTIONAL` arm: every focus node reaches the relation bound, and exactly the
/// violating IRI and the violating blank node are reported.**
#[test]
fn a_blank_focus_node_reaches_a_relation_bound_in_an_optional_arm() {
    let [(position, body), _, _] = bodies();
    check(position, &body);
}

/// **A sub-`SELECT`: the same claim.**
#[test]
fn a_blank_focus_node_reaches_a_relation_bound_in_a_sub_select() {
    let [_, (position, body), _] = bodies();
    check(position, &body);
}

/// **An `EXISTS` below the core: the same claim.**
#[test]
fn a_blank_focus_node_reaches_a_relation_bound_in_an_exists_below_the_core() {
    let [_, _, (position, body)] = bodies();
    check(position, &body);
}
