// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A flat chain's parse costs the same per term however long the chain already is.
//!
//! Each shape is parsed at 2 000, 4 000 and 8 000 terms under a counting allocator,
//! and the cost of the second 4 000 terms is compared with the cost of the first
//! 2 000 doubled. Work that re-walked, re-cloned or re-validated the chain built so
//! far on every term would make that second difference grow with the chain — twice
//! as fast for quadratic work — while per-term work leaves them equal up to the few
//! reallocations a doubling vector makes. The allocator counts operations, not
//! time, so the verdict is the same on a loaded machine.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
use purrdf_sparql_algebra::{
    GraphPattern, ParserOptions, Query, SparqlParser, try_pattern_to_select_query,
};
use std::fmt::Write as _;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const EX: &str = "http://example.org/";

#[derive(Clone, Copy, Debug)]
enum MinusShape {
    None,
    Distinct,
    SharedPrefix,
    FilteredSharedPrefix,
}

/// The parse of `query`, from its text, measured on this thread.
fn parse_cost(query: &str) -> Measurement {
    let window = CurrentThreadWindow::open();
    let parsed = SparqlParser::new().parse_query(query).expect("parses");
    let measured = window.close();
    drop(parsed);
    measured
}

fn nested_stars_with_leaf(
    depth: usize,
    leaf: &str,
    trailing_values: bool,
    minus: MinusShape,
) -> String {
    let contextual_minus = !matches!(minus, MinusShape::None);
    let shared_minus_name = matches!(
        minus,
        MinusShape::SharedPrefix | MinusShape::FilteredSharedPrefix
    );
    let filtered_minus = matches!(minus, MinusShape::FilteredSharedPrefix);
    let mut query = String::new();
    for _ in 1..depth {
        query.push_str("SELECT * WHERE { { ");
    }
    write!(query, "SELECT * WHERE {{ BIND({leaf} AS ?v0)").expect("String write");
    if contextual_minus {
        query.push_str(if shared_minus_name {
            " MINUS { ?v0 <http://example.org/p> ?n0"
        } else {
            " MINUS { ?m0 <http://example.org/p> ?n0"
        });
        if filtered_minus {
            query.push_str(" FILTER(true)");
        }
        query.push_str(" }");
    }
    query.push_str(" }");
    if trailing_values {
        query.push_str(" VALUES ?t0 { 1 }");
    }
    for level in 1..depth {
        write!(query, " }} BIND({level} AS ?v{level})").expect("String write");
        if contextual_minus {
            if shared_minus_name {
                write!(query, " MINUS {{ ?v0 <http://example.org/p> ?n{level}")
                    .expect("String write");
            } else {
                write!(
                    query,
                    " MINUS {{ ?m{level} <http://example.org/p> ?n{level}"
                )
                .expect("String write");
            }
            if filtered_minus {
                query.push_str(" FILTER(true)");
            }
            query.push_str(" }");
        }
        query.push_str(" }");
        if trailing_values {
            write!(query, " VALUES ?t{level} {{ 1 }}").expect("String write");
        }
    }
    query
}

fn nested_stars(depth: usize) -> String {
    nested_stars_with_leaf(depth, "0", false, MinusShape::None)
}

#[test]
fn nested_star_clone_releases_all_live_bytes_after_its_last_owner() {
    let source = nested_stars(8);
    let window = CurrentThreadWindow::open();
    let original = SparqlParser::new()
        .parse_query(&source)
        .expect("source parses");
    let clone = original.clone();
    drop(original);
    assert!(
        window.sample().retained_bytes > 0,
        "clone retains the query"
    );
    drop(clone);
    assert_eq!(
        window.close().retained_bytes,
        0,
        "last owner releases query storage"
    );
}

