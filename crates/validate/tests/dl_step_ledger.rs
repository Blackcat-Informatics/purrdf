// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! THE STEP LEDGER: every exact search-cost figure this workspace pins, in one table.
//!
//! # Why one table rather than an assertion where each fixture lives
//!
//! A DL search's cost is a deterministic function of the ontology and the calculus, so it can
//! be pinned exactly — and an exact pin is the only kind that catches the failure this ledger
//! exists for. The defect that motivated this file was not a wrong verdict: it was an ordinary
//! seventeen-triple ontology that ANSWERED CORRECTLY while spending its entire search budget,
//! and every verdict test in the workspace passed while it did. Only a number moving says that.
//!
//! Scattered across the fixtures they belong to, such numbers are individually unreadable: each
//! one is a literal in a test about something else, and a reviewer looking at a diff that moves
//! four of them has no way to see them as one fact about the search. Gathered here they are a
//! LEDGER — a re-pin is one reviewable diff over one table, and a row that moves in the wrong
//! direction is visible beside the rows that did not.
//!
//! # What a row pins, and why all five numbers
//!
//! `steps` is the round count one budget is denominated in, and `work` is the count the OTHER
//! is: a round is a pass rather than a unit of cost, so an ontology can make each round
//! enormously more expensive without taking more rounds, and only `work` moves when it does.
//! The remaining three say where the cost went, which neither total can: `peak-nodes` is the
//! largest completion graph a decision built, `disjunctions` how many times the `⊔`-rule case
//! split, `peak-depth` how deep that rule's branch stack got. A change that halves the rounds
//! by building a graph twice as large is not the same change as one that halves them by
//! splitting half as often, and a ledger holding only `steps` would call them the same.
//!
//! Every figure is read out of the RENDERED certificate rather than from a Rust API, because
//! the rendering is what a caller across the Python, WASM and C boundaries actually sees — each
//! of those surfaces carries this exact string.
//!
//! # The two guards every decided row also passes
//!
//! An exact pin says a number did not move. It does not say the number is GOOD, and a
//! re-pinning that walked every row back towards a cap one commit at a time would satisfy it
//! the whole way. So each decided row is additionally held to `steps × 10 < budget` AND
//! `work × 10 < work-budget` — the ontology is nowhere near EITHER ceiling — and decided
//! TWICE, with the whole rendering compared, so determinism is asserted at the boundary and
//! not only inside the decision core.
//!
//! # Determinism
//!
//! Both caps are pure functions of the knowledge base's size and the search reads no clock
//! and no hash map, so every figure below is reproducible on every machine and on `wasm32`.
//! That is what makes an exact pin honest here and a flake anywhere a clock is involved.

use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

mod common;
use common::measurement;

/// One ledgered fixture: an ontology, and exactly what deciding it costs.
struct Pin {
    /// The fixture's name, which names its SHAPE — the constructs whose interaction the
    /// search cost is about — so a row stays legible after whatever prompted it is forgotten.
    name: &'static str,
    /// The ontology, in Turtle.
    ontology: &'static str,
    /// How many triples it is, so a fixture edited into a different ontology fails HERE
    /// rather than silently re-pinning a number for something else.
    triples: usize,
    /// The per-decision ROUND cap to decide under; `0` means the knowledge base's own derived
    /// cap, which is what every row but the deliberately-truncated one uses.
    step_cap: u32,
    /// The per-decision WORK cap to decide under; `0` means the knowledge base's own derived
    /// cap, which is what every row uses — the truncated row truncates on ROUNDS, so that the
    /// two caps are exercised by different rows rather than one masking the other.
    work_cap: u32,
    /// The `consistency` answer line, verbatim.
    answer: &'static str,
    /// The certificate's `completeness` value, verbatim.
    completeness: &'static str,
    /// The exact `steps` figure.
    steps: u64,
    /// The exact `work` figure.
    work: u64,
    /// The exact `peak-nodes` figure.
    peak_nodes: u64,
    /// The exact `disjunctions` figure.
    disjunctions: u64,
    /// The exact `peak-depth` figure.
    peak_depth: u64,
}

