// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every deep-nesting shape a request can write, a hundred thousand levels deep, answered
//! end to end through [`NativeSparqlEngine`] on a thread with a 128 KiB stack.
//!
//! A 128 KiB thread holds a few hundred frames of any recursive walk, so an evaluation
//! that answers here at a hundred thousand levels walks its nesting on the heap. What is
//! asserted is the answer, never merely the absence of a crash: each shape is built over
//! data whose answer is known by construction, computed here without the engine
//! ([`Vector::expected`]), and wherever the shape allows it the answer at one depth
//! differs from the answer at the next, so an evaluation that dropped or duplicated a
//! level answers wrongly. Such shapes run at both 100 000 and 100 001 levels.
//!
//! The same constructed answers are checked at depths one to four on an ordinary thread
//! ([`every_vector_answers_its_constructed_value_at_shallow_depths`]), which is what makes
//! the deep expectation a statement about the shape rather than about this file; and
//! every deep request is parsed, copied, compared and dropped on the 128 KiB thread
//! ([`every_deep_request_parses_copies_compares_and_drops_on_a_128_kib_thread`]), the
//! part of the request path in front of the evaluator.
//!
//! A chain of SPARQL-bodied user-defined functions is not among the vectors: its depth is
//! bounded at 32 calls, a termination bound for mutually recursive bodies, not a stack
//! bound.
//!
//! A nested in-process `SERVICE` forwards its whole body as query text at every level,
//! and every level parses what it is sent: the text a chain of `n` levels serializes and
//! parses grows with the square of `n`.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, SparqlRequest, SparqlResult,
    TermValue,
};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{EvalOptions, InProcessServiceResolver, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The stack every deep vector is evaluated on.
const SMALL_STACK: usize = 128 * 1024;

/// The depth every deep vector is written at.
const DEPTH: usize = 100_000;

// ── Data ───────────────────────────────────────────────────────────────────────────

/// `:s1 … :s4`, each `:p` its own number; only `:s1` also `:q :o1`; and one `:a :link :b`.
fn dataset() -> Arc<RdfDataset> {
    subjects_with_q(&["s1"])
}

/// [`dataset`], with `:q :o{n}` on exactly the named subjects.
fn subjects_with_q(with_q: &[&str]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for n in 1..=4 {
        let name = format!("s{n}");
        let s = builder.intern_iri(&format!("{EX}{name}"));
        let value = builder.intern_literal(RdfLiteral::typed(n.to_string(), XSD_INTEGER));
        builder.push_quad(s, p, value, None);
        if with_q.contains(&name.as_str()) {
            let o = builder.intern_iri(&format!("{EX}o{n}"));
            builder.push_quad(s, q, o, None);
        }
    }
    let link = builder.intern_iri(&format!("{EX}link"));
    let a = builder.intern_iri(&format!("{EX}a"));
    let b = builder.intern_iri(&format!("{EX}b"));
    builder.push_quad(a, link, b, None);
    builder.freeze().expect("the fixture dataset")
}

/// The two in-process endpoints a nested `SERVICE` alternates between: `svc0` holds
/// `:s1 :q :o1`, `svc1` holds `:s2 :q :o2`.
fn endpoints() -> InProcessServiceResolver {
    InProcessServiceResolver::new()
        .with_endpoint(format!("{EX}svc0"), subjects_with_q(&["s1"]))
        .with_endpoint(format!("{EX}svc1"), subjects_with_q(&["s2"]))
}

// ── Answers ────────────────────────────────────────────────────────────────────────

/// What a request answers, reduced to something comparable without walking a deep term.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    /// A `SELECT`'s projected variables, and its rows spelled cell by cell and sorted.
    Rows(Vec<String>, Vec<Vec<String>>),
    /// A `SELECT` of one variable answering one row holding a triple term: how many
    /// triple terms its object chain nests, and the innermost object spelled.
    Term { levels: usize, innermost: String },
}

