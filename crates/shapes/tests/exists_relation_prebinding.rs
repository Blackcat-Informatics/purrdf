// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A focus node reaches a relation call inside a constraint's `FILTER EXISTS` or
//! `FILTER NOT EXISTS` body bound as itself, whatever kind of term it is.
//!
//! SHACL pre-binds `$this` into an `EXISTS` body as everywhere else. An IRI or a
//! literal focus node is written into the call's argument as a constant. A blank node
//! or a quoted triple has no constant spelling there, so it is driven into the call by
//! a one-row `VALUES` — and inside an `EXISTS` body that `VALUES` used to bind `$this`
//! again, on a row that already binds it, which the evaluator refuses as a rebinding.
//! Every blank-node and quoted-triple focus node failed the validation outright, while
//! the same constraint validated an IRI focus node.
//!
//! # The oracle
//!
//! The focus nodes are the objects of `ex:mark` — every term kind an object can be:
//! an IRI, a literal, a blank node, a quoted triple, a quoted triple with a blank node
//! inside it and one with a literal inside it — two of each, `good` and `bad`, so the
//! two blank nodes (and the two nested ones) are different nodes with different
//! verdicts. The relation approves exactly the `good` ones, by term identity: it holds
//! the dataset's own terms for them and answers a bound subject with one row when the
//! subject is one of them and none otherwise. It records every invocation with the
//! exact term it was handed, and a free one as free.
//!
//! Every run is then held to the exact report — which focus nodes violate, each by the
//! dataset's own id for it, so a blank focus node reported under the other blank
//! node's verdict fails — and to the relation's invocations: bound every time, each
//! focus node handed to it as itself, as often as every other, and nothing else.
//!
//! Fixture IRIs are `example.org`; PurRDF mints none.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
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
const REL_IRI: &str = "http://example.org/rel/approved";

/// The kinds of focus node, each named by the holder IRI that marks it.
const KINDS: [&str; 6] = ["iri", "lit", "blank", "qt", "qtblank", "qtlit"];

/// The two verdicts every kind has one focus node for.
const VERDICTS: [&str; 2] = ["good", "bad"];

/// The focus node of `kind` and `verdict`, in Turtle.
fn turtle_term(kind: &str, verdict: &str) -> String {
    match kind {
        "iri" => format!("<{EX}{verdict}>"),
        "lit" => format!("\"{verdict}\""),
        "blank" => format!("_:{verdict}"),
        "qt" => format!("<<( <{EX}a> <{EX}r> <{EX}{verdict}> )>>"),
        "qtblank" => format!("<<( _:q{verdict} <{EX}r> <{EX}b> )>>"),
        "qtlit" => format!("<<( <{EX}a> <{EX}r> \"{verdict}\" )>>"),
        other => panic!("no fixture kind {other}"),
    }
}

/// Every focus node, each as the object of `ex:mark` on a holder of its own.
fn data() -> Arc<RdfDataset> {
    let mut turtle = String::new();
    for kind in KINDS {
        for verdict in VERDICTS {
            writeln!(
                turtle,
                "<{EX}h-{kind}-{verdict}> <{EX}mark> {} .",
                turtle_term(kind, verdict)
            )
            .expect("writing to a String cannot fail");
        }
    }
    purrdf_shapes::text_ingest::parse_turtle_to_dataset(&turtle, None)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

/// The focus node of `kind` and `verdict`, by the id the dataset holds it under: the
/// object of its holder's one `ex:mark` quad.
fn node(dataset: &RdfDataset, kind: &str, verdict: &str) -> TermId {
    let holder = dataset
        .term_id_by_iri(&format!("{EX}h-{kind}-{verdict}"))
        .expect("the holder is in the dataset");
    let objects: Vec<TermId> = dataset
        .quads()
        .filter(|quad| quad.s == holder)
        .map(|quad| quad.o)
        .collect();
    assert_eq!(objects.len(), 1, "{kind}-{verdict}: one marked node");
    objects[0]
}

/// The dataset's own id for a term the validation or the relation handed back.
fn id_in(dataset: &RdfDataset, value: &TermValue, what: &str) -> TermId {
    dataset
        .term_id_by_value(value)
        .unwrap_or_else(|| panic!("{what} {value:?} is a node the dataset holds"))
}

/// Approves exactly the terms it was built with, by identity.
#[derive(Debug)]
struct Approved {
    modes: Vec<BindingPattern>,
    good: Vec<TermValue>,
    /// Every invocation's subject: `Some(term)` bound, `None` free.
    seen: Mutex<Vec<Option<TermValue>>>,
}

struct Rows(std::vec::IntoIter<PfRow>);

impl PfCursor for Rows {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.0.next())
    }
}

impl PropertyFunction for Approved {
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
        let subject = args.get(0).cloned();
        self.seen.lock().expect("unpoisoned").push(subject.clone());
        let reason = TermValue::simple_literal("approved");
        let rows: Vec<PfRow> = match subject {
            Some(subject) => self
                .good
                .iter()
                .filter(|approved| **approved == subject)
                .map(|approved| vec![approved.clone(), reason.clone()])
                .collect(),
            None => self
                .good
                .iter()
                .map(|approved| vec![approved.clone(), reason.clone()])
                .collect(),
        };
        Ok(Box::new(Rows(rows.into_iter())))
    }
}

/// One shape targeting every marked node, whose `sh:select` body is `body`.
fn shapes(body: &str) -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:MarkShape
    a sh:NodeShape ;
    sh:targetObjectsOf ex:mark ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "the relation's verdict" ;
        sh:select """{body}""" ;
    ] .
