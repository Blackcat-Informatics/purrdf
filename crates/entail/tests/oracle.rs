// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The engine-swap oracle: a fixture corpus, committed goldens, and a per-rule registry.
//!
//! # Why this file exists
//!
//! `purrdf-entail`'s closure is produced by a hand-written forward chase
//! (`crates/entail/src/rdfs.rs`). A later change replaces that chase with the DL-clause
//! program `calculus_program(regime)` already declares. That is an engine swap on a live
//! calculus, and the only way to make it reviewable is to know — byte for byte, before the
//! swap — what the current engine answers for a corpus that reaches every rule it fires.
//!
//! The oracle is **committed golden files**, not a retained copy of the old chase. A second
//! implementation kept alive "for comparison" is a second implementation: it has to be
//! maintained, it can itself drift, and when the two disagree there is no third party to
//! say which is right. A golden file is inert. It cannot drift, it diffs in review, and a
//! deliberate change to it is a line in a commit rather than an argument.
//!
//! # What is captured, and what deliberately is not
//!
//! Each golden holds, for one fixture and for each of the five regimes `materialize` can
//! run: the closure as **canonical N-Quads** (`purrdf_core::canonicalize` — the repository's
//! RDFC-1.0 canonicalizer, not a serializer written here) and the [`ReasoningReport`]
//! rendered field by field in the report's own documented order.
//!
//! Canonical N-Quads is a statement about the closure's *quad set*, sorted bytewise. The
//! chase's *emission order* is deliberately not pinned here: it is an internal property of
//! one evaluation strategy that a different engine may legitimately choose differently, and
//! it already has its own guard (`rdfs_emission_order_is_deterministic` in the crate's unit
//! tests). What an engine swap must preserve is the closure, and that is what these
//! goldens are.
//!
//! # Fixture inputs are Rust, not data files
//!
//! There is no N-Quads *parser* in this crate's dependency set — parsers live in
//! `purrdf-rdf`, which `purrdf-entail` does not depend on and, under this branch's
//! constraints, may not — so a fixture is declared as a small table of [`Quad`] values and
//! built with `RdfDatasetBuilder`. The input's canonical N-Quads form is nevertheless
//! written into the golden, so the fixture a golden was captured from is itself pinned and
//! readable; editing a fixture table without regenerating fails the gate.
//!
//! # The shipped wasm artifact
//!
//! Nothing here reaches it. This is an integration test (`tests/`), which is never part of
//! the library `cdylib`/`rlib`, and the goldens are read from disk with [`std::fs`] at test
//! time rather than embedded with `include_str!`, so their bytes are not in any compiled
//! artifact at all — shipped or otherwise.
//!
//! # Regenerating
//!
//! ```text
//! cargo test -p purrdf-entail --test oracle -- --ignored --exact regenerate_goldens
//! ```
//!
//! Deliberately `#[ignore]`d so a normal `cargo test` can only ever *compare*.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::str::FromStr as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId, TermValue, canonicalize};
use purrdf_entail::{
    Completeness, EntailError, InconsistencyWitness, ReasoningReport, Regime, RuleId, implemented,
    materialize, rules,
};

// ── Vocabulary ──────────────────────────────────────────────────────────────────
//
// Specification IRIs, spelled out. The crate's own `vocab` module is `pub(crate)`, and
// that is the right shape: an oracle that imported the engine's constants would agree with
// the engine by construction. Fixture-local terms are all `example.org` — PurRDF mints no
// vocabulary.

/// `rdf:type`.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:Property`.
const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
/// `rdfs:subClassOf`.
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
/// `rdfs:subPropertyOf`.
const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
/// `rdfs:domain`.
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
/// `rdfs:range`.
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
/// `rdfs:Class`.
const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
/// `rdfs:Resource`.
const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
/// `rdfs:Literal`.
const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
/// `rdfs:Datatype`.
const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
/// `rdfs:ContainerMembershipProperty`.
const RDFS_CONTAINERMEMBERSHIPPROPERTY: &str =
    "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty";
/// `rdfs:member`.
const RDFS_MEMBER: &str = "http://www.w3.org/2000/01/rdf-schema#member";
/// `rdf:reifies` — RDF 1.2's reifier property.
///
/// Reserved vocabulary the entailment rules say NOTHING special about: an `rdf:reifies`
/// annotation triple is an ordinary triple and flows through `prp-dom`, `prp-rng`,
/// `prp-spo1` and the `scm-*` family exactly as `example.org/p` does. The `reifies_*`
/// fixtures below enumerate that, position by position and rule by rule.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
/// `owl:SymmetricProperty`.
const OWL_SYMMETRIC: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
/// `owl:TransitiveProperty`.
const OWL_TRANSITIVE: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
/// `owl:inverseOf`.
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
/// `owl:equivalentClass`.
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
/// `owl:equivalentProperty`.
const OWL_EQUIVALENTPROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
/// `xsd:string`.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// `owl:AnnotationProperty`.
const OWL_ANNOTATIONPROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
/// `owl:FunctionalProperty`.
const OWL_FUNCTIONALPROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
/// `owl:InverseFunctionalProperty`.
const OWL_INVERSEFUNCTIONALPROPERTY: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
/// `owl:propertyChainAxiom`.
const OWL_PROPERTYCHAINAXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
/// `owl:hasKey`.
const OWL_HASKEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
/// `owl:sameAs`.
const OWL_SAMEAS: &str = "http://www.w3.org/2002/07/owl#sameAs";
/// `owl:Class`.
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
/// `owl:Thing`.
const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
/// `owl:ObjectProperty`.
const OWL_OBJECTPROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
/// `owl:DatatypeProperty`.
const OWL_DATATYPEPROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
/// `owl:onProperty`.
const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
/// `owl:hasValue`.
const OWL_HASVALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
/// `owl:someValuesFrom`.
const OWL_SOMEVALUESFROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
/// `owl:allValuesFrom`.
const OWL_ALLVALUESFROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
/// `owl:intersectionOf`.
const OWL_INTERSECTIONOF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
/// `owl:unionOf`.
const OWL_UNIONOF: &str = "http://www.w3.org/2002/07/owl#unionOf";
/// `rdfs:label` — a built-in annotation property, and `prp-ap`'s witness.
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
/// `owl:Nothing` — `cls-nothing1`'s subject and `cls-nothing2`'s class.
const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
/// `owl:differentFrom` — what `dt-diff` concludes and `eq-diff1` clashes against.
const OWL_DIFFERENTFROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
/// `owl:AllDifferent` — the class `eq-diff2` and `eq-diff3` read a list off.
const OWL_ALLDIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";
/// `owl:members` — the list-valued property of `owl:AllDifferent` and `owl:AllDisjoint*`.
const OWL_MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";
/// `owl:distinctMembers` — `owl:AllDifferent`'s other list-valued property.
const OWL_DISTINCTMEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
/// `owl:IrreflexiveProperty`.
const OWL_IRREFLEXIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";
/// `owl:AsymmetricProperty`.
const OWL_ASYMMETRICPROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";
/// `owl:propertyDisjointWith`.
const OWL_PROPERTYDISJOINTWITH: &str = "http://www.w3.org/2002/07/owl#propertyDisjointWith";
/// `owl:AllDisjointProperties`.
const OWL_ALLDISJOINTPROPERTIES: &str = "http://www.w3.org/2002/07/owl#AllDisjointProperties";
/// `owl:AllDisjointClasses`.
const OWL_ALLDISJOINTCLASSES: &str = "http://www.w3.org/2002/07/owl#AllDisjointClasses";
/// `owl:disjointWith`.
const OWL_DISJOINTWITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
/// `owl:complementOf`.
const OWL_COMPLEMENTOF: &str = "http://www.w3.org/2002/07/owl#complementOf";
/// `owl:oneOf`.
const OWL_ONEOF: &str = "http://www.w3.org/2002/07/owl#oneOf";
/// `owl:maxCardinality`.
const OWL_MAXCARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
/// `owl:maxQualifiedCardinality`.
const OWL_MAXQUALIFIEDCARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
/// `owl:onClass`.
const OWL_ONCLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
/// `owl:sourceIndividual` — a negative property assertion's subject.
const OWL_SOURCEINDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#sourceIndividual";
/// `owl:assertionProperty` — a negative property assertion's predicate.
const OWL_ASSERTIONPROPERTY: &str = "http://www.w3.org/2002/07/owl#assertionProperty";
/// `owl:targetIndividual` — a negative OBJECT-property assertion's object.
const OWL_TARGETINDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#targetIndividual";
/// `owl:targetValue` — a negative DATA-property assertion's object.
const OWL_TARGETVALUE: &str = "http://www.w3.org/2002/07/owl#targetValue";
/// `xsd:nonNegativeInteger` — the datatype OWL 2 Profiles Table 6 writes every cardinality
/// literal of `cls-maxc1`, `cls-maxc2` and the four `cls-maxqc*` rules with.
const XSD_NONNEGATIVEINTEGER: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
/// `xsd:integer` — a datatype supported in OWL 2 RL, and NOT one of the three RDF 1.2
/// Semantics §8 makes mandatory, which is what makes it `dt-type1`'s witness.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// `rdf:first`.
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
/// `rdf:rest`.
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
/// `rdf:nil`.
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// Fixture class `example.org/A`.
const EX_A: &str = "http://example.org/A";
/// Fixture class `example.org/B`.
const EX_B: &str = "http://example.org/B";
/// Fixture class `example.org/C`.
const EX_C: &str = "http://example.org/C";
/// Fixture class `example.org/D`.
const EX_D: &str = "http://example.org/D";
/// Fixture class `example.org/E`.
const EX_E: &str = "http://example.org/E";
/// Fixture class `example.org/F`.
const EX_F: &str = "http://example.org/F";
/// A fixture class that is deliberately NOT typed `rdfs:Class`.
const EX_NOT_A_CLASS: &str = "http://example.org/NotAClass";
/// A fixture datatype IRI. It is `example.org`, so it is NOT one of the datatypes
/// RDF 1.2 Semantics §8 makes every interpretation recognize — which is what makes it a
/// usable control for `rdfs1`.
const EX_DT: &str = "http://example.org/dt";
/// Fixture property `example.org/p`.
const EX_P: &str = "http://example.org/p";
/// Fixture property `example.org/q`.
const EX_Q: &str = "http://example.org/q";
/// Fixture property `example.org/r`.
const EX_R: &str = "http://example.org/r";
/// Fixture property `example.org/says`.
const EX_SAYS: &str = "http://example.org/says";
/// Fixture property `example.org/mentions`.
const EX_MENTIONS: &str = "http://example.org/mentions";
/// Fixture REIFIER resource `example.org/reifier` — the subject of an `rdf:reifies`
/// annotation triple.
///
/// An IRI rather than a blank node, deliberately: RDF 1.2 admits either, and an IRI keeps
/// every assertion below about the RULES rather than about blank-node relabelling.
const EX_REIFIER: &str = "http://example.org/reifier";
/// Fixture individual `example.org/t` — an `rdf:reifies` OBJECT that is an IRI.
///
/// RDF 1.2 writes a triple TERM there, and the `reifies_*` fixtures use one wherever the
/// interaction is about the triple term. This constant is for the cases where it is not:
/// `prp-rng` concludes into SUBJECT position, and a triple term cannot occupy one, so an
/// IRI object is what makes that rule's conclusion observable at all rather than dropped
/// as generalized RDF.
const EX_T: &str = "http://example.org/t";
/// Fixture individual `example.org/x`.
const EX_X: &str = "http://example.org/x";
/// Fixture individual `example.org/y`.
const EX_Y: &str = "http://example.org/y";
/// Fixture individual `example.org/z`.
const EX_Z: &str = "http://example.org/z";
/// Fixture individual `example.org/u`.
const EX_U: &str = "http://example.org/u";
/// Fixture individual `example.org/v`.
const EX_V: &str = "http://example.org/v";
/// Fixture named graph `example.org/g`.
const EX_G: &str = "http://example.org/g";
/// A SECOND fixture named graph, `example.org/h`.
///
/// Two named graphs is the smallest dataset that can show a cross-graph join NOT happening:
/// each named graph is closed against the union of itself and the default graph, so `h` is
/// never in `g`'s seed and a premise pair split across the two derives nothing.
const EX_H: &str = "http://example.org/h";
/// Fixture individual `example.org/w`.
const EX_W: &str = "http://example.org/w";
/// Fixture property `example.org/chained`, the head of a property-chain axiom.
const EX_CHAINED: &str = "http://example.org/chained";
/// The first cell of a fixture RDF collection.
const EX_L0: &str = "http://example.org/l0";
/// The second cell of a fixture RDF collection.
const EX_L1: &str = "http://example.org/l1";

// ── The fixture model ───────────────────────────────────────────────────────────

/// One object-position term a fixture quad can hold.
///
/// Subjects are always IRIs and predicates are always IRIs, so only the object slot needs
/// a sum type. That is exactly the RDF 1.2 shape the chase can *consume*; the interesting
/// cases are the ones it cannot *conclude into*, which is what the object slot supplies.
#[derive(Debug, Clone, Copy)]
enum Term {
    /// An IRI.
    Iri(&'static str),
    /// A datatyped literal.
    Literal {
        /// The lexical form.
        lexical: &'static str,
        /// The datatype IRI.
        datatype: &'static str,
    },
    /// An RDF 1.2 triple term over three IRIs.
    Quoted(&'static str, &'static str, &'static str),
}

/// One quad of a fixture: an IRI subject, an IRI predicate, an object, and a graph.
#[derive(Debug, Clone, Copy)]
struct Quad {
    /// The subject IRI.
    s: &'static str,
    /// The predicate IRI.
    p: &'static str,
    /// The object term.
    o: Term,
    /// The graph IRI; `None` is the default graph.
    g: Option<&'static str>,
}

/// A default-graph triple over three IRIs — the common case.
const fn t(s: &'static str, p: &'static str, o: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Iri(o),
        g: None,
    }
}

/// A default-graph triple whose object is a datatyped literal.
const fn t_lit(s: &'static str, p: &'static str, lexical: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Literal {
            lexical,
            datatype: XSD_STRING,
        },
        g: None,
    }
}

/// A default-graph triple whose object is a literal of an explicit datatype.
///
/// [`t_lit`] is the `xsd:string` case; this one is what the `dt-*` family and OWL 2
/// Profiles Table 6's cardinality literals need, where the datatype IS the premise.
const fn t_lit_dt(
    s: &'static str,
    p: &'static str,
    lexical: &'static str,
    datatype: &'static str,
) -> Quad {
    Quad {
        s,
        p,
        o: Term::Literal { lexical, datatype },
        g: None,
    }
}

/// A default-graph triple whose object is an RDF 1.2 triple term.
const fn t_quoted(
    s: &'static str,
    p: &'static str,
    qs: &'static str,
    qp: &'static str,
    qo: &'static str,
) -> Quad {
    Quad {
        s,
        p,
        o: Term::Quoted(qs, qp, qo),
        g: None,
    }
}

/// A triple over three IRIs, placed in the named graph `g`.
const fn t_in(s: &'static str, p: &'static str, o: &'static str, g: &'static str) -> Quad {
    Quad {
        s,
        p,
        o: Term::Iri(o),
        g: Some(g),
    }
}

/// One input dataset of the corpus, with the reason it exists.
#[derive(Debug, Clone, Copy)]
struct Fixture {
    /// The fixture's name; also the golden file's stem.
    name: &'static str,
    /// Why this fixture exists, one line per element. Rendered into the golden header.
    doc: &'static [&'static str],
    /// The specification rule ids this fixture is meant to reach, by canonical spelling.
    ///
    /// Checked to parse as [`RuleId`]s; documentation for the reader, not an assertion
    /// about the closure (the registry below makes those, per rule).
    exercises: &'static [&'static str],
    /// What moved in THIS golden at each change the corpus has seen, and why the new
    /// answer is licensed.
    ///
    /// Rendered into the golden beneath [`ENGINE_SWAP`], [`AXIOMATIC_PATH`],
    /// [`OWL_RL_TABLES`] and [`OWL_RL_COMPLETE`], each of which states one change's causes
    /// ONCE; these lines name the actual triples and tallies each cause moved HERE. It is
    /// append-only — a fixture's entry accumulates one section per change it lived
    /// through, so a reader of one golden can see the whole history of that answer without
    /// leaving the file.
    ///
    /// A golden regenerated without an entry is the defect this corpus exists to catch, so
    /// the field is not `Option` and an empty slice is a claim — "this golden did not
    /// move" — rather than an omission. A fixture that did not exist before a change says
    /// so instead, which is a different claim and is checked as one; see
    /// [`CLASH_CORPUS`]'s [`REFUSAL_GOLDEN`] for the third case, a fixture that has no golden
    /// at all.
    changed: &'static [&'static str],
    /// The input quads.
    quads: &'static [Quad],
}

/// The three causes of every byte that moved when `materialize` stopped running a
/// hand-written chase and started evaluating [`purrdf_entail::calculus_program`].
///
/// Written into every golden's header, once, above the fixture's own [`Fixture::changed`]
/// accounting — so a reader of one golden file sees both the general reason and the
/// specific triples without having to hold the other twenty-eight in their head.
///
/// A fourth thing that could have moved did NOT, and its absence is asserted rather than
/// assumed: `divergence_literal_subject` still reports the `generalized-rdf` boundary. A
/// Datalog engine derives a literal-subject conclusion in its own term space and meets the
/// RDF 1.2 IR only when the answer is materialized, so the failure mode was the boundary
/// quietly disappearing while the triples still looked right. See
/// `a_would_be_literal_subject_is_abandoned_and_reported`.
const ENGINE_SWAP: &[&str] = &[
    "EVERY GOLDEN IN THIS CORPUS MOVED AT THE ENGINE SWAP — `materialize` stopped",
    "running a hand-written chase and started evaluating the DL-clause program",
    "`calculus_program(regime)` already declared. Three causes account for every byte,",
    "and the fixture's own accounting below names which triples each one moved here.",
    "",
    "  1. THE UNLICENSED REFLEXIVES ARE GONE — a SPEC-CONFORMANCE FIX. The chase",
    "     emitted `c rdfs:subClassOf c` for every subClassOf ENDPOINT and",
    "     `p rdfs:subPropertyOf p` for every PREDICATE. rdfs10 requires `?c rdf:type",
    "     rdfs:Class` and rdfs6 requires `?p rdf:type rdf:Property`, and the declared",
    "     clauses say so, so those conclusions are drawn only where the specification",
    "     licenses them. Both rules still fire — `property_typed` and `class_typed` are",
    "     the fixtures that prove they were narrowed rather than switched off.",
    "",
    "  2. THE RDF LANE IS NOW A FIXPOINT — a BUG FIX. `close_rdf` walked the INPUT",
    "     quads once and typed each predicate it saw, so it never applied rdfD2 to its",
    "     own conclusions. `rdf:type` is a predicate of every one of them, so",
    "     `rdf:type rdf:type rdf:Property` is entailed and was missing. It appears now",
    "     in every RDF closure whose input did not already use `rdf:type` as a",
    "     predicate; where the input did, the RDF closure is unchanged.",
    "",
    "  3. THE BUDGET IS THE EVALUATOR'S OWN MEASUREMENT. `join-steps`,",
    "     `stored-facts` and `term-arena-bytes` are now `purrdf-datalog`'s",
    "     `BudgetReport` rather than a tally the chase kept beside it, so all three",
    "     move in every regime. They are the same three coordinates under the same",
    "     three definitions, counted by the engine that did the work: `stored-facts`",
    "     is the whole saturated store rather than one lane's private index, and",
    "     `term-arena-bytes` counts the terms that actually entered the store rather",
    "     than a vocabulary table interned whether or not the data mentioned it.",
];

/// The four causes of every byte that moved when the `RDFS` lane started asserting the
/// axiomatic triples and firing five more rules.
///
/// Written into every golden's header beneath [`ENGINE_SWAP`], for the same reason that
/// one is: the general cause once, the fixture's own triples in its own
/// [`Fixture::changed`].
///
/// The single strongest fact about this diff is stated first, because it makes the other
/// thirty-two goldens checkable at a glance: EVERY changed golden changed in the `RDFS`
/// section and in no other. `Simple`, `RDF` and `OWL-RL` are byte-identical across all
/// twenty-nine goldens that already existed.
const AXIOMATIC_PATH: &[&str] = &[
    "AND EVERY GOLDEN MOVED AGAIN, IN THE RDFS SECTION AND NOWHERE ELSE — the RDFS lane",
    "now asserts the axiomatic triples and fires five more rules. `Simple`, `RDF` and",
    "`OWL-RL` are byte-identical to the previous golden in all twenty-nine files that",
    "already existed. Four causes account for the new bytes.",
    "",
    "  A. THE FINITE AXIOMATIC TRIPLES ARE ASSERTED, AS PREMISES — a SPEC-CONFORMANCE",
    "     FIX, and the one that motivated this change. RDF 1.2 Semantics §8 (table `RDF",
    "     axioms`, 9 triples) and §9 (table `RDFS axiomatic triples`, 40) are seeded into",
    "     the fact store beside the input, because `S RDFS entails E` is defined over the",
    "     interpretations satisfying S AND those axioms. They are premises, so they are",
    "     NOT in the closure; everything they license is. Above all `rdfs:subClassOf` has",
    "     an axiomatic domain and range of `rdfs:Class`, so a subClassOf edge now types",
    "     BOTH its endpoints and `c rdfs:subClassOf c` comes back — from the premise the",
    "     specification names, not from the endpoint shortcut the engine swap removed.",
    "     Three W3C SPARQL entailment cases (rdfs05, rdfs11, paper-sparqldl-Q1-rdfs) were",
    "     failing for exactly the missing half of that path and now pass.",
    "",
    "  B. FIVE MORE RULES FIRE IN THE RDFS LANE: `rdfD2` (RDFS entailment subsumes RDF",
    "     entailment, and `rules(Rdfs)` always said so), `rdfs1`, `rdfs4`, `rdfs12` and",
    "     `rdfs13`. `implemented(Rdfs)` is 14 of 18 where it was 9, and the four still",
    "     missing — `rdfD1`, `rdfD1a`, `rdfs14`, `rdfs14a` — are missing for one reason:",
    "     each concludes about a FRESH blank node, which is an existential head this",
    "     crate's Datalog evaluator refuses.",
    "",
    "  C. EVERY RDFS CLOSURE CARRIES THE SAME 113-LINE VOCABULARY SATURATION. `rdfs4`",
    "     types every term of every triple an `rdfs:Resource`, and the axioms are triples,",
    "     so the axioms' own vocabulary saturates in every run. That block is INPUT-",
    "     INDEPENDENT: it is exactly `empty.golden`'s RDFS closure, it is a subset of every",
    "     other fixture's, and `the_rdfs_closure_of_every_fixture_contains_the_empty_one`",
    "     asserts so. Each golden below therefore accounts only for the lines BEYOND it.",
    "",
    "  D. THE BUDGET ROSE, AND IS STILL NEGLIGIBLE. An RDFS run now costs about 2,000",
    "     join steps and 165-210 stored facts against ceilings of 1,048,576 and 131,072 —",
    "     0.25% and 0.16% at this corpus's worst case (`subclass_chain`, 2,577 steps).",
];

/// The three causes of every byte that moved when the `OWL-RL` lane started stating the
/// whole of OWL 2 Profiles §4.3 Tables 5, 7 and 9.
///
/// Written into every golden's header beneath [`AXIOMATIC_PATH`], for the same reason the
/// other two are: the general cause once, the fixture's own triples in its own
/// [`Fixture::changed`].
///
/// The single strongest fact about this diff is stated first, because it makes the other
/// thirty-two goldens checkable at a glance: EVERY changed golden changed in the `OWL-RL`
/// section and in no other. `Simple`, `RDF` and `RDFS` are byte-identical across all
/// thirty-three files that already existed.
const OWL_RL_TABLES: &[&str] = &[
    "AND EVERY GOLDEN MOVED A THIRD TIME, IN THE OWL-RL SECTION AND NOWHERE ELSE — the",
    "OWL-RL lane now states the whole of OWL 2 Profiles §4.3 Tables 5, 7 and 9. Simple,",
    "RDF and RDFS are byte-identical to the previous golden in all thirty-three files that",
    "already existed, and NOTHING was removed from any closure anywhere. Three causes",
    "account for the new bytes.",
    "",
    "  i. TWENTY-FIVE MORE RULES FIRE. implemented(OWL-RL) is 37 of 78 where it was 12.",
    "     Table 5 gains prp-ap, prp-fp, prp-ifp, prp-spo2, prp-eqp1, prp-eqp2 and",
    "     prp-key; Table 7 gains cax-eqc1 and cax-eqc2; Table 9 gains scm-cls, scm-eqc2,",
    "     scm-op, scm-dp, scm-eqp2, scm-dom1, scm-dom2, scm-rng1, scm-rng2, scm-hv,",
    "     scm-svf1, scm-svf2, scm-avf1, scm-avf2, scm-int and scm-uni. Every OWL-RL",
    "     report's `missing` list therefore shrinks from 66 to 41, and the lane's contract",
    "     hash moves once — which is the point of the digest: a consumer holding a closure",
    "     minted under the twelve-rule calculus can tell.",
    "",
    " ii. EVERY OWL-RL CLOSURE CARRIES THE SAME NINE-LINE prp-ap BLOCK. prp-ap is",
    "     PREMISE-FREE — it types each built-in annotation property of OWL 2 RL an",
    "     owl:AnnotationProperty, and OWL 2 Structural Specification §5.5 fixes that list",
    "     at nine — so its conclusions are in every OWL-RL closure, including the empty",
    "     graph's. That block is INPUT-INDEPENDENT: it is exactly empty.golden's OWL-RL",
    "     closure, it is a subset of every other fixture's, and",
    "     `the_rdfs_closure_of_every_fixture_contains_the_empty_one` asserts both, by NAME",
    "     rather than by count, so a tenth typing would be an invented one and would fail.",
    "     Each golden below therefore accounts only for the lines BEYOND it, and for",
    "     twenty-eight of the thirty-three there are none.",
    "",
    "iii. EIGHT RULES ARE DECLARED AND NOT FIRED, WHICH IS WHY `missing` IS 41 AND NOT 33.",
    "     prp-irp, prp-asyp, prp-pdw, prp-adp, prp-npa1, prp-npa2, cax-dw and cax-adc all",
    "     conclude `false`. The calculus STATES each of them, with the specification's own",
    "     body — including the `i ≠ j` side condition of the two that read a list, which",
    "     is expressed as negation over an internal reflexive index relation rather than",
    "     dropped — and the semi-naive evaluator refuses the head form by name, because a",
    "     least-fixpoint evaluator over DEFINITE clauses has no semantics for `body →",
    "     false`. They stay in `missing` because they do not fire, which is exactly what",
    "     that list claims. Wiring a `false` head to a typed inconsistency witness is a",
    "     separate change; bending one into an atomic head to shorten this list would put",
    "     a triple in the closure that nothing licenses.",
    "",
    "  THE BUDGET ROSE IN THE OWL-RL LANE AND IS STILL NEGLIGIBLE. That lane's worst case",
    "  in this corpus went from 151 to 209 join steps (subclass_chain) against a ceiling of",
    "  1,048,576 — 0.02%. The corpus worst case is unchanged at 2,577 join steps and 208",
    "  stored facts, both in the RDFS lane, which no rule here touches: 0.25% and 0.16%.",
    "",
    "  TWO RULES READ AN RDF COLLECTION AND ONE PRE-PASS SERVES THEM. scm-int and scm-uni",
    "  join an internal LIST(head, index, member) relation the OWL-RL lane materializes",
    "  once per run from rdf:first / rdf:rest; prp-spo2 and prp-key recurse over the",
    "  collection directly, into internal ternary relations. NONE of those ids is an IRI —",
    "  PurRDF mints no vocabulary — and none can reach a closure: every conclusion whose",
    "  predicate is internal is dropped before materialization and credited to nothing.",
    "  A malformed or cyclic collection an OWL axiom points at stops the run with a named",
    "  error rather than producing a closure over its well-formed prefix.",
];

/// The four causes of every byte that moved when the `OWL-RL` lane stopped stating three
/// of OWL 2 Profiles §4.3's six tables and started stating all six.
///
/// Written into every golden's header beneath [`OWL_RL_TABLES`], for the same reason the
/// other three are: the general cause once, the fixture's own triples in its own
/// [`Fixture::changed`].
///
/// The single strongest fact about this diff is stated first, because it makes the other
/// fifty-nine goldens checkable at a glance: EVERY already-committed golden changed in the
/// `OWL-RL` section, gained a `D` section, and moved in no other way. `Simple`, `RDF` and
/// `RDFS` are byte-identical across all sixty files that already existed.
/// What moved when the four EXISTENTIAL patterns were wired, stated once for every golden.
///
/// It is a shared block rather than a per-fixture note because the cause is shared: no
/// fixture's own data changed, and the same four rules joined the same two lanes for every
/// one of them.
const EXISTENTIAL_CHASE: &[&str] = &[
    "AND EVERY GOLDEN MOVED A FIFTH TIME — the RDF and RDFS lanes now state the four",
    "patterns whose conclusions are EXISTENTIALLY QUANTIFIED, and every lane's",
    "contract-hash moved. Five causes account for every byte, and only one of them adds a",
    "line to any closure.",
    "",
    "  I. rdfD1, rdfD1a, rdfs14 AND rdfs14a NOW FIRE. Each concludes about a FRESH blank",
    "     node — the surrogate RDF 1.2 Semantics writes `_:nnn` — which is an existential",
    "     head a least-fixpoint evaluator over definite clauses has no semantics for. Those",
    "     two lanes are therefore evaluated by purrdf-datalog's RESTRICTED CHASE instead:",
    "     each surrogate is a frontier-addressed Skolem witness, so re-deriving an",
    "     obligation recovers the same witness and the fixpoint converges, and the clause",
    "     set's termination is COMPUTED — constant-refined weak acyclicity over the position",
    "     dependency graph — rather than assumed or asked for. implemented(RDF) is 3 of 3",
    "     and implemented(RDFS) 18 of 18 where they were 1 and 14, so both `missing` lists",
    "     are empty and both completeness lines read exact-within-boundaries.",
    "",
    " II. THE SURROGATES DO NOT REACH THE ANSWER, AND THAT IS REQUIRED. A SPARQL entailment",
    "     regime draws its answers from the SCOPING GRAPH, and a surrogate is not in it, so",
    "     every conclusion mentioning one is dropped at the materialization boundary and",
    "     counted. The W3C case rdfs13 is the proof: it asks `?L rdf:type rdfs:Literal` over",
    "     a graph whose only literal is \"foo\" and demands ZERO rows, which rdfD1's surrogate",
    "     would otherwise supply through rdfs1, rdfs13 and rdfs9. Every RDF and RDFS report",
    "     therefore gains the `surrogate` boundary. Nothing surrogate-FREE is lost: replacing",
    "     a term by a fresh blank node only weakens a triple, so every conclusion that does",
    "     not mention a surrogate was already licensed by the triple it stands for.",
    "",
    "III. ONE LINE IS NEW IN EVERY RDF CLOSURE, AND IT IS A GENUINE ENTAILMENT. rdfD1a is",
    "     premise-free — `_:nnn rdf:type ddd` holds for any graph, even the empty one — so",
    "     the closure really does use rdf:type as a predicate and rdfD2 types it an",
    "     rdf:Property. `rdf:type rdf:type rdf:Property` therefore appears in every RDF",
    "     closure that did not already hold it, INCLUDING the empty graph's. The RDFS lane",
    "     already held that line through the axiomatic triples, so no RDFS closure gains a",
    "     line at all: every RDFS conclusion the surrogates license mentions one.",
    "",
    " IV. THE BUDGET IS THE CHASE'S OWN MEASUREMENT. The RDF and RDFS lanes' join-steps,",
    "     stored-facts and term-arena-bytes are now purrdf-datalog's ChaseOutcome budget",
    "     rather than its Evaluation budget — the same three coordinates under the same three",
    "     definitions, counted by the engine that did the work. The chase is a naive fixpoint",
    "     that re-derives against the whole store each round rather than a semi-naive one, so",
    "     join-steps rises; stored-facts rises by the surrogate facts, which are stored even",
    "     though they are withheld. Both stay far below their ceilings.",
    "",
    "  V. EVERY CONTRACT HASH MOVED, INCLUDING OWL-RL'S AND D'S, AND NO OWL 2 RL RULE",
    "     CHANGED. A rule that concludes `false` is lowered into a clause whose head names a",
    "     clash marker built from the rule's DECLARATION INDEX, and declaring rdfD1 and",
    "     rdfD1a ahead of rdfD2 — where RDF 1.2 Semantics §8.1.1 puts them — renumbers every",
    "     rule after them. The digest is allowed to be conservative in exactly this",
    "     direction: refusing a cached closure that could have been kept is a cost, trusting",
    "     one minted under a different rule set is a defect.",
];

const OWL_RL_COMPLETE: &[&str] = &[
    "AND EVERY GOLDEN MOVED A FOURTH TIME, IN THE OWL-RL SECTION AND NOWHERE ELSE, AND",
    "GAINED A D SECTION — the OWL-RL lane now states OWL 2 Profiles §4.3 Tables 4, 6 and 8",
    "as well, which is the whole of the rule set. Simple, RDF and RDFS are byte-identical to",
    "the previous golden in all sixty files that already existed, and nothing was removed",
    "from any closure anywhere. Four causes account for the new bytes.",
    "",
    "  i. FORTY-ONE MORE RULES FIRE, AND THE LANE IS COMPLETE. implemented(OWL-RL) is 78 of",
    "     78 where it was 37: Table 4's nine eq-*, Table 6's nineteen cls-* and Table 8's",
    "     five dt-* are now stated and evaluated. Every OWL-RL report's `missing` list is",
    "     therefore EMPTY where it held 41 ids, and its completeness reads",
    "     `exact-within-boundaries` where it read `sound-incomplete`. Those are different",
    "     claims and the golden now spells them differently: `exact` means the rule table",
    "     was complete AND nothing got in the way, and no OWL-RL run reaches it, because",
    "     the datatype-value-space boundary is inherent to the lane.",
    "",
    "     SEVENTEEN OF THE FORTY-ONE CONCLUDE `false` AND ARE NOW EVALUATED RATHER THAN",
    "     DECLARED. A body match on eq-diff1, eq-diff2, eq-diff3, prp-irp, prp-asyp,",
    "     prp-pdw, prp-adp, prp-npa1, prp-npa2, cls-nothing2, cls-com, cls-maxc1,",
    "     cls-maxqc1, cls-maxqc2, cax-dw, cax-adc or dt-not-type makes `materialize` REFUSE",
    "     the run, carrying an inconsistency witness that names the rule and the asserted",
    "     triples that satisfied it. There is no closure for such a run, so no golden can",
    "     hold one: the eighteen fixtures that reach those rules — plus dt-diff, whose",
    "     witness is eq-diff1's — live in the oracle's CLASH_CORPUS with their controls",
    "     beside them, and the evidence is the refusal.",
    "",
    " ii. eq-ref ASSERTS `?x owl:sameAs ?x` FOR EVERY TERM OF EVERY TRIPLE, so every OWL-RL",
    "     closure grows by exactly one reflexive assertion per distinct term of that closure",
    "     — which is the single largest source of new lines in this diff and the reason the",
    "     smallest closures here are now three-figure. Each fixture's accounting below names",
    "     the terms ITS closure gained, and nothing else it gained.",
    "",
    "     A reflexive assertion whose subject is a LITERAL or a TRIPLE TERM is generalized",
    "     RDF, which the RDF 1.2 dataset IR cannot represent, so it is derived in the",
    "     evaluator's own term space and abandoned at the materialization boundary. That is",
    "     why `triple_term` is the one already-committed golden whose BOUNDARY list moved:",
    "     eq-ref reaches `<<( A ⊑ B )>> owl:sameAs <<( A ⊑ B )>>` and cannot represent it,",
    "     so that run now reports generalized-rdf beside the triple-term boundary it always",
    "     reported. Every other lane and fixture keeps the boundary list it had.",
    "",
    "iii. THREE MORE RULES ARE PREMISE-FREE, AND EVERY OWL-RL CLOSURE CARRIES THE SAME",
    "     98-LINE BLOCK. cls-thing and cls-nothing1 type owl:Thing and owl:Nothing an",
    "     owl:Class; dt-type1 types each of the thirty-two datatypes OWL 2 Profiles §4.2.1",
    "     lists as supported in OWL 2 RL an rdfs:Datatype; scm-cls draws five distinct",
    "     triples from the two classes; and eq-ref closes over all of it, one reflexive",
    "     assertion for each of the fifty distinct terms. 43 + 5 + 50 = 98, that block is",
    "     exactly empty.golden's OWL-RL closure, it is INPUT-INDEPENDENT and therefore a",
    "     subset of every other OWL-RL closure in the corpus, and",
    "     `the_rdfs_closure_of_every_fixture_contains_the_empty_one` asserts every layer of",
    "     it — the two specification lists by NAME, the other two as DERIVATIONS. Each",
    "     golden below accounts only for the lines BEYOND it. It replaces the nine-line",
    "     prp-ap block the previous change's cause ii describes, which is a subset of it.",
    "",
    " iv. THE OWL-RL LANE'S CONTRACT HASH MOVED ONCE, DELIBERATELY, AND THE D LANE HAS A",
    "     CALCULUS WHERE IT HAD NONE. The hash is the digest of the evaluated program, so a",
    "     consumer holding a closure minted under the 37-rule calculus can tell — which is",
    "     the whole point of carrying it. And `entailment/D` IS datatype entailment,",
    "     realized here as Simple entailment plus Table 8, so `materialize` now runs a lane",
    "     it used to refuse: every golden gains a fifth section for it. A D closure is the",
    "     input plus dt-type1's thirty-two typings and nothing else — dt-type2, dt-eq and",
    "     dt-diff conclude only triples with a literal subject, and that lane has no rule",
    "     that could consume one — which is a small answer, honestly reported, rather than",
    "     an unrun regime.",
    "",
    "  THE BUDGET ROSE IN THE OWL-RL LANE AND IS STILL NEGLIGIBLE. That lane's worst case in",
    "  this corpus went from 209 to 2,416 join steps (union_instance_near_miss) and from 30",
    "  to 170 stored facts (datatype_value_equality), against ceilings of 1,048,576 and",
    "  131,072 — 0.23% and 0.13%. The corpus worst case is still the RDFS lane's, unchanged",
    "  at 2,577 join steps and 208 stored facts in subclass_chain: 0.25% and 0.16%.",
];