/// One cell, spelled: an `example.org` IRI by its local name, a literal by its lexical
/// form, an unbound cell as `-`.
fn spell(cell: Option<&TermValue>) -> String {
    match cell {
        None => "-".to_owned(),
        Some(TermValue::Iri(iri)) => iri.strip_prefix(EX).unwrap_or(iri).to_owned(),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(other) => panic!("a shallow cell, got {other:?}"),
    }
}

/// How many triple terms `value`'s object chain nests, and its innermost object, read
/// with a loop rather than a walk.
fn unwind(value: &TermValue) -> (usize, String) {
    let mut levels = 0;
    let mut term = value;
    while let TermValue::Triple { o, .. } = term {
        levels += 1;
        term = o;
    }
    (levels, spell(Some(term)))
}

fn answer_of(result: SparqlResult) -> Answer {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("every vector is a SELECT");
    };
    if let [row] = rows.as_slice()
        && let [Some(term @ TermValue::Triple { .. })] = row.as_slice()
    {
        let (levels, innermost) = unwind(term);
        return Answer::Term { levels, innermost };
    }
    let mut spelled: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.iter().map(|cell| spell(cell.as_ref())).collect())
        .collect();
    spelled.sort();
    Answer::Rows(variables, spelled)
}

/// Rows of one projected variable.
fn column(variable: &str, values: &[&str]) -> Answer {
    Answer::Rows(
        vec![variable.to_owned()],
        values
            .iter()
            .map(|value| vec![(*value).to_owned()])
            .collect(),
    )
}

/// Rows of two projected variables.
fn pairs(variables: [&str; 2], values: &[[&str; 2]]) -> Answer {
    Answer::Rows(
        variables.iter().map(|&name| name.to_owned()).collect(),
        values
            .iter()
            .map(|row| row.iter().map(|&value| value.to_owned()).collect())
            .collect(),
    )
}

// ── Vectors ────────────────────────────────────────────────────────────────────────

/// One deep-nesting shape.
struct Vector {
    name: &'static str,
    /// The request, `n` levels deep.
    text: fn(usize) -> String,
    /// What the request answers `n` levels deep, worked out from the data by
    /// construction.
    expected: fn(usize) -> Answer,
    /// Whether the request resolves `SERVICE` through [`endpoints`].
    service: bool,
}

/// `open` written `n` times, then `core`, then `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    let mut text = String::with_capacity(open.len() * n + core.len() + close.len() * n);
    for _ in 0..n {
        text.push_str(open);
    }
    text.push_str(core);
    for _ in 0..n {
        text.push_str(close);
    }
    text
}

/// `<<( :s :p … core … )>>`, `n` triple terms deep.
fn triple_chain(n: usize, core: &str) -> String {
    nested(&format!("<<( <{EX}s> <{EX}p> "), core, " )>>", n)
}

const ALL: [&str; 4] = ["s1", "s2", "s3", "s4"];

fn uncorrelated_not_exists(n: usize) -> String {
    // `n` nested `NOT EXISTS` over `?x :q ?y`, whose variables no outer row binds: the
    // innermost body holds, so the whole filter holds exactly when `n` is even.
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o {} }}",
        nested("FILTER NOT EXISTS { ", &format!("?x <{EX}q> ?y"), " }", n)
    )
}

fn uncorrelated_not_exists_expected(n: usize) -> Answer {
    if n.is_multiple_of(2) {
        column("s", &ALL)
    } else {
        column("s", &[])
    }
}

fn correlated_not_exists(n: usize) -> String {
    // Level k is `?s :p ?o FILTER NOT EXISTS { level k - 1 }`, level 0 `?s :q ?z`, every
    // level correlated with the one around it through `?s`: every subject has a `:p`, so
    // level k holds for `?s` iff level k - 1 does not, and the outermost holds for the
    // subjects with a `:q` exactly when `n` is even.
    format!(
        "SELECT ?s WHERE {{ {} }}",
        nested(
            &format!("?s <{EX}p> ?o FILTER NOT EXISTS {{ "),
            &format!("?s <{EX}q> ?z"),
            " }",
            n
        )
    )
}