"#
    );
    parse_shapes(&turtle, None).expect("parse shapes")
}

/// Which verdict a body reports as a violation.
#[derive(Clone, Copy, Debug)]
enum Reports {
    /// The unapproved (`bad`) nodes: a `NOT EXISTS` over the call.
    Unapproved,
    /// The approved (`good`) nodes: an `EXISTS` over the call.
    Approved,
}

/// Validate every marked node under `body` against a relation declaring `modes`, and
/// hold the report and the invocations to the claim.
fn check(body: &str, reports: Reports, modes: &[&str]) {
    let context = format!("{body} under {modes:?}");
    let dataset = data();
    let good: BTreeSet<TermId> = KINDS
        .iter()
        .map(|kind| node(&dataset, kind, "good"))
        .collect();
    let bad: BTreeSet<TermId> = KINDS
        .iter()
        .map(|kind| node(&dataset, kind, "bad"))
        .collect();
    let relation = Arc::new(Approved {
        modes: modes
            .iter()
            .map(|code| BindingPattern::from_code(code))
            .collect(),
        good: good.iter().map(|&id| dataset.term_value(id)).collect(),
        seen: Mutex::new(Vec::new()),
    });
    let mut registry = PropertyFunctionRegistry::new();
    let registered: Arc<dyn PropertyFunction> = Arc::<Approved>::clone(&relation);
    registry.register(REL_IRI.to_owned(), registered);

    let report = {
        let _relations = enter_property_function_scope(Arc::new(registry));
        validate_dataset(&dataset, &shapes(body))
            .unwrap_or_else(|error| panic!("{context}: the validation runs: {error}"))
    };

    let expected = match reports {
        Reports::Unapproved => &bad,
        Reports::Approved => &good,
    };
    let reported: BTreeSet<TermId> = report
        .results
        .iter()
        .map(|result| {
            id_in(
                &dataset,
                &result.focus_node.to_term_value(),
                "the focus node",
            )
        })
        .collect();
    assert_eq!(
        &reported, expected,
        "{context}: exactly the {reports:?} nodes of every kind are reported, each as \
         itself — {report:?}"
    );
    assert_eq!(report.results.len(), KINDS.len(), "{context}: {report:?}");
    assert!(!report.conforms, "{context}");

    // Every invocation is bound — a free one would be `None` — and hands the relation
    // one of the focus nodes, as itself, as often as every other focus node.
    let seen = relation.seen.lock().expect("unpoisoned").clone();
    let mut bound: BTreeMap<TermId, usize> = BTreeMap::new();
    for subject in &seen {
        let subject = subject
            .as_ref()
            .unwrap_or_else(|| panic!("{context}: every invocation is bound — {seen:?}"));
        *bound
            .entry(id_in(&dataset, subject, "a bound subject"))
            .or_default() += 1;
    }
    let per_node = bound
        .get(&node(&dataset, "iri", "good"))
        .copied()
        .unwrap_or_default();
    assert!(
        per_node > 0,
        "{context}: the IRI focus node reached it — {seen:?}"
    );
    let every: BTreeMap<TermId, usize> = good
        .iter()
        .chain(bad.iter())
        .map(|&id| (id, per_node))
        .collect();
    assert_eq!(
        bound, every,
        "{context}: each focus node — every blank and quoted one included, each as \
         itself — was handed to the relation bound as often as every other, and nothing \
         else was — {seen:?}"
    );
}

fn not_exists() -> String {
    format!("SELECT $this WHERE {{ FILTER NOT EXISTS {{ $this <{REL_IRI}> ?why }} }}")
}

fn exists() -> String {
    format!("SELECT $this WHERE {{ FILTER EXISTS {{ $this <{REL_IRI}> ?why }} }}")
}

/// The call after an atom in the same `EXISTS` body, which the parser writes as the
/// right operand of a `LATERAL` over the atom.
fn not_exists_after_atom() -> String {
    format!(
        "SELECT $this WHERE {{ FILTER NOT EXISTS {{ ?holder <{EX}mark> $this . \
           $this <{REL_IRI}> ?why }} }}"
    )
}

/// **`FILTER NOT EXISTS` over a bound-only relation: every unapproved node of every
/// kind is reported, as itself, and every focus node reaches the relation bound.**
#[test]
fn not_exists_over_a_bound_only_relation() {
    check(&not_exists(), Reports::Unapproved, &["bf"]);
}

/// **The same with a relation that also serves the free mode**: a focus node left
/// free would be answered from the whole extent, approving every node — and would be
/// recorded as a free invocation.
#[test]
fn not_exists_over_a_free_capable_relation() {
    check(&not_exists(), Reports::Unapproved, &["bf", "ff"]);
}

/// **`FILTER EXISTS` over a bound-only relation: every approved node is reported.**
#[test]
fn exists_over_a_bound_only_relation() {
    check(&exists(), Reports::Approved, &["bf"]);
}

/// **`FILTER EXISTS` over a relation that also serves the free mode.**
#[test]
fn exists_over_a_free_capable_relation() {
    check(&exists(), Reports::Approved, &["bf", "ff"]);
}

/// **The call after an atom inside the `EXISTS` body**, under both relations.
#[test]
fn not_exists_with_the_call_after_an_atom() {
    check(&not_exists_after_atom(), Reports::Unapproved, &["bf"]);
    check(&not_exists_after_atom(), Reports::Unapproved, &["bf", "ff"]);
}
