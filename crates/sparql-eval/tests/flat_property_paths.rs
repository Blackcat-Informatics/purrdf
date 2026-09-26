// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Flat property-path chains of any length answer, end to end, with the relation the
//! left-nested binary chain they stand for denotes — and what really nests is still
//! refused.
//!
//! A generated `p1/p2/…/pn` or `p1|p2|…|pn` nests nothing: the parser reads it with a
//! loop into one n-ary node. These tests drive thousands of elements of each through
//! the whole request path and hold the answer to an oracle that would see a dropped,
//! reordered or re-associated element:
//!
//! * a 5 000-step sequence over a chain graph where exactly one path of that length
//!   exists, against a neighbour one step shorter that answers a different pair;
//! * a 5 000-arm alternative whose arms overlap, against the multiplicities computed
//!   here and against the `UNION` of the same arms — and a neighbour one arm shorter;
//! * every short sequence and alternative over a small graph with parallel
//!   derivations, flat against the bracketed right-nested chain, row for row in
//!   answer order, and against the SPARQL translation (§18.4: a sequence is a join
//!   through fresh variables, an alternative a union) with every multiplicity;
//! * inverse, `*`, `+`, `?` and negated steps inside a 3 000-step chain.
//!
//! Beside them, bracketed nesting past the recursion budget is still the typed
//! refusal and one level under answers, and a 2 000-step chain crosses a `SERVICE`
//! as forwarded text that re-parses in the in-process endpoint.

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfDiagnostic, SparqlRequest};
use purrdf_core::{SparqlResult, TermValue};
use purrdf_sparql_eval::{InProcessServiceResolver, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";

/// A dataset of `(subject, predicate, object)` local names under [`EX`].
fn dataset_of<'a>(triples: impl IntoIterator<Item = (String, String, &'a str)>) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = builder.intern_iri(&format!("{EX}{s}"));
        let p = builder.intern_iri(&format!("{EX}{p}"));
        let o = builder.intern_iri(&format!("{EX}{o}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture dataset")
}

/// Evaluate `query` against `data`, with an in-process `SERVICE` endpoint `<svc>`
/// serving the same dataset.
fn run(data: &Arc<RdfDataset>, query: &str) -> Result<SparqlResult, RdfDiagnostic> {
    let resolver =
        InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), Arc::clone(data));
    NativeSparqlEngine::new().query_with_source(
        data,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        &resolver,
        QueryOptions::EMPTY,
    )
}

/// The solutions as rows of local names, one cell per projected variable, in answer
/// order; an unbound cell is `-`.
fn rows(result: Result<SparqlResult, RdfDiagnostic>) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } =
        result.unwrap_or_else(|e| panic!("the query answers: {e:?}"))
    else {
        panic!("a SELECT answers with solutions");
    };
    rows.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| match cell {
                    None => "-".to_owned(),
                    Some(TermValue::Iri(iri)) => iri.trim_start_matches(EX).to_owned(),
                    Some(other) => panic!("an IRI, got {other:?}"),
                })
                .collect()
        })
        .collect()
}

/// `rows`, sorted, for an answer whose order the query does not fix.
fn sorted_rows(result: Result<SparqlResult, RdfDiagnostic>) -> Vec<Vec<String>> {
    let mut rows = rows(result);
    rows.sort();
    rows
}

/// One row from string slices.
fn row(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|&cell| cell.to_owned()).collect()
}

/// `n{i} p{i} n{i+1}` for `i` in `0..len`: a chain whose every hop has its own
/// predicate, so the one path `p0/p1/…/p{k-1}` connects `n0` to `n{k}` and nothing else.
fn chain(len: usize) -> Arc<RdfDataset> {
    dataset_of((0..len).map(|i| {
        (
            format!("n{i}"),
            format!("p{i}"),
            leak(format!("n{}", i + 1)),
        )
    }))
}

/// A `&'static str` for a fixture's object name (a test process builds a handful).
fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

/// `<p{from}>/<p{from+1}>/…` for `steps` steps, `op`-joined.
fn predicates(from: usize, steps: usize, op: &str) -> String {
    (from..from + steps)
        .map(|i| format!("<{EX}p{i}>"))
        .collect::<Vec<_>>()
        .join(op)
}

