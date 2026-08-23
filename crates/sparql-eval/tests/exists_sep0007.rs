// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `EXISTS`/`NOT EXISTS` (SEP-0007's `Replace`/`PrjMap` definition — see
//! [`purrdf_sparql_eval::exists_admission_gate`]'s module doc and `crate::enf`'s
//! doc in that crate for the theorem this matrix exercises), end to end, from the
//! vantage a host has: real query text, parsed through
//! [`purrdf_sparql_algebra::SparqlParser`], evaluated through the PUBLIC
//! [`NativeSparqlEngine`] entry points against a fixture dataset — never reaching
//! into either crate. This is the surface-syntax twin of
//! `crates/sparql-eval/src/exists_admission_gate.rs`'s hand-built-algebra matrix:
//! that file proves the probe and the per-row definition AGREE (or, for a refused
//! shape, diverge in the specific way the gate exists to prevent) at the algebra
//! level; this file proves the SAME shapes, written as SPARQL text a host actually
//! submits, answer correctly through the parser and the public engine.
//!
//! # Fixture
//!
//! `PREFIX : <https://example.org/exists#>`. One frozen [`RdfDataset`] serves every
//! test below, organized into per-test-group namespaces (`:aAnchor`/`:aP`/`:aQ` for
//! group A, `:bN`/`:bQ` for group B, …) so no two tests' data can accidentally
//! interact. It carries: two IRI-named graphs (`:cG1`/`:cG2`) plus one
//! BLANK-NODE-named graph (reached only through `GRAPH ?g` enumeration, since a
//! query can never spell a specific dataset blank node in its own text — see
//! group C below); an RDF 1.2 reifier (in fact three, group M, two of them
//! reifying the SAME quoted triple so a `sameTerm` correlation test has both a
//! `true` and a `false` row); a blank-node SUBJECT (group N: `_:nB1`/`_:nB2` each
//! carry their own `:nNote` literal); and enough plain triples that every test
//! below asserts a SPECIFIC row set, most of them a genuine mix of kept and
//! dropped rows (never merely "non-empty").
//!
//! # Adjudicated answers
//!
//! Three tests below required deriving the expected answer from this engine's
//! OWN documented semantics rather than intuition, because SPARQL's official
//! algebra text is exactly the ambiguity SEP-0007 exists to resolve:
//!
//! * [`exists_sep_count_example_answers`] (group E) — the SEP's own COUNT worked
//!   example nests the correlated leaf `?s :eProperty ?w` inside a sub-`SELECT`
//!   that projects only `?C`. Per `crate::expr::substitute_pattern`'s theorem
//!   ("this walk's injection never crosses a `Project` boundary the projection
//!   does not carry") that sub-`SELECT`'s `Project{variables:[C]}` node strips
//!   `?s` out of the injected row BEFORE the walk reaches the `Bgp` leaf beneath
//!   it — so `?C` is the COUNT over the WHOLE (uncorrelated) `:eProperty`
//!   relation, identical for every outer row, not a per-`?s` count. The fixture
//!   is built so the correlated and uncorrelated readings diverge on one row
//!   (`:eS2`), making this the empirically decisive test of which one the engine
//!   implements.
//! * [`exists_sep_disconnected_variable_example`] (group G) and
//!   [`exists_unprojected_outer_variable_stays_unseen`] (group K) both turn on
//!   the identical `Project`-boundary rule, reproduced from the SEP's own Issue-5
//!   example (`SELECT ?y WHERE { ?x :p :c }` never projects `?x`, so `?x` inside
//!   it is, per the theorem, a DIFFERENT — disconnected — `?x` than the outer
//!   scope's, however the two are textually spelled).
//! * [`exists_having_sees_only_the_subselect_scope`] (group L) applies the same
//!   `Project`-narrowing rule to a `HAVING` clause specifically — `crate::enf`'s
//!   module doc calls this position out by name: "PurRDF follows the literal
//!   `PrjMap` reading: a sub-`SELECT`'s `HAVING` filters that sub-`SELECT`'s OWN
//!   scope only… no special correlation channel is invented for it". An outer
//!   name that merely happens to be reused inside an unprojected `HAVING` must
//!   therefore read as UNBOUND there, not as a leak — an unbound comparison is an
//!   error, and `FILTER`/`HAVING` treats an error as `false`.

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest,
    SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    MemoryRelation, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
};

/// The fixture namespace (AGENTS.md: test fixtures live under `example.org`, no
/// minted vocabulary).
const EX: &str = "https://example.org/exists#";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Every query text below opens with this, so `:foo` is legal.
const PFX: &str = "PREFIX : <https://example.org/exists#> \
                    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> ";

fn iri(local: &str) -> String {
    format!("{EX}{local}")
}

fn int_literal(b: &mut RdfDatasetBuilder, n: i64) -> purrdf_core::TermId {
    b.intern_literal(RdfLiteral::typed(n.to_string(), XSD_INTEGER))
}