#[test]
fn rewritten_star_inner_roundtrips_its_saved_projection() {
    let original = SparqlParser::new()
        .parse_query("SELECT * WHERE { ?s ?p ?o }")
        .expect("original parses");
    let Query::Select {
        pattern: GraphPattern::ProjectStar {
            variables: saved, ..
        },
        ..
    } = original
    else {
        panic!("original star snapshot");
    };
    let mut rewritten = SparqlParser::new()
        .parse_query("SELECT * WHERE { ?s ?p ?o BIND(1 AS ?extra) }")
        .expect("rewritten inner parses");
    let Query::Select {
        pattern: GraphPattern::ProjectStar { variables, .. },
        ..
    } = &mut rewritten
    else {
        panic!("rewritten star snapshot");
    };
    *variables = saved;
    let Query::Select { pattern, .. } = rewritten else {
        unreachable!("rewritten SELECT")
    };
    let rendered = try_pattern_to_select_query(&pattern).expect("rewritten query renders");
    let Query::Select {
        pattern: reparsed, ..
    } = SparqlParser::new()
        .parse_query(&rendered)
        .expect("rewritten query reparses")
    else {
        panic!("reparsed SELECT");
    };
    assert_eq!(
        pattern
            .projected_names()
            .expect("saved projection")
            .iter()
            .collect::<Vec<_>>(),
        reparsed
            .projected_names()
            .expect("reparsed projection")
            .iter()
            .collect::<Vec<_>>(),
        "carrier must preserve the saved projection: {rendered}"
    );
}

#[test]
fn rewritten_zero_column_projection_refuses_unrepresentable_carrier() {
    let Query::Select {
        pattern: GraphPattern::ProjectStar {
            variables: empty, ..
        },
        ..
    } = SparqlParser::new()
        .parse_query("SELECT * WHERE { }")
        .expect("empty star parses")
    else {
        panic!("empty star snapshot");
    };
    let mut rewritten = SparqlParser::new()
        .parse_query("SELECT * WHERE { BIND(1 AS ?extra) }")
        .expect("new inner binding parses");
    let Query::Select {
        pattern: GraphPattern::ProjectStar { variables, .. },
        ..
    } = &mut rewritten
    else {
        panic!("star snapshot");
    };
    *variables = empty;
    let Query::Select { pattern, .. } = rewritten else {
        unreachable!("rewritten SELECT")
    };
    assert!(matches!(
        try_pattern_to_select_query(&pattern),
        Err(purrdf_sparql_algebra::ParseError::Unsupported(_))
    ));

    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query("SELECT * WHERE { }")
        .expect("unchanged empty star parses")
    else {
        unreachable!("unchanged SELECT")
    };
    assert!(try_pattern_to_select_query(&pattern).is_ok());
}

#[test]
fn nested_star_accepted_parse_storage_grows_below_threefold() {
    let measured = [2_000, 4_000, 8_000].map(|depth| parse_cost(&nested_stars(depth)));
    eprintln!("nested star accepted parse allocations: {measured:?}");
    for (before, after) in measured.iter().zip(measured.iter().skip(1)) {
        assert!(
            after.requested_bytes < before.requested_bytes * 3,
            "accepted doubling has bounded allocator traffic: {before:?} -> {after:?}"
        );
        assert!(
            after.peak_working_bytes < before.peak_working_bytes * 3,
            "accepted doubling has bounded working bytes: {before:?} -> {after:?}"
        );
    }
}

#[test]
fn nested_star_with_inner_exists_keeps_accepted_scope_work_bounded() {
    let measured = [2_000, 4_000, 8_000].map(|depth| {
        parse_cost(&nested_stars_with_leaf(
            depth,
            "EXISTS { ?s <http://example.org/p> ?o }",
            false,
            MinusShape::None,
        ))
    });
    eprintln!("nested star with EXISTS accepted parse allocations: {measured:?}");
    for (before, after) in measured.iter().zip(measured.iter().skip(1)) {
        assert!(
            after.requested_bytes < before.requested_bytes * 3,
            "accepted EXISTS doubling has bounded allocator traffic: {before:?} -> {after:?}"
        );
        assert!(
            after.peak_working_bytes < before.peak_working_bytes * 3,
            "accepted EXISTS doubling has bounded working bytes: {before:?} -> {after:?}"
        );
    }
}