/// The equivalence-over-untyped-restrictions ontology: an `owl:equivalentClass` over two
/// untyped restrictions — a `∀`-restriction whose filler is an intersection, and an exact
/// cardinality — beside an `owl:inverseOf` and an `rdfs:range`.
///
/// Seventeen triples, and the restrictions deliberately carry no `rdf:type owl:Restriction`:
/// they are restrictions by their `owl:onProperty` / `owl:allValuesFrom` / `owl:cardinality`
/// triples alone, which is legal OWL 2 RDF and is the shape the reverse mapping has to
/// recognize structurally. Retyping them would quietly pin a different parse.
///
/// This ontology once exhausted its search budget outright. The row below is what says it no
/// longer does, and by how much.
const EQUIVALENT_CLASS_SHAPE: &str = r"
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

/// A STRONGER, both-moved variant: the same seventeen triples with BOTH restrictions moved
/// from `owl:equivalentClass` to `rdfs:subClassOf`, so the fixture states no equivalence at
/// all.
///
/// This is NOT the shape the cardinality-only-equivalence control below isolates — see
/// [`CARDINALITY_EQUIVALENCE_SHAPE`] for that one. It is kept alongside it as a second, more
/// extreme data point: with NEITHER restriction reachable as a non-atomic antecedent, the
/// search absorbs into guarded clauses that branch not at all, at whatever cost the inverse
/// role and the range still book.
///
/// A pinned number is only evidence if the thing it measures could have come out otherwise,
/// and holding this row beside [`EQUIVALENT_CLASS_SHAPE`] is part of what shows that shape's
/// cost is about the equivalence rather than about the restrictions, the inverse role or the
/// range.
const SUBCLASS_SHAPE: &str = r"
@prefix : <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:r owl:inverseOf :ri .
:ri rdfs:range :S .

:A rdfs:subClassOf
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
        ] ,
        :S .

:a a :A .
";

/// THE CARDINALITY-ONLY-EQUIVALENCE CONTROL: the same seventeen triples with ONLY the
/// `∀`-restriction moved from `owl:equivalentClass` to `rdfs:subClassOf`. The
/// `owl:cardinality 1` restriction stays exactly where [`EQUIVALENT_CLASS_SHAPE`] put it, as an
/// `owl:equivalentClass` member.
///
/// The `∀`-restriction, not the equivalence as such, is what the search cannot guard:
/// `∀r.(S ⊓ ∀p.D) ⊑ A`, the converse direction an
/// equivalence over a universal restriction reaches the search as, has no atomic antecedent a
/// faithful absorption can trigger on. `=1 c ⊑ A` — the cardinality restriction's own
/// converse — does: a `≥1`/`≤1` pair guards on the `≥1` conjunct with the `≤1` half moved to
/// the head, the same disposition an ordinary qualified cardinality gets. So moving ONLY the
/// `∀`-restriction should already recover most of [`SUBCLASS_SHAPE`]'s cost while the fixture
/// still states an equivalence — which is what makes this the control that isolates the
/// actual culprit, rather than [`SUBCLASS_SHAPE`]'s stronger both-moved rewrite.
const CARDINALITY_EQUIVALENCE_SHAPE: &str = r"
@prefix : <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:r owl:inverseOf :ri .
:ri rdfs:range :S .

:A owl:equivalentClass
        [
            owl:onProperty :c ;
            owl:cardinality 1
        ] ;
    rdfs:subClassOf
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
        :S .

:a a :A .
";

/// An ordinary sub-class chain with one typed individual — the cheapest ontology that still
/// runs a search.
///
/// Ledgered under a step cap of ONE, so it is the row that pins the TRUNCATED path: the
/// answer is `unknown` (never `false`, which would report a resource limit as an entailment),
/// the completeness is `budget-exhausted`, and the search spent exactly the one round it was
/// given. That combination is what makes the exhausted path reachable from a test at all, and
/// pinning its cost here keeps the truncation exact rather than merely present.
const NARROWED_BUDGET_SHAPE: &str = r"
@prefix : <https://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Kitten rdfs:subClassOf :Cat .
:Cat rdfs:subClassOf :Animal .
:tom a :Kitten .
";

/// THE CO-TYPED SHAPE, at two copies: the same seventeen triples twice over, each copy with
/// its own vocabulary, BOTH asserted of one individual.
///
/// The row that pins the shape the WORK cap exists for, at the size that still decides. Each
/// copy contributes a disjunction no absorption can guard, and co-typing makes them interleave
/// on ONE node rather than stand beside each other: two copies already cost sixty-six times
/// the rounds and sixty-seven times the work of one, where two INDEPENDENT copies (one
/// individual each) cost about twice. That ratio is what this row exists to hold still — the
/// full curve, and where it stops deciding, is in `dl_work_budget` and in the reasoner core's
/// own `work_cap` documentation.
const CO_TYPED_SHAPE: &str = r"
@prefix : <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:r0 owl:inverseOf :ri0 .
:ri0 rdfs:range :S0 .