/// The fixture dataset described in the module doc.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    // ---- Group A: F2 (`OPTIONAL` padding), tests 1-3 --------------------------
    // `:aOa` carries `:aP` but NOT `:aQ`; `:aOb` carries `:aQ` but NOT `:aP` —
    // deliberately crossed, so a test that (wrongly) thought the `OPTIONAL`
    // mattered would predict the OPPOSITE of the correct, `OPTIONAL`-blind answer.
    let a_anchor = b.intern_iri(&iri("aAnchor"));
    let a_p = b.intern_iri(&iri("aP"));
    let a_q = b.intern_iri(&iri("aQ"));
    let a_oa = b.intern_iri(&iri("aOa"));
    let a_ob = b.intern_iri(&iri("aOb"));
    let a_p_obj = b.intern_iri(&iri("aPObj"));
    let a_q_obj = b.intern_iri(&iri("aQObj"));
    let yes = b.intern_literal(RdfLiteral::simple("yes"));
    b.push_quad(a_oa, a_anchor, yes, None);
    b.push_quad(a_oa, a_p, a_p_obj, None);
    b.push_quad(a_ob, a_anchor, yes, None);
    b.push_quad(a_ob, a_q, a_q_obj, None);

    // ---- Group B: nested `NOT EXISTS`, tests 5-6 -------------------------------
    let b_n = b.intern_iri(&iri("bN"));
    let b_q = b.intern_iri(&iri("bQ"));
    let b_na = b.intern_iri(&iri("bNa"));
    let b_nb = b.intern_iri(&iri("bNb"));
    let b_q_obj = b.intern_iri(&iri("bQObj"));
    let one = b.intern_literal(RdfLiteral::simple("1"));
    b.push_quad(b_na, b_n, one, None);
    b.push_quad(b_na, b_q, b_q_obj, None);
    b.push_quad(b_nb, b_n, one, None);

    // ---- Group C: correlated `GRAPH ?g`, test 4 --------------------------------
    // `:cR1`/`:cR2` point at the two IRI-named graphs; `:cR3` points at a BLANK
    // node that is ALSO used as a graph name below — the only way a query can
    // ever bind `?g` to a blank-node graph name, since blank-node identity is
    // never spellable in query text (see the module doc).
    let c_link = b.intern_iri(&iri("cLink"));
    let c_flag = b.intern_iri(&iri("cFlag"));
    let c_r1 = b.intern_iri(&iri("cR1"));
    let c_r2 = b.intern_iri(&iri("cR2"));
    let c_r3 = b.intern_iri(&iri("cR3"));
    let c_g1 = b.intern_iri(&iri("cG1"));
    let c_g2 = b.intern_iri(&iri("cG2"));
    let c_bgraph = b.intern_blank("cBg", BlankScope::DEFAULT);
    let c_marked = b.intern_iri(&iri("cMarked"));
    let c_other = b.intern_iri(&iri("cOther"));
    let yes2 = b.intern_literal(RdfLiteral::simple("yes"));
    let no = b.intern_literal(RdfLiteral::simple("no"));
    b.push_quad(c_r1, c_link, c_g1, None);
    b.push_quad(c_r2, c_link, c_g2, None);
    b.push_quad(c_r3, c_link, c_bgraph, None);
    b.push_quad(c_marked, c_flag, yes2, Some(c_g1));
    b.push_quad(c_other, c_flag, no, Some(c_g2));
    b.push_quad(c_marked, c_flag, yes2, Some(c_bgraph));

    // ---- Group D: fresh `BIND` target inside `EXISTS`, test 7 -----------------
    let d_tag = b.intern_iri(&iri("dTag"));
    let d_val = b.intern_iri(&iri("dVal"));
    let d_s1 = b.intern_iri(&iri("dS1"));
    let d_s2 = b.intern_iri(&iri("dS2"));
    let go = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(d_s1, d_tag, go, None);
    let d_val3 = int_literal(&mut b, 3);
    b.push_quad(d_s1, d_val, d_val3, None);
    b.push_quad(d_s2, d_tag, go, None);
    let d_val10 = int_literal(&mut b, 10);
    b.push_quad(d_s2, d_val, d_val10, None);

    // ---- Group E: the SEP's COUNT worked example, test 8 ----------------------
    // Global `:eProperty` count (across every subject) is 3 (`:eW1`/`:eW2` off
    // `:eS1`, `:eW3` off `:eS3`). `:eS2`'s OWN count is 0 — chosen so the
    // correlated ("own count") and uncorrelated ("global count") readings
    // DIVERGE on it (own: 0 < 2 = true; global: 3 < 2 = false), making this
    // fixture empirically decisive about which one the engine implements — see
    // the module doc's "Adjudicated answers".
    let e_value = b.intern_iri(&iri("eValue"));
    let e_property = b.intern_iri(&iri("eProperty"));
    let e_s1 = b.intern_iri(&iri("eS1"));
    let e_s2 = b.intern_iri(&iri("eS2"));
    let e_s3 = b.intern_iri(&iri("eS3"));
    let e_w1 = b.intern_iri(&iri("eW1"));
    let e_w2 = b.intern_iri(&iri("eW2"));
    let e_w3 = b.intern_iri(&iri("eW3"));
    let e_val10a = int_literal(&mut b, 10);
    b.push_quad(e_s1, e_value, e_val10a, None);
    b.push_quad(e_s1, e_property, e_w1, None);
    b.push_quad(e_s1, e_property, e_w2, None);
    let e_val2 = int_literal(&mut b, 2);
    b.push_quad(e_s2, e_value, e_val2, None);
    let e_val10b = int_literal(&mut b, 10);
    b.push_quad(e_s3, e_value, e_val10b, None);
    b.push_quad(e_s3, e_property, e_w3, None);

    // ---- Group F: the SEP's MINUS disjoint-domain example (Issue 4), test 9 ---
    let f_sep_p = b.intern_iri(&iri("sepP"));
    let f_sep_c = b.intern_iri(&iri("sepC"));
    let f_sep_d = b.intern_iri(&iri("sepD"));
    b.push_quad(f_sep_d, f_sep_p, f_sep_c, None);

    // ---- Group G: the SEP's disconnected-variable example (Issue 5), test 10 --
    // `:gD` (the value the outer `BIND` supplies) has NO `:gP :gC` fact of its
    // own; `:gOther` does. Correct (uncorrelated inner `?x`, per the theorem):
    // TRUE, because `{ SELECT ?y WHERE { ?x :gP :gC } }`'s own `?x` is a fresh,
    // disconnected variable that finds `:gOther`'s fact regardless of what the
    // outer `?x` is bound to.
    let g_p = b.intern_iri(&iri("gP"));
    let g_c = b.intern_iri(&iri("gC"));
    let g_other = b.intern_iri(&iri("gOther"));
    b.push_quad(g_other, g_p, g_c, None);
    // `:gD` and `:gIrrelevant` are referenced only via `BIND` in the query text
    // below — no triple needed for either.

    // ---- Group H: LIMIT/OFFSET/DISTINCT/ORDER BY/GROUP BY family, tests 11-15 -
    // The correlation leaf is `?z :hItem ?s` (object position), so the ITEM
    // entities (`:hI1`/`:hI2`/`:hI3`) are the SUBJECTS of `:hItem`, pointing AT
    // `:hCa`/`:hCb`/`:hCc` — mirroring `exists_admission_gate.rs`'s
    // `probe_would_diverge_on_slice`/`_group` witness shape.
    let h_tag = b.intern_iri(&iri("hTag"));
    let h_item = b.intern_iri(&iri("hItem"));
    let h_ca = b.intern_iri(&iri("hCa"));
    let h_cb = b.intern_iri(&iri("hCb"));
    let h_cc = b.intern_iri(&iri("hCc"));
    let h_i1 = b.intern_iri(&iri("hI1"));
    let h_i2 = b.intern_iri(&iri("hI2"));
    let h_i3 = b.intern_iri(&iri("hI3"));
    let x_val = b.intern_literal(RdfLiteral::simple("x"));
    b.push_quad(h_ca, h_tag, x_val, None);
    b.push_quad(h_i1, h_item, h_ca, None);
    b.push_quad(h_i2, h_item, h_ca, None);
    b.push_quad(h_cb, h_tag, x_val, None);
    b.push_quad(h_i3, h_item, h_cb, None);
    b.push_quad(h_cc, h_tag, x_val, None);

    // ---- Group I: bare, fresh `VALUES` inner, test 16 --------------------------
    let i_tag = b.intern_iri(&iri("iTag"));
    let i_s1 = b.intern_iri(&iri("iS1"));
    let i_s2 = b.intern_iri(&iri("iS2"));
    let i_s3 = b.intern_iri(&iri("iS3"));
    let go2 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(i_s1, i_tag, go2, None);
    let go3 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(i_s2, i_tag, go3, None);
    let go4 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(i_s3, i_tag, go4, None);

    // ---- Group J: bare, uncorrelated sub-`SELECT` inner, test 17 --------------
    let j_tag = b.intern_iri(&iri("jTag"));
    let j_flag = b.intern_iri(&iri("jFlag"));
    let j_s1 = b.intern_iri(&iri("jS1"));
    let j_s2 = b.intern_iri(&iri("jS2"));
    let j_mark = b.intern_iri(&iri("jMark"));
    let go5 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(j_s1, j_tag, go5, None);
    let go6 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(j_s2, j_tag, go6, None);
    let yes3 = b.intern_literal(RdfLiteral::simple("yes"));
    b.push_quad(j_mark, j_flag, yes3, None);

    // ---- Group K: unprojected outer variable, the narrowing pin, test 18 -------
    let k_mark = b.intern_iri(&iri("kMark"));
    let k_q = b.intern_iri(&iri("q"));
    let k_m1 = b.intern_iri(&iri("kM1"));
    let k_m2 = b.intern_iri(&iri("kM2"));
    let k_qval = b.intern_iri(&iri("qVal"));
    let keep = b.intern_literal(RdfLiteral::simple("keep"));
    b.push_quad(k_m1, k_mark, keep, None);
    b.push_quad(k_m1, k_q, k_qval, None);
    let keep2 = b.intern_literal(RdfLiteral::simple("keep"));
    b.push_quad(k_m2, k_mark, keep2, None);

    // ---- Group L: the `HAVING`-scope pin, test 19 ------------------------------
    let l_count = b.intern_iri(&iri("lCount"));
    let l_item = b.intern_iri(&iri("lItem"));
    let l_d1 = b.intern_iri(&iri("lD1"));
    let l_d2 = b.intern_iri(&iri("lD2"));
    let l_g1 = b.intern_iri(&iri("lG1"));
    let l_i1 = b.intern_iri(&iri("lI1"));
    let l_val1 = int_literal(&mut b, 1);
    b.push_quad(l_d1, l_count, l_val1, None);
    let l_val10 = int_literal(&mut b, 10);
    b.push_quad(l_d2, l_count, l_val10, None);
    b.push_quad(l_g1, l_item, l_i1, None);

    // ---- Group M: quoted-triple outer binding, test 20 -------------------------
    // TWO reifiers name the SAME statement (`:mA :mP :mOa`); ONE reifier names a
    // DIFFERENT one, alone — the `true`/`false` split for a `sameTerm` "is there
    // ANOTHER reifier of this exact quoted triple" correlation.
    let m_p = b.intern_iri(&iri("mP"));
    let m_a = b.intern_iri(&iri("mA"));
    let m_b = b.intern_iri(&iri("mB"));
    let m_oa = b.intern_iri(&iri("mOa"));
    let m_ob = b.intern_iri(&iri("mOb"));
    b.push_quad(m_a, m_p, m_oa, None);
    b.push_quad(m_b, m_p, m_ob, None);
    let stmt_a = b.intern_triple(m_a, m_p, m_oa);
    let stmt_b = b.intern_triple(m_b, m_p, m_ob);
    let reifier_m1a = b.intern_iri(&iri("reifierM1a"));
    let reifier_m1b = b.intern_iri(&iri("reifierM1b"));
    let reifier_m2 = b.intern_iri(&iri("reifierM2"));
    b.push_reifier(reifier_m1a, stmt_a);
    b.push_reifier(reifier_m1b, stmt_a);
    b.push_reifier(reifier_m2, stmt_b);

    // ---- Group N: blank-node outer binding, test 21 ---------------------------
    // `_:nB1` is reachable via BOTH `:nHasAnon` (from `:nS1`) and `:nAlias` (from
    // `:nLinker`); `_:nB2` (from `:nS2`) is reachable ONLY via `:nHasAnon` — the
    // `true`/`false` split. Both blanks are also SUBJECTS of their own `:nNote`
    // triple (the fixture's blank-node-subject requirement).
    let n_has_anon = b.intern_iri(&iri("nHasAnon"));
    let n_alias = b.intern_iri(&iri("nAlias"));
    let n_note = b.intern_iri(&iri("nNote"));
    let n_s1 = b.intern_iri(&iri("nS1"));
    let n_s2 = b.intern_iri(&iri("nS2"));
    let n_linker = b.intern_iri(&iri("nLinker"));
    let n_b1 = b.intern_blank("nB1", BlankScope::DEFAULT);
    let n_b2 = b.intern_blank("nB2", BlankScope::DEFAULT);
    b.push_quad(n_s1, n_has_anon, n_b1, None);
    b.push_quad(n_s2, n_has_anon, n_b2, None);
    b.push_quad(n_linker, n_alias, n_b1, None);
    let b1note = b.intern_literal(RdfLiteral::simple("b1note"));
    b.push_quad(n_b1, n_note, b1note, None);
    let b2note = b.intern_literal(RdfLiteral::simple("b2note"));
    b.push_quad(n_b2, n_note, b2note, None);

    // ---- Group O: `GRAPH <fixed-iri>` with a correlated body, test 22 ---------
    let o_marker = b.intern_iri(&iri("oMarker"));
    let o_item = b.intern_iri(&iri("oItem"));
    let o_x1 = b.intern_iri(&iri("oX1"));
    let o_x2 = b.intern_iri(&iri("oX2"));
    let o_y1 = b.intern_iri(&iri("oY1"));
    let o_g = b.intern_iri(&iri("oG"));
    let go7 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(o_x1, o_marker, go7, None);
    let go8 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(o_x2, o_marker, go8, None);
    b.push_quad(o_x1, o_item, o_y1, Some(o_g));

    // ---- Group P: property-function inner, test 23 ----------------------------
    let p_marker = b.intern_iri(&iri("pMarker"));
    let p_s1 = b.intern_iri(&iri("pS1"));
    let p_s2 = b.intern_iri(&iri("pS2"));
    let go9 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(p_s1, p_marker, go9, None);
    let go10 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(p_s2, p_marker, go10, None);

    // ---- Group Q: three-level alternating-polarity nesting, test 24 -----------
    let q_mid = b.intern_iri(&iri("qMid"));
    let q_leaf = b.intern_iri(&iri("qLeaf"));
    let q_anchor = b.intern_iri(&iri("qAnchor"));
    let t_marker = b.intern_iri(&iri("tMarker"));
    let t_s1 = b.intern_iri(&iri("tS1"));
    let t_s2 = b.intern_iri(&iri("tS2"));
    let t_leaf_obj = b.intern_iri(&iri("tLeafObj"));
    let one2 = b.intern_literal(RdfLiteral::simple("1"));
    b.push_quad(q_anchor, q_mid, one2, None);
    let go11 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(t_s1, t_marker, go11, None);
    b.push_quad(t_s1, q_leaf, t_leaf_obj, None);
    let go12 = b.intern_literal(RdfLiteral::simple("go"));
    b.push_quad(t_s2, t_marker, go12, None);

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

