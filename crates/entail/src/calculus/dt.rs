// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `dt-*` — the semantics of datatypes, OWL 2 Profiles §4.3 Table 8.
//!
//! Five rules, and four of them are written with NO triple premise at all: they quantify
//! over "each literal `lt`" and "each datatype `dt` supported in OWL 2 RL", which is an
//! infinite set of literals over a finite set of datatypes. A forward chase cannot
//! materialize an infinite premise, so this family quantifies over the literals the DATASET
//! HOLDS — which is the [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace)
//! boundary, reported on every run of every lane that fires these rules.
//!
//! # The value space is walked by a PRE-PASS, not by a clause
//!
//! "the data value of `lt` is in the value space of `dt`" is a computation over XSD
//! values, not a join over triples. [`crate::datatypes`] performs it once per run with
//! `purrdf-xsd` and materializes four internal relations the clauses here read:
//!
//! ```text
//! DT_VALUE(literal, datatype)      the literal's value lies in the datatype's value space
//! DT_EQUAL(lt1, lt2)               two literals denote ONE value (reflexive)
//! DT_ILL_TYPED(literal, datatype)  the literal's own datatype does not accept its lexical
//! ```
//!
//! `purrdf-xsd` is a zero-dependency kernel crate and gains nothing from being used here;
//! the pre-pass calls `parse_by_iri` and `value_eq` and nothing else.
//!
//! # Every conclusion of `dt-type2`, `dt-eq` and `dt-diff` has a LITERAL SUBJECT
//!
//! `T(lt, rdf:type, dt)`, `T(lt1, owl:sameAs, lt2)` and `T(lt1, owl:differentFrom, lt2)`
//! all put a literal in subject position. That is legal in the generalized RDF OWL 2 RL/RDF
//! is defined over and is NOT legal in RDF 1.2, so every one of those conclusions is drawn
//! in the evaluator's own term space and then abandoned at the materialization boundary —
//! the [`Construct::GeneralizedRdf`](crate::Construct::GeneralizedRdf) boundary, reported
//! with a count.
//!
//! They are not therefore inert. A conclusion that cannot be MATERIALIZED can still be a
//! PREMISE, and these three are exactly the premises OWL 2 RL needs them to be:
//! `dt-eq`'s `owl:sameAs` drives `eq-rep-o`, so `:x :p "1"^^xsd:integer` entails
//! `:x :p "01"^^xsd:integer` — a triple the IR holds perfectly well — and it is what lets
//! an ontology that writes `owl:maxCardinality "0"^^xsd:integer` reach the `cls-maxc1`
//! rule Table 6 writes with `"0"^^xsd:nonNegativeInteger`.
//!
//! # `dt-diff` is stated DEMAND-RESTRICTED, and that is exact
//!
//! Written as the specification writes it, `dt-diff` CONCLUDES `owl:differentFrom` for
//! every ordered pair of value-different literals in the graph. Every one of those
//! conclusions has a literal subject, so not one of them is materialized, and
//! `owl:differentFrom` has exactly ONE consumer in the whole of OWL 2 RL — `eq-diff1`,
//! whose other premise is `T(?x, owl:sameAs, ?y)`. So the clause here carries that premise
//! as a guard: the pair is concluded different precisely when something can read it. The
//! materialized closure is identical (both forms emit nothing), every downstream
//! conclusion is identical (the guard is `eq-diff1`'s own other premise), and the DERIVED
//! fact count falls from quadratic in the literals to linear in the equalities the chase
//! found.
//!
//! What the guard does NOT change is the size of the pre-pass's own `DT_DIFFERENT`
//! relation, which is quadratic in the number of literals the dataset holds whose datatype
//! `purrdf-xsd` models. That is inherent to the rule — an inequality over `n` values IS
//! `n²` pairs, and it cannot be a negation here (see [`crate::lists`]) — and it is bounded
//! by [`MAX_STORED_FACTS`](purrdf_datalog::seminaive::MAX_STORED_FACTS) like every other
//! fact: a dataset with more than a few hundred distinct valued literals passes that
//! ceiling and the run is REFUSED with an accurate report, never truncated. That is the
//! one place in this calculus where an ordinary input can meet a ceiling, and it is
//! measured rather than assumed.

use purrdf_datalog::clause::DlClause;

