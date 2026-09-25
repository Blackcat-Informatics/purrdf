// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Under SHACL pre-binding every expression that reads `$this` reads the focus node —
//! above a `GROUP BY`, above a sub-`SELECT` whose projection drops `$this`, and beneath
//! the core — whatever kind of term the focus node is.
//!
//! An IRI or a literal focus node is written into an expression as a constant. A blank
//! node and a quoted triple have no constant form, and used to be left for the `VALUES`
//! seed to bind — which the seed does only where its column reaches. A `GROUP BY`, or a
//! sub-`SELECT` that does not project `$this`, hides the column from every expression
//! above it, so `HAVING(SAMPLE(?x) = $this)` compared against an unbound `$this`: an IRI
//! focus node was reported, and a blank one with the same data conformed.
//!
//! The fixture holds ten focus nodes — a violating and a conforming node of each of five
//! kinds: IRI, blank node, literal, quoted triple, and quoted triple with a blank node
//! inside it. Each owns one status record; the violating ones' statuses start with
//! `bad`. Every body below reports a focus node exactly when its own record is a `bad`
//! one, so the oracle is the same for every kind: the five violating nodes, each held by
//! the term id the dataset gives it — the two blank nodes and the two blank-bearing
//! quoted triples are never merged — each with its own status as the value, and no
//! conforming node. That is what an IRI focus node gets; any kind that reads `$this`
//! unbound anywhere reports a different set.
//!
//! The SHACL surface refuses a sub-`SELECT` that does not project `$this` (SHACL 1.2
//! SPARQL Extensions, Appendix A), so the shapes that need one run through the engine's
//! SHACL pre-binding lane directly, where the same oracle applies.
//!
//! Fixture IRIs are `example.org`; PurRDF mints none.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_core::{SparqlRequest, SparqlResult, TermId, TermValue};
use purrdf_shapes::engine::{parse_shapes, validate_dataset};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions, ShaclPrebinding};

const EX: &str = "http://example.org/";

/// The ten statuses, one per focus node, in the order the fixture writes them.
const STATUSES: [&str; 10] = [
    "bad iri",
    "fine iri",
    "bad blank",
    "fine blank",
    "bad literal",
    "fine literal",
    "bad quoted",
    "fine quoted",
    "bad quoted blank",
    "fine quoted blank",
];

