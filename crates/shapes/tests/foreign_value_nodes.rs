// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Constraints evaluated against a value node the data graph never interned.**
//!
//! Almost every value node a SHACL evaluation sees comes out of the data graph
//! and therefore carries an interned identity, and the constraint layer leans on
//! that hard: membership, identity and set constraints all compare integer term
//! ids rather than owned terms, because the interner is injective and id equality
//! IS term equality — for interned terms.
//!
//! A value node that is **not** interned breaks the second half of that sentence,
//! and it is reachable from ordinary SHACL: `sh:targetNode` naming a node the data
//! graph does not mention makes the focus node itself non-interned, and a
//! zero-length path leg (`sh:zeroOrOnePath`, `sh:zeroOrMorePath`) then carries
//! that same non-interned term into a property shape's value node set. A SHACL-AF
//! node expression can produce one too.
//!
//! The trap this file exists to hold shut is a single inference:
//!
//! > the term is not interned, therefore nothing can equal it, therefore the
//! > constraint is violated.
//!
//! The first clause is true and the third does not follow. **Two non-interned
//! terms can be equal.** `sh:hasValue ex:absent` against a focus node `ex:absent`
//! that the data graph never mentions is a CONFORMING validation, and an
//! evaluator that constant-folds "no candidate is interned" into "everything
//! violates" refuses it — an over-refusal, which looks exactly like correct
//! strictness and breaks nothing visibly until a user writes the shape that
//! should pass and doesn't.
//!
//! So every constraint below is executed twice: once in a spelling that really is
//! violated, and once in a NEIGHBOURING spelling that really conforms. A case
//! table that only ever expected violations would be satisfied perfectly by the
//! bug.
//!
//! # What is covered
//!
//! Both producers of a non-interned value node:
//!
//! * the node-shape route — the focus node is the single value node; and
//! * the property-shape route — a zero-length path leg yields the non-interned
//!   focus term back as a value node.
//!
//! and, over them, the identity/set constraints whose id-space shortcut is the
//! thing at risk (`sh:hasValue`, `sh:in`, `sh:equals`, `sh:disjoint`) plus the
//! recursive constraints that re-enter the evaluator with such a value node as
//! the new focus (`sh:not`, `sh:node`, `sh:and`, `sh:or`, `sh:xone`,
//! `sh:qualifiedValueShape`). `sh:xone` is stated three ways — nought, one and two
//! conforming members — because its verdict is a COUNT and not a boolean, and a
//! count is the easiest of the recursive constraints to get subtly wrong.
//!
//! Every IRI is under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::Arc;

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::data_view::ShaclRead;
use purrdf_shapes::engine::validate_graphs;
use purrdf_shapes::term::Term;
use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;

/// The fixture namespace. Caller-supplied and `example.org` by rule.
const NS: &str = "http://example.org/purrdf/foreign#";

/// The focus node every case targets, and the term whose non-internment is the
/// whole premise of this file. Asserted, not assumed — see
/// [`the_target_node_is_really_absent_from_the_data_graph`].
const ABSENT: &str = "http://example.org/purrdf/foreign#absent";

/// The data graph. It interns `ex:present`, `ex:p`, `ex:q` and `"alpha"`, and
/// deliberately never mentions `ex:absent`, `ex:other`, `ex:another` or `ex:next`.
///
/// It is non-empty on purpose: a validator handed an empty data graph can be
/// wrong about interning in ways that an empty graph hides, and the cases below
/// compare a non-interned value node against BOTH interned and non-interned
/// candidates.
const DATA: &str = concat!(
    "<http://example.org/purrdf/foreign#present> <http://example.org/purrdf/foreign#p> \
     <http://example.org/purrdf/foreign#q> .\n",
    "<http://example.org/purrdf/foreign#present> <http://example.org/purrdf/foreign#tag> \
     \"alpha\" .\n",
);

/// Turtle prefixes every case's shapes graph opens with.
const PREFIXES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/purrdf/foreign#> .\n",
);

