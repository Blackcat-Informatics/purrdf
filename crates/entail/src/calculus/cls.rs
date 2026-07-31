// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `cls-*` — the semantics of classes, OWL 2 Profiles §4.3 Table 6.
//!
//! Nineteen rules, all of them stated here. They divide into four shapes, and the shape is
//! what decides how each is written as DL clauses.
//!
//! # Two premise-free rules
//!
//! `cls-thing` and `cls-nothing1` type `owl:Thing` and `owl:Nothing` an `owl:Class`. They
//! quantify over the OWL 2 RL vocabulary rather than over the graph, so each is a clause
//! with no body — the same shape `prp-ap` and `rdfs1` take.
//!
//! # Five rules that conclude `false`
//!
//! `cls-nothing2`, `cls-com`, `cls-maxc1`, `cls-maxqc1` and `cls-maxqc2` conclude `false`:
//! a body match is an INCONSISTENCY WITNESS, not a triple. Each is declared with
//! `concludes: Inconsistency,` and the specification's own body, and
//! [`super::constraint_clause`] lowers it to the clause the evaluator runs. See the
//! [calculus docs](super) for why that lowering fabricates nothing.
//!
//! # Four rules that read an RDF COLLECTION
//!
//! `cls-int1`, `cls-int2`, `cls-uni` and `cls-oo` are written with the meta-notation
//! `LIST[?x, ?c1, …, ?cn]`, which no clause language has. Three of them need MEMBERSHIP
//! only — a conclusion about each member independently (`cls-int2`, `cls-oo`) or from any
//! one member (`cls-uni`) — and read [`crate::lists`]'s `LIST(head, index, member)`
//! directly.
//!
//! `cls-int1` is the fourth and is different in kind: its premise is the CONJUNCTION
//! `T(?y, rdf:type, ?c1) ∧ … ∧ T(?y, rdf:type, ?cn)`, a body whose length depends on the
//! data. It is stated the way `prp-key` states its own universal quantification — an
//! internal relation accumulating the walk:
//!
//! ```text
//! ALL_TYPES(cell, y)   "y is an instance of every class from `cell` onwards"
//! ```
//!
//! grounded at the LAST cell (`rdf:rest rdf:nil`) so the base case binds every head
//! variable and the clause is range-restricted. A consequence worth stating: the EMPTY
//! intersection concludes nothing, which is correct — OWL 2 requires an intersection to
//! have at least two members, so the encoding is complete for every ontology the
//! specification admits.
//!
//! # The cardinality literals are the specification's own
//!
//! `cls-maxc1`, `cls-maxc2` and the four `cls-maxqc*` rules match the OBJECT of an
//! `owl:maxCardinality` / `owl:maxQualifiedCardinality` triple against the literal
//! `"0"^^xsd:nonNegativeInteger` or `"1"^^xsd:nonNegativeInteger`, exactly as Table 6
//! writes them — these are the only literal constants in the whole declared calculus, and
//! they are transcribed rather than widened. An ontology that writes `"0"^^xsd:integer`
//! instead reaches the same rules through `dt-eq` and `eq-rep-o`, which is precisely why
//! OWL 2 RL carries Table 8; see [`super::dt`].
//!
//! Their surfaces come from [`crate::engine::literal_surface`], the same function that
//! records the value the surface is read back as, so a clause constant and a dataset
//! literal compare as the same bytes without a second rendering convention.

use purrdf_datalog::clause::{ClauseTerm, DlClause};

use super::{atom, internal, internal_graph, iri, quad, var};
use crate::engine::literal_surface;
use crate::lists::{ALL_TYPES_RELATION, LIST_RELATION};
use crate::vocab::{
    OWL_ALLVALUESFROM, OWL_CLASS, OWL_COMPLEMENTOF, OWL_HASVALUE, OWL_INTERSECTIONOF,
    OWL_MAXCARDINALITY, OWL_MAXQUALIFIEDCARDINALITY, OWL_NOTHING, OWL_ONCLASS, OWL_ONEOF,
    OWL_ONPROPERTY, OWL_SAMEAS, OWL_SOMEVALUESFROM, OWL_THING, OWL_UNIONOF, RDF_FIRST, RDF_NIL,
    RDF_REST, RDF_TYPE, XSD_NONNEGATIVEINTEGER,
};