/// The fixture: ten items tagged by `ex:root`, each the subject of one status record.
fn fixture() -> Arc<RdfDataset> {
    let items = [
        format!("<{EX}iri-bad>"),
        format!("<{EX}iri-good>"),
        "_:bad".to_owned(),
        "_:good".to_owned(),
        "\"lit-bad\"".to_owned(),
        "\"lit-good\"".to_owned(),
        format!("<<( <{EX}a> <{EX}r> <{EX}bad> )>>"),
        format!("<<( <{EX}a> <{EX}r> <{EX}good> )>>"),
        format!("<<( <{EX}a> <{EX}r> _:qbad )>>"),
        format!("<<( <{EX}a> <{EX}r> _:qgood )>>"),
    ];
    let mut triples = String::new();
    for (index, (item, status)) in items.iter().zip(STATUSES).enumerate() {
        writeln!(
            triples,
            "<{EX}root> <{EX}tagged> {item} .\n\
             <{EX}rec{index}> <{EX}about> {item} .\n\
             <{EX}rec{index}> <{EX}status> \"{status}\" ."
        )
        .expect("writing to a String cannot fail");
    }
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&triples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

/// Each item's term id, in [`STATUSES`] order, found as the one thing its status record
/// is about — so a blank node is identified by the data it carries, not by its label.
fn items(dataset: &RdfDataset) -> Vec<TermId> {
    let id = |iri: &str| {
        dataset
            .term_id_by_iri(&format!("{EX}{iri}"))
            .unwrap_or_else(|| panic!("{iri} is in the fixture"))
    };
    let (about, status) = (id("about"), id("status"));
    let items: Vec<TermId> = STATUSES
        .iter()
        .map(|literal| {
            let object = dataset
                .term_id_by_value(&TermValue::simple_literal(*literal))
                .unwrap_or_else(|| panic!("the status {literal:?} is in the fixture"));
            let record = dataset
                .quads()
                .find(|quad| quad.p == status && quad.o == object)
                .map(|quad| quad.s)
                .expect("one record owns each status");
            dataset
                .quads()
                .find(|quad| quad.s == record && quad.p == about)
                .map(|quad| quad.o)
                .expect("each record is about one item")
        })
        .collect();
    let distinct: BTreeSet<TermId> = items.iter().copied().collect();
    assert_eq!(distinct.len(), 10, "ten distinct items: {items:?}");
    for (index, kind) in [(2, "blank"), (3, "blank")] {
        assert!(
            matches!(dataset.term_value(items[index]), TermValue::Blank { .. }),
            "item {index} is a {kind} node"
        );
    }
    for index in [8, 9] {
        assert!(
            matches!(
                dataset.term_value(items[index]),
                TermValue::Triple { ref o, .. } if matches!(**o, TermValue::Blank { .. })
            ),
            "item {index} is a quoted triple holding a blank node"
        );
    }
    items
}

/// What every body must report: each violating item, as itself, with its own status.
fn expected(items: &[TermId]) -> BTreeSet<(TermId, String)> {
    STATUSES
        .iter()
        .zip(items)
        .filter(|(status, _)| status.starts_with("bad"))
        .map(|(status, item)| (*item, format!("\"{status}\"")))
        .collect()
}

/// A node shape over every tagged item with `constraint` as its SPARQL-based
/// constraint (`sh:sparql`) or its component declaration (`sh:SPARQLSelectValidator`).
fn shapes_graph(constraint: &str) -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        "\n@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix ex: <{EX}> .\n{constraint}\n"
    );
    parse_shapes(&turtle, None).unwrap_or_else(|error| panic!("{constraint}: {error}"))
}

fn sparql_constraint(select: &str) -> String {
    format!(
        r#"ex:ItemShape a sh:NodeShape ; sh:targetObjectsOf ex:tagged ;
    sh:sparql [ a sh:SPARQLConstraint ; sh:select """{select}""" ] ."#
    )
}

fn select_validator(select: &str) -> String {
    format!(
        r#"ex:Owned a sh:ConstraintComponent ;
    sh:parameter [ sh:path ex:owned ] ;
    sh:nodeValidator ex:OwnedValidator .
ex:OwnedValidator a sh:SPARQLSelectValidator ; sh:select """{select}""" .
ex:ItemShape a sh:NodeShape ; sh:targetObjectsOf ex:tagged ; ex:owned true ."#
    )
}

/// Validate the fixture and hold the report to [`expected`].
fn check_report(position: &str, constraint: &str) {
    let dataset = fixture();
    let items = items(&dataset);
    let report = validate_dataset(&dataset, &shapes_graph(constraint))
        .unwrap_or_else(|error| panic!("{position}: the validation runs: {error}"));
    let reported: BTreeSet<(TermId, String)> = report
        .results
        .iter()
        .map(|result| {
            let focus = result.focus_node.to_term_value();
            (
                dataset
                    .term_id_by_value(&focus)
                    .unwrap_or_else(|| panic!("{position}: {focus:?} is a fixture node")),
                result.value.as_ref().map_or_default(ToString::to_string),
            )
        })
        .collect();
    assert!(!report.conforms, "{position}: {report:?}");
    assert_eq!(
        reported,
        expected(&items),
        "{position}: exactly the five violating nodes — IRI, blank, literal, quoted, quoted \
         with a blank — each reported as itself with its own status, and no conforming node"
    );
    assert_eq!(report.results.len(), 5, "{position}: {report:?}");
}

/// The records that are `bad`, as a pattern binding `?x` (the item) and `?value`.
const BAD_RECORDS: &str = "?rec <http://example.org/about> ?x ; <http://example.org/status> ?value \
                           FILTER(STRSTARTS(?value, \"bad\"))";

