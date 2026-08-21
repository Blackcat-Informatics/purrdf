// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `LATERAL` (SEP-0006), end to end, from the vantage a host has: real query text,
//! parsed through [`purrdf_sparql_algebra::SparqlParser`], evaluated through the
//! PUBLIC [`NativeSparqlEngine`] entry points against a fixture dataset — never
//! reaching into either crate.
//!
//! # Fixture
//!
//! `PREFIX : <https://example.org/lateral#>`. `:a` and `:b` each carry `:p` to an
//! object and TWO `rdfs:label` values; `:z` carries `:p` but no label at all (the
//! SEP's own "labelless subject" example). `:a`/`:b` also carry `:hasChild` edges
//! to `:c1`/`:c2`/`:c3`, each with an `:name` literal. A handful of narrower
//! predicates (`:q`, `:tagged`, `:usesPred`) exist purely to isolate one
//! term-kind or position for one test each; two named graphs (`:g1`/`:g2`) hold a
//! shared subject `:x` under DIFFERENT labels, for the `GRAPH ?g` scoping test.
//! `:a` also carries `:hasAnon` to one blank node (which carries an `:note`
//! literal of its own), and one RDF 1.2 reifier `:reifierA` reifies the `:a :p
//! :oa` triple as a quoted triple term — both exist to bind an outer variable to
//! a term kind an `Expression` has no constant spelling for (a blank node, a
//! quoted triple), for the expression-only correlated-substitution tests. Every
//! sub-select below carries `ORDER BY` before `LIMIT 1`, per the SEP's own
//! examples.

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest,
    SparqlResult, TermValue,
};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions};

/// The fixture namespace (AGENTS.md: test fixtures live under `example.org`, no
/// minted vocabulary).
const EX: &str = "https://example.org/lateral#";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

/// Every query text below opens with this, so `:foo`/`rdfs:bar` are legal.
const PFX: &str = "PREFIX : <https://example.org/lateral#> \
                    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> ";

fn iri(local: &str) -> String {
    format!("{EX}{local}")
}

