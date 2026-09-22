// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The differential oracle**: a memo must answer what the rewrite answers.
//!
//! The hazard this module carries is not that a memo is slow or that it declines too
//! often. It is that a memo answers *differently* from the rewrite it stands in for,
//! in some shape neither the author nor the reviewer thought of — a query whose
//! pre-bound variable sits in a position [`super::walk_query`] never reaches, so the
//! retained tree keeps the previous run's term there and the query silently answers
//! about the wrong focus node.
//!
//! So the oracle is enumerated rather than argued. Every query in [`QUERIES`] is run
//! against every binding set in [`BINDINGS`] on both lanes, and for each combination
//! the memo's answer — the retained tree after its values are written — is compared
//! against the tree [`super::rewrite`] builds from scratch for those same bindings.
//! Equality is over the whole [`Query`], node for node, so nothing is projected away.
//!
//! # What this compares, and what it cannot catch
//!
//! It compares the **substituted algebra**, which is the evaluator's entire input
//! apart from the dataset and the evaluation context, neither of which the memo
//! touches. Two runs handed structurally equal algebra over one dataset cannot
//! produce different answers, so algebra equality is a stronger statement than
//! result equality and a cheaper one to make exact — comparing results would have to
//! pick an ordering for a bag and would say nothing at all about a `CONSTRUCT`
//! template or a `DESCRIBE` target list.
//!
//! It cannot catch a defect in the rewrite ITSELF: if `apply_probes` is wrong, the
//! memo reproduces it faithfully and both sides agree. That is deliberate — this
//! module's claim is that a memo is indistinguishable from the rewrite, not that the
//! rewrite is right, which is what `crate::substitute`'s own tests and the SHACL
//! conformance suites are for.
//!
//! It also cannot see a query shape that is not in [`QUERIES`]. That is why the same
//! comparison runs in production, every time a memo is created:
//! [`PrebindMemo::build`]'s last step replays a moved value set into the memo and
//! refuses the memo unless the result equals what the real rewrite produces. A query
//! shape nobody wrote a case for here still cannot install a memo that disagrees.

use purrdf_sparql_algebra::{
    BlankNode, GroundTerm, GroundTriple, Literal, NamedNode, SparqlParser,
};

use super::{PrebindMemo, ValueShape, rewrite};
use crate::engine::ShaclPrebinding;

/// Both lanes. The SHACL lane adds the expression-position walk on top of the plain
/// lane's pushdown and seed, so a position only one of them writes is a position only
/// one of them can leave stale.
const LANES: [ShaclPrebinding; 2] = [ShaclPrebinding::Applied, ShaclPrebinding::None];

/// Parse `text` as a query, or fail the test naming it.
fn parse(text: &str) -> purrdf_sparql_algebra::Query {
    SparqlParser::new()
        .parse_query(text)
        .unwrap_or_else(|e| panic!("the fixture query does not parse: {e}\n{text}"))
}

/// A pre-binding list from `(name, value)` pairs.
fn probes(pairs: &[(&str, GroundTerm)]) -> Vec<(purrdf_sparql_algebra::Variable, GroundTerm)> {
    pairs
        .iter()
        .map(|(name, ground)| (crate::substitute::interned_variable(name), ground.clone()))
        .collect()
}

/// An `example.org` IRI; the fixtures mint no vocabulary.
fn iri(local: &str) -> GroundTerm {
    GroundTerm::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/purrdf/prebind#{local}"
    )))
}

/// A plain literal.
fn literal(lexical: &str) -> GroundTerm {
    GroundTerm::Literal(Literal::new_simple(lexical))
}

/// A blank node — the value that is bound by the seed and never pushed.
fn blank(label: &str) -> GroundTerm {
    GroundTerm::BlankNode(BlankNode::new(label))
}

/// A quoted triple of IRIs, every position of which a pattern can carry.
fn quoted(local: &str) -> GroundTerm {
    let GroundTerm::NamedNode(predicate) = iri("p") else {
        unreachable!("iri built a NamedNode")
    };
    GroundTerm::Triple(Box::new(GroundTriple {
        subject: iri(local),
        predicate,
        object: iri("o"),
    }))
}

