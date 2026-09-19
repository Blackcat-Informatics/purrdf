// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! THE SUPERSET LAW of incremental validation.
//!
//! `PreparedValidator::validate_focus_node_ids` re-validates a bounded set of focus
//! nodes. That is correct only if the set already CONTAINS every node whose verdict
//! the change could move, and nothing about a short set looks wrong: the report says
//! `conforms`, every assertion passes, and the violation nobody re-checked is simply
//! absent. So the property under test here is not "the expansion equals a list
//! someone wrote down" — a hand-listed expectation just re-states the
//! implementation's own assumptions and agrees with it when both are wrong.
//!
//! The property is DIFFERENTIAL, and it is the only formulation that can fail for
//! the right reason:
//!
//! 1. validate the base graph in full;
//! 2. apply a delta and validate the changed graph in full;
//! 3. diff the two result sets — every tuple that appeared or disappeared is a
//!    verdict that MOVED; and
//! 4. assert every focus node behind a moved verdict is in
//!    `affected_focus_node_ids`.
//!
//! The oracle is full validation itself, so the test knows the right answer without
//! being told it, and an expansion that forgets `sh:inversePath` fails whatever the
//! author believed about inverse paths.
//!
//! # Not vacuous, in three ways
//!
//! A test of a SUPERSET passes trivially if the superset is everything, so each case
//! also asserts:
//!
//! * the delta MOVED at least one verdict (a case that changes nothing proves
//!   nothing);
//! * the expansion is bounded rather than TOP; and
//! * a named `untouched` focus node — a real target of the same shapes graph, with a
//!   verdict the change cannot reach — is NOT in the expansion.
//!
//! And the cases marked [`Case::naive_misses`] assert the mirror fact: that the
//! obvious expansion, `{s | (s, p, o) ∈ Δ}`, comes up SHORT on them. Without that,
//! nothing here would distinguish a real dependency analysis from returning the
//! subjects of the change.
//!
//! # The over-refusal pair
//!
//! Over-refusal is the same bug pointing the other way: an expansion that answers
//! TOP for an ordinary forward path is sound, passes every superset assertion above,
//! and makes incremental validation pointless. Both halves are executed —
//! [`a_shapes_graph_of_forward_paths_stays_bounded`] and
//! [`a_sparql_constraint_reports_an_unreadable_footprint`].
//!
//! Fixture IRIs are under `example.org`: PurRDF mints no vocabulary IRIs.