/// The cardinality literal `"n"^^xsd:nonNegativeInteger`, as Table 6 writes it.
fn cardinality(n: &str) -> ClauseTerm {
    ClauseTerm::literal(literal_surface(n, XSD_NONNEGATIVEINTEGER))
}

/// `cls-thing`: `owl:Thing rdf:type owl:Class`. Premise-free.
pub(super) fn thing() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(iri(OWL_THING), RDF_TYPE, iri(OWL_CLASS)),
        Vec::new(),
    )]
}

/// `cls-nothing1`: `owl:Nothing rdf:type owl:Class`. Premise-free.
pub(super) fn nothing_is_a_class() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(iri(OWL_NOTHING), RDF_TYPE, iri(OWL_CLASS)),
        Vec::new(),
    )]
}

/// `cls-nothing2`: `?x rdf:type owl:Nothing` ⇒ `false`.
pub(super) fn nothing_is_empty() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![atom(
        var("?x"),
        RDF_TYPE,
        iri(OWL_NOTHING),
    )])]
}

/// `cls-int1`: `?c owl:intersectionOf ?x` over the list `?c1 … ?cn`, with `?y` an instance
/// of EVERY `?ci` ⇒ `?y rdf:type ?c`.
///
/// Three clauses on the shape [`super::prp::has_key`] uses: `ALL_TYPES(?cell, ?y)` means
/// "`?y` is an instance of every class from `?cell` onwards", the base case is the last
/// cell so the recursion is grounded in a range-restricted body, and the third clause reads
/// the accumulated relation off the axiom's own list head.
pub(super) fn intersection_membership() -> Vec<DlClause> {
    vec![
        DlClause::datalog(
            internal(
                ALL_TYPES_RELATION,
                var("?cell"),
                var("?y"),
                internal_graph(),
            ),
            vec![
                atom(var("?cell"), RDF_FIRST, var("?ci")),
                atom(var("?cell"), RDF_REST, iri(RDF_NIL)),
                atom(var("?y"), RDF_TYPE, var("?ci")),
            ],
        ),
        DlClause::datalog(
            internal(
                ALL_TYPES_RELATION,
                var("?cell"),
                var("?y"),
                internal_graph(),
            ),
            vec![
                atom(var("?cell"), RDF_FIRST, var("?ci")),
                atom(var("?cell"), RDF_REST, var("?next")),
                atom(var("?y"), RDF_TYPE, var("?ci")),
                internal(
                    ALL_TYPES_RELATION,
                    var("?next"),
                    var("?y"),
                    internal_graph(),
                ),
            ],
        ),
        DlClause::datalog(
            atom(var("?y"), RDF_TYPE, var("?c")),
            vec![
                atom(var("?c"), OWL_INTERSECTIONOF, var("?x")),
                internal(ALL_TYPES_RELATION, var("?x"), var("?y"), internal_graph()),
            ],
        ),
    ]
}

/// `cls-int2`: `?c owl:intersectionOf ?x` over the list `?c1 … ?cn`, with `?y rdf:type ?c`
/// ⇒ `?y rdf:type ?ci` for EVERY `?ci`.
///
/// The conclusion is per-member, so this is membership rather than traversal: one clause
/// over [`crate::lists`]'s `LIST(head, index, member)`.
pub(super) fn intersection_instance() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y"), RDF_TYPE, var("?ci")),
        vec![
            atom(var("?c"), OWL_INTERSECTIONOF, var("?x")),
            internal(LIST_RELATION, var("?x"), var("?ci"), var("?i")),
            atom(var("?y"), RDF_TYPE, var("?c")),
        ],
    )]
}

/// `cls-uni`: `?c owl:unionOf ?x` over the list `?c1 … ?cn`, with `?y rdf:type ?ci` for
/// SOME `?ci` ⇒ `?y rdf:type ?c`.
pub(super) fn union_instance() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y"), RDF_TYPE, var("?c")),
        vec![
            atom(var("?c"), OWL_UNIONOF, var("?x")),
            internal(LIST_RELATION, var("?x"), var("?ci"), var("?i")),
            atom(var("?y"), RDF_TYPE, var("?ci")),
        ],
    )]
}