/// [`run`], with a property-function registry injected (group P only).
fn run_with_registry(
    ds: &Arc<RdfDataset>,
    query_body: &str,
    registry: &PropertyFunctionRegistry,
) -> SparqlResult {
    let text = format!("{PFX}{query_body}");
    NativeSparqlEngine::new()
        .query_with_options_view(
            &**ds,
            request(&text),
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|e| panic!("query failed: {e:?}\nquery: {text}"))
}

/// Render one bound cell the way [`row`]/[`rows`] key it: `<iri>` for an IRI, the
/// bare lexical form for a literal (every literal fixture value here is either a
/// plain string or an unadorned integer, so the lexical form alone disambiguates),
/// `_:label` for a blank node, `TRIPLE` for a quoted triple term (its exact
/// components are never asserted on, only its IDENTITY via `sameTerm`, so a
/// coarse tag is enough), `UNBOUND` for `None`.
fn cell(value: Option<&TermValue>) -> String {
    match value {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(i)) => format!("<{i}>"),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
        Some(TermValue::Triple { .. }) => "TRIPLE".to_owned(),
    }
}

type Row = BTreeMap<String, String>;

/// A SELECT result's rows as variable-name-keyed maps, SORTED — every assertion
/// in this file compares solutions as a SET (SPARQL's multiset order is
/// unspecified outside an explicit top-level `ORDER BY`, which none of the outer
/// queries below use), never by column or row position.
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