#![allow(clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf::ir::ViewLimits;
use purrdf::{DatasetMut, MutableDataset, QuadValues, TermId, TermValue};
use purrdf_shapes::engine::{PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::term::Term;
use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;

/// The prefix header every fixture shapes graph carries.
const PREFIXES: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/ns#> .
";

const EX: &str = "http://example.org/ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

/// One object position of a fixture row.
#[derive(Clone, Copy, Debug)]
enum Obj {
    /// An `ex:`-prefixed IRI.
    Ex(&'static str),
    /// A plain literal.
    Lit(&'static str),
}

/// One row of a delta, as `(subject local name, predicate IRI, object)`.
type Row = (&'static str, &'static str, Obj);

/// One differential case.
struct Case {
    /// What path form or target kind this case exercises.
    name: &'static str,
    /// The shapes graph, appended to [`PREFIXES`].
    shapes: &'static str,
    /// The base data graph, as N-Triples.
    data: &'static str,
    /// Rows the delta adds.
    inserts: &'static [Row],
    /// Rows the delta removes.
    removals: &'static [Row],
    /// A focus node of the same shapes graph whose verdict this change cannot
    /// reach. It must NOT appear in the expansion, which is what stops a
    /// superset assertion from passing by returning the whole graph.
    untouched: &'static str,
    /// Whether the obvious expansion — the SUBJECTS of the changed rows — misses a
    /// focus node whose verdict moved.
    ///
    /// `true` is the interesting direction: it asserts this case actually defeats
    /// the naive expansion, so passing it is evidence of a dependency analysis
    /// rather than of a coincidence.
    naive_misses: bool,
}

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

fn ex_term(local: &str) -> Term {
    Term::NamedNode(purrdf_shapes::term::NamedNode::new_unchecked(ex(local)))
}

fn object(obj: Obj) -> TermValue {
    match obj {
        Obj::Ex(local) => TermValue::iri(ex(local)),
        Obj::Lit(lexical) => TermValue::simple_literal(lexical),
    }
}

fn quad(row: Row) -> QuadValues {
    QuadValues {
        s: TermValue::iri(ex(row.0)),
        p: TermValue::iri(row.1),
        o: object(row.2),
        g: None,
    }
}

/// Every focus node named by a result, keyed by its rendered form — the same
/// rendering [`ValidationReport::result_tuples`] uses for a tuple's focus slot, so a
/// moved tuple can be traced back to the term it is about.
fn focus_terms(reports: [&ValidationReport; 2]) -> BTreeMap<String, Term> {
    reports
        .into_iter()
        .flat_map(|report| report.results.iter())
        .map(|result| (result.focus_node.to_string(), result.focus_node.clone()))
        .collect()
}

/// Run one differential case end to end.
fn assert_expansion_covers_every_moved_verdict(case: &Case) {
    let shapes = Arc::new(
        parse_shapes(&format!("{PREFIXES}{}", case.shapes), None)
            .unwrap_or_else(|error| panic!("{}: shapes must parse: {error}", case.name)),
    );
    let prepared = PreparedShapes::new(shapes);
    let base = parse_ntriples_to_dataset(case.data)
        .unwrap_or_else(|error| panic!("{}: data must parse: {error:?}", case.name));

    let before = prepared
        .bind_dataset(&base)
        .and_then(|validator| validator.validate())
        .unwrap_or_else(|error| panic!("{}: base validation: {error}", case.name));

    let mut mutation = MutableDataset::new(Arc::clone(&base));
    for &row in case.inserts {
        assert!(
            mutation
                .insert(quad(row))
                .unwrap_or_else(|error| panic!("{}: insert: {error}", case.name)),
            "{}: inserting {row:?} changed nothing, so the case tests nothing",
            case.name
        );
    }
    for &row in case.removals {
        assert!(
            mutation.remove(&quad(row)),
            "{}: removing {row:?} changed nothing, so the case tests nothing",
            case.name
        );
    }

    let snapshot = Arc::new(
        mutation
            .snapshot_view()
            .unwrap_or_else(|error| panic!("{}: snapshot: {error}", case.name)),
    );
    let validator = prepared
        .bind_delta_with_shapes_graph(Arc::clone(&snapshot), None, ViewLimits::default())
        .unwrap_or_else(|error| panic!("{}: delta bind: {error}", case.name));
    let after = validator
        .validate()
        .unwrap_or_else(|error| panic!("{}: changed-graph validation: {error}", case.name));

    // The oracle has to be the CHANGED graph, so prove the snapshot really is one:
    // an owned freeze of the same mutation must produce the identical result set.
    // Without this the differential could be comparing a graph against itself.
    let owned = prepared
        .bind_dataset(
            &mutation
                .freeze()
                .unwrap_or_else(|error| panic!("{}: freeze: {error}", case.name)),
        )
        .and_then(|validator| validator.validate())
        .unwrap_or_else(|error| panic!("{}: owned validation: {error}", case.name));
    assert_eq!(
        after.result_tuples(),
        owned.result_tuples(),
        "{}: the mutation snapshot and its owned freeze must validate identically",
        case.name
    );

    let moved: BTreeSet<String> = before
        .result_tuples()
        .symmetric_difference(&after.result_tuples())
        .map(|tuple| tuple.0.clone())
        .collect();
    assert!(
        !moved.is_empty(),
        "{}: the delta moved no verdict, so a passing superset assertion would prove \
         nothing about it",
        case.name
    );

    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .unwrap_or_else(|error| panic!("{}: expansion: {error}", case.name));
    let ids = expansion.ids().unwrap_or_else(|| {
        panic!(
            "{}: this shapes graph is readable, so the expansion must be bounded, not TOP ({:?})",
            case.name,
            expansion.reason()
        )
    });
    let ids: BTreeSet<TermId> = ids.iter().copied().collect();

    // ── The superset law ────────────────────────────────────────────────────────
    let terms = focus_terms([&before, &after]);
    let mut moved_ids: BTreeSet<TermId> = BTreeSet::new();
    for focus in &moved {
        let term = terms
            .get(focus)
            .unwrap_or_else(|| panic!("{}: no result names focus {focus}", case.name));
        let id = validator.term_id(term).unwrap_or_else(|| {
            panic!(
                "{}: focus {focus} moved a verdict in the changed graph but that graph does \
                 not intern it",
                case.name
            )
        });
        assert!(
            ids.contains(&id),
            "{}: focus {focus} moved a verdict the expansion does not cover — a silent drop: \
             re-validating the expansion would report conformance for a node nobody checked",
            case.name
        );
        moved_ids.insert(id);
    }

    // ── Not vacuous: the expansion is not the whole graph ───────────────────────
    let untouched = ex_term(case.untouched);
    let untouched_id = validator.term_id(&untouched).unwrap_or_else(|| {
        panic!(
            "{}: the untouched control node ex:{} is not in the data graph",
            case.name, case.untouched
        )
    });
    assert!(
        !ids.contains(&untouched_id),
        "{}: ex:{} cannot be reached by this change, so an expansion that includes it is \
         answering \"everything\" while claiming to be bounded",
        case.name,
        case.untouched
    );

    // ── Not vacuous: the naive expansion really would have missed ───────────────
    if case.naive_misses {
        let naive: BTreeSet<TermId> = case
            .inserts
            .iter()
            .chain(case.removals)
            .filter_map(|row| validator.term_id(&ex_term(row.0)))
            .collect();
        assert!(
            moved_ids.iter().any(|id| !naive.contains(id)),
            "{}: this case is declared to defeat the subjects-of-delta expansion, but every \
             moved focus node IS a changed subject — so passing it is no evidence of a \
             dependency analysis",
            case.name
        );
    }
}

// ── The fixtures ────────────────────────────────────────────────────────────────

/// `sh:targetClass` + a plain forward predicate path.
const FORWARD_PREDICATE: Case = Case {
    name: "forward predicate path under sh:targetClass",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    ),
    inserts: &[],
    removals: &[("alice", "http://example.org/ns#name", Obj::Lit("Alice"))],
    untouched: "bob",
    naive_misses: false,
};

/// A verdict moving the OTHER way: an insert RETRACTS an existing violation.
///
/// An expansion derived from "what is newly broken" rather than from what the
/// shapes graph READS would pass every other case here and miss this one, leaving a
/// stale violation in a caller's incrementally maintained report forever.
const RETRACTED_VIOLATION: Case = Case {
    name: "an insert retracts a violation through an inverse path",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path [ sh:inversePath ex:parent ] ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#carl> <http://example.org/ns#parent> <http://example.org/ns#alice> .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
    ),
    inserts: &[("dana", "http://example.org/ns#parent", Obj::Ex("bob"))],
    removals: &[],
    untouched: "alice",
    naive_misses: true,
};

/// `sh:inversePath`: the moved focus node is the OBJECT of the changed row.
const INVERSE_PATH: Case = Case {
    name: "sh:inversePath",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path [ sh:inversePath ex:parent ] ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#carl> <http://example.org/ns#parent> <http://example.org/ns#alice> .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#dana> <http://example.org/ns#parent> <http://example.org/ns#bob> .\n",
    ),
    inserts: &[],
    removals: &[("carl", "http://example.org/ns#parent", Obj::Ex("alice"))],
    untouched: "bob",
    naive_misses: true,
};