/// `cls-com`: `?c1 owl:complementOf ?c2`, `?x rdf:type ?c1`, `?x rdf:type ?c2` ⇒ `false`.
pub(super) fn complement() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?c1"), OWL_COMPLEMENTOF, var("?c2")),
        atom(var("?x"), RDF_TYPE, var("?c1")),
        atom(var("?x"), RDF_TYPE, var("?c2")),
    ])]
}

/// `cls-svf1`: `?x owl:someValuesFrom ?y`, `?x owl:onProperty ?p`, `T(?u, ?p, ?v)`,
/// `?v rdf:type ?y` ⇒ `?u rdf:type ?x`.
pub(super) fn some_values_from() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?u"), RDF_TYPE, var("?x")),
        vec![
            atom(var("?x"), OWL_SOMEVALUESFROM, var("?y")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            quad(var("?u"), var("?p"), var("?v")),
            atom(var("?v"), RDF_TYPE, var("?y")),
        ],
    )]
}

/// `cls-svf2`: `?x owl:someValuesFrom owl:Thing`, `?x owl:onProperty ?p`, `T(?u, ?p, ?v)`
/// ⇒ `?u rdf:type ?x`.
///
/// Not a special case of [`some_values_from`] that could be dropped: nothing types every
/// individual `owl:Thing` in this calculus (`cls-thing` types the CLASS, not its
/// instances), so the filler-typed premise the general rule needs is absent and the
/// specification states the `owl:Thing` case separately.
pub(super) fn some_values_from_thing() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?u"), RDF_TYPE, var("?x")),
        vec![
            atom(var("?x"), OWL_SOMEVALUESFROM, iri(OWL_THING)),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            quad(var("?u"), var("?p"), var("?v")),
        ],
    )]
}

/// `cls-avf`: `?x owl:allValuesFrom ?y`, `?x owl:onProperty ?p`, `?u rdf:type ?x`,
/// `T(?u, ?p, ?v)` ⇒ `?v rdf:type ?y`.
pub(super) fn all_values_from() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?v"), RDF_TYPE, var("?y")),
        vec![
            atom(var("?x"), OWL_ALLVALUESFROM, var("?y")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            atom(var("?u"), RDF_TYPE, var("?x")),
            quad(var("?u"), var("?p"), var("?v")),
        ],
    )]
}

/// `cls-hv1`: `?x owl:hasValue ?y`, `?x owl:onProperty ?p`, `?u rdf:type ?x` ⇒
/// `T(?u, ?p, ?y)`.
///
/// The conclusion puts `?p` in PREDICATE position, so a restriction whose `owl:onProperty`
/// object is not an IRI yields a generalized-RDF triple the RDF 1.2 IR cannot hold; it is
/// dropped at the materialization boundary and reported, exactly as `prp-spo1` is.
pub(super) fn has_value_assert() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?u"), var("?p"), var("?y")),
        vec![
            atom(var("?x"), OWL_HASVALUE, var("?y")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            atom(var("?u"), RDF_TYPE, var("?x")),
        ],
    )]
}

/// `cls-hv2`: `?x owl:hasValue ?y`, `?x owl:onProperty ?p`, `T(?u, ?p, ?y)` ⇒
/// `?u rdf:type ?x`.
pub(super) fn has_value_recognize() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?u"), RDF_TYPE, var("?x")),
        vec![
            atom(var("?x"), OWL_HASVALUE, var("?y")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            quad(var("?u"), var("?p"), var("?y")),
        ],
    )]
}

/// `cls-maxc1`: an `owl:maxCardinality 0` restriction with a matching property assertion on
/// one of its instances ⇒ `false`.
pub(super) fn max_cardinality_zero() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), OWL_MAXCARDINALITY, cardinality("0")),
        atom(var("?x"), OWL_ONPROPERTY, var("?p")),
        atom(var("?u"), RDF_TYPE, var("?x")),
        quad(var("?u"), var("?p"), var("?y")),
    ])]
}

/// `cls-maxc2`: an `owl:maxCardinality 1` restriction with two property values on one of
/// its instances ⇒ `?y1 owl:sameAs ?y2`.
pub(super) fn max_cardinality_one() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y1"), OWL_SAMEAS, var("?y2")),
        vec![
            atom(var("?x"), OWL_MAXCARDINALITY, cardinality("1")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            atom(var("?u"), RDF_TYPE, var("?x")),
            quad(var("?u"), var("?p"), var("?y1")),
            quad(var("?u"), var("?p"), var("?y2")),
        ],
    )]
}