/// The fixture dataset described in the module doc.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&iri("p"));
    let label = b.intern_iri(RDFS_LABEL);
    let has_child = b.intern_iri(&iri("hasChild"));
    let name = b.intern_iri(&iri("name"));
    let q = b.intern_iri(&iri("q"));
    let c_obj = b.intern_iri(&iri("c"));
    let tagged = b.intern_iri(&iri("tagged"));
    let uses_pred = b.intern_iri(&iri("usesPred"));
    let uses_graph = b.intern_iri(&iri("usesGraph"));
    let has_subject = b.intern_iri(&iri("hasSubject"));

    let a = b.intern_iri(&iri("a"));
    let bb = b.intern_iri(&iri("b"));
    let z = b.intern_iri(&iri("z"));
    let oa = b.intern_iri(&iri("oa"));
    let ob = b.intern_iri(&iri("ob"));
    let oz = b.intern_iri(&iri("oz"));

    b.push_quad(a, p, oa, None);
    b.push_quad(bb, p, ob, None);
    b.push_quad(z, p, oz, None);

    let a1 = b.intern_literal(RdfLiteral::simple("a1"));
    let a2 = b.intern_literal(RdfLiteral::simple("a2"));
    let b1 = b.intern_literal(RdfLiteral::simple("b1"));
    let b2 = b.intern_literal(RdfLiteral::simple("b2"));
    b.push_quad(a, label, a1, None);
    b.push_quad(a, label, a2, None);
    b.push_quad(bb, label, b1, None);
    b.push_quad(bb, label, b2, None);

    let c1 = b.intern_iri(&iri("c1"));
    let c2 = b.intern_iri(&iri("c2"));
    let c3 = b.intern_iri(&iri("c3"));
    b.push_quad(a, has_child, c1, None);
    b.push_quad(a, has_child, c2, None);
    b.push_quad(bb, has_child, c3, None);
    let n1 = b.intern_literal(RdfLiteral::simple("Carol1"));
    let n2 = b.intern_literal(RdfLiteral::simple("Carol2"));
    let n3 = b.intern_literal(RdfLiteral::simple("Carol3"));
    b.push_quad(c1, name, n1, None);
    b.push_quad(c2, name, n2, None);
    b.push_quad(c3, name, n3, None);

    // `:a :q :c` only — used by the MINUS-under-substitution and
    // shared-variable-constant-injection tests.
    b.push_quad(a, q, c_obj, None);

    // Literal-injection: two tags carrying LITERAL values that match two of the
    // `rdfs:label` literals above exactly.
    let tag1 = b.intern_iri(&iri("tag1"));
    let tag2 = b.intern_iri(&iri("tag2"));
    let tagged_a1 = b.intern_literal(RdfLiteral::simple("a1"));
    let tagged_b2 = b.intern_literal(RdfLiteral::simple("b2"));
    b.push_quad(tag1, tagged, tagged_a1, None);
    b.push_quad(tag2, tagged, tagged_b2, None);

    // Predicate-position injection: `:a` names `:p` as "its own" predicate.
    b.push_quad(a, uses_pred, p, None);

    // GRAPH-scoping: `:x` carries DIFFERENT labels in two DIFFERENT named
    // graphs, and two "callers" each name which graph they want.
    let r1 = b.intern_iri(&iri("r1"));
    let r2 = b.intern_iri(&iri("r2"));
    let g1 = b.intern_iri(&iri("g1"));
    let g2 = b.intern_iri(&iri("g2"));
    let x = b.intern_iri(&iri("x"));
    b.push_quad(r1, uses_graph, g1, None);
    b.push_quad(r1, has_subject, x, None);
    b.push_quad(r2, uses_graph, g2, None);
    b.push_quad(r2, has_subject, x, None);
    let alpha1 = b.intern_literal(RdfLiteral::simple("alpha1"));
    let alpha2 = b.intern_literal(RdfLiteral::simple("alpha2"));
    let beta1 = b.intern_literal(RdfLiteral::simple("beta1"));
    let beta2 = b.intern_literal(RdfLiteral::simple("beta2"));
    b.push_quad(x, label, alpha1, Some(g1));
    b.push_quad(x, label, alpha2, Some(g1));
    b.push_quad(x, label, beta1, Some(g2));
    b.push_quad(x, label, beta2, Some(g2));

    // Blank-node injection: `:a` carries a blank node that itself carries a
    // literal, so an outer row can bind a variable to a `TermValue::Blank` —
    // the term kind an `Expression` cannot spell as a constant — for the
    // `BOUND(?bn)`-in-a-`LATERAL`-`FILTER` test.
    let has_anon = b.intern_iri(&iri("hasAnon"));
    let anon = b.intern_blank("anon1", BlankScope::DEFAULT);
    let note = b.intern_iri(&iri("note"));
    let noted = b.intern_literal(RdfLiteral::simple("noted"));
    b.push_quad(a, has_anon, anon, None);
    b.push_quad(anon, note, noted, None);

    // RDF 1.2 reification: `:reifierA` reifies `:a :p :oa` as a quoted triple
    // term, so `?r rdf:reifies ?tt` binds `?tt` to a `TermValue::Triple` — the
    // other term kind an `Expression` cannot spell as a constant — for the
    // `sameTerm`-in-a-`LATERAL`-`FILTER` test.
    let reifier_a = b.intern_iri(&iri("reifierA"));
    let statement_a = b.intern_triple(a, p, oa);
    b.push_reifier(reifier_a, statement_a);

    b.freeze().expect("freeze fixture")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn run(ds: &Arc<RdfDataset>, query_body: &str) -> SparqlResult {
    let text = format!("{PFX}{query_body}");
    NativeSparqlEngine::new()
        .query(ds, request(&text))
        .unwrap_or_else(|e| panic!("query failed: {e:?}\nquery: {text}"))
}

/// Render one bound cell the way [`row`]/[`rows`] key it: `<iri>` for an IRI, the
/// bare lexical form for a literal (every literal fixture value here is a plain
/// string, so the lexical form alone disambiguates), `_:label` for a blank node,
/// `UNBOUND` for `None`.
fn cell(value: Option<&TermValue>) -> String {
    match value {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(i)) => format!("<{i}>"),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
        Some(other) => format!("{other:?}"),
    }
}

type Row = BTreeMap<String, String>;

/// A SELECT result's rows as variable-name-keyed maps, SORTED — every assertion
/// in this file compares solutions as a set (SPARQL's multiset order is
/// unspecified outside an explicit top-level `ORDER BY`, which none of these
/// queries use), never by column or row position.
fn rows(result: &SparqlResult) -> Vec<Row> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("expected a SELECT result, got {result:?}");
    };
    let mut out: Vec<Row> = rows
        .iter()
        .map(|row| {
            variables
                .iter()
                .cloned()
                .zip(row.iter().map(|c| cell(c.as_ref())))
                .collect()
        })
        .collect();
    out.sort();
    out
}