:A0 owl:equivalentClass
        [
            owl:onProperty :r0 ;
            owl:allValuesFrom [
                owl:intersectionOf (
                    :S0
                    [
                        owl:onProperty :p0 ;
                        owl:allValuesFrom :D0
                    ]
                )
            ]
        ] ,
        [
            owl:onProperty :c0 ;
            owl:cardinality 1
        ] ;
    rdfs:subClassOf :S0 .

:a a :A0 .

:r1 owl:inverseOf :ri1 .
:ri1 rdfs:range :S1 .

:A1 owl:equivalentClass
        [
            owl:onProperty :r1 ;
            owl:allValuesFrom [
                owl:intersectionOf (
                    :S1
                    [
                        owl:onProperty :p1 ;
                        owl:allValuesFrom :D1
                    ]
                )
            ]
        ] ,
        [
            owl:onProperty :c1 ;
            owl:cardinality 1
        ] ;
    rdfs:subClassOf :S1 .

:a a :A1 .
";

/// THE SCHEMA-HEAVY SHAPE: many disjoint `rdfs:domain`/`rdfs:range` pairs over a tiny ABox.
///
/// Twenty-four properties, each declared `rdfs:domain :Dᵢ` and `rdfs:range :Rᵢ` and never once
/// used by the three-triple ABox below — so every one of the forty-eight axioms is a CLASS-FREE
/// guard (`purrdf_entail`'s `owl_dl::clause::ClauseSet::untriggered`) tried at every node of
/// every round, over a role the completion graph has no edge for. That is the shape the
/// hypertableau's per-role achiever memo (`owl_dl::graph::Graph`'s role-achiever cache) exists
/// for: before it, each of those tries rebuilt property `pᵢ`'s (trivial, one-element) achiever
/// closure from scratch — `axioms × nodes × rounds` closure builds — where the property count
/// is the whole reason to grow the ontology and the ABox deliberately does not.
const SCHEMA_HEAVY_SHAPE: &str = r"
@prefix : <https://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:p0 rdfs:domain :D0 ;
      rdfs:range :R0 .
:p1 rdfs:domain :D1 ;
      rdfs:range :R1 .
:p2 rdfs:domain :D2 ;
      rdfs:range :R2 .
:p3 rdfs:domain :D3 ;
      rdfs:range :R3 .
:p4 rdfs:domain :D4 ;
      rdfs:range :R4 .
:p5 rdfs:domain :D5 ;
      rdfs:range :R5 .
:p6 rdfs:domain :D6 ;
      rdfs:range :R6 .
:p7 rdfs:domain :D7 ;
      rdfs:range :R7 .
:p8 rdfs:domain :D8 ;
      rdfs:range :R8 .
:p9 rdfs:domain :D9 ;
      rdfs:range :R9 .
:p10 rdfs:domain :D10 ;
      rdfs:range :R10 .
:p11 rdfs:domain :D11 ;
      rdfs:range :R11 .
:p12 rdfs:domain :D12 ;
      rdfs:range :R12 .
:p13 rdfs:domain :D13 ;
      rdfs:range :R13 .
:p14 rdfs:domain :D14 ;
      rdfs:range :R14 .
:p15 rdfs:domain :D15 ;
      rdfs:range :R15 .
:p16 rdfs:domain :D16 ;
      rdfs:range :R16 .
:p17 rdfs:domain :D17 ;
      rdfs:range :R17 .
:p18 rdfs:domain :D18 ;
      rdfs:range :R18 .
:p19 rdfs:domain :D19 ;
      rdfs:range :R19 .
:p20 rdfs:domain :D20 ;
      rdfs:range :R20 .
:p21 rdfs:domain :D21 ;
      rdfs:range :R21 .
:p22 rdfs:domain :D22 ;
      rdfs:range :R22 .
:p23 rdfs:domain :D23 ;
      rdfs:range :R23 .

:a0 :chain :a1 .
:a1 :chain :a2 .
:a2 a :T .
";