/// **`HAVING` at the top level, through `sh:sparql`.** The seed is joined beneath the
/// `GROUP BY`, which hides `$this` from the `HAVING`; a blank-node focus node reported
/// nothing where an IRI one was reported.
#[test]
fn having_at_the_top_level_reads_the_focus_node() {
    check_report(
        "top-level HAVING",
        &sparql_constraint(&format!(
            "SELECT ?value WHERE {{ {BAD_RECORDS} }} GROUP BY ?value HAVING(SAMPLE(?x) = $this)"
        )),
    );
}

/// **The same constraint as a `sh:SPARQLSelectValidator`,** with a component parameter
/// written in beside `$this`.
#[test]
fn having_at_the_top_level_reads_the_focus_node_in_a_select_validator() {
    check_report(
        "SPARQLSelectValidator HAVING",
        &select_validator(&format!(
            "SELECT ?value WHERE {{ {BAD_RECORDS} }} GROUP BY ?value \
             HAVING(sameTerm(SAMPLE(?x), $this) && $owned)"
        )),
    );
}

/// **`HAVING` in a sub-`SELECT` beside another pattern.** SHACL requires the sub-`SELECT`
/// to project `$this`, so it groups by it; its pattern does not bind it, so the key is
/// unbound in every group and the `HAVING` must read the focus node from the rewrite.
#[test]
fn having_in_a_sub_select_reads_the_focus_node() {
    check_report(
        "sub-SELECT HAVING",
        &sparql_constraint(&format!(
            "SELECT $this ?value WHERE {{ <{EX}root> <{EX}tagged> $this . \
             {{ SELECT $this ?value WHERE {{ {BAD_RECORDS} }} GROUP BY $this ?value \
                HAVING(sameTerm(SAMPLE(?x), $this)) }} }}"
        )),
    );
}

/// **A `SELECT` expression after `GROUP BY`.** Every group is a result; the expression
/// names the group's status only for the focus node's own group, and every other group
/// is reported with the value `"other"`, so the expected set is the five `bad` pairs
/// plus one `"other"` pair per focus node.
#[test]
fn a_select_expression_after_group_by_reads_the_focus_node() {
    let dataset = fixture();
    let items = items(&dataset);
    let constraint = sparql_constraint(
        "SELECT (IF(sameTerm(SAMPLE(?x), $this), ?st, \"other\") AS ?value) WHERE { \
           ?rec <http://example.org/about> ?x ; <http://example.org/status> ?st \
           FILTER(STRSTARTS(?st, \"bad\")) } GROUP BY ?st",
    );
    let report = validate_dataset(&dataset, &shapes_graph(&constraint))
        .unwrap_or_else(|error| panic!("the validation runs: {error}"));
    let reported: BTreeSet<(TermId, String)> = report
        .results
        .iter()
        .map(|result| {
            (
                dataset
                    .term_id_by_value(&result.focus_node.to_term_value())
                    .expect("a fixture node"),
                result.value.as_ref().map_or_default(ToString::to_string),
            )
        })
        .collect();
    let mut want = expected(&items);
    want.extend(items.iter().map(|item| (*item, "\"other\"".to_owned())));
    assert_eq!(reported, want, "{report:?}");
}