/// Why every golden gained a line, and why thirty-six goldens exist that did not.
///
/// Written into every golden's header for the same reason as the five blocks above it: a
/// reader of one file should be able to see why its bytes moved without reading the commit.
const REPORT_SURFACE: &[&str] = &[
    "AND EVERY GOLDEN MOVED A SIXTH TIME — the REPORT surface changed, and no closure did.",
    "Every closure in this corpus is byte-identical to the previous golden. Two causes.",
    "",
    "  a. EVERY REPORT GAINED A `withheld-surrogates` LINE. rdfD1, rdfD1a, rdfs14 and",
    "     rdfs14a fire — cause I of the previous change is what made them — and every",
    "     conclusion they reach mentions a surrogate blank node, which a SPARQL entailment",
    "     regime may not answer with. So none of the four can EVER appear in `rules-fired`,",
    "     which counts triples that entered the closure, and until now the goldens showed",
    "     nothing at all about them: a reader could not tell the RDF and RDFS lanes, which",
    "     fire all four, from Simple, OWL-RL and D, which state none of them. The count is",
    "     the one observable trace they leave, so it is now rendered. It is a MEASUREMENT of",
    "     the run — non-zero exactly where a lane that mints surrogates met a term that",
    "     obliges one, zero elsewhere — and it is what raises the `surrogate` boundary, so",
    "     the two lines agree in every golden below.",
    "",
    "  b. CLASH_CORPUS HAS GOLDENS, WHERE IT HAD NONE. A refused run used to hand back an",
    "     inconsistency witness and nothing else, so those thirty-six fixtures had no report",
    "     to write a golden from and `inconsistency:` was a line no input in this corpus",
    "     could move off `none`. EntailError::Inconsistent now carries an InconsistentRun —",
    "     the witness AND the run's ReasoningReport — because the caller whose data is",
    "     inconsistent is exactly the caller who needs to know which rules had already fired,",
    "     what the evaluation cost and which contract hash refused. Their goldens show four",
    "     regimes closing normally and the refusing one rendering `--- refused:",
    "     inconsistent ---`, the witness's premises, and a report whose `inconsistency:`",
    "     names the rule.",
];

// ── The corpus ──────────────────────────────────────────────────────────────────
//
// Every fixture is minimal on purpose: the smallest input that reaches the rule, so a
// golden diff names one thing. Near-miss fixtures differ from their positive in exactly
// one term — the one the rule's premise binds — so "the rule did not fire" is attributable.