#[test]
fn nested_star_with_outer_exists_keeps_accepted_scope_work_bounded() {
    let measured = [2_000, 4_000, 8_000].map(|depth| {
        let nested = nested_stars(depth);
        parse_cost(&format!(
            "SELECT ?v0 WHERE {{ {{ {nested} }} FILTER EXISTS {{ ?v0 <{EX}p> ?o }} }}"
        ))
    });
    eprintln!("nested star with outer EXISTS accepted parse allocations: {measured:?}");
    for (before, after) in measured.iter().zip(measured.iter().skip(1)) {
        assert!(
            after.requested_bytes < before.requested_bytes * 3,
            "accepted outer EXISTS doubling has bounded allocator traffic: {before:?} -> {after:?}"
        );
        assert!(
            after.peak_working_bytes < before.peak_working_bytes * 3,
            "accepted outer EXISTS doubling has bounded working bytes: {before:?} -> {after:?}"
        );
    }
}

#[test]
fn nested_star_with_trailing_values_keeps_accepted_work_bounded() {
    let measured = [2_000, 4_000, 8_000]
        .map(|depth| parse_cost(&nested_stars_with_leaf(depth, "0", true, MinusShape::None)));
    eprintln!("nested star trailing VALUES allocations: {measured:?}");
    for (before, after) in measured.iter().zip(measured.iter().skip(1)) {
        assert!(
            after.requested_bytes < before.requested_bytes * 3,
            "accepted trailing VALUES doubling has bounded allocator traffic: {before:?} -> {after:?}"
        );
    }
}

#[test]
fn nested_star_rdflib_profile_keeps_accepted_work_bounded() {
    for contextual_minus in [false, true] {
        let measured = [2_000, 4_000, 8_000].map(|depth| {
            let source = nested_stars_with_leaf(
                depth,
                "0",
                false,
                if contextual_minus {
                    MinusShape::Distinct
                } else {
                    MinusShape::None
                },
            );
            let window = CurrentThreadWindow::open();
            let parsed = SparqlParser::new()
                .parse_rdflib_query_with(&source, &ParserOptions::default())
                .expect("RDFLib-profile source parses");
            let Query::Select { pattern, .. } = &parsed else {
                panic!("RDFLib-profile SELECT");
            };
            assert_eq!(
                pattern
                    .projected_names()
                    .map(purrdf_sparql_algebra::ProjectionNames::len),
                Some(depth * if contextual_minus { 3 } else { 1 })
            );
            let measured = window.close();
            drop(parsed);
            measured
        });
        eprintln!("nested star RDFLib minus={contextual_minus} allocations: {measured:?}");
        for (before, after) in measured.iter().zip(measured.iter().skip(1)) {
            assert!(
                after.requested_bytes < before.requested_bytes * 3,
                "accepted RDFLib-profile doubling has bounded allocator traffic: {before:?} -> {after:?}"
            );
        }
    }
}

#[test]
fn rdflib_contextual_minus_keeps_its_distinct_projection_law() {
    let source =
        "SELECT * WHERE { ?s <http://example.org/p> ?o MINUS { ?m <http://example.org/q> ?n } }";
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_rdflib_query_with(source, &ParserOptions::default())
        .expect("RDFLib-profile MINUS parses")
    else {
        panic!("SELECT query");
    };
    let variables = pattern
        .projected_names()
        .expect("contextual projection")
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(variables, ["s", "o", "m", "n"]);
}

