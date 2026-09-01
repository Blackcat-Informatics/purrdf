// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SEP-0009 `FOLD` aggregate and `UNFOLD` graph pattern, evaluated end to end
//! from the vantage a host has: real query text, parsed through
//! [`purrdf_sparql_algebra::SparqlParser`], run through the PUBLIC
//! [`NativeSparqlEngine`].
//!
//! # What this file is for
//!
//! The vendored corpus (`vectors/sparql-cdt/fold/`, `vectors/sparql-cdt/unfold/`)
//! is the contract for the behaviour it writes down, and the conformance harness
//! grades it. This file pins the behaviour the corpus leaves UNWRITTEN — every
//! composition and edge case a host will hit that no upstream case asserts — so
//! each such decision is a test rather than an accident of the implementation:
//!
//! * `FOLD` under `GROUP BY`, in `HAVING`, beside another aggregate, nested
//!   inside a larger expression, and with its own `ORDER BY` beside the query's;
//! * `FOLD(DISTINCT ?k, ?v)` — the map form's de-duplication key;
//! * a `FOLD` expression that RAISES rather than being merely unbound;
//! * a nested composite in `FOLD`'s key position;
//! * `UNFOLD` over an empty, unbound, erroring, non-composite or ill-formed
//!   operand;
//! * `UNFOLD` first in its group, twice in one group, over a nested composite,
//!   and composed with `OPTIONAL`/`UNION`/`MINUS`/`GRAPH`/`VALUES`/`SELECT *`.
//!
//! # Fixture
//!
//! Most cases need no data at all — a composite lives in a literal — so they run
//! against the EMPTY dataset, exactly as every SEP-0009 conformance case does. The
//! two that genuinely need quads (the `GRAPH` and `OPTIONAL` compositions) use a
//! two-graph fixture under `https://example.org/cdt#`.

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

/// The SEP-0009 prologue every query is written under. The namespace is the
/// spec's own fixed string — recognized, never minted — and the fixture
/// namespace is `example.org`, per the repository's fixture rule.
const PFX: &str = "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/> \
                   PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> \
                   PREFIX : <https://example.org/cdt#> ";

const EX: &str = "https://example.org/cdt#";

/// An empty dataset: a composite value lives in a literal, so nothing here needs
/// quads.
fn empty() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("freeze")
}

/// `:s :list "[…]"^^cdt:List` in the default graph, `:s :list "[…]"^^cdt:List` in
/// `:g1` — the two-graph fixture the `GRAPH`/`OPTIONAL` compositions need.
fn graphed() -> Arc<RdfDataset> {
    const CDT_LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let t = b.intern_iri(&format!("{EX}t"));
    let list = b.intern_iri(&format!("{EX}list"));
    let g1 = b.intern_iri(&format!("{EX}g1"));
    let default_value = b.intern_literal(RdfLiteral::typed("[1,2]", CDT_LIST));
    let g1_value = b.intern_literal(RdfLiteral::typed("[3]", CDT_LIST));
    let not_a_list = b.intern_literal(RdfLiteral::simple("not a composite"));
    b.push_quad(s, list, default_value, None);
    b.push_quad(t, list, not_a_list, None);
    b.push_quad(s, list, g1_value, Some(g1));
    b.freeze().expect("freeze")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

/// Evaluate a query against `dataset` through the public engine.
fn evaluate(dataset: &Arc<RdfDataset>, query: &str) -> SparqlResult {
    NativeSparqlEngine::new()
        .query_with_options_view(
            dataset.as_ref(),
            request(&format!("{PFX}{query}")),
            QueryOptions::EMPTY,
        )
        .unwrap_or_else(|error| panic!("evaluate `{query}`: {error:?}"))
}

/// A query expected to HARD-FAIL, returning the rendered diagnostic.
fn evaluate_err(dataset: &Arc<RdfDataset>, query: &str) -> String {
    let error = NativeSparqlEngine::new()
        .query_with_options_view(
            dataset.as_ref(),
            request(&format!("{PFX}{query}")),
            QueryOptions::EMPTY,
        )
        .expect_err("expected a hard failure");
    format!("{error:?}")
}

/// An `ASK`'s boolean answer.
fn ask(query: &str) -> bool {
    match evaluate(&empty(), query) {
        SparqlResult::Boolean(answer) => answer,
        other => panic!("expected a boolean, got {other:?}"),
    }
}

/// One cell, rendered for comparison. `UNBOUND` is a distinct, assertable value:
/// half of `UNFOLD`'s corpus turns on a row being PRODUCED with a column unbound
/// rather than the row being dropped.
fn cell(value: Option<&TermValue>) -> String {
    match value {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(iri)) => format!("<{iri}>"),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
        Some(other) => format!("{other:?}"),
    }
}