/// Every fixture, in the order the goldens are written and compared.
const CORPUS: &[Fixture] = &[
    Fixture {
        name: "empty",
        doc: &[
            "The empty dataset. Nothing fires, but the two INHERENT boundaries (the",
            "infinite axiomatic-triple schemas and the datatype value spaces) still hold",
            "for the lanes that meet them, so this pins that a boundary list is a property",
            "of the lane and not of the data.",
        ],
        exercises: &[],
        changed: &[
            "The CLOSURE does not move: nothing fires on nothing, in any of the four regimes.",
            "Cause 3 only — RDFS and OWL-RL report term-arena-bytes=0 where they reported 594.",
            "The chase pre-interned its thirteen vocabulary constants before looking at the",
            "data; a store interns a term when a term enters it, and none does here.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 0 -> 113 lines.",
            "THE 113 LINES ARE THE WHOLE OF CAUSE C, AND THIS GOLDEN IS WHERE THEY ARE PINNED.",
            "Nothing here is about data — there is none. The closure is what the axiomatic",
            "triples entail about the RDF and RDFS vocabulary itself: rdfs4 types all 32",
            "vocabulary terms rdfs:Resource, rdfs2/rdfs3 type the axioms' subjects and objects",
            "(13 + 8), rdfD2 types the three axiom predicates rdf:Property, rdfs1 types the",
            "three mandatory datatypes rdfs:Datatype, rdfs13 makes each a sub-class of",
            "rdfs:Literal, and rdfs6/rdfs8/rdfs10 close the reflexives. It is a fixpoint of the",
            "vocabulary and cannot depend on the input, which is what makes it a subset of",
            "every other RDFS closure in the corpus.",
            "The two INHERENT boundaries still hold and the report still says sound-incomplete,",
            "so this fixture still pins what it was written to pin.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 0 -> 9 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 9 -> 98 lines, and this golden is where all 98 of them are pinned: it",
            "IS the input-independent premise-free block cause iii describes, and nothing here is",
            "about data because there is none. cls-thing and cls-nothing1 type owl:Thing and",
            "owl:Nothing an owl:Class, dt-type1 types the thirty-two supported datatypes",
            "rdfs:Datatype, prp-ap's nine typings are carried through from the previous change,",
            "scm-cls draws five triples from the two classes, and eq-ref closes over all fifty",
            "distinct terms. 43 + 5 + 50 = 98.",
            "",
            "The tally gains eq-ref=50 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 0 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[],
    },
    Fixture {
        name: "plain_triple",
        doc: &[
            "One triple with no schema at all. Under RDF this is the whole of rdfD2: the",
            "predicate is typed rdf:Property. Under Simple it is the identity closure.",
        ],
        exercises: &["rdfD2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose `p rdfs:subPropertyOf p` and",
            "`rdfs:subPropertyOf rdfs:subPropertyOf rdfs:subPropertyOf` (rdfs6 2 -> 0). Nothing",
            "in this input is typed rdf:Property, so rdfs6 has no premise: the RDFS closure is",
            "now the input alone, which is what one untyped triple entails under RDFS.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "`x p y` puts three terms in the graph, so rdfs4 types all three, rdfD2 types p an",
            "rdf:Property, and rdfs6 draws the reflexive sub-property from THAT premise — the",
            "triple the engine swap removed as unlicensed is back, now with `p rdf:type",
            "rdf:Property` present in the same closure to license it.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 1 -> 10 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 10 -> 102 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 4 about this fixture's own terms.",
            "All 1 lines that were there before are unchanged.",
            "What is new are 3 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:x ex:y",
            "",
            "The tally gains eq-ref=53 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "named_graph",
        doc: &[
            "AWKWARD CASE — a quad outside the default graph. RDF has no standard entailment",
            "relation for a DATASET, so PurRDF states one: the default graph is closed",
            "against itself, each named graph against the union of itself and the default",
            "graph, and a conclusion lands in the graph that PRODUCED it. `x p y` in graph g",
            "is therefore a premise of g's own closure, and everything it licenses appears IN",
            "g; the named-graph boundary is what says this is a defined choice rather than a",
            "derived one. This is also rdfD2's near-miss, and a sharper one than it was: p is",
            "typed rdf:Property in g and NOT in the default graph, so the absence of the",
            "default-graph line is evidence that the routing works rather than that the rule",
            "did not fire.",
        ],
        exercises: &["rdfD2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose four triples (rdfs6 2 -> 0, rdfs10 2 -> 0):",
            "`A rdfs:subClassOf A` and `B rdfs:subClassOf B` fired on the ENDPOINTS of the",
            "input's subClassOf edge rather than on rdfs:Class instances, and",
            "`rdfs:subClassOf rdfs:subPropertyOf rdfs:subClassOf` /",
            "`rdfs:subPropertyOf rdfs:subPropertyOf rdfs:subPropertyOf` fired on predicates",
            "rather than on rdf:Property instances.",
            "What this fixture is FOR is untouched: the named-graph quad is still carried",
            "through unchanged, still supplies no premise, and the boundary is still reported.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 123 lines: the 113-line input-independent block cause C",
            "describes, plus 10 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:x ex:p ex:y ex:g",
            "",
            "What this fixture is FOR is untouched: the named-graph quad is still carried",
            "through, still supplies no premise, and the boundary is still reported — which",
            "is now a sharper claim, because rdfs4 would type x, p and y rdfs:Resource if",
            "the chase read that graph, and it does not. The default-graph subClassOf edge",
            "gets the full axiomatic treatment: both endpoints typed rdfs:Class and both",
            "reflexives drawn.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 102 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 4 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B",
            "",
            "AND THE ABSENCES ARE THE POINT, ON THIS FIXTURE MORE THAN ANY OTHER: x, p and y get",
            "NO reflexive assertion. They appear only in the named graph, the chase reads the",
            "default graph only, and eq-ref's premise is a triple of the graph it reads — so a",
            "rule that fires on literally every triple still does not reach them. A chase that",
            "had started reading named graphs would show up here as three extra lines.",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule. Its boundary list",
            "adds named-graph, which the input holds.",
            "",
            "AT THE DATASET SEMANTICS — THIS IS THE ONLY COMMITTED GOLDEN THAT MOVED, AND IT IS",
            "THE ONLY ONE THAT COULD HAVE: it is the corpus's only fixture with a quad outside",
            "the default graph, and nothing about a single-graph run changes. RDF has no",
            "standard entailment relation for a dataset — RDF 1.2 Semantics defines entailment",
            "over a GRAPH and SPARQL's regimes over the ACTIVE graph — so a reasoner handed one",
            "must choose, and PurRDF now states its choice instead of producing nothing: the",
            "default graph is closed against itself, each named graph against the union of",
            "itself and the default graph, and a conclusion lands in the graph that PRODUCED",
            "it. The named-graph boundary's reason is where that is written down, and it says",
            "so as a DEFINED behaviour rather than a derived one.",
            "",
            "  Simple is byte-identical: the identity closure was already faithful to every",
            "  graph, which is what made its `exact` honest and still does.",
            "",
            "  RDF closure 4 -> 5 lines. The one new line is",
            "",
            "    ex:p rdf:type rdf:Property ex:g",
            "",
            "  — rdfD2 over g's own seed. The tally goes rdfD2 2 -> 3. Note the GRAPH on that",
            "  line: `ex:p rdf:type rdf:Property` is still absent from the DEFAULT graph, which",
            "  is what keeps this fixture rdfD2's near-miss, and makes it a sharper one than it",
            "  was — the rule fired, and the conclusion went where it was produced.",
            "",
            "  RDFS closure 123 -> 128 lines, all five in ex:g and all five about the three",
            "  terms that appear only there —",
            "",
            "    ex:p rdf:type rdf:Property ex:g",
            "    ex:p rdf:type rdfs:Resource ex:g",
            "    ex:p rdfs:subPropertyOf ex:p ex:g",
            "    ex:x rdf:type rdfs:Resource ex:g",
            "    ex:y rdf:type rdfs:Resource ex:g",
            "",
            "  The tally gains rdfD2 4 -> 5, rdfs4 24 -> 27, rdfs6 17 -> 18. NOT ONE of the 113",
            "  input-independent vocabulary lines is restated in ex:g, and that is the",
            "  `conclusion lands in the graph that produced it` rule doing its work: g's run",
            "  re-derives every one of them — the default graph is in its seed — and every one",
            "  is already a default-graph conclusion, so none is emitted twice.",
            "",
            "  OWL-RL closure 102 -> 105 lines: three eq-ref reflexives in ex:g, for the three",
            "  terms the default graph does not hold —",
            "",
            "    ex:p ex:x ex:y",
            "",
            "  — and the tally gains eq-ref 52 -> 55. THE PREVIOUS SECTION PREDICTED EXACTLY",
            "  THIS. It ends `A chase that had started reading named graphs would show up here",
            "  as three extra lines`, and here are the three lines. That prediction was written",
            "  as a tripwire and it fired as designed; it is left standing rather than edited,",
            "  because this accounting is append-only and a reader who wants to know what the",
            "  answer used to be should be able to read it.",
            "",
            "  D closure is UNCHANGED at 34 lines, and that is the strongest single line of",
            "  evidence in this golden. The D lane is Simple entailment plus dt-type1, which is",
            "  premise-free: g's run draws the same thirty-two rdfs:Datatype typings the default",
            "  graph's run drew, every one of them is already a default-graph conclusion, and so",
            "  the named graph gains NOTHING. A per-graph closure that restated its seed's",
            "  conclusions would show up here as thirty-two duplicated lines.",
            "",
            "  THE BUDGET ROUGHLY DOUBLED, WHICH IS THE COST OF THE SEMANTICS AND IS REPORTED",
            "  RATHER THAN HIDDEN. One named graph is 1 + 1 = two evaluations of the same",
            "  declared program, so join-steps — which is WORK — sums: RDF 25 -> 58, RDFS",
            "  8,515 -> 17,291, OWL-RL 1,730 -> 3,562. stored-facts and term-arena-bytes are",
            "  OCCUPANCY of one store, each evaluation gets its own and drops it, and the",
            "  ceiling they are measured against is per-store — so they report the PEAK rather",
            "  than a sum that never existed at any instant: RDFS 182 -> 188 facts,",
            "  1,870 -> 1,936 bytes.",
            "",
            "  THE CORPUS WORST CASE MOVED, into a fixture this change ADDED rather than into",
            "  an existing one. `named_graph_closure_near_miss` holds TWO named graphs, so its",
            "  RDFS run is three evaluations and costs 24,970 join steps; the previous worst",
            "  case, `subclass_chain`'s single-graph RDFS run at 10,393, is untouched. Against",
            "  the fixed ceilings: 24,970 of 1,048,576 join steps is 2.4%. The corpus's largest",
            "  single STORE is still subclass_chain's 219 stored facts of 131,072 — 0.17% —",
            "  because occupancy is a peak and a third evaluation does not enlarge any one",
            "  store. NO CEILING MOVED, and none may: they are constants, measured against and",
            "  never raised.",
        ],
        quads: &[t_in(EX_X, EX_P, EX_Y, EX_G), t(EX_A, RDFS_SUBCLASSOF, EX_B)],
    },
    Fixture {
        name: "named_graph_closure",
        doc: &[
            "THE LAYOUT THE DATASET SEMANTICS EXISTS FOR — a terminology in the DEFAULT graph",
            "and instances in a NAMED graph, which is how essentially every real dataset is",
            "arranged and which used to produce nothing at all. `A rdfs:subClassOf B` sits in",
            "the default graph, `x rdf:type A` sits in ex:g, and each named graph is closed",
            "against the union of itself and the default graph — so rdfs9 / cax-sco has both",
            "premises and `x rdf:type B` is derived INTO ex:g, the graph that produced it.",
            "Its near-miss moves the terminology into a SECOND named graph, where the same two",
            "premises are in two graphs neither of which is in the other's seed.",
        ],
        exercises: &["rdfs9", "cax-sco"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
            t_in(EX_X, RDF_TYPE, EX_A, EX_G),
        ],
    },
    Fixture {
        name: "named_graph_closure_near_miss",
        doc: &[
            "THE CROSS-GRAPH JOIN THAT MUST NOT HAPPEN. Exactly `named_graph_closure` with one",
            "term changed — the terminology moves from the default graph into a second named",
            "graph ex:h — and that one change removes rdfs9's premise from ex:g's seed, because",
            "a named graph is closed against the union of itself and the DEFAULT graph and",
            "never against a sibling. `x rdf:type B` is therefore derived in NO graph: not in",
            "ex:g, which holds the instance and not the terminology; not in ex:h, which holds",
            "the terminology and not the instance; and not in the default graph, which holds",
            "neither. ex:h's own closure still draws everything the subClassOf edge licenses",
            "about A and B, so the absence is attributable to the missing JOIN rather than to a",
            "lane that stopped reasoning.",
        ],
        exercises: &["rdfs9", "cax-sco"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_in(EX_A, RDFS_SUBCLASSOF, EX_B, EX_H),
            t_in(EX_X, RDF_TYPE, EX_A, EX_G),
        ],
    },
    // ── rdf:reifies is ORDINARY DATA, enumerated position by position ────────────
    //
    // RDF 1.2's reifier property carries no entailment of its own: no rule of any lane in
    // this crate mentions `rdf:reifies`, and the only thing the specifications say about it
    // is three AXIOMATIC triples (`rdf:reifies rdf:type rdf:Property`, `rdfs:domain
    // rdfs:Resource`, `rdfs:range rdfs:Proposition`), which the RDFS lane seeds as premises
    // like every other axiom. Everything else follows from the rules a user's own triples
    // trigger — and nothing had ever pinned WHICH. These ten pairs do, one interaction each,
    // and every one of them reuses or mirrors an existing non-reifier fixture so the
    // comparison is `rdf:reifies` against `example.org/p` in the same position.
    Fixture {
        name: "reifies_subject_position",
        doc: &[
            "rdf:reifies in SUBJECT position. A user may type the reifier property like any",
            "other, and scm-op then draws the reflexive sub-property and equivalence for it.",
            "Its near miss is `object_property`, which is this fixture with the subject",
            "changed to example.org/p — so the conclusion is about the rule, not about the",
            "term being reserved vocabulary.",
        ],
        exercises: &["scm-op"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(RDF_REIFIES, RDF_TYPE, OWL_OBJECTPROPERTY)],
    },
    Fixture {
        name: "reifies_object_position",
        doc: &[
            "rdf:reifies in OBJECT position — named as an ordinary resource by an ordinary",
            "triple, whose property has a range. rdfs3 / prp-rng types it, exactly as it",
            "types example.org/y in `range`, which is this fixture with that object changed",
            "and is its near miss.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_P, RDFS_RANGE, EX_B), t(EX_X, EX_P, RDF_REIFIES)],
    },
    Fixture {
        name: "reifies_as_domain_class",
        doc: &[
            "rdf:reifies as the OBJECT OF rdfs:domain — that is, read as a CLASS. Nothing",
            "forbids it: rdfs:domain's range is rdfs:Class, a user may declare rdf:reifies",
            "one, and prp-dom / rdfs2 then types the subject of every p-triple with it. The",
            "near miss is `domain`, which is this fixture with the domain object changed to",
            "example.org/A.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_P, RDFS_DOMAIN, RDF_REIFIES), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "reifies_as_range_class",
        doc: &[
            "rdf:reifies as the OBJECT OF rdfs:range — the other half of the pair above, and",
            "the same reading: rdf:reifies is a class name here, and rdfs3 / prp-rng types",
            "the object of every p-triple with it. The near miss is `range`.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_P, RDFS_RANGE, RDF_REIFIES), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "reifies_domain",
        doc: &[
            "THE CASE THE TASK NAMES: a user puts rdfs:domain ON rdf:reifies, and an",
            "annotation triple then types its REIFIER. `ex:reifier rdf:reifies",
            "<<( A rdfs:subClassOf B )>>` is an ordinary triple, prp-dom / rdfs2 has both",
            "premises, and `ex:reifier rdf:type ex:A` follows.",
            "",
            "Note what is NOT concluded: the quoted `A rdfs:subClassOf B` is NOT asserted",
            "anywhere in this fixture, so nothing in any closure says A is a sub-class of B.",
            "A triple term is one opaque term to the chase — the triple-term boundary — and",
            "an annotation triple about it does not un-quote it.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(RDF_REIFIES, RDFS_DOMAIN, EX_A),
            t_quoted(EX_REIFIER, RDF_REIFIES, EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "reifies_domain_near_miss",
        doc: &[
            "`reifies_domain` with ONE term changed — the domain is declared on",
            "example.org/p instead of on rdf:reifies — so prp-dom / rdfs2 has no premise",
            "over the annotation triple and the reifier is not typed ex:A.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_P, RDFS_DOMAIN, EX_A),
            t_quoted(EX_REIFIER, RDF_REIFIES, EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "reifies_range",
        doc: &[
            "rdfs:range ON rdf:reifies. The object of an rdf:reifies triple is typed —",
            "and the object here is an IRI rather than a triple term ON PURPOSE, because",
            "prp-rng concludes into SUBJECT position and RDF 1.2 admits no triple term",
            "there. With a triple-term object the very same rule fires and its conclusion",
            "is abandoned as generalized RDF, which",
            "`a_reifier_range_conclusion_over_a_triple_term_is_abandoned_and_reported`",
            "asserts over `reifies_domain`, whose reifies object IS a triple term. Both",
            "halves are real behaviour and each is pinned where it is observable.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(RDF_REIFIES, RDFS_RANGE, EX_B),
            t(EX_REIFIER, RDF_REIFIES, EX_T),
        ],
    },
    Fixture {
        name: "reifies_range_near_miss",
        doc: &[
            "`reifies_range` with ONE term changed — the range is declared on example.org/p",
            "instead of on rdf:reifies — so the reifies object is not typed ex:B.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_P, RDFS_RANGE, EX_B), t(EX_REIFIER, RDF_REIFIES, EX_T)],
    },
    Fixture {
        name: "reifies_subproperty",
        doc: &[
            "rdfs:subPropertyOf ON rdf:reifies. prp-spo1 / rdfs7 rewrites the annotation",
            "triple's PREDICATE and copies its object through unchanged — and that object is",
            "a TRIPLE TERM, so the conclusion `ex:reifier ex:q <<( A rdfs:subClassOf B )>>`",
            "is the sharpest available statement that a rule applied to a reifier neither",
            "looks inside the quoted triple nor folds it into something representable.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(RDF_REIFIES, RDFS_SUBPROPERTYOF, EX_Q),
            t_quoted(EX_REIFIER, RDF_REIFIES, EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "reifies_subproperty_near_miss",
        doc: &[
            "`reifies_subproperty` with ONE term changed — the sub-property axiom is stated",
            "about example.org/p instead of rdf:reifies — so the annotation triple is not",
            "rewritten and no ex:q triple exists.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
            t_quoted(EX_REIFIER, RDF_REIFIES, EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "reifies_domain_widened",
        doc: &[
            "scm-dom1 over rdf:reifies: a domain declared on the reifier property is",
            "inherited by every super-class of that domain. Purely schema-level — there is",
            "no annotation triple here at all — which is what separates this from",
            "`reifies_domain`. Its near miss is `domain_widened`, this fixture with the",
            "domain's subject changed to example.org/r.",
        ],
        exercises: &["scm-dom1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(RDF_REIFIES, RDFS_DOMAIN, EX_A),
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "reifies_range_widened",
        doc: &[
            "scm-rng1 over rdf:reifies — the range half of the pair above. Its near miss is",
            "`range_widened`.",
        ],
        exercises: &["scm-rng1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(RDF_REIFIES, RDFS_RANGE, EX_A),
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "reifies_inside_triple_term",
        doc: &[
            "A REIFIER INSIDE A TRIPLE TERM. The quoted triple is itself an rdf:reifies",
            "triple, and the enclosing triple's property has a super-property, so prp-spo1 /",
            "rdfs7 rewrites the OUTER predicate and carries the whole quoted term through",
            "unchanged. Nothing looks inside it: the reifier does not become a subject of",
            "anything, the quoted rdf:reifies triple is not asserted, and no domain or range",
            "of rdf:reifies applies to it — a triple term is one term, and the triple-term",
            "boundary is what says so.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_SAYS, RDFS_SUBPROPERTYOF, EX_MENTIONS),
            t_quoted(EX_X, EX_SAYS, EX_REIFIER, RDF_REIFIES, EX_T),
        ],
    },
    Fixture {
        name: "reifies_inside_triple_term_near_miss",
        doc: &[
            "`reifies_inside_triple_term` with ONE term changed — the sub-property axiom is",
            "about example.org/p rather than example.org/says — so the outer triple is not",
            "rewritten and the quoted reifier term reaches no new triple.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_MENTIONS),
            t_quoted(EX_X, EX_SAYS, EX_REIFIER, RDF_REIFIES, EX_T),
        ],
    },
    Fixture {
        name: "domain",
        doc: &[
            "rdfs2 / prp-dom: a domain declaration types the subject of every triple with",
            "that predicate.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on the",
            "predicates p, rdf:type, rdfs:domain and rdfs:subPropertyOf (rdfs6 4 -> 0).",
            "rdfs2 / prp-dom is untouched: `x rdf:type A` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:domain ex:A",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "rdfs2 / prp-dom is untouched — `x rdf:type A` is still concluded and still credited.",
            "Around it, A is now an rdfs:Class (rdfs3 over the axiomatic range of rdfs:domain)",
            "and p an rdf:Property (rdfs2 over its axiomatic domain), so both reflexives follow.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 3 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:p ex:x ex:y rdfs:domain",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDFS_DOMAIN, EX_A), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "domain_near_miss",
        doc: &[
            "NEAR MISS for rdfs2 / prp-dom: the domain is declared on a DIFFERENT property",
            "(q, not p), so the data triple's subject is not typed.",
        ],
        exercises: &["rdfs2", "prp-dom"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on the",
            "predicates p, rdfs:domain and rdfs:subPropertyOf (rdfs6 3 -> 0). The near miss",
            "still holds: `x rdf:type A` is absent, because the domain is declared on q.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 127 lines: the 113-line input-independent block cause C",
            "describes, plus 14 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:domain ex:A",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `x rdf:type A` is absent, because the domain is declared",
            "on q. Note it is now a STRONGER near miss — q is typed rdf:Property and A an",
            "rdfs:Class exactly as in the positive, so the only difference left between the two",
            "closures is the one triple the rule is about.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 6 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:p ex:q ex:x ex:y rdfs:domain",
            "",
            "The tally gains eq-ref=56 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_Q, RDFS_DOMAIN, EX_A), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "range",
        doc: &[
            "rdfs3 / prp-rng: a range declaration types the object of every triple with",
            "that predicate.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on the",
            "predicates p, rdf:type, rdfs:range and rdfs:subPropertyOf (rdfs6 4 -> 0).",
            "rdfs3 / prp-rng is untouched: `y rdf:type B` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:range ex:B",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type ex:B",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "rdfs3 / prp-rng is untouched — `y rdf:type B` is still concluded and still credited.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 3 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:B ex:p ex:x ex:y rdfs:range",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDFS_RANGE, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "range_near_miss",
        doc: &[
            "NEAR MISS for rdfs3 / prp-rng: the range is declared on a DIFFERENT property",
            "(q, not p), so the data triple's object is not typed.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on the",
            "predicates p, rdfs:range and rdfs:subPropertyOf (rdfs6 3 -> 0). The near miss",
            "still holds: `y rdf:type B` is absent, because the range is declared on q.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 127 lines: the 113-line input-independent block cause C",
            "describes, plus 14 about this fixture's own terms —",
            "",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:range ex:B",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `y rdf:type B` is absent, because the range is on q.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 6 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:B ex:p ex:q ex:x ex:y rdfs:range",
            "",
            "The tally gains eq-ref=56 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_Q, RDFS_RANGE, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "subproperty_chain",
        doc: &["rdfs5 / scm-spo: rdfs:subPropertyOf is transitive."],
        exercises: &["rdfs5", "scm-spo"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose `p subPropertyOf p`, `q subPropertyOf q`,",
            "`r subPropertyOf r` and `rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf`",
            "(rdfs6 4 -> 0): p, q and r appear only as subPropertyOf ENDPOINTS here, and an",
            "endpoint is not an rdf:Property instance. rdfs5 / scm-spo is untouched — the",
            "transitive `p subPropertyOf r` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:p rdfs:subPropertyOf ex:r",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:q rdfs:subPropertyOf ex:r",
            "  ex:r rdf:type rdf:Property",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:r rdfs:subPropertyOf ex:r",
            "",
            "rdfs5 / scm-spo is untouched — the transitive `p subPropertyOf r` is still",
            "concluded and still credited (rdfs5=1). The three reflexives are back, but from",
            "the licensed premise this time: rdfs:subPropertyOf has an axiomatic domain AND",
            "range of rdf:Property, so p, q and r are all typed rdf:Property first.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 3 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:r rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
            t(EX_Q, RDFS_SUBPROPERTYOF, EX_R),
        ],
    },
    Fixture {
        name: "subproperty_chain_near_miss",
        doc: &[
            "NEAR MISS for rdfs5 / scm-spo: the chain is broken at the join point — the",
            "second edge starts at D rather than at q — so p is not a sub-property of r.",
        ],
        exercises: &["rdfs5", "scm-spo"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose the five reflexive subPropertyOf triples on D, p,",
            "q, r and rdfs:subPropertyOf (rdfs6 5 -> 0). The near miss still holds: the broken",
            "chain concludes nothing, and now the closure says exactly that.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 127 lines: the 113-line input-independent block cause C",
            "describes, plus 14 about this fixture's own terms —",
            "",
            "  ex:D rdf:type rdf:Property",
            "  ex:D rdf:type rdfs:Resource",
            "  ex:D rdfs:subPropertyOf ex:D",
            "  ex:D rdfs:subPropertyOf ex:r",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:r rdf:type rdf:Property",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:r rdfs:subPropertyOf ex:r",
            "",
            "The near miss still holds: the chain is broken at the join point, so `p",
            "subPropertyOf r` is absent while every reflexive is present.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:D ex:p ex:q ex:r rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
            t(EX_D, RDFS_SUBPROPERTYOF, EX_R),
        ],
    },
    Fixture {
        name: "subproperty_rewrite",
        doc: &[
            "rdfs7 / prp-spo1: a sub-property assertion re-predicates every triple that",
            "uses the sub-property.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose `p subPropertyOf p`, `q subPropertyOf q` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 3 -> 0).",
            "rdfs7 / prp-spo1 is untouched: `x q y` is still concluded and still credited.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 124 lines: the 113-line input-independent block cause C",
            "describes, plus 11 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:x ex:p ex:y",
            "  ex:x ex:q ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "rdfs7 / prp-spo1 is untouched: `x q y` is still concluded and still credited.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 3 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:x ex:y rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDFS_SUBPROPERTYOF, EX_Q), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "subproperty_rewrite_near_miss",
        doc: &[
            "NEAR MISS for rdfs7 / prp-spo1: the data triple uses r, which is not the",
            "declared sub-property, so nothing is re-predicated into q.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on p, q,",
            "r and rdfs:subPropertyOf (rdfs6 4 -> 0). The near miss still holds: `x q y` is",
            "absent, because the data triple uses r.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 126 lines: the 113-line input-independent block cause C",
            "describes, plus 13 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:r rdf:type rdf:Property",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:r rdfs:subPropertyOf ex:r",
            "  ex:x ex:r ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `x q y` is absent, because the data triple uses r.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 6 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:r ex:x ex:y rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=56 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDFS_SUBPROPERTYOF, EX_Q), t(EX_X, EX_R, EX_Y)],
    },
    Fixture {
        name: "property_typed",
        doc: &[
            "rdfs6: a resource typed rdf:Property is a sub-property of itself. p appears",
            "ONLY as the subject of that typing, never as a predicate, so the conclusion",
            "`p subPropertyOf p` is licensed by rdfs6 and by nothing else here.",
        ],
        exercises: &["rdfs6"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdf:type subPropertyOf rdf:type` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 3 -> 1). The one",
            "LICENSED conclusion stays: `p rdfs:subPropertyOf p`, from the premise",
            "`p rdf:type rdf:Property` this fixture asserts. That is the whole point — rdfs6",
            "was narrowed to its specification premise, not switched off.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 116 lines: the 113-line input-independent block cause C",
            "describes, plus 3 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "",
            "The smallest diff in the corpus, and the most direct: p is typed rdf:Property by",
            "the input itself, so rdfs6's premise never needed the axioms. Only rdfs4's",
            "rdfs:Resource typing is new.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, plus 1 about this fixture's own terms —",
            "",
            "  ex:p owl:equivalentProperty ex:p",
            "",
            "scm-eqp2 reads rdfs6's own conclusion `p rdfs:subPropertyOf p`, which holds in both",
            "directions trivially. rdfs6 is untouched, and scm-op does not fire: p is typed",
            "rdf:Property, not owl:ObjectProperty.",
            "The tally gains prp-ap=9 scm-eqp2=1.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p owl:equivalentProperty rdf:Property rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDF_TYPE, RDF_PROPERTY)],
    },
    Fixture {
        name: "property_typed_near_miss",
        doc: &[
            "NEAR MISS for rdfs6: the rdf:Property typing names q instead of p, and p is",
            "absent from the graph entirely, so `p subPropertyOf p` is not concluded.",
            "",
            "Note WHY the near miss removes p rather than merely un-typing it: the chase",
            "USED to fire the reflexive rule on every PREDICATE as well, so an un-typed p",
            "still standing in predicate position would have been re-concluded anyway.",
            "That is no longer so — `divergence_broad_triggers` is where the change is",
            "accounted for — but the fixture stays as it is: a near miss that would still",
            "hold under a broader rule is the stronger control, not the weaker one.",
        ],
        exercises: &["rdfs6"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdf:type subPropertyOf rdf:type` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 3 -> 1); the licensed",
            "`q rdfs:subPropertyOf q` stays. The near miss still holds: `p subPropertyOf p` is",
            "absent, and now it is absent because p is absent rather than in spite of it.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 116 lines: the 113-line input-independent block cause C",
            "describes, plus 3 about this fixture's own terms —",
            "",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "",
            "The near miss still holds: `p subPropertyOf p` is absent because p is absent.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, plus 1 about this fixture's own terms —",
            "",
            "  ex:q owl:equivalentProperty ex:q",
            "",
            "The same as `property_typed`, on q. The near miss still holds: `p subPropertyOf p`",
            "is absent because p is absent, and so now is `p owl:equivalentProperty p`.",
            "The tally gains prp-ap=9 scm-eqp2=1.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:q owl:equivalentProperty rdf:Property rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_Q, RDF_TYPE, RDF_PROPERTY)],
    },
    Fixture {
        name: "class_typed",
        doc: &[
            "rdfs8 and rdfs10: a resource typed rdfs:Class is a sub-class of rdfs:Resource",
            "and of itself.",
        ],
        exercises: &["rdfs8", "rdfs10"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdfs:Resource rdfs:subClassOf rdfs:Resource`",
            "(rdfs10 2 -> 1), which fired on the ENDPOINT of rdfs8's own conclusion and not on",
            "an rdfs:Class instance, and the three reflexive subPropertyOf triples on rdf:type,",
            "rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 3 -> 0). Both licensed conclusions",
            "stay: `C rdfs:subClassOf rdfs:Resource` (rdfs8) and `C rdfs:subClassOf C`",
            "(rdfs10, on the premise `C rdf:type rdfs:Class` this fixture asserts).",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 117 lines: the 113-line input-independent block cause C",
            "describes, plus 4 about this fixture's own terms —",
            "",
            "  ex:C rdf:type rdfs:Class",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:C rdfs:subClassOf ex:C",
            "  ex:C rdfs:subClassOf rdfs:Resource",
            "",
            "rdfs8 and rdfs10 are untouched: `C rdfs:subClassOf rdfs:Resource` and `C",
            "rdfs:subClassOf C` are still concluded from the input's own rdfs:Class typing.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 3 -> 13 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, plus 1 about this fixture's own terms —",
            "",
            "  ex:C owl:equivalentClass ex:C",
            "",
            "scm-eqc2 reads rdfs10's own conclusion: `C rdfs:subClassOf C` holds in both",
            "directions trivially, so the mutual-subclass premise is met and C is equivalent to",
            "itself. rdfs8 and rdfs10 are untouched — this fixture's own rules still fire, and",
            "scm-cls does NOT, which is what makes this the near miss for `owl_class`.",
            "The tally gains prp-ap=9 scm-eqc2=1.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 13 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 4 lines that were there before are unchanged.",
            "What is new are 3 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C rdfs:Class rdfs:Resource",
            "",
            "The tally gains eq-ref=53 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_C, RDF_TYPE, RDFS_CLASS)],
    },
    Fixture {
        name: "class_typed_near_miss",
        doc: &[
            "NEAR MISS for rdfs8 and rdfs10: C is typed, but not as rdfs:Class, and it is",
            "not a subClassOf endpoint either — so neither the rdfs:Resource conclusion nor",
            "the reflexive one is licensed.",
        ],
        exercises: &["rdfs8", "rdfs10"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `rdf:type subPropertyOf rdf:type` and",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` (rdfs6 2 -> 0). The near",
            "miss still holds: neither `C rdfs:subClassOf rdfs:Resource` nor",
            "`C rdfs:subClassOf C` is concluded, because C is not typed rdfs:Class.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:C rdf:type ex:NotAClass",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:NotAClass rdf:type rdfs:Class",
            "  ex:NotAClass rdf:type rdfs:Resource",
            "  ex:NotAClass rdfs:subClassOf ex:NotAClass",
            "  ex:NotAClass rdfs:subClassOf rdfs:Resource",
            "",
            "The near miss still holds for C: it is not typed rdfs:Class, so neither `C ⊑",
            "rdfs:Resource` nor `C ⊑ C` appears. NotAClass itself now IS an rdfs:Class — it",
            "is the object of an rdf:type triple, whose axiomatic range is rdfs:Class — which",
            "is a conclusion about NotAClass and not about C, and leaves the control intact.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 1 -> 10 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 10 -> 101 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 3 about this fixture's own terms.",
            "All 1 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C ex:NotAClass",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_C, RDF_TYPE, EX_NOT_A_CLASS)],
    },
    Fixture {
        name: "container_membership",
        doc: &[
            "rdfs12: a resource typed rdfs:ContainerMembershipProperty is a sub-property",
            "of rdfs:member. The typing is the graph's own — the axiomatic typings of",
            "rdf:_1, rdf:_2, … are members of the one family this chase cannot assert, so",
            "this is the only way the rule reaches a premise.",
        ],
        exercises: &["rdfs12"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 10 -> 101 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 3 about this fixture's own terms.",
            "All 1 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p rdfs:ContainerMembershipProperty",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDF_TYPE, RDFS_CONTAINERMEMBERSHIPPROPERTY)],
    },
    Fixture {
        name: "container_membership_near_miss",
        doc: &[
            "NEAR MISS for rdfs12: the container-membership typing names q instead of p,",
            "and p is absent from the graph entirely, so `p rdfs:subPropertyOf rdfs:member`",
            "is not concluded.",
        ],
        exercises: &["rdfs12"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 10 -> 101 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 3 about this fixture's own terms.",
            "All 1 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:q rdfs:ContainerMembershipProperty",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_Q, RDF_TYPE, RDFS_CONTAINERMEMBERSHIPPROPERTY)],
    },
    Fixture {
        name: "datatype_declared",
        doc: &[
            "rdfs13: a resource typed rdfs:Datatype is a sub-class of rdfs:Literal. The",
            "datatype is an example.org IRI rather than one of the three RDF 1.2 makes",
            "mandatory, so the conclusion is attributable to the graph's own typing and",
            "not to rdfs1's premise-free one.",
        ],
        exercises: &["rdfs13"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 10 -> 100 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 2 about this fixture's own terms.",
            "All 1 lines that were there before are unchanged.",
            "What is new is one reflexive `owl:sameAs` assertion. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:dt",
            "",
            "The tally gains eq-ref=51 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_DT, RDF_TYPE, RDFS_DATATYPE)],
    },
    Fixture {
        name: "datatype_declared_near_miss",
        doc: &[
            "NEAR MISS for rdfs13: the datatype IRI is typed, but not as rdfs:Datatype, so",
            "`dt rdfs:subClassOf rdfs:Literal` is not concluded.",
        ],
        exercises: &["rdfs13"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 10 -> 101 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 3 about this fixture's own terms.",
            "All 1 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:NotAClass ex:dt",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_DT, RDF_TYPE, EX_NOT_A_CLASS)],
    },
    Fixture {
        name: "subclass_instance",
        doc: &["rdfs9 / cax-sco: a sub-class assertion re-types an instance."],
        exercises: &["rdfs9", "cax-sco"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A` and `B rdfs:subClassOf B`",
            "(rdfs10 2 -> 0, fired on subClassOf endpoints) and the three reflexive",
            "subPropertyOf triples on rdf:type, rdfs:subClassOf and rdfs:subPropertyOf",
            "(rdfs6 3 -> 0). rdfs9 / cax-sco is untouched: `x rdf:type B` is still concluded.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 118 lines: the 113-line input-independent block cause C",
            "describes, plus 5 about this fixture:",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:ContainerMembershipProperty",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p rdfs:subPropertyOf rdfs:member",
            "",
            "NEW FIXTURE. rdfs12's only premise is a typing the graph asserts: the axiomatic",
            "typings of rdf:_1, rdf:_2, … are in the one family this chase cannot assert.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 118 lines: the 113-line input-independent block cause C",
            "describes, plus 5 about this fixture:",
            "",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:ContainerMembershipProperty",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:q rdfs:subPropertyOf rdfs:member",
            "",
            "NEW FIXTURE.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture:",
            "",
            "  ex:dt rdf:type rdfs:Class",
            "  ex:dt rdf:type rdfs:Datatype",
            "  ex:dt rdf:type rdfs:Resource",
            "  ex:dt rdfs:subClassOf ex:dt",
            "  ex:dt rdfs:subClassOf rdfs:Literal",
            "  ex:dt rdfs:subClassOf rdfs:Resource",
            "",
            "NEW FIXTURE. rdfs13 fires four times: once for ex:dt, and once each for the three",
            "datatypes rdfs1 types premise-free — which is why the positive uses an",
            "example.org IRI, so its conclusion is attributable to the graph's own typing.",
            "",
            "AT THE AXIOMATIC PATH — NEW FIXTURE, so nothing moved: this golden did not exist.",
            "Its RDFS closure is 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture:",
            "",
            "  ex:NotAClass rdf:type rdfs:Class",
            "  ex:NotAClass rdf:type rdfs:Resource",
            "  ex:NotAClass rdfs:subClassOf ex:NotAClass",
            "  ex:NotAClass rdfs:subClassOf rdfs:Resource",
            "  ex:dt rdf:type ex:NotAClass",
            "  ex:dt rdf:type rdfs:Resource",
            "",
            "NEW FIXTURE.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 125 lines: the 113-line input-independent block cause C",
            "describes, plus 12 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type ex:B",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "rdfs9 / cax-sco is untouched: `x rdf:type B` is still concluded.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 1 -> 10 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 1 -> 10 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 1 -> 10 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 1 -> 10 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 3 -> 12 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 104 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 6 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 3 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:x",
            "",
            "The tally gains eq-ref=53 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, RDF_TYPE, EX_A)],
    },
    Fixture {
        name: "subclass_instance_near_miss",
        doc: &[
            "NEAR MISS for rdfs9 / cax-sco: x is an instance of D, which is not the",
            "sub-class the axiom names, so it is not re-typed into B.",
        ],
        exercises: &["rdfs9", "cax-sco"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose the same five triples as `subclass_instance`:",
            "`A rdfs:subClassOf A`, `B rdfs:subClassOf B` (rdfs10 2 -> 0) and the reflexive",
            "subPropertyOf triples on rdf:type, rdfs:subClassOf and rdfs:subPropertyOf",
            "(rdfs6 3 -> 0). The near miss still holds: `x rdf:type B` is absent.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 128 lines: the 113-line input-independent block cause C",
            "describes, plus 15 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:D rdf:type rdfs:Class",
            "  ex:D rdf:type rdfs:Resource",
            "  ex:D rdfs:subClassOf ex:D",
            "  ex:D rdfs:subClassOf rdfs:Resource",
            "  ex:x rdf:type ex:D",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "The near miss still holds: `x rdf:type B` is absent.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 104 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 6 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:D ex:x",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, RDF_TYPE, EX_D)],
    },
    Fixture {
        name: "subclass_chain",
        doc: &[
            "AWKWARD CASE — a subClassOf chain deep enough that a single round cannot",
            "close it. A ⊑ B ⊑ C ⊑ D ⊑ E ⊑ F with x an A: the semi-naive frontier must",
            "carry derived edges into later rounds for `A ⊑ F` and `x a F` to appear, so",
            "this fixture is the fixpoint's own test as well as rdfs11's.",
        ],
        exercises: &["rdfs11", "scm-sco", "rdfs9", "cax-sco"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose nine triples, 30 lines down to 21: the six",
            "reflexive `Ci rdfs:subClassOf Ci` for A, B, C, D, E and F (rdfs10 6 -> 0, all six",
            "fired on subClassOf endpoints) and the three reflexive subPropertyOf triples on",
            "rdf:type, rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 3 -> 0).",
            "What this fixture is FOR is untouched: rdfs11 / scm-sco still contributes 10",
            "triples and rdfs9 / cax-sco still contributes 5, so `A rdfs:subClassOf F` and",
            "`x rdf:type F` are still reached — the multi-round fixpoint still closes.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 21 -> 159 lines: the 113-line input-independent block cause C",
            "describes, plus 46 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf ex:C",
            "  ex:A rdfs:subClassOf ex:D",
            "  ex:A rdfs:subClassOf ex:E",
            "  ex:A rdfs:subClassOf ex:F",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf ex:C",
            "  ex:B rdfs:subClassOf ex:D",
            "  ex:B rdfs:subClassOf ex:E",
            "  ex:B rdfs:subClassOf ex:F",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:C rdf:type rdfs:Class",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:C rdfs:subClassOf ex:C",
            "  ex:C rdfs:subClassOf ex:D",
            "  ex:C rdfs:subClassOf ex:E",
            "  ex:C rdfs:subClassOf ex:F",
            "  ex:C rdfs:subClassOf rdfs:Resource",
            "  ex:D rdf:type rdfs:Class",
            "  ex:D rdf:type rdfs:Resource",
            "  ex:D rdfs:subClassOf ex:D",
            "  ex:D rdfs:subClassOf ex:E",
            "  ex:D rdfs:subClassOf ex:F",
            "  ex:D rdfs:subClassOf rdfs:Resource",
            "  ex:E rdf:type rdfs:Class",
            "  ex:E rdf:type rdfs:Resource",
            "  ex:E rdfs:subClassOf ex:E",
            "  ex:E rdfs:subClassOf ex:F",
            "  ex:E rdfs:subClassOf rdfs:Resource",
            "  ex:F rdf:type rdfs:Class",
            "  ex:F rdf:type rdfs:Resource",
            "  ex:F rdfs:subClassOf ex:F",
            "  ex:F rdfs:subClassOf rdfs:Resource",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type ex:B",
            "  ex:x rdf:type ex:C",
            "  ex:x rdf:type ex:D",
            "  ex:x rdf:type ex:E",
            "  ex:x rdf:type ex:F",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "The largest closure in the corpus, and what it is FOR is untouched: rdfs11 /",
            "scm-sco still contributes 10 triples, rdfs9 still reaches `x rdf:type F`, and the",
            "multi-round fixpoint still closes. The six reflexive `Ci ⊑ Ci` are back — every",
            "Ci is now typed rdfs:Class from the axiomatic domain and range of rdfs:subClassOf",
            "— and this is the corpus's budget worst case at 2,577 join steps.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 21 -> 30 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 30 -> 126 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 28 about this fixture's own terms.",
            "All 21 lines that were there before are unchanged.",
            "What is new are 7 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:C ex:D ex:E ex:F ex:x",
            "",
            "The tally gains eq-ref=57 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 6 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_B, RDFS_SUBCLASSOF, EX_C),
            t(EX_C, RDFS_SUBCLASSOF, EX_D),
            t(EX_D, RDFS_SUBCLASSOF, EX_E),
            t(EX_E, RDFS_SUBCLASSOF, EX_F),
            t(EX_X, RDF_TYPE, EX_A),
        ],
    },
    Fixture {
        name: "subclass_chain_near_miss",
        doc: &[
            "NEAR MISS for rdfs11 / scm-sco: the two edges do not meet — A ⊑ B and E ⊑ F",
            "share no endpoint — so `A ⊑ F` is not derivable at any depth.",
        ],
        exercises: &["rdfs11", "scm-sco"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A`, `B rdfs:subClassOf B`,",
            "`E rdfs:subClassOf E` and `F rdfs:subClassOf F` (rdfs10 4 -> 0) and the reflexive",
            "subPropertyOf triples on rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 2 -> 0).",
            "The near miss still holds: `A rdfs:subClassOf F` is absent at every depth.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 131 lines: the 113-line input-independent block cause C",
            "describes, plus 18 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:E rdf:type rdfs:Class",
            "  ex:E rdf:type rdfs:Resource",
            "  ex:E rdfs:subClassOf ex:E",
            "  ex:E rdfs:subClassOf ex:F",
            "  ex:E rdfs:subClassOf rdfs:Resource",
            "  ex:F rdf:type rdfs:Class",
            "  ex:F rdf:type rdfs:Resource",
            "  ex:F rdfs:subClassOf ex:F",
            "  ex:F rdfs:subClassOf rdfs:Resource",
            "",
            "The near miss still holds: `A rdfs:subClassOf F` is absent at every depth.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 104 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 6 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:E ex:F",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_E, RDFS_SUBCLASSOF, EX_F),
        ],
    },
    Fixture {
        name: "symmetric",
        doc: &[
            "prp-symp: a symmetric property mirrors its triples. Also the NEAR MISS for",
            "prp-trp — it differs from `transitive` in exactly one term, the property",
            "characteristic — so the two fixtures are each other's control.",
        ],
        exercises: &["prp-symp"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on p,",
            "rdf:type and rdfs:subPropertyOf (rdfs6 3 -> 0). Under RDFS the closure is now the",
            "input alone, which is correct: RDFS has no rule for owl:SymmetricProperty.",
            "prp-symp is untouched — OWL-RL still mirrors both triples.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 126 lines: the 113-line input-independent block cause C",
            "describes, plus 13 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdf:type owl:SymmetricProperty",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y ex:p ex:z",
            "  ex:y rdf:type rdfs:Resource",
            "  ex:z rdf:type rdfs:Resource",
            "  owl:SymmetricProperty rdf:type rdfs:Class",
            "  owl:SymmetricProperty rdf:type rdfs:Resource",
            "  owl:SymmetricProperty rdfs:subClassOf rdfs:Resource",
            "  owl:SymmetricProperty rdfs:subClassOf owl:SymmetricProperty",
            "",
            "Under RDFS the closure is no longer the input alone, but RDFS still has no rule",
            "for owl:SymmetricProperty: owl:SymmetricProperty appears only as an rdf:type",
            "object, so the axiomatic range of rdf:type makes it an rdfs:Class and nothing",
            "more is said about it. prp-symp is untouched — the OWL-RL section is identical.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 5 -> 14 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 14 -> 108 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 10 about this fixture's own terms.",
            "All 5 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:x ex:y ex:z owl:SymmetricProperty",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDF_TYPE, OWL_SYMMETRIC),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "transitive",
        doc: &[
            "prp-trp: a transitive property composes its triples. Also the NEAR MISS for",
            "prp-symp; see `symmetric`.",
        ],
        exercises: &["prp-trp"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on p,",
            "rdf:type and rdfs:subPropertyOf (rdfs6 3 -> 0). Under RDFS the closure is now the",
            "input alone, which is correct: RDFS has no rule for owl:TransitiveProperty.",
            "prp-trp is untouched — OWL-RL still composes `x p z`.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 126 lines: the 113-line input-independent block cause C",
            "describes, plus 13 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdf:type owl:TransitiveProperty",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y ex:p ex:z",
            "  ex:y rdf:type rdfs:Resource",
            "  ex:z rdf:type rdfs:Resource",
            "  owl:TransitiveProperty rdf:type rdfs:Class",
            "  owl:TransitiveProperty rdf:type rdfs:Resource",
            "  owl:TransitiveProperty rdfs:subClassOf rdfs:Resource",
            "  owl:TransitiveProperty rdfs:subClassOf owl:TransitiveProperty",
            "",
            "As `symmetric`: RDFS says nothing about owl:TransitiveProperty beyond its being",
            "the object of an rdf:type triple and therefore an rdfs:Class. prp-trp is",
            "untouched — the OWL-RL section is identical.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 4 -> 13 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 13 -> 107 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 9 about this fixture's own terms.",
            "All 4 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:x ex:y ex:z owl:TransitiveProperty",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDF_TYPE, OWL_TRANSITIVE),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "inverse_pair",
        doc: &[
            "AWKWARD CASE — one owl:inverseOf axiom exercised in BOTH directions. `x p y`",
            "drives prp-inv1 (mirroring a p-triple into q) and `u q v` drives prp-inv2",
            "(mirroring a q-triple into p) from the same axiom, so the split of the inverse",
            "index into its two halves is observable rather than merely asserted.",
        ],
        exercises: &["prp-inv1", "prp-inv2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 3 -> 4).",
            "Cause 1 — RDFS and OWL-RL lose the four reflexive subPropertyOf triples on p, q,",
            "rdfs:subPropertyOf and owl:inverseOf (rdfs6 4 -> 0). Both halves of the axiom are",
            "untouched: prp-inv1 still mirrors `x p y` into q and prp-inv2 still mirrors",
            "`u q v` into p, each still credited under its own id.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 129 lines: the 113-line input-independent block cause C",
            "describes, plus 16 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p owl:inverseOf ex:q",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:u ex:q ex:v",
            "  ex:u rdf:type rdfs:Resource",
            "  ex:v rdf:type rdfs:Resource",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "  owl:inverseOf rdf:type rdf:Property",
            "  owl:inverseOf rdf:type rdfs:Resource",
            "  owl:inverseOf rdfs:subPropertyOf owl:inverseOf",
            "",
            "Both halves of the axiom are untouched — the OWL-RL section is identical.",
            "owl:inverseOf is now typed rdf:Property because it is a PREDICATE (rdfD2), which",
            "is the one place this fixture shows rdfD2 reaching something the axioms do not.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 5 -> 14 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 14 -> 110 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 12 about this fixture's own terms.",
            "All 5 lines that were there before are unchanged.",
            "What is new are 7 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:u ex:v ex:x ex:y owl:inverseOf",
            "",
            "The tally gains eq-ref=57 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, OWL_INVERSEOF, EX_Q),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_V),
        ],
    },
    Fixture {
        name: "inverse_pair_near_miss",
        doc: &[
            "NEAR MISS for prp-inv1 and prp-inv2: the axiom names r as p's inverse, not q,",
            "so neither mirror between p and q is licensed.",
        ],
        exercises: &["prp-inv1", "prp-inv2"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 3 -> 4).",
            "Cause 1 — RDFS loses the four reflexive subPropertyOf triples on p, q,",
            "rdfs:subPropertyOf and owl:inverseOf (rdfs6 4 -> 0); OWL-RL loses those four and",
            "`r subPropertyOf r` as well (rdfs6 5 -> 0), r being a predicate only because",
            "prp-inv1's own conclusion uses it. The near miss still holds: neither mirror",
            "between p and q appears.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 3 -> 130 lines: the 113-line input-independent block cause C",
            "describes, plus 17 about this fixture's own terms —",
            "",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:p owl:inverseOf ex:r",
            "  ex:q rdf:type rdf:Property",
            "  ex:q rdf:type rdfs:Resource",
            "  ex:q rdfs:subPropertyOf ex:q",
            "  ex:r rdf:type rdfs:Resource",
            "  ex:u ex:q ex:v",
            "  ex:u rdf:type rdfs:Resource",
            "  ex:v rdf:type rdfs:Resource",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "  owl:inverseOf rdf:type rdf:Property",
            "  owl:inverseOf rdf:type rdfs:Resource",
            "  owl:inverseOf rdfs:subPropertyOf owl:inverseOf",
            "",
            "The near miss still holds: neither mirror between p and q appears.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 4 -> 13 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 13 -> 110 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 12 about this fixture's own terms.",
            "All 4 lines that were there before are unchanged.",
            "What is new are 8 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:r ex:u ex:v ex:x ex:y owl:inverseOf",
            "",
            "The tally gains eq-ref=58 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, OWL_INVERSEOF, EX_R),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_V),
        ],
    },
    Fixture {
        name: "equivalent_class",
        doc: &[
            "scm-eqc1: owl:equivalentClass is mutual rdfs:subClassOf. Also the NEAR MISS",
            "for scm-eqp1 — it differs from `equivalent_property` in exactly one term, the",
            "equivalence predicate.",
        ],
        exercises: &["scm-eqc1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS loses `rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` and",
            "`owl:equivalentClass subPropertyOf owl:equivalentClass` (rdfs6 2 -> 0), leaving the",
            "input alone, which is correct: RDFS has no rule for owl:equivalentClass. OWL-RL",
            "loses those two and `rdfs:subClassOf subPropertyOf rdfs:subClassOf` (rdfs6 3 -> 0).",
            "`A rdfs:subClassOf A` and `B rdfs:subClassOf B` STAY, and are not reflexive-rule",
            "survivors: scm-eqc1 gives both `A subClassOf B` and `B subClassOf A`, and",
            "rdfs11 / scm-sco composes each pair — the tally still reads scm-sco=2.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A owl:equivalentClass ex:B",
            "  ex:B rdf:type rdfs:Resource",
            "  owl:equivalentClass rdf:type rdf:Property",
            "  owl:equivalentClass rdf:type rdfs:Resource",
            "  owl:equivalentClass rdfs:subPropertyOf owl:equivalentClass",
            "",
            "RDFS still has no rule for owl:equivalentClass — it is typed rdf:Property because",
            "it is a predicate, and that is all. `A ⊑ A` and `B ⊑ B` are NOT here: nothing",
            "types A or B an rdfs:Class under RDFS, since owl:equivalentClass has no",
            "axiomatic range. The OWL-RL section, where scm-eqc1 does license them, is",
            "identical to before.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 5 -> 17 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, plus 3 about this fixture's own terms —",
            "",
            "  ex:A owl:equivalentClass ex:A",
            "  ex:B owl:equivalentClass ex:A",
            "  ex:B owl:equivalentClass ex:B",
            "",
            "scm-eqc2 is the converse of scm-eqc1 and this fixture now exercises the round trip:",
            "scm-eqc1 turns `A owl:equivalentClass B` into both sub-class edges, rdfs11 / scm-sco",
            "composes each pair into `A ⊑ A` and `B ⊑ B`, and scm-eqc2 reads all three mutual",
            "pairs back as equivalences. The input's own `A owl:equivalentClass B` is not",
            "re-derived — it is a premise, not a conclusion — so the tally reads scm-eqc2=3 and",
            "not 4. scm-eqc1 and scm-sco are untouched at 2 each.",
            "The tally gains prp-ap=9 scm-eqc2=3.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 17 -> 108 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 10 about this fixture's own terms.",
            "All 8 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_A, OWL_EQUIVALENTCLASS, EX_B)],
    },
    Fixture {
        name: "equivalent_property",
        doc: &[
            "scm-eqp1: owl:equivalentProperty is mutual rdfs:subPropertyOf. Also the NEAR",
            "MISS for scm-eqc1; see `equivalent_class`.",
        ],
        exercises: &["scm-eqp1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 1 -> 2).",
            "Cause 1 — RDFS and OWL-RL lose",
            "`rdfs:subPropertyOf subPropertyOf rdfs:subPropertyOf` and",
            "`owl:equivalentProperty subPropertyOf owl:equivalentProperty` (rdfs6 2 -> 0). The",
            "RDFS closure is now the input alone, which is correct: RDFS has no rule for",
            "owl:equivalentProperty. `A subPropertyOf A` and `B subPropertyOf B` STAY under",
            "OWL-RL, licensed by rdfs5 / scm-spo over scm-eqp1's two edges (scm-spo=2), not by",
            "the reflexive rule.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 1 -> 119 lines: the 113-line input-independent block cause C",
            "describes, plus 6 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A owl:equivalentProperty ex:B",
            "  ex:B rdf:type rdfs:Resource",
            "  owl:equivalentProperty rdf:type rdf:Property",
            "  owl:equivalentProperty rdf:type rdfs:Resource",
            "  owl:equivalentProperty rdfs:subPropertyOf owl:equivalentProperty",
            "",
            "As `equivalent_class`, with owl:equivalentProperty. The OWL-RL section is",
            "identical.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 5 -> 17 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, plus 3 about this fixture's own terms —",
            "",
            "  ex:A owl:equivalentProperty ex:A",
            "  ex:B owl:equivalentProperty ex:A",
            "  ex:B owl:equivalentProperty ex:B",
            "",
            "The property mirror of `equivalent_class`: scm-eqp1 gives both sub-property edges,",
            "rdfs5 / scm-spo composes the reflexives, and scm-eqp2 reads all three mutual pairs",
            "back. scm-eqp1 and scm-spo are untouched at 2 each.",
            "The tally gains prp-ap=9 scm-eqp2=3.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 17 -> 110 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 12 about this fixture's own terms.",
            "All 8 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B owl:equivalentProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_A, OWL_EQUIVALENTPROPERTY, EX_B)],
    },
    Fixture {
        name: "shared_conclusion",
        doc: &[
            "AWKWARD CASE — two rules that both conclude the SAME triple. `x rdf:type C`",
            "follows from rdfs9 / cax-sco (x is an A, A ⊑ C) and independently from",
            "rdfs2 / prp-dom (p has domain C, and x is the subject of a p-triple). Exactly",
            "one of them is credited — whichever reached it first in the chase's firing",
            "order — and the golden's per-rule tally is where that choice is pinned. The",
            "count a report gives is 'triples this rule was FIRST to add', so a re-derived",
            "triple contributes to neither total.",
        ],
        exercises: &["rdfs9", "cax-sco", "rdfs2", "prp-dom"],
        changed: &[
            "Cause 2 does NOT apply: the input already uses rdf:type as a predicate, so the RDF",
            "closure is unchanged and only its budget moves.",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A` and `C rdfs:subClassOf C`",
            "(rdfs10 2 -> 0) and the five reflexive subPropertyOf triples on p, rdf:type,",
            "rdfs:domain, rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 5 -> 0).",
            "THE SHARED CONCLUSION ITSELF DID NOT MOVE. `x rdf:type C` is still concluded and",
            "is still credited to rdfs9 / cax-sco rather than to rdfs2 / prp-dom — but for a",
            "stated reason now rather than by firing order: the evaluator picks a round's",
            "winner by a total order over observable provenance, and rdfs9's sources",
            "(`A subClassOf C`, `x a A`) sort before rdfs2's (`p domain C`, `x p y`).",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 5 -> 131 lines: the 113-line input-independent block cause C",
            "describes, plus 18 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:C",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:C rdf:type rdfs:Class",
            "  ex:C rdf:type rdfs:Resource",
            "  ex:C rdfs:subClassOf ex:C",
            "  ex:C rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:domain ex:C",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type ex:A",
            "  ex:x rdf:type ex:C",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "THE SHARED CONCLUSION ITSELF DID NOT MOVE: `x rdf:type C` is still concluded and",
            "still credited to rdfs9 rather than to rdfs2, on the same provenance ordering as",
            "before. rdfs9's tally rose from 1 to 4 because the axioms give it three more",
            "conclusions (through `⊑ rdfs:Resource`), and the sum of the tally is still",
            "exactly the inferred-triple count, which is the invariant this fixture guards.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 5 -> 14 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 14 -> 109 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 11 about this fixture's own terms.",
            "All 5 lines that were there before are unchanged.",
            "What is new are 6 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:C ex:p ex:x ex:y rdfs:domain",
            "",
            "The tally gains eq-ref=56 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 4 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_C),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_P, RDFS_DOMAIN, EX_C),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "triple_term",
        doc: &[
            "AWKWARD CASE — an RDF 1.2 triple term in object position, under a",
            "sub-property axiom that forces a conclusion to be built AROUND it.",
            "",
            "rdfs14 and rdfs14a conclude about a FRESH blank node standing for the triple",
            "term, which is an existential head this crate's Datalog evaluator refuses, so",
            "neither fires and a triple-term boundary is reported; the chase therefore",
            "interns the term as one atomic term and never looks inside it. The",
            "second quad makes the harder thing happen: rdfs7 re-predicates",
            "`x says <<( A ⊑ B )>>` into a `mentions` triple, and the object of that",
            "conclusion has to be re-interned.",
            "",
            "AN EARLIER FIX THIS FIXTURE PINS. The engine used to emit",
            "`x mentions rdfs:Resource` for that conclusion: re-interning folded EVERY",
            "triple term to rdfs:Resource on the way back into the dataset builder, on the",
            "stated assumption that the RDFS/OWL-RL rules never derive one in that",
            "position. rdfs7 / prp-spo1 does, and the substitution was UNSOUND — nothing",
            "in this input entails `x mentions rdfs:Resource`. A triple term is now",
            "rebuilt structurally and recursively, so it re-materializes as itself.",
        ],
        exercises: &["rdfs7", "prp-spo1"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 3 -> 4).",
            "Cause 1 — RDFS and OWL-RL lose `A rdfs:subClassOf A` and `B rdfs:subClassOf B`",
            "(rdfs10 2 -> 0) and the four reflexive subPropertyOf triples on says, mentions,",
            "rdfs:subClassOf and rdfs:subPropertyOf (rdfs6 4 -> 0).",
            "The line this fixture exists for is untouched: rdfs7 / prp-spo1 still concludes",
            "`x mentions <<( A rdfs:subClassOf B )>>`, with the triple term carried through as",
            "itself. The engine now interns a triple term as one lexical surface rather than as",
            "one interner id, which is the same opacity by a different mechanism — rdfs14 and",
            "rdfs14a did not fire AT THAT POINT (they do now; see the fifth change above, which",
            "is where they started to, and note that everything they conclude is still withheld",
            "because it mentions a surrogate), and the triple-term boundary is still reported.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 4 -> 132 lines: the 113-line input-independent block cause C",
            "describes, plus 19 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:mentions rdf:type rdf:Property",
            "  ex:mentions rdf:type rdfs:Resource",
            "  ex:mentions rdfs:subPropertyOf ex:mentions",
            "  ex:says rdf:type rdf:Property",
            "  ex:says rdf:type rdfs:Resource",
            "  ex:says rdfs:subPropertyOf ex:mentions",
            "  ex:says rdfs:subPropertyOf ex:says",
            "  ex:x ex:mentions ( ex:A rdfs:subClassOf ex:B )",
            "  ex:x ex:says ( ex:A rdfs:subClassOf ex:B )",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "The line this fixture exists for is untouched: rdfs7 still concludes `x mentions",
            "<<( A rdfs:subClassOf B )>>` with the triple term carried through as itself, and",
            "there is still exactly one `x mentions` triple. rdfs14 / rdfs14a still do not",
            "fire and the triple-term boundary is still reported — and the boundary's REASON",
            "is now the accurate one: not that the term is opaque, but that both rules",
            "conclude about a fresh blank node the evaluator may not mint. Note that rdfs4",
            "does NOT type the triple term rdfs:Resource: that conclusion has a triple term",
            "in subject position, so it is generalized RDF and is dropped at the boundary.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 4 -> 13 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 13 -> 108 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 10 about this fixture's own terms.",
            "All 4 lines that were there before are unchanged.",
            "What is new are 6 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:mentions ex:says ex:x rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=56 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "AND THE BOUNDARY LIST MOVED, in this golden alone: eq-ref reaches a reflexive",
            "assertion whose subject is the TRIPLE TERM, which is generalized RDF the RDF 1.2 IR",
            "cannot hold, so the run now reports generalized-rdf beside the triple-term boundary",
            "it always reported. Nothing about the triple term itself changed.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule. Its boundary list",
            "adds triple-term, which the input holds.",
        ],
        quads: &[
            t(EX_SAYS, RDFS_SUBPROPERTYOF, EX_MENTIONS),
            t_quoted(EX_X, EX_SAYS, EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
        ],
    },
    Fixture {
        name: "divergence_literal_subject",
        doc: &[
            "DOCUMENTED DIVERGENCE 1 of 2 — NARROWER CONCLUSIONS. It is not a divergence",
            "of the CALCULUS: it is the RDF 1.2 IR declining to hold what the calculus",
            "concludes, and it survives the engine swap for that reason.",
            "",
            "`p rdfs:range A` with `x p \"cat\"^^xsd:string` makes rdfs3 / prp-rng conclude",
            "`\"cat\" rdf:type A`, whose subject is a literal. That is a GENERALIZED-RDF",
            "triple, which the RDF 1.2 dataset IR cannot represent, so the conclusion is",
            "abandoned when the answer is materialized, the drop is counted, and a",
            "generalized-rdf boundary is reported. The golden captures that answer.",
            "",
            "rdfs4 reaches the same wall from the other side — its object clause concludes",
            "`\"cat\" rdf:type rdfs:Resource`, which is a literal subject for the same",
            "reason — so the boundary now has two independent producers here rather than",
            "one, and the literal still never starts a line of the closure.",
            "",
            "The generalized fact is NOT withheld from the calculus — it stays in the",
            "evaluator's own term space and may still serve as a premise. Nothing here",
            "gives it one, so this closure is the input alone either way; what the",
            "distinction buys is that a REPRESENTABLE conclusion is never lost merely",
            "because its derivation passed through an unrepresentable one.",
        ],
        exercises: &["rdfs3", "prp-rng"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1 — RDFS and OWL-RL lose the three reflexive subPropertyOf triples on p,",
            "rdfs:range and rdfs:subPropertyOf (rdfs6 3 -> 0).",
            "THE DIVERGENCE THIS FIXTURE ISOLATES DID NOT MOVE, in either direction. No",
            "conclusion puts the literal in subject position, `\"cat\" rdf:type A` is still",
            "absent, and — the observable that was actually at risk — the generalized-rdf",
            "boundary is still REPORTED in both regimes. The evaluator now derives that",
            "conclusion in its own term space and abandons it when the answer is materialized",
            "back into the RDF 1.2 IR, so the boundary had to survive a mechanism change, not",
            "merely a rule change.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 123 lines: the 113-line input-independent block cause C",
            "describes, plus 10 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:range ex:A",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p \"cat\"",
            "  ex:x rdf:type rdfs:Resource",
            "",
            "THE DIVERGENCE THIS FIXTURE ISOLATES DID NOT MOVE: `\"cat\" rdf:type A` is still",
            "absent and the generalized-rdf boundary is still REPORTED. The claim that the",
            "closure equals the input is gone, and had to go — it was never the divergence,",
            "and rdfs4 now types x, p and A rdfs:Resource. rdfs4 also strengthens the",
            "boundary: its object clause concludes into subject position too, so the literal",
            "is now dropped twice over rather than once.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 104 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 6 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:p ex:x rdfs:range",
            "",
            "AND THE LITERAL STILL NEVER STARTS A LINE: eq-ref concludes `\"cat\" owl:sameAs",
            "\"cat\"`, which is a literal subject, so it is derived in the evaluator's own term",
            "space and abandoned exactly as rdfs3's and rdfs4's conclusions already were. The",
            "generalized-rdf boundary this fixture exists to pin now has a third independent",
            "producer and is reported as it always was.",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule. Its boundary list",
            "adds generalized-rdf, because dt-type2 and dt-eq did derive a literal-subject",
            "conclusion here and it was abandoned.",
        ],
        quads: &[t(EX_P, RDFS_RANGE, EX_A), t_lit(EX_X, EX_P, "cat")],
    },
    Fixture {
        name: "divergence_broad_triggers",
        doc: &[
            "DOCUMENTED DIVERGENCE 2 of 2 — BROADER TRIGGERS. This fixture has now been",
            "the control for two different questions, and holding both is what it is for.",
            "",
            "Nothing here is ASSERTED to be an rdfs:Class or an rdf:Property. The",
            "hand-written chase nevertheless emitted reflexive subClassOf on every",
            "subClassOf ENDPOINT and reflexive subPropertyOf on every PREDICATE; narrowing",
            "rdfs10 and rdfs6 to their specification premises removed those, correctly.",
            "",
            "But the triples were RDFS-entailed all along, by the longer path that",
            "narrowing left unwalked: rdfs:subClassOf has an AXIOMATIC rdfs:domain and",
            "rdfs:range of rdfs:Class, so rdfs2 and rdfs3 type both endpoints and only",
            "THEN does rdfs10 apply; rdfD2 types the predicate and only then does rdfs6.",
            "With the axiomatic triples asserted, the RDFS closure contains all five",
            "conclusions AND the premises that license each of them — while OWL-RL, which",
            "omits the axiomatic triples, still closes to the input alone.",
            "",
            "The property that tells the path from the shortcut is x and y: ex:p declares",
            "no domain and no range, so nothing types them rdfs:Class, and `x ⊑ x` and",
            "`y ⊑ y` are absent under BOTH lanes. A chase that had merely restored the",
            "endpoint shortcut would emit them.",
        ],
        exercises: &["rdfs6", "rdfs10"],
        changed: &[
            "Cause 2 — RDF gains `rdf:type rdf:type rdf:Property` (rdfD2 2 -> 3).",
            "Cause 1, IN FULL — this is the fixture that isolates it. All five unlicensed",
            "conclusions are gone from RDFS and OWL-RL (rdfs6 3 -> 0, rdfs10 2 -> 0):",
            "",
            "  A rdfs:subClassOf A                                    (rdfs10 on an endpoint)",
            "  B rdfs:subClassOf B                                    (rdfs10 on an endpoint)",
            "  p rdfs:subPropertyOf p                                 (rdfs6 on a predicate)",
            "  rdfs:subClassOf rdfs:subPropertyOf rdfs:subClassOf     (rdfs6 on a predicate)",
            "  rdfs:subPropertyOf rdfs:subPropertyOf rdfs:subPropertyOf  (rdfs6 on a predicate)",
            "",
            "Nothing here is typed rdfs:Class or rdf:Property, so the specification licenses",
            "neither rule, and the closure is now the input alone. The direction is the one",
            "predicted: FEWER triples.",
            "",
            "AT THE AXIOMATIC PATH — RDFS ONLY. Simple, RDF and OWL-RL are byte-identical.",
            "RDFS closure 2 -> 128 lines: the 113-line input-independent block cause C",
            "describes, plus 15 about this fixture's own terms —",
            "",
            "  ex:A rdf:type rdfs:Class",
            "  ex:A rdf:type rdfs:Resource",
            "  ex:A rdfs:subClassOf ex:A",
            "  ex:A rdfs:subClassOf ex:B",
            "  ex:A rdfs:subClassOf rdfs:Resource",
            "  ex:B rdf:type rdfs:Class",
            "  ex:B rdf:type rdfs:Resource",
            "  ex:B rdfs:subClassOf ex:B",
            "  ex:B rdfs:subClassOf rdfs:Resource",
            "  ex:p rdf:type rdf:Property",
            "  ex:p rdf:type rdfs:Resource",
            "  ex:p rdfs:subPropertyOf ex:p",
            "  ex:x ex:p ex:y",
            "  ex:x rdf:type rdfs:Resource",
            "  ex:y rdf:type rdfs:Resource",
            "",
            "THIS IS THE FIXTURE THE WHOLE CHANGE IS ABOUT, AND IT HAS CHANGED SIDES UNDER",
            "RDFS. All five conclusions the engine swap removed are back — but each is now",
            "accompanied, in this same closure, by the premise the specification requires:",
            "`A rdf:type rdfs:Class` and `B rdf:type rdfs:Class` (rdfs2 / rdfs3 over the",
            "axiomatic domain and range of rdfs:subClassOf) before `A ⊑ A` and `B ⊑ B`, and",
            "`p rdf:type rdf:Property` (rdfD2) before `p ⊑ p`. The two rdfs:subClassOf /",
            "rdfs:subPropertyOf self-edges are in the 113-line block, from the same path.",
            "THE CONTROL SURVIVES AND IS WHAT THE TEST NOW CHECKS: x and y are typed",
            "rdfs:Resource by rdfs4 and NOTHING ELSE — ex:p declares no domain and no range,",
            "so neither is an rdfs:Class and neither `x ⊑ x` nor `y ⊑ y` appears. A chase",
            "that had simply restored the endpoint shortcut would emit them.",
            "UNDER OWL-RL NOTHING CHANGED AT ALL: that lane asserts no axiomatic triples,",
            "its closure is still the input alone, and the shortcut is still gone.",
            "",
            "AT THE OWL-RL TABLES — OWL-RL ONLY. Simple, RDF and RDFS are byte-identical.",
            "OWL-RL closure 2 -> 11 lines: the nine-line input-independent prp-ap block",
            "cause ii describes, and nothing else: no rule this change added has a premise",
            "in this input, so the fixture's own conclusions are exactly what they were.",
            "The tally gains prp-ap=9.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 11 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 2 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:p ex:x ex:y",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_A, RDFS_SUBCLASSOF, EX_B), t(EX_X, EX_P, EX_Y)],
    },
    // ── Table 5 `prp-*`, Table 7 `cax-*` and Table 9 `scm-*` ──────────────────────
    //
    // Every fixture below is new with the OWL 2 RL tables, so none of them moved a
    // committed golden: each states, in its own `changed` field, that it did not exist.
    Fixture {
        name: "functional",
        doc: &[
            "prp-fp: a functional property's two values are the same thing. Also the NEAR",
            "MISS for prp-ifp — it differs from `inverse_functional` in exactly one term,",
            "the property characteristic — so the two fixtures are each other's control.",
        ],
        exercises: &["prp-fp"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 16 -> 108 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 10 about this fixture's own terms.",
            "All 7 lines that were there before are unchanged.",
            "What is new are 3 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:x owl:FunctionalProperty",
            "",
            "The tally gains eq-ref=53 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDF_TYPE, OWL_FUNCTIONALPROPERTY),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "inverse_functional",
        doc: &[
            "prp-ifp: an inverse-functional property's two subjects are the same thing.",
            "Also the NEAR MISS for prp-fp; see `functional`. The two share a shape and",
            "differ in the direction the property is read, which is exactly the difference",
            "between the two rules.",
        ],
        exercises: &["prp-ifp"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 16 -> 108 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 10 about this fixture's own terms.",
            "All 7 lines that were there before are unchanged.",
            "What is new are 3 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:w owl:InverseFunctionalProperty",
            "",
            "The tally gains eq-ref=53 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDF_TYPE, OWL_INVERSEFUNCTIONALPROPERTY),
            t(EX_X, EX_P, EX_W),
            t(EX_Y, EX_P, EX_W),
        ],
    },
    Fixture {
        name: "property_chain",
        doc: &[
            "AWKWARD CASE — prp-spo2, the first rule whose premise is an RDF COLLECTION of",
            "unbounded length. `chained owl:propertyChainAxiom (p q)` with the path",
            "`x p y q z` concludes `x chained z`.",
            "",
            "The rule is written `LIST[?x, ?p1, …, ?pn]` followed by n body atoms, which is",
            "a conjunction whose length depends on the data and therefore not a clause. It",
            "is stated instead as a recursion over rdf:first / rdf:rest into an INTERNAL",
            "ternary relation, grounded at the last cell so every clause stays",
            "range-restricted. The internal relation's rows are dropped before the answer is",
            "materialized, so this closure holds the conclusion and no trace of the walk.",
        ],
        exercises: &["prp-spo2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 17 -> 118 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 20 about this fixture's own terms.",
            "All 8 lines that were there before are unchanged.",
            "What is new are 12 reflexive `owl:sameAs` assertions. eq-ref draws one for every",
            "term of every triple; these are the ones the shared block does not already hold and",
            "no other rule had already reached, by subject:",
            "",
            "  ex:chained ex:l0 ex:l1 ex:p ex:q ex:x ex:y ex:z owl:propertyChainAxiom rdf:first",
            "  rdf:nil rdf:rest",
            "",
            "The tally gains eq-ref=62 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 7 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_CHAINED, OWL_PROPERTYCHAINAXIOM, EX_L0),
            t(EX_L0, RDF_FIRST, EX_P),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Q),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_Q, EX_Z),
        ],
    },
    Fixture {
        name: "property_chain_near_miss",
        doc: &[
            "NEAR MISS for prp-spo2: the path is broken at the join — the q-triple starts",
            "at u rather than at y — so the chain composes nothing.",
        ],
        exercises: &["prp-spo2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 16 -> 118 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 20 about this fixture's own terms.",
            "All 7 lines that were there before are unchanged.",
            "What is new are 13 reflexive `owl:sameAs` assertions. eq-ref draws one for every",
            "term of every triple; these are the ones the shared block does not already hold and",
            "no other rule had already reached, by subject:",
            "",
            "  ex:chained ex:l0 ex:l1 ex:p ex:q ex:u ex:x ex:y ex:z owl:propertyChainAxiom",
            "  rdf:first rdf:nil rdf:rest",
            "",
            "The tally gains eq-ref=63 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 7 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_CHAINED, OWL_PROPERTYCHAINAXIOM, EX_L0),
            t(EX_L0, RDF_FIRST, EX_P),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Q),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_Z),
        ],
    },
    Fixture {
        name: "equivalent_property_data",
        doc: &[
            "prp-eqp1 and prp-eqp2: an owl:equivalentProperty axiom re-predicates a triple",
            "in both directions. One axiom drives both rules — `x p y` drives prp-eqp1 and",
            "`u q v` drives prp-eqp2 — so the split of the rule into its two halves is",
            "observable rather than merely asserted.",
            "",
            "Both conclusions ALSO follow from scm-eqp1 and then prp-spo1, one round later.",
            "The report credits the rule that was FIRST to add the triple, and these two",
            "fire from the input alone while the two-hop path has to wait for scm-eqp1's",
            "conclusion — which is why the tally reads prp-eqp1=1 and prp-eqp2=1 here.",
        ],
        exercises: &["prp-eqp1", "prp-eqp2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 21 -> 118 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 20 about this fixture's own terms.",
            "All 12 lines that were there before are unchanged.",
            "What is new are 8 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:u ex:v ex:x ex:y owl:equivalentProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=58 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, OWL_EQUIVALENTPROPERTY, EX_Q),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_V),
        ],
    },
    Fixture {
        name: "equivalent_property_data_near_miss",
        doc: &[
            "NEAR MISS for prp-eqp1 and prp-eqp2: the axiom names r as p's equivalent, not",
            "q, so neither the p-triple nor the q-triple is re-predicated into the other.",
        ],
        exercises: &["prp-eqp1", "prp-eqp2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 20 -> 118 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 20 about this fixture's own terms.",
            "All 11 lines that were there before are unchanged.",
            "What is new are 9 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q ex:r ex:u ex:v ex:x ex:y owl:equivalentProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=59 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, OWL_EQUIVALENTPROPERTY, EX_R),
            t(EX_X, EX_P, EX_Y),
            t(EX_U, EX_Q, EX_V),
        ],
    },
    Fixture {
        name: "equivalent_class_instance",
        doc: &[
            "cax-eqc1 and cax-eqc2: an owl:equivalentClass axiom re-types an instance in",
            "both directions. One axiom drives both rules, as `equivalent_property_data`",
            "does for the property pair.",
            "",
            "Both conclusions also follow from scm-eqc1 and then cax-sco, one round later;",
            "these two reach them from the input alone and are therefore the rules the",
            "report credits.",
        ],
        exercises: &["cax-eqc1", "cax-eqc2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 21 -> 114 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 16 about this fixture's own terms.",
            "All 12 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:u ex:x",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, OWL_EQUIVALENTCLASS, EX_B),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_U, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "equivalent_class_instance_near_miss",
        doc: &[
            "NEAR MISS for cax-eqc1 and cax-eqc2: the axiom names C as A's equivalent, not",
            "B, so neither instance is re-typed into the other's class.",
        ],
        exercises: &["cax-eqc1", "cax-eqc2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 20 -> 114 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 16 about this fixture's own terms.",
            "All 11 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:C ex:u ex:x",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 3 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, OWL_EQUIVALENTCLASS, EX_C),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_U, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "owl_class",
        doc: &[
            "scm-cls: an owl:Class is a sub-class of itself and of owl:Thing, equivalent to",
            "itself, and a super-class of owl:Nothing. The specification's conclusion is a",
            "CONJUNCTION of those four triples; the calculus states one clause per conjunct",
            "over the one premise, which is the same statement without a head form the",
            "evaluator refuses.",
            "",
            "`class_typed` is the NEAR MISS: it types C rdfs:Class rather than owl:Class, so",
            "rdfs8 and rdfs10 fire and scm-cls does not — the two fixtures differ in exactly",
            "the one term the premise reads.",
        ],
        exercises: &["scm-cls"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 15 -> 104 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 6 about this fixture's own terms.",
            "Of the 6 fixture-specific lines that were there before, 5 are unchanged and one —",
            "`owl:Nothing rdfs:subClassOf owl:Thing` — is still in the closure but is now part of",
            "the shared block, because cls-nothing1 and cls-thing put it there for every input.",
            "Nothing was lost.",
            "What is new is one reflexive `owl:sameAs` assertion. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C",
            "",
            "The tally gains eq-ref=51 cls-thing=1 cls-nothing1=1 dt-type1=32; the report's",
            "missing list is empty where it held 41 ids, and its completeness reads exact-within-",
            "boundaries where it read sound-incomplete.",
            "AND ATTRIBUTION MOVED, IN BOTH DIRECTIONS. scm-cls reads 9 where it read 4, because",
            "cls-thing and cls-nothing1 give it two more premises: it now says its four things",
            "about owl:Thing and owl:Nothing as well as about C, and five of those eight are",
            "distinct. scm-sco falls from 1 to 0 and leaves the tally, for the same reason and",
            "not for a different one: its single conclusion here was `owl:Nothing rdfs:subClassOf",
            "owl:Thing`, composed from `owl:Nothing ⊑ C` and `C ⊑ owl:Thing`, and scm-cls now",
            "draws that triple directly from `owl:Thing rdf:type owl:Class` in one step. The",
            "triple is still in the closure — it is the line named above as having moved into the",
            "shared block.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_C, RDF_TYPE, OWL_CLASS)],
    },
    Fixture {
        name: "mutual_subclass",
        doc: &[
            "scm-eqc2: two classes that are sub-classes of each other are equivalent — the",
            "converse of scm-eqc1. Also the NEAR MISS for scm-eqp2; see `mutual_subproperty`.",
        ],
        exercises: &["scm-eqc2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 17 -> 108 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 10 about this fixture's own terms.",
            "All 8 lines that were there before are unchanged.",
            "What is new are 2 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B",
            "",
            "The tally gains eq-ref=52 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, RDFS_SUBCLASSOF, EX_B),
            t(EX_B, RDFS_SUBCLASSOF, EX_A),
        ],
    },
    Fixture {
        name: "mutual_subproperty",
        doc: &[
            "scm-eqp2: two properties that are sub-properties of each other are equivalent —",
            "the converse of scm-eqp1. Also the NEAR MISS for scm-eqc2: the two fixtures",
            "have the same shape over the class and the property hierarchy respectively, so",
            "each denies the other's conclusion by naming different terms in the same",
            "position.",
        ],
        exercises: &["scm-eqp2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 17 -> 110 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 12 about this fixture's own terms.",
            "All 8 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p ex:q owl:equivalentProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
            t(EX_Q, RDFS_SUBPROPERTYOF, EX_P),
        ],
    },
    Fixture {
        name: "object_property",
        doc: &[
            "scm-op: an owl:ObjectProperty is a sub-property of, and equivalent to, itself.",
            "Also the NEAR MISS for scm-dp; see `datatype_property`.",
        ],
        exercises: &["scm-op"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:p owl:ObjectProperty owl:equivalentProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_P, RDF_TYPE, OWL_OBJECTPROPERTY)],
    },
    Fixture {
        name: "datatype_property",
        doc: &[
            "scm-dp: an owl:DatatypeProperty is a sub-property of, and equivalent to,",
            "itself. Also the NEAR MISS for scm-op; the two rules differ in exactly one",
            "constant — the class the premise names — and so do the two fixtures, which",
            "additionally name different properties so that neither closure can contain the",
            "other's conclusion.",
        ],
        exercises: &["scm-dp"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:q owl:DatatypeProperty owl:equivalentProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 1 input quad plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_Q, RDF_TYPE, OWL_DATATYPEPROPERTY)],
    },
    Fixture {
        name: "domain_widened",
        doc: &[
            "scm-dom1: a domain widens along rdfs:subClassOf. Also the NEAR MISS for",
            "scm-dom2; see `domain_inherited`.",
        ],
        exercises: &["scm-dom1"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:r rdfs:domain",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_R, RDFS_DOMAIN, EX_A), t(EX_A, RDFS_SUBCLASSOF, EX_B)],
    },
    Fixture {
        name: "domain_inherited",
        doc: &[
            "scm-dom2: a domain is inherited along rdfs:subPropertyOf. Also the NEAR MISS",
            "for scm-dom1: the two rules move a domain declaration along the two different",
            "hierarchies, and each fixture supplies the hierarchy the other does not.",
        ],
        exercises: &["scm-dom2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:p ex:q rdfs:domain rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_Q, RDFS_DOMAIN, EX_A),
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
        ],
    },
    Fixture {
        name: "range_widened",
        doc: &[
            "scm-rng1: a range widens along rdfs:subClassOf. Also the NEAR MISS for",
            "scm-rng2; see `range_inherited`.",
        ],
        exercises: &["scm-rng1"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 105 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 7 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 4 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:r rdfs:range",
            "",
            "The tally gains eq-ref=54 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_R, RDFS_RANGE, EX_A), t(EX_A, RDFS_SUBCLASSOF, EX_B)],
    },
    Fixture {
        name: "range_inherited",
        doc: &[
            "scm-rng2: a range is inherited along rdfs:subPropertyOf. Also the NEAR MISS",
            "for scm-rng1, on the same pairing as `domain_widened` / `domain_inherited`.",
        ],
        exercises: &["scm-rng2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 12 -> 106 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 8 about this fixture's own terms.",
            "All 3 lines that were there before are unchanged.",
            "What is new are 5 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:p ex:q rdfs:range rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=55 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 2 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[t(EX_Q, RDFS_RANGE, EX_A), t(EX_P, RDFS_SUBPROPERTYOF, EX_Q)],
    },
    Fixture {
        name: "has_value_restrictions",
        doc: &[
            "scm-hv: two owl:hasValue restrictions on the SAME value, whose properties are",
            "related by rdfs:subPropertyOf, are ordered by rdfs:subClassOf the same way.",
        ],
        exercises: &["scm-hv"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 15 -> 112 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 14 about this fixture's own terms.",
            "All 6 lines that were there before are unchanged.",
            "What is new are 8 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:p ex:q ex:x owl:hasValue owl:onProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=58 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, OWL_HASVALUE, EX_X),
            t(EX_A, OWL_ONPROPERTY, EX_P),
            t(EX_B, OWL_HASVALUE, EX_X),
            t(EX_B, OWL_ONPROPERTY, EX_Q),
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
        ],
    },
    Fixture {
        name: "has_value_restrictions_near_miss",
        doc: &[
            "NEAR MISS for scm-hv: the two restrictions name DIFFERENT values (x and y), so",
            "the sub-property relation between their properties orders nothing.",
        ],
        exercises: &["scm-hv"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 14 -> 112 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 14 about this fixture's own terms.",
            "All 5 lines that were there before are unchanged.",
            "What is new are 9 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:p ex:q ex:x ex:y owl:hasValue owl:onProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=59 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, OWL_HASVALUE, EX_X),
            t(EX_A, OWL_ONPROPERTY, EX_P),
            t(EX_B, OWL_HASVALUE, EX_Y),
            t(EX_B, OWL_ONPROPERTY, EX_Q),
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
        ],
    },
    Fixture {
        name: "some_values_filler",
        doc: &[
            "scm-svf1: two owl:someValuesFrom restrictions on ONE property, ordered by",
            "their fillers. Also the NEAR MISS for scm-svf2; see `some_values_property`.",
        ],
        exercises: &["scm-svf1"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 15 -> 111 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 13 about this fixture's own terms.",
            "All 6 lines that were there before are unchanged.",
            "What is new are 7 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:C ex:D ex:p owl:onProperty owl:someValuesFrom",
            "",
            "The tally gains eq-ref=57 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, OWL_SOMEVALUESFROM, EX_C),
            t(EX_A, OWL_ONPROPERTY, EX_P),
            t(EX_B, OWL_SOMEVALUESFROM, EX_D),
            t(EX_B, OWL_ONPROPERTY, EX_P),
            t(EX_C, RDFS_SUBCLASSOF, EX_D),
        ],
    },
    Fixture {
        name: "some_values_property",
        doc: &[
            "scm-svf2: two owl:someValuesFrom restrictions on ONE filler, ordered by their",
            "properties. Also the NEAR MISS for scm-svf1: the two rules differ in which of",
            "the restriction's two coordinates varies, and the fixtures name different",
            "classes so neither closure can hold the other's conclusion.",
        ],
        exercises: &["scm-svf2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 15 -> 112 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 14 about this fixture's own terms.",
            "All 6 lines that were there before are unchanged.",
            "What is new are 8 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C ex:E ex:F ex:p ex:q owl:onProperty owl:someValuesFrom rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=58 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_E, OWL_SOMEVALUESFROM, EX_C),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_F, OWL_SOMEVALUESFROM, EX_C),
            t(EX_F, OWL_ONPROPERTY, EX_Q),
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
        ],
    },
    Fixture {
        name: "all_values_filler",
        doc: &[
            "scm-avf1: two owl:allValuesFrom restrictions on ONE property, ordered by their",
            "fillers — covariantly, exactly like scm-svf1. Also the NEAR MISS for scm-avf2;",
            "see `all_values_property`.",
        ],
        exercises: &["scm-avf1"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 15 -> 111 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 13 about this fixture's own terms.",
            "All 6 lines that were there before are unchanged.",
            "What is new are 7 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:C ex:D ex:p owl:allValuesFrom owl:onProperty",
            "",
            "The tally gains eq-ref=57 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_A, OWL_ALLVALUESFROM, EX_C),
            t(EX_A, OWL_ONPROPERTY, EX_P),
            t(EX_B, OWL_ALLVALUESFROM, EX_D),
            t(EX_B, OWL_ONPROPERTY, EX_P),
            t(EX_C, RDFS_SUBCLASSOF, EX_D),
        ],
    },
    Fixture {
        name: "all_values_property",
        doc: &[
            "AWKWARD CASE — scm-avf2, the one rule of Table 9 whose conclusion is",
            "CONTRAVARIANT. Two owl:allValuesFrom restrictions on one filler with",
            "`p rdfs:subPropertyOf q` conclude `F rdfs:subClassOf E`, the other way round",
            "from scm-svf2's `E rdfs:subClassOf F` on the identical premise shape: a",
            "universal restriction over the WIDER property is the stronger class. This",
            "fixture and `some_values_property` are byte-for-byte the same input but for the",
            "restriction predicate, so the direction of the conclusion is attributable to",
            "the rule and to nothing else.",
            "",
            "It is also the NEAR MISS for scm-avf1; see `all_values_filler`.",
        ],
        exercises: &["scm-avf2"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 15 -> 112 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 14 about this fixture's own terms.",
            "All 6 lines that were there before are unchanged.",
            "What is new are 8 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C ex:E ex:F ex:p ex:q owl:allValuesFrom owl:onProperty rdfs:subPropertyOf",
            "",
            "The tally gains eq-ref=58 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_E, OWL_ALLVALUESFROM, EX_C),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_F, OWL_ALLVALUESFROM, EX_C),
            t(EX_F, OWL_ONPROPERTY, EX_Q),
            t(EX_P, RDFS_SUBPROPERTYOF, EX_Q),
        ],
    },
    Fixture {
        name: "intersection_of",
        doc: &[
            "scm-int: an intersection is a sub-class of each of its members. The premise is",
            "an RDF COLLECTION, read through the list pre-pass rather than through a",
            "recursion: the rule concludes one triple per member INDEPENDENTLY, so",
            "membership is all it needs and the member's position is bound and unused.",
            "",
            "Also the NEAR MISS for scm-uni; see `union_of`.",
        ],
        exercises: &["scm-int"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 16 -> 114 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 16 about this fixture's own terms.",
            "All 7 lines that were there before are unchanged.",
            "What is new are 9 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:C ex:l0 ex:l1 owl:intersectionOf rdf:first rdf:nil rdf:rest",
            "",
            "The tally gains eq-ref=59 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_C, OWL_INTERSECTIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
        ],
    },
    Fixture {
        name: "union_of",
        doc: &[
            "scm-uni: a union is a super-class of each of its members. The same collection",
            "as `intersection_of`, under the other connective and a different class, so the",
            "two are each other's near miss and the DIRECTION of each rule's conclusion is",
            "what separates them.",
        ],
        exercises: &["scm-uni"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 16 -> 114 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 16 about this fixture's own terms.",
            "All 7 lines that were there before are unchanged.",
            "What is new are 9 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:A ex:B ex:D ex:l0 ex:l1 owl:unionOf rdf:first rdf:nil rdf:rest",
            "",
            "The tally gains eq-ref=59 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "THE NEW D SECTION: this fixture's 5 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_D, OWL_UNIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
        ],
    },
    Fixture {
        name: "has_key",
        doc: &[
            "AWKWARD CASE — prp-key, whose premise UNIVERSALLY quantifies over an RDF",
            "collection: two instances of C that agree on every property of C's key are the",
            "same thing.",
            "",
            "Like prp-spo2 this is stated as a recursion over rdf:first / rdf:rest into an",
            "internal ternary relation — `agree from this cell onwards` — and the internal",
            "rows never reach the answer. The key here is one property long, which is the",
            "smallest list for which the universal quantifier means anything.",
        ],
        exercises: &["prp-key"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 20 -> 117 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 19 about this fixture's own terms.",
            "All 11 lines that were there before are unchanged.",
            "What is new are 8 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C ex:l0 ex:p ex:z owl:hasKey rdf:first rdf:nil rdf:rest",
            "",
            "The tally gains eq-ref=60 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "AND ATTRIBUTION MOVED: prp-key reads 2 where it read 4. Nothing stopped firing and",
            "no triple left the closure. Two of prp-key's four conclusions were the trivial `x",
            "owl:sameAs x` and `y owl:sameAs y` — every instance agrees with ITSELF on every key",
            "property — and eq-ref reaches both from the input triples in one step where prp-key",
            "has to walk the key list first. A report credits the shorter proof, so prp-key keeps",
            "exactly the two conclusions that are about the key: `x owl:sameAs y` and its",
            "symmetric partner.",
            "THE NEW D SECTION: this fixture's 7 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_C, OWL_HASKEY, EX_L0),
            t(EX_L0, RDF_FIRST, EX_P),
            t(EX_L0, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_C),
            t(EX_Y, RDF_TYPE, EX_C),
            t(EX_X, EX_P, EX_Z),
            t(EX_Y, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "has_key_near_miss",
        doc: &[
            "NEAR MISS for prp-key: the two instances DISAGREE on the key property — y's",
            "value is u where x's is z — so they are not identified.",
        ],
        exercises: &["prp-key"],
        changed: &[
            "NEW FIXTURE — no committed golden moved; this one did not exist.",
            "",
            "AT THE COMPLETED OWL-RL TABLES — OWL-RL ONLY, PLUS A NEW D SECTION. Simple, RDF",
            "and RDFS are byte-identical.",
            "OWL-RL closure 18 -> 116 lines: the 98-line input-independent premise-free block",
            "cause iii describes — which subsumes the nine prp-ap lines the previous change's",
            "cause ii named — plus 18 about this fixture's own terms.",
            "All 9 lines that were there before are unchanged.",
            "What is new are 9 reflexive `owl:sameAs` assertions. eq-ref draws one for every term",
            "of every triple; these are the ones the shared block does not already hold and no",
            "other rule had already reached, by subject:",
            "",
            "  ex:C ex:l0 ex:p ex:u ex:z owl:hasKey rdf:first rdf:nil rdf:rest",
            "",
            "The tally gains eq-ref=61 cls-thing=1 cls-nothing1=1 dt-type1=32 scm-cls=5; the",
            "report's missing list is empty where it held 41 ids, and its completeness reads",
            "exact-within-boundaries where it read sound-incomplete.",
            "AND ONE RULE LEAVES THE TALLY: prp-key falls from 2 to 0 and is no longer named at",
            "all. That is the same effect as in `has_key` taken to its conclusion — BOTH of the",
            "conclusions prp-key had here were the trivial self-agreements `x owl:sameAs x` and",
            "`y owl:sameAs y`, because this fixture is the near miss and x and y do NOT agree on",
            "the key — and eq-ref now reaches both in fewer steps. No triple left the closure,",
            "and prp-key is still credited where it says something: `has_key`, at 2.",
            "THE NEW D SECTION: this fixture's 7 input quads plus dt-type1's thirty-two",
            "rdfs:Datatype typings, and nothing else. dt-type2, dt-eq and dt-diff conclude only",
            "triples with a literal subject, and the D lane holds no rule that could consume one,",
            "so its closure is Simple entailment plus one premise-free rule.",
        ],
        quads: &[
            t(EX_C, OWL_HASKEY, EX_L0),
            t(EX_L0, RDF_FIRST, EX_P),
            t(EX_L0, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_C),
            t(EX_Y, RDF_TYPE, EX_C),
            t(EX_X, EX_P, EX_Z),
            t(EX_Y, EX_P, EX_U),
        ],
    },
    // ── Table 4 `eq-*`, Table 6 `cls-*` and Table 8 `dt-*` ────────────────────────
    //
    // Every fixture below is new with the completed OWL 2 RL rule table, so none of them
    // moved a committed golden: each states, in its own `changed` field, that it did not
    // exist. The fixtures for the rules that conclude `false` are NOT here — a run over one
    // of those has no closure to write a golden from — they are in [`CLASH_CORPUS`].
    Fixture {
        name: "same_as",
        doc: &[
            "eq-sym: owl:sameAs is symmetric. Differs from `plain_triple` in exactly one",
            "term — the predicate — so the two are each other's control: `plain_triple`",
            "concludes no owl:sameAs between distinct terms at all.",
        ],
        exercises: &["eq-sym"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_X, OWL_SAMEAS, EX_Y)],
    },
    Fixture {
        name: "same_as_chain",
        doc: &["eq-trans: owl:sameAs is transitive."],
        exercises: &["eq-trans"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_X, OWL_SAMEAS, EX_Y), t(EX_Y, OWL_SAMEAS, EX_Z)],
    },
    Fixture {
        name: "same_as_chain_near_miss",
        doc: &[
            "NEAR MISS for eq-trans: the chain is broken at the join point — the second",
            "edge starts at u rather than at y — so x and z are not identified. eq-sym",
            "closes each edge in both directions and still reaches nothing across the gap.",
        ],
        exercises: &["eq-trans"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_X, OWL_SAMEAS, EX_Y), t(EX_U, OWL_SAMEAS, EX_Z)],
    },
    Fixture {
        name: "same_as_subject",
        doc: &[
            "eq-rep-s: equality substitutes in SUBJECT position. `x owl:sameAs u` with",
            "`x p y` concludes `u p y`.",
            "",
            "This fixture, `same_as_predicate` and `same_as_object` are the same two triples",
            "differing only in WHICH POSITION of `x p y` the owl:sameAs axiom names, which",
            "is exactly the difference between the three eq-rep-* rules — so the three are",
            "each other's near misses and no other input is needed to deny any of them.",
        ],
        exercises: &["eq-rep-s"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_X, OWL_SAMEAS, EX_U), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "same_as_predicate",
        doc: &[
            "eq-rep-p: equality substitutes in PREDICATE position. `p owl:sameAs q` with",
            "`x p y` concludes `x q y` — the one rule of the calculus that rewrites a",
            "predicate from a variable bound in the OBJECT position of another atom.",
            "",
            "Also the near miss for eq-rep-s and eq-rep-o; see `same_as_subject`.",
        ],
        exercises: &["eq-rep-p"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_P, OWL_SAMEAS, EX_Q), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "same_as_object",
        doc: &[
            "eq-rep-o: equality substitutes in OBJECT position. `y owl:sameAs v` with",
            "`x p y` concludes `x p v`.",
            "",
            "Also the near miss for eq-rep-s; see `same_as_subject`.",
        ],
        exercises: &["eq-rep-o"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[t(EX_Y, OWL_SAMEAS, EX_V), t(EX_X, EX_P, EX_Y)],
    },
    Fixture {
        name: "intersection_instance",
        doc: &[
            "cls-int1: an instance of EVERY member of an intersection list is an instance",
            "of the intersection. The premise is a conjunction whose length depends on the",
            "data, stated as a recursion into an internal relation, so a two-member list is",
            "the smallest input for which the universal quantifier means anything.",
        ],
        exercises: &["cls-int1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_C, OWL_INTERSECTIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_X, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "intersection_instance_near_miss",
        doc: &[
            "NEAR MISS for cls-int1: x is an instance of the FIRST member and of E, which",
            "is not in the list, so the conjunction over the list is unsatisfied and",
            "`x rdf:type C` is not concluded.",
        ],
        exercises: &["cls-int1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_C, OWL_INTERSECTIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_X, RDF_TYPE, EX_E),
        ],
    },
    Fixture {
        name: "intersection_member_typing",
        doc: &[
            "cls-int2: an instance of an intersection is an instance of EVERY member. The",
            "conclusion is per-member, so this rule reads list MEMBERSHIP where cls-int1",
            "has to walk the list.",
            "",
            "`intersection_of` is the NEAR MISS: it is the identical axiom with no instance,",
            "so the members are named and nothing is typed by them.",
        ],
        exercises: &["cls-int2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_C, OWL_INTERSECTIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_C),
        ],
    },
    Fixture {
        name: "union_instance",
        doc: &[
            "cls-uni: an instance of ANY member of a union list is an instance of the",
            "union — the existential counterpart of cls-int1's universal.",
        ],
        exercises: &["cls-uni"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_D, OWL_UNIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_A),
        ],
    },
    Fixture {
        name: "union_instance_near_miss",
        doc: &[
            "NEAR MISS for cls-uni: x is an instance of E, which is not a member of the",
            "union, so `x rdf:type D` is not concluded.",
        ],
        exercises: &["cls-uni"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_D, OWL_UNIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_E),
        ],
    },
    Fixture {
        name: "some_values_instance",
        doc: &[
            "cls-svf1: an existential restriction recognizes its instances — x has a",
            "p-value typed by the filler, so x is an instance of the restriction.",
        ],
        exercises: &["cls-svf1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_SOMEVALUESFROM, EX_C),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, RDF_TYPE, EX_C),
        ],
    },
    Fixture {
        name: "some_values_instance_near_miss",
        doc: &[
            "NEAR MISS for cls-svf1 AND for cls-svf2, in the one input. The p-value y is",
            "not typed by the filler, so cls-svf1 has no premise; and the filler is C rather",
            "than owl:Thing, so cls-svf2 has none either. `x rdf:type E` is absent both ways.",
        ],
        exercises: &["cls-svf1", "cls-svf2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_SOMEVALUESFROM, EX_C),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "some_values_thing",
        doc: &[
            "cls-svf2: an existential restriction over owl:Thing recognizes anything with a",
            "value at all. Not a redundant special case of cls-svf1: nothing in this",
            "calculus types an INDIVIDUAL owl:Thing — cls-thing types the CLASS — so the",
            "filler-typed premise cls-svf1 needs is unreachable and the specification states",
            "this case separately. It differs from `some_values_instance_near_miss` in",
            "exactly the filler.",
        ],
        exercises: &["cls-svf2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_SOMEVALUESFROM, OWL_THING),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "all_values_instance",
        doc: &[
            "cls-avf: a universal restriction types the values of its instances — x is an",
            "instance of the restriction and has a p-value, so that value is typed by the",
            "filler.",
        ],
        exercises: &["cls-avf"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_ALLVALUESFROM, EX_C),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "all_values_instance_near_miss",
        doc: &[
            "NEAR MISS for cls-avf: the restriction's instance is u, not x, so the p-value",
            "y belongs to something the restriction says nothing about and is not typed.",
        ],
        exercises: &["cls-avf"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_ALLVALUESFROM, EX_C),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_U, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "has_value_assert",
        doc: &[
            "cls-hv1: an owl:hasValue restriction ASSERTS the value on each of its",
            "instances. The value is a LITERAL, which is what makes this rule's conclusion",
            "one the three-IRI shape cannot express — the registry's conclusion type carries",
            "a typed literal for exactly this case.",
        ],
        exercises: &["cls-hv1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit(EX_E, OWL_HASVALUE, "cat"),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, RDF_TYPE, EX_E),
        ],
    },
    Fixture {
        name: "has_value_recognize",
        doc: &[
            "cls-hv2: the converse of cls-hv1 — whatever carries the value is RECOGNIZED as",
            "an instance of the restriction.",
        ],
        exercises: &["cls-hv2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit(EX_E, OWL_HASVALUE, "cat"),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t_lit(EX_X, EX_P, "cat"),
        ],
    },
    Fixture {
        name: "has_value_near_miss",
        doc: &[
            "NEAR MISS for cls-hv1 AND for cls-hv2, in the one input. x carries \"dog\"",
            "where the restriction names \"cat\", so cls-hv2 does not recognize x and",
            "nothing types x an instance for cls-hv1 to assert \"cat\" on. Both of the",
            "conclusions those two rules license are therefore absent.",
        ],
        exercises: &["cls-hv1", "cls-hv2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit(EX_E, OWL_HASVALUE, "cat"),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t_lit(EX_X, EX_P, "dog"),
        ],
    },
    Fixture {
        name: "max_cardinality_one",
        doc: &[
            "cls-maxc2: two p-values on an instance of an owl:maxCardinality 1 restriction",
            "are the same thing. The cardinality literal is",
            "\"1\"^^xsd:nonNegativeInteger exactly as OWL 2 Profiles Table 6 writes it.",
        ],
        exercises: &["cls-maxc2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(EX_E, OWL_MAXCARDINALITY, "1", XSD_NONNEGATIVEINTEGER),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "max_cardinality_one_near_miss",
        doc: &[
            "NEAR MISS for cls-maxc2: the cardinality is \"2\", and Table 6 matches the",
            "literals \"0\" and \"1\" and no others, so no rule of the family has a premise",
            "and the two values are not identified.",
        ],
        exercises: &["cls-maxc2"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(EX_E, OWL_MAXCARDINALITY, "2", XSD_NONNEGATIVEINTEGER),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "max_qualified_one",
        doc: &[
            "cls-maxqc3: two p-values TYPED BY THE QUALIFYING CLASS on an instance of an",
            "owl:maxQualifiedCardinality 1 restriction are the same thing.",
        ],
        exercises: &["cls-maxqc3"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "1",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, EX_C),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, RDF_TYPE, EX_C),
            t(EX_X, EX_P, EX_Z),
            t(EX_Z, RDF_TYPE, EX_C),
        ],
    },
    Fixture {
        name: "max_qualified_one_near_miss",
        doc: &[
            "NEAR MISS for cls-maxqc3 AND for cls-maxqc4, in the one input. z is typed D",
            "rather than the qualifying class C, so cls-maxqc3 sees one qualifying value",
            "and not two; and the qualifying class is C rather than owl:Thing, so cls-maxqc4",
            "has no premise at all. `y owl:sameAs z` is absent both ways.",
        ],
        exercises: &["cls-maxqc3", "cls-maxqc4"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "1",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, EX_C),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, RDF_TYPE, EX_C),
            t(EX_X, EX_P, EX_Z),
            t(EX_Z, RDF_TYPE, EX_D),
        ],
    },
    Fixture {
        name: "max_qualified_one_thing",
        doc: &[
            "cls-maxqc4: the same as cls-maxqc3 with owl:Thing as the qualifying class,",
            "where the values need no typing at all. It differs from",
            "`max_qualified_one_near_miss` in the owl:onClass term, which is exactly the",
            "difference between the two rules.",
        ],
        exercises: &["cls-maxqc4"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "1",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, OWL_THING),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "one_of",
        doc: &[
            "cls-oo: every member of an owl:oneOf enumeration is an instance of it. Read",
            "through the list pre-pass, like cls-int2 and cls-uni: the conclusion is",
            "per-member, so membership is all the rule needs.",
        ],
        exercises: &["cls-oo"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_C, OWL_ONEOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_X),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Y),
            t(EX_L1, RDF_REST, RDF_NIL),
        ],
    },
    Fixture {
        name: "one_of_near_miss",
        doc: &[
            "NEAR MISS for cls-oo: the identical list under owl:unionOf. A union types",
            "nothing until something is typed by a member, and nothing here is, so",
            "`x rdf:type C` is absent — the enumeration's own claim, and only the",
            "enumeration's.",
        ],
        exercises: &["cls-oo"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_C, OWL_UNIONOF, EX_L0),
            t(EX_L0, RDF_FIRST, EX_X),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Y),
            t(EX_L1, RDF_REST, RDF_NIL),
        ],
    },
    Fixture {
        name: "datatype_value_typing",
        doc: &[
            "dt-type2, evidenced DOWNSTREAM. Every conclusion of dt-type2 has a LITERAL",
            "subject — `lt rdf:type dt` — so not one of them can be materialized into the",
            "RDF 1.2 IR and the rule can never be credited in a closure. What it licenses",
            "IS observable: `\"1\"^^xsd:integer rdf:type xsd:integer` is the filler-typed",
            "premise cls-svf1 needs, so the restriction E recognizes x.",
            "",
            "`x rdf:type E` is therefore dt-type2's evidence, and the generalized-rdf",
            "boundary on the same run is the evidence that the intermediate conclusion was",
            "derived and then abandoned rather than never drawn.",
        ],
        exercises: &["dt-type2", "cls-svf1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_SOMEVALUESFROM, XSD_INTEGER),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER),
        ],
    },
    Fixture {
        name: "datatype_value_typing_near_miss",
        doc: &[
            "NEAR MISS for dt-type2: the filler is xsd:string, and the data value of",
            "\"1\"^^xsd:integer is not in xsd:string's value space, so dt-type2 does not",
            "type the literal by it and cls-svf1 has no premise: `x rdf:type E` is absent.",
        ],
        exercises: &["dt-type2", "cls-svf1"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t(EX_E, OWL_SOMEVALUESFROM, XSD_STRING),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER),
        ],
    },
    Fixture {
        name: "datatype_value_equality",
        doc: &[
            "dt-eq, evidenced DOWNSTREAM, for the same reason dt-type2 is: every conclusion",
            "of the rule — `lt1 owl:sameAs lt2` — has a literal subject and is dropped at",
            "the materialization boundary.",
            "",
            "\"1\"^^xsd:integer and \"01\"^^xsd:integer are two lexical forms of ONE data",
            "value, so dt-eq makes them owl:sameAs and eq-rep-o rewrites the object of",
            "`x p \"1\"` into `x p \"01\"`. That triple the IR holds perfectly well, and it",
            "is the whole of what dt-eq is for: the rule exists so an ontology written in",
            "one lexical form can meet a rule written in another.",
        ],
        exercises: &["dt-eq", "eq-rep-o"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER),
            t_lit_dt(EX_Y, EX_Q, "01", XSD_INTEGER),
        ],
    },
    Fixture {
        name: "datatype_value_equality_near_miss",
        doc: &[
            "NEAR MISS for dt-eq: \"2\"^^xsd:integer is a DIFFERENT data value from",
            "\"1\"^^xsd:integer, so the two literals are not owl:sameAs, eq-rep-o has no",
            "premise, and `x p \"01\"^^xsd:integer` is absent. Nothing here is inconsistent",
            "either: dt-diff concludes owl:differentFrom only where something already",
            "asserts owl:sameAs over the same pair, and nothing does.",
        ],
        exercises: &["dt-eq", "eq-rep-o"],
        changed: &["NEW FIXTURE — no committed golden moved; this one did not exist."],
        quads: &[
            t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER),
            t_lit_dt(EX_Y, EX_Q, "2", XSD_INTEGER),
        ],
    },
];

/// The [`Fixture::changed`] accounting every [`CLASH_CORPUS`] fixture carries.
///
/// A `changed` entry is a claim about a golden, and these fixtures have none, so the claim
/// they make is that there is nothing to claim — stated once, shared, and asserted by
/// `the_goldens_directory_is_exactly_the_corpus` rather than left as a convention.
const REFUSAL_GOLDEN: &[&str] = &[
    "A REFUSAL GOLDEN. This fixture belongs to CLASH_CORPUS, whose evidence is the OUTCOME",
    "OF A RUN rather than a closure: the `clashes` half must refuse with a named",
    "inconsistency witness, and the refusing half has no closure. Its control is kept beside",
    "it for the same reason — a pair of runs is the unit of evidence here, and splitting it",
    "across two tables would put half of a control in a corpus that cannot hold the other",
    "half.",
    "",
    "These fixtures had NO golden until the refusal started carrying a report. It used to",
    "carry an InconsistencyWitness and nothing else, so the caller whose data was",
    "inconsistent — the one caller who most needed to know which rules had fired, what the",
    "evaluation had cost and which calculus hash refused — was the only caller who got none",
    "of it, and `inconsistency` was a report field no input could move off `none`. It now",
    "carries an InconsistentRun: the witness AND the run's ReasoningReport. So a refusing",
    "regime's section below renders the witness's premises and then that report, whose",
    "`inconsistency:` line names the rule instead of reading `none` — the only place in this",
    "corpus where it does.",
];

/// The fixtures whose evidence is that a run over them is REFUSED, and their controls.
///
/// The seventeen OWL 2 RL rules that conclude `false`, plus `dt-diff`, are evidenced by
/// [`RuleFixtures::Refuting`]: `materialize(.., Regime::OwlRl)` over the `clashes` fixture
/// must return [`purrdf_entail::EntailError::Inconsistent`] naming the expected rule, and
/// must succeed over the `consistent` one. Neither half can live in [`CORPUS`]: the golden
/// writer materializes every fixture under all five regimes and would panic on the first.
///
/// Every pair differs in exactly one term — the term the rule's premise reads — for the
/// same reason every near miss in [`CORPUS`] does: "the rule did not fire" has to be
/// attributable.
const CLASH_CORPUS: &[Fixture] = &[
    Fixture {
        name: "eq_diff1_clash",
        doc: &["eq-diff1: two individuals asserted both owl:sameAs and owl:differentFrom."],
        exercises: &["eq-diff1"],
        changed: REFUSAL_GOLDEN,
        quads: &[t(EX_X, OWL_SAMEAS, EX_Y), t(EX_X, OWL_DIFFERENTFROM, EX_Y)],
    },
    Fixture {
        name: "eq_diff1_consistent",
        doc: &[
            "CONTROL for eq-diff1: the owl:differentFrom assertion names z, not y, so the",
            "two assertions are about different pairs and the run closes.",
        ],
        exercises: &["eq-diff1"],
        changed: REFUSAL_GOLDEN,
        quads: &[t(EX_X, OWL_SAMEAS, EX_Y), t(EX_X, OWL_DIFFERENTFROM, EX_Z)],
    },
    Fixture {
        name: "eq_diff2_clash",
        doc: &[
            "eq-diff2: an owl:AllDifferent whose owl:members list holds two members asserted",
            "owl:sameAs. Two members is the smallest list for which the rule's `i ≠ j` side",
            "condition can hold at all.",
        ],
        exercises: &["eq-diff2"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDIFFERENT),
            t(EX_W, OWL_MEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_X),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Y),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, OWL_SAMEAS, EX_Y),
        ],
    },
    Fixture {
        name: "eq_diff2_consistent",
        doc: &[
            "CONTROL for eq-diff2: the equality names z, which is not a member of the list,",
            "so no two members of the owl:AllDifferent are identified.",
        ],
        exercises: &["eq-diff2"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDIFFERENT),
            t(EX_W, OWL_MEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_X),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Y),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, OWL_SAMEAS, EX_Z),
        ],
    },
    Fixture {
        name: "eq_diff3_clash",
        doc: &[
            "eq-diff3: the same as eq-diff2 over owl:distinctMembers. Not redundant — OWL",
            "2's RDF mapping writes an owl:AllDifferent axiom with either property, and a",
            "graph may carry the OWL 1 spelling.",
        ],
        exercises: &["eq-diff3"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDIFFERENT),
            t(EX_W, OWL_DISTINCTMEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_X),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Y),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, OWL_SAMEAS, EX_Y),
        ],
    },
    Fixture {
        name: "eq_diff3_consistent",
        doc: &[
            "CONTROL for eq-diff3: the equality names z, which is not in the",
            "owl:distinctMembers list.",
        ],
        exercises: &["eq-diff3"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDIFFERENT),
            t(EX_W, OWL_DISTINCTMEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_X),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Y),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, OWL_SAMEAS, EX_Z),
        ],
    },
    Fixture {
        name: "prp_irp_clash",
        doc: &["prp-irp: an irreflexive property relating something to itself."],
        exercises: &["prp-irp"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, RDF_TYPE, OWL_IRREFLEXIVEPROPERTY),
            t(EX_X, EX_P, EX_X),
        ],
    },
    Fixture {
        name: "prp_irp_consistent",
        doc: &[
            "CONTROL for prp-irp: the triple relates x to y, so the property is used and",
            "not used reflexively.",
        ],
        exercises: &["prp-irp"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, RDF_TYPE, OWL_IRREFLEXIVEPROPERTY),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "prp_asyp_clash",
        doc: &["prp-asyp: an asymmetric property asserted in both directions."],
        exercises: &["prp-asyp"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, RDF_TYPE, OWL_ASYMMETRICPROPERTY),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_P, EX_X),
        ],
    },
    Fixture {
        name: "prp_asyp_consistent",
        doc: &[
            "CONTROL for prp-asyp: the return direction names z, so no pair is asserted",
            "both ways.",
        ],
        exercises: &["prp-asyp"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, RDF_TYPE, OWL_ASYMMETRICPROPERTY),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "prp_pdw_clash",
        doc: &["prp-pdw: two disjoint properties sharing a subject/object pair."],
        exercises: &["prp-pdw"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, OWL_PROPERTYDISJOINTWITH, EX_Q),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_Q, EX_Y),
        ],
    },
    Fixture {
        name: "prp_pdw_consistent",
        doc: &[
            "CONTROL for prp-pdw: the q-triple's object is z, so the two properties are",
            "both used and share no pair.",
        ],
        exercises: &["prp-pdw"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, OWL_PROPERTYDISJOINTWITH, EX_Q),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_Q, EX_Z),
        ],
    },
    Fixture {
        name: "prp_adp_clash",
        doc: &[
            "prp-adp: an owl:AllDisjointProperties whose owl:members list holds two",
            "properties sharing a subject/object pair.",
        ],
        exercises: &["prp-adp"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDISJOINTPROPERTIES),
            t(EX_W, OWL_MEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_P),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Q),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_Q, EX_Y),
        ],
    },
    Fixture {
        name: "prp_adp_consistent",
        doc: &["CONTROL for prp-adp: the q-triple's object is z, so no pair is shared."],
        exercises: &["prp-adp"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDISJOINTPROPERTIES),
            t(EX_W, OWL_MEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_P),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_Q),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, EX_P, EX_Y),
            t(EX_X, EX_Q, EX_Z),
        ],
    },
    Fixture {
        name: "prp_npa1_clash",
        doc: &[
            "prp-npa1: a negative OBJECT-property assertion whose triple is nevertheless",
            "asserted.",
        ],
        exercises: &["prp-npa1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, OWL_SOURCEINDIVIDUAL, EX_X),
            t(EX_W, OWL_ASSERTIONPROPERTY, EX_P),
            t(EX_W, OWL_TARGETINDIVIDUAL, EX_Y),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "prp_npa1_consistent",
        doc: &[
            "CONTROL for prp-npa1: the asserted triple's object is z, so the assertion the",
            "axiom denies is not made.",
        ],
        exercises: &["prp-npa1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, OWL_SOURCEINDIVIDUAL, EX_X),
            t(EX_W, OWL_ASSERTIONPROPERTY, EX_P),
            t(EX_W, OWL_TARGETINDIVIDUAL, EX_Y),
            t(EX_X, EX_P, EX_Z),
        ],
    },
    Fixture {
        name: "prp_npa2_clash",
        doc: &[
            "prp-npa2: a negative DATA-property assertion whose triple is nevertheless",
            "asserted. The target is a literal, which is what separates this rule from",
            "prp-npa1.",
        ],
        exercises: &["prp-npa2"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, OWL_SOURCEINDIVIDUAL, EX_X),
            t(EX_W, OWL_ASSERTIONPROPERTY, EX_P),
            t_lit(EX_W, OWL_TARGETVALUE, "cat"),
            t_lit(EX_X, EX_P, "cat"),
        ],
    },
    Fixture {
        name: "prp_npa2_consistent",
        doc: &[
            "CONTROL for prp-npa2: the asserted value is \"dog\", which is a different data",
            "value from the \"cat\" the axiom denies, so dt-eq does not identify the two",
            "and the run closes.",
        ],
        exercises: &["prp-npa2"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, OWL_SOURCEINDIVIDUAL, EX_X),
            t(EX_W, OWL_ASSERTIONPROPERTY, EX_P),
            t_lit(EX_W, OWL_TARGETVALUE, "cat"),
            t_lit(EX_X, EX_P, "dog"),
        ],
    },
    Fixture {
        name: "cls_nothing2_clash",
        doc: &["cls-nothing2: an instance of owl:Nothing, the empty class."],
        exercises: &["cls-nothing2"],
        changed: REFUSAL_GOLDEN,
        quads: &[t(EX_X, RDF_TYPE, OWL_NOTHING)],
    },
    Fixture {
        name: "cls_nothing2_consistent",
        doc: &[
            "CONTROL for cls-nothing2: an instance of owl:Thing rather than owl:Nothing —",
            "the one term the rule reads.",
        ],
        exercises: &["cls-nothing2"],
        changed: REFUSAL_GOLDEN,
        quads: &[t(EX_X, RDF_TYPE, OWL_THING)],
    },
    Fixture {
        name: "cls_com_clash",
        doc: &["cls-com: something typed by a class and by its complement."],
        exercises: &["cls-com"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_A, OWL_COMPLEMENTOF, EX_B),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_X, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "cls_com_consistent",
        doc: &[
            "CONTROL for cls-com: the second typing is about y, so both classes are",
            "inhabited and neither instance is in both.",
        ],
        exercises: &["cls-com"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_A, OWL_COMPLEMENTOF, EX_B),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_Y, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "cls_maxc1_clash",
        doc: &[
            "cls-maxc1: a p-value on an instance of an owl:maxCardinality 0 restriction.",
            "The cardinality literal is \"0\"^^xsd:nonNegativeInteger, exactly as OWL 2",
            "Profiles Table 6 writes it.",
        ],
        exercises: &["cls-maxc1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t_lit_dt(EX_E, OWL_MAXCARDINALITY, "0", XSD_NONNEGATIVEINTEGER),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "cls_maxc1_consistent",
        doc: &[
            "CONTROL for cls-maxc1: the cardinality is \"1\", so the same input is an",
            "ordinary cls-maxc2 premise instead — the one term the rule reads, and the",
            "clearest evidence that the literal is matched rather than parsed loosely.",
        ],
        exercises: &["cls-maxc1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t_lit_dt(EX_E, OWL_MAXCARDINALITY, "1", XSD_NONNEGATIVEINTEGER),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "cls_maxqc1_clash",
        doc: &[
            "cls-maxqc1: a p-value TYPED BY THE QUALIFYING CLASS on an instance of an",
            "owl:maxQualifiedCardinality 0 restriction.",
        ],
        exercises: &["cls-maxqc1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "0",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, EX_C),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_Y, RDF_TYPE, EX_C),
        ],
    },
    Fixture {
        name: "cls_maxqc1_consistent",
        doc: &[
            "CONTROL for cls-maxqc1: the qualifying typing is about z rather than about the",
            "p-value y, so the restriction counts no qualifying value.",
        ],
        exercises: &["cls-maxqc1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "0",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, EX_C),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
            t(EX_Z, RDF_TYPE, EX_C),
        ],
    },
    Fixture {
        name: "cls_maxqc2_clash",
        doc: &[
            "cls-maxqc2: the owl:Thing-qualified form of cls-maxqc1, where ANY p-value on an",
            "instance clashes because no typing of the value is required.",
        ],
        exercises: &["cls-maxqc2"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "0",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, OWL_THING),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "cls_maxqc2_consistent",
        doc: &[
            "CONTROL for cls-maxqc2: the qualifying class is C rather than owl:Thing, and",
            "nothing types the p-value a C, so neither cls-maxqc2 (which requires the",
            "owl:Thing constant) nor cls-maxqc1 (which requires the typing) has a premise.",
        ],
        exercises: &["cls-maxqc2"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t_lit_dt(
                EX_E,
                OWL_MAXQUALIFIEDCARDINALITY,
                "0",
                XSD_NONNEGATIVEINTEGER,
            ),
            t(EX_E, OWL_ONPROPERTY, EX_P),
            t(EX_E, OWL_ONCLASS, EX_C),
            t(EX_X, RDF_TYPE, EX_E),
            t(EX_X, EX_P, EX_Y),
        ],
    },
    Fixture {
        name: "cax_dw_clash",
        doc: &["cax-dw: two classes declared owl:disjointWith sharing an instance."],
        exercises: &["cax-dw"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_A, OWL_DISJOINTWITH, EX_B),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_X, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "cax_dw_consistent",
        doc: &[
            "CONTROL for cax-dw: the second typing is about y, so the disjoint classes are",
            "both inhabited and share nothing.",
        ],
        exercises: &["cax-dw"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_A, OWL_DISJOINTWITH, EX_B),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_Y, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "cax_adc_clash",
        doc: &[
            "cax-adc: an owl:AllDisjointClasses whose owl:members list holds two classes",
            "sharing an instance.",
        ],
        exercises: &["cax-adc"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDISJOINTCLASSES),
            t(EX_W, OWL_MEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_X, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "cax_adc_consistent",
        doc: &["CONTROL for cax-adc: the second typing is about y, so no instance is shared."],
        exercises: &["cax-adc"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_W, RDF_TYPE, OWL_ALLDISJOINTCLASSES),
            t(EX_W, OWL_MEMBERS, EX_L0),
            t(EX_L0, RDF_FIRST, EX_A),
            t(EX_L0, RDF_REST, EX_L1),
            t(EX_L1, RDF_FIRST, EX_B),
            t(EX_L1, RDF_REST, RDF_NIL),
            t(EX_X, RDF_TYPE, EX_A),
            t(EX_Y, RDF_TYPE, EX_B),
        ],
    },
    Fixture {
        name: "dt_not_type_clash",
        doc: &[
            "dt-not-type: a literal whose lexical form is outside the value space of the",
            "datatype IT ITSELF CARRIES — the ill-typed-literal clash. Read without that",
            "qualification the rule would make every graph holding a string inconsistent,",
            "because a string is not in xsd:integer's value space; the datatype in question",
            "is the literal's own.",
        ],
        exercises: &["dt-not-type"],
        changed: REFUSAL_GOLDEN,
        quads: &[t_lit_dt(EX_X, EX_P, "cat", XSD_INTEGER)],
    },
    Fixture {
        name: "dt_not_type_consistent",
        doc: &[
            "CONTROL for dt-not-type: the same triple with a lexical form that IS in",
            "xsd:integer's value space.",
        ],
        exercises: &["dt-not-type"],
        changed: REFUSAL_GOLDEN,
        quads: &[t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER)],
    },
    Fixture {
        name: "dt_diff_clash",
        doc: &[
            "dt-diff, whose witness is eq-diff1 rather than itself. dt-diff CONCLUDES —",
            "`lt1 owl:differentFrom lt2` — so it cannot refuse a run on its own; what it",
            "does is supply eq-diff1's second premise.",
            "",
            "A functional property with two value-DIFFERENT values makes prp-fp conclude",
            "`\"1\" owl:sameAs \"2\"`, dt-diff then concludes",
            "`\"1\" owl:differentFrom \"2\"` over the same pair, and eq-diff1 has both of",
            "its premises. Every one of those intermediate conclusions has a literal",
            "subject and none of them can be materialized, which is exactly why the rule",
            "needs a refusal to be observable at all.",
        ],
        exercises: &["dt-diff", "prp-fp", "eq-diff1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, RDF_TYPE, OWL_FUNCTIONALPROPERTY),
            t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER),
            t_lit_dt(EX_X, EX_P, "2", XSD_INTEGER),
        ],
    },
    Fixture {
        name: "dt_diff_consistent",
        doc: &[
            "CONTROL for dt-diff: \"01\"^^xsd:integer is the SAME data value as",
            "\"1\"^^xsd:integer, so prp-fp's owl:sameAs is licensed by dt-eq as well and",
            "dt-diff has nothing to conclude. The one term that changed is the lexical form.",
        ],
        exercises: &["dt-diff", "prp-fp", "eq-diff1"],
        changed: REFUSAL_GOLDEN,
        quads: &[
            t(EX_P, RDF_TYPE, OWL_FUNCTIONALPROPERTY),
            t_lit_dt(EX_X, EX_P, "1", XSD_INTEGER),
            t_lit_dt(EX_X, EX_P, "01", XSD_INTEGER),
        ],
    },
];