/// Build one expected row from `(variable, rendered-value)` pairs.
fn row(pairs: &[(&str, &str)]) -> Row {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// Sort a hand-written expected row set the same way [`rows`] sorts the actual
/// one, so `assert_eq!` compares two SETS rather than two SEQUENCES.
fn expect(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort();
    rows
}

// ---------------------------------------------------------------------------
// 1. Top-1-per-group, plus its plain-join control (the laterality proof).
// ---------------------------------------------------------------------------

/// `LATERAL`'s right side is driven by EACH left row: every subject gets its
/// OWN smallest label. A plain (non-`LATERAL`) join instead evaluates the
/// sub-select ONCE, unconstrained, so only the single globally-smallest label
/// survives, joined against whichever subject it happens to belong to.
#[test]
fn lateral_subselect_star_is_top_1_per_group() {
    let ds = dataset();
    let lateral = run(
        &ds,
        "SELECT * { ?s :p ?o \
         LATERAL { SELECT * { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } }",
    );
    assert_eq!(
        rows(&lateral),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("label", "a1")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("label", "b1")
            ]),
        ]),
        "each subject gets its OWN top-1 label — :z has none and drops (inner-join semantics)"
    );

    let control = run(
        &ds,
        "SELECT * { ?s :p ?o \
         { SELECT * { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } }",
    );
    assert_eq!(
        rows(&control),
        expect(vec![row(&[
            ("s", "<https://example.org/lateral#a>"),
            ("o", "<https://example.org/lateral#oa>"),
            ("label", "a1")
        ])]),
        "the plain join evaluates the sub-select ONCE: only the globally-smallest \
         label (:a's \"a1\") survives, joined only against :a"
    );

    assert_ne!(
        rows(&lateral),
        rows(&control),
        "LATERAL and a plain join over the identical sub-select text must disagree \
         — this IS the laterality the parser now makes writable"
    );
}

// ---------------------------------------------------------------------------
// 2. The scoping oracle: a non-projected sub-select variable is NOT correlated.
// ---------------------------------------------------------------------------

/// When the sub-select projects only `?label` (not `?s`), the `Project`
/// boundary narrows the injected row to nothing the sub-select can see: it
/// evaluates exactly as the uncorrelated control above does, for every left
/// row alike.
#[test]
fn lateral_subselect_nonprojected_var_is_not_correlated() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o \
         LATERAL { SELECT ?label { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("label", "a1")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("label", "a1")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#z>"),
                ("o", "<https://example.org/lateral#oz>"),
                ("label", "a1")
            ]),
        ]),
        "the SAME globally-smallest label (\"a1\") is cross-joined onto EVERY left \
         row, :z included — the sub-select never saw ?s"
    );

    let star = run(
        &ds,
        "SELECT * { ?s :p ?o \
         LATERAL { SELECT * { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } }",
    );
    assert_ne!(
        rows(&result),
        rows(&star),
        "projecting ?s (the star form) vs not (this form) must disagree — the \
         scoping oracle for Project-boundary narrowing"
    );
}

// ---------------------------------------------------------------------------
// 3. SEP ex.2: OPTIONAL pads the labelless subject instead of dropping it.
// ---------------------------------------------------------------------------

#[test]
fn lateral_optional_rhs_pads_labelless_subject() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o \
         LATERAL { OPTIONAL { SELECT * { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("label", "a1")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("label", "b1")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#z>"),
                ("o", "<https://example.org/lateral#oz>"),
                ("label", "UNBOUND")
            ]),
        ]),
        ":z has no label at all, and OPTIONAL is what keeps its row (padded, not dropped)"
    );
}

// ---------------------------------------------------------------------------
// 4. A shared IRI variable is injected as a constant at a triple leaf.
// ---------------------------------------------------------------------------

#[test]
fn lateral_shared_variable_is_injected_as_constant() {
    let ds = dataset();
    let result = run(&ds, "SELECT * { ?s :p ?o LATERAL { ?s :q ?qval } }");
    assert_eq!(
        rows(&result),
        expect(vec![row(&[
            ("s", "<https://example.org/lateral#a>"),
            ("o", "<https://example.org/lateral#oa>"),
            ("qval", "<https://example.org/lateral#c>")
        ])]),
        "only :a carries :q, and ?s must have been injected as the constant :a \
         (not left free, which would also have matched nothing else here anyway \
         — the point is :b/:z correctly produce NO row)"
    );
}

