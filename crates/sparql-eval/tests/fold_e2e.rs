// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SEP-0009's `FOLD` aggregate, end to end through the PUBLIC
//! [`NativeSparqlEngine`] query entry — never reaching into the crate.
//!
//! # What has to be true
//!
//! `FOLD(expr)` folds a group into a `cdt:List` literal and `FOLD(kexpr, vexpr)`
//! into a `cdt:Map` literal. The cases below pin every branch the spec
//! distinguishes, because each one is a place a plausible implementation quietly
//! gives a different answer:
//!
//! * an EMPTY group folds to the EMPTY composite, never to unbound;
//! * an unbound argument is a `null` ELEMENT of the list, so it still counts
//!   towards the size — the opposite of every other SPARQL aggregate, all of
//!   which skip such a row;
//! * a map pair with a `null` (or blank-node) KEY vanishes; one with a `null`
//!   VALUE does not;
//! * a repeated map key keeps the LAST binding in row order, which is only
//!   defined when `ORDER BY` is written;
//! * `DISTINCT` dedups on the argument tuple by TERM, so `"1"^^xsd:integer` and
//!   `"01"^^xsd:integer` are two elements, and two unbound arguments are one;
//! * `FOLD`'s own `ORDER BY` sorts the GROUP before the fold, with SPARQL's own
//!   ordering (unbound sorts first) and `ASC`/`DESC`.
//!
//! Every case is written as an `ASK` whose body constrains the folded literal,
//! which reads the answer off the production evaluator rather than off an
//! accumulator called directly. The `ASK` shapes mirror the vendored SEP-0009
//! corpus under `vectors/sparql-cdt/fold/`; the named corpus case each one
//! mirrors is given in its doc comment.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::NativeSparqlEngine;

const PROLOGUE: &str = "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/>\n\
                        PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n";

/// The canonical spelling of an `xsd:integer` composite ELEMENT. A composite
/// literal PurRDF mints is always explicit — `"10"^^<…#integer>`, never the bare
/// `10` shorthand the lexical space also admits — because the canonical form is
/// chosen for byte-determinism, not brevity (see `purrdf_cdt::render`). Every
/// case that asserts on the minted BYTES goes through this; every case that only
/// asserts on the VALUE uses `=` and does not care.
fn canon_int(lexical: u32) -> String {
    format!("\"{lexical}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
}

/// An empty default graph — every case's data comes from its own `VALUES` block.
fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("empty dataset")
}

/// Evaluate one `ASK` (with the CDT/XSD prologue prepended) and answer its boolean.
fn ask(body: &str) -> bool {
    let query = format!("{PROLOGUE}{body}");
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    match result {
        SparqlResult::Boolean(b) => b,
        other => panic!("expected a boolean ASK result, got {other:?}"),
    }
}

/// Evaluate one single-column `SELECT` and answer the one row's one binding's
/// lexical form, asserting the row count is EXACTLY one.
fn one_lexical(body: &str) -> String {
    let query = format!("{PROLOGUE}{body}");
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected a SELECT result");
    };
    assert_eq!(
        rows.len(),
        1,
        "a single ungrouped aggregate produces exactly one row, got {}",
        rows.len()
    );
    match rows[0].first().and_then(Option::as_ref) {
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        other => panic!("expected a composite literal binding, got {other:?}"),
    }
}

/// `fold-list-01`: a one-element group folds to a one-element list.
#[test]
fn fold_one_element_list() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v) AS ?list) WHERE { VALUES ?v { 1 } } } \
         FILTER(?list = \"[1]\"^^cdt:List) }"
    ));
}

/// `fold-list-02`: an EMPTY group folds to the EMPTY list, not to unbound.
#[test]
fn fold_empty_group_is_the_empty_list() {
    assert_eq!(
        one_lexical("SELECT (FOLD(?v) AS ?list) WHERE { FILTER(false) }"),
        "[]"
    );
}

/// `fold-map-01`: an EMPTY group folds to the EMPTY map.
#[test]
fn fold_empty_group_is_the_empty_map() {
    assert_eq!(
        one_lexical("SELECT (FOLD(?k, ?v) AS ?map) WHERE { FILTER(false) }"),
        "{}"
    );
}

/// `fold-list-04`: an UNBOUND argument is a `null` element — the list has size
/// two, not one. This is the case a "skip the row like every other aggregate"
/// implementation gets wrong while every ordinary test still passes.
#[test]
fn fold_writes_an_unbound_argument_as_a_null_element() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v) AS ?list) WHERE { VALUES ?v { 1 UNDEF } } } \
         FILTER( cdt:size(?list) = 2 ) \
         BIND( cdt:get(?list, 1) AS ?e1 ) BIND( cdt:get(?list, 2) AS ?e2 ) \
         FILTER( ( ?e1 = 1 && ! BOUND(?e2) ) || ( ?e2 = 1 && ! BOUND(?e1) ) ) }"
    ));
}