type Row = BTreeMap<String, String>;

/// A `SELECT` result's rows as variable-keyed maps, in the order the engine
/// produced them — `UNFOLD`'s output order is normative (list order), so these are
/// deliberately NOT sorted.
fn rows(result: &SparqlResult) -> Vec<Row> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("expected a SELECT result, got {result:?}");
    };
    rows.iter()
        .map(|row| {
            variables
                .iter()
                .cloned()
                .zip(row.iter().map(|cell_value| cell(cell_value.as_ref())))
                .collect()
        })
        .collect()
}

/// The canonical lexical form `purrdf-cdt` mints for a `cdt:List` of
/// `xsd:integer`s — every literal fully spelled, no whitespace. A folded value is
/// one PurRDF COMPUTED, so it carries this form rather than whatever the query
/// text happened to look like (see `crate::cdt_fn::composite_literal`), and
/// spelling it out here is what keeps these expectations honest about it.
fn canonical_integer_list(values: &[i64]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|v| format!("\"{v}\"^^<http://www.w3.org/2001/XMLSchema#integer>"))
        .collect();
    format!("[{}]", items.join(","))
}

/// Build one expected row from `(variable, rendered-value)` pairs.
fn row(pairs: &[(&str, &str)]) -> Row {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

// ---------------------------------------------------------------------------
// FOLD — the shapes the corpus writes, confirmed through the public engine
// ---------------------------------------------------------------------------

/// The two forms, the empty group, and the retained `null` — the corpus's own
/// core rules, asserted once here so this file's edge cases are read against a
/// baseline that is known to hold through the PUBLIC entry point.
#[test]
fn the_two_forms_the_empty_group_and_the_retained_null() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?v) AS ?l) WHERE { VALUES ?v { 1 } } } \
         FILTER(?l = \"[1]\"^^cdt:List) }"
    ));
    assert!(ask(
        "ASK { { SELECT (FOLD(?k, ?v) AS ?m) WHERE { VALUES (?k ?v) { (1 2) } } } \
         FILTER(?m = \"{1:2}\"^^cdt:Map) }"
    ));
    // An empty group is a BOUND empty composite, never unbound.
    assert!(ask(
        "ASK { { SELECT (FOLD(?v) AS ?l) WHERE { FILTER(false) } } \
         FILTER(BOUND(?l)) FILTER(?l = \"[]\"^^cdt:List) }"
    ));
    assert!(ask(
        "ASK { { SELECT (FOLD(?k, ?v) AS ?m) WHERE { FILTER(false) } } \
         FILTER(BOUND(?m)) FILTER(?m = \"{}\"^^cdt:Map) }"
    ));
    // An unbound row is a RETAINED null that counts toward the length.
    assert!(ask("ASK { { SELECT (FOLD(?v ORDER BY ?sort) AS ?l) \
         WHERE { VALUES (?sort ?v) { (1 1) (2 UNDEF) (3 3) } } } \
         FILTER(cdt:size(?l) = 3) FILTER(?l = \"[1,null,3]\"^^cdt:List) }"));
}

// ---------------------------------------------------------------------------
// FOLD — decisions the corpus does not pin
// ---------------------------------------------------------------------------

/// A `FOLD` under `GROUP BY` folds each group SEPARATELY — one composite per
/// group, over that group's rows only. The corpus only ever folds an implicit
/// single group.
#[test]
fn fold_under_group_by_folds_each_group_separately() {
    let result = evaluate(
        &empty(),
        "SELECT ?g (FOLD(?v ORDER BY ?v) AS ?l) \
         WHERE { VALUES (?g ?v) { (1 10) (1 11) (2 20) } } \
         GROUP BY ?g ORDER BY ?g",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("g", "1"), ("l", &canonical_integer_list(&[10, 11]))]),
            row(&[("g", "2"), ("l", &canonical_integer_list(&[20]))]),
        ]
    );
}

