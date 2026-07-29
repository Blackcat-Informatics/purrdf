// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `eq-*` — the semantics of equality, OWL 2 Profiles §4.3 Table 4.
//!
//! Nine rules: the three that make `owl:sameAs` an equivalence relation (`eq-ref`,
//! `eq-sym`, `eq-trans`), the three that make it a CONGRUENCE over every triple position
//! (`eq-rep-s`, `eq-rep-p`, `eq-rep-o`), and the three that clash it against
//! `owl:differentFrom` (`eq-diff1`, `eq-diff2`, `eq-diff3`).
//!
//! # Why the congruence is MATERIALIZED rather than rewritten
//!
//! The published near-linear treatment of `owl:sameAs` — Motik, Nenov, Piro and Horrocks,
//! *Handling owl:sameAs via Rewriting* (AAAI 2015), the RDFox approach — keeps the chase
//! small by reasoning over one canonical representative per equivalence class and
//! expanding the congruence only when an answer is produced. Its saving is real and it is
//! a saving on the ANSWER: a system that answers queries without materializing the
//! congruence never pays for the `|class|³` variants of a triple.
//!
//! [`crate::materialize`] is not that system. It returns the closure itself, so the
//! `|class|³` variants are the answer, and rewriting would move the same facts from the
//! chase's store into the emission step without removing one of them. What it WOULD remove
//! is the evaluator's exact attribution — `purrdf-datalog` credits each conclusion to the
//! rule that reached it in fewest steps, and a hand-written expansion would have to guess
//! which of `eq-rep-s`, `eq-rep-p` and `eq-rep-o` a given variant came from. So the rules
//! are stated as the specification states them and the evaluator runs them, and the four
//! hazards the rewriting literature names are answered here rather than designed around:
//!
//! * **canonicity** — there is no representative to choose, so there is no choice to get
//!   wrong. `purrdf-datalog` commits a round's winner by a total order over observable
//!   provenance and emits derivations in lexical order, so the closure and the report are
//!   byte-stable across runs whatever the input's term order.
//! * **the multiplier** — a class of size `k` multiplies every triple it touches by up to
//!   `k³`, and the growth is bounded by [`MAX_STORED_FACTS`](purrdf_datalog::seminaive::MAX_STORED_FACTS),
//!   not by the step budget. Passing it is [`EntailError::Evaluate`](crate::EntailError):
//!   a REFUSAL carrying the observation, never a truncated closure.
//! * **the predicate position** — [`equal_predicate`] rewrites a triple's PREDICATE, which
//!   is expressible only because a [`ClauseAtom`](purrdf_datalog::clause::ClauseAtom)
//!   carries the predicate as DATA. A rewrite that puts a non-IRI there is a
//!   generalized-RDF triple the RDF 1.2 IR cannot hold, so it is dropped at the
//!   materialization boundary and reported.
//! * **triple terms** — `owl:sameAs` does NOT substitute inside an RDF 1.2 triple term.
//!   The chase interns a triple term as one atomic term and never looks inside it, so
//!   `<<( :a :p :b )>>` and `<<( :a :p :c )>>` stay two terms even when `:b owl:sameAs :c`.
//!   That is the [`Construct::TripleTerm`](crate::Construct::TripleTerm) boundary, and it
//!   is reported on every run whose input holds one.
//!
//! # `eq-ref` fires on every triple, and that is the specification
//!
//! `eq-ref` types every term of every triple `owl:sameAs` itself, so a closure carries one
//! reflexive assertion per distinct term. It is also what makes `eq-diff1` able to see
//! `T(?x, owl:differentFrom, ?x)` as a clash, and what makes `eq-rep-*` idempotent rather
//! than partial. A reflexive assertion whose subject is a LITERAL is a generalized-RDF
//! triple and is dropped at the boundary, which is why `eq-ref` is stated as three clauses
//! — one per position — rather than one conjunctive head: the subject and predicate
//! conjuncts stay licensed when the object conjunct cannot be represented.

use purrdf_datalog::clause::DlClause;

use super::{atom, internal, internal_graph, iri, quad, var};
use crate::lists::{INDEX_DISTINCT_RELATION, LIST_RELATION};
use crate::vocab::{
    OWL_ALLDIFFERENT, OWL_DIFFERENTFROM, OWL_DISTINCTMEMBERS, OWL_MEMBERS, OWL_SAMEAS, RDF_TYPE,
};