#[test]
fn trailing_values_star_preserves_first_appearance_order() {
    let Query::Select { pattern, .. } = SparqlParser::new()
        .parse_query("SELECT * WHERE { BIND(1 AS ?v) } VALUES ?t { 2 }")
        .expect("trailing VALUES query parses")
    else {
        panic!("SELECT query");
    };
    let variables = pattern
        .projected_names()
        .expect("star projection")
        .iter()
        .map(|name| name.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(variables, ["v", "t"]);
}

#[test]
fn nested_star_carrier_roundtrips_at_eight_thousand_levels_without_expanding() {
    for (trailing_values, minus) in [
        (false, MinusShape::None),
        (true, MinusShape::None),
        (false, MinusShape::Distinct),
        (false, MinusShape::SharedPrefix),
        (false, MinusShape::FilteredSharedPrefix),
    ] {
        let contextual_minus = !matches!(minus, MinusShape::None);
        let shared_minus_name = matches!(
            minus,
            MinusShape::SharedPrefix | MinusShape::FilteredSharedPrefix
        );
        let mut rendered_sizes = Vec::new();
        for depth in [2_000, 4_000, 8_000] {
            let source = nested_stars_with_leaf(depth, "0", trailing_values, minus);
            let parser = SparqlParser::new();
            let parsed = if contextual_minus {
                parser.parse_rdflib_query_with(&source, &ParserOptions::default())
            } else {
                parser.parse_query(&source)
            }
            .expect("source parses");
            let Query::Select { pattern, .. } = &parsed else {
                panic!("SELECT query");
            };
            let rendered = try_pattern_to_select_query(pattern).expect("carrier renders");
            let reparsed = if contextual_minus {
                parser.parse_rdflib_query_with(&rendered, &ParserOptions::default())
            } else {
                parser.parse_query(&rendered)
            }
            .expect("carrier reparses");
            let Query::Select { pattern, .. } = &reparsed else {
                panic!("rendered SELECT query");
            };
            assert_eq!(
                pattern
                    .projected_names()
                    .map(purrdf_sparql_algebra::ProjectionNames::len),
                Some(
                    depth
                        * if contextual_minus && !shared_minus_name {
                            3
                        } else if trailing_values || shared_minus_name {
                            2
                        } else {
                            1
                        }
                ),
                "rendered projection: {}",
                &rendered[..rendered.len().min(120)]
            );
            // The innermost contextual MINUS has no earlier shared Star root;
            // its three names may be spelled explicitly once.
            assert!(rendered.matches("SELECT *").count() >= depth - usize::from(contextual_minus));
            rendered_sizes.push(rendered.len());
        }
        eprintln!(
            "nested star trailing={trailing_values} minus={minus:?} rendered bytes: {rendered_sizes:?}"
        );
        for pair in rendered_sizes.windows(2) {
            assert!(
                pair[1] < pair[0] * 3,
                "rendered byte growth: {rendered_sizes:?}"
            );
        }
    }
}

#[test]
fn nested_star_carrier_storage_grows_below_threefold() {
    for (trailing_values, minus) in [
        (false, MinusShape::None),
        (true, MinusShape::None),
        (false, MinusShape::Distinct),
        (false, MinusShape::SharedPrefix),
        (false, MinusShape::FilteredSharedPrefix),
    ] {
        let contextual_minus = !matches!(minus, MinusShape::None);
        let measured = [2_000, 4_000, 8_000].map(|depth| {
            let source = nested_stars_with_leaf(depth, "0", trailing_values, minus);
            let parser = SparqlParser::new();
            let query = if contextual_minus {
                parser.parse_rdflib_query_with(&source, &ParserOptions::default())
            } else {
                parser.parse_query(&source)
            }
            .expect("source parses");
            let Query::Select { pattern, .. } = &query else {
                panic!("SELECT query");
            };
            let window = CurrentThreadWindow::open();
            let rendered = try_pattern_to_select_query(pattern).expect("carrier renders");
            let measurement = window.close();
            drop(rendered);
            measurement
        });
        eprintln!(
            "nested star trailing={trailing_values} minus={minus:?} carrier allocations: {measured:?}"
        );
        for (before, after) in measured.iter().zip(measured.iter().skip(1)) {
            assert!(
                after.requested_bytes < before.requested_bytes * 3,
                "carrier doubling has bounded allocator traffic: {before:?} -> {after:?}"
            );
            assert!(
                after.peak_working_bytes < before.peak_working_bytes * 3,
                "carrier doubling has bounded working bytes: {before:?} -> {after:?}"
            );
        }
    }
}

/// `shape`'s query at `terms` terms.
fn query(shape: &str, terms: usize) -> String {
    match shape {
        "union" => format!(
            "SELECT ?s WHERE {{ {} }}",
            (0..terms)
                .map(|k| format!("{{ ?s <{EX}p> ?v FILTER(?v = {}) }}", k % 30))
                .collect::<Vec<_>>()
                .join(" UNION ")
        ),
        "or" => format!(
            "SELECT ?s WHERE {{ ?s <{EX}p> ?v FILTER({}) }}",
            (0..terms)
                .map(|k| format!("?v = {}", k % 30))
                .collect::<Vec<_>>()
                .join(" || ")
        ),
        "sequence" | "alternative" | "linear_sequence" => format!(
            "SELECT ?s WHERE {{ ?s {} ?v }}",
            (0..terms)
                .map(|k| match (shape, k % 4) {
                    (_, 0) | ("linear_sequence", 2) => format!("<{EX}p{}>", k % 30),
                    (_, 1) | ("linear_sequence", _) => format!("^<{EX}p{}>", k % 30),
                    (_, 2) => format!("<{EX}p{}>*", k % 30),
                    _ => format!("!(<{EX}p{}>|^<{EX}q>)", k % 30),
                })
                .collect::<Vec<_>>()
                .join(if shape == "alternative" { "|" } else { "/" })
        ),
        _ => format!(
            "SELECT ?s WHERE {{ ?s <{EX}p> ?v BIND(?v{} AS ?r) }}",
            [" + 1", " - 2.5", " * 3", " / 4.0E0"]
                .iter()
                .cycle()
                .take(terms)
                .copied()
                .collect::<String>()
        ),
    }
}

/// For a `UNION` chain (each arm a group with a triple and a `FILTER`), a `||` chain,
/// an arithmetic chain mixing all four operators, and property-path `/` and `|`
/// chains mixing plain, inverse, modified and negated steps, plus a wholly linear
/// sequence lowered to triples, the allocations and the bytes
/// requested for terms 4 000 to 8 000 are twice those for terms 2 000 to 4 000, to
/// within the handful of reallocations a doubling vector makes — so no term costs
/// more because terms came before it. Measured: 21 allocations an arm, 8 an `||`
/// operand, under 5 an arithmetic step, 6.5 a path step of the mix, each exactly
/// constant across the three lengths. A second difference four times the first would be quadratic work.
///
/// What the allocator cannot see is work that re-reads the chain without
/// allocating; the parser has none. Every per-term step of the three loops is
/// constant time: `GraphPattern::union`, `Expression::or`,
/// `Expression::arithmetic`, `PropertyPathExpression::sequence` and
/// `PropertyPathExpression::alternative` push onto the chain's vector, the height and node
/// budgets compare and assign one integer each, and the variables of a `UNION`
/// chain are collected once, after its last arm.
#[test]
fn a_chain_s_parse_costs_the_same_per_term_at_every_length() {
    for shape in [
        "union",
        "or",
        "sum",
        "sequence",
        "alternative",
        "linear_sequence",
    ] {
        let [small, middle, large] =
            [2_000, 4_000, 8_000].map(|terms| parse_cost(&query(shape, terms)));
        let first = middle.allocations - small.allocations;
        let second = large.allocations - middle.allocations;
        assert!(
            first > 4_000,
            "{shape}: the window sees the parse ({first})"
        );
        assert!(
            second.abs_diff(2 * first) <= 8,
            "{shape}: 4 000 more terms cost {second} allocations after 2 000 more cost \
             {first}\n{small:?}\n{middle:?}\n{large:?}"
        );
        let first = middle.requested_bytes - small.requested_bytes;
        let second = large.requested_bytes - middle.requested_bytes;
        assert!(
            second.abs_diff(2 * first) <= first / 100,
            "{shape}: 4 000 more terms requested {second} bytes after 2 000 more \
             requested {first}\n{small:?}\n{middle:?}\n{large:?}"
        );
    }
}