/// A `FOLD` is usable in `HAVING`, like any other aggregate: the group's folded
/// value is available to the post-grouping filter.
#[test]
fn fold_is_usable_in_having() {
    let result = evaluate(
        &empty(),
        "SELECT ?g WHERE { VALUES (?g ?v) { (1 10) (1 11) (2 20) } } \
         GROUP BY ?g HAVING(cdt:size(FOLD(?v)) > 1) ORDER BY ?g",
    );
    assert_eq!(rows(&result), vec![row(&[("g", "1")])]);
}

/// A `FOLD` sits beside another aggregate in one `SELECT`, each folding the same
/// group independently.
#[test]
fn fold_coexists_with_another_aggregate_over_one_group() {
    let result = evaluate(
        &empty(),
        "SELECT (FOLD(?v ORDER BY ?v) AS ?l) (COUNT(?v) AS ?c) (SUM(?v) AS ?s) \
         WHERE { VALUES ?v { 3 1 2 } }",
    );
    assert_eq!(
        rows(&result),
        vec![row(&[
            ("l", &canonical_integer_list(&[1, 2, 3])),
            ("c", "3"),
            ("s", "6"),
        ])]
    );
}

/// A `FOLD` nests inside a larger expression: the aggregate lifts to a synthetic
/// variable the surrounding expression reads, exactly as `COUNT` does.
#[test]
fn fold_nests_inside_a_larger_expression() {
    assert!(ask(
        "ASK { { SELECT ((cdt:size(FOLD(?v)) * 10) AS ?n) WHERE { VALUES ?v { 1 2 3 } } } \
         FILTER(?n = 30) }"
    ));
    assert!(ask(
        "ASK { { SELECT (cdt:get(FOLD(?v ORDER BY DESC(?v)), 1) AS ?first) \
         WHERE { VALUES ?v { 1 2 3 } } } FILTER(?first = 3) }"
    ));
}

/// The aggregate's `ORDER BY` and the query's own are INDEPENDENT: the inner one
/// orders the elements inside each group's composite, the outer one orders the
/// result rows. Reversing one must not reverse the other.
#[test]
fn an_inner_fold_order_by_is_independent_of_the_outer_one() {
    let result = evaluate(
        &empty(),
        "SELECT ?g (FOLD(?v ORDER BY ASC(?v)) AS ?l) \
         WHERE { VALUES (?g ?v) { (1 11) (1 10) (2 21) (2 20) } } \
         GROUP BY ?g ORDER BY DESC(?g)",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("g", "2"), ("l", &canonical_integer_list(&[20, 21]))]),
            row(&[("g", "1"), ("l", &canonical_integer_list(&[10, 11]))]),
        ],
        "rows descend by ?g while each list ascends by ?v"
    );
}

/// `FOLD(DISTINCT ?k, ?v)` — a form the corpus never writes. The decision:
/// de-duplication is on the WHOLE `(key, value)` pair, the rule a multi-argument
/// custom aggregate already follows. Two rows sharing a key but not a value are
/// therefore BOTH folded (and the later one wins the key, per SEP-0009), while an
/// exact duplicate pair collapses.
#[test]
fn distinct_over_the_map_form_deduplicates_the_whole_pair() {
    // Same key, DIFFERENT values: `DISTINCT` keeps both rows, so the map's
    // last-in-order rule decides — descending by ?sort makes 201 the last folded.
    assert!(ask(
        "ASK { { SELECT (FOLD(DISTINCT ?k, ?v ORDER BY DESC(?sort)) AS ?m) \
         WHERE { VALUES (?k ?v ?sort) { (1 201 1) (1 202 2) } } } \
         FILTER(?m = \"{1:201}\"^^cdt:Map) }"
    ));
    // The same query WITHOUT `DISTINCT` answers identically — which is the point:
    // pair-wise de-duplication does not change which value survives when the pairs
    // differ.
    assert!(ask(
        "ASK { { SELECT (FOLD(?k, ?v ORDER BY DESC(?sort)) AS ?m) \
         WHERE { VALUES (?k ?v ?sort) { (1 201 1) (1 202 2) } } } \
         FILTER(?m = \"{1:201}\"^^cdt:Map) }"
    ));
    // An EXACT duplicate pair collapses to one entry, and a distinct pair survives.
    assert!(ask("ASK { { SELECT (FOLD(DISTINCT ?k, ?v) AS ?m) \
         WHERE { VALUES (?k ?v) { (1 2) (1 2) (3 4) } } } \
         FILTER(?m = \"{1:2, 3:4}\"^^cdt:Map) FILTER(cdt:size(?m) = 2) }"));
}