// ── Building and running a fixture ──────────────────────────────────────────────

/// Intern one fixture object term into `builder`.
fn intern_object(builder: &mut RdfDatasetBuilder, term: Term) -> TermId {
    match term {
        Term::Iri(iri) => builder.intern_iri(iri),
        Term::Literal { lexical, datatype } => {
            builder.intern_literal(RdfLiteral::typed(lexical, datatype))
        }
        Term::Quoted(s, p, o) => {
            let s = builder.intern_iri(s);
            let p = builder.intern_iri(p);
            let o = builder.intern_iri(o);
            builder.intern_triple(s, p, o)
        }
    }
}

/// Freeze `fixture`'s quads into a dataset.
fn build(fixture: &Fixture) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for quad in fixture.quads {
        let s = builder.intern_iri(quad.s);
        let p = builder.intern_iri(quad.p);
        let o = intern_object(&mut builder, quad.o);
        let g = quad.g.map(|g| builder.intern_iri(g));
        builder.push_quad(s, p, o, g);
    }
    builder.freeze().expect("fixture dataset freezes")
}

/// The fixture named `name`, from either table.
///
/// [`CORPUS`] and [`CLASH_CORPUS`] partition the fixtures by what kind of evidence they
/// carry — a closure or a refusal — but they are one namespace, and
/// `the_goldens_directory_is_exactly_the_corpus` asserts that no name is in both.
fn fixture(name: &str) -> &'static Fixture {
    CORPUS
        .iter()
        .chain(CLASH_CORPUS)
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no fixture named {name} in the corpus"))
}