/// The `sh:targetNode` line every case shares, so each case body is only the
/// constraint under test.
const TARGET: &str = "ex:S a sh:NodeShape ; sh:targetNode ex:absent ;\n";

/// One constraint spelling and the results it must produce.
struct ForeignCase {
    /// Appears in every assertion message, so a failure names which spelling broke.
    name: &'static str,
    /// The body of `ex:S`, appended to [`TARGET`], terminated by the case.
    shape: &'static str,
    /// The local names of the constraint components the case must report, in
    /// order. Empty means the case must CONFORM — which is the half of this table
    /// that the over-refusal bug fails.
    expect: &'static [&'static str],
}

/// Every constraint spelling this file pins, conforming and violating side by
/// side.
///
/// The pairing is the point. For each constraint there is a spelling whose
/// non-interned value node really does satisfy it and one whose does not, and
/// both are executed: an evaluator that refuses every non-interned value node
/// passes the violating half of the table and fails the conforming half, and an
/// evaluator that accepts every non-interned value node does the reverse.
const CASES: &[ForeignCase] = &[
    // ── sh:hasValue ────────────────────────────────────────────────────────────
    //
    // The required term is not interned either, so the interned-id comparison
    // cannot decide this and term equality must. Two non-interned terms are
    // equal here, and the constraint is SATISFIED.
    ForeignCase {
        name: "has_value/foreign required term equals the foreign value node",
        shape: "    sh:hasValue ex:absent .",
        expect: &[],
    },
    ForeignCase {
        name: "has_value/foreign required term differs from the foreign value node",
        shape: "    sh:hasValue ex:other .",
        expect: &["HasValueConstraintComponent"],
    },
    // The required term IS interned and the value node is not. Here the
    // injectivity argument really does apply: a term equal to an interned term is
    // interned, so a non-interned value node cannot be it.
    ForeignCase {
        name: "has_value/interned required term cannot equal a foreign value node",
        shape: "    sh:hasValue ex:q .",
        expect: &["HasValueConstraintComponent"],
    },
    // ── sh:in ──────────────────────────────────────────────────────────────────
    //
    // NO member of this list is interned, so the id set the bind resolved is
    // EMPTY — and an empty id set is not a constant verdict. The foreign value
    // node equals a member, and the constraint is satisfied.
    ForeignCase {
        name: "in/no member is interned but one equals the foreign value node",
        shape: "    sh:in ( ex:absent ex:other ) .",
        expect: &[],
    },
    ForeignCase {
        name: "in/no member is interned and none equals the foreign value node",
        shape: "    sh:in ( ex:other ex:another ) .",
        expect: &["InConstraintComponent"],
    },
    // A mixed list: the id-set probe misses, and the term comparison behind it
    // must still run.
    ForeignCase {
        name: "in/mixed interned and foreign members, the foreign one matches",
        shape: "    sh:in ( ex:q ex:absent ) .",
        expect: &[],
    },
    // ── sh:disjoint / sh:equals ────────────────────────────────────────────────
    //
    // A non-interned focus node has no outgoing quads, so the compared
    // predicate's object set is empty. Disjointness from the empty set holds…
    ForeignCase {
        name: "disjoint/a foreign value node is disjoint from an empty comparand set",
        shape: "    sh:disjoint ex:p .",
        expect: &[],
    },
    // …and equality with it does not: the value node is missing from the
    // comparand set, which is exactly one result naming that value node.
    ForeignCase {
        name: "equals/a foreign value node is absent from an empty comparand set",
        shape: "    sh:equals ex:p .",
        expect: &["EqualsConstraintComponent"],
    },
    // The valid neighbour for `sh:equals` under a foreign focus: when the path
    // yields NO value nodes, both sides are empty and the constraint holds. This
    // is what says the arm refuses a mismatch rather than refusing a foreign
    // focus.
    ForeignCase {
        name: "equals/an empty value set under a foreign focus conforms",
        shape: "    sh:property [ sh:path ex:tag ; sh:equals ex:p ] .",
        expect: &[],
    },
    // ── sh:not (recursive) ─────────────────────────────────────────────────────
    //
    // The inner shape CONFORMS against the foreign value node (the `sh:hasValue`
    // case above), so the negation is violated. Polarity is the thing being
    // pinned: an evaluator that refused the inner shape would report this as
    // conforming.
    ForeignCase {
        name: "not/the negated shape conforms against the foreign value node",
        shape: "    sh:not [ a sh:NodeShape ; sh:hasValue ex:absent ] .",
        expect: &["NotConstraintComponent"],
    },
    ForeignCase {
        name: "not/the negated shape does not conform against the foreign value node",
        shape: "    sh:not [ a sh:NodeShape ; sh:hasValue ex:other ] .",
        expect: &[],
    },
    // ── sh:node (recursive) ────────────────────────────────────────────────────
    ForeignCase {
        name: "node/the inner shape conforms against the foreign value node",
        shape: "    sh:node [ a sh:NodeShape ; sh:in ( ex:absent ) ] .",
        expect: &[],
    },
    ForeignCase {
        name: "node/the inner shape does not conform against the foreign value node",
        shape: "    sh:node [ a sh:NodeShape ; sh:in ( ex:q ) ] .",
        expect: &["NodeConstraintComponent"],
    },
    // ── sh:and / sh:or (recursive) ─────────────────────────────────────────────
    ForeignCase {
        name: "and/every member conforms against the foreign value node",
        shape: "    sh:and (
        [ a sh:NodeShape ; sh:hasValue ex:absent ]
        [ a sh:NodeShape ; sh:in ( ex:absent ) ]
    ) .",
        expect: &[],
    },
    ForeignCase {
        name: "and/one member does not conform against the foreign value node",
        shape: "    sh:and (
        [ a sh:NodeShape ; sh:hasValue ex:absent ]
        [ a sh:NodeShape ; sh:hasValue ex:other ]
    ) .",
        expect: &["AndConstraintComponent"],
    },
    ForeignCase {
        name: "or/the second member conforms against the foreign value node",
        shape: "    sh:or (
        [ a sh:NodeShape ; sh:hasValue ex:other ]
        [ a sh:NodeShape ; sh:hasValue ex:absent ]
    ) .",
        expect: &[],
    },
    ForeignCase {
        name: "or/no member conforms against the foreign value node",
        shape: "    sh:or (
        [ a sh:NodeShape ; sh:hasValue ex:other ]
        [ a sh:NodeShape ; sh:hasValue ex:another ]
    ) .",
        expect: &["OrConstraintComponent"],
    },
    // ── sh:xone (recursive, and a COUNT rather than a boolean) ─────────────────
    //
    // Stated three ways so the count itself is pinned and not merely its
    // zero/non-zero shadow.
    ForeignCase {
        name: "xone/exactly one member conforms against the foreign value node",
        shape: "    sh:xone (
        [ a sh:NodeShape ; sh:hasValue ex:absent ]
        [ a sh:NodeShape ; sh:hasValue ex:other ]
    ) .",
        expect: &[],
    },
    ForeignCase {
        name: "xone/two members conform against the foreign value node",
        shape: "    sh:xone (
        [ a sh:NodeShape ; sh:hasValue ex:absent ]
        [ a sh:NodeShape ; sh:in ( ex:absent ) ]
    ) .",
        expect: &["XoneConstraintComponent"],
    },
    ForeignCase {
        name: "xone/no member conforms against the foreign value node",
        shape: "    sh:xone (
        [ a sh:NodeShape ; sh:hasValue ex:other ]
        [ a sh:NodeShape ; sh:hasValue ex:another ]
    ) .",
        expect: &["XoneConstraintComponent"],
    },
    // ── The property-shape producer ────────────────────────────────────────────
    //
    // A zero-length path leg yields the non-interned focus term back as a VALUE
    // NODE, which is the second and less obvious way a foreign value node reaches
    // a constraint — and the only one that reaches the property-shape evaluator.
    ForeignCase {
        name: "property/a zero-length path leg carries the foreign focus into hasValue",
        shape: "    sh:property [ sh:path [ sh:zeroOrOnePath ex:next ] ; sh:hasValue ex:absent ] .",
        expect: &[],
    },
    ForeignCase {
        name: "property/a zero-length path leg carries the foreign focus into a failing sh:in",
        shape: "    sh:property [ sh:path [ sh:zeroOrOnePath ex:next ] ; sh:in ( ex:other ) ] .",
        expect: &["InConstraintComponent"],
    },
    // ── sh:qualifiedValueShape (recursive, over the property-shape producer) ───
    ForeignCase {
        name: "qualified/the foreign value node conforms to the qualified shape",
        shape: "    sh:property [
        sh:path [ sh:zeroOrOnePath ex:next ] ;
        sh:qualifiedValueShape [ a sh:NodeShape ; sh:hasValue ex:absent ] ;
        sh:qualifiedMinCount 1
    ] .",
        expect: &[],
    },
    ForeignCase {
        name: "qualified/the foreign value node does not conform to the qualified shape",
        shape: "    sh:property [
        sh:path [ sh:zeroOrOnePath ex:next ] ;
        sh:qualifiedValueShape [ a sh:NodeShape ; sh:hasValue ex:other ] ;
        sh:qualifiedMinCount 1
    ] .",
        expect: &["QualifiedMinCountConstraintComponent"],
    },
];