/// A sequence path: the changed row sits at the LAST step, one hop from the focus.
const SEQUENCE_PATH: Case = Case {
    name: "sequence path",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ( ex:address ex:city ) ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#address> <http://example.org/ns#addr1> .\n",
        "<http://example.org/ns#addr1> <http://example.org/ns#city> \"Ottawa\" .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#address> <http://example.org/ns#addr2> .\n",
        "<http://example.org/ns#addr2> <http://example.org/ns#city> \"Halifax\" .\n",
    ),
    inserts: &[],
    removals: &[("addr1", "http://example.org/ns#city", Obj::Lit("Ottawa"))],
    untouched: "bob",
    naive_misses: true,
};

/// An alternative path with an inverse branch.
const ALTERNATIVE_PATH: Case = Case {
    name: "alternative path with an inverse branch",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
        sh:path [ sh:alternativePath ( ex:name [ sh:inversePath ex:nameOf ] ) ] ;
        sh:minCount 1 ;
    ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#label1> <http://example.org/ns#nameOf> <http://example.org/ns#alice> .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    ),
    inserts: &[],
    removals: &[("label1", "http://example.org/ns#nameOf", Obj::Ex("alice"))],
    untouched: "bob",
    naive_misses: true,
};

/// `sh:oneOrMorePath`: the changed row is an unbounded number of hops away.
const CLOSURE_PATH: Case = Case {
    name: "sh:oneOrMorePath closure",
    shapes: r"
ex:ChainShape a sh:NodeShape ;
    sh:targetClass ex:Chain ;
    sh:property [ sh:path [ sh:oneOrMorePath ex:next ] ; sh:nodeKind sh:IRI ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Chain> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#next> <http://example.org/ns#b> .\n",
        "<http://example.org/ns#b> <http://example.org/ns#next> <http://example.org/ns#c> .\n",
        "<http://example.org/ns#dave> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Chain> .\n",
        "<http://example.org/ns#dave> <http://example.org/ns#next> <http://example.org/ns#e> .\n",
    ),
    inserts: &[("c", "http://example.org/ns#next", Obj::Lit("not an IRI"))],
    removals: &[],
    untouched: "dave",
    naive_misses: true,
};