/// `cls-maxqc1`: an `owl:maxQualifiedCardinality 0` restriction on `?c` with a matching
/// value typed `?c` ⇒ `false`.
pub(super) fn max_qualified_zero() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), OWL_MAXQUALIFIEDCARDINALITY, cardinality("0")),
        atom(var("?x"), OWL_ONPROPERTY, var("?p")),
        atom(var("?x"), OWL_ONCLASS, var("?c")),
        atom(var("?u"), RDF_TYPE, var("?x")),
        quad(var("?u"), var("?p"), var("?y")),
        atom(var("?y"), RDF_TYPE, var("?c")),
    ])]
}

/// `cls-maxqc2`: an `owl:maxQualifiedCardinality 0` restriction on `owl:Thing` with any
/// matching value ⇒ `false`.
pub(super) fn max_qualified_zero_thing() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), OWL_MAXQUALIFIEDCARDINALITY, cardinality("0")),
        atom(var("?x"), OWL_ONPROPERTY, var("?p")),
        atom(var("?x"), OWL_ONCLASS, iri(OWL_THING)),
        atom(var("?u"), RDF_TYPE, var("?x")),
        quad(var("?u"), var("?p"), var("?y")),
    ])]
}

/// `cls-maxqc3`: an `owl:maxQualifiedCardinality 1` restriction on `?c` with two values
/// typed `?c` ⇒ `?y1 owl:sameAs ?y2`.
pub(super) fn max_qualified_one() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y1"), OWL_SAMEAS, var("?y2")),
        vec![
            atom(var("?x"), OWL_MAXQUALIFIEDCARDINALITY, cardinality("1")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            atom(var("?x"), OWL_ONCLASS, var("?c")),
            atom(var("?u"), RDF_TYPE, var("?x")),
            quad(var("?u"), var("?p"), var("?y1")),
            atom(var("?y1"), RDF_TYPE, var("?c")),
            quad(var("?u"), var("?p"), var("?y2")),
            atom(var("?y2"), RDF_TYPE, var("?c")),
        ],
    )]
}

/// `cls-maxqc4`: an `owl:maxQualifiedCardinality 1` restriction on `owl:Thing` with two
/// values ⇒ `?y1 owl:sameAs ?y2`.
pub(super) fn max_qualified_one_thing() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y1"), OWL_SAMEAS, var("?y2")),
        vec![
            atom(var("?x"), OWL_MAXQUALIFIEDCARDINALITY, cardinality("1")),
            atom(var("?x"), OWL_ONPROPERTY, var("?p")),
            atom(var("?x"), OWL_ONCLASS, iri(OWL_THING)),
            atom(var("?u"), RDF_TYPE, var("?x")),
            quad(var("?u"), var("?p"), var("?y1")),
            quad(var("?u"), var("?p"), var("?y2")),
        ],
    )]
}

/// `cls-oo`: `?c owl:oneOf ?x` over the list `?y1 … ?yn` ⇒ `?yi rdf:type ?c` for every
/// `?yi`.
pub(super) fn one_of() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?yi"), RDF_TYPE, var("?c")),
        vec![
            atom(var("?c"), OWL_ONEOF, var("?x")),
            internal(LIST_RELATION, var("?x"), var("?yi"), var("?i")),
        ],
    )]
}

