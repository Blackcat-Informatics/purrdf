// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Description-Logic reasoning services, and the OWL 2 profile certifier.
//!
//! Two properties are asserted of EVERY service call these tests make, because they are the
//! two a certificate exists to guarantee and neither is observable from a single happy-path
//! assertion:
//!
//! * no certificate overclaims — a `Decided` verdict never sits beside a boundary list,
//!   which [`DlCertificate::completeness`] guarantees by deriving the verdict from the
//!   boundary list rather than storing it, so `honest` exercises that derivation directly;
//! * the answer is reproducible — the same dataset reasoned over twice gives equal answers
//!   and equal certificates, step counts included.
//!
//! The profile-certifier tests assert the ONE-DIRECTIONAL doctrine as well: a certification
//! is a proof of membership, a violation is only the cheap structural condition failing.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_entail::reasoner::{
    DlAxiom, DlCompleteness, OwlProfile, ProfileCertificate, Reasoner, Verdict, profile,
};
use purrdf_entail::{QNode, QTriple};

// --- vocabulary ------------------------------------------------------------------------

const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const SUB_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const SUB_PROPERTY: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const THING: &str = "http://www.w3.org/2002/07/owl#Thing";
const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
const RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
const ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const SOME_VALUES: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
const ALL_VALUES: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
const MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
const MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";
const INTERSECTION: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
const UNION: &str = "http://www.w3.org/2002/07/owl#unionOf";
const COMPLEMENT: &str = "http://www.w3.org/2002/07/owl#complementOf";
const ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";
const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
const TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
const PROPERTY_CHAIN: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
const DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
const FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
const XSD_NON_NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
const ON_DATATYPE: &str = "http://www.w3.org/2002/07/owl#onDatatype";
const WITH_RESTRICTIONS: &str = "http://www.w3.org/2002/07/owl#withRestrictions";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_MIN_INCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#minInclusive";
const XSD_MAX_INCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#maxInclusive";
const XSD_PATTERN: &str = "http://www.w3.org/2001/XMLSchema#pattern";

/// A fixture term. PurRDF mints no vocabulary, so every fixture IRI is `example.org`.
fn ex(local: &str) -> String {
    format!("http://example.org/{local}")
}

// --- fixture construction --------------------------------------------------------------