// ===========================================================================
// Group A: F2 (`OPTIONAL` padding), tests 1-3.
// ===========================================================================

/// `EXISTS { OPTIONAL { ?x :q ?y } }` — `crate::enf`'s Law 1 (THE F2 FIX BY LAW)
/// erases the `LeftJoin` to its LEFT operand, here the identity pattern (an empty
/// `Bgp`, always non-empty) — so `EXISTS` is `true` for EVERY row REGARDLESS of
/// whether the `OPTIONAL` itself ever matches. `:aOb` carries no `:aP` at all and
/// (crucially) is fully independent of `:aQ` too — its row must survive anyway.
#[test]
fn exists_optional_padding_answers_per_spec() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?x :aAnchor \"yes\" FILTER EXISTS { OPTIONAL { ?x :aQ ?y } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("x", "<https://example.org/exists#aOa>")]),
            row(&[("x", "<https://example.org/exists#aOb>")])
        ]),
        "the F2 shape pads EVERY row true, `:aOa` (no `:aQ` fact at all) included"
    );
}

/// The `NOT EXISTS` twin of [`exists_optional_padding_answers_per_spec`]: since
/// `EXISTS { OPTIONAL {…} }` is always `true`, `NOT EXISTS { OPTIONAL {…} }` is
/// always `false` — every row that reaches the filter is dropped.
#[test]
fn not_exists_optional_padding_does_not_fabricate() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?x :aAnchor \"yes\" FILTER NOT EXISTS { OPTIONAL { ?x :aQ ?y } } }",
    );
    assert_eq!(
        rows(&result),
        Vec::<Row>::new(),
        "the negated F2 shape must drop every row, never fabricate one"
    );
}

/// `EXISTS { ?a :p ?b OPTIONAL { ?x :q ?y } }` — the OFF-identity variant: the
/// `LeftJoin`'s LEFT operand is now a real `Bgp` (`?x :aP ?b`, correlated to the
/// outer row), so Law 1 erases the `OPTIONAL` to nothing and the answer tracks
/// SOLELY whether `?x` has an `:aP` edge — `:aQ`'s presence/absence is provably
/// irrelevant, which the crossed fixture (`:aOa` has `:aP` but not `:aQ`; `:aOb`
/// has `:aQ` but not `:aP`) makes observable: a reader who thought the `OPTIONAL`
/// mattered would predict the OPPOSITE answer.
#[test]
fn exists_optional_beside_a_bgp_answers_per_spec() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?x :aAnchor \"yes\" FILTER EXISTS { ?x :aP ?b OPTIONAL { ?x :aQ ?y } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("x", "<https://example.org/exists#aOa>")])]),
        "only `:aOa` (has `:aP`) survives — `:aOb` has `:aQ` but no `:aP`, and the \
         `OPTIONAL` cannot rescue it"
    );
}

// ===========================================================================
// Group B: nested `NOT EXISTS`, tests 5-6.
// ===========================================================================

/// `EXISTS { FILTER NOT EXISTS { ?x :q ?y } }` — the inner group is a lone
/// `Filter` over the identity pattern, correlated through `?x`: `true` iff `?x`
/// has NO `:bQ` edge.
#[test]
fn exists_nested_not_exists_correlates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?x :bN \"1\" FILTER EXISTS { FILTER NOT EXISTS { ?x :bQ ?y } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("x", "<https://example.org/exists#bNb>")])]),
        "`:bNb` (no `:bQ` edge) is the only row whose inner NOT EXISTS is true"
    );
}

/// The outer-negated twin: `NOT EXISTS { FILTER NOT EXISTS { ?x :q ?y } }` is a
/// double negation, `true` iff `?x` DOES have a `:bQ` edge.
#[test]
fn not_exists_nested_not_exists_correlates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?x :bN \"1\" FILTER NOT EXISTS { FILTER NOT EXISTS { ?x :bQ ?y } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("x", "<https://example.org/exists#bNa>")])]),
        "`:bNa` (has a `:bQ` edge) is the only row the double negation keeps"
    );
}

// ===========================================================================
// Group C: correlated `GRAPH ?g`, test 4.
// ===========================================================================

