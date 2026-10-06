// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every numeric operation that runs on the arbitrary-precision tower is charged to
//! the governor before it runs — its limb work against the fuel ceiling, its working
//! set against the scratch-byte ceiling — so no short query can force a vast
//! computation or allocation past a ceiling the caller set.
//!
//! Each refusal is paired with a neighbour of the SAME shape (the same operators,
//! rows and bindings) whose numbers stay small: the ceiling is the neighbour's own
//! metered consumption, so the neighbour answers under it, and the refused query
//! differs only in the work its numbers make. Without the charge both would cost the
//! same per-row fuel, and the refused query would answer too.
//!
//! The binary counts allocations, so the memory refusals also prove that the refused
//! work never allocated: the peak working set of a refused vast query stays far
//! below what it would have built.

mod support;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::{ResourceDimension, SparqlRequest, SparqlResult, TermValue, TrippedGovernor};
use purrdf_sparql_eval::{
    AggregateRegistry, ExtensionEnv, GovernedOutcome, NativeSparqlEngine, QueryGovernors,
    QueryOptions,
};
use purrdf_xsd::exact::{DivisionPolicy, Rounding};
use support::{empty_dataset, squaring_chain_from as squaring};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const STAT: &str = "http://example.org/stat#";

fn governed(query: &str, options: QueryOptions<'_>, governors: &QueryGovernors) -> GovernedOutcome {
    let query = format!("PREFIX xsd: <{XSD}>\n{query}");
    NativeSparqlEngine::new()
        .query_governed(
            &empty_dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            options,
            governors,
        )
        .unwrap_or_else(|e| panic!("a governed outcome, not a query error: {e}\n{query}"))
}

/// The first cell of a complete outcome, as its lexical form (`-` when unbound).
fn first_cell(outcome: &GovernedOutcome) -> String {
    let GovernedOutcome::Complete { result, .. } = outcome else {
        panic!("expected a complete answer, got {outcome:?}");
    };
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    match rows
        .first()
        .and_then(|row| row.first())
        .and_then(Option::as_ref)
    {
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        None => "-".to_owned(),
        Some(other) => format!("{other:?}"),
    }
}

/// Run `neighbour` metered, then both queries under a `dimension` ceiling equal to
/// the neighbour's own consumption: the neighbour must answer, `refused` must trip
/// that dimension. Returns the neighbour's answer.
fn refused_beside(
    refused: &str,
    neighbour: &str,
    options: QueryOptions<'_>,
    dimension: ResourceDimension,
) -> String {
    let metered = governed(neighbour, options, &QueryGovernors::METERED);
    assert!(
        matches!(metered, GovernedOutcome::Complete { .. }),
        "the neighbour completes metered: {metered:?}"
    );
    let spent = metered.evidence().consumed_in(dimension);
    let ceiling = match dimension {
        ResourceDimension::Fuel => QueryGovernors::UNBOUNDED.with_fuel(spent),
        ResourceDimension::ScratchBytes => QueryGovernors::UNBOUNDED.with_max_scratch_bytes(spent),
        other => panic!("no ceiling helper for {other:?}"),
    };
    let answered = governed(neighbour, options, &ceiling);
    let answer = first_cell(&answered);
    let stopped = governed(refused, options, &ceiling);
    let GovernedOutcome::BudgetExhausted(exhausted) = &stopped else {
        panic!("{refused:.200}\nmust be refused at {spent} {dimension:?}, got {stopped:?}");
    };
    assert!(
        matches!(exhausted.tripped, TrippedGovernor::Budget { dimension: d, .. } if d == dimension),
        "{:?}",
        exhausted.tripped
    );
    answer
}

fn decimal(lexical: &str) -> String {
    format!("\"{lexical}\"^^xsd:decimal")
}

fn integer(lexical: &str) -> String {
    format!("\"{lexical}\"^^xsd:integer")
}

/// `1.` followed by `zeros` zeros and `last`: a decimal of `zeros + 2` digits whose
/// leading position is the units'.
fn long_fraction(zeros: usize, last: char) -> String {
    format!("1.{}{last}", "0".repeat(zeros))
}