/// **`ORDER BY` after `GROUP BY`.** The first group is the focus node's own `bad` one
/// when it has one, and otherwise the last `bad` status in text order — so a conforming
/// node is reported too, with `"bad quoted blank"`, and a violating one with its own.
#[test]
fn an_order_by_after_group_by_reads_the_focus_node() {
    let dataset = fixture();
    let items = items(&dataset);
    let constraint = sparql_constraint(&format!(
        "SELECT ?value WHERE {{ {BAD_RECORDS} }} GROUP BY ?value \
         ORDER BY DESC(sameTerm(SAMPLE(?x), $this)) DESC(?value) LIMIT 1"
    ));
    let report = validate_dataset(&dataset, &shapes_graph(&constraint))
        .unwrap_or_else(|error| panic!("the validation runs: {error}"));
    let reported: BTreeSet<(TermId, String)> = report
        .results
        .iter()
        .map(|result| {
            (
                dataset
                    .term_id_by_value(&result.focus_node.to_term_value())
                    .expect("a fixture node"),
                result.value.as_ref().map_or_default(ToString::to_string),
            )
        })
        .collect();
    let want: BTreeSet<(TermId, String)> = STATUSES
        .iter()
        .zip(&items)
        .map(|(status, item)| {
            let value = if status.starts_with("bad") {
                *status
            } else {
                "bad quoted blank"
            };
            (*item, format!("\"{value}\""))
        })
        .collect();
    assert_eq!(reported, want, "{report:?}");
}

/// **An `ORDER BY` inside a sub-`SELECT` beside another pattern** — beneath the seed,
/// where it reads the focus node from the rewrite rather than from its rows.
#[test]
fn an_order_by_beneath_the_seed_reads_the_focus_node() {
    check_report(
        "sub-SELECT ORDER BY",
        &sparql_constraint(&format!(
            "SELECT $this ?value WHERE {{ <{EX}root> <{EX}tagged> $this . \
             {{ SELECT $this ?value WHERE {{ {BAD_RECORDS} }} \
                ORDER BY DESC(sameTerm(?x, $this)) LIMIT 1 }} \
             FILTER EXISTS {{ ?r <{EX}about> $this ; <{EX}status> ?value }} }}"
        )),
    );
}

/// **An `OPTIONAL`'s condition** — beneath the seed, reading the arm's own row.
#[test]
fn an_optional_condition_reads_the_focus_node() {
    check_report(
        "OPTIONAL condition",
        &sparql_constraint(&format!(
            "SELECT ?value WHERE {{ ?rec <{EX}status> ?value \
               OPTIONAL {{ ?rec <{EX}about> ?x FILTER(sameTerm(?x, $this)) }} \
               FILTER(BOUND(?x) && STRSTARTS(?value, \"bad\")) }}"
        )),
    );
}

/// **`HAVING EXISTS` naming `$this` in its body.** The body is matched against the
/// group's rows, which the `GROUP BY` stripped of `$this`, so it matched a free `$this`
/// for every kind of focus node — an IRI included — and every node was reported with
/// every `bad` status. The rows now carry the focus node.
#[test]
fn having_exists_matches_the_focus_node() {
    check_report(
        "HAVING EXISTS",
        &sparql_constraint(&format!(
            "SELECT ?value WHERE {{ {BAD_RECORDS} }} GROUP BY ?value \
             HAVING EXISTS {{ ?other <{EX}about> $this ; <{EX}status> ?value }}"
        )),
    );
}

// ---------------------------------------------------------------------------
// The engine's SHACL pre-binding lane, for the shapes the SHACL surface refuses
// ---------------------------------------------------------------------------