/// `eq-ref`: `T(?s, ?p, ?o)` ⇒ `?s owl:sameAs ?s`, `?p owl:sameAs ?p`, `?o owl:sameAs ?o`.
///
/// Three clauses, because the conclusion is a CONJUNCTION and a conjunctive head is not a
/// Datalog clause — the same shape `rdfs4` takes for the same reason.
pub(super) fn reflexive() -> Vec<DlClause> {
    vec![
        DlClause::datalog(
            atom(var("?s"), OWL_SAMEAS, var("?s")),
            vec![quad(var("?s"), var("?p"), var("?o"))],
        ),
        DlClause::datalog(
            atom(var("?p"), OWL_SAMEAS, var("?p")),
            vec![quad(var("?s"), var("?p"), var("?o"))],
        ),
        DlClause::datalog(
            atom(var("?o"), OWL_SAMEAS, var("?o")),
            vec![quad(var("?s"), var("?p"), var("?o"))],
        ),
    ]
}

/// `eq-sym`: `?x owl:sameAs ?y` ⇒ `?y owl:sameAs ?x`.
pub(super) fn symmetric() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y"), OWL_SAMEAS, var("?x")),
        vec![atom(var("?x"), OWL_SAMEAS, var("?y"))],
    )]
}

/// `eq-trans`: `?x owl:sameAs ?y`, `?y owl:sameAs ?z` ⇒ `?x owl:sameAs ?z`.
pub(super) fn transitive() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?x"), OWL_SAMEAS, var("?z")),
        vec![
            atom(var("?x"), OWL_SAMEAS, var("?y")),
            atom(var("?y"), OWL_SAMEAS, var("?z")),
        ],
    )]
}

/// `eq-rep-s`: `?s owl:sameAs ?s'`, `T(?s, ?p, ?o)` ⇒ `T(?s', ?p, ?o)`.
pub(super) fn equal_subject() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?s2"), var("?p"), var("?o")),
        vec![
            atom(var("?s"), OWL_SAMEAS, var("?s2")),
            quad(var("?s"), var("?p"), var("?o")),
        ],
    )]
}

/// `eq-rep-p`: `?p owl:sameAs ?p'`, `T(?s, ?p, ?o)` ⇒ `T(?s, ?p', ?o)`.
///
/// The one rule of the whole calculus whose conclusion rewrites the PREDICATE position
/// from a variable bound in a DIFFERENT position of another atom — `?p2` is the OBJECT of
/// the `owl:sameAs` triple and the PREDICATE of the conclusion. It is expressible only
/// because a clause atom carries its predicate as data; an IR that addressed relations by
/// predicate symbol could not write it. A `?p2` that is not an IRI makes the conclusion a
/// generalized-RDF triple, dropped at the boundary and reported.
pub(super) fn equal_predicate() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?s"), var("?p2"), var("?o")),
        vec![
            atom(var("?p"), OWL_SAMEAS, var("?p2")),
            quad(var("?s"), var("?p"), var("?o")),
        ],
    )]
}

/// `eq-rep-o`: `?o owl:sameAs ?o'`, `T(?s, ?p, ?o)` ⇒ `T(?s, ?p, ?o')`.
pub(super) fn equal_object() -> Vec<DlClause> {
    vec![DlClause::datalog(
        quad(var("?s"), var("?p"), var("?o2")),
        vec![
            atom(var("?o"), OWL_SAMEAS, var("?o2")),
            quad(var("?s"), var("?p"), var("?o")),
        ],
    )]
}

/// `eq-diff1`: `?x owl:sameAs ?y`, `?x owl:differentFrom ?y` ⇒ `false`.
pub(super) fn different1() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), OWL_SAMEAS, var("?y")),
        atom(var("?x"), OWL_DIFFERENTFROM, var("?y")),
    ])]
}