/// A 5 000-step sequence answers the one pair a path of that length connects, from a
/// bound start, from a bound end, and with both ends free (where the evaluator walks
/// every node of the graph as a start). The neighbour one step shorter answers a
/// different pair, so a dropped or reordered step would be seen.
#[test]
fn a_five_thousand_step_sequence_answers_the_one_pair_it_connects() {
    let data = chain(5_000);
    let path = predicates(0, 5_000, "/");
    assert_eq!(
        rows(run(
            &data,
            &format!("SELECT ?s ?o WHERE {{ ?s {path} ?o }}")
        )),
        vec![row(&["n0", "n5000"])]
    );
    assert_eq!(
        rows(run(
            &data,
            &format!("SELECT ?o WHERE {{ <{EX}n0> {path} ?o }}")
        )),
        vec![row(&["n5000"])]
    );
    assert_eq!(
        rows(run(
            &data,
            &format!("SELECT ?s WHERE {{ ?s {path} <{EX}n5000> }}")
        )),
        vec![row(&["n0"])]
    );

    let neighbour = predicates(0, 4_999, "/");
    assert_eq!(
        rows(run(
            &data,
            &format!("SELECT ?s ?o WHERE {{ ?s {neighbour} ?o }}")
        )),
        vec![row(&["n0", "n4999"])]
    );
    // The same steps out of order connect nothing.
    let swapped = format!("{}/<{EX}p4999>/<{EX}p4998>", predicates(0, 4_998, "/"));
    assert_eq!(
        rows(run(
            &data,
            &format!("SELECT ?s ?o WHERE {{ ?s {swapped} ?o }}")
        )),
        Vec::<Vec<String>>::new()
    );
}

/// `s a{k} o{k % 7}` for every `k` in `0..2 500`: an arm naming `a{k}` reaches
/// `o{k % 7}`.
fn fan() -> Arc<RdfDataset> {
    dataset_of((0..2_500).map(|k| ("s".to_owned(), format!("a{k}"), leak(format!("o{}", k % 7)))))
}

/// The 5 000 arms `a{k % 2 500}` for `k` in `0..arms`: every predicate twice, so every
/// object is reached by several arms, and each arm counts.
fn arms(arms: usize) -> Vec<String> {
    (0..arms).map(|k| format!("<{EX}a{}>", k % 2_500)).collect()
}

/// The multiplicity of each object among `arms`' answers: one per arm whose
/// predicate reaches it.
fn expected_counts(arms: usize) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for k in 0..arms {
        *counts.entry(format!("o{}", (k % 2_500) % 7)).or_insert(0) += 1;
    }
    counts
}

fn counts(rows: &[Vec<String>]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row.join(" ")).or_insert(0) += 1;
    }
    counts
}

/// A 5 000-arm alternative answers the bag union of its arms: every object as many
/// times as arms reach it, in arm order, and exactly what the `UNION` of the same arms
/// answers. The neighbour one arm shorter loses exactly that arm's row.
#[test]
fn a_five_thousand_arm_alternative_is_the_bag_union_of_its_arms() {
    let data = fan();
    let treatment = rows(run(
        &data,
        &format!("SELECT ?o WHERE {{ <{EX}s> {} ?o }}", arms(5_000).join("|")),
    ));
    assert_eq!(treatment.len(), 5_000);
    assert_eq!(counts(&treatment), expected_counts(5_000));
    // Arm order: arm `k` answers `o{(k % 2 500) % 7}`.
    let in_arm_order = (0..5_000)
        .map(|k| vec![format!("o{}", (k % 2_500) % 7)])
        .collect::<Vec<_>>();
    assert_eq!(treatment, in_arm_order);

    let union = arms(5_000)
        .iter()
        .map(|p| format!("{{ <{EX}s> {p} ?o }}"))
        .collect::<Vec<_>>()
        .join(" UNION ");
    let oracle = rows(run(&data, &format!("SELECT ?o WHERE {{ {union} }}")));
    assert_eq!(
        treatment, oracle,
        "the alternative answers what UNION answers"
    );

    let neighbour = rows(run(
        &data,
        &format!("SELECT ?o WHERE {{ <{EX}s> {} ?o }}", arms(4_999).join("|")),
    ));
    assert_eq!(counts(&neighbour), expected_counts(4_999));
    assert_ne!(counts(&neighbour), counts(&treatment));
}