/// Run `query` with `$this` pre-bound to `focus` under the SHACL pre-binding rewrite, and
/// return every `?value` it answers.
fn lane_values(dataset: &RdfDataset, query: &str, focus: &TermValue) -> BTreeSet<String> {
    let substitutions = [("this".to_owned(), focus.clone())];
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &substitutions,
            },
            QueryOptions {
                prebinding: ShaclPrebinding::Applied,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("{query} with {focus:?}: {}", error.message));
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("{query}: a SELECT answers solutions");
    };
    let column = variables
        .iter()
        .position(|variable| variable == "value")
        .expect("the query projects ?value");
    rows.iter()
        .map(|row| match &row[column] {
            Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

/// Hold `query` to the oracle on the SHACL lane for every focus node: its own status
/// exactly when it is a violating node, and nothing otherwise.
fn check_lane(position: &str, query: &str) {
    let dataset = fixture();
    let items = items(&dataset);
    for (status, item) in STATUSES.iter().zip(&items) {
        let focus = dataset.term_value(*item);
        let want: BTreeSet<String> = if status.starts_with("bad") {
            std::iter::once((*status).to_owned()).collect()
        } else {
            BTreeSet::new()
        };
        assert_eq!(
            lane_values(&dataset, query, &focus),
            want,
            "{position}: focus {focus:?}"
        );
    }
}

const ALL_RECORDS: &str = "?rec <http://example.org/about> ?x ; <http://example.org/status> ?value";

/// **A `FILTER` above a sub-`SELECT` that drops `$this`.**
#[test]
fn a_filter_above_a_sub_select_dropping_this_reads_the_focus_node() {
    check_lane(
        "FILTER above a sub-SELECT",
        &format!(
            "SELECT ?value WHERE {{ {{ SELECT ?x ?value WHERE {{ {ALL_RECORDS} }} }} \
             FILTER(sameTerm(?x, $this) && STRSTARTS(?value, \"bad\")) }}"
        ),
    );
}

/// **A `BIND` above a sub-`SELECT` that drops `$this`.**
#[test]
fn a_bind_above_a_sub_select_dropping_this_reads_the_focus_node() {
    check_lane(
        "BIND above a sub-SELECT",
        &format!(
            "SELECT ?value WHERE {{ {{ SELECT ?x ?value WHERE {{ {ALL_RECORDS} }} }} \
             BIND($this AS ?me) FILTER(sameTerm(?x, ?me) && STRSTARTS(?value, \"bad\")) }}"
        ),
    );
}

/// **`HAVING` in a sub-`SELECT` that drops `$this`.**
#[test]
fn having_in_a_sub_select_dropping_this_reads_the_focus_node() {
    check_lane(
        "HAVING in a sub-SELECT dropping $this",
        &format!(
            "SELECT ?value WHERE {{ {{ SELECT ?value WHERE {{ {ALL_RECORDS} }} GROUP BY ?value \
             HAVING(sameTerm(SAMPLE(?x), $this) && STRSTARTS(?value, \"bad\")) }} }}"
        ),
    );
}

/// **`UNFOLD` above a `GROUP BY`, and beneath the seed in a `UNION` branch.** The
/// expression unfolds a group's folded statuses only for the focus node's own group.
#[test]
fn unfold_reads_the_focus_node_above_and_beneath() {
    let folded = format!(
        "{{ SELECT ?x (FOLD(?st) AS ?l) WHERE {{ ?rec <{EX}about> ?x ; <{EX}status> ?st }} \
           GROUP BY ?x }}"
    );
    check_lane(
        "UNFOLD above a GROUP BY",
        &format!(
            "SELECT ?value WHERE {{ {folded} UNFOLD(IF(sameTerm(?x, $this), ?l, ?none) AS ?value) \
             FILTER(STRSTARTS(?value, \"bad\")) }}"
        ),
    );
    check_lane(
        "UNFOLD beneath the seed",
        &format!(
            "SELECT ?value WHERE {{ {{ {folded} UNFOLD(IF(sameTerm(?x, $this), ?l, ?none) AS \
             ?value) }} UNION {{ FILTER(false) }} FILTER(STRSTARTS(?value, \"bad\")) }}"
        ),
    );
}

/// **An expression inside an `EXISTS` body reading `$this` beside a variable of the row
/// it filters.** The body's `FILTER` is driven with the focus node, and the row's
/// `?value` — which the body's own pattern does not bind — still reaches it: the drive
/// carries every variable the `FILTER` names, not only the columns it exposes.
#[test]
fn an_exists_body_expression_reads_the_focus_node_and_the_filtered_row() {
    check_report(
        "EXISTS body reading $this and an outer variable",
        &sparql_constraint(&format!(
            "SELECT $this ?value WHERE {{ ?rec <{EX}about> $this ; <{EX}status> ?value \
             FILTER EXISTS {{ ?other <{EX}status> ?s \
               FILTER(sameTerm(?s, ?value) && STRSTARTS(?s, \"bad\") \
                      && !sameTerm($this, <{EX}nothing>)) }} }}"
        )),
    );
}