/// `fold-list-05`: two unbound arguments are two DISTINCT `null` elements without
/// `DISTINCT` — size three.
#[test]
fn fold_keeps_every_null_element_without_distinct() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v) AS ?list) WHERE { VALUES ?v { 1 UNDEF UNDEF } } } \
         FILTER( cdt:size(?list) = 3 ) }"
    ));
}

/// `fold-list-distinct-05`: `DISTINCT` collapses the two `null`s to ONE — the
/// unbound slot is part of the tuple's identity, not an absent tuple.
#[test]
fn fold_distinct_collapses_repeated_nulls_to_one() {
    assert!(ask(
        "ASK { { SELECT (FOLD(DISTINCT ?v) AS ?list) WHERE { VALUES ?v { 1 UNDEF UNDEF } } } \
         FILTER( cdt:size(?list) = 2 ) }"
    ));
}

/// `fold-list-distinct-07`: `DISTINCT` is by TERM, so two value-equal but
/// lexically distinct integers are two elements. The neighbouring case below
/// proves the dedup still fires for a genuinely repeated term.
#[test]
fn fold_distinct_separates_two_spellings_of_one_value() {
    assert!(ask("ASK { { SELECT (FOLD(DISTINCT ?v) AS ?list) \
         WHERE { VALUES ?v { \"1\"^^xsd:integer \"01\"^^xsd:integer } } } \
         FILTER( cdt:size(?list) = 2 ) }"));
}

/// `fold-list-distinct-01`: the neighbouring positive control — a genuinely
/// repeated term IS deduplicated.
#[test]
fn fold_distinct_collapses_a_repeated_term() {
    assert!(ask(
        "ASK { { SELECT (FOLD(DISTINCT ?v) AS ?list) WHERE { VALUES ?v { 1 1 } } } \
         FILTER(?list = \"[1]\"^^cdt:List) }"
    ));
}

/// `fold-list-orderby-01`: `ORDER BY` sorts the group before the fold, so the
/// element order is defined.
#[test]
fn fold_order_by_sorts_the_group_before_folding() {
    assert!(ask("ASK { { SELECT (FOLD(?v ORDER BY ?v) AS ?list) \
         WHERE { VALUES ?v { 1 2 3 6 5 4 } } } \
         FILTER(?list = \"[1,2,3,4,5,6]\"^^cdt:List) }"));
}

/// `fold-list-orderby-03`: `DESC` reverses it.
#[test]
fn fold_order_by_desc_reverses_the_group() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v ORDER BY DESC(?sort)) AS ?list) \
         WHERE { VALUES (?sort ?v) { (1 \"one\") (2 \"two\") (3 \"three\") } } } \
         FILTER(?list = \"['three','two','one']\"^^cdt:List) }"
    ));
}

/// `fold-list-orderby-05`: an UNBOUND sort key sorts FIRST, below every bound
/// term — SPARQL's own §15.1 ordering, not a separate one.
#[test]
fn fold_order_by_sorts_an_unbound_key_first() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v ORDER BY ASC(?sort)) AS ?list) \
         WHERE { VALUES (?sort ?v) { (<http://example.org/i> 3) (\"literal\" 4) (UNDEF 1) } } } \
         FILTER(?list = \"[1,3,4]\"^^cdt:List) }"
    ));
}

/// `fold-list-orderby-06`: several sort conditions apply left to right.
#[test]
fn fold_order_by_applies_multiple_conditions_in_order() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v ORDER BY ASC(?sort1) ASC(?sort2)) AS ?list) \
         WHERE { VALUES (?sort1 ?sort2 ?v) { \
         (2 2 \"three\") (2 3 \"four\") (2 1 \"two\") (1 UNDEF \"one\") } } } \
         FILTER(?list = \"['one','two','three','four']\"^^cdt:List) }"
    ));
}

/// `fold-map-02`: the two-argument form builds a map, whose canonical form
/// orders entries by key regardless of row order.
#[test]
fn fold_two_arguments_build_a_map() {
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?map) \
         WHERE { VALUES (?k ?v) { (2 \"two\") (1 \"one\") (\"hello\"@en \"there\"@en) } } } \
         FILTER(?map = \"{\\\"hello\\\"@en:'there'@en,1:'one',2:'two'}\"^^cdt:Map) }"));
}

