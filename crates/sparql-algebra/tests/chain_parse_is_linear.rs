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
use purrdf_sparql_algebra::SparqlParser;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const EX: &str = "http://example.org/";

/// The parse of `query`, from its text, measured on this thread.
fn parse_cost(query: &str) -> Measurement {
    let window = CurrentThreadWindow::open();
    let parsed = SparqlParser::new().parse_query(query).expect("parses");
    let measured = window.close();
    drop(parsed);
    measured
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

/// For a `UNION` chain (each arm a group with a triple and a `FILTER`), a `||` chain
/// and an arithmetic chain mixing all four operators, the allocations and the bytes
/// requested for terms 4 000 to 8 000 are twice those for terms 2 000 to 4 000, to
/// within the handful of reallocations a doubling vector makes — so no term costs
/// more because terms came before it. Measured: 21 allocations an arm, 8 an `||`
/// operand, under 5 an arithmetic step, each exactly constant across the three
/// lengths. A second difference four times the first would be quadratic work.
///
/// What the allocator cannot see is work that re-reads the chain without
/// allocating; the parser has none. Every per-term step of the three loops is
/// constant time: `GraphPattern::union`, `Expression::or` and
/// `Expression::arithmetic` push onto the chain's vector, the height and node
/// budgets compare and assign one integer each, and the variables of a `UNION`
/// chain are collected once, after its last arm.
#[test]
fn a_chain_s_parse_costs_the_same_per_term_at_every_length() {
    for shape in ["union", "or", "sum"] {
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
