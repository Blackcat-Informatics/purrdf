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
//! a conforming IRI, and a violating and a conforming blank node — and the violating
//! nodes are told apart by the status literal each one owns, so a blank focus node
//! whose identity was swapped for the other blank's would report the wrong value or
//! none.
//!
//! Fixture IRIs are `example.org`; PurRDF mints none.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use purrdf::RdfDataset;
use purrdf_core::TermValue;
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
    /// Every subject a bound invocation was handed, rendered.
    seen: Mutex<Vec<String>>,
}

impl SelfRelation {
    fn seen(&self) -> Vec<String> {
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
        let rendered = match subject {
            TermValue::Blank { .. } => "blank".to_owned(),
            TermValue::Iri(iri) => iri.clone(),
            other => format!("{other:?}"),
        };
        self.seen.lock().expect("unpoisoned").push(rendered);
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
/// bound, and exactly the two violating nodes are reported, each with its own status.
fn check(position: &str, body: &str) {
    let (registry, relation) = relation();
    let report = {
        let _relations = enter_property_function_scope(registry);
        validate_dataset(&items(), &shapes(body))
            .unwrap_or_else(|error| panic!("{position}: the validation runs: {error}"))
    };
    assert!(!report.conforms, "{position}: {report:?}");
    let reported: BTreeSet<(bool, String)> = report
        .results
        .iter()
        .map(|result| {
            (
                result.focus_node.to_string().starts_with("_:"),
                result
                    .value
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )
        })
        .collect();
    let expected: BTreeSet<(bool, String)> = [
        (false, "\"bad iri\"".to_owned()),
        (true, "\"bad blank\"".to_owned()),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        reported, expected,
        "{position}: exactly the violating IRI and the violating blank node, each with \
         the status it owns — {report:?}"
    );
    assert_eq!(report.results.len(), 2, "{position}: {report:?}");

    // Every focus node reached the relation bound — the relation refuses a free
    // subject, so every recorded invocation is a bound one — and the blank focus
    // nodes were handed to it exactly as often as the IRI ones: the validation
    // runs its query the same number of times per focus node whatever its kind.
    let seen = relation.seen();
    let count = |subject: &str| seen.iter().filter(|seen| seen.as_str() == subject).count();
    let per_iri = count(&format!("{EX}iri-bad"));
    assert!(
        per_iri > 0,
        "{position}: the IRI focus nodes reached it — {seen:?}"
    );
    assert_eq!(
        count(&format!("{EX}iri-good")),
        per_iri,
        "{position}: {seen:?}"
    );
    assert_eq!(
        count("blank"),
        2 * per_iri,
        "{position}: both blank focus nodes were handed to the relation bound, as often \
         as each IRI one — {seen:?}"
    );
    assert_eq!(
        seen.len(),
        4 * per_iri,
        "{position}: nothing else — {seen:?}"
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