/// `DISTINCT` reads only the exprlist. Two rows agreeing on the folded value
/// collapse however much they disagree on the sort key — and the survivor keeps
/// its OWN (first-in-row-order) key, which is what the pre-sort de-duplication
/// order means.
#[test]
fn distinct_ignores_the_sort_key_and_keeps_the_first_occurrence() {
    assert!(ask(
        "ASK { { SELECT (FOLD(DISTINCT ?v ORDER BY ASC(?sort)) AS ?l) \
         WHERE { VALUES (?sort ?v) { (2 'x') (1 'y') (0 'x') } } } \
         FILTER(?l = \"['y','x']\"^^cdt:List) }"
    ));
}

/// An expression that RAISES a genuine evaluation error — not merely an unbound
/// variable — is the same retained `null`. SEP-0009 treats "unbound" and "raised"
/// identically for a constructor argument, and a `FOLD` element is a constructor
/// argument spread over rows.
#[test]
fn a_raised_expression_is_a_retained_null_not_a_skipped_row() {
    // `1/0` is a SPARQL evaluation error. The list keeps its length…
    assert!(ask(
        "ASK { { SELECT (FOLD(IF(?v = 2, 1/0, ?v) ORDER BY ?v) AS ?l) \
         WHERE { VALUES ?v { 1 2 3 } } } \
         FILTER(cdt:size(?l) = 3) FILTER(?l = \"[1,null,3]\"^^cdt:List) }"
    ));
    // …and in the MAP form a raised KEY drops its entry, exactly as an unbound one
    // does, while a raised VALUE keeps the entry with a null.
    assert!(ask("ASK { { SELECT (FOLD(IF(?k = 2, 1/0, ?k), ?v) AS ?m) \
         WHERE { VALUES (?k ?v) { (1 10) (2 20) } } } \
         FILTER(?m = \"{1 : 10}\"^^cdt:Map) }"));
    assert!(ask(
        "ASK { { SELECT (FOLD(?k, IF(?v = 20, 1/0, ?v)) AS ?m) \
         WHERE { VALUES (?k ?v) { (1 10) (2 20) } } } \
         FILTER(cdt:size(?m) = 2) FILTER(cdt:containsKey(?m, 2)) \
         BIND(cdt:get(?m, 2) AS ?got) FILTER(!BOUND(?got)) }"
    ));
}

/// A NESTED COMPOSITE in the key position drops its entry, the same way a blank
/// node does: SEP-0009's `MapKey` production admits an IRI and a literal and
/// nothing else, so a composite denotes no key at all. In the VALUE position the
/// same term is perfectly ordinary and is stored as a nested composite.
#[test]
fn a_nested_composite_key_drops_its_entry_but_a_composite_value_is_kept() {
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?m) \
         WHERE { VALUES (?k ?v) { (\"[1]\"^^cdt:List 10) (2 20) } } } \
         FILTER(cdt:size(?m) = 1) FILTER(?m = \"{2:20}\"^^cdt:Map) }"));
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?m) \
         WHERE { VALUES (?k ?v) { (1 \"[10]\"^^cdt:List) } } } \
         FILTER(cdt:get(cdt:get(?m, 1), 1) = 10) }"));
    // A composite ELEMENT of a folded list nests one bracket level rather than
    // flattening.
    assert!(ask("ASK { { SELECT (FOLD(?v ORDER BY ?sort) AS ?l) \
         WHERE { VALUES (?sort ?v) { (1 \"[1,2]\"^^cdt:List) (2 3) } } } \
         FILTER(cdt:size(?l) = 2) FILTER(cdt:get(cdt:get(?l, 1), 2) = 2) }"));
}