/// `sh:targetSubjectsOf`: the change creates a new target.
const TARGET_SUBJECTS_OF: Case = Case {
    name: "sh:targetSubjectsOf",
    shapes: r"
ex:EmployedShape a sh:NodeShape ;
    sh:targetSubjectsOf ex:worksFor ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#worksFor> <http://example.org/ns#acme> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
    ),
    inserts: &[("mallory", "http://example.org/ns#worksFor", Obj::Ex("acme"))],
    removals: &[],
    untouched: "alice",
    naive_misses: false,
};

/// `sh:targetObjectsOf`: the new target is the OBJECT of the changed row.
const TARGET_OBJECTS_OF: Case = Case {
    name: "sh:targetObjectsOf",
    shapes: r"
ex:EmployerShape a sh:NodeShape ;
    sh:targetObjectsOf ex:worksFor ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#worksFor> <http://example.org/ns#acme> .\n",
        "<http://example.org/ns#acme> <http://example.org/ns#name> \"Acme\" .\n",
        "<http://example.org/ns#zed> <http://example.org/ns#name> \"Zed\" .\n",
    ),
    inserts: &[(
        "alice",
        "http://example.org/ns#worksFor",
        Obj::Ex("initech"),
    )],
    removals: &[],
    untouched: "acme",
    naive_misses: true,
};

/// `sh:targetClass` reached through the `rdf:type` edge.
const TARGET_CLASS_TYPE_EDGE: Case = Case {
    name: "sh:targetClass through the rdf:type edge",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
        "<http://example.org/ns#mallory> <http://example.org/ns#age> \"41\" .\n",
    ),
    inserts: &[("mallory", RDF_TYPE, Obj::Ex("Person"))],
    removals: &[],
    untouched: "alice",
    naive_misses: false,
};

/// `sh:targetClass` moved by an `rdfs:subClassOf` edge: the changed row's subject is
/// a CLASS, and every affected focus node is two reversed hops away from it.
const TARGET_CLASS_SUBCLASS_EDGE: Case = Case {
    name: "sh:targetClass through an rdfs:subClassOf edge",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
        "<http://example.org/ns#mallory> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n",
        "<http://example.org/ns#mallory> <http://example.org/ns#age> \"41\" .\n",
    ),
    inserts: &[("Employee", RDFS_SUB_CLASS_OF, Obj::Ex("Person"))],
    removals: &[],
    untouched: "alice",
    naive_misses: true,
};

/// `sh:node` recursion: the constraint that moves is one level down.
const NODE_RECURSION: Case = Case {
    name: "sh:node recursion",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:address ; sh:node ex:AddressShape ] .

ex:AddressShape a sh:NodeShape ;
    sh:property [ sh:path ex:city ; sh:minCount 1 ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#address> <http://example.org/ns#addr1> .\n",
        "<http://example.org/ns#addr1> <http://example.org/ns#city> \"Ottawa\" .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#address> <http://example.org/ns#addr2> .\n",
        "<http://example.org/ns#addr2> <http://example.org/ns#city> \"Halifax\" .\n",
    ),
    inserts: &[],
    removals: &[("addr1", "http://example.org/ns#city", Obj::Lit("Ottawa"))],
    untouched: "bob",
    naive_misses: true,
};

/// A property-pair comparand, reached through an `sh:node` so that the comparand
/// predicate is read at a node the focus node does not equal.
const PROPERTY_PAIR_COMPARAND: Case = Case {
    name: "sh:equals comparand under sh:node",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:address ; sh:node ex:AddressShape ] .