/// Squaring `0.1` doubles its scale at every step while its coefficient stays one
/// digit: the product's rendering is what grows, and it is charged. Squaring `1` in
/// the same chain stays a machine word.
#[test]
fn a_products_growing_scale_is_charged_for_fuel() {
    let answer = refused_beside(
        &squaring("0.1", 20),
        &squaring("1.0", 20),
        QueryOptions::EMPTY,
        ResourceDimension::Fuel,
    );
    assert_eq!(answer, "1");
    // The exact answer at a modest scale: 0.1^(2^10) has 1,024 fractional digits.
    let metered = governed(
        &squaring("0.1", 10),
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    assert_eq!(first_cell(&metered), "1026");
}

/// The same chain under a scratch-byte ceiling: a step whose rendering would not fit
/// beside what the query has already minted is refused before it allocates, and a
/// chain that would reach four gigabytes never allocates more than a few megabytes.
#[test]
fn a_products_growing_scale_is_charged_for_memory_before_it_allocates() {
    refused_beside(
        &squaring("0.1", 18),
        &squaring("1.0", 18),
        QueryOptions::EMPTY,
        ResourceDimension::ScratchBytes,
    );
    // Thirty-two squarings would render 0.1^(2^32): four billion digits.
    let ceiling = QueryGovernors::UNBOUNDED.with_max_scratch_bytes(8 << 20);
    let window = WholeProcessWindow::open();
    let outcome = governed(&squaring("0.1", 32), QueryOptions::EMPTY, &ceiling);
    let measured = window.close();
    assert!(
        matches!(&outcome, GovernedOutcome::BudgetExhausted(exhausted)
            if matches!(exhausted.tripped, TrippedGovernor::Budget { dimension: ResourceDimension::ScratchBytes, .. })),
        "{outcome:?}"
    );
    assert!(
        measured.peak_working_bytes < 64 << 20,
        "the refused chain must not allocate its digits: {measured:?}"
    );
}

/// Two long decimals with the same leading position align their coefficients to
/// compare; the same lengths with different leading positions decide at once.
#[test]
fn a_comparison_that_aligns_is_charged() {
    let a = decimal(&long_fraction(400_000, '1'));
    let b = decimal(&long_fraction(400_000, '2'));
    let c = decimal(&format!("1{}.5", "0".repeat(400_000)));
    let answer = refused_beside(
        &format!("SELECT ({a} < {b} AS ?y) WHERE {{}}"),
        &format!("SELECT ({a} < {c} AS ?y) WHERE {{}}"),
        QueryOptions::EMPTY,
        ResourceDimension::Fuel,
    );
    assert_eq!(answer, "true");
}

/// An exact operand meeting a double is converted or compared exactly: a long
/// fraction near one forms `10^scale` and divides, while an integer past the double
/// range converts from its length alone.
#[test]
fn a_tower_operand_meeting_a_double_is_charged() {
    let near_one = decimal(&long_fraction(200_000, '1'));
    let past_range = integer(&format!("1{}", "0".repeat(200_001)));
    let answer = refused_beside(
        &format!("SELECT ({near_one} + 1.0e0 AS ?y) WHERE {{}}"),
        &format!("SELECT ({past_range} + 1.0e0 AS ?y) WHERE {{}}"),
        QueryOptions::EMPTY,
        ResourceDimension::Fuel,
    );
    assert_eq!(answer, "INF");
    let answer = refused_beside(
        &format!("SELECT (xsd:double({near_one}) AS ?y) WHERE {{}}"),
        &format!("SELECT (xsd:double({past_range}) AS ?y) WHERE {{}}"),
        QueryOptions::EMPTY,
        ResourceDimension::Fuel,
    );
    assert_eq!(answer, "INF");
}

/// A division under a stated scale runs on the tower even for two small operands,
/// and its million-digit quotient is charged before it is formed.
#[test]
fn a_non_default_division_policy_is_charged() {
    let wide = DivisionPolicy::scale(1_000_000, Rounding::TowardZero);
    let narrow = DivisionPolicy::scale(10, Rounding::TowardZero);
    let query = "SELECT (1 / 3 AS ?y) WHERE {}";
    let metered = governed(
        query,
        QueryOptions::EMPTY.with_division(narrow),
        &QueryGovernors::METERED,
    );
    assert_eq!(first_cell(&metered), "0.3333333333");
    let ceiling = QueryGovernors::UNBOUNDED
        .with_fuel(metered.evidence().consumed_in(ResourceDimension::Fuel));
    assert!(matches!(
        governed(query, QueryOptions::EMPTY.with_division(narrow), &ceiling),
        GovernedOutcome::Complete { .. }
    ));
    assert!(matches!(
        governed(query, QueryOptions::EMPTY.with_division(wide), &ceiling),
        GovernedOutcome::BudgetExhausted(_)
    ));
    // AVG forms its mean under the same policy, and is charged the same way.
    let avg = "SELECT (AVG(?v) AS ?y) WHERE { VALUES ?v { 1 2 4 } }";
    let metered = governed(
        avg,
        QueryOptions::EMPTY.with_division(narrow),
        &QueryGovernors::METERED,
    );
    assert_eq!(first_cell(&metered), "2.3333333333");
    let ceiling = QueryGovernors::UNBOUNDED
        .with_fuel(metered.evidence().consumed_in(ResourceDimension::Fuel));
    assert!(matches!(
        governed(avg, QueryOptions::EMPTY.with_division(narrow), &ceiling),
        GovernedOutcome::Complete { .. }
    ));
    assert!(matches!(
        governed(avg, QueryOptions::EMPTY.with_division(wide), &ceiling),
        GovernedOutcome::BudgetExhausted(_)
    ));
}

/// `SUM` and `AVG` over a group whose first value has a vast scale align every later
/// addend to it; the same group with a short first value adds in machine words.
#[test]
fn sum_and_avg_are_charged_for_their_chain() {
    let ones = " 1".repeat(200);
    let vast = decimal(&format!("0.{}1", "0".repeat(100_000)));
    let short = decimal("0.5");
    for aggregate in ["SUM", "AVG"] {
        refused_beside(
            &format!("SELECT ({aggregate}(?v) AS ?y) WHERE {{ VALUES ?v {{ {vast}{ones} }} }}"),
            &format!("SELECT ({aggregate}(?v) AS ?y) WHERE {{ VALUES ?v {{ {short}{ones} }} }}"),
            QueryOptions::EMPTY,
            ResourceDimension::Fuel,
        );
    }
}

/// `MIN`/`MAX` and `ORDER BY` compare: long values of one leading position align,
/// long values of different leading positions decide at once.
#[test]
fn extremes_and_sorts_are_charged_for_their_comparisons() {
    let aligned: Vec<String> = (1..=9)
        .map(|digit| decimal(&long_fraction(60_000, char::from(b'0' + digit))))
        .collect();
    let spread: Vec<String> = (1..=9)
        .map(|digit| {
            decimal(&format!(
                "{}{}.5",
                digit,
                "0".repeat(digit as usize * 7_000)
            ))
        })
        .collect();
    for aggregate in ["MIN", "MAX"] {
        refused_beside(
            &format!(
                "SELECT ({aggregate}(?v) AS ?y) WHERE {{ VALUES ?v {{ {} }} }}",
                aligned.join(" ")
            ),
            &format!(
                "SELECT ({aggregate}(?v) AS ?y) WHERE {{ VALUES ?v {{ {} }} }}",
                spread.join(" ")
            ),
            QueryOptions::EMPTY,
            ResourceDimension::Fuel,
        );
    }
    refused_beside(
        &format!(
            "SELECT (STRLEN(STR(?v)) AS ?y) WHERE {{ VALUES ?v {{ {} }} }} ORDER BY ?v",
            aligned.join(" ")
        ),
        &format!(
            "SELECT (STRLEN(STR(?v)) AS ?y) WHERE {{ VALUES ?v {{ {} }} }} ORDER BY ?v",
            spread.join(" ")
        ),
        QueryOptions::EMPTY,
        ResourceDimension::Fuel,
    );
}

/// The statistical aggregates price their moments and sorts: the squares of long
/// decimals are long products; short ones are machine words.
#[test]
fn statistical_aggregates_are_charged() {
    let mut registry = AggregateRegistry::default();
    registry.register_statistical_aggregates(STAT);
    let env = ExtensionEnv::over_aggregates(registry).expect("the statistical set reads cleanly");
    let options = QueryOptions::new().with_env(&env);
    let long: Vec<String> = (1..=5)
        .map(|digit| decimal(&long_fraction(30_000, char::from(b'0' + digit))))
        .collect();
    let short: Vec<String> = (1..=5)
        .map(|digit| decimal(&format!("1.{digit}")))
        .collect();
    for local in ["VARIANCE", "STDDEV", "MEDIAN"] {
        refused_beside(
            &format!(
                "SELECT (AGG(<{STAT}{local}>, ?v) AS ?y) WHERE {{ VALUES ?v {{ {} }} }}",
                long.join(" ")
            ),
            &format!(
                "SELECT (AGG(<{STAT}{local}>, ?v) AS ?y) WHERE {{ VALUES ?v {{ {} }} }}",
                short.join(" ")
            ),
            options,
            ResourceDimension::Fuel,
        );
    }
}

/// The unary functions and the casts are linear passes over the coefficient, and the
/// cast to a string is the rendering.
#[test]
fn unary_functions_and_casts_are_charged() {
    let long = decimal(&format!("{}.5", "9".repeat(300_000)));
    let short = decimal("9.5");
    for template in [
        "ABS({})",
        "CEIL({})",
        "FLOOR({})",
        "ROUND({})",
        "(-{})",
        "xsd:integer({})",
        "xsd:string({})",
    ] {
        let refused = template.replace("{}", &long);
        let neighbour = template.replace("{}", &short);
        refused_beside(
            &format!("SELECT (STRLEN(STR({refused})) AS ?y) WHERE {{}}"),
            &format!("SELECT (STRLEN(STR({neighbour})) AS ?y) WHERE {{}}"),
            QueryOptions::EMPTY,
            ResourceDimension::Fuel,
        );
    }
}

/// Machine-word numbers charge nothing extra: a query whose numbers all fit buys the
/// same execution it bought before the tower was charged.
#[test]
fn machine_word_numbers_charge_no_tower_work() {
    let small = governed(
        "SELECT (SUM(?v * 2 + 1) AS ?y) WHERE { VALUES ?v { 1 2 3 4.5 } } ORDER BY ?y",
        QueryOptions::EMPTY,
        &QueryGovernors::METERED,
    );
    assert_eq!(first_cell(&small), "25");
    assert!(
        small.evidence().consumed_in(ResourceDimension::Fuel) < 200,
        "{:?}",
        small.evidence()
    );
}
