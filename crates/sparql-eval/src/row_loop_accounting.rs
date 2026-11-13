// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Accounting pins for the per-row `EXISTS` and aggregate paths.
//!
//! The row loops of `FILTER`, `BIND` and inline aggregates take shorter routes
//! when the workspace reports no storage budget. These cases pin what those
//! routes must not change: the answers, the smallest bounded account each
//! query completes under (with one byte less refusing), the governor totals a
//! metered run reports, and the row on which an `EXISTS` scope collision is
//! refused.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension};
use purrdf_sparql_algebra::{
    Child, Expression, GraphPattern, Literal, NamedNode, NamedNodePattern, SparqlParser,
    TermPattern, TriplePattern, Variable,
};

use crate::SolutionTerm;
use crate::error::EvalError;
use crate::eval::{EvalCtx, EvaluatedOutcome, Outcome, eval_evaluated, evaluate_query_evaluated};
use crate::workspace::owned_row_regressions::account;
use crate::{GovernorState, QueryGovernors};

const EX: &str = "http://example.org/";
const XSD_FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";

/// Twelve subjects, each with a float score and one of five statuses.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let score = b.intern_iri(&format!("{EX}score")).unwrap();
    let status = b.intern_iri(&format!("{EX}status")).unwrap();
    for i in 0..12_u64 {
        let s = b.intern_iri(&format!("{EX}s{i}")).unwrap();
        let v = i.wrapping_mul(2_654_435_761) % 1000;
        let lit = b
            .intern_literal(RdfLiteral::typed(format!("{v}.0"), XSD_FLOAT.to_owned()))
            .unwrap();
        b.push_quad(s, score, lit, None).unwrap();
        let st = b.intern_iri(&format!("{EX}st{}", i % 5)).unwrap();
        b.push_quad(s, status, st, None).unwrap();
    }
    b.freeze().unwrap()
}

fn query(body: &str) -> purrdf_sparql_algebra::Query {
    SparqlParser::new()
        .parse_query(&format!("PREFIX ex: <{EX}>\n{body}"))
        .unwrap()
}

/// `FILTER NOT EXISTS` over a one-pattern body, correlated on `?x`.
const NOT_EXISTS: &str =
    "SELECT ?x ?s WHERE { ?x ex:score ?s . FILTER NOT EXISTS { ?x ex:status ex:st0 } }";
/// `BIND(EXISTS ...)`: the answer is kept as a column.
const BIND_EXISTS: &str =
    "SELECT ?x ?h WHERE { ?x ex:score ?s . BIND(EXISTS { ?x ex:status ex:st0 } AS ?h) }";
/// Two sites in one expression, with different bodies.
const TWO_SITES: &str = "SELECT ?x WHERE { ?x ex:score ?s . \
     FILTER(NOT EXISTS { ?x ex:status ex:st0 } && EXISTS { ?x ex:status ex:st1 }) }";
/// One textual body at two sites whose outer schemas differ.
const BRANCH_SCHEMAS: &str = "SELECT ?x ?s ?st WHERE { \
     { ?x ex:score ?s . FILTER NOT EXISTS { ?x ex:status ex:st2 } } UNION \
     { ?x ex:status ?st . FILTER NOT EXISTS { ?x ex:status ex:st2 } } }";
/// OPTIONAL leaves the correlated `?st` unbound on some rows of one FILTER.
const OPTIONAL_ROWS: &str = "SELECT ?x ?st WHERE { ?x ex:score ?s . \
     OPTIONAL { ?x ex:status ?st . FILTER(?st = ex:st3) } \
     FILTER NOT EXISTS { ?y ex:status ?st . FILTER(?y != ?x) } }";