/// `?s :cLink ?g FILTER EXISTS { GRAPH ?g { SELECT * {…} LIMIT 1 } }` — a `Slice`
/// sits inside a `Graph`, which `crate::enf`'s spine NEVER recurses through (it is
/// classified alongside `Join`/`Filter`/`Minus`/`LeftJoin`, not the transparent
/// wrappers), so the `LIMIT 1` is never erased; `crate::governor::soundness`'s
/// `admissible_rec` refuses EVERY `Slice` unconditionally, so this shape is always
/// classified inadmissible and answered via the per-row DEFINITION path — the
/// shape this test names. `:cR3` correlates `?g` to a BLANK-node graph name (the
/// only way one is ever reachable, since a blank node cannot be spelled in query
/// text) and must restrict correctly too, exercising `Graph`'s
/// blank-node-binding-keeps-`?g`-a-variable path documented in
/// `crate::expr::substitute_pattern`.
#[test]
fn exists_correlated_graph_variable_e2e() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?r :cLink ?g \
         FILTER EXISTS { GRAPH ?g { SELECT * { ?m :cFlag \"yes\" } LIMIT 1 } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("r", "<https://example.org/exists#cR1>"),
                ("g", "<https://example.org/exists#cG1>")
            ]),
            row(&[("r", "<https://example.org/exists#cR3>"), ("g", "_:cBg")]),
        ]),
        "`:cG1` and the blank-named graph both carry a `:cFlag \"yes\"` fact; \
         `:cG2` carries only `:cFlag \"no\"` and correctly drops `:cR2`'s row"
    );
}

// ===========================================================================
// Group D: fresh `BIND` target inside `EXISTS`, test 7.
// ===========================================================================

/// `EXISTS { ?s :dVal ?n BIND(?n * 2 AS ?doubled) FILTER(?doubled > 10) }` — a
/// `BIND` target that never collides with anything the outer query names
/// (`crate::governor::soundness::admissible_rec`'s `Extend` arm: fresh unless an
/// outer-visible name), so this is admissible and evaluated correctly by whichever
/// path the gate selects. `:dS1`'s value doubles to 6 (not `> 10`); `:dS2`'s
/// doubles to 20.
#[test]
fn exists_bind_fresh_variable_evaluates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :dTag \"go\" \
         FILTER EXISTS { ?s :dVal ?n BIND(?n * 2 AS ?doubled) FILTER(?doubled > 10) } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("s", "<https://example.org/exists#dS2>")])]),
        "only `:dS2` (value 10, doubled 20) clears the `> 10` bar"
    );
}

// ===========================================================================
// Group E: the SEP's COUNT worked example, test 8.
// ===========================================================================

/// The SEP-0007 document's own worked example: an outer value compared against a
/// `COUNT(*)` computed by a correlated sub-`SELECT`.
///
/// # Adjudication
///
/// The sub-`SELECT`'s own `Project{variables:[C]}` node does not list `?s`, so
/// per `crate::expr::substitute_pattern`'s theorem the per-row substitution never
/// reaches the `Bgp` leaf `?s :eProperty ?w` beneath it — `?C` is the COUNT over
/// the WHOLE `:eProperty` relation (3, from `:eS1`'s two facts and `:eS3`'s one),
/// identical for every outer row, not a per-`?s` count. The fixture is built so a
/// (WRONG, per-subject-correlated) reading would answer differently on `:eS2`
/// (own count 0, `0 < 2` = true) than the correct global reading (`3 < 2` =
/// false), making the two hypotheses empirically distinguishable rather than
/// merely asserted.
#[test]
fn exists_sep_count_example_answers() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :eValue ?v \
         FILTER EXISTS { { SELECT (COUNT(*) AS ?C) { ?s :eProperty ?w } } FILTER(?C < ?v) } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s", "<https://example.org/exists#eS1>"), ("v", "10")]),
            row(&[("s", "<https://example.org/exists#eS3>"), ("v", "10")]),
        ]),
        "the global `:eProperty` count is 3: `3 < 10` holds for `:eS1`/`:eS3`, but \
         `3 < 2` fails for `:eS2` — a per-subject (wrongly correlated) count would \
         instead have kept `:eS2` (own count 0, `0 < 2`)"
    );
}

// ===========================================================================
// Group F: the SEP's MINUS disjoint-domain example (Issue 4), test 9.
// ===========================================================================

/// The SEP-0007 document's own Issue-4 example, reproduced with this fixture's
/// namespace (`:sepP`/`:sepC`/`:sepD` standing in for the SEP's `:p`/`:c`/`:d`):
/// `SELECT ?x { ?x :p :c FILTER EXISTS { ?x :p :c MINUS { ?x :p :c } } }` over a
/// dataset holding only `:d :p :c`.
///
/// A naive CONSTANT-substitution reading turns both `MINUS` operands into the
/// SAME ground (variable-free) pattern, so both solution sets have the EMPTY
/// domain — SPARQL's `MINUS` never subtracts across disjoint domains, so a
/// zero-domain `MINUS` wrongly answers non-empty (the "flip" the SEP names).
/// PurRDF's Values-Insertion keeps `?x` a real, SHARED column on both sides (see
/// `crate::expr::substitute_pattern`'s doc, "It also fixes the MINUS
/// disjoint-domain flip"), so the domains overlap, the row IS subtracted, and the
/// correct (non-flipped) answer is the EMPTY result set — proven end to end here
/// exactly as `crate::exists_admission_gate`'s `probe_would_diverge_on_minus` and
/// `lateral_e2e.rs`'s `lateral_minus_rhs_disjoint_domain_not_flipped` prove it at
/// the algebra/LATERAL layers.
#[test]
fn exists_sep_minus_example_answers() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT ?x { ?x :sepP :sepC FILTER EXISTS { ?x :sepP :sepC MINUS { ?x :sepP :sepC } } }",
    );
    assert_eq!(
        rows(&result),
        Vec::<Row>::new(),
        "the correct (non-flipped) answer is empty: `?x = :sepD` is always \
         subtracted by its own identical right-hand match"
    );
}

// ===========================================================================
// Group G: the SEP's disconnected-variable example (Issue 5), test 10.
// ===========================================================================