use super::{atom, internal, internal_graph, iri, quad, var};
use crate::lists::{
    DT_DIFFERENT_RELATION, DT_EQUAL_RELATION, DT_ILL_TYPED_RELATION, DT_VALUE_RELATION,
};
use crate::vocab::{
    OWL_DIFFERENTFROM, OWL_SAMEAS, RDF_PLAINLITERAL, RDF_TYPE, RDF_XMLLITERAL, RDFS_DATATYPE,
    RDFS_LITERAL, XSD_ANYURI, XSD_BASE64BINARY, XSD_BOOLEAN, XSD_BYTE, XSD_DATETIME,
    XSD_DATETIMESTAMP, XSD_DECIMAL, XSD_DOUBLE, XSD_FLOAT, XSD_HEXBINARY, XSD_INT, XSD_INTEGER,
    XSD_LANGUAGE, XSD_LONG, XSD_NAME, XSD_NCNAME, XSD_NEGATIVEINTEGER, XSD_NMTOKEN,
    XSD_NONNEGATIVEINTEGER, XSD_NONPOSITIVEINTEGER, XSD_NORMALIZEDSTRING, XSD_POSITIVEINTEGER,
    XSD_SHORT, XSD_STRING, XSD_TOKEN, XSD_UNSIGNEDBYTE, XSD_UNSIGNEDINT, XSD_UNSIGNEDLONG,
    XSD_UNSIGNEDSHORT,
};

/// The datatypes SUPPORTED IN OWL 2 RL, transcribed from OWL 2 Profiles §4.2.1.
///
/// Thirty-two IRIs, and the list is the specification's own rather than a set this crate
/// chose: PurRDF mints no vocabulary, and `dt-type1` quantifies over exactly these. Note
/// what is ABSENT — `owl:real` and `owl:rational` are in the OWL 2 datatype map and are NOT
/// supported in OWL 2 RL, so neither is typed here.
///
/// [`crate::datatypes`] decides value-space membership over the same list, so a datatype
/// this crate types `rdfs:Datatype` and a datatype it tests literals against cannot drift
/// apart.
pub(crate) const SUPPORTED_DATATYPES: [&str; 32] = [
    RDF_PLAINLITERAL,
    RDF_XMLLITERAL,
    RDFS_LITERAL,
    XSD_DECIMAL,
    XSD_INTEGER,
    XSD_NONNEGATIVEINTEGER,
    XSD_NONPOSITIVEINTEGER,
    XSD_POSITIVEINTEGER,
    XSD_NEGATIVEINTEGER,
    XSD_LONG,
    XSD_INT,
    XSD_SHORT,
    XSD_BYTE,
    XSD_UNSIGNEDLONG,
    XSD_UNSIGNEDINT,
    XSD_UNSIGNEDSHORT,
    XSD_UNSIGNEDBYTE,
    XSD_FLOAT,
    XSD_DOUBLE,
    XSD_STRING,
    XSD_NORMALIZEDSTRING,
    XSD_TOKEN,
    XSD_LANGUAGE,
    XSD_NAME,
    XSD_NCNAME,
    XSD_NMTOKEN,
    XSD_BOOLEAN,
    XSD_HEXBINARY,
    XSD_BASE64BINARY,
    XSD_ANYURI,
    XSD_DATETIME,
    XSD_DATETIMESTAMP,
];

/// `dt-type1`: `dt rdf:type rdfs:Datatype` for each datatype supported in OWL 2 RL.
///
/// One premise-free clause per datatype, in [`SUPPORTED_DATATYPES`] order — the same shape
/// `prp-ap`, `cls-thing` and `rdfs1` take, and for the same reason: the rule quantifies
/// over the specification's vocabulary rather than over the graph, so it holds for every
/// input including the empty one.
pub(super) fn datatypes_are_datatypes() -> Vec<DlClause> {
    SUPPORTED_DATATYPES
        .into_iter()
        .map(|datatype| {
            DlClause::datalog(
                atom(iri(datatype), RDF_TYPE, iri(RDFS_DATATYPE)),
                Vec::new(),
            )
        })
        .collect()
}

