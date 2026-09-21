// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! The expansion's precondition on the READ SURFACE carries its own pair, for the
//! same reason: [`expanding_a_change_on_an_unprojected_delta_binding_is_refused`]
//! is the refusal, and
//! [`expanding_a_change_on_a_projected_delta_binding_still_answers`] is the
//! neighbouring case a guard that refused every delta binding would break.
//!
//! Fixture IRIs are under `example.org`: PurRDF mints no vocabulary IRIs.

#![allow(clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf::ir::ViewLimits;
use purrdf::{DatasetMut, MutableDataset, QuadValues, TermId, TermValue};
use purrdf_shapes::engine::{FocusId, PreparedShapes, PreparedValidator, parse_shapes};
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
/// The RDF 1.2 reifier predicate. A row `(r, rdf:reifies, <<( s p o )>>)` is a
/// REIFIER DECLARATION, not an ordinary quad: it lands in the statement side-table
/// and reclassifies the rows about `r` in the graph that declared it.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// One object position of a fixture row.
#[derive(Clone, Copy, Debug)]
enum Obj {
    /// An `ex:`-prefixed IRI.
    Ex(&'static str),
    /// A plain literal.
    Lit(&'static str),
    /// A triple term `<<( ex:s <p> ex:o )>>`, by `(subject local name, predicate
    /// IRI, object local name)` — the object a reifier declaration takes.
    Triple(&'static str, &'static str, &'static str),
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
    /// Rows folded into the BASE before it freezes, for the RDF 1.2 statement
    /// layer that N-Triples text cannot spell: a `rdf:reifies` row becomes a
    /// reifier declaration, and a row about a declared reifier becomes that
    /// reifier's annotation. Empty for a case with no overlay, which then parses
    /// its base exactly as before.
    base_overlay: &'static [Row],
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
        Obj::Triple(s, p, o) => TermValue::Triple {
            s: Box::new(TermValue::iri(ex(s))),
            p: Box::new(TermValue::iri(p)),
            o: Box::new(TermValue::iri(ex(o))),
        },
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

/// Fold [`Case::base_overlay`] into the parsed base, so the case's BASE — not its
/// delta — already carries an RDF 1.2 statement layer.
///
/// Inserting the rows into a mutation and freezing it is the same classifier the
/// delta path uses, so the reifier declaration lands in the reifier side-table and
/// a row about that reifier lands in the annotation side-table. A case with no
/// overlay is handed back its parsed base untouched, never a re-frozen copy of it.
fn fold_overlay_into_base(case: &Case, base: Arc<purrdf::RdfDataset>) -> Arc<purrdf::RdfDataset> {
    if case.base_overlay.is_empty() {
        return base;
    }
    let mut mutation = MutableDataset::new(base);
    for &row in case.base_overlay {
        assert!(
            mutation
                .insert(quad(row))
                .unwrap_or_else(|error| panic!("{}: base overlay insert: {error}", case.name)),
            "{}: base overlay row {row:?} changed nothing",
            case.name
        );
    }
    mutation
        .freeze()
        .unwrap_or_else(|error| panic!("{}: base overlay freeze: {error}", case.name))
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
    let base = fold_overlay_into_base(case, base);

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
    // Compared in the bare id space: every id of one expansion carries the same
    // binding stamp, so the stamp is not part of what distinguishes them.
    let ids: BTreeSet<TermId> = ids.iter().copied().map(FocusId::term_id).collect();

    // ── The superset law ────────────────────────────────────────────────────────
    let terms = focus_terms([&before, &after]);
    let mut moved_ids: BTreeSet<TermId> = BTreeSet::new();
    for focus in &moved {
        let term = terms
            .get(focus)
            .unwrap_or_else(|| panic!("{}: no result names focus {focus}", case.name));
        let id = validator
            .term_id(term)
            .unwrap_or_else(|| {
                panic!(
                    "{}: focus {focus} moved a verdict in the changed graph but that graph does \
                     not intern it",
                    case.name
                )
            })
            .term_id();
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
    let untouched_id = validator
        .term_id(&untouched)
        .unwrap_or_else(|| {
            panic!(
                "{}: the untouched control node ex:{} is not in the data graph",
                case.name, case.untouched
            )
        })
        .term_id();
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
            .filter_map(|row| validator.term_id(&ex_term(row.0)).map(FocusId::term_id))
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
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
    base_overlay: &[],
    inserts: &[("alice", "http://example.org/ns#nickname", Obj::Lit("Al"))],
    removals: &[],
    untouched: "bob",
    naive_misses: false,
};

// ── The RDF 1.2 statement layer ─────────────────────────────────────────────────
//
// A delta may change the statement layer instead of the plain one, and those rows
// live in side-tables of their own: a `rdf:reifies` declaration and a row about a
// declared reifier never reach the delta's plain quad table at all. A change set
// assembled from that one table would return nothing for any of the three cases
// below while the graph plainly changed, so each drives the same differential
// oracle over a delta that only touches the overlay.

/// A REIFIER DECLARATION added by the delta, read back as an `rdf:reifies` row.
const REIFIER_DECLARATION_ADDED: Case = Case {
    name: "an added rdf:reifies declaration",
    shapes: r"
ex:ReifiedShape a sh:NodeShape ;
    sh:targetNode ex:alice, ex:bob ;
    sh:property [
        sh:path <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ;
        sh:minCount 1 ;
    ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    ),
    base_overlay: &[],
    inserts: &[(
        "alice",
        RDF_REIFIES,
        Obj::Triple("carl", "http://example.org/ns#parent", "dana"),
    )],
    removals: &[],
    untouched: "bob",
    naive_misses: false,
};

/// The same declaration SUPPRESSED, which is the direction a change set assembled
/// from added rows alone gets wrong.
const REIFIER_DECLARATION_SUPPRESSED: Case = Case {
    name: "a suppressed rdf:reifies declaration",
    shapes: r"
ex:ReifiedShape a sh:NodeShape ;
    sh:targetNode ex:stmt1, ex:bob ;
    sh:property [
        sh:path <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ;
        sh:minCount 1 ;
    ] .
",
    data: "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    base_overlay: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("carl", "http://example.org/ns#parent", "dana"),
    )],
    inserts: &[],
    removals: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("carl", "http://example.org/ns#parent", "dana"),
    )],
    untouched: "bob",
    naive_misses: false,
};

/// A row added ABOUT a declared reifier. The delta admits it as that reifier's
/// ANNOTATION — a third table again — and SHACL reads it at `ex:stmt1` all the
/// same, so the change set has to carry it under the subject it was written with.
const ANNOTATION_ADDED_ON_A_DECLARED_REIFIER: Case = Case {
    name: "an annotation added on a declared reifier",
    shapes: r"
ex:ConfidenceShape a sh:NodeShape ;
    sh:targetNode ex:stmt1, ex:bob ;
    sh:property [ sh:path ex:confidence ; sh:minCount 1 ] .
",
    data: "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    base_overlay: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("carl", "http://example.org/ns#parent", "dana"),
    )],
    inserts: &[("stmt1", "http://example.org/ns#confidence", Obj::Lit("0.9"))],
    removals: &[],
    untouched: "bob",
    naive_misses: false,
};