/// A `FOLD` whose element would nest past `purrdf-cdt`'s depth bound is a HARD
/// failure of the query, never an unbound answer — the same tri-state rule
/// `cdt:List(…)` follows. Degrading a refused mint to unbound would let a resource
/// refusal satisfy a `FILTER(!BOUND(?x))`.
#[test]
fn a_fold_past_the_nesting_bound_is_a_hard_failure() {
    // 64 nested lists is the deepest composite that can exist, so folding one into
    // a 65th level has nowhere to go.
    let mut deepest = "cdt:List()".to_owned();
    for _ in 1..64 {
        deepest = format!("cdt:List({deepest})");
    }
    let message = evaluate_err(
        &empty(),
        &format!("SELECT (FOLD(?v) AS ?l) WHERE {{ BIND({deepest} AS ?v) }}"),
    );
    assert!(
        message.contains("nesting"),
        "the refusal must name the bound it crossed: {message}"
    );
}

// ---------------------------------------------------------------------------
// UNFOLD — the shapes the corpus writes
// ---------------------------------------------------------------------------

/// The four readings, through the public engine: list element, 1-based
/// `xsd:integer` index, map key, map value — in the composite's own order, with
/// duplicates preserved.
#[test]
fn the_four_readings_in_composite_order() {
    let result = evaluate(
        &empty(),
        "SELECT ?e ?i WHERE { BIND(\"[1,1,2]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?i) }",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("e", "1"), ("i", "1")]),
            row(&[("e", "1"), ("i", "2")]),
            row(&[("e", "2"), ("i", "3")]),
        ],
        "list order, duplicates preserved, index 1-based"
    );
    // The index really is an `xsd:integer`, not a plain string.
    assert!(ask(
        "ASK { BIND(\"[9]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?i) \
         FILTER(SAMETERM(?i, \"1\"^^xsd:integer)) }"
    ));

    let result = evaluate(
        &empty(),
        "SELECT ?k ?v WHERE { BIND(\"{1:10, 2:20}\"^^cdt:Map AS ?m) UNFOLD(?m AS ?k, ?v) }",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("k", "1"), ("v", "10")]),
            row(&[("k", "2"), ("v", "20")]),
        ]
    );
}

/// A `null` element produces a row with the element UNBOUND — not zero rows, and
/// not an error — while the index beside it stays bound.
#[test]
fn a_null_element_produces_a_row_with_an_unbound_column() {
    let result = evaluate(
        &empty(),
        "SELECT ?e ?i WHERE { BIND(\"[1,null]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?i) }",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("e", "1"), ("i", "1")]),
            row(&[("e", "UNBOUND"), ("i", "2")]),
        ]
    );
    let result = evaluate(
        &empty(),
        "SELECT ?k ?v WHERE { BIND(\"{1:null, 2:20}\"^^cdt:Map AS ?m) UNFOLD(?m AS ?k, ?v) }",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("k", "1"), ("v", "UNBOUND")]),
            row(&[("k", "2"), ("v", "20")]),
        ]
    );
}

// ---------------------------------------------------------------------------
// UNFOLD — decisions the corpus does not pin
// ---------------------------------------------------------------------------

/// An EMPTY composite contributes ZERO rows, not one row with an unbound target:
/// `UNFOLD` is one row per element, and there is no element.
#[test]
fn an_empty_composite_contributes_zero_rows() {
    for operand in [
        "\"[]\"^^cdt:List",
        "\"{}\"^^cdt:Map",
        "cdt:List()",
        "cdt:Map()",
    ] {
        let result = evaluate(
            &empty(),
            &format!("SELECT ?e WHERE {{ BIND({operand} AS ?c) UNFOLD(?c AS ?e) }}"),
        );
        assert!(
            rows(&result).is_empty(),
            "`{operand}` must expand to no rows, got {:?}",
            rows(&result)
        );
    }
}