// ---------------------------------------------------------------------------
// 5. A UNION right-hand side substitutes into BOTH branches.
// ---------------------------------------------------------------------------

#[test]
fn lateral_union_rhs_substitutes_both_branches() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o LATERAL { { ?s :q ?x } UNION { ?s :hasChild ?x } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("x", "<https://example.org/lateral#c>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("x", "<https://example.org/lateral#c1>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("x", "<https://example.org/lateral#c2>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("x", "<https://example.org/lateral#c3>")
            ]),
        ]),
        "both UNION branches must see ?s substituted: :a matches its OWN :q branch \
         AND its OWN :hasChild branch, :b only its :hasChild branch, :z neither"
    );
}

// ---------------------------------------------------------------------------
// 6. A nested LATERAL re-enters the correlated-substitution guard.
// ---------------------------------------------------------------------------

#[test]
fn nested_lateral_reenters_the_substitution_guard() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o \
         LATERAL { ?s :hasChild ?c LATERAL { ?c :name ?n } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("c", "<https://example.org/lateral#c1>"),
                ("n", "Carol1")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("c", "<https://example.org/lateral#c2>"),
                ("n", "Carol2")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("c", "<https://example.org/lateral#c3>"),
                ("n", "Carol3")
            ]),
        ]),
        "the OUTER Lateral's per-row substitution wraps an INNER Lateral, which \
         must itself re-enter (not corrupt) the correlated-evaluation guard for \
         EACH of the outer's own left rows"
    );
}

// ---------------------------------------------------------------------------
// 7. GRAPH ?g scoped per row.
// ---------------------------------------------------------------------------

/// `:x` carries DIFFERENT labels in two DIFFERENT named graphs. `:r1`/`:r2` each
/// name which graph and which subject they want, via a shared `?g` that is
/// ALREADY bound on `LATERAL`'s left before the right-hand side runs.
///
/// This test was written to catch a hypothesized gap: `substitute_pattern`'s
/// `Graph` arm leaves a variable graph NAME unresolved rather than resolving
/// it from μ the way `Service`'s variable endpoint does (see that function's
/// doc). The hypothesis was that an unresolved `?g`, combined with a
/// `LIMIT`-bearing inner, would let the `LIMIT` truncate a merged multi-graph
/// scan before the caller's own compatibility filter (on `?g`) ever ran,
/// keeping a row from the WRONG graph and dropping a left row outright.
///
/// It does not happen, and this test — passing with `Graph`'s name left
/// exactly as unresolved as it always was — is why: `GRAPH ?g { P }`'s OWN
/// semantics are a per-graph UNION, not a merge-then-scan. The evaluator
/// evaluates `P` (here including its `ORDER BY … LIMIT 1`) SEPARATELY within
/// EACH named graph and unions the per-graph results with `?g` bound to that
/// graph — so the `LIMIT` was already correctly scoped to one graph at a
/// time regardless of whether `?g` arrives pre-bound. An unresolved `?g` at
/// substitution time therefore costs an evaluation of every OTHER graph's
/// (locally correct, merely irrelevant) branch, which the row's own
/// compatibility check on `?g` then discards — a performance question, not
/// a correctness one (confirmed by running `GRAPH ?g { … LIMIT 1 }` with
/// `?g` genuinely free at authoring time: it already yields one row per
/// graph, each already scoped to that graph's own `LIMIT 1`). So `Service`'s
/// "resolve the constant, because a joined VALUES
/// row cannot supply it" rationale does not transfer to `Graph`: a `VALUES`
/// row is exactly what the caller's own post-hoc compatibility filter
/// already amounts to here, because the per-graph union never let another
/// graph's rows cross the `LIMIT` boundary in the first place.
#[test]
fn lateral_graph_rhs_is_scoped_per_row() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT ?r ?g ?label { ?r :usesGraph ?g . ?r :hasSubject ?s \
         LATERAL { GRAPH ?g { SELECT * { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("r", "<https://example.org/lateral#r1>"),
                ("g", "<https://example.org/lateral#g1>"),
                ("label", "alpha1")
            ]),
            row(&[
                ("r", "<https://example.org/lateral#r2>"),
                ("g", "<https://example.org/lateral#g2>"),
                ("label", "beta1")
            ]),
        ]),
        "BOTH rows must survive, each reading its OWN row's graph: :r1's top \
         label from :g1 (\"alpha1\"), :r2's from :g2 (\"beta1\") — each \
         graph's LIMIT 1 is scoped to that graph alone by GRAPH's own \
         per-graph-union semantics, independent of whether ?g arrived \
         pre-bound"
    );
}