/// The `cls-*` rules this chase states, in OWL 2 Profiles Table 6 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means — including
/// what the optional `concludes:` field says about the five rules that conclude `false`.
macro_rules! cls_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `cls-thing` — `owl:Thing` is a class. Premise-free. `OWL-RL` only.
            Thing {
                id: ClsThing,
                lanes: [OwlRl],
                clauses: cls::thing,
            },
            /// `cls-nothing1` — `owl:Nothing` is a class. Premise-free. `OWL-RL` only.
            NothingIsAClass {
                id: ClsNothing1,
                lanes: [OwlRl],
                clauses: cls::nothing_is_a_class,
            },
            /// `cls-nothing2` — an instance of `owl:Nothing` is an inconsistency.
            NothingIsEmpty {
                id: ClsNothing2,
                lanes: [OwlRl],
                clauses: cls::nothing_is_empty,
                concludes: Inconsistency,
            },
            /// `cls-int1` — an instance of every member of an intersection list is an
            /// instance of the intersection. `OWL-RL` only.
            IntersectionMembership {
                id: ClsInt1,
                lanes: [OwlRl],
                clauses: cls::intersection_membership,
            },
            /// `cls-int2` — an instance of an intersection is an instance of every member.
            /// `OWL-RL` only.
            IntersectionInstance {
                id: ClsInt2,
                lanes: [OwlRl],
                clauses: cls::intersection_instance,
            },
            /// `cls-uni` — an instance of any member of a union list is an instance of the
            /// union. `OWL-RL` only.
            UnionInstance {
                id: ClsUni,
                lanes: [OwlRl],
                clauses: cls::union_instance,
            },
            /// `cls-com` — something in a class and in its complement is an inconsistency.
            Complement {
                id: ClsCom,
                lanes: [OwlRl],
                clauses: cls::complement,
                concludes: Inconsistency,
            },
            /// `cls-svf1` — an existential restriction recognizes its instances.
            /// `OWL-RL` only.
            SomeValuesFrom {
                id: ClsSvf1,
                lanes: [OwlRl],
                clauses: cls::some_values_from,
            },
            /// `cls-svf2` — the same, with `owl:Thing` as the filler. `OWL-RL` only.
            SomeValuesFromThing {
                id: ClsSvf2,
                lanes: [OwlRl],
                clauses: cls::some_values_from_thing,
            },
            /// `cls-avf` — a universal restriction types the values of its instances.
            /// `OWL-RL` only.
            AllValuesFrom {
                id: ClsAvf,
                lanes: [OwlRl],
                clauses: cls::all_values_from,
            },
            /// `cls-hv1` — a `owl:hasValue` restriction asserts the value on its
            /// instances. `OWL-RL` only.
            HasValueAssert {
                id: ClsHv1,
                lanes: [OwlRl],
                clauses: cls::has_value_assert,
            },
            /// `cls-hv2` — and recognizes as instances whatever carries the value.
            /// `OWL-RL` only.
            HasValueRecognize {
                id: ClsHv2,
                lanes: [OwlRl],
                clauses: cls::has_value_recognize,
            },
            /// `cls-maxc1` — a value on a `max 0` restriction's instance is an
            /// inconsistency.
            MaxCardinalityZero {
                id: ClsMaxc1,
                lanes: [OwlRl],
                clauses: cls::max_cardinality_zero,
                concludes: Inconsistency,
            },
            /// `cls-maxc2` — two values on a `max 1` restriction's instance are one thing.
            /// `OWL-RL` only.
            MaxCardinalityOne {
                id: ClsMaxc2,
                lanes: [OwlRl],
                clauses: cls::max_cardinality_one,
            },
            /// `cls-maxqc1` — a qualified value on a `max 0` restriction's instance is an
            /// inconsistency.
            MaxQualifiedZero {
                id: ClsMaxqc1,
                lanes: [OwlRl],
                clauses: cls::max_qualified_zero,
                concludes: Inconsistency,
            },
            /// `cls-maxqc2` — the same, qualified on `owl:Thing`.
            MaxQualifiedZeroThing {
                id: ClsMaxqc2,
                lanes: [OwlRl],
                clauses: cls::max_qualified_zero_thing,
                concludes: Inconsistency,
            },
            /// `cls-maxqc3` — two qualified values on a `max 1` restriction's instance are
            /// one thing. `OWL-RL` only.
            MaxQualifiedOne {
                id: ClsMaxqc3,
                lanes: [OwlRl],
                clauses: cls::max_qualified_one,
            },
            /// `cls-maxqc4` — the same, qualified on `owl:Thing`. `OWL-RL` only.
            MaxQualifiedOneThing {
                id: ClsMaxqc4,
                lanes: [OwlRl],
                clauses: cls::max_qualified_one_thing,
            },
            /// `cls-oo` — every member of an enumeration is an instance of it.
            /// `OWL-RL` only.
            OneOf {
                id: ClsOo,
                lanes: [OwlRl],
                clauses: cls::one_of,
            },
        }
    };
}

pub(crate) use cls_rules;