/// An operand that denotes NO composite PASSES THE ROW THROUGH, whichever of the
/// four ways it fails to denote one: unbound, raised, not `cdt:`-typed, or
/// `cdt:`-typed with a lexical form that does not parse.
///
/// SEP-0009 §12.3 states this outcome directly in both operator definitions —
/// "expr(μ) is an error or an RDF term that is neither a well-formed cdt:List
/// literal nor a well-formed cdt:Map literal, then Unfold1(μ, var, expr) = { μ }"
/// — so the result is ONE row with the target unbound, not zero rows. The
/// vendored corpus exercises none of these operands, which is precisely why the
/// spec text rather than the corpus is the authority here.
#[test]
fn an_operand_that_denotes_no_composite_passes_the_row_through() {
    for operand in [
        "?nosuchvariable",             // unbound
        "(1/0)",                       // raised
        "42",                          // not composite-typed
        "\"[1,2]\"",                   // a plain string that merely looks like one
        "\"1\"^^cdt:List",             // composite-TYPED but ill-formed
        "<https://example.org/cdt#s>", // an IRI
    ] {
        let result = evaluate(
            &empty(),
            &format!("SELECT ?e WHERE {{ UNFOLD({operand} AS ?e) }}"),
        );
        assert_eq!(
            rows(&result),
            vec![row(&[("e", "UNBOUND")])],
            "`{operand}` denotes no composite, so §12.3 keeps the row with `?e` unbound"
        );
    }
    // The NEIGHBOURING case that must still give the other answer: a WELL-FORMED
    // but EMPTY composite is the one condition §12.3's Notes assign the empty
    // multiset. If the pass-through above were implemented by treating every
    // non-expanding row alike, this would wrongly gain a row.
    for operand in ["\"[]\"^^cdt:List", "\"{}\"^^cdt:Map"] {
        let result = evaluate(
            &empty(),
            &format!("SELECT ?e WHERE {{ UNFOLD({operand} AS ?e) }}"),
        );
        assert!(
            rows(&result).is_empty(),
            "`{operand}` is well-formed and empty, so it still contributes no rows"
        );
    }
}

/// A row whose operand denotes no composite passes through while its NEIGHBOURS
/// expand — the per-row reading of the rule above, which is what makes `UNFOLD`
/// usable over a column that is only sometimes a composite.
///
/// `ORDER BY ?e` puts the passed-through row first, because SPARQL sorts unbound
/// before every bound value.
#[test]
fn a_non_composite_row_passes_through_while_its_neighbours_expand() {
    let result = evaluate(
        &graphed(),
        "SELECT ?s ?e WHERE { ?s :list ?c UNFOLD(?c AS ?e) } ORDER BY ?e",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("s", "<https://example.org/cdt#t>"), ("e", "UNBOUND")]),
            row(&[("s", "<https://example.org/cdt#s>"), ("e", "1")]),
            row(&[("s", "<https://example.org/cdt#s>"), ("e", "2")]),
        ],
        "`:t`'s plain-string object keeps its row with `?e` unbound; `:s`'s list expands"
    );
}

/// `UNFOLD` written FIRST in its group — with nothing before it to bind — drives
/// off the identity table and expands the constructed value directly.
#[test]
fn unfold_first_in_its_group_drives_off_the_identity_table() {
    let result = evaluate(&empty(), "SELECT ?e WHERE { UNFOLD(cdt:List(7, 8) AS ?e) }");
    assert_eq!(rows(&result), vec![row(&[("e", "7")]), row(&[("e", "8")])]);
}

/// Two `UNFOLD`s in one group compose as nested expansions — the second runs once
/// per row the first produced, so the output is their cross product in the outer
/// one's order.
#[test]
fn two_unfolds_in_one_group_compose() {
    let result = evaluate(
        &empty(),
        "SELECT ?a ?b WHERE { UNFOLD(cdt:List(1, 2) AS ?a) UNFOLD(cdt:List(10, 20) AS ?b) }",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("a", "1"), ("b", "10")]),
            row(&[("a", "1"), ("b", "20")]),
            row(&[("a", "2"), ("b", "10")]),
            row(&[("a", "2"), ("b", "20")]),
        ]
    );
}

/// A NESTED composite comes back as an ordinary `cdt:`-typed literal, so a second
/// `UNFOLD` expands it — the operator composes with itself over depth as well as
/// over breadth.
#[test]
fn unfold_expands_a_nested_composite_again() {
    let result = evaluate(
        &empty(),
        "SELECT ?inner WHERE { BIND(\"[[1,2],[3]]\"^^cdt:List AS ?l) \
         UNFOLD(?l AS ?outer) UNFOLD(?outer AS ?inner) }",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("inner", "1")]),
            row(&[("inner", "2")]),
            row(&[("inner", "3")]),
        ]
    );
}