/// The canonical N-Quads lines of `regime`'s closure over the fixture named `name`.
fn closure_lines(name: &str, regime: Regime) -> BTreeSet<String> {
    let ds = build(fixture(name));
    let (closed, _report) = materialize(&ds, regime).expect("the five oracle regimes run");
    canonicalize(&closed)
        .nquads
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

/// One default-graph triple over three IRIs, in the canonical N-Quads spelling.
fn nquads_line(s: &str, p: &str, o: &str) -> String {
    format!("<{s}> <{p}> <{o}> .")
}

// ── Rendering a golden ──────────────────────────────────────────────────────────

/// The five regimes `materialize` can run, with the names the goldens use.
///
/// `OWL-Direct` and `RIF` are refused by the façade — one needs the query's class
/// expressions and the other a parsed rule set — so an oracle over `materialize` cannot and
/// must not include them.
///
/// `D` was refused for the same kind of reason until Table 8 was stated, and it is not any
/// more: `entailment/D` IS datatype entailment, this crate realizes it as Simple entailment
/// plus the five `dt-*` rules, and `materialize` runs it. Leaving it out would make the
/// module's own claim — that a golden holds every regime `materialize` can run — false, and
/// would leave the newest lane in the crate the only one nothing pins.
const ORACLE_REGIMES: [(Regime, &str); 5] = [
    (Regime::Simple, "Simple"),
    (Regime::Rdf, "RDF"),
    (Regime::Rdfs, "RDFS"),
    (Regime::OwlRl, "OWL-RL"),
    (Regime::D, "D"),
];

/// How many space-separated tokens a wrapped list puts on one line.
///
/// A fixed count rather than a column budget: the wrap must not depend on how long a rule
/// id happens to be, or renaming one would reflow every golden.
const TOKENS_PER_LINE: usize = 8;

/// Append `tokens` under `label`, wrapped at [`TOKENS_PER_LINE`] with a fixed indent.
fn write_wrapped(out: &mut String, indent: &str, label: &str, tokens: &[String]) {
    let _ = writeln!(out, "{indent}{label} ({})", tokens.len());
    for chunk in tokens.chunks(TOKENS_PER_LINE) {
        let _ = writeln!(out, "{indent}    {}", chunk.join(" "));
    }
}

/// Render `report` deterministically, field by field, in the report's documented order.
///
/// Every sequence a report carries already has a fixed order — missing and fired rules in
/// specification table order, boundaries in `Construct` declaration order — so this
/// function adds no sorting of its own. A boundary is rendered by name only: its reason is
/// a pure function of the construct (`Boundary::of` is the only constructor), pinned by the
/// crate's unit tests, and repeating a paragraph of prose in thirty golden files would make
/// the goldens harder to diff without making them say more.
///
/// `completeness` is rendered as three distinct words rather than the two
/// [`Completeness::is_exact`] collapses to. `exact` and `exact-within-boundaries` differ in
/// whether the run met a construct OUTSIDE the rule table, which is precisely the
/// distinction the third variant exists to make visible; a golden that printed one word for
/// both could not show a lane going from complete-and-unobstructed to
/// complete-but-bounded, which is what the `OWL-RL` lane just did.
fn render_report(out: &mut String, report: &ReasoningReport) {
    let indent = "  ";
    let _ = writeln!(
        out,
        "{indent}completeness: {}",
        match report.completeness() {
            Completeness::Exact => "exact",
            Completeness::ExactWithinBoundaries => "exact-within-boundaries",
            Completeness::SoundIncomplete { .. } => "sound-incomplete",
        }
    );
    let missing: Vec<String> = report
        .completeness()
        .missing()
        .iter()
        .map(|rule| rule.as_str().to_owned())
        .collect();
    write_wrapped(out, indent, "missing:", &missing);
    let fired: Vec<String> = report
        .rules_fired()
        .iter()
        .map(|&(rule, count)| format!("{}={count}", rule.as_str()))
        .collect();
    write_wrapped(out, indent, "rules-fired:", &fired);
    let boundaries: Vec<String> = report
        .boundaries()
        .iter()
        .map(|boundary| boundary.construct().as_str().to_owned())
        .collect();
    write_wrapped(out, indent, "boundaries:", &boundaries);
    // The ONLY observable trace of `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a`. All four
    // fire, and every conclusion they reach mentions a surrogate blank node, which a SPARQL
    // entailment regime may not answer with — so none of them can ever appear in
    // `rules-fired`, and without this line a reader of the golden could not tell a lane
    // that fires them from one that does not. It is a measurement of the run, so it is zero
    // for the three lanes that state none of the four.
    let _ = writeln!(
        out,
        "{indent}withheld-surrogates: {}",
        report.withheld_surrogates()
    );
    let budget = report.budget();
    let _ = writeln!(
        out,
        "{indent}budget: join-steps={} stored-facts={} term-arena-bytes={}",
        budget.join_steps(),
        budget.stored_facts(),
        budget.term_arena_bytes()
    );
    let _ = writeln!(out, "{indent}contract-hash: {}", report.contract_hash());
    let _ = writeln!(
        out,
        "{indent}inconsistency: {}",
        report
            .inconsistency()
            .map_or_else(|| "none".to_owned(), |w| w.rule().as_str().to_owned())
    );
    let _ = writeln!(out, "{indent}overclaims: {}", report.overclaims());
}

/// Append `lines` as `#`-prefixed header comment lines, with a bare `#` for a blank.
fn write_comment_block(out: &mut String, lines: &[&str]) {
    for line in lines {
        if line.is_empty() {
            out.push_str("#\n");
        } else {
            let _ = writeln!(out, "# {line}");
        }
    }
}

/// Append a canonical N-Quads block under a `--- label (N lines) ---` banner.
fn write_nquads(out: &mut String, label: &str, nquads: &str) {
    let count = nquads.lines().count();
    let _ = writeln!(out, "--- {label} ({count} lines) ---");
    out.push_str(nquads);
}

/// Render the whole golden for `fixture`: the header, the input, and every regime's
/// closure and report.
fn render_golden(fixture: &Fixture) -> String {
    let mut out = String::new();
    out.push_str(
        "# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. \
         <paudley@blackcatinformatics.ca>\n\
         # SPDX-License-Identifier: MIT OR Apache-2.0\n\
         #\n\
         # GOLDEN — generated by crates/entail/tests/oracle.rs. Do not hand-edit.\n\
         # Regenerate deliberately with:\n\
         #   cargo test -p purrdf-entail --test oracle -- --ignored --exact \
         regenerate_goldens\n\
         #\n",
    );
    let _ = writeln!(out, "# fixture: {}", fixture.name);
    write_comment_block(&mut out, fixture.doc);
    out.push_str("#\n");
    write_comment_block(&mut out, ENGINE_SWAP);
    out.push_str("#\n");
    write_comment_block(&mut out, AXIOMATIC_PATH);
    out.push_str("#\n");
    write_comment_block(&mut out, OWL_RL_TABLES);
    out.push_str("#\n");
    write_comment_block(&mut out, OWL_RL_COMPLETE);
    out.push_str("#\n");
    write_comment_block(&mut out, EXISTENTIAL_CHASE);
    out.push_str("#\n");
    write_comment_block(&mut out, REPORT_SURFACE);
    out.push_str("#\n# WHAT MOVED IN THIS GOLDEN:\n#\n");
    write_comment_block(&mut out, fixture.changed);
    let _ = writeln!(out, "# exercises: {}", fixture.exercises.join(" "));
    out.push('\n');

    let ds = build(fixture);
    write_nquads(&mut out, "input", &canonicalize(&ds).nquads);

    for (regime, name) in ORACLE_REGIMES {
        let _ = writeln!(out, "\n=== regime {name} ===");
        match materialize(&ds, regime) {
            Ok((closed, report)) => {
                write_nquads(&mut out, "closure", &canonicalize(&closed).nquads);
                out.push_str("--- report ---\n");
                render_report(&mut out, &report);
            }
            // A REFUSAL IS A RESULT, AND IT HAS A REPORT. An inconsistent knowledge base
            // entails every triple, so there is no closure to render — but the run happened,
            // and `EntailError::Inconsistent` carries what it did. Rendering it is what lets
            // the clash corpus have goldens at all, and it is the only place the oracle can
            // show `inconsistency:` naming a rule instead of reading `none`.
            Err(EntailError::Inconsistent(run)) => {
                let _ = writeln!(
                    out,
                    "--- refused: inconsistent ({} premises, graph {}) ---",
                    run.witness().premises().len(),
                    run.witness()
                        .graph()
                        .map_or_else(|| "default".to_owned(), surface_of)
                );
                for premise in run.witness().premises() {
                    let _ = writeln!(
                        out,
                        "  premise: {} {} {}",
                        surface_of(premise.subject()),
                        surface_of(premise.predicate()),
                        surface_of(premise.object())
                    );
                }
                out.push_str("--- report ---\n");
                render_report(&mut out, run.report());
            }
            Err(other) => panic!("{name}: unexpected refusal: {other}"),
        }
    }
    out
}

/// A witness term's N-Triples-shaped surface, for the refusal block above.
///
/// The premises are [`TermValue`]s rather than dataset ids — a witness outlives the dataset
/// it was drawn from — so the golden renders them itself rather than reaching for the
/// canonical serializer, which needs a dataset. Exhaustive on the four variants, so a fifth
/// term kind fails to compile here rather than rendering as a debug blob.
fn surface_of(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => format!("<{iri}>"),
        TermValue::Blank { label, .. } => format!("_:{label}"),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            let mut out = format!("{lexical_form:?}");
            if let Some(tag) = language {
                out.push('@');
                out.push_str(tag);
                if let Some(dir) = direction {
                    let _ = write!(out, "--{dir:?}");
                }
            } else {
                let _ = write!(out, "^^<{datatype}>");
            }
            out
        }
        TermValue::Triple { s, p, o } => format!(
            "<<( {} {} {} )>>",
            surface_of(s),
            surface_of(p),
            surface_of(o)
        ),
    }
}

/// The directory the goldens live in.
fn goldens_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
}