// ---------------------------------------------------------------------------
// 8. A property PATH leaf is correlated, not just a plain triple leaf.
// ---------------------------------------------------------------------------

#[test]
fn lateral_path_rhs_is_correlated() {
    let ds = dataset();
    let result = run(&ds, "SELECT * { ?s :p ?o LATERAL { ?s :hasChild+ ?d } }");
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("d", "<https://example.org/lateral#c1>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("d", "<https://example.org/lateral#c2>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("d", "<https://example.org/lateral#c3>")
            ]),
        ]),
        "a `Path` leaf (not a `Bgp`) must be correlated the same way: :z, which \
         has no children, correctly drops"
    );
}

// ---------------------------------------------------------------------------
// 9. A LITERAL-bound outer variable is injected (not just an IRI).
// ---------------------------------------------------------------------------

#[test]
fn lateral_literal_bound_variable_is_injected() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s rdfs:label ?label LATERAL { ?tag :tagged ?label } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("label", "a1"),
                ("tag", "<https://example.org/lateral#tag1>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("label", "b2"),
                ("tag", "<https://example.org/lateral#tag2>")
            ]),
        ]),
        "the LITERAL value bound to ?label must have been injected into the \
         `:tagged` leaf as a constant — \"a2\" and \"b1\" have no matching \
         :tagged triple and correctly produce no row"
    );
}

// ---------------------------------------------------------------------------
// 10. A variable in PREDICATE position is injected.
// ---------------------------------------------------------------------------

#[test]
fn lateral_predicate_position_is_injected() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :usesPred ?pred LATERAL { ?s ?pred ?val } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[
            ("s", "<https://example.org/lateral#a>"),
            ("pred", "<https://example.org/lateral#p>"),
            ("val", "<https://example.org/lateral#oa>")
        ])]),
        "?pred (a PREDICATE-position variable) must have been injected as the \
         constant :p, so the leaf reads exactly :a's :p triple"
    );
}

// ---------------------------------------------------------------------------
// 11. A bare VALUES right-hand side re-evaluates (cross-joins) per left row.
// ---------------------------------------------------------------------------

#[test]
fn lateral_values_rhs_joins_per_row() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o LATERAL { VALUES ?tag { \"x\" \"y\" } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("tag", "x")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("tag", "y")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("tag", "x")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("tag", "y")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#z>"),
                ("o", "<https://example.org/lateral#oz>"),
                ("tag", "x")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#z>"),
                ("o", "<https://example.org/lateral#oz>"),
                ("tag", "y")
            ]),
        ]),
        "an uncorrelated VALUES right-hand side is a cross join: every left row \
         paired with every VALUES row"
    );
}

// ---------------------------------------------------------------------------
// 12. A BIND right-hand side referencing the shared variable (expression path).
// ---------------------------------------------------------------------------

#[test]
fn lateral_bind_rhs_shared_var() {
    let ds = dataset();
    let result = run(&ds, "SELECT * { ?s :p ?o LATERAL { BIND(?o AS ?o2) } }");
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>"),
                ("o2", "<https://example.org/lateral#oa>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>"),
                ("o2", "<https://example.org/lateral#ob>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#z>"),
                ("o", "<https://example.org/lateral#oz>"),
                ("o2", "<https://example.org/lateral#oz>")
            ]),
        ]),
        "BIND's expression-position substitution (unchanged since before this \
         task) must still see ?o's value"
    );
}

// ---------------------------------------------------------------------------
// 13. Truncation commits per left row: a partial block is never emitted short.
// ---------------------------------------------------------------------------