fn correlated_not_exists_expected(n: usize) -> Answer {
    if n.is_multiple_of(2) {
        column("s", &["s1"])
    } else {
        column("s", &["s2", "s3", "s4"])
    }
}

fn optional_spine(n: usize) -> String {
    // `n` sibling `OPTIONAL` groups: an algebra spine `n` left joins tall. Each binds
    // `?z` for `:s1` alone, compatibly with every one before it.
    format!(
        "SELECT ?s ?z WHERE {{ ?s <{EX}p> ?o {} }}",
        format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(n)
    )
}

fn optional_spine_expected(_: usize) -> Answer {
    pairs(
        ["s", "z"],
        &[["s1", "o1"], ["s2", "-"], ["s3", "-"], ["s4", "-"]],
    )
}

fn uncorrelated_lateral(n: usize) -> String {
    // `n` nested `LATERAL` groups whose innermost `?x :q ?y` shares no variable with the
    // row around it: every subject pairs with `:s1`.
    format!(
        "SELECT ?s ?x WHERE {{ ?s <{EX}p> ?o {} }}",
        nested("LATERAL { ", &format!("?x <{EX}q> ?y"), " }", n)
    )
}

fn uncorrelated_lateral_expected(_: usize) -> Answer {
    pairs(
        ["s", "x"],
        &[["s1", "s1"], ["s2", "s1"], ["s3", "s1"], ["s4", "s1"]],
    )
}

fn correlated_lateral(n: usize) -> String {
    // `n` nested `LATERAL` groups, each re-reading `?s :p ?o` for the row around it, the
    // innermost requiring `?s :q ?z`: only `:s1`.
    format!(
        "SELECT ?s WHERE {{ {} }}",
        nested(
            &format!("?s <{EX}p> ?o LATERAL {{ "),
            &format!("?s <{EX}q> ?z"),
            " }",
            n
        )
    )
}

fn only_s1(_: usize) -> Answer {
    column("s", &["s1"])
}

fn only_s2(_: usize) -> Answer {
    column("s", &["s2"])
}

fn nested_parentheses(n: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
        nested("(", "?o", ")", n)
    )
}

fn nested_single_groups(n: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ {} }}",
        nested("{ ", &format!("?s <{EX}p> ?o FILTER(?o = 1)"), " }", n)
    )
}

fn nested_joined_groups(n: usize) -> String {
    // Every level joins `?s :p ?o` with the group inside it; the innermost requires
    // `?s :q ?z`.
    format!(
        "SELECT ?s WHERE {{ {} }}",
        nested(
            &format!("?s <{EX}p> ?o {{ "),
            &format!("?s <{EX}q> ?z"),
            " }",
            n
        )
    )
}

fn nested_negations(n: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}(?o = 1)) }}",
        "!".repeat(n)
    )
}

fn nested_negations_expected(n: usize) -> Answer {
    if n.is_multiple_of(2) {
        column("s", &["s1"])
    } else {
        column("s", &["s2", "s3", "s4"])
    }
}

fn nested_arithmetic(n: usize) -> String {
    // e0 = ?o, ek = (3 - e(k-1)): `?o` at even depths, `3 - ?o` at odd ones, so
    // `e(n) = 1` keeps `:s1` (1) at even depths and `:s2` (3 - 2) at odd ones.
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({} = 1) }}",
        nested("(3 - ", "?o", ")", n)
    )
}

fn nested_arithmetic_expected(n: usize) -> Answer {
    if n.is_multiple_of(2) {
        column("s", &["s1"])
    } else {
        column("s", &["s2"])
    }
}

fn nested_path_groups(n: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s {} ?o FILTER(?o = 2) }}",
        nested("(", &format!("<{EX}p>"), ")", n)
    )
}