/// The SEP-0007 document's own Issue-5 example, reproduced in shape (the SEP's
/// own pseudo-syntax elides the extra `{ }` a real `SubSelect` needs as a
/// `BIND` sibling; SPARQL's grammar requires it): `SELECT ?x { BIND(:gD AS ?x)
/// FILTER EXISTS { BIND(:gIrrelevant AS ?z) { SELECT ?y WHERE { ?x :gP :gC } } } }`.
///
/// # Adjudication
///
/// The inner `SELECT ?y` never projects `?x`, so — per the SAME `Project`-
/// boundary theorem [`exists_sep_count_example_answers`] turns on — the inner
/// `?x` is a DIFFERENT (disconnected) variable from the outer `?x`, however
/// identically spelled. The fixture makes this decisive: `:gD` (the outer BOUND
/// value) has NO `:gP :gC` fact, but `:gOther` does — so a WRONGLY-correlated
/// reading (naive substitution, exactly the hazard the SEP's Issue 5 names) would
/// answer `false` (`:gD` has no such fact), while the correct, disconnected
/// reading answers `true` (the inner, unconstrained `?x` finds `:gOther`'s fact).
/// `BIND(:gIrrelevant AS ?z)` is included verbatim from the SEP's own text
/// specifically to prove it is harmless: a wholly unrelated fresh `BIND` sitting
/// beside the disconnected sub-`SELECT` does not perturb the answer at all.
#[test]
fn exists_sep_disconnected_variable_example() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT ?x { BIND(:gD AS ?x) \
         FILTER EXISTS { BIND(:gIrrelevant AS ?z) { SELECT ?y WHERE { ?x :gP :gC } } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("x", "<https://example.org/exists#gD>")])]),
        "the inner, unprojected `?x` is disconnected from the outer `?x = :gD`: it \
         finds `:gOther`'s (unrelated) `:gP :gC` fact and the row survives, even \
         though `:gD` itself carries no such fact — the exact divergence the SEP's \
         Issue 5 names, and the harmless `BIND(:gIrrelevant AS ?z)` beside it \
         changes nothing"
    );
}

// ===========================================================================
// Group H: LIMIT/OFFSET/DISTINCT/ORDER BY/GROUP BY family, tests 11-15.
// ===========================================================================
//
// Shared data: `:hCa` carries two `:hItem` facts, `:hCb` one, `:hCc` none.

/// `EXISTS { SELECT * { ?z :hItem ?s } LIMIT 1 }` — `crate::enf`'s Law 4a erases
/// an offset-zero `Slice` with room for at least one row ON THE SPINE, so this
/// reduces to the plain existence check before `probe_admissible` ever sees a
/// `Slice` node at all.
#[test]
fn exists_over_limit_one_subselect() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :hTag \"x\" FILTER EXISTS { SELECT * { ?z :hItem ?s } LIMIT 1 } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s", "<https://example.org/exists#hCa>")]),
            row(&[("s", "<https://example.org/exists#hCb>")]),
        ]),
        "`:hCa` (two items) and `:hCb` (one item) both have at least one match; \
         `:hCc` (none) correctly drops"
    );
}

/// `EXISTS { SELECT * { ?z :hItem ?s } OFFSET 1 }` — a restricting-offset `Slice`
/// is NEVER erased by `crate::enf` regardless of position, and
/// `admissible_rec`'s `Slice` arm refuses unconditionally — the DEFINITION path,
/// committed per outer row: `:hCa` has TWO matches (offset 1 leaves one, still
/// non-empty), `:hCb` has exactly ONE (offset 1 empties it).
#[test]
fn exists_over_offset_subselect_commits_per_row() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :hTag \"x\" FILTER EXISTS { SELECT * { ?z :hItem ?s } OFFSET 1 } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("s", "<https://example.org/exists#hCa>")])]),
        "only `:hCa` (two matches) survives an OFFSET of 1; `:hCb` (one match) is \
         emptied by it and `:hCc` (none) was already empty"
    );
}

/// `EXISTS { SELECT DISTINCT * { ?z :hItem ?s } }` — Law 3 erases a spine-level
/// `DISTINCT` (de-duplication can only remove rows, never emptiness itself).
#[test]
fn exists_over_distinct_subselect() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :hTag \"x\" FILTER EXISTS { SELECT DISTINCT * { ?z :hItem ?s } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s", "<https://example.org/exists#hCa>")]),
            row(&[("s", "<https://example.org/exists#hCb>")]),
        ]),
        "DISTINCT never changes emptiness: same answer as the plain existence check"
    );
}

/// `EXISTS { SELECT * { ?z :hItem ?s } ORDER BY ?z }` — Law 2 erases a spine-level
/// `ORDER BY` (sorting is a permutation, never a filter).
#[test]
fn exists_over_order_by_subselect() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :hTag \"x\" FILTER EXISTS { SELECT * { ?z :hItem ?s } ORDER BY ?z } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s", "<https://example.org/exists#hCa>")]),
            row(&[("s", "<https://example.org/exists#hCb>")]),
        ]),
        "ORDER BY never changes emptiness: same answer as the plain existence check"
    );
}

/// `EXISTS { SELECT ?s (COUNT(*) AS ?c) { ?w :hItem ?s } GROUP BY ?s
/// HAVING(COUNT(*) >= 2) }` — UNLIKE [`exists_sep_count_example_answers`], `?s`
/// IS listed in the inner `SELECT`'s own projection (it is the `GROUP BY` key),
/// so the `Project`-boundary theorem keeps it in the injected row all the way to
/// the `Bgp` leaf: this IS a genuine per-`?s` correlated count.
/// `admissible_rec`'s `Group` arm refuses unconditionally, so this is always the
/// DEFINITION path. The `HAVING` clause re-states the aggregate expression
/// (`COUNT(*)`) rather than referencing the `?c` alias the outer `SELECT`
/// assigns it to: `HAVING` runs against the `Group`'s own output, BEFORE the
/// alias-assigning `Extend`/`Project` around it — `?c` is the alias, not yet a
/// bound variable, at the point `HAVING` reads it (confirmed empirically: `?c`
/// alone in `HAVING` here is unbound, not the count), so `HAVING(?c >= 2)` would
/// be an unbound-operand error on every group and answer nothing regardless of
/// the true counts — [`exists_having_sees_only_the_subselect_scope`] is the
/// dedicated test for exactly that error-as-false discipline, so this test uses
/// the form that actually reads the count.
#[test]
fn exists_over_group_by_subselect_correlates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :hTag \"x\" \
         FILTER EXISTS { SELECT ?s (COUNT(*) AS ?c) { ?w :hItem ?s } GROUP BY ?s HAVING(COUNT(*) >= 2) } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("s", "<https://example.org/exists#hCa>")])]),
        "only `:hCa` (two `:hItem` facts, count 2) clears the `>= 2` bar; `:hCb` \
         (count 1) and `:hCc` (grouping by `?s` over zero rows yields ZERO groups, \
         not one group with count 0) both correctly drop"
    );
}