/// `COUNT` of one variable per group.
const GROUP_COUNT: &str = "SELECT ?st (COUNT(?x) AS ?n) WHERE { ?x ex:status ?st } GROUP BY ?st";
/// `NOT EXISTS`, `GROUP BY` and `HAVING` in one query.
const GROUP_NOT_EXISTS: &str = "SELECT ?st (COUNT(?x) AS ?n) WHERE { ?x ex:score ?s ; ex:status ?st . \
     FILTER NOT EXISTS { ?x ex:status ex:st0 } } GROUP BY ?st HAVING (COUNT(?x) > 1)";
/// `DISTINCT`, `SAMPLE` and `MIN` read their argument variable too.
const GROUP_VALUES: &str = "SELECT ?st (COUNT(DISTINCT ?x) AS ?n) (MIN(?s) AS ?low) (SAMPLE(?x) AS ?one) \
     WHERE { ?x ex:score ?s ; ex:status ?st } GROUP BY ?st";

const QUERIES: [&str; 8] = [
    NOT_EXISTS,
    BIND_EXISTS,
    TWO_SITES,
    BRANCH_SCHEMAS,
    OPTIONAL_ROWS,
    GROUP_COUNT,
    GROUP_NOT_EXISTS,
    GROUP_VALUES,
];

/// The answer a query reports — its variables and each row's cells, a computed cell
/// by its value — without the storage the rows happen to be kept in.
fn render(outcome: &EvaluatedOutcome, ctx: &EvalCtx<'_, RdfDataset>) -> String {
    let EvaluatedOutcome::Complete(Outcome::Solutions(solutions)) = outcome else {
        panic!("every query here completes with solutions: {outcome:?}");
    };
    let mut out = format!("{:?}", solutions.schema.vars());
    for row in solutions.rows.iter() {
        out.push('\n');
        for cell in row.iter() {
            match cell {
                None => out.push_str("- "),
                Some(SolutionTerm::Existing(id)) => out.push_str(&format!("{id:?} ")),
                Some(SolutionTerm::Computed(sid)) => {
                    out.push_str(&format!("{:?} ", ctx.scratch.computed_value(*sid)));
                }
            }
        }
    }
    out
}

/// The rendered answer of an unbounded, ungoverned run.
fn answer(dataset: &RdfDataset, text: &str) -> String {
    let mut ctx = EvalCtx::new(dataset);
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).unwrap();
    render(&outcome, &ctx)
}

/// The rendered answer under a bounded account of `limit` bytes, or `None` when the
/// account refuses the query.
fn bounded(dataset: &RdfDataset, text: &str, limit: u64) -> Option<String> {
    let (workspace, _ledger, _) = account(limit);
    let mut ctx = EvalCtx::new(dataset);
    ctx.growth = workspace.capability();
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).ok()?;
    Some(render(&outcome, &ctx))
}

/// The consumption a metered run reports, per dimension, with its rendered answer.
fn metered(dataset: &RdfDataset, text: &str) -> ([u64; 4], String) {
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let mut ctx = EvalCtx::new(dataset).with_governors(Arc::clone(&state));
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).unwrap();
    let evidence = state.evidence();
    assert_eq!(evidence.tripped(), None, "a metered run never refuses");
    (
        [
            ResourceDimension::Fuel,
            ResourceDimension::ScratchBytes,
            ResourceDimension::IntermediateCells,
            ResourceDimension::AnswerRows,
        ]
        .map(|dimension| evidence.consumed_in(dimension)),
        render(&outcome, &ctx),
    )
}

/// The smallest account each query completes under: a property of the bounded
/// (admitted) path, which the shorter unbounded routes leave untouched.
const BOUNDED_MINIMUM: [u64; 8] = [25147, 26499, 32349, 34191, 59323, 18598, 30641, 33902];

/// What a metered run of each query reports: fuel, scratch bytes, intermediate
/// cells and answer rows.
const METERED_TOTALS: [[u64; 4]; 8] = [
    [65, 153, 24, 9],
    [71, 153, 36, 12],
    [61, 153, 24, 3],
    [150, 153, 60, 20],
    [694, 153, 36, 0],
    [65, 784, 24, 5],
    [121, 1328, 36, 4],
    [133, 2318, 36, 5],
];

