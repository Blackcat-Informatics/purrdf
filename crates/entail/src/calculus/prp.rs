// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `prp-*` — the semantics of axioms about properties, OWL 2 Profiles §4.3 Table 5.
//!
//! This family holds the property-axiom rules the `OWL-RL` lane fires and the RDFS tables
//! never had: the built-in annotation properties, the property characteristics
//! (`owl:FunctionalProperty`, `owl:InverseFunctionalProperty`, `owl:IrreflexiveProperty`,
//! `owl:SymmetricProperty`, `owl:AsymmetricProperty`, `owl:TransitiveProperty`), property
//! chains, property equivalence and disjointness, `owl:inverseOf`, keys, and negative
//! property assertions.
//!
//! `prp-dom`, `prp-rng` and `prp-spo1` are NOT here. They are the OWL 2 RL names for
//! `rdfs2`, `rdfs3` and `rdfs7`, and one clause with two names is stated once, in
//! [`super::rdfs`], where the RDFS numbering orders it; [`super::ChaseRule::rule_id`]
//! answers with the OWL name under the `OWL-RL` lane.
//!
//! # The six rules stated here that conclude `false`
//!
//! `prp-irp`, `prp-asyp`, `prp-pdw`, `prp-adp`, `prp-npa1` and `prp-npa2` all conclude
//! `false`: a body match is an INCONSISTENCY WITNESS, not a triple. Each is declared with
//! `concludes: Inconsistency,` and the specification's own body, and
//! [`super::constraint_clause`] lowers it — mechanically — into a clause whose head is one
//! atom of the internal clash relation, which the evaluator runs like any other. A match
//! becomes [`EntailError::Inconsistent`](crate::EntailError) carrying the matched premises
//! as the witness. See the [calculus docs](super) for why that lowering puts nothing in
//! the closure.
//!
//! # The two rules that walk a list
//!
//! `prp-spo2` and `prp-key` are written with the meta-notation `LIST[?x, ?p1, …, ?pn]` and
//! then a body of `n` atoms — a conjunction whose LENGTH depends on the data, which no
//! fixed clause has. Both are nonetheless ordinary Datalog once the traversal is named: an
//! internal ternary relation recursing over `rdf:first` / `rdf:rest` accumulates the walk,
//! and a third clause reads the accumulated relation off the axiom's own list head. See
//! [`crate::lists`] for the internal relations and for why none of them is an IRI.
//!
//! The recursion is grounded at `rdf:nil` rather than started from it, because a clause
//! whose body never mentions its head's variables is not range-restricted: the base case is
//! the LAST cell (`rdf:rest rdf:nil`), which binds everything the head needs. A consequence
//! worth stating: the EMPTY chain and the EMPTY key list conclude nothing here. OWL 2
//! forbids both — a property chain has at least two properties and a key at least one — so
//! the encoding is complete for every ontology the specification admits.

use purrdf_datalog::clause::DlClause;

use super::{atom, internal, internal_graph, iri, quad, var};
use crate::lists::{AGREE_RELATION, CHAIN_RELATION, INDEX_DISTINCT_RELATION, LIST_RELATION};
use crate::vocab::{
    OWL_ALLDISJOINTPROPERTIES, OWL_ANNOTATIONPROPERTY, OWL_ASSERTIONPROPERTY,
    OWL_ASYMMETRICPROPERTY, OWL_BACKWARDCOMPATIBLEWITH, OWL_DEPRECATED, OWL_EQUIVALENTPROPERTY,
    OWL_FUNCTIONALPROPERTY, OWL_HASKEY, OWL_INCOMPATIBLEWITH, OWL_INVERSEFUNCTIONALPROPERTY,
    OWL_INVERSEOF, OWL_IRREFLEXIVEPROPERTY, OWL_MEMBERS, OWL_PRIORVERSION, OWL_PROPERTYCHAINAXIOM,
    OWL_PROPERTYDISJOINTWITH, OWL_SAMEAS, OWL_SOURCEINDIVIDUAL, OWL_SYMMETRICPROPERTY,
    OWL_TARGETINDIVIDUAL, OWL_TARGETVALUE, OWL_TRANSITIVEPROPERTY, OWL_VERSIONINFO, RDF_FIRST,
    RDF_NIL, RDF_REST, RDF_TYPE, RDFS_COMMENT, RDFS_ISDEFINEDBY, RDFS_LABEL, RDFS_SEEALSO,
};