/// `SELECT *` projects both targets alongside everything the inner pattern bound.
#[test]
fn select_star_projects_the_unfold_targets() {
    let result = evaluate(
        &empty(),
        "SELECT * WHERE { BIND(\"{1 : 10}\"^^cdt:Map AS ?m) UNFOLD(?m AS ?k, ?v) }",
    );
    assert_eq!(
        rows(&result),
        vec![row(&[("m", "{1 : 10}"), ("k", "1"), ("v", "10")])]
    );
}

/// A later `FILTER` in the same group sees the bound targets — the corpus's own
/// `unfold-get-*` idiom, and the reason `UNFOLD`'s targets must be in the
/// enclosing group's scope rather than hidden inside a sub-pattern.
#[test]
fn a_later_filter_in_the_same_group_sees_the_targets() {
    let result = evaluate(
        &empty(),
        "SELECT ?e WHERE { BIND(\"[1,2,3]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e) FILTER(?e > 1) }",
    );
    assert_eq!(rows(&result), vec![row(&[("e", "2")]), row(&[("e", "3")])]);
}

/// `OPTIONAL`: a left row whose operand is not a composite keeps its row with the
/// target unbound, exactly as any other optional pattern that fails to match
/// does. This is the composition the "zero rows, never a refusal" decision exists
/// to make work: a refusal would have failed the whole query on `:t`'s
/// plain-string object instead of leaving `?e` unbound for that row alone.
///
/// The `OPTIONAL` body binds its own operand, because SPARQL 1.1 §18.2.2.6
/// evaluates a `LeftJoin`'s right operand INDEPENDENTLY of its left and joins
/// afterwards — see [`an_uncorrelated_optional_body_sees_no_left_binding`] for
/// the shape that does not, and why that is ordinary SPARQL rather than anything
/// `UNFOLD` decides.
#[test]
fn unfold_inside_optional_leaves_a_non_matching_left_row_intact() {
    let result = evaluate(
        &graphed(),
        "SELECT ?s ?e WHERE { ?s :list ?c \
         OPTIONAL { ?s :list ?d UNFOLD(?d AS ?e) } } ORDER BY ?s ?e",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("s", "<https://example.org/cdt#s>"), ("e", "1")]),
            row(&[("s", "<https://example.org/cdt#s>"), ("e", "2")]),
            row(&[("s", "<https://example.org/cdt#t>"), ("e", "UNBOUND")]),
        ]
    );
}

/// The other half of the rule above, stated so it is not mistaken for an `UNFOLD`
/// defect: an `OPTIONAL` body that reads a variable only the LEFT operand binds
/// sees it unbound, because §18.2.2.6 evaluates the right operand on its own.
/// `UNFOLD` then expands nothing and every left row keeps its target unbound —
/// EXACTLY what `BIND` does in the same position, which is what makes this
/// `LeftJoin` semantics rather than anything this operator decided.
#[test]
fn an_uncorrelated_optional_body_sees_no_left_binding() {
    let unfolded = evaluate(
        &graphed(),
        "SELECT ?s ?e WHERE { ?s :list ?c OPTIONAL { UNFOLD(?c AS ?e) } } ORDER BY ?s",
    );
    let bound = evaluate(
        &graphed(),
        "SELECT ?s ?e WHERE { ?s :list ?c OPTIONAL { BIND(cdt:size(?c) AS ?e) } } ORDER BY ?s",
    );
    let all_unbound = |result: &SparqlResult| {
        rows(result)
            .iter()
            .all(|row| row.get("e").map(String::as_str) == Some("UNBOUND"))
    };
    assert!(all_unbound(&unfolded), "{:?}", rows(&unfolded));
    assert!(
        all_unbound(&bound),
        "BIND behaves identically, so this is LeftJoin semantics, not UNFOLD: {:?}",
        rows(&bound)
    );
    assert_eq!(rows(&unfolded).len(), rows(&bound).len());
}