/// `eq-diff2`: `?x rdf:type owl:AllDifferent`, `?x owl:members ?y` over the list
/// `?z1 … ?zn`, `?zi owl:sameAs ?zj` with `i ≠ j` ⇒ `false`.
///
/// The two members come from [`crate::lists`]'s `LIST(head, index, member)` and `i ≠ j` is
/// `INDEX_DISTINCT(?i, ?j)` over the index pairs the pre-pass materializes —
/// exactly as `prp-adp` and `cax-adc` express the same side condition. Dropping it would
/// let `i = j` match and make `eq-ref`'s own reflexive assertion an inconsistency for
/// every one-member `owl:AllDifferent` list, which is unsound.
pub(super) fn different_members() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), RDF_TYPE, iri(OWL_ALLDIFFERENT)),
        atom(var("?x"), OWL_MEMBERS, var("?y")),
        internal(LIST_RELATION, var("?y"), var("?zi"), var("?i")),
        internal(LIST_RELATION, var("?y"), var("?zj"), var("?j")),
        internal(
            INDEX_DISTINCT_RELATION,
            var("?i"),
            var("?j"),
            internal_graph(),
        ),
        atom(var("?zi"), OWL_SAMEAS, var("?zj")),
    ])]
}

/// `eq-diff3`: the same over `owl:distinctMembers`.
///
/// Not redundant with [`different_members`]: OWL 2's RDF mapping writes an
/// `owl:AllDifferent` axiom with `owl:members` when it is a plain list and with
/// `owl:distinctMembers` in the form OWL 1 fixed, and a graph may carry either.
pub(super) fn different_distinct_members() -> Vec<DlClause> {
    vec![DlClause::inconsistency(vec![
        atom(var("?x"), RDF_TYPE, iri(OWL_ALLDIFFERENT)),
        atom(var("?x"), OWL_DISTINCTMEMBERS, var("?y")),
        internal(LIST_RELATION, var("?y"), var("?zi"), var("?i")),
        internal(LIST_RELATION, var("?y"), var("?zj"), var("?j")),
        internal(
            INDEX_DISTINCT_RELATION,
            var("?i"),
            var("?j"),
            internal_graph(),
        ),
        atom(var("?zi"), OWL_SAMEAS, var("?zj")),
    ])]
}

/// The `eq-*` rules this chase states, in OWL 2 Profiles Table 4 order.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means — including
/// what the optional `concludes:` field says about the three rules that conclude `false`.
macro_rules! eq_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `eq-ref` — every term of every triple is the same as itself. `OWL-RL` only.
            Reflexive {
                id: EqRef,
                lanes: [OwlRl],
                clauses: eq::reflexive,
            },
            /// `eq-sym` — `owl:sameAs` is symmetric. `OWL-RL` only.
            SameAsSymmetric {
                id: EqSym,
                lanes: [OwlRl],
                clauses: eq::symmetric,
            },
            /// `eq-trans` — `owl:sameAs` is transitive. `OWL-RL` only.
            SameAsTransitive {
                id: EqTrans,
                lanes: [OwlRl],
                clauses: eq::transitive,
            },
            /// `eq-rep-s` — equality substitutes in SUBJECT position. `OWL-RL` only.
            EqualSubject {
                id: EqRepS,
                lanes: [OwlRl],
                clauses: eq::equal_subject,
            },
            /// `eq-rep-p` — equality substitutes in PREDICATE position. `OWL-RL` only.
            EqualPredicate {
                id: EqRepP,
                lanes: [OwlRl],
                clauses: eq::equal_predicate,
            },
            /// `eq-rep-o` — equality substitutes in OBJECT position. `OWL-RL` only.
            EqualObject {
                id: EqRepO,
                lanes: [OwlRl],
                clauses: eq::equal_object,
            },
            /// `eq-diff1` — two things both same and different is an inconsistency.
            Different1 {
                id: EqDiff1,
                lanes: [OwlRl],
                clauses: eq::different1,
                concludes: Inconsistency,
            },
            /// `eq-diff2` — the same, over an `owl:AllDifferent` `owl:members` list.
            DifferentMembers {
                id: EqDiff2,
                lanes: [OwlRl],
                clauses: eq::different_members,
                concludes: Inconsistency,
            },
            /// `eq-diff3` — the same, over an `owl:distinctMembers` list.
            DifferentDistinctMembers {
                id: EqDiff3,
                lanes: [OwlRl],
                clauses: eq::different_distinct_members,
                concludes: Inconsistency,
            },
        }
    };
}

pub(crate) use eq_rules;