// ===========================================================================
// Group I: bare, fresh `VALUES` inner, test 16.
// ===========================================================================

/// `EXISTS { VALUES ?w { "a" "b" } }` — `?w` is fresh (shares no name with the
/// outer query), so `admissible_rec`'s `Values` arm admits it; a non-empty,
/// uncorrelated `VALUES` block is `true` for every row regardless of outer
/// content — SEP-0007 Issue 1's bare `toMultiSet` shape.
#[test]
fn exists_bare_values_inner() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :iTag \"go\" FILTER EXISTS { VALUES ?w { \"a\" \"b\" } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s", "<https://example.org/exists#iS1>")]),
            row(&[("s", "<https://example.org/exists#iS2>")]),
            row(&[("s", "<https://example.org/exists#iS3>")]),
        ]),
        "an uncorrelated, non-empty VALUES block keeps every outer row"
    );
}

// ===========================================================================
// Group J: bare, uncorrelated sub-`SELECT` inner, test 17.
// ===========================================================================

/// `EXISTS { SELECT ?anything { ?m :jFlag "yes" } }` — no variable name is shared
/// with the outer query at all, so this is trivially uncorrelated regardless of
/// admissibility; it is `true` for every outer row because `:jMark :jFlag "yes"`
/// exists somewhere in the dataset.
#[test]
fn exists_bare_subselect_inner() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :jTag \"go\" FILTER EXISTS { SELECT ?anything { ?m :jFlag \"yes\" } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s", "<https://example.org/exists#jS1>")]),
            row(&[("s", "<https://example.org/exists#jS2>")]),
        ]),
        "an uncorrelated, non-empty bare sub-SELECT keeps every outer row"
    );
}

// ===========================================================================
// Group K: unprojected outer variable, the narrowing pin, test 18.
// ===========================================================================

/// `FILTER EXISTS { SELECT ?a { ?s2 :q ?a } }` with the outer row ALSO binding a
/// variable literally named `?s2` — the exact `Project`-narrowing shape (SEP
/// Issue 5's own hazard, reproduced generically) `crate::exists_admission_gate`'s
/// module doc pins as the narrowing non-regression case. `:kM1` has a `:q` fact;
/// `:kM2` does not, but BOTH rows must answer IDENTICALLY (since the inner `?s2`
/// is disconnected) — a leak would instead show `:kM1` true and `:kM2` false.
#[test]
fn exists_unprojected_outer_variable_stays_unseen() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s2 :kMark \"keep\" FILTER EXISTS { SELECT ?a { ?s2 :q ?a } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[("s2", "<https://example.org/exists#kM1>")]),
            row(&[("s2", "<https://example.org/exists#kM2>")]),
        ]),
        "both rows must answer the SAME (true): the inner `?s2` is disconnected \
         from the outer `?s2`, so `:kM1`'s own `:q` fact makes the unconstrained \
         inner non-empty for EVERY outer row, `:kM2` included — a leaking \
         implementation would instead have dropped `:kM2`"
    );
}

// ===========================================================================
// Group L: the `HAVING`-scope pin, test 19.
// ===========================================================================

/// `FILTER EXISTS { SELECT ?g { ?g :lItem ?w } GROUP BY ?g HAVING(?c < 100) }`
/// with the outer row ALSO binding `?c` (to a small, `< 100` integer). Per
/// `crate::enf`'s own "The `HAVING` position" doc, `HAVING` is just another
/// expression inside the sub-`SELECT`'s scope, subject to the SAME
/// `Project`-narrowing rule as everything else: the inner `SELECT ?g` never lists
/// `?c`, so `?c` inside `HAVING` is UNBOUND, not leaked from outer — an unbound
/// `<` comparison is an error, and `FILTER`/`HAVING` treats an error as `false`,
/// dropping the one group `:lG1`'s `:lItem` fact would otherwise have formed. The
/// two outer rows carry DIFFERENT `?c` values (1 and 10) specifically so a
/// leaking implementation's answer would have been VALUE-DEPENDENT (both `< 100`,
/// so it would have kept both) rather than uniformly `false`.
#[test]
fn exists_having_sees_only_the_subselect_scope() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?d :lCount ?c \
         FILTER EXISTS { SELECT ?g { ?g :lItem ?w } GROUP BY ?g HAVING(?c < 100) } }",
    );
    assert_eq!(
        rows(&result),
        Vec::<Row>::new(),
        "the outer `?c` never reaches `HAVING`'s own (disconnected) scope, so the \
         comparison is always an unbound-operand error — every row is dropped, \
         regardless of `?c`'s actual value"
    );
}

// ===========================================================================
// Group M: quoted-triple outer binding, test 20.
// ===========================================================================

/// `?r1 rdf:reifies ?tt FILTER EXISTS { ?r2 rdf:reifies ?x FILTER(sameTerm(?tt,
/// ?x) && ?r2 != ?r1) }` — `?tt` is bound to a QUOTED TRIPLE TERM, referenced
/// ONLY in an expression position (`sameTerm`), the term kind `Expression` has no
/// constant spelling for (`crate::expr::substitute_expr`'s doc): it must be
/// carried by `wrap_with_expr_term_only_values`, exactly as
/// `lateral_e2e.rs`'s `lateral_triple_term_bound_variable_in_a_filter` proves for
/// `LATERAL`. `:mA :mP :mOa` is reified TWICE (`:reifierM1a`/`:reifierM1b`); `:mB
/// :mP :mOb` is reified once, alone.
#[test]
fn exists_quoted_triple_outer_binding_correlates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?r1 rdf:reifies ?tt \
         FILTER EXISTS { ?r2 rdf:reifies ?x FILTER(sameTerm(?tt, ?x) && ?r2 != ?r1) } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![
            row(&[
                ("r1", "<https://example.org/exists#reifierM1a>"),
                ("tt", "TRIPLE")
            ]),
            row(&[
                ("r1", "<https://example.org/exists#reifierM1b>"),
                ("tt", "TRIPLE")
            ]),
        ]),
        "`:reifierM1a`/`:reifierM1b` each find the OTHER as a second reifier of the \
         identical quoted triple; `:reifierM2`, the sole reifier of a DIFFERENT \
         statement, finds no other and correctly drops"
    );
}

// ===========================================================================
// Group N: blank-node outer binding, test 21.
// ===========================================================================