/// `sh:reificationRequired` with NO `sh:reifierShape`.
///
/// The verdict turns on whether a reifier EXISTS for the value triple
/// `<<( ex:alice ex:parent ex:carl )>>`, and the row that supplies one is an
/// `rdf:reifies` declaration whose subject is the reifier and whose object is the
/// triple term — so neither the focus node nor the value node appears anywhere in
/// the changed row. Nothing on the property path changed, and yet `ex:alice` goes
/// from violating to conforming.
const REIFICATION_REQUIRED: Case = Case {
    name: "sh:reificationRequired with no reifier shape",
    shapes: r"
ex:ParentageShape a sh:NodeShape ;
    sh:targetNode ex:alice, ex:bob ;
    sh:property [ sh:path ex:parent ; sh:reificationRequired true ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#parent> <http://example.org/ns#carl> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#parent> <http://example.org/ns#dana> .\n",
    ),
    base_overlay: &[],
    inserts: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("alice", "http://example.org/ns#parent", "carl"),
    )],
    removals: &[],
    untouched: "bob",
    naive_misses: true,
};

/// The mirror direction: the base CONFORMS because a reifier is declared, and the
/// delta suppresses that declaration, so `ex:alice` starts violating.
const REIFICATION_REQUIRED_SUPPRESSED: Case = Case {
    name: "sh:reificationRequired loses its reifier",
    shapes: r"
ex:ParentageShape a sh:NodeShape ;
    sh:targetNode ex:alice, ex:bob ;
    sh:property [ sh:path ex:parent ; sh:reificationRequired true ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#parent> <http://example.org/ns#carl> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#parent> <http://example.org/ns#dana> .\n",
    ),
    base_overlay: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("alice", "http://example.org/ns#parent", "carl"),
    )],
    inserts: &[],
    removals: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("alice", "http://example.org/ns#parent", "carl"),
    )],
    untouched: "bob",
    naive_misses: true,
};