/// The query shapes the memo is enumerated over.
///
/// Chosen for the positions the rewrite writes and the boundaries it stops at, not
/// for variety: a leaf with a pre-bound subject; an `OPTIONAL` and a `MINUS`, whose
/// right arms the pushdown must NOT enter; a `Project` boundary, which stops the
/// pushdown but not the SHACL expression walk; expression positions including
/// `BOUND`; a `GRAPH` name; a property path; a quoted triple in a pattern position; a
/// `GROUP BY` with a sort key inside the aggregate; and a `VALUES` block the query
/// wrote itself, whose cells must never be mistaken for the seed's.
const QUERIES: &[&str] = &[
    "SELECT ?o WHERE { $this <http://example.org/purrdf/prebind#p> ?o }",
    "SELECT ?o WHERE { ?s <http://example.org/purrdf/prebind#p> ?o \
     OPTIONAL { $this <http://example.org/purrdf/prebind#q> ?o } }",
    "SELECT ?s WHERE { ?s <http://example.org/purrdf/prebind#p> ?o \
     MINUS { $this <http://example.org/purrdf/prebind#q> ?o } }",
    "SELECT ?o WHERE { { SELECT ?o WHERE { $this <http://example.org/purrdf/prebind#p> ?o } } }",
    "SELECT ?o WHERE { ?s <http://example.org/purrdf/prebind#p> ?o \
     FILTER(?o = $this && BOUND($this)) }",
    "SELECT ?o WHERE { GRAPH $this { ?s <http://example.org/purrdf/prebind#p> ?o } }",
    "SELECT ?o WHERE { $this <http://example.org/purrdf/prebind#p>+ ?o }",
    "SELECT ?o WHERE { << $this <http://example.org/purrdf/prebind#p> ?o >> \
     <http://example.org/purrdf/prebind#q> ?x }",
    "SELECT ?s (GROUP_CONCAT(?o) AS ?g) WHERE { ?s <http://example.org/purrdf/prebind#p> ?o \
     FILTER(?s != $this) } GROUP BY ?s",
    "SELECT ?o WHERE { VALUES ?o { <http://example.org/purrdf/prebind#fixed> } \
     $this <http://example.org/purrdf/prebind#p> ?o }",
    "SELECT ?o WHERE { $this <http://example.org/purrdf/prebind#p> ?o \
     FILTER EXISTS { ?o <http://example.org/purrdf/prebind#q> $this } }",
    "SELECT ?o WHERE { BIND($this AS ?o) }",
    "ASK { $this <http://example.org/purrdf/prebind#p> ?o }",
    "CONSTRUCT { $this <http://example.org/purrdf/prebind#p> ?o } \
     WHERE { $this <http://example.org/purrdf/prebind#p> ?o }",
    "SELECT ?o WHERE { ?s <http://example.org/purrdf/prebind#p> ?o } ORDER BY $this",
    "SELECT ?o WHERE { $this <http://example.org/purrdf/prebind#p> ?o . \
     $other <http://example.org/purrdf/prebind#q> ?o }",
];

/// The binding sets every query is run against, in the order a run would see them.
///
/// Two parameters throughout, because a one-parameter list cannot expose an
/// attribution mistake between two of them. `$other` is declared by every query in
/// [`QUERIES`] or by none of them — a pre-binding naming a variable the query does
/// not mention is exactly the no-op case worth carrying.
fn bindings() -> Vec<Vec<(&'static str, GroundTerm)>> {
    vec![
        vec![("this", iri("a")), ("other", iri("b"))],
        vec![("this", iri("c")), ("other", iri("d"))],
        // The same term in both slots: the case that would make attribution
        // ambiguous if the build varied every parameter at once instead of one at a
        // time.
        vec![("this", iri("e")), ("other", iri("e"))],
        vec![("this", literal("x")), ("other", literal("y"))],
        vec![("this", blank("b0")), ("other", blank("b1"))],
        // Mixed shapes: a pushed value beside one that only the seed can carry.
        vec![("this", iri("f")), ("other", blank("b2"))],
        vec![("this", blank("b3")), ("other", iri("g"))],
        vec![("this", quoted("h")), ("other", iri("i"))],
    ]
}