/// The BUILT-IN annotation properties of OWL 2, which `prp-ap` types.
///
/// OWL 2 Structural Specification §5.5 fixes this list and OWL 2 Profiles §4.3 Table 5
/// refers to it as "each built-in annotation property of OWL 2 RL". It is therefore not a
/// default this crate invented: PurRDF mints no vocabulary, and every IRI here is
/// specification vocabulary the rule names.
const BUILT_IN_ANNOTATION_PROPERTIES: [&str; 9] = [
    RDFS_LABEL,
    RDFS_COMMENT,
    RDFS_SEEALSO,
    RDFS_ISDEFINEDBY,
    OWL_DEPRECATED,
    OWL_VERSIONINFO,
    OWL_PRIORVERSION,
    OWL_BACKWARDCOMPATIBLEWITH,
    OWL_INCOMPATIBLEWITH,
];

/// `prp-ap`: `ap rdf:type owl:AnnotationProperty` for each built-in annotation property.
///
/// One premise-free clause per property, in [`BUILT_IN_ANNOTATION_PROPERTIES`] order. The
/// rule quantifies over the OWL 2 RL vocabulary rather than over the graph, so it holds for
/// every input including the empty one, and a clause with no body is exactly that
/// statement — the same shape `rdfs1` takes in [`super::rdfs`].
pub(super) fn annotation_properties() -> Vec<DlClause> {
    BUILT_IN_ANNOTATION_PROPERTIES
        .into_iter()
        .map(|property| {
            DlClause::datalog(
                atom(iri(property), RDF_TYPE, iri(OWL_ANNOTATIONPROPERTY)),
                Vec::new(),
            )
        })
        .collect()
}

/// `prp-fp`: `?p rdf:type owl:FunctionalProperty`, `T(?x, ?p, ?y1)`, `T(?x, ?p, ?y2)` ⇒
/// `?y1 owl:sameAs ?y2`.
pub(super) fn functional() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y1"), OWL_SAMEAS, var("?y2")),
        vec![
            atom(var("?p"), RDF_TYPE, iri(OWL_FUNCTIONALPROPERTY)),
            quad(var("?x"), var("?p"), var("?y1")),
            quad(var("?x"), var("?p"), var("?y2")),
        ],
    )]
}

/// `prp-ifp`: `?p rdf:type owl:InverseFunctionalProperty`, `T(?x1, ?p, ?y)`,
/// `T(?x2, ?p, ?y)` ⇒ `?x1 owl:sameAs ?x2`.
pub(super) fn inverse_functional() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?x1"), OWL_SAMEAS, var("?x2")),
        vec![
            atom(var("?p"), RDF_TYPE, iri(OWL_INVERSEFUNCTIONALPROPERTY)),
            quad(var("?x1"), var("?p"), var("?y")),
            quad(var("?x2"), var("?p"), var("?y")),
        ],
    )]
}

/// `prp-irp`: `?p rdf:type owl:IrreflexiveProperty`, `T(?x, ?p, ?x)` ⇒ `false`.
pub(super) fn irreflexive() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?p"), RDF_TYPE, iri(OWL_IRREFLEXIVEPROPERTY)),
        quad(var("?x"), var("?p"), var("?x")),
    ])]
}

/// `prp-symp`: `?p rdf:type owl:SymmetricProperty`, `T(?x, ?p, ?y)` ⇒ `T(?y, ?p, ?x)`.
pub(super) fn symmetric() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?y"), var("?p"), var("?x")),
        vec![
            atom(var("?p"), RDF_TYPE, iri(OWL_SYMMETRICPROPERTY)),
            quad(var("?x"), var("?p"), var("?y")),
        ],
    )]
}

/// `prp-asyp`: `?p rdf:type owl:AsymmetricProperty`, `T(?x, ?p, ?y)`, `T(?y, ?p, ?x)` ⇒
/// `false`.
pub(super) fn asymmetric() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?p"), RDF_TYPE, iri(OWL_ASYMMETRICPROPERTY)),
        quad(var("?x"), var("?p"), var("?y")),
        quad(var("?y"), var("?p"), var("?x")),
    ])]
}

/// `prp-trp`: `?p rdf:type owl:TransitiveProperty`, `T(?x, ?p, ?y)`, `T(?y, ?p, ?z)` ⇒
/// `T(?x, ?p, ?z)`.
pub(super) fn transitive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?x"), var("?p"), var("?z")),
        vec![
            atom(var("?p"), RDF_TYPE, iri(OWL_TRANSITIVEPROPERTY)),
            quad(var("?x"), var("?p"), var("?y")),
            quad(var("?y"), var("?p"), var("?z")),
        ],
    )]
}