/// The golden path for `name`.
fn golden_path(name: &str) -> PathBuf {
    goldens_dir().join(format!("{name}.golden"))
}

// ── The oracle gate ─────────────────────────────────────────────────────────────

/// Every fixture of BOTH tables, in one order, for the golden gate to walk.
///
/// [`CLASH_CORPUS`] joined [`CORPUS`] here when the refusal started carrying a report:
/// [`render_golden`] can render a refused run now, so the reason those fixtures had no
/// golden — "there is no closure and no report to write one from" — stopped being true.
fn golden_fixtures() -> impl Iterator<Item = &'static Fixture> {
    CORPUS.iter().chain(CLASH_CORPUS)
}

/// THE ORACLE. Every fixture's committed golden equals what the engine produces now.
///
/// This is the test the engine swap has to survive. A failure here is not a flaky
/// assertion: it means the closure or the report changed for an input the corpus covers.
/// Either that change is the point of the commit — in which case regenerate, and the diff
/// is the evidence a reviewer reads — or it is a regression.
#[test]
fn goldens_match_the_current_engine() {
    let mut mismatched = Vec::new();
    for fixture in golden_fixtures() {
        let path = golden_path(fixture.name);
        let Ok(committed) = std::fs::read_to_string(&path) else {
            mismatched.push(format!("{}: no committed golden", fixture.name));
            continue;
        };
        let rendered = render_golden(fixture);
        if committed != rendered {
            let first_diff = committed
                .lines()
                .zip(rendered.lines())
                .position(|(a, b)| a != b)
                .map_or_else(
                    || "length differs".to_owned(),
                    |index| {
                        let line = index + 1;
                        let committed_line = committed.lines().nth(index).unwrap_or("");
                        let rendered_line = rendered.lines().nth(index).unwrap_or("");
                        format!("line {line}:\n    committed: {committed_line}\n    now:       {rendered_line}")
                    },
                );
            mismatched.push(format!("{}: {first_diff}", fixture.name));
        }
    }
    assert!(
        mismatched.is_empty(),
        "the closure or the report changed for {} fixture(s):\n{}\n\nIf the change is \
         deliberate, regenerate with:\n  cargo test -p purrdf-entail --test oracle -- \
         --ignored --exact regenerate_goldens",
        mismatched.len(),
        mismatched.join("\n")
    );
}

/// Maintainer-only: rewrite every committed golden from the current engine.
///
/// `#[ignore]`d so a normal test run can only ever compare. Rewriting the oracle is a
/// deliberate act whose whole value is the diff it produces, so it must be typed on
/// purpose and reviewed.
#[test]
#[ignore = "maintainer-only: rewrites the committed goldens; run deliberately"]
fn regenerate_goldens() {
    std::fs::create_dir_all(goldens_dir()).expect("create the goldens directory");
    for fixture in golden_fixtures() {
        std::fs::write(golden_path(fixture.name), render_golden(fixture))
            .expect("write the golden");
    }
}

/// Rendering is a pure function of the fixture: two renders in one process agree byte for
/// byte, for every fixture and therefore for every regime.
///
/// Cheap, but it is the property the goldens rest on. `materialize` seeds a freshly-hashed
/// fact set per call, so an order-sensitive emission would show up here as two different
/// strings from the same input.
#[test]
fn rendering_is_byte_stable_within_a_run() {
    for fixture in golden_fixtures() {
        assert_eq!(
            render_golden(fixture),
            render_golden(fixture),
            "{} rendered differently twice in one process",
            fixture.name
        );
    }
}

/// The goldens directory holds exactly [`CORPUS`] plus [`CLASH_CORPUS`], and nothing else.
///
/// Without this, deleting a fixture would leave a golden nobody compares — an oracle that
/// looks larger than it is.
///
/// [`CLASH_CORPUS`] used to be excluded, on the ground that a refused run has no closure
/// and therefore nothing to write a golden from. Half of that was always true and the
/// other half stopped being true: the refusal now carries the run's
/// [`purrdf_entail::ReasoningReport`], so those thirty-six fixtures have goldens showing
/// the four regimes that close normally AND the refusal — its witness, its premises, and
/// the only reports in this corpus whose `inconsistency:` line names a rule.
#[test]
fn the_goldens_directory_is_exactly_the_corpus() {
    let expected: BTreeSet<String> = golden_fixtures()
        .map(|fixture| format!("{}.golden", fixture.name))
        .collect();
    assert_eq!(
        expected.len(),
        CORPUS.len() + CLASH_CORPUS.len(),
        "two fixtures share a name"
    );
    let found: BTreeSet<String> = std::fs::read_dir(goldens_dir())
        .expect("read the goldens directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(found, expected);
    for fixture in CLASH_CORPUS {
        // A `changed` field is a claim about a golden, and every clash fixture makes the
        // same one — that its golden is a REFUSAL golden — in the same words, rather than
        // each inventing a way to say it.
        assert_eq!(
            fixture.changed, REFUSAL_GOLDEN,
            "{}: a CLASH_CORPUS fixture states what its golden shows, verbatim",
            fixture.name
        );
    }
}

/// A refusal golden is a refusal: at least one regime's section is the refused shape, and
/// its report names the rule that refused.
///
/// The half of the oracle a byte-comparison cannot state. Without it a change that made
/// every clash fixture close normally would regenerate thirty-six goldens full of closures
/// and pass, because the goldens would still equal what the engine produces.
#[test]
fn every_clash_golden_shows_a_refusal_with_a_named_witness() {
    // `CLASH_CORPUS` holds both halves of every refuting control, so the split is read from
    // the registry rather than guessed from the table: a `clashes` fixture must refuse, and
    // its `consistent` twin must not. Asserting only the first half would pass a corpus in
    // which every input had become inconsistent.
    let mut clashing: BTreeSet<&str> = BTreeSet::new();
    let mut controls: BTreeSet<&str> = BTreeSet::new();
    for regime in REGISTRY_REGIMES {
        for &(_, clashes, _, consistent) in refuting_rows(regime) {
            clashing.insert(clashes);
            controls.insert(consistent);
        }
    }
    for fixture in CLASH_CORPUS {
        let rendered = render_golden(fixture);
        let refused = rendered.contains("--- refused: inconsistent (");
        if clashing.contains(fixture.name) {
            assert!(refused, "{}: no regime refused", fixture.name);
            assert!(
                rendered.lines().any(|line| {
                    line.starts_with("  inconsistency: ") && line != "  inconsistency: none"
                }),
                "{}: a refused run's report must name the rule that refused",
                fixture.name
            );
            assert!(
                rendered.contains("  premise: "),
                "{}: a witness names the asserted triples that satisfied the rule",
                fixture.name
            );
        } else {
            assert!(
                controls.contains(fixture.name),
                "{}: a CLASH_CORPUS fixture is either a clash or its control",
                fixture.name
            );
            assert!(
                !refused,
                "{}: the control of a refusing fixture must close",
                fixture.name
            );
        }
    }
}

/// Every rule id a fixture of either table claims to exercise is a real rule id.
#[test]
fn fixture_exercise_lists_name_real_rules() {
    for fixture in CORPUS.iter().chain(CLASH_CORPUS) {
        for spelling in fixture.exercises {
            let rule = RuleId::from_str(spelling)
                .unwrap_or_else(|_| panic!("{}: {spelling} is not a rule id", fixture.name));
            assert_eq!(
                rule.as_str(),
                *spelling,
                "{}: use the canonical spelling",
                fixture.name
            );
        }
    }
}

// ── The rule fixture registry ───────────────────────────────────────────────────

/// One conclusion the registry looks for, as a canonical N-Quads line.
///
/// Two constructors rather than one, because the corpus's terms are `&'static str`
/// constants and a canonical line cannot be assembled from them at compile time. Every
/// conclusion is a DEFAULT-GRAPH triple whose subject and predicate are IRIs — no rule of
/// any lane concludes otherwise, and one that could would be a generalized-RDF triple the
/// RDF 1.2 IR cannot hold — so the only position that needs a sum type is the object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Conclusion {
    /// A triple over three IRIs — the common case.
    Iris(&'static str, &'static str, &'static str),
    /// A triple whose object is an RDF 1.2 TRIPLE TERM over three IRIs.
    ///
    /// `prp-spo1` and `rdfs7` rewrite a triple's PREDICATE and copy its object through
    /// whatever kind of term that object is, so a rule applied to an `rdf:reifies`
    /// annotation triple concludes into this shape. The chase never looks inside the term
    /// — that is the triple-term boundary — and this is what "carried through unchanged"
    /// looks like as a checkable line.
    Quoted {
        /// The subject IRI.
        subject: &'static str,
        /// The predicate IRI.
        predicate: &'static str,
        /// The quoted triple's subject IRI.
        qs: &'static str,
        /// The quoted triple's predicate IRI.
        qp: &'static str,
        /// The quoted triple's object IRI.
        qo: &'static str,
    },
    /// A triple whose object is a datatyped literal, which `cls-hv1` concludes whenever the
    /// `owl:hasValue` is one, and which `dt-eq`'s downstream `eq-rep-o` conclusion always
    /// is.
    Literal {
        /// The subject IRI.
        subject: &'static str,
        /// The predicate IRI.
        predicate: &'static str,
        /// The object literal's lexical form.
        lexical: &'static str,
        /// The object literal's datatype IRI.
        datatype: &'static str,
    },
}

impl Conclusion {
    /// The canonical N-Quads line this conclusion is, exactly as `purrdf_core::canonicalize`
    /// writes it — including RDF 1.1 C0.1's rule that an `xsd:string` literal leaves its
    /// datatype implicit, which is why the datatype is not printed unconditionally.
    fn line(self) -> String {
        match self {
            Self::Iris(s, p, o) => format!("<{s}> <{p}> <{o}> ."),
            Self::Quoted {
                subject,
                predicate,
                qs,
                qp,
                qo,
            } => format!("<{subject}> <{predicate}> <<( <{qs}> <{qp}> <{qo}> )>> ."),
            Self::Literal {
                subject,
                predicate,
                lexical,
                datatype,
            } => {
                if datatype == XSD_STRING {
                    format!("<{subject}> <{predicate}> \"{lexical}\" .")
                } else {
                    format!("<{subject}> <{predicate}> \"{lexical}\"^^<{datatype}> .")
                }
            }
        }
    }
}

/// One side of a rule's evidence: a fixture, and the conclusion to look for in its closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Case {
    /// The fixture's name.
    fixture: &'static str,
    /// The conclusion, as the canonical N-Quads line it must be (or must not be).
    conclusion: Conclusion,
}

/// What the corpus can say about one rule of one regime.
///
/// Five states, and the gaps between them are the point. `NotYetImplemented` is a
/// COMPLETE entry: it is a true, checked statement that the chase does not fire this rule,
/// asserted against the inventory rather than assumed. A rule with no entry at all is not
/// a state this type can express, which is what makes "one fixture per rule" hold.
///
/// The three states beyond `Registered` exist because three families of rule have no
/// conclusion the ordinary present/absent control can read: a premise-free rule holds for
/// every input, a rule that concludes `false` produces no closure at all, and a rule whose
/// every conclusion is generalized RDF produces nothing the RDF 1.2 IR can hold. Each is
/// evidenced by the strongest thing that IS observable about it, and none of them is
/// weakened to a bare "it is declared".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleFixtures {
    /// The chase fires this rule, and the corpus proves it both ways.
    Registered {
        /// An input where the rule fires: the conclusion must be PRESENT.
        positive: Case,
        /// The same input, changed in exactly the way that removes the rule's premise:
        /// the same conclusion must be ABSENT.
        near_miss: Case,
    },
    /// The chase fires this rule with NO premise, so the two-fixture control above is
    /// not merely unwritten, it is impossible: the conclusion holds for EVERY input, and
    /// an input that denied it would be a bug in the rule rather than a near miss.
    ///
    /// The evidence is therefore rearranged rather than weakened. The presence side is
    /// asserted over the EMPTY dataset, which is the strongest witness available — a
    /// premise-free rule that fires on nothing at all is fired on everything — and the
    /// absence side moves inside that same input: a conclusion of the same SHAPE, over a
    /// term the rule must NOT range over, which is what keeps "the rule fires" from
    /// degenerating into "something types everything".
    Axiomatic {
        /// The empty dataset, and the conclusion that must be PRESENT in its closure.
        holds: Case,
        /// The same dataset, and a same-shaped conclusion that must be ABSENT from it.
        denied: Case,
    },
    /// The rule's conclusion is `false`, so a body match REFUSES the run rather than adding
    /// a triple, and the evidence is a pair of RUNS rather than a pair of closures.
    ///
    /// `materialize(.., Regime::OwlRl)` over `clashes` must return
    /// [`purrdf_entail::EntailError::Inconsistent`] carrying a witness that names `witness`;
    /// over `consistent` it must return a closure. The `witness` id is normally the rule's
    /// own — the rule that refused is the rule the witness names — with exactly one
    /// exception, and it is a real fact about the rule rather than a bookkeeping one:
    /// `dt-diff` CONCLUDES `owl:differentFrom` rather than `false`, so it can never refuse
    /// a run itself. What it does is supply `eq-diff1`'s second premise, and every one of
    /// its own conclusions has a literal subject and is dropped at the materialization
    /// boundary. `eq-diff1`'s refusal on an input where dt-diff is the only possible source
    /// of that premise is therefore the only observation that `dt-diff` fired at all.
    Refuting {
        /// An input the rule refuses: `materialize` must fail with an inconsistency.
        clashes: &'static str,
        /// The rule the witness must name — the rule itself, except for `dt-diff`.
        witness: RuleId,
        /// The same input, changed in exactly the way that removes the rule's premise:
        /// `materialize` must succeed.
        consistent: &'static str,
    },
    /// EVERY conclusion of the rule is generalized RDF — a literal in subject position —
    /// so none of them can be materialized and the rule can never be credited in a closure.
    ///
    /// The evidence moves one hop DOWNSTREAM, to a triple the rule licenses and the RDF 1.2
    /// IR can hold, plus the [`purrdf_entail::Construct::GeneralizedRdf`] boundary on the
    /// positive run. The boundary is what separates "the rule fired and its conclusion was
    /// abandoned" from "the rule never fired": without it, a chase that had simply dropped
    /// the rule would still pass the downstream assertion if any other path reached the same
    /// triple, and with it the run has to have derived something it could not represent.
    Generalized {
        /// An input where the rule fires: the DOWNSTREAM conclusion must be PRESENT, and
        /// the run must report the generalized-rdf boundary.
        positive: Case,
        /// The same input, changed in exactly the way that removes the rule's premise:
        /// the same downstream conclusion must be ABSENT.
        near_miss: Case,
    },
    /// EVERY conclusion of the rule mentions a SURROGATE blank node the chase invented, so
    /// none of them reaches the answer and the rule can never be credited in a closure.
    ///
    /// The four rules `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` conclude about a fresh
    /// `_:nnn`, and a SPARQL entailment regime draws its answers from the scoping graph —
    /// so a solution binding a variable to a surrogate is not an answer it admits and every
    /// conclusion mentioning one is dropped at the materialization boundary. The W3C case
    /// `rdfs13` is the proof that this is required rather than convenient: it asks
    /// `?L rdf:type rdfs:Literal` over a graph whose only literal is `"foo"` and demands
    /// ZERO rows, which `rdfD1`'s surrogate would otherwise supply through `rdfs1`,
    /// `rdfs13` and `rdfs9`.
    ///
    /// So the evidence is the WITHHELD COUNT — the one thing a caller can observe about a
    /// rule whose every conclusion is withheld — and it is a two-sided control like every
    /// other state here: `positive`'s run withholds strictly more than `baseline`'s. For a
    /// PREMISE-FREE rule there is no input that denies it, so `baseline` is `None` and the
    /// comparison is against zero over the EMPTY dataset, exactly as
    /// [`RuleFixtures::Axiomatic`] moves its own control inside one input.
    Withheld {
        /// A fixture whose run withholds surrogate conclusions.
        positive: &'static str,
        /// The same input without the term the rule observes; `None` for a premise-free
        /// rule, whose positive fixture is the empty dataset and whose comparison is
        /// against zero.
        baseline: Option<&'static str>,
    },
    /// The chase does not fire this rule, so the corpus has nothing to show.
    NotYetImplemented,
}

/// A registry row: the rule, its positive fixture and conclusion, and its near-miss
/// fixture. The conclusion is shared — a near miss asserts the ABSENCE of the very triple
/// the positive asserts the presence of, which is what makes the pair a control.
type Row = (RuleId, &'static str, Conclusion, &'static str);

/// `Regime::Rdf`'s registered rules.
///
/// `named_graph` is `rdfD2`'s near miss, and under the defined dataset semantics it is a
/// SHARPER one than it used to be. Its only predicate sits in `ex:g`, each named graph is
/// closed against the union of itself and the default graph, and a conclusion lands in the
/// graph that produced it — so `ex:p rdf:type rdf:Property` IS drawn, in `ex:g`, and the
/// DEFAULT-graph line this row denies is absent because the routing put the conclusion
/// where it belongs. The control therefore witnesses the routing rather than a rule that
/// failed to fire, and `a_named_graph_is_closed_and_its_conclusions_land_in_it` asserts both
/// halves so neither can quietly become the other.
const RDF_ROWS: &[Row] = &[(
    RuleId::RdfD2,
    "plain_triple",
    Conclusion::Iris(EX_P, RDF_TYPE, RDF_PROPERTY),
    "named_graph",
)];

/// A premise-free rule's registry row: the rule, the fixture (always the empty dataset),
/// the conclusion that must be PRESENT in its closure, and a same-shaped conclusion that
/// must be ABSENT from it. See [`RuleFixtures::Axiomatic`] for why the control moves
/// inside one fixture instead of across two.
type AxiomaticRow = (RuleId, &'static str, Conclusion, Conclusion);

/// A refuting rule's registry row: the rule, the fixture whose run must be REFUSED, the
/// rule the inconsistency witness must name, and the fixture whose run must succeed. See
/// [`RuleFixtures::Refuting`], including why `dt-diff`'s witness is `eq-diff1`.
type RefutingRow = (RuleId, &'static str, RuleId, &'static str);

/// The premise-free rules the `RDFS` lane fires.
///
/// `rdfs1` types every IRI of the recognized datatype set `D` an `rdfs:Datatype`, and
/// RDF 1.2 Semantics §8 fixes `D` — for the unqualified phrase "RDFS entails" — at
/// `{rdf:langString, rdf:dirLangString, xsd:string}`. `xsd:string` is therefore typed in
/// EVERY closure, including the empty graph's, and `example.org/dt` is typed in none:
/// that pair is the whole content of the rule, stated as one presence and one absence.
const RDFS_AXIOMATIC_ROWS: &[AxiomaticRow] = &[(
    RuleId::Rdfs1,
    "empty",
    Conclusion::Iris(XSD_STRING, RDF_TYPE, RDFS_DATATYPE),
    Conclusion::Iris(EX_DT, RDF_TYPE, RDFS_DATATYPE),
)];

/// The premise-free rules the `OWL-RL` lane fires, in specification table order.
///
/// Four of them, and each quantifies over a list the SPECIFICATION fixes rather than over
/// the graph — which is what makes the empty dataset the right witness and a term outside
/// the list the right denial:
///
/// * `cls-thing` and `cls-nothing1` type `owl:Thing` and `owl:Nothing` an `owl:Class`,
///   and nothing else. `example.org/A` is a class name the corpus uses everywhere and is
///   typed an `owl:Class` by neither.
/// * `dt-type1` types each of the thirty-two datatypes OWL 2 Profiles §4.2.1 lists as
///   supported in OWL 2 RL an `rdfs:Datatype`. `xsd:integer` is one; `example.org/dt` is
///   not, and neither is `owl:real`, which is in the OWL 2 datatype map and deliberately
///   NOT supported in OWL 2 RL.
/// * `prp-ap` types each BUILT-IN annotation property of OWL 2 RL an
///   `owl:AnnotationProperty`, and OWL 2 Structural Specification §5.5 fixes that list at
///   nine. `rdfs:label` is one; `example.org/p` is not.
const OWL_RL_AXIOMATIC_ROWS: &[AxiomaticRow] = &[
    (
        RuleId::PrpAp,
        "empty",
        Conclusion::Iris(RDFS_LABEL, RDF_TYPE, OWL_ANNOTATIONPROPERTY),
        Conclusion::Iris(EX_P, RDF_TYPE, OWL_ANNOTATIONPROPERTY),
    ),
    (
        RuleId::ClsThing,
        "empty",
        Conclusion::Iris(OWL_THING, RDF_TYPE, OWL_CLASS),
        Conclusion::Iris(EX_A, RDF_TYPE, OWL_CLASS),
    ),
    (
        RuleId::ClsNothing1,
        "empty",
        Conclusion::Iris(OWL_NOTHING, RDF_TYPE, OWL_CLASS),
        Conclusion::Iris(EX_A, RDF_TYPE, OWL_CLASS),
    ),
    (
        RuleId::DtType1,
        "empty",
        Conclusion::Iris(XSD_INTEGER, RDF_TYPE, RDFS_DATATYPE),
        Conclusion::Iris(EX_DT, RDF_TYPE, RDFS_DATATYPE),
    ),
];

/// The premise-free rules `regime` registers.
fn axiomatic_rows(regime: Regime) -> &'static [AxiomaticRow] {
    match regime {
        Regime::Rdfs => RDFS_AXIOMATIC_ROWS,
        Regime::OwlRl => OWL_RL_AXIOMATIC_ROWS,
        Regime::Simple | Regime::Rdf | Regime::OwlDirect | Regime::Rif | Regime::D => &[],
    }
}

/// `Regime::Rdfs`'s registered rules, in specification table order.
///
/// `rdfD2` heads the list because RDFS entailment subsumes RDF entailment: the lane fires
/// the §8.1.1 pattern as well as the §9.2.1 ones, and `rules(Regime::Rdfs)` has always
/// listed it. `rdfs1` is not here — it is premise-free, so it is in
/// [`RDFS_AXIOMATIC_ROWS`] instead.
const RDFS_ROWS: &[Row] = &[
    (
        RuleId::RdfD2,
        "plain_triple",
        Conclusion::Iris(EX_P, RDF_TYPE, RDF_PROPERTY),
        "named_graph",
    ),
    (
        RuleId::Rdfs2,
        "domain",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_A),
        "domain_near_miss",
    ),
    (
        RuleId::Rdfs3,
        "range",
        Conclusion::Iris(EX_Y, RDF_TYPE, EX_B),
        "range_near_miss",
    ),
    (
        RuleId::Rdfs4,
        "plain_triple",
        Conclusion::Iris(EX_X, RDF_TYPE, RDFS_RESOURCE),
        "named_graph",
    ),
    (
        RuleId::Rdfs5,
        "subproperty_chain",
        Conclusion::Iris(EX_P, RDFS_SUBPROPERTYOF, EX_R),
        "subproperty_chain_near_miss",
    ),
    (
        RuleId::Rdfs6,
        "property_typed",
        Conclusion::Iris(EX_P, RDFS_SUBPROPERTYOF, EX_P),
        "property_typed_near_miss",
    ),
    (
        RuleId::Rdfs7,
        "subproperty_rewrite",
        Conclusion::Iris(EX_X, EX_Q, EX_Y),
        "subproperty_rewrite_near_miss",
    ),
    (
        RuleId::Rdfs8,
        "class_typed",
        Conclusion::Iris(EX_C, RDFS_SUBCLASSOF, RDFS_RESOURCE),
        "class_typed_near_miss",
    ),
    (
        RuleId::Rdfs9,
        "subclass_instance",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_B),
        "subclass_instance_near_miss",
    ),
    (
        RuleId::Rdfs10,
        "class_typed",
        Conclusion::Iris(EX_C, RDFS_SUBCLASSOF, EX_C),
        "class_typed_near_miss",
    ),
    (
        RuleId::Rdfs11,
        "subclass_chain",
        Conclusion::Iris(EX_A, RDFS_SUBCLASSOF, EX_F),
        "subclass_chain_near_miss",
    ),
    (
        RuleId::Rdfs12,
        "container_membership",
        Conclusion::Iris(EX_P, RDFS_SUBPROPERTYOF, RDFS_MEMBER),
        "container_membership_near_miss",
    ),
    (
        RuleId::Rdfs13,
        "datatype_declared",
        Conclusion::Iris(EX_DT, RDFS_SUBCLASSOF, RDFS_LITERAL),
        "datatype_declared_near_miss",
    ),
];

/// `Regime::OwlRl`'s rules that conclude a triple on a premise, grouped by the table each
/// belongs to.
///
/// Nine of them are the RDFS rules above under their OWL 2 RL names, evaluated in the OWL
/// lane — the same fixture, a different calculus — and the rest are the lane's own.
///
/// Twenty-four of the lane's seventy-eight rules are NOT here, and each is in the table
/// that says what it does instead: the four premise-free ones in
/// [`OWL_RL_AXIOMATIC_ROWS`], the eighteen that refuse a run in [`OWL_RL_REFUTING_ROWS`],
/// and the two whose every conclusion is generalized RDF in [`OWL_RL_GENERALIZED_ROWS`].
const OWL_RL_ROWS: &[Row] = &[
    (
        RuleId::PrpDom,
        "domain",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_A),
        "domain_near_miss",
    ),
    (
        RuleId::PrpRng,
        "range",
        Conclusion::Iris(EX_Y, RDF_TYPE, EX_B),
        "range_near_miss",
    ),
    (
        RuleId::PrpSymp,
        "symmetric",
        Conclusion::Iris(EX_Y, EX_P, EX_X),
        "transitive",
    ),
    (
        RuleId::PrpTrp,
        "transitive",
        Conclusion::Iris(EX_X, EX_P, EX_Z),
        "symmetric",
    ),
    (
        RuleId::PrpSpo1,
        "subproperty_rewrite",
        Conclusion::Iris(EX_X, EX_Q, EX_Y),
        "subproperty_rewrite_near_miss",
    ),
    (
        RuleId::PrpInv1,
        "inverse_pair",
        Conclusion::Iris(EX_Y, EX_Q, EX_X),
        "inverse_pair_near_miss",
    ),
    (
        RuleId::PrpInv2,
        "inverse_pair",
        Conclusion::Iris(EX_V, EX_P, EX_U),
        "inverse_pair_near_miss",
    ),
    (
        RuleId::CaxSco,
        "subclass_instance",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_B),
        "subclass_instance_near_miss",
    ),
    (
        RuleId::ScmSco,
        "subclass_chain",
        Conclusion::Iris(EX_A, RDFS_SUBCLASSOF, EX_F),
        "subclass_chain_near_miss",
    ),
    (
        RuleId::ScmEqc1,
        "equivalent_class",
        Conclusion::Iris(EX_B, RDFS_SUBCLASSOF, EX_A),
        "equivalent_property",
    ),
    (
        RuleId::ScmSpo,
        "subproperty_chain",
        Conclusion::Iris(EX_P, RDFS_SUBPROPERTYOF, EX_R),
        "subproperty_chain_near_miss",
    ),
    (
        RuleId::ScmEqp1,
        "equivalent_property",
        Conclusion::Iris(EX_B, RDFS_SUBPROPERTYOF, EX_A),
        "equivalent_class",
    ),
    (
        RuleId::PrpFp,
        "functional",
        Conclusion::Iris(EX_Y, OWL_SAMEAS, EX_Z),
        "inverse_functional",
    ),
    (
        RuleId::PrpIfp,
        "inverse_functional",
        Conclusion::Iris(EX_X, OWL_SAMEAS, EX_Y),
        "functional",
    ),
    (
        RuleId::PrpSpo2,
        "property_chain",
        Conclusion::Iris(EX_X, EX_CHAINED, EX_Z),
        "property_chain_near_miss",
    ),
    (
        RuleId::PrpEqp1,
        "equivalent_property_data",
        Conclusion::Iris(EX_X, EX_Q, EX_Y),
        "equivalent_property_data_near_miss",
    ),
    (
        RuleId::PrpEqp2,
        "equivalent_property_data",
        Conclusion::Iris(EX_U, EX_P, EX_V),
        "equivalent_property_data_near_miss",
    ),
    (
        RuleId::PrpKey,
        "has_key",
        Conclusion::Iris(EX_X, OWL_SAMEAS, EX_Y),
        "has_key_near_miss",
    ),
    (
        RuleId::CaxEqc1,
        "equivalent_class_instance",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_B),
        "equivalent_class_instance_near_miss",
    ),
    (
        RuleId::CaxEqc2,
        "equivalent_class_instance",
        Conclusion::Iris(EX_U, RDF_TYPE, EX_A),
        "equivalent_class_instance_near_miss",
    ),
    (
        RuleId::ScmCls,
        "owl_class",
        Conclusion::Iris(EX_C, RDFS_SUBCLASSOF, OWL_THING),
        "class_typed",
    ),
    (
        RuleId::ScmEqc2,
        "mutual_subclass",
        Conclusion::Iris(EX_A, OWL_EQUIVALENTCLASS, EX_B),
        "mutual_subproperty",
    ),
    (
        RuleId::ScmOp,
        "object_property",
        Conclusion::Iris(EX_P, OWL_EQUIVALENTPROPERTY, EX_P),
        "datatype_property",
    ),
    (
        RuleId::ScmDp,
        "datatype_property",
        Conclusion::Iris(EX_Q, OWL_EQUIVALENTPROPERTY, EX_Q),
        "object_property",
    ),
    (
        RuleId::ScmEqp2,
        "mutual_subproperty",
        Conclusion::Iris(EX_P, OWL_EQUIVALENTPROPERTY, EX_Q),
        "mutual_subclass",
    ),
    (
        RuleId::ScmDom1,
        "domain_widened",
        Conclusion::Iris(EX_R, RDFS_DOMAIN, EX_B),
        "domain_inherited",
    ),
    (
        RuleId::ScmDom2,
        "domain_inherited",
        Conclusion::Iris(EX_P, RDFS_DOMAIN, EX_A),
        "domain_widened",
    ),
    (
        RuleId::ScmRng1,
        "range_widened",
        Conclusion::Iris(EX_R, RDFS_RANGE, EX_B),
        "range_inherited",
    ),
    (
        RuleId::ScmRng2,
        "range_inherited",
        Conclusion::Iris(EX_P, RDFS_RANGE, EX_A),
        "range_widened",
    ),
    (
        RuleId::ScmHv,
        "has_value_restrictions",
        Conclusion::Iris(EX_A, RDFS_SUBCLASSOF, EX_B),
        "has_value_restrictions_near_miss",
    ),
    (
        RuleId::ScmSvf1,
        "some_values_filler",
        Conclusion::Iris(EX_A, RDFS_SUBCLASSOF, EX_B),
        "some_values_property",
    ),
    (
        RuleId::ScmSvf2,
        "some_values_property",
        Conclusion::Iris(EX_E, RDFS_SUBCLASSOF, EX_F),
        "some_values_filler",
    ),
    (
        RuleId::ScmAvf1,
        "all_values_filler",
        Conclusion::Iris(EX_A, RDFS_SUBCLASSOF, EX_B),
        "all_values_property",
    ),
    (
        RuleId::ScmAvf2,
        "all_values_property",
        Conclusion::Iris(EX_F, RDFS_SUBCLASSOF, EX_E),
        "all_values_filler",
    ),
    (
        RuleId::ScmInt,
        "intersection_of",
        Conclusion::Iris(EX_C, RDFS_SUBCLASSOF, EX_A),
        "union_of",
    ),
    (
        RuleId::ScmUni,
        "union_of",
        Conclusion::Iris(EX_A, RDFS_SUBCLASSOF, EX_D),
        "intersection_of",
    ),
    // --- Table 4, the six `eq-*` rules that conclude a triple. ---
    //
    // `eq-ref` has a premise — `T(?s, ?p, ?o)` — so it is NOT axiomatic, even though it
    // fires on the empty graph: the premise-free rules put triples there for it to read.
    // `named_graph` is the near miss for exactly that reason: the chase reads the default
    // graph only, so a term that appears solely in a named graph is a term no triple of
    // this run mentions, and it is the one input where `x owl:sameAs x` can be absent.
    (
        RuleId::EqRef,
        "plain_triple",
        Conclusion::Iris(EX_X, OWL_SAMEAS, EX_X),
        "named_graph",
    ),
    (
        RuleId::EqSym,
        "same_as",
        Conclusion::Iris(EX_Y, OWL_SAMEAS, EX_X),
        "plain_triple",
    ),
    (
        RuleId::EqTrans,
        "same_as_chain",
        Conclusion::Iris(EX_X, OWL_SAMEAS, EX_Z),
        "same_as_chain_near_miss",
    ),
    (
        RuleId::EqRepS,
        "same_as_subject",
        Conclusion::Iris(EX_U, EX_P, EX_Y),
        "same_as_object",
    ),
    (
        RuleId::EqRepP,
        "same_as_predicate",
        Conclusion::Iris(EX_X, EX_Q, EX_Y),
        "same_as_subject",
    ),
    (
        RuleId::EqRepO,
        "same_as_object",
        Conclusion::Iris(EX_X, EX_P, EX_V),
        "same_as_predicate",
    ),
    // --- Table 6, the twelve `cls-*` rules that conclude a triple. ---
    (
        RuleId::ClsInt1,
        "intersection_instance",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_C),
        "intersection_instance_near_miss",
    ),
    (
        RuleId::ClsInt2,
        "intersection_member_typing",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_A),
        "intersection_of",
    ),
    (
        RuleId::ClsUni,
        "union_instance",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_D),
        "union_instance_near_miss",
    ),
    (
        RuleId::ClsSvf1,
        "some_values_instance",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_E),
        "some_values_instance_near_miss",
    ),
    (
        RuleId::ClsSvf2,
        "some_values_thing",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_E),
        "some_values_instance_near_miss",
    ),
    (
        RuleId::ClsAvf,
        "all_values_instance",
        Conclusion::Iris(EX_Y, RDF_TYPE, EX_C),
        "all_values_instance_near_miss",
    ),
    (
        RuleId::ClsHv1,
        "has_value_assert",
        Conclusion::Literal {
            subject: EX_X,
            predicate: EX_P,
            lexical: "cat",
            datatype: XSD_STRING,
        },
        "has_value_near_miss",
    ),
    (
        RuleId::ClsHv2,
        "has_value_recognize",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_E),
        "has_value_near_miss",
    ),
    (
        RuleId::ClsMaxc2,
        "max_cardinality_one",
        Conclusion::Iris(EX_Y, OWL_SAMEAS, EX_Z),
        "max_cardinality_one_near_miss",
    ),
    (
        RuleId::ClsMaxqc3,
        "max_qualified_one",
        Conclusion::Iris(EX_Y, OWL_SAMEAS, EX_Z),
        "max_qualified_one_near_miss",
    ),
    (
        RuleId::ClsMaxqc4,
        "max_qualified_one_thing",
        Conclusion::Iris(EX_Y, OWL_SAMEAS, EX_Z),
        "max_qualified_one_near_miss",
    ),
    (
        RuleId::ClsOo,
        "one_of",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_C),
        "one_of_near_miss",
    ),
];