/// `fold-map-04`: an unbound KEY drops the whole pair.
#[test]
fn fold_map_drops_a_pair_with_an_unbound_key() {
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?map) \
         WHERE { VALUES (?k ?v) { (UNDEF \"two\") (1 \"one\") } } } \
         FILTER(?map = \"{1:'one'}\"^^cdt:Map) }"));
}

/// `fold-map-03`: an unbound VALUE keeps its key, with a `null` value — the
/// neighbouring valid case for the drop above, and the asymmetry the spec is
/// explicit about.
#[test]
fn fold_map_keeps_a_pair_with_an_unbound_value() {
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?map) \
         WHERE { VALUES (?k ?v) { (2 UNDEF) (1 \"one\") } } } \
         FILTER(cdt:get(?map, 1) = \"one\") \
         FILTER(cdt:containsKey(?map, 2)) \
         BIND(cdt:get(?map, 2) AS ?null) FILTER(!BOUND(?null)) }"));
}

/// `fold-map-05`: a BLANK NODE is not a map key, so its pair vanishes.
#[test]
fn fold_map_drops_a_pair_with_a_blank_node_key() {
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?map) WHERE { \
         { BIND( BNODE() AS ?k ) BIND( 1 AS ?v ) } UNION { BIND( 42 AS ?k ) BIND( 2 AS ?v ) } } } \
         FILTER(?map = \"{42:2}\"^^cdt:Map) }"));
}

/// `fold-map-orderby-01`: with `ORDER BY`, a repeated key keeps the LAST binding
/// in the sorted row order.
#[test]
fn fold_map_order_by_keeps_the_last_binding_of_a_repeated_key() {
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v ORDER BY ?sort) AS ?map) \
         WHERE { VALUES (?k ?v ?sort) { \
         (1 100 \"irrelevant\") (2 201 1) (2 203 3) (2 202 2) } } } \
         FILTER( ?map = \"{1:100, 2:203}\"^^cdt:Map ) }"));
}

/// `fold-map-orderby-02`: reversing the order changes which binding survives —
/// proof the sort really happens BEFORE the fold, not after it.
#[test]
fn fold_map_order_by_desc_changes_which_binding_survives() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?k, ?v ORDER BY DESC(?sort)) AS ?map) \
         WHERE { VALUES (?k ?v ?sort) { \
         (1 100 \"irrelevant\") (2 201 1) (2 203 3) (2 202 2) } } } \
         FILTER( ?map = \"{1:100, 2:201}\"^^cdt:Map ) }"
    ));
}

/// `FOLD` groups like any other aggregate: one composite per `GROUP BY` key.
#[test]
fn fold_produces_one_composite_per_group() {
    let query = format!(
        "{PROLOGUE}SELECT ?g (FOLD(?v ORDER BY ?v) AS ?list) WHERE {{ \
         VALUES (?g ?v) {{ (1 10) (1 11) (2 20) }} }} GROUP BY ?g ORDER BY ?g"
    );
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            &empty_dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected a SELECT result");
    };
    let lexicals: Vec<String> = rows
        .iter()
        .map(|row| match row.get(1).and_then(Option::as_ref) {
            Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
            other => panic!("expected a composite literal, got {other:?}"),
        })
        .collect();
    assert_eq!(
        lexicals,
        vec![
            format!("[{},{}]", canon_int(10), canon_int(11)),
            format!("[{}]", canon_int(20)),
        ]
    );
}

/// A large group must fold to the SAME list whether or not the engine chunked
/// it: `combine` appends element vectors rather than re-stepping a finished
/// list, so the chunk boundary is invisible. The size here is deliberately far
/// above any plausible chunking threshold.
#[test]
fn a_large_group_folds_to_the_same_list_a_small_one_would() {
    let values: String = (1..=512).fold(String::new(), |mut acc, i| {
        use std::fmt::Write as _;
        let _ = write!(acc, " {i}");
        acc
    });
    let folded = one_lexical(&format!(
        "SELECT (FOLD(?v ORDER BY ?v) AS ?list) WHERE {{ VALUES ?v {{{values} }} }}"
    ));
    let expected: Vec<String> = (1..=512).map(canon_int).collect();
    assert_eq!(folded, format!("[{}]", expected.join(",")));
}

/// `FOLD` mints its value in the canonical form, so two independent evaluations
/// of the same fold are the SAME RDF term. That is what `sameTerm` reads.
#[test]
fn two_evaluations_of_one_fold_are_the_same_term() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v ORDER BY ?v) AS ?a) WHERE { VALUES ?v { 2 1 } } } \
         { SELECT (FOLD(?v ORDER BY ?v) AS ?b) WHERE { VALUES ?v { 1 2 } } } \
         FILTER(sameTerm(?a, ?b)) }"
    ));
}
