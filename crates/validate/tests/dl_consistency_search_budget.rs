// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! An ORDINARY satisfiable ontology decides far inside its search budget.
//!
//! The DL lane's step cap is a global search budget summed over every branch the hypertableau
//! explores, and an ontology reaching it is reported `unknown` — a resource limit, not an
//! answer. The seventeen triples below are as ordinary as an OWL 2 DL ontology gets: one
//! `owl:equivalentClass` over two anonymous restrictions, an `owl:inverseOf`, an `rdfs:range`,
//! one type assertion. They nonetheless exhausted the cap outright, because the terminology
//! reached the search internalized — every inclusion a disjunction seeded into every node's
//! label, branching per node per axiom — and the sum over those branches is what the cap
//! bounds.
//!
//! So this is a budget test rather than a verdict test, and it asserts four separate things:
//!
//! 1. the ontology is CONSISTENT, which is the answer a case split that never terminated could
//!    not give;
//! 2. the certificate says `completeness decided`, so the verdict is a decision and not a
//!    truncation reported as one;
//! 3. the rounds spent are under a TENTH of the budget the run itself declared;
//! 4. the WORK spent is under a tenth of the work budget the run itself declared.
//!
//! The fourth is not a restatement of the third. A round is a PASS rather than a unit of cost,
//! so "three percent of the round budget" is compatible with a search that runs for hours —
//! which is exactly what the co-typed shape in `dl_work_budget` does — and only a counted work
//! figure separates the two. An ontology this ordinary must be far inside BOTH ceilings.
//!
//! Both are asserted against the certificate's OWN numbers rather than against literals. The
//! caps are derived from the knowledge base's size (`step_cap` and `work_cap` in the reasoner
//! core), so a literal here would pin an implementation detail and would have to be revised
//! every time a derivation changed; a ratio between two rendered fields survives that and
//! keeps saying the same thing — this ontology is nowhere near either ceiling.
//!
//! The EXACT cost — rounds, peak nodes, case splits and branch depth — is pinned in
//! `dl_step_ledger` beside this ontology's control, where every exact search-cost figure in
//! the workspace lives in one table. This file is the narrative; that one is the ledger.
//!
//! # Why the restrictions carry no `rdf:type`
//!
//! Verbatim from the equivalence-over-untyped-restrictions ontology, and deliberately kept so.
//! The anonymous class
//! expressions are `owl:Restriction`s by their `owl:onProperty`/`owl:allValuesFrom` and
//! `owl:cardinality` triples alone — no `rdf:type owl:Restriction` states it — which is legal
//! OWL 2 RDF and is the shape the reverse mapping has to recognize structurally. Retyping them
//! would quietly test a different parse.

use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

mod common;
use common::measurement;

/// The equivalence-over-untyped-restrictions ontology, verbatim.
const ONTOLOGY: &str = r"
@prefix : <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:r owl:inverseOf :ri .
:ri rdfs:range :S .

:A owl:equivalentClass
        [
            owl:onProperty :r ;
            owl:allValuesFrom [
                owl:intersectionOf (
                    :S
                    [
                        owl:onProperty :p ;
                        owl:allValuesFrom :D
                    ]
                )
            ]
        ] ,
        [
            owl:onProperty :c ;
            owl:cardinality 1
        ] ;
    rdfs:subClassOf :S .

:a a :A .
";

#[test]
fn an_ordinary_ontology_decides_far_inside_its_step_budget() {
    let dataset =
        parse_dataset(ONTOLOGY.as_bytes(), "text/turtle", None).expect("the ontology parses");
    let bytes = serialize_dataset(&*dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("the ontology serializes");
    let document = String::from_utf8(bytes).expect("N-Quads is UTF-8");
    assert_eq!(
        document.lines().count(),
        17,
        "the fixture is the equivalence-over-untyped-restrictions ontology's seventeen \
         triples:\n{document}"
    );

    let answer = purrdf_validate::regime::consistency_to_string(&document, 0, 0)
        .expect("the ontology reverse-maps");
    assert_eq!(
        answer.answer(),
        "consistency true\n",
        "the ontology has a model:\n{}",
        answer.certificate()
    );

    let certificate = answer.certificate();
    assert!(
        certificate.contains("\ncompleteness decided\n"),
        "a decided verdict, not a truncated search:\n{certificate}"
    );

    let steps = measurement(certificate, "steps");
    let budget = measurement(certificate, "budget");
    assert!(
        steps * 10 < budget,
        "the search spent {steps} of its own declared {budget}-round budget, which is not \
         `far inside` it:\n{certificate}"
    );

    let work = measurement(certificate, "work");
    let work_budget = measurement(certificate, "work-budget");
    assert!(
        work * 10 < work_budget,
        "the search spent {work} of its own declared {work_budget}-unit work budget, which is \
         not `far inside` it. This is the ceiling the round count cannot see — see \
         `dl_work_budget` for the shape that reaches it while `steps` reports a few \
         percent:\n{certificate}"
    );
}