ex:AddressShape a sh:NodeShape ;
    sh:property [ sh:path ex:city ; sh:equals ex:town ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#address> <http://example.org/ns#addr1> .\n",
        "<http://example.org/ns#addr1> <http://example.org/ns#city> \"Ottawa\" .\n",
        "<http://example.org/ns#addr1> <http://example.org/ns#town> \"Ottawa\" .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#address> <http://example.org/ns#addr2> .\n",
        "<http://example.org/ns#addr2> <http://example.org/ns#city> \"Halifax\" .\n",
        "<http://example.org/ns#addr2> <http://example.org/ns#town> \"Halifax\" .\n",
    ),
    inserts: &[("addr1", "http://example.org/ns#town", Obj::Lit("Gatineau"))],
    removals: &[],
    untouched: "bob",
    naive_misses: true,
};

/// `sh:class` on a value node: the moved verdict follows the VALUE node's type edge.
const CLASS_CONSTRAINT: Case = Case {
    name: "sh:class on a value node",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:knows ; sh:class ex:Person ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#carl> .\n",
        "<http://example.org/ns#carl> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#knows> <http://example.org/ns#dana> .\n",
        "<http://example.org/ns#dana> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
    ),
    inserts: &[],
    removals: &[("carl", RDF_TYPE, Obj::Ex("Person"))],
    untouched: "bob",
    naive_misses: true,
};

/// `sh:closed`: a read that binds no predicate at all, and is bounded anyway.
const CLOSED_SHAPE: Case = Case {
    name: "sh:closed",
    shapes: r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:closed true ;
    sh:ignoredProperties ( rdfs:label ) ;
    sh:property [ sh:path ex:name ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
        "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    ),
    inserts: &[("alice", "http://example.org/ns#nickname", Obj::Lit("Al"))],
    removals: &[],
    untouched: "bob",
    naive_misses: false,
};

/// Every case, so one failure names the form it belongs to.
const CASES: &[&Case] = &[
    &FORWARD_PREDICATE,
    &RETRACTED_VIOLATION,
    &INVERSE_PATH,
    &SEQUENCE_PATH,
    &ALTERNATIVE_PATH,
    &CLOSURE_PATH,
    &TARGET_SUBJECTS_OF,
    &TARGET_OBJECTS_OF,
    &TARGET_CLASS_TYPE_EDGE,
    &TARGET_CLASS_SUBCLASS_EDGE,
    &NODE_RECURSION,
    &PROPERTY_PAIR_COMPARAND,
    &CLASS_CONSTRAINT,
    &CLOSED_SHAPE,
];

#[test]
fn the_expansion_is_a_superset_of_every_verdict_the_change_moves() {
    for case in CASES {
        assert_expansion_covers_every_moved_verdict(case);
    }
}

// ── The over-refusal pair ───────────────────────────────────────────────────────

/// Build a one-insert snapshot over `data` and bind `shapes` to it.
fn bound(
    shapes: &str,
    data: &str,
    row: Row,
) -> (Arc<purrdf::ir::DeltaDatasetView>, PreparedValidator) {
    let parsed = Arc::new(
        parse_shapes(&format!("{PREFIXES}{shapes}"), None).expect("fixture shapes must parse"),
    );
    let prepared = PreparedShapes::new(parsed);
    let base = parse_ntriples_to_dataset(data).expect("fixture data must parse");
    let mut mutation = MutableDataset::new(base);
    assert!(mutation.insert(quad(row)).expect("insert"));
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
    let validator = prepared
        .bind_delta_with_shapes_graph(Arc::clone(&snapshot), None, ViewLimits::default())
        .expect("delta bind");
    (snapshot, validator)
}

const TYPED_ALICE: &str = concat!(
    "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
    "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
);

/// HALF ONE of the over-refusal pair: a shapes graph this walk can read in full must
/// produce a BOUNDED expansion.
///
/// An implementation that answered TOP here would satisfy every superset assertion
/// above and would still have destroyed the feature.
#[test]
fn a_shapes_graph_of_forward_paths_stays_bounded() {
    let (snapshot, validator) = bound(
        r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:maxCount 1 ] ;
    sh:property [ sh:path ex:age ; sh:datatype <http://www.w3.org/2001/XMLSchema#string> ] .
",
        TYPED_ALICE,
        ("mallory", RDF_TYPE, Obj::Ex("Person")),
    );
    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("expansion");
    assert!(
        !expansion.is_everything(),
        "forward predicate paths are fully readable, so answering TOP is over-refusal: {:?}",
        expansion.reason()
    );
    let ids = expansion.ids().expect("bounded");
    let mallory = validator
        .term_id(&ex_term("mallory"))
        .expect("the inserted subject is interned");
    assert!(ids.contains(&mallory), "the new target must be expanded");
    let alice = validator
        .term_id(&ex_term("alice"))
        .expect("the untouched focus node is interned");
    assert!(
        !ids.contains(&alice),
        "ex:alice cannot be reached by this change; including it would make the answer TOP \
         under another name"
    );
}

/// HALF TWO of the over-refusal pair: a single `sh:sparql` constraint makes the
/// footprint unreadable, and that must be REPORTED rather than silently
/// under-approximated.
#[test]
fn a_sparql_constraint_reports_an_unreadable_footprint() {
    let (snapshot, validator) = bound(
        r#"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "every person needs a name" ;
        sh:select "SELECT $this WHERE { FILTER NOT EXISTS { $this <http://example.org/ns#name> ?n } }" ;
    ] .
"#,
        TYPED_ALICE,
        ("mallory", RDF_TYPE, Obj::Ex("Person")),
    );
    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("expansion");
    assert!(
        expansion.is_everything(),
        "a SHACL-SPARQL constraint reads through query text this walk does not parse, so no \
         bounded superset can be derived from the shapes graph"
    );
    assert!(
        expansion.ids().is_none(),
        "TOP must not be handed out as a list of ids: an empty list and \"every node\" are \
         opposite instructions"
    );
    let reason = expansion.reason().expect("TOP carries its reason");
    assert!(
        reason.contains("query text"),
        "the reason must name the construct a caller has to change, got {reason:?}"
    );
}