/// The `OWL-RL` rules whose conclusion is `false`, and the two runs that evidence each.
///
/// Eighteen rows: the seventeen rules of Tables 4–8 that conclude `false`, plus `dt-diff`,
/// whose witness is `eq-diff1` for the reason [`RuleFixtures::Refuting`] gives. Every
/// fixture named here is in [`CLASH_CORPUS`], because the `clashes` half has no closure and
/// therefore no golden, and a control belongs beside the thing it controls.
const OWL_RL_REFUTING_ROWS: &[RefutingRow] = &[
    (
        RuleId::EqDiff1,
        "eq_diff1_clash",
        RuleId::EqDiff1,
        "eq_diff1_consistent",
    ),
    (
        RuleId::EqDiff2,
        "eq_diff2_clash",
        RuleId::EqDiff2,
        "eq_diff2_consistent",
    ),
    (
        RuleId::EqDiff3,
        "eq_diff3_clash",
        RuleId::EqDiff3,
        "eq_diff3_consistent",
    ),
    (
        RuleId::PrpIrp,
        "prp_irp_clash",
        RuleId::PrpIrp,
        "prp_irp_consistent",
    ),
    (
        RuleId::PrpAsyp,
        "prp_asyp_clash",
        RuleId::PrpAsyp,
        "prp_asyp_consistent",
    ),
    (
        RuleId::PrpPdw,
        "prp_pdw_clash",
        RuleId::PrpPdw,
        "prp_pdw_consistent",
    ),
    (
        RuleId::PrpAdp,
        "prp_adp_clash",
        RuleId::PrpAdp,
        "prp_adp_consistent",
    ),
    (
        RuleId::PrpNpa1,
        "prp_npa1_clash",
        RuleId::PrpNpa1,
        "prp_npa1_consistent",
    ),
    (
        RuleId::PrpNpa2,
        "prp_npa2_clash",
        RuleId::PrpNpa2,
        "prp_npa2_consistent",
    ),
    (
        RuleId::ClsNothing2,
        "cls_nothing2_clash",
        RuleId::ClsNothing2,
        "cls_nothing2_consistent",
    ),
    (
        RuleId::ClsCom,
        "cls_com_clash",
        RuleId::ClsCom,
        "cls_com_consistent",
    ),
    (
        RuleId::ClsMaxc1,
        "cls_maxc1_clash",
        RuleId::ClsMaxc1,
        "cls_maxc1_consistent",
    ),
    (
        RuleId::ClsMaxqc1,
        "cls_maxqc1_clash",
        RuleId::ClsMaxqc1,
        "cls_maxqc1_consistent",
    ),
    (
        RuleId::ClsMaxqc2,
        "cls_maxqc2_clash",
        RuleId::ClsMaxqc2,
        "cls_maxqc2_consistent",
    ),
    (
        RuleId::CaxDw,
        "cax_dw_clash",
        RuleId::CaxDw,
        "cax_dw_consistent",
    ),
    (
        RuleId::CaxAdc,
        "cax_adc_clash",
        RuleId::CaxAdc,
        "cax_adc_consistent",
    ),
    // `dt-diff` is the one row whose witness is not its own id; see
    // [`RuleFixtures::Refuting`]. It concludes `owl:differentFrom`, every one of those
    // conclusions has a literal subject and is dropped, and `eq-diff1` is the only consumer
    // in the whole of OWL 2 RL — so eq-diff1's refusal on an input where nothing else can
    // supply that premise is the only observation that dt-diff fired.
    (
        RuleId::DtDiff,
        "dt_diff_clash",
        RuleId::EqDiff1,
        "dt_diff_consistent",
    ),
    (
        RuleId::DtNotType,
        "dt_not_type_clash",
        RuleId::DtNotType,
        "dt_not_type_consistent",
    ),
];

/// The `OWL-RL` rules every one of whose conclusions is generalized RDF, and the DOWNSTREAM
/// triple each licenses.
///
/// Two rows. `dt-type2` concludes `lt rdf:type dt` and `dt-eq` concludes
/// `lt1 owl:sameAs lt2`; both put a literal in SUBJECT position, which the RDF 1.2 dataset
/// IR cannot represent, so neither rule can ever put a line in a closure or be credited in
/// a report. The rows below name the triple one hop later that the RDF 1.2 IR does hold —
/// `cls-svf1`'s and `eq-rep-o`'s respectively — and
/// `every_rule_is_registered_or_declared_unimplemented` additionally requires the positive
/// run to report the generalized-rdf boundary, so "the conclusion was abandoned" is
/// distinguishable from "the rule never fired".
const OWL_RL_GENERALIZED_ROWS: &[Row] = &[
    (
        RuleId::DtType2,
        "datatype_value_typing",
        Conclusion::Iris(EX_X, RDF_TYPE, EX_E),
        "datatype_value_typing_near_miss",
    ),
    (
        RuleId::DtEq,
        "datatype_value_equality",
        Conclusion::Literal {
            subject: EX_X,
            predicate: EX_P,
            lexical: "01",
            datatype: XSD_INTEGER,
        },
        "datatype_value_equality_near_miss",
    ),
];

/// A withheld-conclusion registry row: the rule, its positive fixture, and the baseline
/// its withheld count must exceed (`None` for a premise-free rule).
type WithheldRow = (RuleId, &'static str, Option<&'static str>);

/// The `RDF` lane's rules whose every conclusion mentions a surrogate.
///
/// `rdfD1` observes a datatyped literal, so `divergence_literal_subject` — whose graph
/// holds one — withholds more than `plain_triple`, whose graph holds none. `rdfD1a` is
/// PREMISE-FREE, so
/// its control is against zero on the empty dataset: a rule that mints a witness over the
/// empty graph mints one over every graph.
const RDF_WITHHELD_ROWS: &[WithheldRow] = &[
    (
        RuleId::RdfD1,
        "divergence_literal_subject",
        Some("plain_triple"),
    ),
    (RuleId::RdfD1a, "empty", None),
];

/// The `RDFS` lane's — the two RDF patterns plus the two triple-term ones.
///
/// `rdfs14` observes a TRIPLE TERM, so `triple_term` withholds more than `plain_triple`.
const RDFS_WITHHELD_ROWS: &[WithheldRow] = &[
    (
        RuleId::RdfD1,
        "divergence_literal_subject",
        Some("plain_triple"),
    ),
    (RuleId::RdfD1a, "empty", None),
    (RuleId::Rdfs14, "triple_term", Some("plain_triple")),
    (RuleId::Rdfs14a, "empty", None),
];

/// The withheld-conclusion rules `regime` registers.
fn withheld_rows(regime: Regime) -> &'static [WithheldRow] {
    match regime {
        Regime::Rdf => RDF_WITHHELD_ROWS,
        Regime::Rdfs => RDFS_WITHHELD_ROWS,
        Regime::Simple | Regime::OwlRl | Regime::OwlDirect | Regime::Rif | Regime::D => &[],
    }
}

/// The refuting rules `regime` registers.
fn refuting_rows(regime: Regime) -> &'static [RefutingRow] {
    match regime {
        Regime::OwlRl => OWL_RL_REFUTING_ROWS,
        Regime::Simple
        | Regime::Rdf
        | Regime::Rdfs
        | Regime::OwlDirect
        | Regime::Rif
        | Regime::D => &[],
    }
}

/// The generalized-conclusion rules `regime` registers.
fn generalized_rows(regime: Regime) -> &'static [Row] {
    match regime {
        Regime::OwlRl => OWL_RL_GENERALIZED_ROWS,
        Regime::Simple
        | Regime::Rdf
        | Regime::Rdfs
        | Regime::OwlDirect
        | Regime::Rif
        | Regime::D => &[],
    }
}

/// The rows `regime` registers.
fn rows(regime: Regime) -> &'static [Row] {
    match regime {
        Regime::Rdf => RDF_ROWS,
        Regime::Rdfs => RDFS_ROWS,
        Regime::OwlRl => OWL_RL_ROWS,
        Regime::Simple | Regime::OwlDirect | Regime::Rif | Regime::D => &[],
    }
}

/// What the corpus says about `id` under `regime`.
fn registration(regime: Regime, id: RuleId) -> RuleFixtures {
    if let Some(&(_, positive, conclusion, near_miss)) = rows(regime).iter().find(|row| row.0 == id)
    {
        return RuleFixtures::Registered {
            positive: Case {
                fixture: positive,
                conclusion,
            },
            near_miss: Case {
                fixture: near_miss,
                conclusion,
            },
        };
    }
    if let Some(&(_, fixture, holds, denied)) =
        axiomatic_rows(regime).iter().find(|row| row.0 == id)
    {
        return RuleFixtures::Axiomatic {
            holds: Case {
                fixture,
                conclusion: holds,
            },
            denied: Case {
                fixture,
                conclusion: denied,
            },
        };
    }
    if let Some(&(_, clashes, witness, consistent)) =
        refuting_rows(regime).iter().find(|row| row.0 == id)
    {
        return RuleFixtures::Refuting {
            clashes,
            witness,
            consistent,
        };
    }
    if let Some(&(_, positive, conclusion, near_miss)) =
        generalized_rows(regime).iter().find(|row| row.0 == id)
    {
        return RuleFixtures::Generalized {
            positive: Case {
                fixture: positive,
                conclusion,
            },
            near_miss: Case {
                fixture: near_miss,
                conclusion,
            },
        };
    }
    if let Some(&(_, positive, baseline)) = withheld_rows(regime).iter().find(|row| row.0 == id) {
        return RuleFixtures::Withheld { positive, baseline };
    }
    RuleFixtures::NotYetImplemented
}

/// The four regimes the registry ranges over.
///
/// NOT every regime `materialize` can run: `D` is absent. The registry's unit of evidence
/// is a rule's conclusion in a CLOSURE, and `D` is Simple entailment plus OWL 2 Profiles
/// Table 8 — a lane in which `dt-type2`, `dt-eq` and `dt-diff` have no consumer at all, so
/// the downstream triples [`RuleFixtures::Generalized`] and [`RuleFixtures::Refuting`] rely
/// on (`cls-svf1`, `eq-rep-o`, `eq-diff1`, `prp-fp`) are not in the lane and the evidence
/// those two states are built from does not exist there. The `D` lane is pinned instead by
/// the goldens, which hold its closure and its report for every fixture of the corpus: its
/// rule table is Table 8 entire, and the `OWL-RL` rows below are what evidence each of
/// those five rules.
const REGISTRY_REGIMES: [Regime; 4] = [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::OwlRl];

/// THE REGISTRY. Every rule of every runnable regime is in exactly one of five states, and
/// the `NotYetImplemented` set is EXACTLY the inventory's gap.
///
/// Two independent statements meet here. The report DERIVES its missing list as
/// `rules(r)` minus `implemented(r)`; this test derives the same set from what the corpus
/// can and cannot demonstrate. They must agree, so:
///
/// * implementing a rule without adding fixtures fails — the id leaves `implemented`'s
///   complement while the registry still calls it `NotYetImplemented`;
/// * adding fixtures without implementing the rule fails the same way, from the other
///   side;
/// * a `Registered` rule that stops firing fails on its own positive assertion;
/// * a `Refuting` rule that stops refusing, or starts naming a different rule in its
///   witness, fails on the run itself;
/// * and a `Generalized` rule that stops firing fails on the downstream triple, or — the
///   subtler regression — on the boundary, if its conclusion were ever silently skipped
///   instead of derived and abandoned.
///
/// That is the mechanism that makes "one test per rule" non-re-interpretable for all 78
/// OWL 2 RL rules at once.
#[test]
fn every_rule_is_registered_or_declared_unimplemented() {
    for regime in REGISTRY_REGIMES {
        let mut not_yet: BTreeSet<RuleId> = BTreeSet::new();
        let mut registered: BTreeSet<RuleId> = BTreeSet::new();
        for &id in rules(regime) {
            match registration(regime, id) {
                RuleFixtures::Registered {
                    positive,
                    near_miss,
                } => {
                    assert!(registered.insert(id), "{regime:?} registers {id} twice");
                    assert_ne!(
                        positive.fixture, near_miss.fixture,
                        "{regime:?} / {id}: a near miss must be a DIFFERENT input"
                    );
                    assert_eq!(
                        positive.conclusion, near_miss.conclusion,
                        "{regime:?} / {id}: a near miss must deny the same conclusion the \
                         positive asserts"
                    );
                    let line = positive.conclusion.line();
                    assert!(
                        closure_lines(positive.fixture, regime).contains(&line),
                        "{regime:?} / {id}: positive fixture {} did not conclude {line}",
                        positive.fixture
                    );
                    assert!(
                        !closure_lines(near_miss.fixture, regime).contains(&line),
                        "{regime:?} / {id}: near-miss fixture {} concluded {line} anyway",
                        near_miss.fixture
                    );
                }
                RuleFixtures::Axiomatic { holds, denied } => {
                    assert!(registered.insert(id), "{regime:?} registers {id} twice");
                    assert_eq!(
                        holds.fixture, denied.fixture,
                        "{regime:?} / {id}: a premise-free rule's control is the SAME input"
                    );
                    assert_eq!(
                        holds.fixture, "empty",
                        "{regime:?} / {id}: a premise-free rule must be witnessed on the \
                         EMPTY dataset, which is what makes it premise-free"
                    );
                    assert_ne!(
                        holds.conclusion, denied.conclusion,
                        "{regime:?} / {id}: the denied conclusion must differ from the one \
                         that holds"
                    );
                    let lines = closure_lines(holds.fixture, regime);
                    assert!(
                        lines.contains(&holds.conclusion.line()),
                        "{regime:?} / {id}: the empty dataset did not conclude {}",
                        holds.conclusion.line()
                    );
                    assert!(
                        !lines.contains(&denied.conclusion.line()),
                        "{regime:?} / {id}: the rule ranged over a term it must not, \
                         concluding {}",
                        denied.conclusion.line()
                    );
                }
                RuleFixtures::Refuting {
                    clashes,
                    witness,
                    consistent,
                } => {
                    assert!(registered.insert(id), "{regime:?} registers {id} twice");
                    assert_ne!(
                        clashes, consistent,
                        "{regime:?} / {id}: a control must be a DIFFERENT input"
                    );
                    let error = materialize(&build(fixture(clashes)), regime)
                        .err()
                        .unwrap_or_else(|| {
                            panic!("{regime:?} / {id}: {clashes} closed instead of refusing")
                        });
                    let EntailError::Inconsistent(found) = error else {
                        panic!("{regime:?} / {id}: {clashes} failed with {error}, not a clash");
                    };
                    assert_eq!(
                        found.witness().rule(),
                        witness,
                        "{regime:?} / {id}: {clashes} was refused by the wrong rule"
                    );
                    // The refusal carries the RUN, so the report is checkable too: its
                    // `inconsistency` is the same witness, which is the state that makes
                    // that field observable at all.
                    assert_eq!(
                        found
                            .report()
                            .inconsistency()
                            .map(InconsistencyWitness::rule),
                        Some(witness),
                        "{regime:?} / {id}: the report must name the rule that refused"
                    );
                    assert!(
                        !found.witness().premises().is_empty(),
                        "{regime:?} / {id}: a witness must name the triples that satisfied it"
                    );
                    assert!(
                        materialize(&build(fixture(consistent)), regime).is_ok(),
                        "{regime:?} / {id}: the control {consistent} was refused too, so the \
                         refusal is not attributable to the rule's premise"
                    );
                }
                RuleFixtures::Generalized {
                    positive,
                    near_miss,
                } => {
                    assert!(registered.insert(id), "{regime:?} registers {id} twice");
                    assert_ne!(
                        positive.fixture, near_miss.fixture,
                        "{regime:?} / {id}: a near miss must be a DIFFERENT input"
                    );
                    assert_eq!(
                        positive.conclusion, near_miss.conclusion,
                        "{regime:?} / {id}: a near miss must deny the same conclusion the \
                         positive asserts"
                    );
                    let (closed, report) = materialize(&build(fixture(positive.fixture)), regime)
                        .expect("the positive fixture of a generalized rule closes");
                    let line = positive.conclusion.line();
                    assert!(
                        canonicalize(&closed).nquads.lines().any(|l| l == line),
                        "{regime:?} / {id}: positive fixture {} did not reach {line}",
                        positive.fixture
                    );
                    assert!(
                        report
                            .boundaries()
                            .iter()
                            .any(|b| b.construct().as_str() == "generalized-rdf"),
                        "{regime:?} / {id}: the rule's own conclusions are generalized RDF, \
                         so the run that fired it must report that boundary — without it, \
                         the downstream triple is evidence of nothing in particular"
                    );
                    assert!(
                        !closure_lines(near_miss.fixture, regime).contains(&line),
                        "{regime:?} / {id}: near-miss fixture {} reached {line} anyway",
                        near_miss.fixture
                    );
                }
                RuleFixtures::Withheld { positive, baseline } => {
                    registered.insert(id);
                    let withheld = |fixture: &str| -> u64 {
                        let ds = build(CORPUS.iter().find(|f| f.name == fixture).expect(fixture));
                        materialize(&ds, regime)
                            .expect("runnable regime")
                            .1
                            .withheld_surrogates()
                    };
                    let got = withheld(positive);
                    let floor = baseline.map_or(0, withheld);
                    assert!(
                        got > floor,
                        "{regime:?} / {id}: {positive} must withhold more surrogate \
                         conclusions than {} — {got} vs {floor}",
                        baseline.unwrap_or("nothing at all")
                    );
                    assert!(
                        materialize(
                            &build(CORPUS.iter().find(|f| f.name == positive).unwrap()),
                            regime
                        )
                        .expect("runnable regime")
                        .1
                        .boundaries()
                        .iter()
                        .any(|b| b.construct().as_str() == "surrogate"),
                        "{regime:?} / {id}: a run that withheld a conclusion must report \
                         the surrogate boundary"
                    );
                }
                RuleFixtures::NotYetImplemented => {
                    not_yet.insert(id);
                }
            }
        }

        // THE GAP, stated twice and checked once.
        let inventory_gap: BTreeSet<RuleId> = rules(regime)
            .iter()
            .copied()
            .filter(|rule| !implemented(regime).contains(rule))
            .collect();
        assert_eq!(
            not_yet, inventory_gap,
            "{regime:?}: the registry's unimplemented set must equal rules(r) minus \
             implemented(r), exactly"
        );
        let done: BTreeSet<RuleId> = implemented(regime).iter().copied().collect();
        assert_eq!(
            registered, done,
            "{regime:?}: every implemented rule must carry a positive and a near-miss \
             fixture, and nothing else may"
        );
    }
}

/// The registry's shape today, pinned as a ratchet.
///
/// A ratchet, not a drift guard: when a later change teaches the chase a rule these
/// numbers MUST move, in the same commit that adds the rule and its fixtures. Never widen
/// it to an inequality.
///
/// # The last column is DERIVED, because pinning it is how it came to lie
///
/// This ratchet used to pin four independent numbers per regime and label the last one
/// "rules not yet implemented", where it read `("RDFS", 18, 14, 4)` and `("RDF", 3, 1, 2)`.
/// The 4 and the 2 were `rdfD1`, `rdfD1a`, `rdfs14` and `rdfs14a` — rules the chase DOES
/// fire — and the only thing the arithmetic had actually noticed was that
/// [`withheld_rows`] was missing from the sum, because a rule whose every conclusion is
/// withheld carries its evidence in that fifth table rather than in a positive/near-miss
/// pair. So the ratchet asserted the opposite of [`implemented`], and did it in a
/// hand-written constant that no amount of implementing rules could move.
///
/// It is now two facts, each read from its own source: the EVIDENCE count sums all five
/// registry tables, and the unimplemented count is `rules(r)` minus `implemented(r)` — the
/// inventory itself, not a subtraction that happens to land there. A rule that gains an
/// implementation without gaining evidence moves the second column; a rule that gains
/// neither moves the third. Neither can be satisfied by the other going wrong.
#[test]
fn the_registry_shape_is_pinned() {
    let shape: Vec<(&str, usize, usize, usize)> = REGISTRY_REGIMES
        .iter()
        .map(|&regime| {
            let evidenced = rows(regime).len()
                + axiomatic_rows(regime).len()
                + refuting_rows(regime).len()
                + generalized_rows(regime).len()
                + withheld_rows(regime).len();
            let unimplemented = rules(regime)
                .iter()
                .filter(|rule| !implemented(regime).contains(rule))
                .count();
            (
                regime_label(regime),
                rules(regime).len(),
                evidenced,
                unimplemented,
            )
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            ("Simple", 0, 0, 0),
            ("RDF", 3, 3, 0),
            ("RDFS", 18, 18, 0),
            ("OWL-RL", 78, 78, 0),
        ],
        "(regime, rules the spec defines, rules the registry carries evidence for, \
         rules NOT implemented)"
    );
    // The relationship the three columns are in, stated once rather than pinned three
    // times: every rule the specification defines carries evidence, and no rule the chase
    // fires is missing from the registry.
    for &(_, total, evidenced, unimplemented) in &shape {
        assert_eq!(evidenced, total, "every defined rule must carry evidence");
        assert_eq!(unimplemented, 0, "no defined rule is unimplemented today");
    }
}

/// A regime's name, for messages and the shape ratchet. Exhaustive on purpose: a new
/// `Regime` variant fails to compile here.
const fn regime_label(regime: Regime) -> &'static str {
    match regime {
        Regime::Simple => "Simple",
        Regime::Rdf => "RDF",
        Regime::Rdfs => "RDFS",
        Regime::OwlRl => "OWL-RL",
        Regime::OwlDirect => "OWL-Direct",
        Regime::Rif => "RIF",
        Regime::D => "D",
    }
}

/// Every fixture the registry names is in one of the two corpus tables, and every fixture
/// of either table is used by the registry or is one of the awkward/divergence cases that
/// carry their own tests.
#[test]
fn the_registry_and_the_corpus_agree() {
    let corpus: BTreeSet<&str> = CORPUS
        .iter()
        .chain(CLASH_CORPUS)
        .map(|fixture| fixture.name)
        .collect();
    let mut used: BTreeMap<&str, usize> = BTreeMap::new();
    for regime in REGISTRY_REGIMES {
        for &(_, positive, _, near_miss) in rows(regime).iter().chain(generalized_rows(regime)) {
            for name in [positive, near_miss] {
                assert!(corpus.contains(name), "{name} is not in the corpus");
                *used.entry(name).or_default() += 1;
            }
        }
        for &(_, fixture, _, _) in axiomatic_rows(regime) {
            assert!(corpus.contains(fixture), "{fixture} is not in the corpus");
            *used.entry(fixture).or_default() += 1;
        }
        for &(_, clashes, _, consistent) in refuting_rows(regime) {
            for name in [clashes, consistent] {
                assert!(corpus.contains(name), "{name} is not in the corpus");
                *used.entry(name).or_default() += 1;
            }
        }
    }
    // The reifier enumeration is a SECOND registry over the same corpus, keyed by
    // INTERACTION rather than by rule — because "what happens when a user puts rdfs:domain
    // on rdf:reifies" is not a question a rule-keyed table can ask, and several of its rows
    // land on rules the per-rule registry already spends its one row on. Its fixtures count
    // as referenced for the same reason theirs do: something asserts a conclusion over them.
    for interaction in REIFIER_INTERACTIONS {
        for name in [interaction.positive, interaction.near_miss] {
            assert!(corpus.contains(name), "{name} is not in the corpus");
            *used.entry(name).or_default() += 1;
        }
    }
    // A `CLASH_CORPUS` fixture exists ONLY to be a registry row's half, so every one of
    // them must be reached — there is no "carries its own test" escape on that side, and a
    // clash fixture nothing names would be an input nothing ever runs.
    for fixture in CLASH_CORPUS {
        assert!(
            used.contains_key(fixture.name),
            "{}: a CLASH_CORPUS fixture the registry never names is never run",
            fixture.name
        );
    }
    // The fixtures the registry does NOT reach, named explicitly. Each one exists for a
    // reason the registry cannot express — a boundary, a fixpoint depth, a shared
    // conclusion, or a documented divergence — and each has its own test below.
    let unreferenced: BTreeSet<&str> = corpus
        .iter()
        .copied()
        .filter(|name| !used.contains_key(name))
        .collect();
    assert_eq!(
        unreferenced,
        [
            "divergence_broad_triggers",
            "divergence_literal_subject",
            // The two dataset-semantics fixtures. The registry is keyed by RULE and asks
            // "did this rule fire", which is not the question either of these poses: both
            // turn on rdfs9 / cax-sco firing or not firing IN A PARTICULAR GRAPH, and a
            // registry row's conclusion is a graph-less triple. Their own test is
            // `a_terminology_in_the_default_graph_types_instances_in_a_named_graph`.
            "named_graph_closure",
            "named_graph_closure_near_miss",
            "shared_conclusion",
            "triple_term",
        ]
        .into_iter()
        .collect::<BTreeSet<&str>>()
    );
}

// ── The awkward cases, and the two documented divergences ───────────────────────

/// A named graph is closed, its conclusions land in IT, and the default graph does not
/// receive them.
///
/// The three halves of the defined dataset semantics, over the one fixture that has a quad
/// outside the default graph. RDF has no standard entailment relation for a dataset, so
/// what a run does with one is a choice; this is the choice, asserted rather than described.
#[test]
fn a_named_graph_is_closed_and_its_conclusions_land_in_it() {
    for regime in [Regime::Rdf, Regime::Rdfs, Regime::OwlRl] {
        let ds = build(fixture("named_graph"));
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        assert!(
            report
                .boundaries()
                .iter()
                .any(|b| b.construct().as_str() == "named-graph"),
            "{regime:?} did not report the named-graph boundary"
        );
        let lines = canonicalize(&closed).nquads;
        assert!(
            lines.contains(&format!("<{EX_X}> <{EX_P}> <{EX_Y}> <{EX_G}> .")),
            "{regime:?} did not carry the named-graph quad through"
        );
        // The premise IS read, and its conclusion is in the graph that produced it. Which
        // conclusion depends on the lane: the two RDF-shaped lanes type the predicate
        // (rdfD2), and OWL-RL — whose tables omit the RDF axiomatic material entirely —
        // reaches the same three terms through eq-ref instead. Both are named, so neither
        // lane can pass on the other's evidence.
        let drawn = match regime {
            Regime::OwlRl => format!("<{EX_P}> <{OWL_SAMEAS}> <{EX_P}> <{EX_G}> ."),
            _ => format!("<{EX_P}> <{RDF_TYPE}> <{RDF_PROPERTY}> <{EX_G}> ."),
        };
        assert!(
            lines.lines().any(|line| line == drawn),
            "{regime:?} did not close the named graph: {drawn}"
        );
        // …and NOT in the default graph, which never held the premise.
        let default_graph_form = drawn.replace(&format!(" <{EX_G}> ."), " .");
        assert!(
            !lines.lines().any(|line| line == default_graph_form),
            "{regime:?} routed a named graph's conclusion into the default graph"
        );
        assert!(
            !lines
                .lines()
                .any(|line| line == nquads_line(EX_P, RDF_TYPE, RDF_PROPERTY)),
            "{regime:?} routed a named graph's conclusion into the default graph"
        );
    }
}

/// THE LAYOUT THE SEMANTICS EXISTS FOR: a terminology in the default graph and instances in
/// a named graph derive the expected triples INTO the named graph.
///
/// And the control that makes it a statement about the JOIN rather than about the lane:
/// move the terminology into a sibling named graph and the same conclusion appears in no
/// graph at all, because a named graph is closed against the union of itself and the
/// DEFAULT graph and never against a sibling.
#[test]
fn a_terminology_in_the_default_graph_types_instances_in_a_named_graph() {
    let derived = format!("<{EX_X}> <{RDF_TYPE}> <{EX_B}> <{EX_G}> .");
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let positive = canonicalize(
            &materialize(&build(fixture("named_graph_closure")), regime)
                .expect("runnable regime")
                .0,
        )
        .nquads;
        assert!(
            positive.lines().any(|line| line == derived),
            "{regime:?}: the terminology did not reach the named graph's instances"
        );
        // The conclusion is in ex:g and NOWHERE else — not in the default graph, which
        // holds the terminology but no instance.
        assert!(
            !positive
                .lines()
                .any(|line| line == nquads_line(EX_X, RDF_TYPE, EX_B)),
            "{regime:?}: a named graph's conclusion reached the default graph"
        );

        // THE CROSS-GRAPH JOIN THAT MUST NOT HAPPEN.
        let near_miss = canonicalize(
            &materialize(&build(fixture("named_graph_closure_near_miss")), regime)
                .expect("runnable regime")
                .0,
        )
        .nquads;
        for graph in ["", &format!(" <{EX_G}>"), &format!(" <{EX_H}>")] {
            let line = format!("<{EX_X}> <{RDF_TYPE}> <{EX_B}>{graph} .");
            assert!(
                !near_miss.lines().any(|l| l == line),
                "{regime:?}: two named graphs joined — {line}"
            );
        }
        // …and the sibling graph really was closed, so the absence above is the missing
        // join rather than a lane that did nothing.
        assert!(
            near_miss.lines().any(|line| line
                == format!("<{EX_A}> <{RDF_TYPE}> <{RDFS_CLASS}> <{EX_H}> .")
                || line == format!("<{EX_A}> <{OWL_SAMEAS}> <{EX_A}> <{EX_H}> .")),
            "{regime:?}: the sibling graph drew nothing at all:\n{near_miss}"
        );
    }
}

// ── rdf:reifies is ordinary data, and this is the enumeration ───────────────────

/// One `rdf:reifies` interaction: where the term sits, and what that makes fire.
///
/// A two-sided control like every registry row: the positive fixture's closure must hold
/// `conclusion` and the near miss's must not. The near miss differs from the positive in
/// EXACTLY ONE TERM, and for six of the ten that one term is `rdf:reifies` itself replaced
/// by `example.org/p` — which is the whole claim of this table, stated as evidence: the
/// reifier property is ordinary data, and a rule fires on it exactly when it would have
/// fired on a user's own IRI in the same position.
#[derive(Debug, Clone, Copy)]
struct ReifierInteraction {
    /// What this row pins, for a reader of a failure message.
    what: &'static str,
    /// The regime the interaction is asserted under.
    ///
    /// Not every lane: `prp-dom`, `prp-rng`, `prp-spo1`, `scm-dom1` and `scm-rng1` are OWL
    /// 2 RL's own, and the two positional rows are asserted under `RDFS`, where the
    /// corresponding `rdfs2`/`rdfs3` live. Naming the lane per row is what keeps each
    /// assertion about ONE rule table.
    regime: Regime,
    /// The fixture the conclusion must be PRESENT in.
    positive: &'static str,
    /// The conclusion, as the canonical N-Quads line it must be (or must not be).
    conclusion: Conclusion,
    /// The fixture the same conclusion must be ABSENT from.
    near_miss: &'static str,
}

/// EVERY `rdf:reifies` interaction this crate's rule set can have, enumerated.
///
/// Ten rows, and the enumeration is the point: the term can occupy a subject, an object,
/// the class slot of an `rdfs:domain` or an `rdfs:range`, the property slot of either, a
/// sub-property axiom, or the inside of a triple term, and each of those makes a DIFFERENT
/// rule fire. Choosing three of them and calling the interaction understood is exactly the
/// gap this table closes.
const REIFIER_INTERACTIONS: &[ReifierInteraction] = &[
    ReifierInteraction {
        what: "rdf:reifies in SUBJECT position is typed like any property (scm-op)",
        regime: Regime::OwlRl,
        positive: "reifies_subject_position",
        conclusion: Conclusion::Iris(RDF_REIFIES, OWL_EQUIVALENTPROPERTY, RDF_REIFIES),
        near_miss: "object_property",
    },
    ReifierInteraction {
        what: "rdf:reifies in OBJECT position is typed by the property's range (rdfs3)",
        regime: Regime::Rdfs,
        positive: "reifies_object_position",
        conclusion: Conclusion::Iris(RDF_REIFIES, RDF_TYPE, EX_B),
        near_miss: "range",
    },
    ReifierInteraction {
        what: "rdf:reifies as the OBJECT OF rdfs:domain is read as a class (rdfs2)",
        regime: Regime::Rdfs,
        positive: "reifies_as_domain_class",
        conclusion: Conclusion::Iris(EX_X, RDF_TYPE, RDF_REIFIES),
        near_miss: "domain",
    },
    ReifierInteraction {
        what: "rdf:reifies as the OBJECT OF rdfs:range is read as a class (rdfs3)",
        regime: Regime::Rdfs,
        positive: "reifies_as_range_class",
        conclusion: Conclusion::Iris(EX_Y, RDF_TYPE, RDF_REIFIES),
        near_miss: "range",
    },
    ReifierInteraction {
        what: "prp-dom over an annotation triple types the REIFIER",
        regime: Regime::OwlRl,
        positive: "reifies_domain",
        conclusion: Conclusion::Iris(EX_REIFIER, RDF_TYPE, EX_A),
        near_miss: "reifies_domain_near_miss",
    },
    ReifierInteraction {
        what: "prp-rng over an annotation triple types the REIFIED term",
        regime: Regime::OwlRl,
        positive: "reifies_range",
        conclusion: Conclusion::Iris(EX_T, RDF_TYPE, EX_B),
        near_miss: "reifies_range_near_miss",
    },
    ReifierInteraction {
        what: "prp-spo1 rewrites an annotation triple's predicate, triple term and all",
        regime: Regime::OwlRl,
        positive: "reifies_subproperty",
        conclusion: Conclusion::Quoted {
            subject: EX_REIFIER,
            predicate: EX_Q,
            qs: EX_A,
            qp: RDFS_SUBCLASSOF,
            qo: EX_B,
        },
        near_miss: "reifies_subproperty_near_miss",
    },
    ReifierInteraction {
        what: "scm-dom1 widens a domain declared ON rdf:reifies",
        regime: Regime::OwlRl,
        positive: "reifies_domain_widened",
        conclusion: Conclusion::Iris(RDF_REIFIES, RDFS_DOMAIN, EX_B),
        near_miss: "domain_widened",
    },
    ReifierInteraction {
        what: "scm-rng1 widens a range declared ON rdf:reifies",
        regime: Regime::OwlRl,
        positive: "reifies_range_widened",
        conclusion: Conclusion::Iris(RDF_REIFIES, RDFS_RANGE, EX_B),
        near_miss: "range_widened",
    },
    ReifierInteraction {
        what: "a REIFIER INSIDE A TRIPLE TERM is carried through as one opaque term",
        regime: Regime::OwlRl,
        positive: "reifies_inside_triple_term",
        conclusion: Conclusion::Quoted {
            subject: EX_X,
            predicate: EX_MENTIONS,
            qs: EX_REIFIER,
            qp: RDF_REIFIES,
            qo: EX_T,
        },
        near_miss: "reifies_inside_triple_term_near_miss",
    },
];

/// EVERY enumerated `rdf:reifies` interaction fires where it should and nowhere else.
#[test]
fn every_reifier_interaction_has_a_positive_and_a_near_miss() {
    for interaction in REIFIER_INTERACTIONS {
        let ReifierInteraction {
            what,
            regime,
            positive,
            conclusion,
            near_miss,
        } = *interaction;
        assert_ne!(
            positive, near_miss,
            "{what}: a near miss must be a DIFFERENT input"
        );
        let line = conclusion.line();
        assert!(
            closure_lines(positive, regime).contains(&line),
            "{what}: {positive} did not reach {line} under {regime:?}"
        );
        assert!(
            !closure_lines(near_miss, regime).contains(&line),
            "{what}: near miss {near_miss} reached {line} anyway under {regime:?}"
        );
    }
    // The table really is an ENUMERATION and not a sample: ten distinct positives, ten
    // distinct claims, and no row silently duplicating another's evidence.
    let positives: BTreeSet<&str> = REIFIER_INTERACTIONS.iter().map(|i| i.positive).collect();
    assert_eq!(positives.len(), REIFIER_INTERACTIONS.len());
    let claims: BTreeSet<String> = REIFIER_INTERACTIONS
        .iter()
        .map(|i| i.conclusion.line())
        .collect();
    assert_eq!(claims.len(), REIFIER_INTERACTIONS.len());
}

/// NO RULE OF ANY LANE MENTIONS `rdf:reifies`, which is why the table above is a statement
/// about ORDINARY data rather than about special handling.
///
/// Six of the ten rows differ from their near miss by replacing `rdf:reifies` with
/// `example.org/p` and nothing else, so a lane that special-cased the term would break
/// them. This is the same claim made from the other side, against the declared calculus
/// itself: not one clause of any regime names the reifier property. What the RDFS lane DOES
/// say about it is three axiomatic PREMISES, and those are data too.
#[test]
fn no_clause_of_any_lane_mentions_the_reifier_property() {
    for regime in ORACLE_REGIMES.map(|(regime, _)| regime) {
        for clause in purrdf_entail::calculus_program(regime) {
            for atom in clause.body().iter().chain(clause.head_atoms()) {
                for term in atom.terms() {
                    assert_ne!(
                        term.surface().as_deref(),
                        Some(format!("<{RDF_REIFIES}>").as_str()),
                        "{regime:?}: a clause names rdf:reifies, so the reifier property is \
                         no longer ordinary data and the enumeration above no longer says \
                         what it claims"
                    );
                }
            }
        }
    }
}

/// A TRIPLE TERM IS NOT UN-QUOTED BY AN ANNOTATION TRIPLE ABOUT IT.
///
/// `reifies_domain` asserts `ex:reifier rdf:reifies <<( A rdfs:subClassOf B )>>` and never
/// asserts `A rdfs:subClassOf B`. No closure of any regime may hold it: reifying a triple
/// says something ABOUT it, not that it holds, and a chase that looked inside the term
/// would be asserting the ontology's own annotations as axioms.
#[test]
fn a_reified_triple_is_not_asserted_by_reifying_it() {
    for (regime, _) in ORACLE_REGIMES {
        let lines = closure_lines("reifies_domain", regime);
        assert!(
            !lines.contains(&nquads_line(EX_A, RDFS_SUBCLASSOF, EX_B)),
            "{regime:?}: reifying a triple asserted it"
        );
        // …and nothing DOWNSTREAM of that non-assertion appeared either: had the quoted
        // subClassOf been asserted, rdfs9 / cax-sco would have typed the reifier's own
        // domain typing through it. `ex:reifier rdf:type ex:B` is that consequence, and it
        // is absent because its premise is.
        assert!(
            !lines.contains(&nquads_line(EX_REIFIER, RDF_TYPE, EX_B)),
            "{regime:?}: a consequence of the quoted triple reached the closure"
        );
    }
}