/// A fuel ceiling tight enough to stop mid-execution: whatever rows survive
/// must form WHOLE per-subject label blocks (0 or 2 — never a 1-of-2
/// fragment), because [`crate::binop::eval_lateral`]'s per-left-row loop
/// discards a left row's ENTIRE right-hand block when that row's evaluation
/// trips rather than emitting it short (`binop.rs`'s own "commit granularity"
/// doc). The exact fuel boundary is not this crate's contract to pin — a scan
/// over every ceiling between "nothing runs" and "everything runs" is, and it
/// is run here so the test tracks the evaluator's real cost accounting
/// instead of a number transcribed once and left to rot.
#[test]
fn lateral_truncation_commits_per_left_row() {
    let ds = dataset();
    let query = format!("{PFX}SELECT * {{ ?s :p ?o LATERAL {{ ?s rdfs:label ?label }} }}");
    let engine = NativeSparqlEngine::new();

    let complete = engine
        .query_governed(
            &ds,
            request(&query),
            QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .expect("a metered run must not error");
    let GovernedOutcome::Complete {
        result, evidence, ..
    } = complete
    else {
        panic!("METERED bounds nothing, so this must complete");
    };
    let full = rows(&result);
    assert_eq!(full.len(), 4, "2 labels each for :a and :b, none for :z");
    let spend = evidence.consumed.get(purrdf_core::ResourceDimension::Fuel);
    assert!(spend > 0, "evaluating LATERAL is not free");

    // Every fuel ceiling from 1 up to (but excluding) the full spend that
    // produces a NONEMPTY, budget-exhausted result: each one's rows, grouped
    // by subject, must be an exact multiple of the subject's full 2-row block
    // — never a lone fragment. (A trip can still be reported after every ROW
    // has already been committed — some bookkeeping after the last row can
    // consume the remaining fuel — so "rows.len() == full.len()" here is not
    // itself a contradiction; the SEPARATE `saw_strictly_partial` below is
    // what proves the scan actually caught a mid-run cut, not just the
    // trailing bookkeeping.)
    let mut saw_nonempty = false;
    let mut saw_strictly_partial = false;
    for fuel in 1..spend {
        let outcome = engine
            .query_governed(
                &ds,
                request(&query),
                QueryOptions::EMPTY,
                &QueryGovernors::UNBOUNDED.with_fuel(fuel),
            )
            .expect("a governor trip is an outcome, never an error");
        let GovernedOutcome::BudgetExhausted(exhausted) = outcome else {
            continue;
        };
        let Some(partial) = exhausted.partial.result() else {
            continue;
        };
        let SparqlResult::Solutions {
            variables, rows, ..
        } = partial.result()
        else {
            panic!("expected solutions");
        };
        if rows.is_empty() {
            continue;
        }
        saw_nonempty = true;
        if rows.len() < full.len() {
            saw_strictly_partial = true;
        }
        let s_index = variables
            .iter()
            .position(|v| v == "s")
            .expect("?s is projected");
        let mut by_subject: BTreeMap<String, usize> = BTreeMap::new();
        for r in rows {
            *by_subject.entry(cell(r[s_index].as_ref())).or_insert(0) += 1;
        }
        for (s, count) in &by_subject {
            assert_eq!(
                *count, 2,
                "fuel={fuel}: subject {s} contributed {count} row(s) under the \
                 trip — a left row's block must be admitted whole or not at \
                 all, never split"
            );
        }
    }
    assert!(
        saw_nonempty,
        "no fuel ceiling between 1 and the full spend ({spend}) produced a \
         nonempty partial result — the scan found nothing to check"
    );
    assert!(
        saw_strictly_partial,
        "no fuel ceiling between 1 and the full spend ({spend}) left a subject's \
         block out entirely — the scan never caught a genuine mid-run cut"
    );
}

// ---------------------------------------------------------------------------
// 14. The RHS-introduced variable is a stable column of the output schema.
// ---------------------------------------------------------------------------

#[test]
fn lateral_rhs_schema_union_is_stable() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o LATERAL { ?s rdfs:label ?label } }",
    );
    let SparqlResult::Solutions { variables, .. } = &result else {
        panic!("expected solutions");
    };
    assert!(
        variables.iter().any(|v| v == "label"),
        "the LATERAL right-hand side's own variable must be a column of the \
         output schema (the union AC2's rejected parenthetical would have \
         denied): got {variables:?}"
    );
    let rendered = rows(&result);
    assert!(
        rendered
            .iter()
            .any(|r| r.get("label") != Some(&"UNBOUND".to_owned())),
        "and it must actually carry real values, not merely be declared: {rendered:?}"
    );
}

// ---------------------------------------------------------------------------
// 15. The MINUS disjoint-domain flip, fixed — LIVE on the real answer.
// ---------------------------------------------------------------------------