/// A small graph with parallel derivations (`x0 p x1 p x3` and `x0 p x2 p x3`), cycles,
/// a self loop and edges of both predicates in both directions.
fn diamond() -> Arc<RdfDataset> {
    dataset_of(
        [
            ("x0", "p", "x1"),
            ("x0", "p", "x2"),
            ("x1", "p", "x3"),
            ("x2", "p", "x3"),
            ("x1", "q", "x0"),
            ("x3", "q", "x0"),
            ("x2", "q", "x2"),
            ("x0", "q", "x3"),
            ("x3", "p", "x1"),
        ]
        .map(|(s, p, o)| (s.to_owned(), p.to_owned(), o)),
    )
}

/// One step of a generated chain: its path text, and its triple pattern between two
/// terms of the §18.4 translation.
#[derive(Clone, Copy)]
enum Step {
    P,
    Q,
    InverseP,
    InverseQ,
}

impl Step {
    const ALL: [Self; 4] = [Self::P, Self::Q, Self::InverseP, Self::InverseQ];

    fn path(self) -> String {
        match self {
            Self::P => format!("<{EX}p>"),
            Self::Q => format!("<{EX}q>"),
            Self::InverseP => format!("^<{EX}p>"),
            Self::InverseQ => format!("^<{EX}q>"),
        }
    }

    fn triple(self, from: &str, to: &str) -> String {
        match self {
            Self::P => format!("{from} <{EX}p> {to} ."),
            Self::Q => format!("{from} <{EX}q> {to} ."),
            Self::InverseP => format!("{to} <{EX}p> {from} ."),
            Self::InverseQ => format!("{to} <{EX}q> {from} ."),
        }
    }
}

/// Every sequence of `len` steps.
fn every_chain(len: usize) -> Vec<Vec<Step>> {
    let mut chains = vec![Vec::new()];
    for _ in 0..len {
        chains = chains
            .into_iter()
            .flat_map(|chain| {
                Step::ALL.iter().map(move |&step| {
                    let mut next = chain.clone();
                    next.push(step);
                    next
                })
            })
            .collect();
    }
    chains
}

/// `a op (b op (c …))`: the chain bracketed to the right, which stays a real nesting
/// of one node inside another.
fn right_nested(parts: &[String], op: &str) -> String {
    match parts {
        [] => unreachable!("a chain has an element"),
        [last] => last.clone(),
        [first, rest @ ..] => format!("{first}{op}({})", right_nested(rest, op)),
    }
}

/// The subject and object terms of the three endpoint shapes: both free, a bound
/// start (forward evaluation), a bound end (backward evaluation).
const ENDPOINTS: [(&str, &str, &str); 3] = [
    ("?s", "?o", "?s ?o"),
    ("<http://example.org/x0>", "?o", "?o"),
    ("?s", "<http://example.org/x3>", "?s"),
];

/// Every sequence of one to four steps over the four step kinds answers, flat, exactly
/// what the bracketed right-nested chain answers — the same rows in the same order —
/// and exactly the bag the §18.4 translation's join through fresh variables answers,
/// multiplicities included, from both ends and with both ends free.
#[test]
fn every_short_flat_sequence_is_its_nested_chain_and_its_translation() {
    let data = diamond();
    let mut nonempty = 0;
    for len in 1..=4 {
        for chain in every_chain(len) {
            let parts = chain.iter().map(|step| step.path()).collect::<Vec<_>>();
            for (s, o, projection) in ENDPOINTS {
                let query = |path: &str| format!("SELECT {projection} WHERE {{ {s} {path} {o} }}");
                let flat = rows(run(&data, &query(&parts.join("/"))));
                let nested = rows(run(&data, &query(&right_nested(&parts, "/"))));
                assert_eq!(flat, nested, "{} from {s} to {o}", parts.join("/"));

                let terms = (0..=len)
                    .map(|i| match i {
                        0 => s.to_owned(),
                        i if i == len => o.to_owned(),
                        i => format!("?m{i}"),
                    })
                    .collect::<Vec<_>>();
                let joined = chain
                    .iter()
                    .enumerate()
                    .map(|(i, step)| step.triple(&terms[i], &terms[i + 1]))
                    .collect::<String>();
                let translated = sorted_rows(run(
                    &data,
                    &format!("SELECT {projection} WHERE {{ {joined} }}"),
                ));
                let mut flat_sorted = flat.clone();
                flat_sorted.sort();
                assert_eq!(
                    flat_sorted,
                    translated,
                    "{} from {s} to {o}",
                    parts.join("/")
                );
                nonempty += usize::from(!flat.is_empty());
            }
        }
    }
    assert!(
        nonempty > 300,
        "the fixture answers most chains ({nonempty})"
    );
}

