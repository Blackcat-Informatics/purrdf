// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The OWL 2 reasoning services report terms in `TermValue`'s total order, in which
//! blank-node scope distinguishes terms.
//!
//! Read through the public entry points: [`Reasoner::instances`] (instance
//! retrieval) and [`profile`] (profile violations), over a graph whose individuals
//! and violating nodes are IRIs and blank nodes of several scopes sharing one label.
//! The retired sort key joined a term's parts into one string and tied those blank
//! nodes; the order is now `TermValue::cmp`. The expectation is computed with
//! `TermValue::cmp` and also pinned as a literal sequence, so a change of the order
//! is a visible diff. Individuals and profile-violating nodes are IRIs or blank
//! nodes (an individual is never a literal), so those are the kinds a report can
//! mix.
//!
//! Fixtures use `example.org` throughout; every IRI is fixture configuration.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_entail::reasoner::{Reasoner, profile};
use purrdf_iri::vocab::owl::{CLASS, FUNCTIONAL_PROPERTY};
use purrdf_iri::vocab::rdf::TYPE;

const CLASS_C: &str = "http://example.org/C";

fn blank(label: &str, scope: u32) -> TermValue {
    TermValue::Blank {
        label: label.into(),
        scope: BlankScope(scope),
    }
}

/// The nodes, deliberately not in order, and two blank nodes that share a label.
fn nodes() -> Vec<TermValue> {
    vec![
        TermValue::iri("http://example.org/i2"),
        TermValue::iri("http://example.org/i1"),
        blank("x", 2),
        blank("x", 1),
        blank("x", 0),
        blank("a", 1),
    ]
}

fn sorted_by_cmp() -> Vec<TermValue> {
    let mut nodes = nodes();
    nodes.sort();
    nodes
}

/// `node a <class>` for every node.
fn typed_by(class: &str, declare_class: bool) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(TYPE);
    let class = b.intern_iri(class);
    if declare_class {
        let owl_class = b.intern_iri(CLASS);
        b.push_quad(class, ty, owl_class, None);
    }
    for node in nodes() {
        let id = match &node {
            TermValue::Iri(iri) => b.intern_iri(iri),
            TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
            other => unreachable!("fixture nodes are IRIs and blank nodes: {other:?}"),
        };
        b.push_quad(id, ty, class, None);
    }
    b.freeze().expect("freeze")
}

fn describe(terms: &[TermValue]) -> Vec<String> {
    terms
        .iter()
        .map(|term| match term {
            TermValue::Iri(iri) => format!("iri {iri}"),
            TermValue::Blank { label, scope } => format!("blank {label} scope {}", scope.ordinal()),
            other => unreachable!("fixture nodes are IRIs and blank nodes: {other:?}"),
        })
        .collect()
}

/// IRIs, then blank nodes by label and then scope.
const PINNED: [&str; 6] = [
    "iri http://example.org/i1",
    "iri http://example.org/i2",
    "blank a scope 1",
    "blank x scope 0",
    "blank x scope 1",
    "blank x scope 2",
];

#[test]
fn instance_retrieval_lists_individuals_in_term_value_order() {
    let dataset = typed_by(CLASS_C, true);
    let mut reasoner = Reasoner::new(&dataset).expect("the ontology loads");
    let certified = reasoner
        .instances(&TermValue::iri(CLASS_C))
        .expect("the ontology is satisfiable");
    let answer = certified.answer().clone();
    assert_eq!(answer, sorted_by_cmp(), "instances follow TermValue::cmp");
    assert_eq!(
        answer.len(),
        6,
        "same-label blank nodes of different scopes do not tie"
    );
    assert_eq!(PINNED.to_vec(), describe(&answer));
}

#[test]
fn profile_violations_list_nodes_in_term_value_order() {
    let dataset = typed_by(FUNCTIONAL_PROPERTY, false);
    let certificate = profile(&dataset);
    // Every node violates each profile that excludes the construct; within one
    // profile the subjects are in TermValue order.
    assert!(
        !certificate.violations().is_empty(),
        "the construct is excluded by some profile"
    );
    let mut seen = Vec::new();
    for violation in certificate.violations() {
        if !seen.contains(&violation.profile()) {
            seen.push(violation.profile());
        }
    }
    for wanted in seen {
        let subjects: Vec<TermValue> = certificate
            .violations()
            .iter()
            .filter(|violation| violation.profile() == wanted)
            .map(|violation| violation.subject().clone())
            .collect();
        assert_eq!(
            subjects,
            sorted_by_cmp(),
            "{wanted:?}: subjects follow TermValue::cmp"
        );
        assert_eq!(PINNED.to_vec(), describe(&subjects), "{wanted:?}");
    }
}