#[test]
fn lateral_minus_rhs_disjoint_domain_not_flipped() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o LATERAL { ?s :q :c MINUS { ?s :q :c } } }",
    );
    assert_eq!(
        rows(&result),
        Vec::<Row>::new(),
        "MINUS must see the SAME domain on both sides (the substituted leaf \
         keeps its ?s column rather than being rewritten to a bare constant), \
         so :a's :q :c match is correctly subtracted by its own identical \
         right-hand match, and :b/:z never matched the left side at all — the \
         true answer is empty on EVERY row"
    );
}

// ---------------------------------------------------------------------------
// 16 & 17. LATERAL × EXISTS, both directions.
// ---------------------------------------------------------------------------

/// A `FILTER EXISTS` written INSIDE a `LATERAL` right-hand side must see ?s
/// already substituted (the walk's doc: a nested `EXISTS`'s inner pattern gets
/// the FULL row, not just the expression-position half).
#[test]
fn exists_inside_lateral_rhs_correlates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :p ?o LATERAL { FILTER EXISTS { ?s :q :c } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[
            ("s", "<https://example.org/lateral#a>"),
            ("o", "<https://example.org/lateral#oa>")
        ])]),
        "only :a has a :q :c triple, so only :a's EXISTS check must pass"
    );
}

/// A `LATERAL` written INSIDE a `FILTER EXISTS`'s inner pattern, correlated to
/// the outer row SOLELY through a triple position (never an expression) —
/// this is the widened correlation-detection reachability the `LATERAL`
/// surface's own parser creates (see `crate::expr::exists`'s
/// `is_row_sensitive` branch): the inner
/// carries a `LIMIT 1` sub-select, so evaluating it once, unconstrained, and
/// merely probing per outer row (the pre-widening fast path) would let ONE
/// globally-smallest-label subject win the `EXISTS` for every outer row that
/// happens to share nothing else — dropping the true answer for every OTHER
/// subject that also has a label.
#[test]
fn lateral_inside_exists_pattern_evaluates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT ?s ?o { ?s :p ?o \
         FILTER EXISTS { LATERAL { SELECT * { ?s rdfs:label ?label } ORDER BY ?label LIMIT 1 } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("s", "<https://example.org/lateral#a>"),
                ("o", "<https://example.org/lateral#oa>")
            ]),
            row(&[
                ("s", "<https://example.org/lateral#b>"),
                ("o", "<https://example.org/lateral#ob>")
            ]),
        ]),
        ":a AND :b both have a label (so EXISTS must be true for BOTH), :z has \
         none — an unwidened, uncorrelated EXISTS would keep only :a (whichever \
         subject wins the GLOBAL LIMIT-1 race, here \"a1\" < \"b1\") and wrongly \
         drop :b"
    );
}

// ---------------------------------------------------------------------------
// 18. A LATERAL spine, adversarially deep: typed error or bounded eval, never
//     a stack abort.
// ---------------------------------------------------------------------------

/// Chained (sibling, not nested-brace) `LATERAL` clauses build a LEFT-DEEP
/// algebra spine whose STRUCTURAL depth is unrelated to the parser's
/// brace-nesting guard (`group_pattern_depth`, which pops back down between
/// sibling elements at the same textual level and never sees this growth).
/// The evaluator's own nesting guard
/// (`crate::governor::soundness::validate_graph_pattern_depth`) walks the
/// PARSED ALGEBRA structurally with an explicit stack (no native recursion,
/// so it cannot itself overflow) and is what must catch this before any
/// recursive evaluation begins.
#[test]
fn lateral_spine_depth_is_bounded_or_errors() {
    let n = purrdf_sparql_algebra::MAX_GRAPH_PATTERN_DEPTH + 64;
    let mut body = String::from("SELECT * WHERE { ?s :q ?o ");
    for _ in 0..n {
        body.push_str("LATERAL { ?a :q ?b } ");
    }
    body.push('}');
    let text = format!("{PFX}{body}");

    // The parser's own brace-depth guard does not bound a sibling spine: this
    // must parse (each `LATERAL { … }` is one level below the enclosing
    // group, popped back before the next sibling is parsed).
    let query = SparqlParser::new()
        .parse_query(&text)
        .expect("a sibling LATERAL spine is not brace-nested and must parse");

    // Evaluating it must be a typed refusal or a bounded (in-process, non-
    // aborting) answer — never a stack overflow. Reaching this line at all
    // (the test process did not abort) already demonstrates the "never a
    // stack abort" half; the typed-error half is asserted explicitly.
    let ds = dataset();
    let outcome = NativeSparqlEngine::new().query(&ds, request(&text));
    match outcome {
        Err(e) => {
            let message = format!("{e:?}");
            assert!(
                message.to_lowercase().contains("depth") || message.to_lowercase().contains("nest"),
                "a depth refusal should name depth/nesting, got: {message}"
            );
        }
        Ok(result) => {
            // A bounded (non-erroring) evaluator is equally acceptable — but it
            // must actually have produced a genuine, finite answer, not hung.
            let _ = rows(&result);
        }
    }
    let _ = query;
}

