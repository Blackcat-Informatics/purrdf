// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! THE CO-TYPED SHAPE: the class of ontology the round cap cannot bound, and the answer it
//! gets now.
//!
//! `dl_consistency_search_budget` holds one copy of the reported ontology — an
//! `owl:equivalentClass` over two untyped restrictions, an `owl:inverseOf`, an `rdfs:range`,
//! one type assertion — and shows it decides far inside its budgets. This file holds `n`
//! copies of it, each over its own vocabulary, all asserted of ONE individual. That single
//! difference is the whole fixture: the converse direction of each equivalence has an
//! antecedent no faithful absorption can guard and so reaches the search as a disjunction, and
//! `n` such disjunctions on ONE node interleave instead of standing beside each other.
//!
//! # What the round cap could not see
//!
//! A derivation round is a PASS over the completion graph, not a unit of cost: its price is
//! the graph it runs over times the clauses matched against it. Co-typing multiplies that
//! price — the matcher's join steps, the successor-subset enumerations a `≤n` clause body
//! walks, the achiever closures every neighbourhood read takes, the branch-state clone each
//! alternative starts from — while the number of rounds grows far more slowly. Measured
//! UNCAPPED, this family costs about nine times as much work per added copy: 5.4 million units
//! at three copies, 77 million at four, 695 million at five, 4.4 BILLION at six. At ten copies
//! it does not finish, and the failure it used to fail with was the dangerous kind — the run
//! ground on while its certificate reported `steps` at a few percent of the round budget,
//! which reads exactly like a search with plenty of room left.
//!
//! # What this file asserts
//!
//! Two copies DECIDE, `consistency true` under `completeness decided`, with both budgets
//! largely unspent — so the work cap is not a blunt instrument that refuses the shape.
//!
//! Ten copies answer `consistency unknown` under `completeness budget-exhausted`, with `work`
//! equal to `work-budget` — which is the certificate saying, in its own two numbers, that it
//! was the WORK cap and not the round cap that ended the run. That equality is the assertion
//! this file exists for: `steps` stays far below `budget` in the same certificate, so a
//! reader who had only the round figures would still see a search with room to spare.
//!
//! Promptness is BY CONSTRUCTION and is deliberately not asserted with a clock. The search
//! stops after a bounded, counted amount of work — every enumerator polls the same meter — so
//! a wall-time assertion would add a flake without adding a fact. The measured figure, for the
//! record rather than for the gate: ten copies answer in about six tenths of a second where
//! they previously did not answer at all.

use std::fmt::Write as _;

use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

mod common;
use common::measurement;

/// `blocks` copies of the reported ontology, each over its own vocabulary, ALL asserted of the
/// single individual `:a`.
///
/// Every block is the reported seventeen-triple shape with its names suffixed, so a block is
/// the same ontology this workspace already ledgers rather than a fixture invented to be
/// expensive. The restrictions deliberately carry no `rdf:type owl:Restriction`, exactly as
/// the reported ontology did not: they are restrictions by their `owl:onProperty` /
/// `owl:allValuesFrom` / `owl:cardinality` triples alone, which is legal OWL 2 RDF and is the
/// shape the reverse mapping must recognize structurally.
fn co_typed(blocks: usize) -> String {
    let mut out = String::from(
        "@prefix : <https://example.org/> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n",
    );
    for k in 0..blocks {
        let _ = write!(
            out,
            ":r{k} owl:inverseOf :ri{k} .\n\
             :ri{k} rdfs:range :S{k} .\n\
             :A{k} owl:equivalentClass\n\
             \x20       [\n\
             \x20           owl:onProperty :r{k} ;\n\
             \x20           owl:allValuesFrom [\n\
             \x20               owl:intersectionOf (\n\
             \x20                   :S{k}\n\
             \x20                   [\n\
             \x20                       owl:onProperty :p{k} ;\n\
             \x20                       owl:allValuesFrom :D{k}\n\
             \x20                   ]\n\
             \x20               )\n\
             \x20           ]\n\
             \x20       ] ,\n\
             \x20       [\n\
             \x20           owl:onProperty :c{k} ;\n\
             \x20           owl:cardinality 1\n\
             \x20       ] ;\n\
             \x20   rdfs:subClassOf :S{k} .\n\
             :a a :A{k} .\n\n"
        );
    }
    out
}