/// The other half of the same gate: `sh:reifierShape` with no
/// `sh:reificationRequired`.
///
/// The reifier shape here reads no triple of its own — `sh:in` judges the reifier's
/// own identity — so the footprint stays bounded and the only thing standing between
/// the delta and a silent drop is the EXISTENCE read. Declaring `ex:stmt1` as a
/// reifier submits a brand-new node to that shape, which rejects it.
const REIFIER_SHAPE_GAINS_A_REIFIER: Case = Case {
    name: "sh:reifierShape gains a reifier to judge",
    shapes: r"
ex:ParentageShape a sh:NodeShape ;
    sh:targetNode ex:alice, ex:bob ;
    sh:property [ sh:path ex:parent ; sh:reifierShape [ sh:in ( ex:approved ) ] ] .
",
    data: concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#parent> <http://example.org/ns#carl> .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#parent> <http://example.org/ns#dana> .\n",
    ),
    base_overlay: &[],
    inserts: &[(
        "stmt1",
        RDF_REIFIES,
        Obj::Triple("alice", "http://example.org/ns#parent", "carl"),
    )],
    removals: &[],
    untouched: "bob",
    naive_misses: true,
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

// The overlay cases run as tests of their own rather than as three more entries in
// `CASES`: they are three independent tables of one layer, and a loop stops at the
// first panic, which would hide the other two behind whichever fails first.

#[test]
fn an_added_reifier_declaration_is_in_the_change_set() {
    assert_expansion_covers_every_moved_verdict(&REIFIER_DECLARATION_ADDED);
}

#[test]
fn a_suppressed_reifier_declaration_is_in_the_change_set() {
    assert_expansion_covers_every_moved_verdict(&REIFIER_DECLARATION_SUPPRESSED);
}

#[test]
fn an_added_annotation_is_in_the_change_set_under_its_own_subject() {
    assert_expansion_covers_every_moved_verdict(&ANNOTATION_ADDED_ON_A_DECLARED_REIFIER);
}

#[test]
fn a_reifier_gained_for_a_required_reification_expands_the_focus_node() {
    assert_expansion_covers_every_moved_verdict(&REIFICATION_REQUIRED);
}

#[test]
fn a_reifier_lost_for_a_required_reification_expands_the_focus_node() {
    assert_expansion_covers_every_moved_verdict(&REIFICATION_REQUIRED_SUPPRESSED);
}

#[test]
fn a_reifier_gained_under_a_reifier_shape_expands_the_focus_node() {
    assert_expansion_covers_every_moved_verdict(&REIFIER_SHAPE_GAINS_A_REIFIER);
}