/// Every alternative of one to three elements drawn from single steps and two-step
/// sequences answers, flat, what its bracketed right-nested form answers row for row,
/// and the bag the `UNION` of its elements answers.
#[test]
fn every_short_flat_alternative_is_its_nested_chain_and_its_union() {
    let data = diamond();
    let elements = [
        Step::P.path(),
        Step::Q.path(),
        Step::InverseP.path(),
        Step::InverseQ.path(),
        format!("{}/{}", Step::P.path(), Step::Q.path()),
        format!("{}/{}", Step::Q.path(), Step::InverseP.path()),
    ];
    let mut choices: Vec<Vec<&String>> = vec![Vec::new()];
    let mut checked = 0;
    for _ in 1..=3 {
        choices = choices
            .into_iter()
            .flat_map(|chosen| {
                elements.iter().map(move |element| {
                    let mut next = chosen.clone();
                    next.push(element);
                    next
                })
            })
            .collect();
        for chosen in &choices {
            // A two-step element needs its brackets only inside a sequence; inside an
            // alternative it binds tighter already.
            let parts = chosen.iter().map(|e| (*e).clone()).collect::<Vec<_>>();
            for (s, o, projection) in ENDPOINTS {
                let query = |path: &str| format!("SELECT {projection} WHERE {{ {s} {path} {o} }}");
                let flat = rows(run(&data, &query(&parts.join("|"))));
                let nested = rows(run(&data, &query(&right_nested(&parts, "|"))));
                assert_eq!(flat, nested, "{} from {s} to {o}", parts.join("|"));
                let union = parts
                    .iter()
                    .map(|part| format!("{{ {s} {part} {o} }}"))
                    .collect::<Vec<_>>()
                    .join(" UNION ");
                let oracle = sorted_rows(run(
                    &data,
                    &format!("SELECT {projection} WHERE {{ {union} }}"),
                ));
                let mut flat_sorted = flat;
                flat_sorted.sort();
                assert_eq!(flat_sorted, oracle, "{} from {s} to {o}", parts.join("|"));
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 3 * (6 + 36 + 216));
}

/// A chain whose hops are written five ways by position: `n{i} p{i} n{i+1}` for most
/// `i`, and `n{i+1} r{i} n{i}` where the step is an inverse.
fn zigzag(len: usize) -> Arc<RdfDataset> {
    dataset_of((0..len).map(|i| {
        if i % 5 == 1 {
            (
                format!("n{}", i + 1),
                format!("r{i}"),
                leak(format!("n{i}")),
            )
        } else {
            (
                format!("n{i}"),
                format!("p{i}"),
                leak(format!("n{}", i + 1)),
            )
        }
    }))
}

/// Step `i` of the zigzag path: `p{i}`, `^r{i}`, `p{i}+`, `p{i}?` or `!(<x>)` by
/// `i % 5`; with `plain_at` a step written as a plain `p{i}` whatever its kind.
fn zigzag_path(len: usize, plain_at: Option<usize>) -> String {
    (0..len)
        .map(|i| match (i % 5, Some(i) == plain_at) {
            (_, true) | (0, _) => format!("<{EX}p{i}>"),
            (1, _) => format!("^<{EX}r{i}>"),
            (2, _) => format!("<{EX}p{i}>+"),
            (3, _) => format!("<{EX}p{i}>?"),
            _ => format!("!(<{EX}x>)"),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Inverse, `+`, `?` and negated-set steps inside a 3 000-step chain keep their own
/// meaning. The last `?` (step 2 998) may stay put, so the frontier before the final
/// negated step is `{n2998, n2999}`, and the negated step (any predicate but `<x>`)
/// takes each one hop on: the answer is exactly `n2999` and `n3000`. The neighbour with
/// that `?` written as a plain step answers `n3000` alone.
#[test]
fn modified_steps_inside_a_long_chain_keep_their_meaning() {
    let data = zigzag(3_000);
    let answer = sorted_rows(run(
        &data,
        &format!(
            "SELECT ?o WHERE {{ <{EX}n0> {} ?o }}",
            zigzag_path(3_000, None)
        ),
    ));
    assert_eq!(answer, vec![row(&["n2999"]), row(&["n3000"])]);
    let backward = sorted_rows(run(
        &data,
        &format!(
            "SELECT ?s WHERE {{ ?s {} <{EX}n3000> }}",
            zigzag_path(3_000, None)
        ),
    ));
    assert_eq!(backward, vec![row(&["n0"])]);

    let neighbour = sorted_rows(run(
        &data,
        &format!(
            "SELECT ?o WHERE {{ <{EX}n0> {} ?o }}",
            zigzag_path(3_000, Some(2_998))
        ),
    ));
    assert_eq!(neighbour, vec![row(&["n3000"])]);
}

/// Assert `result` is the parser's typed nesting refusal naming `limit`.
fn assert_nesting_refusal(result: Result<SparqlResult, RdfDiagnostic>, limit: usize, what: &str) {
    let diagnostic = result.expect_err(&format!("{what} is refused"));
    assert!(
        diagnostic
            .message
            .contains(&format!("nesting exceeds the safety limit of {limit}")),
        "{what}: {diagnostic:?}"
    );
}

/// What really nests is still the typed refusal: 200 right-nested path groups,
/// `p0/(p1/(p2/…))`, are past the recursion budget. One hundred answer the one pair
/// their 101 steps connect — not the whole graph, and not a shorter chain's pair.
#[test]
fn nested_path_groups_past_the_budget_are_still_refused() {
    let data = chain(300);
    let nested = |depth: usize| {
        let parts = (0..=depth)
            .map(|i| format!("<{EX}p{i}>"))
            .collect::<Vec<_>>();
        format!(
            "SELECT ?s ?o WHERE {{ ?s {} ?o }}",
            right_nested(&parts, "/")
        )
    };
    assert_nesting_refusal(run(&data, &nested(200)), 128, "200 nested path groups");
    assert_eq!(rows(run(&data, &nested(100))), vec![row(&["n0", "n101"])]);
}

/// A `SERVICE` forwards its body as text the in-process endpoint re-parses. A body
/// holding a 2 000-step sequence, or a 2 000-arm alternative, is forwarded as the flat
/// chain it is and answers what the same body answers without the `SERVICE`. Under
/// `SILENT` a refused re-parse would answer the join identity, one empty row, which the
/// expected rows tell apart.
#[test]
fn two_thousand_step_paths_cross_a_service_as_text() {
    let data = chain(2_000);
    let body = format!("?s {} ?o", predicates(0, 2_000, "/"));
    let local = rows(run(&data, &format!("SELECT ?s ?o WHERE {{ {body} }}")));
    assert_eq!(local, vec![row(&["n0", "n2000"])]);
    for silent in ["", "SILENT "] {
        let forwarded = rows(run(
            &data,
            &format!("SELECT ?s ?o WHERE {{ SERVICE {silent}<{EX}svc> {{ {body} }} }}"),
        ));
        assert_eq!(forwarded, local, "SERVICE {silent}");
    }

    let data = fan();
    let body = format!("<{EX}s> {} ?o", arms(2_000).join("|"));
    let local = rows(run(&data, &format!("SELECT ?o WHERE {{ {body} }}")));
    assert_eq!(counts(&local), expected_counts(2_000));
    let forwarded = rows(run(
        &data,
        &format!("SELECT ?o WHERE {{ SERVICE <{EX}svc> {{ {body} }} }}"),
    ));
    assert_eq!(counts(&forwarded), counts(&local));
}