/// `LATERAL` is the correlating form, and over an `UNFOLD` it does what the naive
/// `OPTIONAL` above cannot: each left row's OWN operand is expanded.
///
/// `:t`'s plain-string object denotes no composite, so §12.3 keeps its row with
/// `?e` unbound, and `ORDER BY ?e` sorts that unbound cell first.
#[test]
fn lateral_correlates_an_unfold_with_its_left_row() {
    let result = evaluate(
        &graphed(),
        "SELECT ?s ?e WHERE { ?s :list ?c LATERAL { UNFOLD(?c AS ?e) } } ORDER BY ?e",
    );
    assert_eq!(
        rows(&result),
        vec![
            row(&[("s", "<https://example.org/cdt#t>"), ("e", "UNBOUND")]),
            row(&[("s", "<https://example.org/cdt#s>"), ("e", "1")]),
            row(&[("s", "<https://example.org/cdt#s>"), ("e", "2")]),
        ]
    );
}

/// `UNION`: each arm expands its own operand, and the two bags concatenate.
#[test]
fn unfold_composes_with_union() {
    let result = evaluate(
        &empty(),
        "SELECT ?e WHERE { { UNFOLD(cdt:List(1) AS ?e) } UNION { UNFOLD(cdt:List(2, 3) AS ?e) } }",
    );
    assert_eq!(
        rows(&result),
        vec![row(&[("e", "1")]), row(&[("e", "2")]), row(&[("e", "3")])]
    );
}

/// `MINUS` subtracts against the expanded rows, so an expansion is an ordinary
/// left operand.
#[test]
fn unfold_composes_with_minus() {
    let result = evaluate(
        &empty(),
        "SELECT ?e WHERE { UNFOLD(cdt:List(1, 2, 3) AS ?e) \
         MINUS { VALUES ?e { 2 } } }",
    );
    assert_eq!(rows(&result), vec![row(&[("e", "1")]), row(&[("e", "3")])]);
}

/// `GRAPH` scopes the pattern that supplies the operand; the expansion happens
/// inside that scope.
#[test]
fn unfold_composes_with_graph() {
    let result = evaluate(
        &graphed(),
        "SELECT ?e WHERE { GRAPH :g1 { ?s :list ?c UNFOLD(?c AS ?e) } }",
    );
    assert_eq!(rows(&result), vec![row(&[("e", "3")])]);
}

/// `VALUES` supplies the operand column, one expansion per data row.
#[test]
fn unfold_composes_with_values() {
    let result = evaluate(
        &empty(),
        "SELECT ?e WHERE { VALUES ?c { \"[1]\"^^cdt:List \"[2,3]\"^^cdt:List } \
         UNFOLD(?c AS ?e) }",
    );
    assert_eq!(
        rows(&result),
        vec![row(&[("e", "1")]), row(&[("e", "2")]), row(&[("e", "3")])]
    );
}

/// The two nouns are inverses over a list with no nulls: unfold it, fold it back
/// by index, and the original value is reconstructed.
#[test]
fn fold_inverts_unfold_over_a_list() {
    assert!(ask(
        "ASK { { SELECT (FOLD(?e ORDER BY ?i) AS ?back) WHERE { \
         BIND(\"[3,1,2]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?i) } } \
         FILTER(?back = \"[3,1,2]\"^^cdt:List) }"
    ));
    // …and over a map, through the key/value reading.
    assert!(ask("ASK { { SELECT (FOLD(?k, ?v) AS ?back) WHERE { \
         BIND(\"{1:10, 2:20}\"^^cdt:Map AS ?m) UNFOLD(?m AS ?k, ?v) } } \
         FILTER(?back = \"{1:10, 2:20}\"^^cdt:Map) }"));
}

/// A blank node inside a composite is scoped to that literal and shared within
/// it: two occurrences of one label unfold to the SAME node, two labels to two.
#[test]
fn blank_labels_are_scoped_to_the_literal_and_shared_within_it() {
    assert!(ask(
        "ASK { BIND(\"[_:b, _:b]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?i) \
         FILTER(?i = 2) FILTER(SAMETERM(?e, cdt:get(?l, 1))) }"
    ));
    assert!(ask(
        "ASK { BIND(\"[_:b1, _:b2]\"^^cdt:List AS ?l) UNFOLD(?l AS ?e, ?i) \
         FILTER(?i = 2) FILTER(!SAMETERM(?e, cdt:get(?l, 1))) }"
    ));
}