/// The over-refusal half of the reification read: the SAME data and the SAME delta,
/// under a property shape that declares neither `sh:reifierShape` nor
/// `sh:reificationRequired`.
///
/// Such a shape never asks the statement layer anything, so a reifier declaration
/// cannot move its verdict and `ex:alice` must NOT be expanded. Emitting the
/// `rdf:reifies` trigger unconditionally would satisfy every superset assertion above
/// while re-validating a node for a change it cannot see — the mirror bug, and
/// invisible without this control.
#[test]
fn a_property_shape_without_reification_ignores_a_reifier_declaration() {
    let (snapshot, validator) = bound(
        r"
ex:ParentageShape a sh:NodeShape ;
    sh:targetNode ex:alice, ex:bob ;
    sh:property [ sh:path ex:parent ; sh:minCount 1 ] .
",
        concat!(
            "<http://example.org/ns#alice> <http://example.org/ns#parent> <http://example.org/ns#carl> .\n",
            "<http://example.org/ns#bob> <http://example.org/ns#parent> <http://example.org/ns#dana> .\n",
        ),
        (
            "stmt1",
            RDF_REIFIES,
            Obj::Triple("alice", "http://example.org/ns#parent", "carl"),
        ),
    );
    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("expansion");
    assert!(
        !expansion.is_everything(),
        "a property shape with no reification clause reads nothing this walk cannot bound, so \
         answering TOP is over-refusal: {:?}",
        expansion.reason()
    );
    let ids = expansion.ids().expect("bounded");
    let alice = validator
        .term_id(&ex_term("alice"))
        .expect("the base interned ex:alice");
    assert!(
        !ids.contains(&alice),
        "this shapes graph never asks whether ex:alice's value triple has a reifier, so a \
         reifier declaration must not expand it"
    );
}

/// WHY the change set may omit a row the overlay RECLASSIFIES — and the two facts
/// that have to stay true for that to remain sound.
///
/// Declaring a reifier for `ex:alice` demotes every row about `ex:alice` out of the
/// snapshot's plain table and into its annotation table. That row is in no added
/// and no suppressed table, so `changed_quads` does not name it, and a reader of
/// `DeltaDatasetView::quads` alone would see a statement disappear that no change
/// set mentioned.
///
/// SHACL is not that reader. Its data view projects the RDF 1.2 statement layer
/// onto the plain stream, so a demoted row is still read at the same subject with
/// the same predicate: it changed tables, it did not leave the graph, and no verdict
/// can move because of it. This test asserts BOTH halves — the demotion really
/// happens, and validation really is unmoved by it — because either half alone is
/// consistent with a silent drop.
///
/// The statement projection is therefore load-bearing, and it is ENFORCED rather
/// than assumed: `ShaclDatasetView::delta` takes the projection as a public
/// argument, so an unprojected delta binding is constructible through
/// `PreparedShapes::bind_view`, and `affected_focus_node_ids` refuses exactly that
/// binding — see
/// `expanding_a_change_on_an_unprojected_delta_binding_is_refused`. If that guard
/// is ever removed, this seam must be reopened: the change set would then owe its
/// consumer the demoted row itself.
#[test]
fn a_reclassifying_delta_moves_no_verdict_because_statements_are_projected() {
    use purrdf::prelude::{DatasetView, GraphMatch};

    let shapes = r"
ex:PersonShape a sh:NodeShape ;
    sh:targetNode ex:alice, ex:bob ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
";
    let parsed = Arc::new(
        parse_shapes(&format!("{PREFIXES}{shapes}"), None).expect("fixture shapes must parse"),
    );
    let prepared = PreparedShapes::new(parsed);
    let base = parse_ntriples_to_dataset(concat!(
        "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n",
        "<http://example.org/ns#bob> <http://example.org/ns#name> \"Bob\" .\n",
    ))
    .expect("fixture data must parse");
    let before = prepared
        .bind_dataset(&base)
        .and_then(|validator| validator.validate())
        .expect("base validation");
    assert!(
        before.conforms,
        "both targets have a name in the base, so the base must conform for this test to be \
         about the delta at all"
    );

    let mut mutation = MutableDataset::new(Arc::clone(&base));
    assert!(
        mutation
            .insert(quad((
                "alice",
                RDF_REIFIES,
                Obj::Triple("carl", "http://example.org/ns#parent", "dana"),
            )))
            .expect("insert")
    );
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));

    // Half one: the demotion is real. The row is gone from the plain table…
    let alice = snapshot
        .term_id_by_value(&TermValue::iri(ex("alice")))
        .expect("the base interned ex:alice");
    let name = snapshot
        .term_id_by_value(&TermValue::iri(ex("name")))
        .expect("the base interned ex:name");
    assert_eq!(
        snapshot
            .quads_for_pattern(Some(alice), Some(name), None, GraphMatch::Any)
            .count(),
        0,
        "declaring a reifier for ex:alice must demote the rows about it out of the plain \
         table, or this test is not exercising reclassification at all"
    );
    // …and it is on the annotation table instead, not lost.
    assert!(
        snapshot
            .annotations_of_with_graph(alice)
            .any(|(p, _, _)| p == name),
        "the demoted row must be readable as an annotation of ex:alice"
    );

    // Half two: SHACL is unmoved, because it reads the projection of both tables.
    let validator = prepared
        .bind_delta_with_shapes_graph(Arc::clone(&snapshot), None, ViewLimits::default())
        .expect("delta bind");
    let after = validator.validate().expect("changed-graph validation");
    assert_eq!(
        before.result_tuples(),
        after.result_tuples(),
        "a row that only changed tables must move no verdict; if it can, the change set owes \
         its consumer that row and affected_focus_node_ids is under-approximating"
    );
    let owned = prepared
        .bind_dataset(&mutation.freeze().expect("freeze"))
        .and_then(|validator| validator.validate())
        .expect("owned validation");
    assert_eq!(
        after.result_tuples(),
        owned.result_tuples(),
        "the snapshot and its owned freeze must validate identically"
    );
    assert!(
        !validator
            .affected_focus_node_ids(&snapshot)
            .expect("expansion")
            .is_everything(),
        "an overlay-only delta is fully readable, so answering TOP would be over-refusal"
    );
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