/// **The oracle.** For every query, lane and binding set, a memo built for that
/// binding set's shapes and then bound to a binding set must hold exactly the tree
/// the rewrite builds from scratch.
///
/// The inner loop rebinds one memo across every binding set of matching shape, which
/// is the real usage: a memo is built once and written into many times, so a cell the
/// replay fails to overwrite shows up as the EARLIER binding's term surviving into a
/// later run's tree.
#[test]
fn a_memo_holds_what_the_rewrite_builds() {
    for text in QUERIES {
        let parsed = parse(text);
        for lane in LANES {
            for built_from in bindings() {
                let build_probes = probes(&built_from);
                let Some(mut memo) = PrebindMemo::build(&parsed, lane, &build_probes) else {
                    // A declined memo is a performance decision, never an answer:
                    // the run takes the ordinary rewrite. The only shape declined
                    // here is a quoted-triple value, and the case below pins that.
                    continue;
                };
                for bound_to in bindings() {
                    let run_probes = probes(&bound_to);
                    if !memo.matches(lane, &run_probes) {
                        continue;
                    }
                    let expected = rewrite(parsed.clone(), lane, run_probes.clone());
                    assert_eq!(
                        memo.bind(&run_probes),
                        &expected,
                        "memo built from {built_from:?} and bound to {bound_to:?} disagrees \
                         with the rewrite\n  lane: {lane:?}\n  query: {text}"
                    );
                }
            }
        }
    }
}

/// A memo is built for the SHAPES it saw, and answers for no others.
///
/// This is the guard constraint 3 of the change rests on: a blank-node value is bound
/// by the `VALUES` seed alone while an IRI one is also pushed into the leaves, so the
/// two produce trees with different cells. A memo built for one must refuse the
/// other rather than write an IRI into a tree that has no pushed position for it.
#[test]
fn a_memo_refuses_a_shape_it_was_not_built_for() {
    let parsed = parse(QUERIES[0]);
    let as_iri = probes(&[("this", iri("a")), ("other", iri("b"))]);
    let as_blank = probes(&[("this", blank("b0")), ("other", iri("b"))]);

    let memo = PrebindMemo::build(&parsed, ShaclPrebinding::None, &as_iri)
        .expect("an IRI-shaped binding list is memoizable");
    assert!(memo.matches(ShaclPrebinding::None, &as_iri));
    assert!(
        !memo.matches(ShaclPrebinding::None, &as_blank),
        "a blank-node value is bound by the seed alone, so it occupies different cells"
    );
    assert!(
        !memo.matches(ShaclPrebinding::Applied, &as_iri),
        "the two lanes rewrite the same query into different trees"
    );

    // And the neighbouring case really is valid: the blank-shaped list memoizes on
    // its own, so the refusal above is about the SHAPE and not about blank nodes
    // being unsupported.
    let blank_memo = PrebindMemo::build(&parsed, ShaclPrebinding::None, &as_blank)
        .expect("a blank-node binding list is memoizable in its own right");
    assert!(blank_memo.matches(ShaclPrebinding::None, &as_blank));
}

/// A quoted-triple value is declined, and declining it is not a refusal of the query.
#[test]
fn a_quoted_triple_value_declines_the_memo_and_still_rewrites() {
    let parsed = parse(QUERIES[0]);
    let nested = probes(&[("this", quoted("a")), ("other", iri("b"))]);
    assert_eq!(ValueShape::of(&nested[0].1), ValueShape::NestedTriple);
    assert!(
        PrebindMemo::build(&parsed, ShaclPrebinding::None, &nested).is_none(),
        "a value that changes how many term positions a pattern holds is declined"
    );
    // The ordinary rewrite still answers for it, unchanged — the decline costs the
    // run its memo and nothing else.
    let rewritten = rewrite(parsed.clone(), ShaclPrebinding::None, nested);
    assert_ne!(rewritten, parsed, "the rewrite still pre-binds the value");
}

/// A repeated pre-bound name keeps `apply_probes`' own per-variable path, and a memo
/// declines rather than describing the combined-seed shape it does not have.
#[test]
fn a_repeated_name_declines_the_memo() {
    let parsed = parse(QUERIES[0]);
    let repeated = probes(&[("this", iri("a")), ("this", iri("b"))]);
    assert!(
        PrebindMemo::build(&parsed, ShaclPrebinding::None, &repeated).is_none(),
        "a repeated name takes the per-variable path, which builds a different tree"
    );
    // And the per-variable path is still what runs: two seeds binding one variable to
    // two different terms are incompatible, which is the defined answer rather than
    // an error.
    let rewritten = rewrite(parsed, ShaclPrebinding::None, repeated);
    assert!(
        format!("{rewritten:?}").matches("Values").count() >= 2,
        "the per-variable path plants one seed per pre-binding"
    );
}