#[test]
fn bounded_accounts_complete_at_the_pinned_minimum_and_refuse_one_byte_below() {
    let dataset = dataset();
    for (text, minimum) in QUERIES.into_iter().zip(BOUNDED_MINIMUM) {
        let expected = answer(&dataset, text);
        assert_eq!(
            bounded(&dataset, text, minimum).as_deref(),
            Some(expected.as_str()),
            "{text}: completes with the unbounded answer at {minimum} bytes"
        );
        assert_eq!(
            bounded(&dataset, text, minimum - 1),
            None,
            "{text}: refused one byte below"
        );
    }
}

#[test]
fn metered_runs_report_the_pinned_totals_and_the_unbounded_answer() {
    let dataset = dataset();
    for (text, totals) in QUERIES.into_iter().zip(METERED_TOTALS) {
        let (consumed, rendered) = metered(&dataset, text);
        assert_eq!(consumed, totals, "{text}: metered totals");
        assert_eq!(rendered, answer(&dataset, text), "{text}: metered answer");
    }
}

/// Hand-built algebra the parser would refuse: `FILTER NOT EXISTS { BIND("x" AS ?o) }`
/// over rows of which only the last binds `?o`.
fn collision_pattern(with_binding_row: bool) -> GraphPattern {
    let triple = |object: TermPattern| GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(Variable::new("x")),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!("{EX}score"))),
            object,
        }],
    };
    let unbound = triple(TermPattern::Variable(Variable::new("w")));
    let inner = if with_binding_row {
        GraphPattern::union(unbound, triple(TermPattern::Variable(Variable::new("o"))))
    } else {
        unbound
    };
    let body = GraphPattern::Extend {
        inner: Child::new(GraphPattern::Bgp {
            patterns: Vec::new(),
        }),
        variable: Variable::new("o"),
        expression: Expression::Literal(Literal::new_simple("x")),
    };
    GraphPattern::Filter {
        expr: Expression::Not(Child::new(Expression::Exists(Child::new(body)))),
        inner: Child::new(inner),
    }
}

#[test]
fn exists_scope_collision_is_refused_on_the_row_that_binds_the_variable() {
    let dataset = dataset();
    // Twelve rows leave `?o` unbound; every one of them is answered.
    let mut ctx = EvalCtx::new(dataset.as_ref());
    let kept = eval_evaluated(&collision_pattern(false), &mut ctx).unwrap();
    assert_eq!(
        kept.rows().len(),
        0,
        "BIND always yields a row, so NOT EXISTS keeps none"
    );
    // The same twelve rows, then twelve that bind `?o`: the first of those refuses.
    let mut ctx = EvalCtx::new(dataset.as_ref());
    match eval_evaluated(&collision_pattern(true), &mut ctx) {
        Err(EvalError::ExistsScopeCollision { variable, intro }) => {
            assert_eq!(variable.as_str(), "o");
            assert_eq!(intro, "BIND target");
        }
        other => panic!("expected the scope collision, got {other:?}"),
    }
}

/// Run with `--ignored --nocapture` on the base revision to read the values pinned above.
#[test]
#[ignore = "measures the pinned values"]
fn measure_pinned_values() {
    let dataset = dataset();
    for text in QUERIES {
        let base = crate::workspace::QueryWorkspace::<
            crate::workspace::owned_row_regressions::Refusal,
        >::control_bytes()
        .unwrap();
        let mut high = base + 4096;
        while bounded(&dataset, text, high).is_none() {
            high *= 2;
        }
        let mut low = base;
        while low < high {
            let mid = low + (high - low) / 2;
            if bounded(&dataset, text, mid).is_some() {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let (totals, _) = metered(&dataset, text);
        eprintln!("PIN {low} {totals:?} {text}");
    }
}