// ── Identity of the change set ──────────────────────────────────────────────────

/// A [`TermId`] is an index, so ids derived from ANOTHER snapshot's changes are
/// valid indices into this binding's table and would silently name the wrong nodes.
/// The pairing is checked rather than trusted.
#[test]
fn a_foreign_snapshot_is_refused() {
    let (_own, validator) = bound(
        r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
        TYPED_ALICE,
        ("mallory", RDF_TYPE, Obj::Ex("Person")),
    );
    let (foreign, _foreign_validator) = bound(
        r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
",
        TYPED_ALICE,
        ("trudy", RDF_TYPE, Obj::Ex("Person")),
    );
    let error = validator
        .affected_focus_node_ids(&foreign)
        .expect_err("a snapshot this validator does not read must be refused");
    assert!(
        error.contains("not the one this validator was bound to"),
        "the refusal must say WHICH pairing failed, got {error:?}"
    );
}

/// A validator over an ordinary frozen dataset has no change to expand, and says so
/// instead of answering with an empty set that a caller would read as "nothing
/// moved".
#[test]
fn a_binding_without_a_snapshot_has_no_change_to_expand() {
    let shapes = Arc::new(
        parse_shapes(
            &format!(
                "{PREFIXES}{}",
                r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"
            ),
            None,
        )
        .expect("shapes"),
    );
    let prepared = PreparedShapes::new(shapes);
    let base = parse_ntriples_to_dataset(TYPED_ALICE).expect("data");
    let mut mutation = MutableDataset::new(Arc::clone(&base));
    assert!(
        mutation
            .insert(quad(("mallory", RDF_TYPE, Obj::Ex("Person"))))
            .expect("insert")
    );
    let snapshot = mutation.snapshot_view().expect("snapshot");
    let frozen = prepared.bind_dataset(&base).expect("frozen bind");
    let error = frozen
        .affected_focus_node_ids(&snapshot)
        .expect_err("a frozen binding carries no change");
    assert!(
        error.contains("not bound to a mutation snapshot"),
        "the refusal must name the missing precondition, got {error:?}"
    );
}

/// The expansion is a value a caller may log, compare or pin, so it is canonically
/// ordered and free of repeats — two facts a first-seen accumulation does not have.
#[test]
fn the_expansion_is_ordered_and_deduplicated() {
    let (snapshot, validator) = bound(
        r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
    sh:property [ sh:path ex:name ; sh:maxCount 1 ] .
",
        TYPED_ALICE,
        ("mallory", "http://example.org/ns#name", Obj::Lit("Mallory")),
    );
    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("expansion");
    let ids = expansion.ids().expect("bounded");
    assert!(
        ids.windows(2).all(|pair| pair[0].index() < pair[1].index()),
        "the expansion must be strictly ascending, got {ids:?}"
    );
}