/// `?s :nHasAnon ?bn FILTER EXISTS { ?other :nAlias ?bn2 FILTER(sameTerm(?bn,
/// ?bn2)) }` — `?bn` is bound to a BLANK NODE, the other term kind `Expression`
/// has no constant spelling for, referenced ONLY inside the `sameTerm` filter —
/// the `LATERAL` analogue is `lateral_e2e.rs`'s
/// `lateral_bnode_bound_variable_in_a_filter`. `_:nB1` (via `:nS1`) is ALSO
/// reachable via `:nAlias` (from `:nLinker`); `_:nB2` (via `:nS2`) is not.
#[test]
fn exists_blank_node_outer_binding_correlates() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :nHasAnon ?bn \
         FILTER EXISTS { ?other :nAlias ?bn2 FILTER(sameTerm(?bn, ?bn2)) } }",
    );
    let solutions = rows(&result);
    assert_eq!(
        solutions.len(),
        1,
        "only `:nS1`'s blank node (`_:nB1`, aliased) survives; got {solutions:?}"
    );
    let r = &solutions[0];
    assert_eq!(r["s"], "<https://example.org/exists#nS1>");
    assert!(
        r["bn"].starts_with("_:"),
        "?bn must still be the outer row's own blank-node term, got {:?}",
        r["bn"]
    );
}

// ===========================================================================
// Group O: `GRAPH <fixed-iri>` with a correlated body, test 22.
// ===========================================================================

/// `FILTER EXISTS { GRAPH <…oG> { ?x :oItem ?y } }` — a FIXED graph name (not a
/// variable); `admissible_rec`'s `Graph` arm is name-blind and defers entirely to
/// its inner pattern, which is a plain, correlated `Bgp`. `:oX1` has an `:oItem`
/// edge INSIDE graph `:oG`; `:oX2` has none there.
#[test]
fn exists_graph_iri_name_with_correlated_body() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?x :oMarker \"go\" \
         FILTER EXISTS { GRAPH <https://example.org/exists#oG> { ?x :oItem ?y } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("x", "<https://example.org/exists#oX1>")])]),
        "only `:oX1` has an `:oItem` edge inside `:oG`; `:oX2` correctly drops"
    );
}

// ===========================================================================
// Group P: property-function inner, test 23.
// ===========================================================================

/// A test property function registered exactly the way
/// `property_function_e2e.rs` registers one: a single (subject-bound) relation
/// mapping `:pS1` to `"matched"`, `:pS2` to nothing. `admissible_rec`'s
/// `PropertyFunction` arm refuses unconditionally, so this is always evaluated by
/// the per-row DEFINITION path, which substitutes the outer `?s` binding into the
/// call's subject argument BEFORE dispatch (`crate::expr::substitute_pattern`'s
/// doc, "Property-function arguments") — exactly the mechanism
/// `crate::exists_admission_gate`'s `probe_would_diverge_on_property_function`
/// documents as the reason a bare call is unconditionally inadmissible in the
/// first place.
fn pf_registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        iri("pf"),
        Arc::new(
            MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri(iri("pS1")),
                    TermValue::typed_literal("matched", "http://www.w3.org/2001/XMLSchema#string"),
                ]],
            )
            .expect("fixture: one row, two values wide"),
        ),
    );
    registry
}

#[test]
fn exists_property_function_inner_sees_outer_binding() {
    let ds = dataset();
    let registry = pf_registry();
    let result = run_with_registry(
        &ds,
        "SELECT * { ?s :pMarker \"go\" FILTER EXISTS { (?s) :pf (?out) } }",
        &registry,
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("s", "<https://example.org/exists#pS1>")])]),
        "only `:pS1` is a subject the relation matches; `:pS2` correctly drops"
    );
}

// ===========================================================================
// Group Q: three-level alternating-polarity nesting, test 24.
// ===========================================================================

/// `EXISTS { ?m1 :qMid "1" FILTER NOT EXISTS { ?m2 :qMid "1" FILTER EXISTS { ?s
/// :qLeaf ?v } } }` — three alternating levels, correlated SOLELY through the
/// innermost `EXISTS`'s `?s`. Both `?m1`/`?m2` anchors are always satisfiable
/// (`:qAnchor :qMid "1"`), so algebraically this reduces to `Filter(!leaf(?s),
/// anchor) `inside `Filter(!!leaf(?s), anchor)` — i.e. the overall answer is
/// `true` iff `?s` does NOT have a `:qLeaf` edge (the innermost `EXISTS` is
/// negated exactly once, by the single `NOT EXISTS` at the middle level).
#[test]
fn exists_triple_nesting_alternating_polarity() {
    let ds = dataset();
    let result = run(
        &ds,
        "SELECT * { ?s :tMarker \"go\" \
         FILTER EXISTS { ?m1 :qMid \"1\" \
           FILTER NOT EXISTS { ?m2 :qMid \"1\" FILTER EXISTS { ?s :qLeaf ?v } } } }",
    );
    assert_eq!(
        rows(&result),
        expect(vec![row(&[("s", "<https://example.org/exists#tS2>")])]),
        "`:tS1` has a `:qLeaf` edge, so the innermost EXISTS is true, the middle \
         NOT EXISTS is false, and the outer EXISTS is false — dropped. `:tS2` has \
         none, flipping all the way back to true — kept"
    );
}

// ===========================================================================
// Matrix completeness.
// ===========================================================================

/// The number of `#[test]` functions this file is contracted to carry — the
/// 24 enumerated `EXISTS`/`NOT EXISTS` shapes
/// (`exists_optional_padding_answers_per_spec` through
/// `exists_triple_nesting_alternating_polarity`, tests 1-24 above) plus this
/// completeness check itself. [`matrix_is_complete`] does not compare this
/// constant against a copy of itself: it re-reads this file's OWN source text
/// via `include_str!` and counts the lines that are exactly a bare `#[test]`
/// attribute, then checks that LIVE count against the number below.
/// A reviewer who deletes or merges one of the 24 shape tests shrinks the
/// live count while this constant stays put, so the comparison fails —
/// a build a reviewer cannot pass by leaving the matrix silently shrunk.
const EXPECTED_TESTS: usize = 25;

#[test]
fn matrix_is_complete() {
    let live_tests = include_str!("exists_sep0007.rs")
        .lines()
        .filter(|line| line.trim() == "#[test]")
        .count();
    assert_eq!(
        live_tests, EXPECTED_TESTS,
        "this file is contracted to carry exactly 24 EXISTS/NOT EXISTS shapes \
         (tests 1-24 in the module doc's enumeration) plus this completeness \
         check itself — {EXPECTED_TESTS} `#[test]` functions in total; the \
         file's own source text now has {live_tests}. If a shape test was \
         deleted or merged, either restore it or update the module doc's \
         enumeration and this constant together"
    );
}