/// `dt-type2`: `lt rdf:type dt` for each literal whose data value lies in the value space
/// of a supported datatype `dt`.
///
/// The subject is a LITERAL, so the conclusion is generalized RDF and is dropped at the
/// materialization boundary; see the [module docs](self).
pub(super) fn literals_are_typed() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?lt"), RDF_TYPE, var("?dt")),
        vec![internal(
            DT_VALUE_RELATION,
            var("?lt"),
            var("?dt"),
            internal_graph(),
        )],
    )]
}

/// `dt-eq`: `lt1 owl:sameAs lt2` for all literals with the same data value.
pub(super) fn equal_values() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?lt1"), OWL_SAMEAS, var("?lt2")),
        vec![internal(
            DT_EQUAL_RELATION,
            var("?lt1"),
            var("?lt2"),
            internal_graph(),
        )],
    )]
}

/// `dt-diff`: `lt1 owl:differentFrom lt2` for all literals with different data values,
/// stated demand-restricted to the pairs `eq-diff1` can read.
///
/// See the [module docs](self) for why the `owl:sameAs` guard costs no conclusion and why
/// the two reflexive `DT_EQUAL` atoms are load-bearing rather than decorative.
pub(super) fn different_values() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?lt1"), OWL_DIFFERENTFROM, var("?lt2")),
        vec![
            atom(var("?lt1"), OWL_SAMEAS, var("?lt2")),
            internal(
                DT_DIFFERENT_RELATION,
                var("?lt1"),
                var("?lt2"),
                internal_graph(),
            ),
        ],
    )]
}

/// `dt-not-type`: a literal whose data value is NOT in the value space of the datatype it
/// carries ⇒ `false`.
///
/// The specification writes this rule with an empty premise column and the side condition
/// "for each literal `lt` and each datatype `dt` supported in OWL 2 RL such that the data
/// value of `lt` is not in the value space of `dt`". Read without qualification that would
/// make every graph holding `"cat"^^xsd:string` inconsistent, because a string is not in
/// `xsd:integer`'s value space — an unsound reading, and not the one the rule has: the
/// literal's OWN datatype is the `dt` in question, so the rule is the ill-typed-literal
/// clash. [`crate::datatypes`] decides that, and `DT_ILL_TYPED(lt, dt)` is its answer.
///
/// The occurrence atom `T(?s, ?p, ?lt)` is the "each literal" quantifier a forward chase
/// can actually range over — a literal the dataset does not hold is not a literal this run
/// has — and it is what lets the inconsistency witness name the triple that carries the
/// bad literal rather than merely the literal.
pub(super) fn ill_typed_literal() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        quad(var("?s"), var("?p"), var("?lt")),
        internal(
            DT_ILL_TYPED_RELATION,
            var("?lt"),
            var("?dt"),
            internal_graph(),
        ),
    ])]
}

/// The `dt-*` rules this chase states, in OWL 2 Profiles Table 8 order.
///
/// Every one of them names TWO lanes. `OWL-RL` fires them as Table 8 of its own rule set;
/// `D` fires them as the whole of its calculus, because `entailment/D` IS datatype
/// entailment and this crate realizes it as Simple entailment plus these five rules. See
/// [`crate::rules::rules`] for what that makes `rules(Regime::D)`.
macro_rules! dt_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `dt-type1` — every datatype supported in OWL 2 RL is an `rdfs:Datatype`.
            /// Premise-free.
            DatatypesAreDatatypes {
                id: DtType1,
                lanes: [OwlRl, D],
                clauses: dt::datatypes_are_datatypes,
            },
            /// `dt-type2` — a literal is typed by every datatype whose value space holds
            /// its value.
            LiteralsAreTyped {
                id: DtType2,
                lanes: [OwlRl, D],
                clauses: dt::literals_are_typed,
            },
            /// `dt-eq` — two literals with one data value are the same thing.
            EqualValues {
                id: DtEq,
                lanes: [OwlRl, D],
                clauses: dt::equal_values,
            },
            /// `dt-diff` — two literals with different data values are different things.
            DifferentValues {
                id: DtDiff,
                lanes: [OwlRl, D],
                clauses: dt::different_values,
            },
            /// `dt-not-type` — an ill-typed literal is an inconsistency.
            IllTypedLiteral {
                id: DtNotType,
                lanes: [OwlRl, D],
                clauses: dt::ill_typed_literal,
                concludes: Inconsistency,
            },
        }
    };
}

pub(crate) use dt_rules;