/// THE LEDGER.
///
/// Rows are in the order a reader wants them: the shape the search was hardened for, the
/// cardinality-only-equivalence control that isolates why, a stronger both-moved variant kept
/// beside it, the co-typed shape the work budget exists for, the schema-heavy shape the
/// achiever cache exists for, then the truncated path.
const LEDGER: &[Pin] = &[
    Pin {
        name: "equivalent-class-allvalues-cardinality",
        ontology: EQUIVALENT_CLASS_SHAPE,
        triples: 17,
        step_cap: 0,
        work_cap: 0,
        answer: "consistency true\n",
        completeness: "decided",
        steps: 11,
        work: 2724,
        peak_nodes: 4,
        disjunctions: 3,
        peak_depth: 3,
    },
    Pin {
        name: "cardinality-only-equivalence",
        ontology: CARDINALITY_EQUIVALENCE_SHAPE,
        triples: 17,
        step_cap: 0,
        work_cap: 0,
        answer: "consistency true\n",
        completeness: "decided",
        steps: 3,
        work: 243,
        peak_nodes: 2,
        disjunctions: 0,
        peak_depth: 0,
    },
    Pin {
        name: "subclass-only-both-restrictions",
        ontology: SUBCLASS_SHAPE,
        triples: 17,
        step_cap: 0,
        work_cap: 0,
        answer: "consistency true\n",
        completeness: "decided",
        steps: 3,
        work: 206,
        peak_nodes: 2,
        disjunctions: 0,
        peak_depth: 0,
    },
    Pin {
        name: "co-typed-equivalence-blocks",
        ontology: CO_TYPED_SHAPE,
        triples: 34,
        step_cap: 0,
        work_cap: 0,
        answer: "consistency true\n",
        completeness: "decided",
        steps: 71,
        work: 185_099,
        peak_nodes: 15,
        disjunctions: 28,
        peak_depth: 28,
    },
    Pin {
        name: "schema-heavy-domain-range-pairs",
        ontology: SCHEMA_HEAVY_SHAPE,
        triples: 51,
        step_cap: 0,
        work_cap: 0,
        answer: "consistency true\n",
        completeness: "decided",
        steps: 1,
        work: 1048,
        peak_nodes: 3,
        disjunctions: 0,
        peak_depth: 0,
    },
    Pin {
        name: "narrowed-budget-subclass-chain",
        ontology: NARROWED_BUDGET_SHAPE,
        triples: 3,
        step_cap: 1,
        work_cap: 0,
        answer: "consistency unknown\n",
        completeness: "budget-exhausted",
        steps: 1,
        work: 11,
        peak_nodes: 1,
        disjunctions: 0,
        peak_depth: 0,
    },
];

/// The ontology as canonical N-Quads. `pin.triples` is the separate, hand-counted figure a
/// caller checks the parse against — this function returns only the document text.
fn as_nquads(pin: &Pin) -> String {
    let dataset = parse_dataset(pin.ontology.as_bytes(), "text/turtle", None)
        .unwrap_or_else(|error| panic!("{}: the ontology parses: {error}", pin.name));
    let bytes = serialize_dataset(&*dataset, "application/n-quads", SerializeGraph::Dataset)
        .unwrap_or_else(|error| panic!("{}: the ontology serializes: {error}", pin.name));
    String::from_utf8(bytes).expect("N-Quads is UTF-8")
}

#[test]
fn every_ledgered_search_costs_exactly_what_it_is_pinned_to() {
    for pin in LEDGER {
        let document = as_nquads(pin);
        assert_eq!(
            document.lines().count(),
            pin.triples,
            "{}: the fixture is no longer the ontology this row pins:\n{document}",
            pin.name
        );

        let answer =
            purrdf_validate::regime::consistency_to_string(&document, pin.step_cap, pin.work_cap)
                .unwrap_or_else(|error| panic!("{}: the ontology reverse-maps: {error}", pin.name));
        let certificate = answer.certificate();
        assert_eq!(
            answer.answer(),
            pin.answer,
            "{}: the verdict moved, which is a STOP rather than a re-pin:\n{certificate}",
            pin.name
        );
        assert!(
            certificate.contains(&format!("\ncompleteness {}\n", pin.completeness)),
            "{}: the completeness moved, which is a STOP rather than a re-pin:\n{certificate}",
            pin.name
        );

        let measured = (
            measurement(certificate, "steps"),
            measurement(certificate, "work"),
            measurement(certificate, "peak-nodes"),
            measurement(certificate, "disjunctions"),
            measurement(certificate, "peak-depth"),
        );
        assert_eq!(
            measured,
            (
                pin.steps,
                pin.work,
                pin.peak_nodes,
                pin.disjunctions,
                pin.peak_depth
            ),
            "{}: (steps, work, peak-nodes, disjunctions, peak-depth) moved. Re-pin the row \
             DELIBERATELY, and only after reading which of the five moved and why:\n{certificate}",
            pin.name
        );
    }
}