fn nested_inverse_paths(n: usize) -> String {
    // `n` nested inverses of `:link`: `:a` to `:b` at even depths, `:b` to `:a` at odd.
    format!(
        "SELECT ?x ?y WHERE {{ ?x {} ?y }}",
        nested("^(", &format!("<{EX}link>"), ")", n)
    )
}

fn nested_inverse_paths_expected(n: usize) -> Answer {
    if n.is_multiple_of(2) {
        pairs(["x", "y"], &[["a", "b"]])
    } else {
        pairs(["x", "y"], &[["b", "a"]])
    }
}

fn triple_term_pattern(n: usize) -> String {
    // The dataset holds no triple term, so a pattern whose object is one matches nothing
    // at any depth.
    format!(
        "SELECT ?a ?o WHERE {{ ?a <{EX}q> {} }}",
        triple_chain(n, "?o")
    )
}

fn no_rows(_: usize) -> Answer {
    Answer::Rows(vec!["a".to_owned(), "o".to_owned()], Vec::new())
}

fn triple_term_values(n: usize) -> String {
    format!(
        "SELECT ?t WHERE {{ VALUES ?t {{ {} }} }}",
        triple_chain(n, "7")
    )
}

fn triple_term_values_expected(n: usize) -> Answer {
    Answer::Term {
        levels: n,
        innermost: "7".to_owned(),
    }
}

fn nested_service(n: usize) -> String {
    // Level k (outermost 0) forwards to `svc{k mod 2}`; only the innermost level's
    // endpoint evaluates the triple pattern, every other one relays.
    let mut text = String::from("SELECT ?s WHERE { ");
    for level in 0..n {
        write!(text, "SERVICE <{EX}svc{}> {{ ", level % 2).expect("a String accepts text");
    }
    write!(text, "?s <{EX}q> ?z").expect("a String accepts text");
    text.push_str(&" }".repeat(n));
    text.push_str(" }");
    text
}

fn nested_service_expected(n: usize) -> Answer {
    // The innermost level is `n - 1`: `svc0` (`:s1`) when that is even, `svc1` (`:s2`)
    // when it is odd.
    if (n - 1).is_multiple_of(2) {
        column("s", &["s1"])
    } else {
        column("s", &["s2"])
    }
}

const fn vector(
    name: &'static str,
    text: fn(usize) -> String,
    expected: fn(usize) -> Answer,
) -> Vector {
    Vector {
        name,
        text,
        expected,
        service: false,
    }
}

/// Every vector, by name.
fn vectors() -> Vec<Vector> {
    vec![
        vector(
            "uncorrelated NOT EXISTS",
            uncorrelated_not_exists,
            uncorrelated_not_exists_expected,
        ),
        vector(
            "correlated NOT EXISTS",
            correlated_not_exists,
            correlated_not_exists_expected,
        ),
        vector("OPTIONAL spine", optional_spine, optional_spine_expected),
        vector(
            "uncorrelated LATERAL",
            uncorrelated_lateral,
            uncorrelated_lateral_expected,
        ),
        vector("correlated LATERAL", correlated_lateral, only_s1),
        vector("parentheses", nested_parentheses, only_s1),
        vector("single-element groups", nested_single_groups, only_s1),
        vector("joined groups", nested_joined_groups, only_s1),
        vector("negations", nested_negations, nested_negations_expected),
        vector(
            "FILTER arithmetic",
            nested_arithmetic,
            nested_arithmetic_expected,
        ),
        vector("path groups", nested_path_groups, only_s2),
        vector(
            "inverse paths",
            nested_inverse_paths,
            nested_inverse_paths_expected,
        ),
        vector("triple term in a pattern", triple_term_pattern, no_rows),
        vector(
            "triple term in VALUES",
            triple_term_values,
            triple_term_values_expected,
        ),
        Vector {
            service: true,
            ..vector(
                "in-process SERVICE",
                nested_service,
                nested_service_expected,
            )
        },
    ]
}