/// `prp-spo2`: `?p owl:propertyChainAxiom ?x` over the list `?p1 … ?pn`, matched by a chain
/// `T(?u1, ?p1, ?u2) … T(?un, ?pn, ?un+1)` ⇒ `T(?u1, ?p, ?un+1)`.
///
/// Three clauses. `CHAIN(?cell, ?u, ?v)` means "the sub-list from `?cell` onwards is
/// matched by a path from `?u` to `?v`", which is the traversal the `LIST[…]` notation
/// stands for:
///
/// 1. the LAST cell — `rdf:rest rdf:nil` — is matched by a single triple, which grounds the
///    recursion in a body that binds every head variable;
/// 2. any other cell is matched by one triple followed by the rest of the list;
/// 3. the axiom reads the accumulated relation off its own list head.
///
/// The conclusion of clause 3 puts `?p` in PREDICATE position, so an axiom whose subject is
/// not an IRI yields a generalized-RDF triple the RDF 1.2 IR cannot hold, exactly as
/// `prp-spo1` does; it is dropped at the materialization boundary and reported.
pub(super) fn property_chain() -> Vec<DlClause> {
    vec![
        DlClause::datalog(
            internal(CHAIN_RELATION, var("?cell"), var("?u1"), var("?u2")),
            vec![
                atom(var("?cell"), RDF_FIRST, var("?pi")),
                atom(var("?cell"), RDF_REST, iri(RDF_NIL)),
                quad(var("?u1"), var("?pi"), var("?u2")),
            ],
        ),
        DlClause::datalog(
            internal(CHAIN_RELATION, var("?cell"), var("?u1"), var("?u3")),
            vec![
                atom(var("?cell"), RDF_FIRST, var("?pi")),
                atom(var("?cell"), RDF_REST, var("?next")),
                quad(var("?u1"), var("?pi"), var("?u2")),
                internal(CHAIN_RELATION, var("?next"), var("?u2"), var("?u3")),
            ],
        ),
        DlClause::datalog(
            quad(var("?u1"), var("?p"), var("?un")),
            vec![
                atom(var("?p"), OWL_PROPERTYCHAINAXIOM, var("?x")),
                internal(CHAIN_RELATION, var("?x"), var("?u1"), var("?un")),
            ],
        ),
    ]
}

/// `prp-eqp1`: `?p1 owl:equivalentProperty ?p2`, `T(?x, ?p1, ?y)` ⇒ `T(?x, ?p2, ?y)`.
pub(super) fn equivalent_property_left() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?x"), var("?p2"), var("?y")),
        vec![
            atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2")),
            quad(var("?x"), var("?p1"), var("?y")),
        ],
    )]
}

/// `prp-eqp2`: `?p1 owl:equivalentProperty ?p2`, `T(?x, ?p2, ?y)` ⇒ `T(?x, ?p1, ?y)`.
pub(super) fn equivalent_property_right() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?x"), var("?p1"), var("?y")),
        vec![
            atom(var("?p1"), OWL_EQUIVALENTPROPERTY, var("?p2")),
            quad(var("?x"), var("?p2"), var("?y")),
        ],
    )]
}

/// `prp-pdw`: `?p1 owl:propertyDisjointWith ?p2`, `T(?x, ?p1, ?y)`, `T(?x, ?p2, ?y)` ⇒
/// `false`.
pub(super) fn property_disjoint() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?p1"), OWL_PROPERTYDISJOINTWITH, var("?p2")),
        quad(var("?x"), var("?p1"), var("?y")),
        quad(var("?x"), var("?p2"), var("?y")),
    ])]
}

/// `prp-adp`: `?x rdf:type owl:AllDisjointProperties`, `?x owl:members ?y` over the list
/// `?p1 … ?pn`, `T(?u, ?pi, ?v)`, `T(?u, ?pj, ?v)` with `i ≠ j` ⇒ `false`.
///
/// The two members are read from [`crate::lists`]'s `LIST(head, index, member)`, and the
/// `i ≠ j` side condition is `INDEX_DISTINCT(?i, ?j)` — the inequality relation the
/// pre-pass materializes. Stating the rule WITHOUT that condition would let
/// `i = j` match and make a single property assertion an inconsistency, which is unsound;
/// the condition is therefore expressed rather than dropped, even though the `false` head
/// is lowered rather than refused.
pub(super) fn all_disjoint_properties() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), RDF_TYPE, iri(OWL_ALLDISJOINTPROPERTIES)),
        atom(var("?x"), OWL_MEMBERS, var("?y")),
        internal(LIST_RELATION, var("?y"), var("?pi"), var("?i")),
        internal(LIST_RELATION, var("?y"), var("?pj"), var("?j")),
        internal(
            INDEX_DISTINCT_RELATION,
            var("?i"),
            var("?j"),
            internal_graph(),
        ),
        quad(var("?u"), var("?pi"), var("?v")),
        quad(var("?u"), var("?pj"), var("?v")),
    ])]
}