/// The cardinality-only-equivalence control and the stronger both-moved variant read as
/// seventeen triples each, and [`Pin`] does not pin a per-row budget — the round cap is
/// DERIVED from the knowledge base's size, not stated. So this is where the two rows are told
/// apart instead: moving only the `∀`-restriction states an equivalence [`SUBCLASS_SHAPE`]
/// does not, and that is a different ontology with a different cap, not the same fixture
/// measured twice. The control's own figure — 133,856 — is pinned here directly, on the
/// derived cap rather than on a search figure a re-pin could move for an unrelated reason.
#[test]
fn the_cardinality_only_control_is_a_distinct_ontology_from_the_both_moved_variant() {
    let cardinality_only = LEDGER
        .iter()
        .find(|pin| pin.name == "cardinality-only-equivalence")
        .expect("the cardinality-only-equivalence control is ledgered");
    let both_moved = LEDGER
        .iter()
        .find(|pin| pin.name == "subclass-only-both-restrictions")
        .expect("the both-moved variant is ledgered");

    let document = as_nquads(cardinality_only);
    let answer = purrdf_validate::regime::consistency_to_string(
        &document,
        cardinality_only.step_cap,
        cardinality_only.work_cap,
    )
    .unwrap_or_else(|error| {
        panic!(
            "{}: the ontology reverse-maps: {error}",
            cardinality_only.name
        )
    });
    let budget = measurement(answer.certificate(), "budget");
    assert_eq!(
        budget,
        133_856,
        "the cardinality-only-equivalence control's own figure for its round cap must still \
         hold:\n{}",
        answer.certificate()
    );

    let both_moved_document = as_nquads(both_moved);
    let both_moved_answer = purrdf_validate::regime::consistency_to_string(
        &both_moved_document,
        both_moved.step_cap,
        both_moved.work_cap,
    )
    .unwrap_or_else(|error| panic!("{}: the ontology reverse-maps: {error}", both_moved.name));
    let both_moved_budget = measurement(both_moved_answer.certificate(), "budget");
    assert_ne!(
        budget,
        both_moved_budget,
        "the cardinality-only-equivalence control and the both-moved variant must be DISTINCT \
         ontologies, not the same fixture measured twice:\ncontrol:\n{}\nboth-moved:\n{}",
        answer.certificate(),
        both_moved_answer.certificate()
    );
}

#[test]
fn every_decided_ledgered_search_stays_far_inside_its_budget() {
    for pin in LEDGER {
        if pin.completeness != "decided" {
            continue;
        }
        let document = as_nquads(pin);
        let answer =
            purrdf_validate::regime::consistency_to_string(&document, pin.step_cap, pin.work_cap)
                .unwrap_or_else(|error| panic!("{}: the ontology reverse-maps: {error}", pin.name));
        let certificate = answer.certificate();
        let steps = measurement(certificate, "steps");
        let budget = measurement(certificate, "budget");
        assert!(
            steps * 10 < budget,
            "{}: the search spent {steps} of its own declared {budget}-round budget, which is \
             not `far inside` it. The exact pin above would have accepted this drift one \
             commit at a time; this guard is what does not:\n{certificate}",
            pin.name
        );
        let work = measurement(certificate, "work");
        let work_budget = measurement(certificate, "work-budget");
        assert!(
            work * 10 < work_budget,
            "{}: the search spent {work} of its own declared {work_budget}-unit work budget, \
             which is not `far inside` it. A row drifting towards THIS ceiling is the one the \
             round figures cannot show:\n{certificate}",
            pin.name
        );
    }
}

#[test]
fn every_ledgered_search_renders_identically_twice() {
    for pin in LEDGER {
        let document = as_nquads(pin);
        let first =
            purrdf_validate::regime::consistency_to_string(&document, pin.step_cap, pin.work_cap)
                .unwrap_or_else(|error| panic!("{}: the ontology reverse-maps: {error}", pin.name));
        let again =
            purrdf_validate::regime::consistency_to_string(&document, pin.step_cap, pin.work_cap)
                .unwrap_or_else(|error| panic!("{}: the ontology reverse-maps: {error}", pin.name));
        assert_eq!(
            first, again,
            "{}: two runs, one rendering — answer AND certificate, every counter included",
            pin.name
        );
    }
}