// ---------------------------------------------------------------------------
// 19. An outer binding with no `Expression` constant form (blank node, quoted
//     triple), referenced ONLY in a `LATERAL` RHS expression position — the
//     wrap `wrap_with_expr_term_only_values` adds so `substitute_expr`'s
//     IRI/literal-only rewrite is not the only carrier into an expression.
// ---------------------------------------------------------------------------

/// `?bn` is bound to a BLANK NODE by the outer pattern and referenced ONLY
/// inside the `LATERAL` RHS's `FILTER(BOUND(?bn))` — no leaf occurrence of
/// `?bn` anywhere in the RHS for Values Insertion's leaf join to carry it.
/// Before the fix, `substitute_expr`'s `Bound` arm consulted only
/// `row.expr` (which has no entry for a blank-node binding — `Expression`
/// has no constant spelling for one), so the rewritten `BOUND(?bn)` stayed
/// literally `BOUND(?bn)` and was evaluated against a RHS solution row where
/// `?bn` is genuinely absent: `BOUND` of an absent variable is `false`, the
/// `FILTER` drops the row, and the whole outer row is lost even though it
/// binds `?bn`. `:a` is the only subject with a `:hasAnon` blank node, so
/// exactly one row must survive.
#[test]
fn lateral_bnode_bound_variable_in_a_filter() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :hasAnon ?bn LATERAL { ?s :p ?o FILTER(BOUND(?bn)) } }",
    );
    let solutions = rows(&result);
    assert_eq!(
        solutions.len(),
        1,
        "the outer row's blank-node binding must survive a LATERAL RHS FILTER \
         that references it only in an expression position; got {solutions:?}"
    );
    let r = &solutions[0];
    assert_eq!(r["s"], "<https://example.org/lateral#a>");
    assert_eq!(r["o"], "<https://example.org/lateral#oa>");
    assert!(
        r["bn"].starts_with("_:"),
        "?bn must still be the outer row's blank-node term, got {:?}",
        r["bn"]
    );
}

/// `?tt` is bound to a QUOTED TRIPLE TERM (via the RDF 1.2 `rdf:reifies`
/// virtual predicate) by the outer pattern and referenced ONLY inside the
/// `LATERAL` RHS's `FILTER(sameTerm(?tt, ?x))` — no leaf occurrence of `?tt`
/// in the RHS (the RHS's own reifies-pattern binds a DIFFERENT variable,
/// `?x`, to the same triple term). Before the fix, `substitute_expr` had no
/// `Expression` constant form for a quoted triple either, so `?tt` was
/// absent from the RHS solution row `sameTerm` evaluated against: an absent
/// operand is an evaluation error, `FILTER` drops the row, and the single
/// candidate row (the dataset has exactly one reifier) is lost.
#[test]
fn lateral_triple_term_bound_variable_in_a_filter() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { \
         ?r1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ?tt \
         LATERAL { \
           ?r2 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ?x \
           FILTER(sameTerm(?tt, ?x)) \
         } }",
    );
    let solutions = rows(&result);
    assert_eq!(
        solutions.len(),
        1,
        "the outer row's quoted-triple-term binding must survive a LATERAL RHS \
         FILTER that references it only in an expression position; got {solutions:?}"
    );
    let r = &solutions[0];
    assert_eq!(r["r1"], "<https://example.org/lateral#reifierA>");
    assert_eq!(r["r2"], "<https://example.org/lateral#reifierA>");
    assert_eq!(
        r["tt"], r["x"],
        "sameTerm(?tt, ?x) only kept this row because the two ARE the same \
         quoted triple term — their rendered cells must match"
    );
}