// ── The read surface the change set is stated over ──────────────────────────────

/// The shapes graph both halves of the projection pair are bound against.
const PROJECTION_PAIR_SHAPES: &str = r"
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
";

/// A base, a one-row mutation that gains `ex:mallory` a violation, and the shapes
/// both halves of the projection pair share. The snapshot is the SAME value in both
/// halves, so the only thing that differs between them is the view mode.
fn projection_pair_fixture() -> (PreparedShapes, Arc<purrdf::ir::DeltaDatasetView>) {
    let parsed = Arc::new(
        parse_shapes(&format!("{PREFIXES}{PROJECTION_PAIR_SHAPES}"), None)
            .expect("fixture shapes must parse"),
    );
    let base = parse_ntriples_to_dataset(TYPED_ALICE).expect("fixture data must parse");
    let mut mutation = MutableDataset::new(base);
    assert!(
        mutation
            .insert(quad(("mallory", RDF_TYPE, Obj::Ex("Person"))))
            .expect("insert")
    );
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
    (PreparedShapes::new(parsed), snapshot)
}

/// REFUSED: a delta binding that does not project the RDF 1.2 statement layer
/// cannot be expanded, because the change set is not stated over the surface it
/// reads.
///
/// `ShaclDatasetView::delta` takes the projection as a public argument and
/// `PreparedShapes::bind_view` applies none of its own, so composing the two public
/// functions yields a delta-backed validator reading the plain table ALONE. The
/// change set names every row that joins or leaves the union of the plain and both
/// statement tables; it deliberately does NOT name a row the overlay merely
/// reclassifies, because a reclassified row never leaves that union. Read the plain
/// table alone and a demotion IS a disappearance, so the expansion would be short by
/// exactly the focus nodes that row moves — and a short expansion reports `conforms`
/// about a node nobody re-checked.
///
/// The binding is refused HERE rather than silently under-approximating, and the
/// refusal is scoped to the expansion: this same binding still validates, because
/// validation promises only to check the nodes it is handed under the view's own
/// semantics. That scoping is asserted below, and it is what keeps the narrow view a
/// legitimate mode rather than a broken one.
#[test]
fn expanding_a_change_on_an_unprojected_delta_binding_is_refused() {
    use purrdf_shapes::data_view::ShaclDatasetView;

    let (prepared, snapshot) = projection_pair_fixture();
    let view = Arc::new(
        ShaclDatasetView::delta(Arc::clone(&snapshot), false, ViewLimits::default())
            .expect("an unprojected delta view is a legitimate carrier and must build"),
    );
    // Both directions of the predicate, on the same carrier: it reads the mode it
    // was constructed with rather than answering a constant, which is the only
    // reason a guard written on it means anything.
    assert!(
        !view.statements_projected(),
        "the fixture is only about this defect if the view really reads the narrow surface"
    );
    assert!(
        ShaclDatasetView::delta(Arc::clone(&snapshot), true, ViewLimits::default())
            .expect("a projected delta view over the same snapshot must build")
            .statements_projected(),
        "the same carrier constructed projected must answer the other way, or the predicate is \
         not reporting the view mode at all"
    );
    let validator = prepared
        .bind_view(Arc::clone(&view))
        .expect("binding an unprojected delta view is legal; only EXPANDING it is not");

    let error = validator
        .affected_focus_node_ids(&snapshot)
        .expect_err("a delta binding that reads the plain table alone must be refused");
    assert!(
        error.contains("does not project the RDF 1.2 statement layer"),
        "the refusal must name the precondition that failed, got {error:?}"
    );
    assert!(
        error.contains("bind_delta_with_shapes_graph"),
        "the refusal must name the constructor that satisfies the precondition, got {error:?}"
    );

    // The refusal is the EXPANSION's, not the binding's: the narrow view is still a
    // view, and a guard that took validation down with it would have refused a mode
    // this crate deliberately supports.
    validator
        .validate()
        .expect("an unprojected delta binding still validates under its own read semantics");
}