/// The premise of every case in this file, asserted rather than assumed.
///
/// If the data graph ever DID intern `ex:absent`, every value node below would be
/// an ordinary interned one, the whole table would still pass, and it would be
/// pinning nothing at all — the most comfortable way for a coverage file to
/// become decorative.
#[test]
fn the_target_node_is_really_absent_from_the_data_graph() {
    let data: Arc<_> = parse_ntriples_to_dataset(DATA).expect("the fixture data graph parses");
    let store = ShaclData::new(Arc::clone(&data), data, None);
    let view = store.core_view();
    assert!(
        view.term_id_by_iri(ABSENT).is_none(),
        "{ABSENT} must not be interned by the fixture data graph, or no case in this file \
         evaluates a non-interned value node"
    );
    assert!(
        view.term_id_by_iri(&format!("{NS}q")).is_some(),
        "the fixture data graph must intern {NS}q, or the cases that contrast an interned \
         candidate against a foreign value node are contrasting nothing"
    );
}

/// Render one result as the case table spells it: the constraint component's
/// local name.
fn component_of(result: &purrdf_shapes::report::ValidationResult) -> String {
    let iri = result.source_constraint_component.as_str();
    iri.rsplit_once('#')
        .map_or_else(|| iri.to_owned(), |(_, local)| local.to_owned())
}

/// **Every constraint reaches the right verdict against a value node the data
/// graph never interned.**
#[test]
fn constraints_decide_foreign_value_nodes_by_term_equality() {
    let absent = Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(ABSENT));
    for case in CASES {
        let shapes = format!("{PREFIXES}{TARGET}{}\n", case.shape);
        let report = validate_graphs(DATA, &shapes, None).unwrap_or_else(|error| {
            panic!(
                "case {}: the shapes graph must validate: {error}\n{shapes}",
                case.name
            )
        });

        let produced: Vec<String> = report.results.iter().map(component_of).collect();
        let expected: Vec<String> = case.expect.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(
            produced, expected,
            "case {}: wrong results against a non-interned value node.\nshapes:\n{shapes}",
            case.name
        );
        assert_eq!(
            report.conforms,
            case.expect.is_empty(),
            "case {}: sh:conforms disagrees with the results it accompanies",
            case.name
        );
        for result in &report.results {
            assert_eq!(
                result.focus_node, absent,
                "case {}: every result must name the foreign target as its focus node",
                case.name
            );
        }
    }
}