/// The fixture as the canonical N-Quads the string boundary parses.
fn document(blocks: usize) -> String {
    let ontology = co_typed(blocks);
    let dataset =
        parse_dataset(ontology.as_bytes(), "text/turtle", None).expect("the ontology parses");
    let bytes = serialize_dataset(&*dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("the ontology serializes");
    String::from_utf8(bytes).expect("N-Quads is UTF-8")
}

/// Two co-typed copies still DECIDE, and nowhere near either ceiling.
///
/// The half of the claim that keeps the other half honest: a work cap that refused this shape
/// outright would satisfy the exhaustion test below while making the reasoner answer less.
#[test]
fn two_co_typed_copies_decide_inside_both_budgets() {
    let document = document(2);
    let answer = purrdf_validate::regime::consistency_to_string(&document, 0, 0)
        .expect("the ontology reverse-maps");
    let certificate = answer.certificate();
    assert_eq!(
        answer.answer(),
        "consistency true\n",
        "two co-typed copies have a model, and the search finds it:\n{certificate}"
    );
    assert!(
        certificate.contains("\ncompleteness decided\n"),
        "a decided verdict, not a truncated search:\n{certificate}"
    );
    let work = measurement(certificate, "work");
    let work_budget = measurement(certificate, "work-budget");
    assert!(
        work * 10 < work_budget,
        "two copies spent {work} of a {work_budget}-unit work budget, which is not `far \
         inside` it — the cap has tightened onto a case it is supposed to decide \
         comfortably:\n{certificate}"
    );
}

/// TEN co-typed copies answer — `unknown`, `budget-exhausted`, and the WORK cap is what says
/// so.
///
/// Three assertions, and the third is the one this file exists for. `work == work-budget`
/// identifies which ceiling ended the run, and `steps * 10 < budget` in the SAME certificate
/// is the proof that the round cap could not have: the search stopped with over ninety percent
/// of its rounds unspent.
#[test]
fn ten_co_typed_copies_answer_unknown_at_the_work_cap() {
    let document = document(10);
    let answer = purrdf_validate::regime::consistency_to_string(&document, 0, 0)
        .expect("the ontology reverse-maps");
    let certificate = answer.certificate();
    assert_eq!(
        answer.answer(),
        "consistency unknown\n",
        "a truncated search answers `unknown`, never `false` — reporting a resource limit as \
         an entailment is the one substitution this lane refuses:\n{certificate}"
    );
    assert!(
        certificate.contains("\ncompleteness budget-exhausted\n"),
        "the certificate says the search was truncated:\n{certificate}"
    );

    let work = measurement(certificate, "work");
    let work_budget = measurement(certificate, "work-budget");
    assert_eq!(
        work, work_budget,
        "the run stopped at its WORK cap, so the two figures are the same number:\n{certificate}"
    );

    let steps = measurement(certificate, "steps");
    let budget = measurement(certificate, "budget");
    assert!(
        steps * 10 < budget,
        "the round figures must show what they could NOT see: this search exhausted itself \
         while spending {steps} of a {budget}-round budget. If that ratio ever stops holding, \
         the round cap has started catching this shape and this test's premise needs \
         re-reading:\n{certificate}"
    );
}

/// The exhausted answer is byte-identical run to run, certificate included.
///
/// The determinism doctrine reaches the new figures too: `work` is counted off the search —
/// edges scanned, body atoms joined, subsets enumerated, nodes cloned — and never off a clock,
/// so two runs of a search that STOPS at its budget stop in the same place.
#[test]
fn the_exhausted_answer_is_identical_twice() {
    let document = document(10);
    let first = purrdf_validate::regime::consistency_to_string(&document, 0, 0)
        .expect("the ontology reverse-maps");
    let again = purrdf_validate::regime::consistency_to_string(&document, 0, 0)
        .expect("the ontology reverse-maps");
    assert_eq!(
        first, again,
        "two runs, one rendering — answer AND certificate, the work figures included"
    );
}