/// A node of a fixture triple.
#[derive(Clone)]
enum N {
    /// An `example.org` IRI, by local name.
    E(&'static str),
    /// A vocabulary IRI, in full.
    V(&'static str),
    /// A blank node, by label.
    B(&'static str),
    /// A typed literal.
    L(&'static str, &'static str),
}

/// Build a default-graph dataset from fixture triples.
fn ds(triples: &[(N, N, N)]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let mut node = |n: &N| match n {
        N::E(local) => b.intern_iri(&ex(local)),
        N::V(iri) => b.intern_iri(iri),
        N::B(label) => b.intern_blank(label, BlankScope::DEFAULT),
        N::L(lexical, datatype) => b.intern_literal(purrdf_core::RdfLiteral {
            lexical_form: (*lexical).to_owned(),
            datatype: Some((*datatype).to_owned()),
            language: None,
            direction: None,
        }),
    };
    let resolved: Vec<_> = triples
        .iter()
        .map(|(s, p, o)| (node(s), node(p), node(o)))
        .collect();
    for (s, p, o) in resolved {
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("the fixture freezes")
}

/// `A ⊑ B` as a fixture triple.
fn sub(a: N, b: N) -> (N, N, N) {
    (a, N::V(SUB_CLASS), b)
}

/// `a : C` as a fixture triple.
fn typed(a: N, c: N) -> (N, N, N) {
    (a, N::V(TYPE), c)
}

/// The `example.org` IRI term a fixture local name denotes.
fn iri(local: &str) -> TermValue {
    TermValue::iri(ex(local))
}

/// The pair `(subject IRI, object IRI)` a subsumption or type answer is compared against.
fn pairs(rows: &[(TermValue, TermValue)]) -> Vec<(String, String)> {
    rows.iter().map(|(a, b)| (show(a), show(b))).collect()
}

/// A term's IRI (or blank label) as a comparable string.
fn show(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => iri.clone(),
        TermValue::Blank { label, .. } => format!("_:{label}"),
        other => format!("{other:?}"),
    }
}

/// `(local, local)` shorthand for an expected `example.org` pair.
fn expect(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter().map(|(a, b)| (ex(a), ex(b))).collect()
}

// --- shared fixtures --------------------------------------------------------------------

/// `Kitten ⊑ Cat ⊑ Animal`, with `tom : Kitten`.
fn kittens() -> Arc<RdfDataset> {
    ds(&[
        typed(N::E("Animal"), N::V(OWL_CLASS)),
        typed(N::E("Cat"), N::V(OWL_CLASS)),
        typed(N::E("Kitten"), N::V(OWL_CLASS)),
        sub(N::E("Cat"), N::E("Animal")),
        sub(N::E("Kitten"), N::E("Cat")),
        typed(N::E("tom"), N::E("Kitten")),
    ])
}

/// Assert the certificate of one service call is honest, and return it for further checks.
///
/// "Honest" is exercised here rather than gated: `DlCertificate` has no `overclaims`
/// predicate to call, because `completeness` derives its verdict from `boundaries` on every
/// call and there is no second, independently-stored verdict for the two to disagree over.
/// This match is that same derivation, checked against the boundary list this particular
/// certificate actually carries.
fn honest<T>(answer: &purrdf_entail::Certified<T>) -> &purrdf_entail::DlCertificate {
    let certificate = answer.certificate();
    match certificate.completeness() {
        DlCompleteness::Decided => assert!(
            certificate.boundaries().is_empty(),
            "`Decided` beside {} boundaries would be an overclaim",
            certificate.boundaries().len()
        ),
        DlCompleteness::DecidedWithinBoundaries => assert!(
            !certificate.boundaries().is_empty(),
            "`DecidedWithinBoundaries` beside no boundaries should have derived `Decided`"
        ),
        DlCompleteness::BudgetExhausted => {}
    }
    assert!(
        certificate.steps()
            <= certificate
                .budget()
                .saturating_mul(certificate.decisions().max(1)),
        "the step tally exceeds what {} decisions of {} steps could consume",
        certificate.decisions(),
        certificate.budget()
    );
    // The three shape counters, held to the invariants that make them readable — and held
    // here, in the helper EVERY service test runs its answer through, because what they are
    // aggregated by differs per counter (work sums, sizes peak) and an aggregation applied to
    // the wrong one is invisible on any single service.
    // Deliberately NOT "a search that spent a round built a node". A question about the
    // terminology alone — which is what `classify` asks once the classifying saturation has
    // derived what it can — has no individual to build a root for, so its completion graph is
    // legitimately EMPTY and saturates in one round over nothing. Asserting otherwise would
    // pin an accident of which ontologies these tests happen to use.
    assert!(
        certificate.peak_depth() <= certificate.disjunctions(),
        "a branch stack {} deep over {} case splits",
        certificate.peak_depth(),
        certificate.disjunctions()
    );
    assert!(
        certificate.disjunctions() <= certificate.steps(),
        "{} case splits over {} rounds: a split costs at least the round that derived it",
        certificate.disjunctions(),
        certificate.steps()
    );
    certificate
}

// --- the reasoner façade ----------------------------------------------------------------

#[test]
fn a_consistent_ontology_is_reported_consistent_and_decided() {
    let dataset = kittens();
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::True);
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::Decided);
    assert!(certificate.boundaries().is_empty());
    assert_eq!(certificate.decisions(), 1, "consistency is ONE decision");
    assert!(certificate.steps() > 0, "a decision consumes steps");
}

#[test]
fn a_contradictory_ontology_is_reported_inconsistent_rather_than_thrown() {
    // `A ⊑ B`, `A ⊑ ¬B`, `a : A` — no model.
    let dataset = ds(&[
        typed(N::E("A"), N::V(OWL_CLASS)),
        typed(N::E("B"), N::V(OWL_CLASS)),
        sub(N::E("A"), N::E("B")),
        sub(N::E("A"), N::B("notB")),
        (N::B("notB"), N::V(COMPLEMENT), N::E("B")),
        typed(N::E("a"), N::E("A")),
    ]);
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");
    assert_eq!(*reasoner.consistency().answer(), Verdict::False);
    // Every OTHER service refuses, because an inconsistent ontology entails everything.
    assert!(matches!(
        reasoner.classify(),
        Err(purrdf_entail::EntailError::Unsatisfiable)
    ));
    assert!(matches!(
        reasoner.realize(),
        Err(purrdf_entail::EntailError::Unsatisfiable)
    ));
    assert!(matches!(
        reasoner.instances(&iri("A")),
        Err(purrdf_entail::EntailError::Unsatisfiable)
    ));
}

/// `ex:C ⊑ ∃ex:p.(xsd:integer[<facets>])` with `ex:a : ex:C`, as fixture triples.
///
/// `facets` is a list of `(facet IRI, lexical form)` over `xsd:integer` unless the facet is
/// `xsd:pattern`, whose value is an `xsd:string`.
fn facet_ontology(facets: &[(&'static str, &'static str)]) -> Arc<RdfDataset> {
    let mut triples = vec![
        typed(N::E("C"), N::V(OWL_CLASS)),
        typed(N::E("p"), N::V(DATATYPE_PROPERTY)),
        sub(N::E("C"), N::B("restriction")),
        typed(N::B("restriction"), N::V(RESTRICTION)),
        (N::B("restriction"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("restriction"), N::V(SOME_VALUES), N::B("range")),
        typed(N::B("range"), N::V(RDFS_DATATYPE)),
        (N::B("range"), N::V(ON_DATATYPE), N::V(XSD_INTEGER)),
        (N::B("range"), N::V(WITH_RESTRICTIONS), N::B("cell0")),
        typed(N::E("a"), N::E("C")),
    ];
    // The facet cells, written as an RDF collection over `cell0`, `cell1`, …
    let cells: [&'static str; 2] = ["cell0", "cell1"];
    let facet_nodes: [&'static str; 2] = ["facet0", "facet1"];
    for (index, &(facet, lexical)) in facets.iter().enumerate() {
        let datatype = if facet == XSD_PATTERN {
            XSD_STRING
        } else {
            XSD_INTEGER
        };
        triples.push((
            N::B(facet_nodes[index]),
            N::V(facet),
            N::L(lexical, datatype),
        ));
        triples.push((N::B(cells[index]), N::V(FIRST), N::B(facet_nodes[index])));
        let tail = if index + 1 == facets.len() {
            N::V(NIL)
        } else {
            N::B(cells[index + 1])
        };
        triples.push((N::B(cells[index]), N::V(REST), tail));
    }
    ds(&triples)
}

/// AN EMPTY DATATYPE RANGE IS AN INCONSISTENCY, and the certificate says `Decided`.
///
/// `xsd:integer` bounded below by 5 and above by 3 holds no value, so a class whose every
/// member must have a `p`-value in it is unsatisfiable and an individual asserted into it
/// makes the whole ontology so. Nothing here is bounded: the facets were evaluated over the
/// integers, and the certificate is entitled to claim a decided answer.
#[test]
fn an_empty_datatype_range_is_reported_inconsistent_and_decided() {
    let dataset = facet_ontology(&[(XSD_MIN_INCLUSIVE, "5"), (XSD_MAX_INCLUSIVE, "3")]);
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::False);
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::Decided);
    assert!(certificate.boundaries().is_empty());
}

/// …and the class ITSELF is unsatisfiable, which is the narrower question and the one that
/// holds without any individual being asserted.
#[test]
fn a_class_forced_into_an_empty_datatype_range_is_unsatisfiable() {
    // The same terminology, with no instance: the ontology is consistent (the class is simply
    // empty) and `ex:C` is the unsatisfiable class.
    let dataset = ds(&[
        typed(N::E("C"), N::V(OWL_CLASS)),
        typed(N::E("p"), N::V(DATATYPE_PROPERTY)),
        sub(N::E("C"), N::B("restriction")),
        typed(N::B("restriction"), N::V(RESTRICTION)),
        (N::B("restriction"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("restriction"), N::V(SOME_VALUES), N::B("range")),
        typed(N::B("range"), N::V(RDFS_DATATYPE)),
        (N::B("range"), N::V(ON_DATATYPE), N::V(XSD_INTEGER)),
        (N::B("range"), N::V(WITH_RESTRICTIONS), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::B("facet0")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
        (
            N::B("facet0"),
            N::V(XSD_MAX_INCLUSIVE),
            N::L("3", XSD_INTEGER),
        ),
        // `ex:C ⊑ ∃p.[≤3]` alone is satisfiable; adding `⊑ ∃p.[≥5]` under a FUNCTIONAL `p`
        // is what empties it, because the one value must then be in both.
        typed(N::E("p"), N::V(FUNCTIONAL_PROPERTY)),
        sub(N::E("C"), N::B("restriction2")),
        typed(N::B("restriction2"), N::V(RESTRICTION)),
        (N::B("restriction2"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("restriction2"), N::V(ALL_VALUES), N::B("range2")),
        typed(N::B("range2"), N::V(RDFS_DATATYPE)),
        (N::B("range2"), N::V(ON_DATATYPE), N::V(XSD_INTEGER)),
        (N::B("range2"), N::V(WITH_RESTRICTIONS), N::B("cell1")),
        (N::B("cell1"), N::V(FIRST), N::B("facet1")),
        (N::B("cell1"), N::V(REST), N::V(NIL)),
        (
            N::B("facet1"),
            N::V(XSD_MIN_INCLUSIVE),
            N::L("5", XSD_INTEGER),
        ),
    ]);
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");
    assert_eq!(
        *reasoner.consistency().answer(),
        Verdict::True,
        "an empty class is not an inconsistent ontology"
    );
    let answer = reasoner
        .class_satisfiability(&iri("C"))
        .expect("the class is decided");
    assert_eq!(
        *answer.answer(),
        Verdict::False,
        "every member of ex:C needs one p-value that is both ≥5 and ≤3"
    );
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::Decided);
}

/// A SATISFIABLE facet stays satisfiable. Over-eager emptiness would be unsoundness in the
/// other direction, and it is the more dangerous one: an invented inconsistency entails every
/// answer a caller could ask for.
#[test]
fn a_satisfiable_datatype_range_is_reported_consistent_and_decided() {
    let dataset = facet_ontology(&[(XSD_MIN_INCLUSIVE, "3"), (XSD_MAX_INCLUSIVE, "5")]);
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::True);
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::Decided);
    assert!(certificate.boundaries().is_empty());
}

/// THE RESIDUE IS NAMED. An `xsd:pattern` facet is a regular-language question this layer
/// does not decide, so the range is undecided — and an undecided range may not close a
/// branch. The verdict stays `True` and the certificate stops claiming a flatly decided run.
#[test]
fn an_undecidable_facet_is_reported_within_boundaries_rather_than_guessed() {
    let dataset = facet_ontology(&[(XSD_PATTERN, "[0-9]+")]);
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.consistency();
    assert_eq!(
        *answer.answer(),
        Verdict::True,
        "an undecided range must never invent an inconsistency"
    );
    let certificate = honest(&answer);
    assert_eq!(
        certificate.completeness(),
        DlCompleteness::DecidedWithinBoundaries
    );
    let constructs: Vec<String> = certificate
        .boundaries()
        .iter()
        .map(|boundary| boundary.construct().to_string())
        .collect();
    assert_eq!(constructs, vec!["data-range".to_owned()]);
    assert!(
        certificate.boundaries()[0].reason().contains("xsd:pattern"),
        "the reason must name the facet that was not decided"
    );
}

#[test]
fn classification_computes_the_transitive_relation_and_its_reduction() {
    let dataset = kittens();
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.classify().expect("consistent");
    honest(&answer);
    let hierarchy = answer.answer();

    // The full relation is transitively closed: `Kitten ⊑ Animal` is entailed and listed
    // even though no triple states it.
    let subsumptions = pairs(hierarchy.subsumptions());
    for row in expect(&[("Kitten", "Cat"), ("Cat", "Animal"), ("Kitten", "Animal")]) {
        assert!(subsumptions.contains(&row), "missing {row:?}");
    }
    // …and `owl:Thing`/`owl:Nothing` participate as `⊤`/`⊥`.
    assert!(subsumptions.contains(&(ex("Cat"), THING.to_owned())));
    assert!(subsumptions.contains(&(NOTHING.to_owned(), ex("Cat"))));

    // The reduction drops the transitive edge.
    let direct = pairs(hierarchy.direct_subsumptions());
    assert!(direct.contains(&(ex("Kitten"), ex("Cat"))));
    assert!(direct.contains(&(ex("Cat"), ex("Animal"))));
    assert!(
        !direct.contains(&(ex("Kitten"), ex("Animal"))),
        "a transitive edge is not a DIRECT subsumption: {direct:?}"
    );
    assert!(
        hierarchy.equivalences().is_empty(),
        "nothing in this ontology is equivalent: {:?}",
        hierarchy.equivalences()
    );
    // Only `owl:Nothing` is empty.
    let empty: Vec<String> = hierarchy.unsatisfiable().iter().map(show).collect();
    assert_eq!(empty, vec![NOTHING.to_owned()]);
}

#[test]
fn classification_finds_an_equivalence_and_an_unsatisfiable_class() {
    let dataset = ds(&[
        typed(N::E("A"), N::V(OWL_CLASS)),
        typed(N::E("B"), N::V(OWL_CLASS)),
        typed(N::E("Empty"), N::V(OWL_CLASS)),
        (N::E("A"), N::V(EQUIVALENT_CLASS), N::E("B")),
        // `Empty ⊑ A` and `Empty ⊑ ¬A` force `Empty ⊑ ⊥`.
        sub(N::E("Empty"), N::E("A")),
        sub(N::E("Empty"), N::B("notA")),
        (N::B("notA"), N::V(COMPLEMENT), N::E("A")),
    ]);
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.classify().expect("consistent");
    honest(&answer);
    let hierarchy = answer.answer();

    assert!(
        pairs(hierarchy.equivalences()).contains(&(ex("A"), ex("B"))),
        "A ≡ B is entailed: {:?}",
        hierarchy.equivalences()
    );
    let empty: Vec<String> = hierarchy.unsatisfiable().iter().map(show).collect();
    assert!(
        empty.contains(&ex("Empty")),
        "Empty is unsatisfiable: {empty:?}"
    );
    assert!(empty.contains(&NOTHING.to_owned()));
}

#[test]
fn realization_reports_every_entailed_type_and_the_most_specific_ones() {
    let dataset = kittens();
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.realize().expect("consistent");
    honest(&answer);
    let realization = answer.answer();

    let types = pairs(realization.types());
    for row in expect(&[("tom", "Kitten"), ("tom", "Cat"), ("tom", "Animal")]) {
        assert!(types.contains(&row), "missing {row:?}");
    }
    assert!(
        types.contains(&(ex("tom"), THING.to_owned())),
        "everything is an owl:Thing, and an entailed answer is not omitted for being obvious"
    );

    let direct = pairs(realization.direct_types());
    assert_eq!(
        direct,
        expect(&[("tom", "Kitten")]),
        "the only MOST SPECIFIC type of tom is Kitten"
    );
}

#[test]
fn instance_retrieval_reaches_a_class_nothing_asserts() {
    let dataset = kittens();
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.instances(&iri("Animal")).expect("consistent");
    honest(&answer);
    let names: Vec<String> = answer.answer().iter().map(show).collect();
    assert_eq!(names, vec![ex("tom")]);

    // A class the ontology never mentions is an unconstrained atomic name, which is a real
    // answer rather than a rejection.
    let unknown = reasoner.instances(&iri("Vegetable")).expect("consistent");
    assert!(unknown.answer().is_empty());
    honest(&unknown);
}

#[test]
fn class_satisfiability_decides_thing_nothing_and_a_forced_empty_class() {
    let dataset = ds(&[
        typed(N::E("A"), N::V(OWL_CLASS)),
        sub(N::E("Empty"), N::E("A")),
        sub(N::E("Empty"), N::B("notA")),
        (N::B("notA"), N::V(COMPLEMENT), N::E("A")),
    ]);
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");
    for (class, expected) in [
        (TermValue::iri(THING), Verdict::True),
        (TermValue::iri(NOTHING), Verdict::False),
        (iri("A"), Verdict::True),
        (iri("Empty"), Verdict::False),
        // Nothing constrains it, so it is satisfiable.
        (iri("Unheard"), Verdict::True),
    ] {
        let answer = reasoner
            .class_satisfiability(&class)
            .expect("the ontology is consistent");
        honest(&answer);
        assert_eq!(*answer.answer(), expected, "satisfiability of {class:?}");
    }
}

#[test]
fn every_axiom_form_is_decided_by_refutation() {
    // A fixture that exercises all eight: a hierarchy, an equivalence, a disjointness, an
    // assertion, a role, an equality and an inequality.
    let dataset = ds(&[
        typed(N::E("Cat"), N::V(OWL_CLASS)),
        typed(N::E("Animal"), N::V(OWL_CLASS)),
        typed(N::E("Dog"), N::V(OWL_CLASS)),
        sub(N::E("Cat"), N::E("Animal")),
        (N::E("Cat"), N::V(DISJOINT_WITH), N::E("Dog")),
        typed(N::E("p"), N::V(OBJECT_PROPERTY)),
        typed(N::E("q"), N::V(OBJECT_PROPERTY)),
        (N::E("p"), N::V(SUB_PROPERTY), N::E("q")),
        typed(N::E("tom"), N::E("Cat")),
        (N::E("tom"), N::E("p"), N::E("jerry")),
        (N::E("tom"), N::V(SAME_AS), N::E("thomas")),
        (N::E("tom"), N::V(DIFFERENT_FROM), N::E("jerry")),
    ]);
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");

    let cases: Vec<(DlAxiom, Verdict, &str)> = vec![
        (
            DlAxiom::SubClassOf {
                sub: iri("Cat"),
                sup: iri("Animal"),
            },
            Verdict::True,
            "asserted subsumption",
        ),
        (
            DlAxiom::SubClassOf {
                sub: iri("Animal"),
                sup: iri("Cat"),
            },
            Verdict::False,
            "the converse is not entailed",
        ),
        (
            DlAxiom::EquivalentClasses {
                left: iri("Cat"),
                right: iri("Cat"),
            },
            Verdict::True,
            "reflexive equivalence",
        ),
        (
            DlAxiom::EquivalentClasses {
                left: iri("Cat"),
                right: iri("Animal"),
            },
            Verdict::False,
            "one-directional subsumption is not equivalence",
        ),
        (
            DlAxiom::DisjointClasses {
                left: iri("Cat"),
                right: iri("Dog"),
            },
            Verdict::True,
            "asserted disjointness",
        ),
        (
            DlAxiom::DisjointClasses {
                left: iri("Cat"),
                right: iri("Animal"),
            },
            Verdict::False,
            "a subsumed class is not disjoint from its subsumer",
        ),
        (
            DlAxiom::ClassAssertion {
                individual: iri("tom"),
                class: iri("Animal"),
            },
            Verdict::True,
            "the class assertion is DERIVED, not asserted",
        ),
        (
            DlAxiom::ClassAssertion {
                individual: iri("jerry"),
                class: iri("Cat"),
            },
            Verdict::False,
            "nothing types jerry",
        ),
        (
            DlAxiom::ObjectPropertyAssertion {
                subject: iri("tom"),
                property: iri("q"),
                object: iri("jerry"),
            },
            Verdict::True,
            "q(tom, jerry) follows from p ⊑ q",
        ),
        (
            DlAxiom::ObjectPropertyAssertion {
                subject: iri("jerry"),
                property: iri("p"),
                object: iri("tom"),
            },
            Verdict::False,
            "p is not symmetric",
        ),
        (
            DlAxiom::SameIndividual {
                left: iri("tom"),
                right: iri("thomas"),
            },
            Verdict::True,
            "asserted equality",
        ),
        (
            DlAxiom::SameIndividual {
                left: iri("tom"),
                right: iri("jerry"),
            },
            Verdict::False,
            "no unique name assumption, but nothing merges these two either",
        ),
        (
            DlAxiom::DifferentIndividuals {
                left: iri("tom"),
                right: iri("jerry"),
            },
            Verdict::True,
            "asserted inequality",
        ),
        (
            DlAxiom::DifferentIndividuals {
                left: iri("tom"),
                right: iri("thomas"),
            },
            Verdict::False,
            "these two are the SAME individual",
        ),
        (
            DlAxiom::SubObjectPropertyOf {
                sub: iri("p"),
                sup: iri("q"),
            },
            Verdict::True,
            "asserted role inclusion, decided over FRESH blank-node symbols",
        ),
        (
            DlAxiom::SubObjectPropertyOf {
                sub: iri("q"),
                sup: iri("p"),
            },
            Verdict::False,
            "the converse role inclusion is not entailed",
        ),
    ];
    for (axiom, expected, why) in cases {
        let answer = reasoner.entails(&axiom).expect("consistent");
        honest(&answer);
        assert_eq!(*answer.answer(), expected, "{why}: {axiom:?}");
    }
}

#[test]
fn a_refutation_symbol_avoids_a_colliding_blank_label_in_the_data() {
    // The data carries blank nodes labelled exactly as the refutation generator's first two
    // symbols would be, and CONSTRAINS them: `_:purrdfDlRefutation0 : ¬∃p.⊤` says the node
    // has no `p`-successor at all. A role-inclusion refutation asserts `x p y` over its
    // fresh pair, so a generator that reused those labels would clash on the assumption
    // itself and report EVERY role inclusion entailed.
    //
    // Only `q ⊑ p` is asserted here, so `p ⊑ q` is NOT entailed and the honest answer is
    // `False`. A colliding symbol would answer `True` — unsound, not merely incomplete.
    let dataset = ds(&[
        typed(N::E("p"), N::V(OBJECT_PROPERTY)),
        typed(N::E("q"), N::V(OBJECT_PROPERTY)),
        (N::E("q"), N::V(SUB_PROPERTY), N::E("p")),
        typed(N::B("purrdfDlRefutation0"), N::B("noP")),
        typed(N::B("purrdfDlRefutation1"), N::B("noP")),
        (N::B("noP"), N::V(COMPLEMENT), N::B("someP")),
        typed(N::B("someP"), N::V(RESTRICTION)),
        (N::B("someP"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("someP"), N::V(SOME_VALUES), N::V(THING)),
    ]);
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner
        .entails(&DlAxiom::SubObjectPropertyOf {
            sub: iri("p"),
            sup: iri("q"),
        })
        .expect("consistent");
    honest(&answer);
    assert_eq!(
        *answer.answer(),
        Verdict::False,
        "the fresh symbols must not alias the data's blank nodes"
    );

    // The converse IS entailed, over the same colliding data — so the fixture is testing
    // freshness rather than a reasoner that answers `False` to everything.
    let converse = reasoner
        .entails(&DlAxiom::SubObjectPropertyOf {
            sub: iri("q"),
            sup: iri("p"),
        })
        .expect("consistent");
    honest(&converse);
    assert_eq!(*converse.answer(), Verdict::True);
}

#[test]
fn a_narrowed_budget_reports_unknown_rather_than_a_false_negative() {
    let dataset = kittens();
    let reasoner = Reasoner::new(&dataset)
        .expect("reverse-map")
        // One step decides nothing at all, which is the point: the exhausted path must be
        // reachable and must report itself.
        .with_step_cap(1);
    assert_eq!(reasoner.step_cap(), 1);

    let answer = reasoner.consistency();
    assert_eq!(
        *answer.answer(),
        Verdict::Unknown,
        "an exhausted search is UNKNOWN, never `false`"
    );
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::BudgetExhausted);
    assert!(!certificate.completeness().is_decided());
    // What a truncated search COSTS is pinned exactly in the workspace's one step ledger
    // (`purrdf-validate`'s `dl_step_ledger`), which decides this same shape under this same
    // cap of one and holds its rounds, nodes, case splits and depth in a single table. An
    // exact literal here would be a second copy of one of those figures, and two copies of a
    // number are how a re-pin comes to move one of them. What belongs in this test is the
    // SHAPE of the truncated answer: `unknown`, `budget-exhausted`, and no more spent than
    // the cap allowed.
    assert!(
        certificate.steps() <= certificate.budget(),
        "a truncated search cannot have spent more than the cap that truncated it"
    );

    // Every aggregate service degrades the same way: an empty answer under an exhausted
    // certificate, rather than a confidently wrong one.
    let hierarchy = reasoner.classify().expect("not refused, just undecided");
    assert!(hierarchy.answer().subsumptions().is_empty());
    assert_eq!(
        hierarchy.certificate().completeness(),
        DlCompleteness::BudgetExhausted
    );
    honest(&hierarchy);
}

/// The SECOND budget degrades the same way, and the certificate says it was the one.
///
/// Not a duplicate of the test above. A derivation round is a PASS over the completion graph
/// rather than a unit of cost, so the two caps bound different quantities: an ontology can
/// make each round enormously more expensive without making the search take more rounds, and
/// only this one sees that. What is checked is that work exhaustion travels the SAME honest
/// channel — `Verdict::Unknown`, `DlCompleteness::BudgetExhausted` — and that the two work
/// figures identify it, which the round figures in the same certificate cannot.
#[test]
fn a_narrowed_work_budget_reports_unknown_through_the_same_channel() {
    let dataset = kittens();
    let reasoner = Reasoner::new(&dataset)
        .expect("reverse-map")
        .with_work_cap(1);
    assert_eq!(reasoner.work_cap(), 1);
    assert!(
        reasoner.step_cap() > 1,
        "the ROUND cap is untouched, so what truncates this run is unambiguous"
    );

    let answer = reasoner.consistency();
    assert_eq!(
        *answer.answer(),
        Verdict::Unknown,
        "a work-exhausted search is UNKNOWN, never `false`"
    );
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::BudgetExhausted);
    assert_eq!(
        certificate.work(),
        certificate.work_budget(),
        "the run stopped at its work cap, and the two figures say so"
    );
    assert!(
        certificate.steps() * 10 < certificate.budget(),
        "the ROUND figures show what they could not see: {certificate:?}"
    );
}

/// A NARROW work cap must cut a LARGE bulk scan off near its own cap rather than letting one
/// oversized enumeration run to completion first — the stability gap FB-1 closed.
///
/// `:a` gets five thousand `:p`-neighbours and a `∀p.C` restriction that has to read every one
/// of them through the completion graph's role-neighbour scan. Before the fix, a scan already
/// in flight when the meter's bulk charge exhausted it kept walking its own bulk regardless —
/// the edge loop behind `Graph::neighbors`, the role-hierarchy closure behind it, and the
/// per-node clause scan `Hyper::round` drives it all from — so a cap far smaller than the
/// scan still paid for (most of) the scan before reporting itself exhausted. What is asserted
/// here is the OUTWARD-FACING half of that fix: the certificate says exhausted honestly
/// (`Unknown`, `BudgetExhausted`, `work == work-budget`), and it says the SAME thing twice —
/// determinism is not a property that survives an early exit by accident, since a search that
/// stopped at a different point on its second run would report a different round/work/shape
/// tuple. The crate-internal mechanism itself — that the scan's own inner loops poll the meter
/// rather than run unconditionally to the end — is pinned directly against `Graph`/`Hyper` in
/// `owl_dl::graph::tests` and `owl_dl::hyper::tests`, where the truncated candidate/edge/role
/// counts are countable; nothing crosses the `Reasoner` boundary that could make that latency
/// observable without timing, which this crate does not assert.
#[test]
fn a_narrow_work_cap_exhausts_promptly_over_a_large_role_scan_and_is_deterministic() {
    const NEIGHBOURS: usize = 5_000;
    let mut triples: Vec<(N, N, N)> = vec![
        typed(N::E("p"), N::V(OBJECT_PROPERTY)),
        typed(N::E("C"), N::V(OWL_CLASS)),
        typed(N::E("A"), N::V(OWL_CLASS)),
        sub(N::E("A"), N::B("allP")),
        typed(N::B("allP"), N::V(RESTRICTION)),
        (N::B("allP"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("allP"), N::V(ALL_VALUES), N::E("C")),
        typed(N::E("a"), N::E("A")),
    ];
    let neighbours: Vec<String> = (0..NEIGHBOURS).map(|i| format!("n{i}")).collect();
    for label in &neighbours {
        // `N::E` needs a `&'static str`; leaking a handful of kilobytes for the run of one
        // test is the ordinary price of that in a fixture built at runtime.
        let leaked: &'static str = Box::leak(label.clone().into_boxed_str());
        triples.push((N::E("a"), N::E("p"), N::E(leaked)));
    }
    let dataset = ds(&triples);

    let reasoner = Reasoner::new(&dataset)
        .expect("reverse-map")
        .with_work_cap(64);
    assert_eq!(reasoner.work_cap(), 64);

    let answer = reasoner.consistency();
    assert_eq!(
        *answer.answer(),
        Verdict::Unknown,
        "a work-exhausted search over a large scan is UNKNOWN, never a guessed verdict"
    );
    let certificate = honest(&answer);
    assert_eq!(certificate.completeness(), DlCompleteness::BudgetExhausted);
    assert_eq!(
        certificate.work(),
        certificate.work_budget(),
        "the meter clamps exactly at its cap, whatever the {NEIGHBOURS}-edge scan it cut \
         short still had left to walk: {certificate:?}"
    );

    // Two runs, one exhausted certificate: nothing behind the scans this cap cut off reads a
    // clock or a hash iteration order, so an early exit lands at the same candidate every time.
    let again = reasoner.consistency();
    assert_eq!(*again.answer(), Verdict::Unknown);
    let again_certificate = honest(&again);
    assert_eq!(
        certificate, again_certificate,
        "two runs of the same narrow-cap search must agree on every field, truncation included"
    );
}

#[test]
fn the_step_cap_can_be_narrowed_and_never_widened() {
    let dataset = kittens();
    let ceiling = Reasoner::new(&dataset).expect("reverse-map").step_cap();
    assert!(ceiling > 1, "the derived ceiling is generous: {ceiling}");
    let widened = Reasoner::new(&dataset)
        .expect("reverse-map")
        .with_step_cap(u64::MAX);
    assert_eq!(
        widened.step_cap(),
        ceiling,
        "a request above the ceiling is CLAMPED, never honoured"
    );
}

/// The work cap clamps exactly as the round cap does: narrow only.
///
/// Ceilings here are measured and reported, never tuned upward until a hard instance fits, and
/// this is the assertion that keeps the second one honest too.
#[test]
fn the_work_cap_can_be_narrowed_and_never_widened() {
    let dataset = kittens();
    let ceiling = Reasoner::new(&dataset).expect("reverse-map").work_cap();
    assert!(ceiling > 1, "the derived ceiling is generous: {ceiling}");
    let widened = Reasoner::new(&dataset)
        .expect("reverse-map")
        .with_work_cap(u64::MAX);
    assert_eq!(
        widened.work_cap(),
        ceiling,
        "a request above the ceiling is CLAMPED, never honoured"
    );
}

#[test]
fn an_ontology_carrying_a_bounded_construct_is_never_reported_decided() {
    // A property chain is read as a boundary rather than as a role assertion.
    let dataset = ds(&[
        typed(N::E("q"), N::V(OBJECT_PROPERTY)),
        (N::E("chained"), N::V(PROPERTY_CHAIN), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("q")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
        typed(N::E("A"), N::V(OWL_CLASS)),
    ]);
    let reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner.classify().expect("consistent");
    let certificate = honest(&answer);
    assert_eq!(
        certificate.completeness(),
        DlCompleteness::DecidedWithinBoundaries,
        "a run that met a boundary has not decided the whole ontology"
    );
    assert!(certificate.completeness().is_decided());
    let names: Vec<&str> = certificate
        .boundaries()
        .iter()
        .map(|boundary| boundary.construct().as_str())
        .collect();
    assert_eq!(names, vec!["property-chain"]);
}

#[test]
fn a_nominal_class_is_subsumed_through_an_assertion() {
    // `Only ≡ {alice}` and `alice : Female` entail `Only ⊑ Female` — a subsumption that
    // exists only because of an ABOX assertion, which a TBox-only classifier cannot see.
    let dataset = ds(&[
        typed(N::E("Female"), N::V(OWL_CLASS)),
        typed(N::E("Only"), N::V(OWL_CLASS)),
        (N::E("Only"), N::V(EQUIVALENT_CLASS), N::B("justAlice")),
        (N::B("justAlice"), N::V(ONE_OF), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("alice")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
        typed(N::E("alice"), N::E("Female")),
    ]);
    let mut reasoner = Reasoner::new(&dataset).expect("reverse-map");
    let answer = reasoner
        .entails(&DlAxiom::SubClassOf {
            sub: iri("Only"),
            sup: iri("Female"),
        })
        .expect("consistent");
    honest(&answer);
    assert_eq!(
        *answer.answer(),
        Verdict::True,
        "subsumption is decided against the WHOLE knowledge base, ABox included"
    );
}

#[test]
fn every_service_is_reproducible_answer_and_certificate_alike() {
    let dataset = kittens();
    let one = Reasoner::new(&dataset).expect("reverse-map");
    let two = Reasoner::new(&dataset).expect("reverse-map");
    assert_eq!(one.consistency(), two.consistency());
    assert_eq!(
        one.classify().expect("consistent"),
        two.classify().expect("consistent")
    );
    assert_eq!(
        one.realize().expect("consistent"),
        two.realize().expect("consistent")
    );
    let mut three = Reasoner::new(&dataset).expect("reverse-map");
    let mut four = Reasoner::new(&dataset).expect("reverse-map");
    assert_eq!(
        three.instances(&iri("Animal")).expect("consistent"),
        four.instances(&iri("Animal")).expect("consistent")
    );
    let axiom = DlAxiom::SubClassOf {
        sub: iri("Kitten"),
        sup: iri("Animal"),
    };
    assert_eq!(
        three.entails(&axiom).expect("consistent"),
        four.entails(&axiom).expect("consistent")
    );
    assert_eq!(one.signature(), two.signature());
    assert_eq!(one.named_individuals(), two.named_individuals());
}

// --- the query-directed augmentation ------------------------------------------------------

/// Whether the dataset's default graph holds the IRI triple `(s, p, o)`.
fn holds(dataset: &RdfDataset, s: &str, p: &str, o: &str) -> bool {
    dataset.quads().any(|quad| {
        quad.g.is_none()
            && dataset.term_value(quad.s) == TermValue::iri(s)
            && dataset.term_value(quad.p) == TermValue::iri(p)
            && dataset.term_value(quad.o) == TermValue::iri(o)
    })
}

/// A BGP triple over an `example.org` predicate and two variables.
fn pattern(subject: QNode, predicate: &str, object: QNode) -> QTriple {
    QTriple {
        s: subject,
        p: QNode::Term(TermValue::iri(predicate)),
        o: object,
    }
}

/// A query variable node.
fn var(name: &str) -> QNode {
    QNode::Var(name.to_owned())
}

#[test]
fn the_augmentation_states_a_role_assertion_the_property_hierarchy_entails() {
    // `p ⊑ q` and `a p b` entail `a q b`, which NO triple states.
    let dataset = ds(&[
        typed(N::E("p"), N::V(OBJECT_PROPERTY)),
        typed(N::E("q"), N::V(OBJECT_PROPERTY)),
        (N::E("p"), N::V(SUB_PROPERTY), N::E("q")),
        (N::E("a"), N::E("p"), N::E("b")),
    ]);
    let bgp = vec![pattern(var("x"), &ex("q"), var("y"))];
    let (augmented, _) =
        purrdf_entail::materialize_dl_reported(&dataset, &bgp).expect("consistent");
    assert!(
        holds(&augmented, &ex("a"), &ex("q"), &ex("b")),
        "the entailed super-property edge must reach the augmentation"
    );

    // A property the query never names costs nothing and is not injected.
    let unrelated = vec![pattern(var("x"), &ex("unrelated"), var("y"))];
    let (narrow, _) =
        purrdf_entail::materialize_dl_reported(&dataset, &unrelated).expect("consistent");
    assert!(
        !holds(&narrow, &ex("a"), &ex("q"), &ex("b")),
        "the injection is query-DIRECTED: {} quads",
        narrow.quads().count()
    );
}

#[test]
fn the_augmentation_states_a_role_assertion_transitivity_entails() {
    let dataset = ds(&[
        typed(N::E("p"), N::V(TRANSITIVE_PROPERTY)),
        (N::E("a"), N::E("p"), N::E("b")),
        (N::E("b"), N::E("p"), N::E("c")),
    ]);
    let bgp = vec![pattern(var("x"), &ex("p"), var("y"))];
    let (augmented, _) =
        purrdf_entail::materialize_dl_reported(&dataset, &bgp).expect("consistent");
    assert!(
        holds(&augmented, &ex("a"), &ex("p"), &ex("c")),
        "transitivity entails a p c"
    );
}

#[test]
fn the_augmentation_states_a_subsumption_only_an_assertion_entails() {
    // `Only ≡ {alice}` and `alice : Female` entail `Only ⊑ Female`. The old TBox-only
    // subsumption test could not see the assertion, so the augmentation withheld an
    // entailed atom while claiming to hold every one.
    let dataset = ds(&[
        typed(N::E("Female"), N::V(OWL_CLASS)),
        typed(N::E("Only"), N::V(OWL_CLASS)),
        (N::E("Only"), N::V(EQUIVALENT_CLASS), N::B("justAlice")),
        (N::B("justAlice"), N::V(ONE_OF), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("alice")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
        typed(N::E("alice"), N::E("Female")),
    ]);
    let bgp = vec![QTriple {
        s: var("c"),
        p: QNode::Term(TermValue::iri(SUB_CLASS)),
        o: var("d"),
    }];
    let (augmented, _) =
        purrdf_entail::materialize_dl_reported(&dataset, &bgp).expect("consistent");
    assert!(
        holds(&augmented, &ex("Only"), SUB_CLASS, &ex("Female")),
        "an ABox-dependent subsumption must reach the augmentation"
    );
}

#[test]
fn a_query_blank_node_outside_a_class_expression_raises_the_residue_boundary() {
    let dataset = ds(&[
        typed(N::E("C"), N::V(OWL_CLASS)),
        typed(N::E("a"), N::E("C")),
        (N::E("a"), N::E("p"), N::E("b")),
    ]);
    // `_:existential p ?y` — the blank node is a non-distinguished variable, and no finite
    // augmentation answers a BGP that has one.
    let bgp = vec![pattern(
        QNode::Term(TermValue::blank("existential")),
        &ex("p"),
        var("y"),
    )];
    let (_, report) = purrdf_entail::materialize_dl_reported(&dataset, &bgp).expect("consistent");
    let names: Vec<&str> = report
        .boundaries()
        .iter()
        .map(|boundary| boundary.construct().as_str())
        .collect();
    assert!(
        names.contains(&"non-distinguished-variable"),
        "the residue must be reported: {names:?}"
    );
    assert_eq!(
        report.completeness(),
        purrdf_entail::Completeness::ExactWithinBoundaries,
        "a run that met the residue is not exact"
    );
}

#[test]
fn a_class_expression_scaffold_blank_node_is_not_a_non_distinguished_variable() {
    let dataset = ds(&[
        typed(N::E("C"), N::V(OWL_CLASS)),
        typed(N::E("a"), N::E("C")),
        (N::E("a"), N::E("p"), N::E("b")),
    ]);
    // `?x rdf:type [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom owl:Thing ]`
    // — the blank nodes are SYNTAX for a class, not existential variables.
    let bnode = |label: &str| QNode::Term(TermValue::blank(label));
    let bgp = vec![
        QTriple {
            s: var("x"),
            p: QNode::Term(TermValue::iri(TYPE)),
            o: bnode("r"),
        },
        QTriple {
            s: bnode("r"),
            p: QNode::Term(TermValue::iri(TYPE)),
            o: QNode::Term(TermValue::iri(RESTRICTION)),
        },
        QTriple {
            s: bnode("r"),
            p: QNode::Term(TermValue::iri(ON_PROPERTY)),
            o: QNode::Term(TermValue::iri(ex("p"))),
        },
        QTriple {
            s: bnode("r"),
            p: QNode::Term(TermValue::iri(SOME_VALUES)),
            o: QNode::Term(TermValue::iri(THING)),
        },
    ];
    let (_, report) = purrdf_entail::materialize_dl_reported(&dataset, &bgp).expect("consistent");
    let names: Vec<&str> = report
        .boundaries()
        .iter()
        .map(|boundary| boundary.construct().as_str())
        .collect();
    assert!(
        !names.contains(&"non-distinguished-variable"),
        "a class-expression scaffold is ground syntax: {names:?}"
    );
}

// --- locality module extraction ----------------------------------------------------------

/// A many-branched fixture: a chain, a sibling, a disjointness, a role with a domain and a
/// range, an annotation, and a class expression under a blank node.
fn zoo() -> Arc<RdfDataset> {
    ds(&[
        typed(N::E("Animal"), N::V(OWL_CLASS)),
        typed(N::E("Mammal"), N::V(OWL_CLASS)),
        typed(N::E("Cat"), N::V(OWL_CLASS)),
        typed(N::E("Fish"), N::V(OWL_CLASS)),
        typed(N::E("Plant"), N::V(OWL_CLASS)),
        sub(N::E("Cat"), N::E("Mammal")),
        sub(N::E("Mammal"), N::E("Animal")),
        sub(N::E("Fish"), N::E("Animal")),
        (N::E("Animal"), N::V(DISJOINT_WITH), N::E("Plant")),
        // `Cat ⊑ ∃eats.Fish` — a class expression under a blank node.
        sub(N::E("Cat"), N::B("eatsFish")),
        typed(N::B("eatsFish"), N::V(RESTRICTION)),
        (N::B("eatsFish"), N::V(ON_PROPERTY), N::E("eats")),
        (N::B("eatsFish"), N::V(SOME_VALUES), N::E("Fish")),
        typed(N::E("eats"), N::V(OBJECT_PROPERTY)),
        (N::E("eats"), N::V(SUB_PROPERTY), N::E("interactsWith")),
        // `∃eats.⊤ ⊑ Predator` — a GCI whose LEFT-hand side is complex. Its subject is a
        // blank node NOTHING points at, so an extractor that classifies named subjects only
        // never sees it, and `Cat ⊑ Predator` silently stops being entailed by the module.
        typed(N::E("Predator"), N::V(OWL_CLASS)),
        (N::B("eatsAnything"), N::V(SUB_CLASS), N::E("Predator")),
        typed(N::B("eatsAnything"), N::V(RESTRICTION)),
        (N::B("eatsAnything"), N::V(ON_PROPERTY), N::E("eats")),
        (N::B("eatsAnything"), N::V(SOME_VALUES), N::V(THING)),
        typed(N::E("tom"), N::E("Cat")),
        typed(N::E("nemo"), N::E("Fish")),
    ])
}

/// Every entailment of `full` expressible over `seed` must also be an entailment of
/// `module` — the SOUNDNESS doctrine, checked rather than asserted.
///
/// The comparison is driven by the SEED, never by the module's own signature: a signature
/// read back off the module shrinks exactly when the module under-extracts, which would
/// make this check pass by losing the very rows it exists to demand.
fn module_preserves_entailments(full: &Arc<RdfDataset>, module: &Arc<RdfDataset>, seed: &[&str]) {
    let names: Vec<String> = seed.iter().map(|local| ex(local)).collect();
    let inside = |row: &(String, String)| names.contains(&row.0) && names.contains(&row.1);

    let full_reasoner = Reasoner::new(full).expect("reverse-map the full ontology");
    let module_reasoner = Reasoner::new(module).expect("reverse-map the module");

    let full_hierarchy = full_reasoner.classify().expect("consistent");
    let module_hierarchy = module_reasoner.classify().expect("consistent");
    honest(&full_hierarchy);
    honest(&module_hierarchy);
    let kept = pairs(module_hierarchy.answer().subsumptions());
    for row in pairs(full_hierarchy.answer().subsumptions())
        .into_iter()
        .filter(inside)
    {
        assert!(
            kept.contains(&row),
            "the module dropped the entailed subsumption {row:?} over seed {seed:?}"
        );
    }

    let full_realization = full_reasoner.realize().expect("consistent");
    let module_realization = module_reasoner.realize().expect("consistent");
    honest(&full_realization);
    honest(&module_realization);
    let kept = pairs(module_realization.answer().types());
    for row in pairs(full_realization.answer().types())
        .into_iter()
        .filter(inside)
    {
        assert!(
            kept.contains(&row),
            "the module dropped the entailed type {row:?} over seed {seed:?}"
        );
    }

    // Disjointness is not visible in a hierarchy or a realization, and it is exactly the
    // axiom shape whose locality rule differs from every other one, so it is asked directly.
    let mut full_reasoner = full_reasoner;
    let mut module_reasoner = module_reasoner;
    for left in seed {
        for right in seed {
            let axiom = DlAxiom::DisjointClasses {
                left: iri(left),
                right: iri(right),
            };
            let expected = full_reasoner.entails(&axiom).expect("consistent");
            let got = module_reasoner.entails(&axiom).expect("consistent");
            honest(&expected);
            honest(&got);
            if expected.answer().is_true() {
                assert_eq!(
                    *got.answer(),
                    Verdict::True,
                    "the module dropped the entailed disjointness of {left} and {right}"
                );
            }
        }
    }
}

#[test]
fn a_module_entails_everything_the_full_ontology_entails_over_its_signature() {
    let full = zoo();
    // Seeds that actually have something to preserve — several related terms each, so a
    // module that under-extracts loses a row this check demands. A one-term seed would make
    // the comparison vacuous, which is a way of passing rather than a way of holding.
    for seed in [
        vec!["Cat", "Animal"],
        vec!["Cat", "Mammal", "Animal"],
        vec!["tom", "Cat", "Animal"],
        vec!["nemo", "Fish", "Animal"],
        vec!["Animal", "Plant"],
        vec!["Cat", "Plant", "Animal", "Mammal"],
        vec!["Cat", "Fish", "eats", "interactsWith"],
        // `Cat ⊑ ∃eats.Fish ⊑ ∃eats.⊤ ⊑ Predator` — entailed only through the GCI with a
        // complex left-hand side.
        vec!["Cat", "Predator"],
        vec!["tom", "Predator", "Animal"],
    ] {
        for method in purrdf_entail::ModuleMethod::ALL {
            let terms: Vec<TermValue> = seed.iter().map(|local| iri(local)).collect();
            let extracted = purrdf_entail::extract_module(&full, &terms, method).expect("extract");
            module_preserves_entailments(&full, extracted.module(), &seed);
            assert!(
                extracted.module().quads().count() <= full.quads().count(),
                "a module is a SUBSET of the ontology"
            );
            for term in extracted.signature() {
                let _ = term;
            }
        }
    }
}

#[test]
fn an_axiom_whose_only_signature_contact_is_inside_its_complex_left_side_is_kept() {
    // `∃eats.⊤ ⊑ Predator`: the seed names only `eats`, which occurs nowhere in the axiom
    // except INSIDE the blank-node class expression on its left. An extractor that tested
    // the object endpoint alone would drop it.
    let extracted =
        purrdf_entail::extract_module(&zoo(), &[iri("eats")], purrdf_entail::ModuleMethod::Bot)
            .expect("extract");
    let module = extracted.module();
    let kept_lhs = module.quads().any(|quad| {
        matches!(module.term_value(quad.s), TermValue::Blank { .. })
            && module.term_value(quad.p) == TermValue::iri(SUB_CLASS)
            && module.term_value(quad.o) == TermValue::iri(ex("Predator"))
    });
    assert!(
        kept_lhs,
        "an axiom reached only through its complex left-hand side must survive"
    );
    assert!(
        extracted
            .signature()
            .iter()
            .map(show)
            .any(|t| t == ex("Predator")),
        "…and its conclusion joins the signature: {:?}",
        extracted.signature().iter().map(show).collect::<Vec<_>>()
    );
}

#[test]
fn bot_follows_the_chain_upward_and_top_follows_it_downward() {
    let full = zoo();
    let seed = [iri("Mammal")];

    let bot = purrdf_entail::extract_module(&full, &seed, purrdf_entail::ModuleMethod::Bot)
        .expect("extract");
    let bot_signature: Vec<String> = bot.signature().iter().map(show).collect();
    assert!(
        bot_signature.contains(&ex("Animal")),
        "⊥-locality follows `Mammal ⊑ Animal` upward: {bot_signature:?}"
    );
    assert!(
        !bot_signature.contains(&ex("Fish")),
        "…and does not reach a sibling of `Mammal`: {bot_signature:?}"
    );

    let top = purrdf_entail::extract_module(&full, &seed, purrdf_entail::ModuleMethod::Top)
        .expect("extract");
    let top_signature: Vec<String> = top.signature().iter().map(show).collect();
    assert!(
        top_signature.contains(&ex("Cat")),
        "⊤-locality follows `Cat ⊑ Mammal` downward: {top_signature:?}"
    );
    assert!(
        !top_signature.contains(&ex("Animal")),
        "…and does not reach `Mammal`'s subsumer: {top_signature:?}"
    );
}

#[test]
fn a_module_carries_the_whole_class_expression_under_a_kept_axiom() {
    let full = zoo();
    let seed = [iri("Cat")];
    let extracted = purrdf_entail::extract_module(&full, &seed, purrdf_entail::ModuleMethod::Bot)
        .expect("extract");
    // `Cat ⊑ ∃eats.Fish` is kept, so the restriction's THREE defining triples must all be
    // in the module — a class expression truncated halfway is not a class expression.
    let module = extracted.module();
    let restriction_triples = module
        .quads()
        .filter(|quad| {
            matches!(
                module.term_value(quad.s),
                TermValue::Blank { ref label, .. } if label == "eatsFish"
            )
        })
        .count();
    assert_eq!(
        restriction_triples, 3,
        "the blank-node closure of a kept axiom rides along whole"
    );
    // …and the class expression's own vocabulary joined the signature.
    let signature: Vec<String> = extracted.signature().iter().map(show).collect();
    assert!(signature.contains(&ex("eats")), "{signature:?}");
    assert!(signature.contains(&ex("Fish")), "{signature:?}");
}

#[test]
fn a_construct_locality_does_not_decide_is_kept_conservatively_and_reported() {
    // `owl:hasKey` has no exact locality rule here; its subject is in the seed, so it is
    // kept — and the keep is REPORTED rather than passed off as an exact one.
    let full = ds(&[
        typed(N::E("Cat"), N::V(OWL_CLASS)),
        (
            N::E("Cat"),
            N::V("http://www.w3.org/2002/07/owl#hasKey"),
            N::B("cell0"),
        ),
        (N::B("cell0"), N::V(FIRST), N::E("chipId")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
    ]);
    let extracted =
        purrdf_entail::extract_module(&full, &[iri("Cat")], purrdf_entail::ModuleMethod::Bot)
            .expect("extract");
    let reported: Vec<(String, String)> = extracted
        .conservative_keeps()
        .iter()
        .map(|keep| (show(keep.subject()), show(keep.predicate())))
        .collect();
    assert_eq!(
        reported,
        vec![(ex("Cat"), "http://www.w3.org/2002/07/owl#hasKey".to_owned())],
        "a conservative keep is visible, not absorbed"
    );
    assert!(
        extracted
            .signature()
            .iter()
            .map(show)
            .any(|t| t == ex("chipId")),
        "a conservative keep pulls what it reaches into the signature"
    );
}

#[test]
fn an_empty_seed_extracts_an_empty_module() {
    let extracted = purrdf_entail::extract_module(&zoo(), &[], purrdf_entail::ModuleMethod::Star)
        .expect("extract");
    assert_eq!(extracted.axioms(), 0);
    assert_eq!(extracted.module().quads().count(), 0);
    assert!(extracted.conservative_keeps().is_empty());
}

#[test]
fn module_extraction_is_reproducible() {
    let full = zoo();
    let seed = [iri("Cat"), iri("Plant")];
    for method in purrdf_entail::ModuleMethod::ALL {
        let one = purrdf_entail::extract_module(&full, &seed, method).expect("extract");
        let two = purrdf_entail::extract_module(&full, &seed, method).expect("extract");
        assert_eq!(one.signature(), two.signature());
        assert_eq!(one.conservative_keeps(), two.conservative_keeps());
        assert_eq!(one.axioms(), two.axioms());
        let quads = |extraction: &purrdf_entail::ModuleExtraction| {
            let module = Arc::clone(extraction.module());
            module
                .quads()
                .map(|quad| {
                    (
                        show(&module.term_value(quad.s)),
                        show(&module.term_value(quad.p)),
                        show(&module.term_value(quad.o)),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(quads(&one), quads(&two), "{method} is byte-stable");
    }
}

// --- profile certification ---------------------------------------------------------------

/// The profiles a certificate blocks, as strings, for readable assertions.
fn blocked(certificate: &ProfileCertificate) -> Vec<&'static str> {
    OwlProfile::ALL
        .into_iter()
        .filter(|&p| !certificate.certifies(p))
        .map(OwlProfile::as_str)
        .collect()
}

#[test]
fn a_bare_hierarchy_certifies_every_profile() {
    let certificate = profile(&kittens());
    assert_eq!(certificate.certified(), OwlProfile::ALL.to_vec());
    assert!(
        certificate.violations().is_empty(),
        "{:?}",
        certificate.violations()
    );
}

#[test]
fn owl_full_is_certified_unconditionally() {
    // An ontology that is in NO other profile: a chain axiom, a union in superclass
    // position, a min-cardinality, and a reserved term the mapping does not know.
    let dataset = ds(&[
        (N::E("chained"), N::V(PROPERTY_CHAIN), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("q")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
        sub(N::E("A"), N::B("either")),
        (N::B("either"), N::V(UNION), N::B("cell1")),
        (N::B("cell1"), N::V(FIRST), N::E("B")),
        (N::B("cell1"), N::V(REST), N::V(NIL)),
        sub(N::E("A"), N::B("atLeast")),
        typed(N::B("atLeast"), N::V(RESTRICTION)),
        (N::B("atLeast"), N::V(ON_PROPERTY), N::E("q")),
        (
            N::B("atLeast"),
            N::V(MIN_CARDINALITY),
            N::L("1", XSD_NON_NEGATIVE_INTEGER),
        ),
    ]);
    let certificate = profile(&dataset);
    assert!(certificate.certifies(OwlProfile::Full));
    assert_eq!(blocked(&certificate), vec!["EL", "QL", "RL", "DL"]);
}

#[test]
fn a_union_is_in_rl_beneath_an_inclusion_and_in_neither_el_nor_ql() {
    // `(B ⊔ C) ⊑ A` — the union is in SUBCLASS position, which OWL 2 RL admits.
    let dataset = ds(&[
        sub(N::B("either"), N::E("A")),
        (N::B("either"), N::V(UNION), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("B")),
        (N::B("cell0"), N::V(REST), N::B("cell1")),
        (N::B("cell1"), N::V(FIRST), N::E("C")),
        (N::B("cell1"), N::V(REST), N::V(NIL)),
    ]);
    let certificate = profile(&dataset);
    assert!(
        certificate.certifies(OwlProfile::Rl),
        "{:?}",
        certificate.violations()
    );
    assert!(certificate.certifies(OwlProfile::Dl));
    assert_eq!(blocked(&certificate), vec!["EL", "QL"]);

    // The SAME union above the inclusion is outside RL, because no rule head can produce a
    // disjunction.
    let flipped = ds(&[
        sub(N::E("A"), N::B("either")),
        (N::B("either"), N::V(UNION), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("B")),
        (N::B("cell0"), N::V(REST), N::B("cell1")),
        (N::B("cell1"), N::V(FIRST), N::E("C")),
        (N::B("cell1"), N::V(REST), N::V(NIL)),
    ]);
    let certificate = profile(&flipped);
    assert!(!certificate.certifies(OwlProfile::Rl));
    let reason = certificate.violations_of(OwlProfile::Rl)[0].reason();
    assert!(
        reason.starts_with("only in"),
        "a POSITIONAL exclusion says so: {reason}"
    );
}

#[test]
fn an_existential_is_in_el_and_in_rl_only_beneath_an_inclusion() {
    // `∃p.C ⊑ A` — subclass position.
    let below = ds(&[
        sub(N::B("r"), N::E("A")),
        typed(N::B("r"), N::V(RESTRICTION)),
        (N::B("r"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("r"), N::V(SOME_VALUES), N::E("C")),
    ]);
    let certificate = profile(&below);
    assert!(
        certificate.certifies(OwlProfile::El),
        "{:?}",
        certificate.violations()
    );
    assert!(certificate.certifies(OwlProfile::Rl));

    // `A ⊑ ∃p.C` — superclass position, which RL excludes and EL admits.
    let above = ds(&[
        sub(N::E("A"), N::B("r")),
        typed(N::B("r"), N::V(RESTRICTION)),
        (N::B("r"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("r"), N::V(SOME_VALUES), N::E("C")),
    ]);
    let certificate = profile(&above);
    assert!(certificate.certifies(OwlProfile::El));
    assert!(!certificate.certifies(OwlProfile::Rl));
}

#[test]
fn rl_admits_a_max_cardinality_of_one_and_refuses_a_max_of_two() {
    let build = |bound: &'static str| {
        ds(&[
            sub(N::E("A"), N::B("r")),
            typed(N::B("r"), N::V(RESTRICTION)),
            (N::B("r"), N::V(ON_PROPERTY), N::E("p")),
            (
                N::B("r"),
                N::V(MAX_CARDINALITY),
                N::L(bound, XSD_NON_NEGATIVE_INTEGER),
            ),
        ])
    };
    for bound in ["0", "1"] {
        let certificate = profile(&build(bound));
        assert!(
            certificate.certifies(OwlProfile::Rl),
            "RL admits max {bound}: {:?}",
            certificate.violations()
        );
    }
    let certificate = profile(&build("2"));
    assert!(
        !certificate.certifies(OwlProfile::Rl),
        "RL admits max cardinality only at 0 or 1"
    );
    // EL admits no cardinality restriction at any bound.
    assert!(!profile(&build("1")).certifies(OwlProfile::El));
}

#[test]
fn el_admits_a_singleton_enumeration_and_refuses_a_larger_one() {
    let build = |members: &[N]| {
        let mut triples = vec![
            sub(N::E("A"), N::B("some")),
            (N::B("some"), N::V(ONE_OF), N::B("cell0")),
        ];
        for (index, member) in members.iter().enumerate() {
            triples.push((N::B(cell(index)), N::V(FIRST), member.clone()));
            triples.push(if index + 1 == members.len() {
                (N::B(cell(index)), N::V(REST), N::V(NIL))
            } else {
                (N::B(cell(index)), N::V(REST), N::B(cell(index + 1)))
            });
        }
        ds(&triples)
    };
    assert!(
        profile(&build(&[N::E("alice")])).certifies(OwlProfile::El),
        "ObjectOneOf with one member is in EL"
    );
    assert!(
        !profile(&build(&[N::E("alice"), N::E("bob")])).certifies(OwlProfile::El),
        "an enumeration of two is a disjunction, and EL has none"
    );
}

/// The fixture label of the `index`-th collection cell.
fn cell(index: usize) -> &'static str {
    ["cell0", "cell1", "cell2"][index]
}

#[test]
fn a_property_chain_blocks_dl_because_regularity_is_not_decided() {
    let dataset = ds(&[
        (N::E("chained"), N::V(PROPERTY_CHAIN), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("q")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
    ]);
    let certificate = profile(&dataset);
    assert!(!certificate.certifies(OwlProfile::Dl));
    let reason = certificate.violations_of(OwlProfile::Dl)[0].reason();
    assert!(
        reason.contains("REGULAR"),
        "the reason names the check that is missing: {reason}"
    );
    // …and QL excludes chains outright, while EL and RL admit them.
    assert!(!certificate.certifies(OwlProfile::Ql));
    assert!(
        certificate.certifies(OwlProfile::El),
        "{:?}",
        certificate.violations()
    );
    assert!(certificate.certifies(OwlProfile::Rl));
}

#[test]
fn counting_a_transitive_role_blocks_dl() {
    let build = |transitive: bool| {
        let mut triples = vec![
            sub(N::E("A"), N::B("r")),
            typed(N::B("r"), N::V(RESTRICTION)),
            (N::B("r"), N::V(ON_PROPERTY), N::E("p")),
            (
                N::B("r"),
                N::V(MAX_CARDINALITY),
                N::L("1", XSD_NON_NEGATIVE_INTEGER),
            ),
            (N::E("p"), N::V(SUB_PROPERTY), N::E("ancestor")),
        ];
        if transitive {
            triples.push(typed(N::E("ancestor"), N::V(TRANSITIVE_PROPERTY)));
        }
        ds(&triples)
    };
    assert!(
        profile(&build(false)).certifies(OwlProfile::Dl),
        "counting a SIMPLE role is ordinary OWL 2 DL"
    );
    // `p ⊑ ancestor` and `ancestor` transitive makes `ancestor` non-simple; the restriction
    // counts `p`, which is still simple, so the DL violation is on `ancestor` alone…
    let transitive = build(true);
    let certificate = profile(&transitive);
    assert!(
        certificate.certifies(OwlProfile::Dl),
        "a transitive SUPER-role does not make its sub-role non-simple: {:?}",
        certificate.violations()
    );

    // …whereas counting the transitive role itself does.
    let counted = ds(&[
        sub(N::E("A"), N::B("r")),
        typed(N::B("r"), N::V(RESTRICTION)),
        (N::B("r"), N::V(ON_PROPERTY), N::E("ancestor")),
        (
            N::B("r"),
            N::V(MAX_CARDINALITY),
            N::L("1", XSD_NON_NEGATIVE_INTEGER),
        ),
        (N::E("p"), N::V(SUB_PROPERTY), N::E("ancestor")),
        typed(N::E("p"), N::V(TRANSITIVE_PROPERTY)),
    ]);
    let certificate = profile(&counted);
    assert!(!certificate.certifies(OwlProfile::Dl));
    assert!(
        certificate.violations_of(OwlProfile::Dl)[0]
            .reason()
            .contains("SIMPLE")
    );
}

#[test]
fn one_iri_declared_both_an_object_and_a_data_property_blocks_dl() {
    let dataset = ds(&[
        typed(N::E("p"), N::V(OBJECT_PROPERTY)),
        typed(N::E("p"), N::V(DATATYPE_PROPERTY)),
    ]);
    let certificate = profile(&dataset);
    assert!(!certificate.certifies(OwlProfile::Dl));
    assert!(
        certificate.violations_of(OwlProfile::Dl)[0]
            .reason()
            .contains("pairwise disjoint")
    );
}

#[test]
fn a_malformed_operand_collection_blocks_dl() {
    // A cell with no `rdf:rest` — the walk cannot reach `rdf:nil`.
    let dataset = ds(&[
        sub(N::E("A"), N::B("both")),
        (N::B("both"), N::V(INTERSECTION), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("B")),
    ]);
    let certificate = profile(&dataset);
    assert!(!certificate.certifies(OwlProfile::Dl));
    assert!(
        certificate
            .violations_of(OwlProfile::Dl)
            .iter()
            .any(|violation| violation.reason().contains("well-formed RDF collection"))
    );
}

#[test]
fn a_reserved_term_the_mapping_does_not_know_blocks_dl() {
    let dataset = ds(&[(
        N::E("a"),
        N::V("http://www.w3.org/2002/07/owl#thisTermDoesNotExist"),
        N::E("b"),
    )]);
    let certificate = profile(&dataset);
    assert!(!certificate.certifies(OwlProfile::Dl));
    assert!(
        certificate.violations_of(OwlProfile::Dl)[0]
            .reason()
            .contains("OWL-2-RDF mapping")
    );
    // A term in a caller's OWN namespace is ordinary user vocabulary and blocks nothing.
    let user = ds(&[(N::E("a"), N::E("thisTermDoesNotExist"), N::E("b"))]);
    assert!(profile(&user).certifies(OwlProfile::Dl));
}

#[test]
fn ql_excludes_equality_functionality_transitivity_and_keys() {
    for (triples, why) in [
        (
            vec![(N::E("a"), N::V(SAME_AS), N::E("b"))],
            "SameIndividual",
        ),
        (
            vec![typed(N::E("p"), N::V(FUNCTIONAL_PROPERTY))],
            "FunctionalObjectProperty",
        ),
        (
            vec![typed(N::E("p"), N::V(TRANSITIVE_PROPERTY))],
            "TransitiveObjectProperty",
        ),
    ] {
        let certificate = profile(&ds(&triples));
        assert!(
            !certificate.certifies(OwlProfile::Ql),
            "OWL 2 QL has no {why}: {:?}",
            certificate.violations()
        );
    }
}

#[test]
fn an_unplaceable_class_expression_is_checked_against_both_positions() {
    // A restriction nothing references: no axiom places it, so the certifier treats it as
    // occurring on both sides and demands it satisfy both grammars. `owl:allValuesFrom` is
    // legal in RL's superclass position and not in its subclass position, so the
    // conservative reading DENIES it — which is the safe direction.
    let dataset = ds(&[
        typed(N::B("r"), N::V(RESTRICTION)),
        (N::B("r"), N::V(ON_PROPERTY), N::E("p")),
        (N::B("r"), N::V(ALL_VALUES), N::E("C")),
    ]);
    let certificate = profile(&dataset);
    assert!(
        !certificate.certifies(OwlProfile::Rl),
        "an unplaceable occurrence can cause a violation and must never hide one"
    );
}

#[test]
fn profile_certification_is_reproducible() {
    let dataset = kittens();
    assert_eq!(profile(&dataset), profile(&dataset));
    let complex = ds(&[
        sub(N::E("A"), N::B("either")),
        (N::B("either"), N::V(UNION), N::B("cell0")),
        (N::B("cell0"), N::V(FIRST), N::E("B")),
        (N::B("cell0"), N::V(REST), N::V(NIL)),
        typed(N::E("p"), N::V(FUNCTIONAL_PROPERTY)),
    ]);
    assert_eq!(profile(&complex), profile(&complex));
    let violations = profile(&complex);
    let profiles: Vec<OwlProfile> = violations
        .violations()
        .iter()
        .map(purrdf_entail::ProfileViolation::profile)
        .collect();
    let mut sorted = profiles.clone();
    sorted.sort_unstable();
    assert_eq!(profiles, sorted, "violations are sorted by profile");
}