/// A RANGE ON `rdf:reifies` CONCLUDES INTO A TRIPLE TERM'S SUBJECT POSITION, WHICH RDF 1.2
/// CANNOT HOLD — so the conclusion is abandoned and the run says so.
///
/// `reifies_domain`'s annotation triple has a TRIPLE-TERM object, and the RDFS lane seeds
/// `rdf:reifies rdfs:range rdfs:Proposition` as an axiom, so `rdfs3` really does derive
/// `<<( A rdfs:subClassOf B )>> rdf:type rdfs:Proposition`. That is a generalized-RDF triple:
/// it is derived in the evaluator's own term space, dropped when the answer is materialized,
/// and reported. Both halves are asserted — nothing in the closure has a triple term in
/// subject position, and the boundary is on the report — because the failure mode is the
/// drop happening silently.
#[test]
fn a_reifier_range_conclusion_over_a_triple_term_is_abandoned_and_reported() {
    let ds = build(fixture("reifies_domain"));
    let (closed, report) = materialize(&ds, Regime::Rdfs).expect("rdfs");
    assert!(
        report
            .boundaries()
            .iter()
            .any(|b| b.construct().as_str() == "generalized-rdf"),
        "the axiomatic range of rdf:reifies did not reach the triple term"
    );
    assert!(
        report
            .boundaries()
            .iter()
            .any(|b| b.construct().as_str() == "triple-term"),
        "the input holds a triple term and the report must say so"
    );
    for line in canonicalize(&closed).nquads.lines() {
        assert!(
            !line.starts_with("<<("),
            "a triple term reached subject position: {line}"
        );
    }
}

/// A REIFIER SIDE-TABLE ROUND-TRIPS BYTE-IDENTICALLY THROUGH `materialize`.
///
/// The RDF 1.2 IR carries reifier bindings and annotations in SIDE TABLES rather than as
/// quads, and `purrdf_core::canonicalize` renders each as a sentinel row so the two stay
/// observable in the canonical form. A closure is a NEW dataset built from the input plus
/// the inferred triples, so the side tables have to be carried across that rebuild — and
/// nothing had ever asserted that they are. If they were dropped, every reifier in a
/// caller's data would vanish the moment they asked for entailment, and no assertion about
/// the quads would notice.
///
/// Every term here is an IRI on purpose: RDFC-1.0 assigns blank-node labels itself, so a
/// blank reifier would make this a test of the canonicalizer's labelling under a changed
/// quad set rather than of the side table surviving. The COUNTS are checked for a blank
/// reifier too, below, which is the part that does not depend on labelling.
#[test]
fn a_reifier_side_table_round_trips_through_materialize() {
    let mut b = RdfDatasetBuilder::new();
    let reifier = b.intern_iri(EX_REIFIER);
    let s = b.intern_iri(EX_A);
    let p = b.intern_iri(RDFS_SUBCLASSOF);
    let o = b.intern_iri(EX_B);
    let triple = b.intern_triple(s, p, o);
    let says = b.intern_iri(EX_SAYS);
    let x = b.intern_iri(EX_X);
    b.push_quad(s, p, o, None);
    b.push_quad(x, says, triple, None);
    b.push_reifier(reifier, triple);
    b.push_annotation(reifier, says, x);
    let ds = b.freeze().expect("freeze");

    /// The sentinel rows `purrdf_core::canonicalize` writes for the two side tables.
    fn overlay(nquads: &str) -> Vec<String> {
        nquads
            .lines()
            .filter(|line| line.contains("urn:purrdf:rdfc:"))
            .map(ToOwned::to_owned)
            .collect()
    }
    let before = overlay(&canonicalize(&ds).nquads);
    assert_eq!(
        before.len(),
        2,
        "the fixture must exercise BOTH side tables"
    );

    for (regime, name) in ORACLE_REGIMES {
        let (closed, _) = materialize(&ds, regime).expect("the five oracle regimes run");
        assert_eq!(
            overlay(&canonicalize(&closed).nquads),
            before,
            "{name}: the reifier side table did not survive materialization"
        );
        assert_eq!(closed.reifier_refs().count(), 1, "{name}");
        assert_eq!(closed.annotation_refs().count(), 1, "{name}");
    }

    // The same claim for a BLANK-node reifier, as counts rather than bytes: RDFC-1.0
    // assigns the label, and the closure's quad set is not the input's, so the label may
    // legitimately differ. What may not differ is that the rows are still there.
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(EX_A);
    let p = b.intern_iri(RDFS_SUBCLASSOF);
    let o = b.intern_iri(EX_B);
    let triple = b.intern_triple(s, p, o);
    let says = b.intern_iri(EX_SAYS);
    let x = b.intern_iri(EX_X);
    let blank = b.intern_blank("r", purrdf_core::BlankScope::DEFAULT);
    b.push_quad(x, says, triple, None);
    b.push_reifier(blank, triple);
    b.push_annotation(blank, says, x);
    let ds = b.freeze().expect("freeze");
    for (regime, name) in ORACLE_REGIMES {
        let (closed, _) = materialize(&ds, regime).expect("the five oracle regimes run");
        assert_eq!(closed.reifier_refs().count(), 1, "{name}");
        assert_eq!(closed.annotation_refs().count(), 1, "{name}");
        assert_eq!(
            overlay(&canonicalize(&closed).nquads).len(),
            2,
            "{name}: a blank-node reifier's side-table rows went missing"
        );
    }
}

/// A triple term is one atomic term to the chase — a reported boundary — and a conclusion
/// built AROUND it carries it through unchanged.
///
/// `x says <<( A ⊑ B )>>` with `says ⊑ mentions` makes rdfs7 / prp-spo1 conclude
/// `x mentions <<( A ⊑ B )>>`: the rule rewrites the PREDICATE and copies the object
/// through, so the object of the conclusion is the object of the premise, whatever kind of
/// term that is.
///
/// The engine used to emit `x mentions rdfs:Resource` instead — the re-interning path
/// folded any triple term to `rdfs:Resource` on the way back into the dataset builder, on
/// the stated assumption that "the RDFS/OWL-RL rules never derive" one there. rdfs7 does,
/// and the substitution was UNSOUND: `x mentions rdfs:Resource` is entailed by this input
/// under none of these regimes, so it was a wrong triple rather than a missing one. Both
/// halves are asserted below — the licensed conclusion present, the fabricated one absent —
/// so a regression in either direction fails here and not only in the golden.
///
/// Opacity itself is NOT the bug and is not repaired: the chase still never reasons INTO
/// the quoted triple. rdfs14 / rdfs14a do fire over it — each concludes about a fresh
/// surrogate blank node the answer may not bind, so the conclusion is withheld and counted
/// rather than materialized — which withholds conclusions rather than inventing them, and
/// the triple-term boundary is what tells a caller so.
#[test]
fn a_derived_triple_term_object_is_carried_through_not_folded() {
    let ds = build(fixture("triple_term"));
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        assert!(
            report
                .boundaries()
                .iter()
                .any(|b| b.construct().as_str() == "triple-term"),
            "{regime:?} did not report the triple-term boundary"
        );
        let lines = canonicalize(&closed).nquads;
        assert!(
            lines.contains(&format!(
                "<{EX_X}> <{EX_MENTIONS}> <<( <{EX_A}> <{RDFS_SUBCLASSOF}> <{EX_B}> )>> ."
            )),
            "{regime:?}: rdfs7 did not carry the triple term through the rewrite"
        );
        assert!(
            !lines.contains(&nquads_line(EX_X, EX_MENTIONS, RDFS_RESOURCE)),
            "{regime:?}: the unsound fold to rdfs:Resource is back"
        );
        // The exact derived set about `x mentions`: one conclusion, and it is that one.
        let mentions: Vec<&str> = lines
            .lines()
            .filter(|line| line.starts_with(&format!("<{EX_X}> <{EX_MENTIONS}> ")))
            .collect();
        assert_eq!(
            mentions,
            vec![format!(
                "<{EX_X}> <{EX_MENTIONS}> <<( <{EX_A}> <{RDFS_SUBCLASSOF}> <{EX_B}> )>> ."
            )],
            "{regime:?}: the rewrite concluded more than the one licensed triple"
        );
    }
}

/// The deep chain closes: several rounds of the fixpoint are genuinely required.
#[test]
fn a_deep_subclass_chain_needs_several_rounds() {
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let lines = closure_lines("subclass_chain", regime);
        // A ⊑ F is five edges away; x a F is that plus one type hop.
        assert!(
            lines.contains(&nquads_line(EX_A, RDFS_SUBCLASSOF, EX_F)),
            "{regime:?}"
        );
        assert!(
            lines.contains(&nquads_line(EX_X, RDF_TYPE, EX_F)),
            "{regime:?}"
        );
        for class in [EX_B, EX_C, EX_D, EX_E, EX_F] {
            assert!(
                lines.contains(&nquads_line(EX_X, RDF_TYPE, class)),
                "{regime:?}"
            );
        }
    }
}

/// Two rules conclude the same triple; exactly one is credited, and the totals still add up.
#[test]
fn a_shared_conclusion_is_credited_once() {
    let ds = build(fixture("shared_conclusion"));
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        assert!(
            canonicalize(&closed)
                .nquads
                .contains(&nquads_line(EX_X, RDF_TYPE, EX_C)),
            "{regime:?} did not conclude the shared triple"
        );
        // The invariant that makes the tally checkable: one count is one triple this rule
        // was FIRST to add, so the counts sum to the inferred triples — no double credit.
        let inferred = closed.quad_refs().count() - ds.quad_refs().count();
        let total: u64 = report.rules_fired().iter().map(|&(_, n)| n).sum();
        assert_eq!(
            usize::try_from(total).expect("count fits usize"),
            inferred,
            "{regime:?}: a shared conclusion was credited twice"
        );
    }
}

/// DOCUMENTED DIVERGENCE 1 — NARROWER CONCLUSIONS. A would-be literal subject is
/// abandoned, counted, and reported.
///
/// THE OBSERVABLE AT RISK IS THE BOUNDARY, NOT THE TRIPLES. No engine may put a literal in
/// subject position, so `"cat" rdf:type A` stays absent whatever runs — the closure could
/// not have moved and did not. What could have vanished silently is the EVIDENCE: the
/// evaluator derives the generalized triple in its own term space and meets the RDF 1.2 IR
/// only when the answer is materialized, so a materializer that simply skipped what it
/// could not represent would produce a closure that looks exactly right and a report that
/// no longer says anything was dropped. The boundary assertion below is the guard against
/// precisely that, and it is the reason this test asserts the report and not only the
/// quads.
#[test]
fn a_would_be_literal_subject_is_abandoned_and_reported() {
    let ds = build(fixture("divergence_literal_subject"));
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let (closed, report) = materialize(&ds, regime).expect("runnable regime");
        let nquads = canonicalize(&closed).nquads;
        // The literal is still in the closure — it is an input OBJECT. What may never
        // appear is a line that STARTS with it.
        assert!(
            nquads.contains(&format!("<{EX_X}> <{EX_P}> \"cat\"")),
            "{regime:?}: the input triple must survive the closure"
        );
        assert!(
            !nquads.lines().any(|line| line.starts_with('"')),
            "{regime:?}: no conclusion may put a literal in subject position"
        );
        assert!(
            !nquads.contains(&format!("<{RDF_TYPE}> <{EX_A}> .")),
            "{regime:?}: rdfs3's only candidate conclusion was the abandoned one"
        );
        assert!(
            report
                .boundaries()
                .iter()
                .any(|b| b.construct().as_str() == "generalized-rdf"),
            "{regime:?}: the abandoned conclusion must be reported, not silently dropped"
        );
        assert!(!report.overclaims(), "{regime:?}");
    }
}

/// DOCUMENTED DIVERGENCE 2 — BROADER TRIGGERS, RESOLVED, AND THE LONGER PATH WALKED.
///
/// `divergence_broad_triggers` is `A rdfs:subClassOf B` and `x p y`, and nothing in it is
/// ASSERTED to be an `rdfs:Class` or an `rdf:Property`. It has now been the control for
/// two different questions, and the difference between them is the whole point of this
/// test.
///
/// The hand-written chase emitted `A ⊑ A`, `B ⊑ B` for every subClassOf ENDPOINT and
/// `p ⊑ p` for every PREDICATE. Narrowing `rdfs6` and `rdfs10` to their specification
/// premises removed those, correctly — but it removed real entailments with them, because
/// the triples are RDFS-entailed by a LONGER path the chase did not walk. `rdfs:subClassOf`
/// has an axiomatic `rdfs:domain` and `rdfs:range` of `rdfs:Class`, so `rdfs2` and `rdfs3`
/// type both endpoints, and only THEN does `rdfs10` license the reflexive triple; `rdfD2`
/// types the predicate, and only then does `rdfs6`.
///
/// So the contract this test states is neither "the shortcut" nor "nothing". It is:
///
/// * `OWL-RL` — still nothing about the hierarchies. OWL 2 Profiles §4.3 omits the RDF and
///   RDFS axiomatic triples, so that lane has no premise to reach, and beyond the
///   premise-free block its closure is the input plus `eq-ref`'s reflexive `owl:sameAs`
///   for each of the input's five terms. That addition is Table 4's and is licensed by
///   `eq-ref`'s own premise — every term of every triple — rather than by anything about
///   `rdfs:Class` or `rdf:Property`, which is why it does not blunt the control: the
///   `rdfs:subClassOf` and `rdfs:subPropertyOf` shortcut must not come back.
/// * `RDFS` — the reflexive triples are back, each with its premise DERIVED and present
///   in the same closure, so the path is checkable rather than asserted.
/// * And the property that separates "walked the path" from "went back to the shortcut":
///   `x` and `y` are the subject and object of an `ex:p` triple whose predicate declares
///   no domain and no range, so nothing types them `rdfs:Class` — and `x ⊑ x` and
///   `y ⊑ y` must therefore still be absent, under BOTH lanes.
#[test]
fn the_reflexive_rules_fire_on_their_licensed_premises_and_the_axioms_supply_them() {
    // OWL-RL asserts no axiomatic triples, so its lane has no path to the premises of
    // rdfs6 and rdfs10 and this input still entails nothing about the SUB-CLASS and
    // SUB-PROPERTY hierarchies at all. The premise-free block — `prp-ap`'s nine typings,
    // `cls-thing`, `cls-nothing1`, `dt-type1`'s thirty-two, and everything `scm-cls` and
    // `eq-ref` draw from those — holds for every input including the empty graph, so it is
    // subtracted here rather than listed. That subtraction is what keeps this a statement
    // about the DATA, and `the_rdfs_closure_of_every_fixture_contains_the_empty_one`
    // asserts that the subtracted block really is input-independent.
    //
    // What remains is the input, plus one thing that is new with Table 4 and belongs here:
    // `eq-ref` types every term of every triple `owl:sameAs` ITSELF, which is five terms
    // for these two triples. Those five are conclusions about the DATA and so are not
    // subtracted, and they are exactly why this residue grew — the reflexive owl:sameAs
    // assertions are licensed by eq-ref's own premise `T(?s, ?p, ?o)`, which the input
    // supplies directly, where the reflexive rdfs:subClassOf and rdfs:subPropertyOf
    // assertions are licensed only through axiomatic triples this lane does not assert.
    // The two reflexivities are therefore separable, and the point of this fixture — that
    // OWL-RL says nothing about the hierarchies here — survives verbatim.
    let premise_free = closure_lines("empty", Regime::OwlRl);
    let owl: BTreeSet<String> = closure_lines("divergence_broad_triggers", Regime::OwlRl)
        .difference(&premise_free)
        .cloned()
        .collect();
    assert_eq!(
        owl,
        [
            nquads_line(EX_A, RDFS_SUBCLASSOF, EX_B),
            nquads_line(EX_X, EX_P, EX_Y),
            nquads_line(EX_A, OWL_SAMEAS, EX_A),
            nquads_line(EX_B, OWL_SAMEAS, EX_B),
            nquads_line(EX_P, OWL_SAMEAS, EX_P),
            nquads_line(EX_X, OWL_SAMEAS, EX_X),
            nquads_line(EX_Y, OWL_SAMEAS, EX_Y),
        ]
        .into_iter()
        .collect::<BTreeSet<String>>(),
        "OWL-RL: this input entails its own two triples, eq-ref's reflexive owl:sameAs for \
         each of their five terms, and — under a lane that omits the axiomatic triples — \
         nothing else"
    );

    // RDFS reaches the premises through the axioms, and only then draws the conclusions.
    let rdfs = closure_lines("divergence_broad_triggers", Regime::Rdfs);
    for (s, p, o) in [
        // The PREMISES, derived: `rdfs:subClassOf` has an axiomatic domain and range of
        // `rdfs:Class` (rdfs2, rdfs3), and `p` is a predicate (rdfD2).
        (EX_A, RDF_TYPE, RDFS_CLASS),
        (EX_B, RDF_TYPE, RDFS_CLASS),
        (EX_P, RDF_TYPE, RDF_PROPERTY),
        // …and only then the CONCLUSIONS the reflexive rules license from them.
        (EX_A, RDFS_SUBCLASSOF, EX_A),
        (EX_B, RDFS_SUBCLASSOF, EX_B),
        (EX_P, RDFS_SUBPROPERTYOF, EX_P),
    ] {
        assert!(
            rdfs.contains(&nquads_line(s, p, o)),
            "RDFS: <{s}> <{p}> <{o}> is entailed through the axiomatic triples and is missing"
        );
    }

    // THE CONTROL. `x` and `y` are an ordinary subject and object; `ex:p` declares no
    // domain and no range, so nothing types them `rdfs:Class` and the reflexive rule has
    // no premise about them. A chase that had merely restored the endpoint shortcut would
    // fail here — under RDFS as well as under OWL-RL.
    for regime in [Regime::Rdfs, Regime::OwlRl] {
        let lines = closure_lines("divergence_broad_triggers", regime);
        for term in [EX_X, EX_Y] {
            assert!(
                !lines.contains(&nquads_line(term, RDF_TYPE, RDFS_CLASS)),
                "{regime:?}: <{term}> is typed rdfs:Class on no premise"
            );
            assert!(
                !lines.contains(&nquads_line(term, RDFS_SUBCLASSOF, term)),
                "{regime:?}: <{term}> rdfs:subClassOf itself is emitted on no premise"
            );
        }

        // NARROWED, NOT SWITCHED OFF: given the premise outright, each rule still fires.
        assert!(
            closure_lines("class_typed", regime).contains(&nquads_line(
                EX_C,
                RDFS_SUBCLASSOF,
                EX_C
            )),
            "{regime:?}: rdfs10 no longer fires on an rdfs:Class instance"
        );
        assert!(
            closure_lines("property_typed", regime).contains(&nquads_line(
                EX_P,
                RDFS_SUBPROPERTYOF,
                EX_P
            )),
            "{regime:?}: rdfs6 no longer fires on an rdf:Property instance"
        );
    }
}

/// EVERY fixture's RDFS closure contains the empty graph's RDFS closure, exactly.
///
/// The `RDFS` lane asserts the axiomatic triples as premises, and what they entail about
/// the RDF and RDFS vocabulary itself does not depend on the input at all: `rdfs4` types
/// the axioms' own terms, `rdfs2` / `rdfs3` type their subjects and objects, and the
/// reflexive rules close over those. So `empty.golden`'s 113-line closure is a LOWER BOUND
/// on every other, and that is the fact each golden's accounting leans on when it reports
/// only the lines beyond it.
///
/// It is also the guard against the accounting going stale: a change that made any of
/// those 113 lines input-dependent would leave thirty-two goldens claiming a shared block
/// they no longer share, and it would fail here rather than being spread thinly across
/// thirty-two diffs.
#[test]
fn the_rdfs_closure_of_every_fixture_contains_the_empty_one() {
    let base = closure_lines("empty", Regime::Rdfs);
    assert_eq!(
        base.len(),
        113,
        "the input-independent block every golden's accounting names"
    );
    for fixture in CORPUS {
        let lines = closure_lines(fixture.name, Regime::Rdfs);
        let absent: Vec<&String> = base.difference(&lines).collect();
        assert!(
            absent.is_empty(),
            "{}: the axiomatic block is not input-independent — {absent:?} is missing",
            fixture.name
        );
    }
    // And it really is the AXIOMS that put the 113 lines there: no other lane asserts
    // them, so no other lane has them. `Simple` copies the input, so it closes the empty
    // graph into nothing at all.
    assert!(
        closure_lines("empty", Regime::Simple).is_empty(),
        "Simple closed the empty graph into something"
    );
    // `RDF` closes it into exactly ONE line, and that line is a genuine entailment of the
    // EMPTY graph rather than a leak. `rdfD1a` is premise-free — "for any graph, even the
    // empty one, `_:nnn rdf:type ddd` holds for each recognized `ddd`" — so the closure
    // really does use `rdf:type` as a predicate, and `rdfD2` types every predicate an
    // `rdf:Property`. The surrogate ITSELF is withheld (a SPARQL entailment regime does
    // not answer with one), which is why this is one line and not four.
    let rdf = closure_lines("empty", Regime::Rdf);
    assert_eq!(
        rdf.iter().collect::<Vec<_>>(),
        vec![&format!("<{RDF_TYPE}> <{RDF_TYPE}> <{RDF_PROPERTY}> .")],
        "the RDF lane's empty-graph closure"
    );
    the_owl_rl_closure_of_every_fixture_contains_the_empty_one();
}

/// The `OWL-RL` half of the same claim, over the block FOUR premise-free rules now put in
/// every closure.
///
/// `OWL-RL` omits the RDF and RDFS axiomatic triples by OWL 2 Profiles §4.3's own choice, so
/// its empty-graph closure is not the 113-line RDFS block; it is what four premise-free
/// rules and their consequences make, and it is pinned in four layers.
///
/// The two layers whose content is a SPECIFICATION'S LIST are pinned by NAME rather than by
/// count, because a name is checkable against the document and a count is not — a tenth
/// annotation property or a thirty-third datatype would be an invented one:
///
/// * `prp-ap` types the nine built-in annotation properties OWL 2 Structural Specification
///   §5.5 fixes;
/// * `dt-type1` types the thirty-two datatypes OWL 2 Profiles §4.2.1 lists as supported in
///   OWL 2 RL. `owl:real` and `owl:rational` are in the OWL 2 datatype map and are
///   deliberately NOT in that list, so both are named here as absences.
///
/// The two layers that are DERIVED are pinned as derivations rather than as literals, so
/// they cannot be updated by transcription when the layer beneath them moves:
///
/// * `cls-thing` and `cls-nothing1` type `owl:Thing` and `owl:Nothing` an `owl:Class`, and
///   `scm-cls` says exactly four things about each `owl:Class` — five distinct triples over
///   those two, because three of the eight coincide;
/// * `eq-ref` types every term of every triple `owl:sameAs` itself, so the reflexive block
///   is a FUNCTION of the rest of the closure: one assertion per distinct term, and the only
///   term the reflexive assertions themselves introduce is `owl:sameAs`.
///
/// The whole is then asserted to be a LOWER BOUND on every other `OWL-RL` closure in the
/// corpus, which is the fact each golden's accounting leans on when it reports only the
/// lines beyond it.
fn the_owl_rl_closure_of_every_fixture_contains_the_empty_one() {
    /// OWL 2 Structural Specification §5.5 — the built-in annotation properties.
    const BUILT_IN_ANNOTATION_PROPERTIES: [&str; 9] = [
        "http://www.w3.org/2000/01/rdf-schema#label",
        "http://www.w3.org/2000/01/rdf-schema#comment",
        "http://www.w3.org/2000/01/rdf-schema#seeAlso",
        "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
        "http://www.w3.org/2002/07/owl#deprecated",
        "http://www.w3.org/2002/07/owl#versionInfo",
        "http://www.w3.org/2002/07/owl#priorVersion",
        "http://www.w3.org/2002/07/owl#backwardCompatibleWith",
        "http://www.w3.org/2002/07/owl#incompatibleWith",
    ];
    /// OWL 2 Profiles §4.2.1 — the datatypes supported in OWL 2 RL, transcribed
    /// independently of the crate's own list so the two are a cross-check rather than one
    /// list read twice.
    const SUPPORTED_DATATYPES: [&str; 32] = [
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral",
        "http://www.w3.org/2000/01/rdf-schema#Literal",
        "http://www.w3.org/2001/XMLSchema#decimal",
        "http://www.w3.org/2001/XMLSchema#integer",
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger",
        "http://www.w3.org/2001/XMLSchema#nonPositiveInteger",
        "http://www.w3.org/2001/XMLSchema#positiveInteger",
        "http://www.w3.org/2001/XMLSchema#negativeInteger",
        "http://www.w3.org/2001/XMLSchema#long",
        "http://www.w3.org/2001/XMLSchema#int",
        "http://www.w3.org/2001/XMLSchema#short",
        "http://www.w3.org/2001/XMLSchema#byte",
        "http://www.w3.org/2001/XMLSchema#unsignedLong",
        "http://www.w3.org/2001/XMLSchema#unsignedInt",
        "http://www.w3.org/2001/XMLSchema#unsignedShort",
        "http://www.w3.org/2001/XMLSchema#unsignedByte",
        "http://www.w3.org/2001/XMLSchema#float",
        "http://www.w3.org/2001/XMLSchema#double",
        "http://www.w3.org/2001/XMLSchema#string",
        "http://www.w3.org/2001/XMLSchema#normalizedString",
        "http://www.w3.org/2001/XMLSchema#token",
        "http://www.w3.org/2001/XMLSchema#language",
        "http://www.w3.org/2001/XMLSchema#Name",
        "http://www.w3.org/2001/XMLSchema#NCName",
        "http://www.w3.org/2001/XMLSchema#NMTOKEN",
        "http://www.w3.org/2001/XMLSchema#boolean",
        "http://www.w3.org/2001/XMLSchema#hexBinary",
        "http://www.w3.org/2001/XMLSchema#base64Binary",
        "http://www.w3.org/2001/XMLSchema#anyURI",
        "http://www.w3.org/2001/XMLSchema#dateTime",
        "http://www.w3.org/2001/XMLSchema#dateTimeStamp",
    ];

    let owl_empty = closure_lines("empty", Regime::OwlRl);

    // LAYER 1 and 2 — the two specification lists, plus the two `owl:Class` typings, are
    // the whole of the closure's `rdf:type` lines. Equality, so nothing is typed that these
    // four rules do not type.
    let mut typings: BTreeSet<String> = BUILT_IN_ANNOTATION_PROPERTIES
        .into_iter()
        .map(|property| nquads_line(property, RDF_TYPE, OWL_ANNOTATIONPROPERTY))
        .collect();
    typings.extend(
        SUPPORTED_DATATYPES
            .into_iter()
            .map(|datatype| nquads_line(datatype, RDF_TYPE, RDFS_DATATYPE)),
    );
    typings.insert(nquads_line(OWL_THING, RDF_TYPE, OWL_CLASS));
    typings.insert(nquads_line(OWL_NOTHING, RDF_TYPE, OWL_CLASS));
    let found_typings: BTreeSet<String> = owl_empty
        .iter()
        .filter(|line| line.contains(&format!("> <{RDF_TYPE}> <")))
        .cloned()
        .collect();
    assert_eq!(
        found_typings, typings,
        "OWL-RL's empty-graph typings are exactly prp-ap's nine, dt-type1's thirty-two, and \
         cls-thing's and cls-nothing1's one each"
    );
    // The two datatypes OWL 2 RL deliberately does NOT support, named as absences.
    for unsupported in [
        "http://www.w3.org/2002/07/owl#real",
        "http://www.w3.org/2002/07/owl#rational",
    ] {
        assert!(
            !owl_empty.contains(&nquads_line(unsupported, RDF_TYPE, RDFS_DATATYPE)),
            "dt-type1 typed {unsupported}, which OWL 2 Profiles §4.2.1 excludes"
        );
    }

    // LAYER 3 — scm-cls over the two classes cls-thing and cls-nothing1 assert. Four
    // conclusions each; `owl:Thing rdfs:subClassOf owl:Thing` and
    // `owl:Nothing rdfs:subClassOf owl:Nothing` are each drawn twice and
    // `owl:Nothing rdfs:subClassOf owl:Thing` three times, so five triples remain.
    let schema: BTreeSet<String> = [
        nquads_line(OWL_THING, RDFS_SUBCLASSOF, OWL_THING),
        nquads_line(OWL_THING, OWL_EQUIVALENTCLASS, OWL_THING),
        nquads_line(OWL_NOTHING, RDFS_SUBCLASSOF, OWL_NOTHING),
        nquads_line(OWL_NOTHING, RDFS_SUBCLASSOF, OWL_THING),
        nquads_line(OWL_NOTHING, OWL_EQUIVALENTCLASS, OWL_NOTHING),
    ]
    .into_iter()
    .collect();

    // LAYER 4 — eq-ref, DERIVED from the three layers below it rather than transcribed.
    let named: BTreeSet<String> = typings
        .iter()
        .chain(&schema)
        .flat_map(|line| iri_terms(line))
        .chain(std::iter::once(OWL_SAMEAS.to_owned()))
        .collect();
    let reflexive: BTreeSet<String> = named
        .iter()
        .map(|term| nquads_line(term, OWL_SAMEAS, term))
        .collect();

    let expected: BTreeSet<String> = typings
        .iter()
        .chain(&schema)
        .chain(&reflexive)
        .cloned()
        .collect();
    assert_eq!(
        owl_empty, expected,
        "OWL-RL's empty-graph closure is exactly the four premise-free rules, what scm-cls \
         draws from two of them, and eq-ref over all of it"
    );

    // …and, as with the RDFS block, it is a LOWER BOUND on every other OWL-RL closure.
    for fixture in CORPUS {
        let lines = closure_lines(fixture.name, Regime::OwlRl);
        let absent: Vec<&String> = owl_empty.difference(&lines).collect();
        assert!(
            absent.is_empty(),
            "{}: the premise-free block is not input-independent — {absent:?} is missing",
            fixture.name
        );
    }
}

/// The three IRIs of a canonical N-Quads line over three IRIs, without their brackets.
///
/// Only ever applied to the `OWL-RL` empty-graph closure, every line of which is a
/// default-graph triple over three IRIs — a shape the assertion above checks by
/// construction, since it builds the same lines from the specification's own names.
fn iri_terms(line: &str) -> Vec<String> {
    line.trim_end_matches(" .")
        .split_whitespace()
        .map(|term| term.trim_matches(['<', '>']).to_owned())
        .collect()
}

/// The corpus reaches every rule the chase fires, by ATTRIBUTION rather than by outcome.
///
/// The registry above asserts that each rule's conclusion is present; this asserts that
/// the rule was CREDITED for it. The two are not the same claim — `shared_conclusion` is
/// the case that separates them, where one triple has two possible producers and only the
/// first is credited — so a rule could pass the registry on a triple some other rule
/// actually derived. The union of `rules_fired` over the whole corpus closes that gap.
///
/// The expected set is `implemented(regime)`, plus — for `OWL-RL` only — the three
/// RDFS-shaped rules that lane fires under no OWL 2 RL name, MINUS the twenty rules that
/// can never appear in `rules_fired` at all. Equality, not containment: a rule credited
/// that the inventory does not list is as much a defect as one the corpus never reaches.
///
/// # Why twenty rules are subtracted, and why that is not a hole
///
/// `rules_fired` counts triples a rule was credited with ADDING to the closure, so a rule
/// that adds no triple can never be in it. Two families cannot, for two different reasons,
/// and neither is a gap in the corpus's coverage — each is evidenced by the registry state
/// that fits what the rule actually does:
///
/// * the eighteen [`RuleFixtures::Refuting`] rules conclude `false` (or, for `dt-diff`,
///   conclude only into `eq-diff1`'s premise). A body match REFUSES the run, so there is no
///   report to credit anything in — `materialize` returned an error instead of a report —
///   and the evidence is the refusal plus its named witness;
/// * the two [`RuleFixtures::Generalized`] rules, `dt-type2` and `dt-eq`, conclude only
///   triples with a LITERAL SUBJECT. Every one of those is derived in the evaluator's own
///   term space and abandoned at the materialization boundary, and a conclusion the RDF 1.2
///   IR cannot hold is credited to nobody by construction — it is reported as the
///   generalized-rdf boundary instead, which is what those two rows assert.
///
/// The subtracted set is asserted to be EXACTLY the union of those two registry tables, so
/// the exemption list here and the registry cannot drift: adding a row to either table
/// without a reason, or removing one, moves both sides of that equality.
#[test]
fn every_rule_the_chase_fires_is_credited_somewhere_in_the_corpus() {
    /// The rules the `OWL-RL` lane fires that OWL 2 Profiles gives no rule id.
    const OWL_RL_RDFS_SHAPED_EXTRAS: [RuleId; 3] = [RuleId::Rdfs6, RuleId::Rdfs8, RuleId::Rdfs10];
    /// The rules that produce no creditable triple, BY NAME. Seventeen conclude `false`;
    /// `dt-diff` concludes only `owl:differentFrom` over literal subjects; `dt-type2` and
    /// `dt-eq` conclude only over literal subjects too.
    const UNCREDITABLE: [RuleId; 24] = [
        RuleId::RdfD1,
        RuleId::RdfD1a,
        RuleId::Rdfs14,
        RuleId::Rdfs14a,
        RuleId::EqDiff1,
        RuleId::EqDiff2,
        RuleId::EqDiff3,
        RuleId::PrpIrp,
        RuleId::PrpAsyp,
        RuleId::PrpPdw,
        RuleId::PrpAdp,
        RuleId::PrpNpa1,
        RuleId::PrpNpa2,
        RuleId::ClsNothing2,
        RuleId::ClsCom,
        RuleId::ClsMaxc1,
        RuleId::ClsMaxqc1,
        RuleId::ClsMaxqc2,
        RuleId::CaxDw,
        RuleId::CaxAdc,
        RuleId::DtType2,
        RuleId::DtEq,
        RuleId::DtDiff,
        RuleId::DtNotType,
    ];

    // The subtraction and the registry are the same statement, made twice.
    let by_registry: BTreeSet<RuleId> = OWL_RL_REFUTING_ROWS
        .iter()
        .map(|row| row.0)
        .chain(OWL_RL_GENERALIZED_ROWS.iter().map(|row| row.0))
        .chain(RDFS_WITHHELD_ROWS.iter().map(|row| row.0))
        .collect();
    assert_eq!(
        by_registry,
        UNCREDITABLE.into_iter().collect::<BTreeSet<RuleId>>(),
        "the rules exempted from attribution must be exactly the Refuting, Generalized and \
         Withheld registry rows, so an exemption cannot outlive the evidence that replaced \
         it"
    );

    for (regime, label) in ORACLE_REGIMES {
        let mut credited: BTreeSet<RuleId> = BTreeSet::new();
        for fixture in CORPUS {
            let ds = build(fixture);
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            credited.extend(report.rules_fired().iter().map(|&(rule, _)| rule));
        }
        let mut expected: BTreeSet<RuleId> = implemented(regime).iter().copied().collect();
        if matches!(regime, Regime::OwlRl) {
            expected.extend(OWL_RL_RDFS_SHAPED_EXTRAS);
        }
        for rule in UNCREDITABLE {
            expected.remove(&rule);
        }
        assert_eq!(
            credited, expected,
            "{label}: the corpus must credit exactly the rules this lane fires"
        );
    }
}

/// No run over this corpus ever overclaims — `Exact` while naming a boundary.
///
/// The crate's unit tests make this claim over their own fixtures; making it again over
/// every fixture here is cheap and widens the evidence to the awkward cases.
#[test]
fn no_report_over_the_corpus_overclaims() {
    for fixture in CORPUS {
        let ds = build(fixture);
        for (regime, name) in ORACLE_REGIMES {
            let (_, report) = materialize(&ds, regime).expect("runnable regime");
            assert!(
                !report.overclaims(),
                "{}/{name}: Exact alongside {:?}",
                fixture.name,
                report.boundaries()
            );
        }
    }
}