fn find(name: &str) -> Vector {
    vectors()
        .into_iter()
        .find(|vector| vector.name == name)
        .unwrap_or_else(|| panic!("no vector named {name}"))
}

// ── Harness ────────────────────────────────────────────────────────────────────────

/// Evaluate `text` over [`dataset`] on the calling thread, sequentially so the whole
/// evaluation stays on it.
fn evaluate(text: &str, service: bool) -> Result<Answer, RdfDiagnostic> {
    let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
        force_sequential: true,
        ..EvalOptions::default()
    });
    let resolver = endpoints();
    let options = if service {
        QueryOptions::new().with_remote(Some(&resolver))
    } else {
        QueryOptions::EMPTY
    };
    let data = dataset();
    engine
        .query_with_options_view(
            &*data,
            SparqlRequest {
                query: text,
                base_iri: None,
                substitutions: &[],
            },
            options,
        )
        .map(answer_of)
}

/// Run `body` on a fresh thread with [`SMALL_STACK`] of stack.
fn on_small_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(body)
        .expect("spawn")
        .join()
        .expect("the 128 KiB thread returned")
}

/// Evaluate the vector named `name` at `depth` on the 128 KiB thread and assert its
/// constructed answer.
fn assert_deep(name: &str, depth: usize) {
    let vector = find(name);
    let text = (vector.text)(depth);
    let service = vector.service;
    let answered = on_small_stack(move || evaluate(&text, service));
    match answered {
        Ok(answer) => assert_eq!(
            answer,
            (vector.expected)(depth),
            "{name}, {depth} levels deep, on a 128 KiB stack"
        ),
        Err(refused) => panic!("{name}, {depth} levels deep, on a 128 KiB stack: {refused:?}"),
    }
}

// ── Checks that hold for every vector ──────────────────────────────────────────────

/// The constructed answers are the engine's answers at depths one to four, on an
/// ordinary test thread: the expectation each deep test asserts is the shape's, not a
/// guess about the engine.
#[test]
fn every_vector_answers_its_constructed_value_at_shallow_depths() {
    for vector in vectors() {
        for depth in 1..=4 {
            let text = (vector.text)(depth);
            let answer = evaluate(&text, vector.service)
                .unwrap_or_else(|refused| panic!("{} at {depth}: {refused:?}", vector.name));
            assert_eq!(
                answer,
                (vector.expected)(depth),
                "{} at {depth}: {text}",
                vector.name
            );
        }
    }
}

/// Every deep request parses on the 128 KiB thread, and its algebra is copied, compared
/// with its copy and dropped there too.
#[test]
fn every_deep_request_parses_copies_compares_and_drops_on_a_128_kib_thread() {
    for vector in vectors() {
        let name = vector.name;
        let text = (vector.text)(DEPTH);
        on_small_stack(move || {
            let parsed = SparqlParser::new()
                .parse_query(&text)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            let copy = parsed.clone();
            assert!(copy == parsed, "{name}: the copy equals the original");
            drop(copy);
            drop(parsed);
        });
    }
}