/// `prp-inv1`: `?p1 owl:inverseOf ?p2`, `T(?x, ?p1, ?y)` ⇒ `T(?y, ?p2, ?x)`.
pub(super) fn inverse1() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?y"), var("?p2"), var("?x")),
        vec![
            atom(var("?p1"), OWL_INVERSEOF, var("?p2")),
            quad(var("?x"), var("?p1"), var("?y")),
        ],
    )]
}

/// `prp-inv2`: `?p1 owl:inverseOf ?p2`, `T(?x, ?p2, ?y)` ⇒ `T(?y, ?p1, ?x)`.
pub(super) fn inverse2() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?y"), var("?p1"), var("?x")),
        vec![
            atom(var("?p1"), OWL_INVERSEOF, var("?p2")),
            quad(var("?x"), var("?p2"), var("?y")),
        ],
    )]
}

/// `prp-key`: `?c owl:hasKey ?u` over the list `?p1 … ?pn`, with `?x` and `?y` both
/// instances of `?c` agreeing on every key property ⇒ `?x owl:sameAs ?y`.
///
/// Three clauses, on the same shape as [`property_chain`]: `AGREE(?cell, ?x, ?y)` means
/// "`?x` and `?y` share a value for every property from `?cell` onwards", which is exactly
/// the UNIVERSAL quantification over the list the specification writes as `n` paired body
/// atoms. The base case is the last cell, so the recursion is grounded in a range-restricted
/// body; the third clause requires both individuals to be instances of `?c`, which is the
/// half of the rule that makes it a key rather than a value comparison.
pub(super) fn has_key() -> Vec<DlClause> {
    vec![
        DlClause::datalog(
            internal(AGREE_RELATION, var("?cell"), var("?x"), var("?y")),
            vec![
                atom(var("?cell"), RDF_FIRST, var("?pi")),
                atom(var("?cell"), RDF_REST, iri(RDF_NIL)),
                quad(var("?x"), var("?pi"), var("?zi")),
                quad(var("?y"), var("?pi"), var("?zi")),
            ],
        ),
        DlClause::datalog(
            internal(AGREE_RELATION, var("?cell"), var("?x"), var("?y")),
            vec![
                atom(var("?cell"), RDF_FIRST, var("?pi")),
                atom(var("?cell"), RDF_REST, var("?next")),
                quad(var("?x"), var("?pi"), var("?zi")),
                quad(var("?y"), var("?pi"), var("?zi")),
                internal(AGREE_RELATION, var("?next"), var("?x"), var("?y")),
            ],
        ),
        DlClause::datalog(
            atom(var("?x"), OWL_SAMEAS, var("?y")),
            vec![
                atom(var("?c"), OWL_HASKEY, var("?u")),
                atom(var("?x"), RDF_TYPE, var("?c")),
                atom(var("?y"), RDF_TYPE, var("?c")),
                internal(AGREE_RELATION, var("?u"), var("?x"), var("?y")),
            ],
        ),
    ]
}

/// `prp-npa1`: `?x owl:sourceIndividual ?i1`, `?x owl:assertionProperty ?p`,
/// `?x owl:targetIndividual ?i2`, `T(?i1, ?p, ?i2)` ⇒ `false`.
pub(super) fn negative_object_assertion() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), OWL_SOURCEINDIVIDUAL, var("?i1")),
        atom(var("?x"), OWL_ASSERTIONPROPERTY, var("?p")),
        atom(var("?x"), OWL_TARGETINDIVIDUAL, var("?i2")),
        quad(var("?i1"), var("?p"), var("?i2")),
    ])]
}

/// `prp-npa2`: `?x owl:sourceIndividual ?i`, `?x owl:assertionProperty ?p`,
/// `?x owl:targetValue ?lt`, `T(?i, ?p, ?lt)` ⇒ `false`.
pub(super) fn negative_data_assertion() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), OWL_SOURCEINDIVIDUAL, var("?i")),
        atom(var("?x"), OWL_ASSERTIONPROPERTY, var("?p")),
        atom(var("?x"), OWL_TARGETVALUE, var("?lt")),
        quad(var("?i"), var("?p"), var("?lt")),
    ])]
}