/// ACCEPTED: the projecting constructor still binds, still expands, and still
/// returns the focus node the change moved.
///
/// The neighbouring case to the refusal above, executed because a guard that refused
/// every delta binding would pass that test and destroy incremental validation. Both
/// halves use the SAME snapshot and the SAME shapes graph, so the only difference
/// between a refusal and an answer is the read surface — which is precisely the
/// distinction the guard claims to draw.
#[test]
fn expanding_a_change_on_a_projected_delta_binding_still_answers() {
    let (prepared, snapshot) = projection_pair_fixture();
    let validator = prepared
        .bind_delta_with_shapes_graph(Arc::clone(&snapshot), None, ViewLimits::default())
        .expect("delta bind");

    // This is also the tripwire on the constructor itself: `bind_delta_with_shapes_graph`
    // builds its core view projected, and if that is ever switched off the guard turns
    // this `expect` into a failure rather than letting the change path go quietly
    // unsound.
    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("the projecting constructor's binding must expand");
    let ids: BTreeSet<TermId> = expansion
        .ids()
        .expect("this shapes graph is readable, so the expansion must be bounded")
        .iter()
        .copied()
        .map(FocusId::term_id)
        .collect();

    // Non-vacuity: the node whose verdict the change actually moved is IN the answer,
    // so the guard is not merely letting an empty set through.
    let mallory = validator
        .term_id(&ex_term("mallory"))
        .expect("the delta interned ex:mallory")
        .term_id();
    assert!(
        ids.contains(&mallory),
        "ex:mallory gained a violation, so the expansion owes the caller that focus node; got \
         {ids:?}"
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
        ids.windows(2)
            .all(|pair| pair[0].term_id().index() < pair[1].term_id().index()),
        "the expansion must be strictly ascending, got {ids:?}"
    );
}