// ── The deep vectors ───────────────────────────────────────────────────────────────

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and expr::exists evaluates each EXISTS body by recursing into eval::eval_evaluated"]
fn uncorrelated_not_exists_100k() {
    assert_deep("uncorrelated NOT EXISTS", DEPTH);
    assert_deep("uncorrelated NOT EXISTS", DEPTH + 1);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and expr::exists evaluates each correlated EXISTS body by recursing through eval_correlated into eval::eval_evaluated"]
fn correlated_not_exists_100k() {
    assert_deep("correlated NOT EXISTS", DEPTH);
    assert_deep("correlated NOT EXISTS", DEPTH + 1);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and the LeftJoin operator evaluates its left operand by recursing into eval::eval_evaluated"]
fn optional_spine_100k() {
    assert_deep("OPTIONAL spine", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and the Lateral operator evaluates its right side by recursing through eval_correlated into eval::eval_evaluated"]
fn uncorrelated_lateral_100k() {
    assert_deep("uncorrelated LATERAL", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and the Lateral operator evaluates its right side by recursing through eval_correlated into eval::eval_evaluated"]
fn correlated_lateral_100k() {
    assert_deep("correlated LATERAL", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left, however shallow the plan"]
fn parentheses_100k() {
    assert_deep("parentheses", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left, however shallow the plan"]
fn single_element_groups_100k() {
    assert_deep("single-element groups", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and the Join operator evaluates each operand by recursing into eval::eval_evaluated"]
fn joined_groups_100k() {
    assert_deep("joined groups", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and expr::eval_expr evaluates the operand of a negation by recursing into itself"]
fn negations_100k() {
    assert_deep("negations", DEPTH);
    assert_deep("negations", DEPTH + 1);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and expr::eval_expr evaluates the operands of arithmetic by recursing into itself"]
fn filter_arithmetic_100k() {
    assert_deep("FILTER arithmetic", DEPTH);
    assert_deep("FILTER arithmetic", DEPTH + 1);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left, however shallow the plan"]
fn path_groups_100k() {
    assert_deep("path groups", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and path::reach_uncached reaches the operand of an inverse path by recursing through reach_cached"]
fn inverse_paths_100k() {
    assert_deep("inverse paths", DEPTH);
    assert_deep("inverse paths", DEPTH + 1);
}

// Nothing between the parser and the answer recurses over a triple term's depth any more.
// The pattern vector's path: parse (heap frames) → algebra → `stack::height` admission
// (iterative, and a term is no level of it) → `blank_scope`, the endpoint scan and the
// soundness analyses (work lists) → the one-operator plan's BGP leaf, where
// `bgp::compile_term` indexes the nested triples flat and `compile_triple_pos` compiles
// them over an explicit frame stack, answered empty before any lookup where the dataset
// vouches for a shallower bound → no rows. The `VALUES` vector's path: the same admission
// → the `VALUES` leaf, where `convert::ground_term_to_value` assembles the term bottom-up
// over a work list, `ScratchInterner::intern` hashes it (`TermValue::hash`, a work list),
// looks it up (`RdfDataset::term_id_by_value`, a bottom-up fold) and measures it
// (`scratch::value_bytes`, a work list) → the row's cell is cloned out (`TermValue::clone`,
// a work list) → the answer. The evaluator's operator recursion is one level for a single
// leaf, so it does not bind either vector. What still holds both back on a 128 KiB thread
// is the evaluator's own stack guard, which measures the thread rather than the plan:
// `stack::height` refuses every plan when less than `purrdf_stack::MARGIN_BYTES` is left,
// and `eval::eval_evaluated` refuses its first operator there the same way.
#[test]
#[ignore = "stack::height refuses every plan, and eval::eval_evaluated its first operator, on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; nothing on the path recurses over the term's depth"]
fn triple_term_in_a_pattern_100k() {
    assert_deep("triple term in a pattern", DEPTH);
}

#[test]
#[ignore = "stack::height refuses every plan, and eval::eval_evaluated its first operator, on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; nothing on the path recurses over the term's depth"]
fn triple_term_in_values_100k() {
    assert_deep("triple term in VALUES", DEPTH);
    assert_deep("triple term in VALUES", DEPTH + 1);
}

#[test]
#[ignore = "stack::height refuses every plan on a thread with less than purrdf_stack::MARGIN_BYTES (128 KiB) left; and the Service operator evaluates an in-process body by recursing through remote::evaluate_in_memory into eval::evaluate_query_evaluated"]
fn in_process_service_100k() {
    assert_deep("in-process SERVICE", DEPTH);
    assert_deep("in-process SERVICE", DEPTH + 1);
}