/// The `prp-*` rules this chase states, in OWL 2 Profiles Table 5 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means — including
/// what the optional `refuses:` field says about the six rules that conclude `false`.
macro_rules! prp_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `prp-ap` — every built-in annotation property is one. Premise-free.
            /// `OWL-RL` only.
            AnnotationProperties {
                id: PrpAp,
                lanes: [OwlRl],
                clauses: prp::annotation_properties,
            },
            /// `prp-fp` — a functional property's two values are the same thing.
            /// `OWL-RL` only.
            Functional {
                id: PrpFp,
                lanes: [OwlRl],
                clauses: prp::functional,
            },
            /// `prp-ifp` — an inverse-functional property's two subjects are the same
            /// thing. `OWL-RL` only.
            InverseFunctional {
                id: PrpIfp,
                lanes: [OwlRl],
                clauses: prp::inverse_functional,
            },
            /// `prp-irp` — an irreflexive property relating something to itself is an
            /// inconsistency. DECLARED, not evaluated: the head is `false`.
            Irreflexive {
                id: PrpIrp,
                lanes: [OwlRl],
                clauses: prp::irreflexive,
                concludes: Inconsistency,
            },
            /// `prp-symp` — a symmetric property mirrors its triples. `OWL-RL` only.
            Symmetric {
                id: PrpSymp,
                lanes: [OwlRl],
                clauses: prp::symmetric,
            },
            /// `prp-asyp` — an asymmetric property holding both ways is an inconsistency.
            /// DECLARED, not evaluated: the head is `false`.
            Asymmetric {
                id: PrpAsyp,
                lanes: [OwlRl],
                clauses: prp::asymmetric,
                concludes: Inconsistency,
            },
            /// `prp-trp` — a transitive property composes its triples. `OWL-RL` only.
            Transitive {
                id: PrpTrp,
                lanes: [OwlRl],
                clauses: prp::transitive,
            },
            /// `prp-spo2` — a property chain axiom composes a path into one triple.
            /// `OWL-RL` only.
            PropertyChain {
                id: PrpSpo2,
                lanes: [OwlRl],
                clauses: prp::property_chain,
            },
            /// `prp-eqp1` — an `owl:equivalentProperty` assertion, read left to right.
            /// `OWL-RL` only.
            EquivalentPropertyLeft {
                id: PrpEqp1,
                lanes: [OwlRl],
                clauses: prp::equivalent_property_left,
            },
            /// `prp-eqp2` — an `owl:equivalentProperty` assertion, read right to left.
            /// `OWL-RL` only.
            EquivalentPropertyRight {
                id: PrpEqp2,
                lanes: [OwlRl],
                clauses: prp::equivalent_property_right,
            },
            /// `prp-pdw` — two disjoint properties sharing a subject/object pair is an
            /// inconsistency. DECLARED, not evaluated: the head is `false`.
            PropertyDisjoint {
                id: PrpPdw,
                lanes: [OwlRl],
                clauses: prp::property_disjoint,
                concludes: Inconsistency,
            },
            /// `prp-adp` — the same, over an `owl:AllDisjointProperties` list. DECLARED,
            /// not evaluated: the head is `false`.
            AllDisjointProperties {
                id: PrpAdp,
                lanes: [OwlRl],
                clauses: prp::all_disjoint_properties,
                concludes: Inconsistency,
            },
            /// `prp-inv1` — an `owl:inverseOf` assertion, read left to right. `OWL-RL`
            /// only.
            Inverse1 {
                id: PrpInv1,
                lanes: [OwlRl],
                clauses: prp::inverse1,
            },
            /// `prp-inv2` — an `owl:inverseOf` assertion, read right to left. `OWL-RL`
            /// only.
            Inverse2 {
                id: PrpInv2,
                lanes: [OwlRl],
                clauses: prp::inverse2,
            },
            /// `prp-key` — two instances agreeing on every key property are the same
            /// thing. `OWL-RL` only.
            HasKey {
                id: PrpKey,
                lanes: [OwlRl],
                clauses: prp::has_key,
            },
            /// `prp-npa1` — a negative OBJECT-property assertion whose triple is asserted
            /// is an inconsistency. DECLARED, not evaluated: the head is `false`.
            NegativeObjectAssertion {
                id: PrpNpa1,
                lanes: [OwlRl],
                clauses: prp::negative_object_assertion,
                concludes: Inconsistency,
            },
            /// `prp-npa2` — the same for a negative DATA-property assertion. DECLARED,
            /// not evaluated: the head is `false`.
            NegativeDataAssertion {
                id: PrpNpa2,
                lanes: [OwlRl],
                clauses: prp::negative_data_assertion,
                concludes: Inconsistency,
            },
        }
    };
}

pub(crate) use prp_rules;
