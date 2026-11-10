// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One property-function call, read on demand through
//! `NativeSparqlEngine::open_call_cursor`.
//!
//! The on-demand read is the governed lane's invocation with the rows produced as
//! they are asked for, so every test here holds it to that lane: the rows, their
//! order and the witness are compared with what `query_prepared_governed_view`
//! answers for the same prepared plan. The shapes it refuses are each executed
//! beside a shape it admits.

mod support;

use purrdf_core::term_fixture::empty_dataset as dataset;
use support::with_env;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::{SparqlResult, TermValue};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, GovernedOutcome, IndexGeneration, NativeSparqlEngine,
    PfArgs, PfArity, PfAttestation, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    QueryGovernors, ServiceLevel, Volatility,
};

/// The relation IRI every query calls; host configuration, never minted vocabulary.
const REL: &str = "https://example.org/rel/ranked";

/// How many rows the fixture relation holds.
const HOLDS: u64 = 12;

#[test]
fn ordinary_select_cursor_resumes_on_workers_in_completed_row_order() {
    use purrdf_core::RdfDatasetBuilder;

    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p").unwrap();
    for iri in [
        "http://example.org/a",
        "http://example.org/b",
        "http://example.org/c",
    ] {
        let value = builder.intern_iri(iri).unwrap();
        builder.push_quad(value, predicate, value, None).unwrap();
    }
    let dataset = builder.freeze().unwrap();
    for (text, substitutions) in [
        (
            "SELECT ?value WHERE { VALUES ?value { <http://example.org/a> <http://example.org/b> <http://example.org/c> } }",
            Vec::new(),
        ),
        (
            "SELECT ?value WHERE { ?value <http://example.org/p> ?value }",
            Vec::new(),
        ),
        (
            "SELECT ?left ?right WHERE { { { ?left <http://example.org/p> ?left } UNION { ?left <http://example.org/p> ?left } } ?right <http://example.org/p> ?right }",
            Vec::new(),
        ),
        (
            "SELECT ?value WHERE { { { ?value <http://example.org/p> ?value } UNION { ?value <http://example.org/p> ?value } } ?value <http://example.org/p> ?value }",
            Vec::new(),
        ),
        (
            "SELECT ?label WHERE { ?value <http://example.org/p> ?value FILTER(?value != <http://example.org/a>) BIND(STR(?value) AS ?label) } OFFSET 1 LIMIT 1",
            Vec::new(),
        ),
        (
            "SELECT ?value ?missing WHERE { ?value <http://example.org/p> ?value OPTIONAL { ?value <http://example.org/absent> ?missing } }",
            Vec::new(),
        ),
        (
            "SELECT ?value WHERE { { SELECT ?value WHERE { ?value <http://example.org/p> ?value } } }",
            Vec::new(),
        ),
        (
            "SELECT ?left ?right WHERE { ?left <http://example.org/p>+ ?right }",
            Vec::new(),
        ),
        (
            "SELECT ?value WHERE { ?value <http://example.org/p> ?value } ORDER BY DESC(?value)",
            Vec::new(),
        ),
        (
            "SELECT (COUNT(?value) AS ?count) WHERE { ?value <http://example.org/p> ?value }",
            Vec::new(),
        ),
        (
            "SELECT DISTINCT ?value WHERE { { ?value <http://example.org/p> ?value } UNION { ?value <http://example.org/p> ?value } }",
            Vec::new(),
        ),
        (
            "SELECT ?value ?draw ?blank WHERE { ?value <http://example.org/p> ?value BIND(RAND() AS ?draw) BIND(BNODE(STR(?value)) AS ?blank) }",
            Vec::new(),
        ),
        (
            "SELECT ?left ?right WHERE { { SELECT ?left WHERE { ?left <http://example.org/p> ?left } } { SELECT ?right WHERE { ?right <http://example.org/p> ?right } } }",
            Vec::new(),
        ),
        (
            "SELECT ?value WHERE { { SELECT ?value WHERE { ?value <http://example.org/p> ?value } } { SELECT ?value WHERE { ?value <http://example.org/p> ?value } } }",
            Vec::new(),
        ),
        (
            "SELECT ?left ?right WHERE { { SELECT ?left ?shared WHERE { VALUES (?left ?shared) { (1 UNDEF) (2 10) } } } { SELECT ?right ?shared WHERE { VALUES (?right ?shared) { (4 UNDEF) (3 10) (5 20) } } } }",
            Vec::new(),
        ),
        (
            "SELECT ?value ?label WHERE { ?value <http://example.org/p> ?value BIND(STR(?value) AS ?label) }",
            vec![("value".to_owned(), TermValue::iri("http://example.org/b"))],
        ),
    ] {
        let engine = NativeSparqlEngine::new();
        let prepared = engine.prepare_query(text, None).unwrap();
        let options = purrdf_sparql_eval::QueryOptions::EMPTY
            .with_entropy_seed(67)
            .with_bnode_mint_prefix(Some("cursor_"));
        let SparqlResult::Solutions {
            variables: names,
            rows: expected,
            ..
        } = engine
            .query_prepared_view(dataset.as_ref(), &prepared, &substitutions, &options)
            .unwrap()
        else {
            panic!("SELECT solutions");
        };
        let mut cursor = engine
            .open_select_cursor(dataset.as_ref(), &prepared, &substitutions, &options)
            .unwrap();
        assert_eq!(
            cursor
                .schema()
                .vars()
                .iter()
                .map(purrdf_sparql_algebra::Variable::as_str)
                .collect::<Vec<_>>(),
            names.iter().map(String::as_str).collect::<Vec<_>>()
        );
        let mut actual = Vec::new();
        loop {
            let (returned, polled) = std::thread::scope(|scope| {
                scope
                    .spawn(move || {
                        let polled = cursor.next_row().map(|row| {
                            row.map(|row| {
                                let cells = row
                                    .cells()
                                    .iter()
                                    .map(|cell| {
                                        cell.map(|term| {
                                            row.context()
                                                .scratch
                                                .try_value_of(row.context().dataset, term)
                                                .unwrap()
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                (
                                    row.schema()
                                        .vars()
                                        .iter()
                                        .map(|variable| variable.as_str().to_owned())
                                        .collect::<Vec<_>>(),
                                    cells,
                                )
                            })
                        });
                        (cursor, polled)
                    })
                    .join()
                    .unwrap()
            });
            cursor = returned;
            let Some((columns, cells)) = polled.unwrap() else {
                break;
            };
            assert_eq!(columns, names);
            actual.push(cells);
            if actual.len() < expected.len() {
                assert!(
                    !cursor.is_complete(),
                    "unread rows are not a complete delivered answer"
                );
            }
        }
        assert!(cursor.is_complete());
        assert_eq!(actual, expected);
        let settled = cursor.settle().unwrap();
        assert!(matches!(
            settled.completion,
            purrdf_sparql_eval::eval::SelectCompletion::Complete
        ));
        assert_eq!(settled.delivered, expected.len());
        assert_eq!(settled.established, expected.len());
        assert_eq!(settled.undecided, 0);
        let report = cursor.buffering_report();
        use purrdf_sparql_eval::eval::CursorBufferReason;
        if text.contains("ORDER BY") {
            assert!(report.contains(CursorBufferReason::OrderedInput));
        }
        if text.contains("COUNT(") {
            assert!(report.contains(CursorBufferReason::AggregateState));
        }
        if text.contains("DISTINCT") {
            assert!(report.contains(CursorBufferReason::DistinctKeys));
        }
        if text.contains("RAND()") {
            assert!(report.contains(CursorBufferReason::StatefulExpression));
        }
        if text == "SELECT ?value WHERE { ?value <http://example.org/p> ?value }" {
            assert!(
                report.is_empty(),
                "one indexed BGP retains no answer relation: {report:?}"
            );
        }
        assert!(cursor.next_row().unwrap().is_none());
        assert_eq!(cursor.settle().unwrap().delivered, settled.delivered);
    }
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(
            "SELECT ?value WHERE { ?value <http://example.org/p> ?value } ORDER BY ?value LIMIT 2",
            None,
        )
        .unwrap();
    for early in [false, true] {
        let governors = QueryGovernors::UNBOUNDED.with_fuel(u64::MAX - 2);
        let mut cursor = engine
            .open_select_cursor_governed_view(
                dataset.as_ref(),
                &prepared,
                &[],
                &purrdf_sparql_eval::QueryOptions::EMPTY,
                &governors,
            )
            .unwrap();
        assert!(cursor.next_row().unwrap().is_some());
        assert!(!cursor.is_complete(), "one unread buffered row remains");
        assert!(
            cursor
                .buffering_report()
                .contains(purrdf_sparql_eval::eval::CursorBufferReason::GovernorSettlement)
        );
        if early {
            assert!(matches!(
                cursor.settle().unwrap().completion,
                purrdf_sparql_eval::eval::SelectCompletion::ConsumerStopped
            ));
            assert!(!cursor.is_complete());
        } else {
            assert!(cursor.next_row().unwrap().is_some());
            assert!(
                !cursor.is_complete(),
                "final operator acknowledgement is still pending"
            );
            assert!(cursor.next_row().unwrap().is_none());
            assert!(cursor.is_complete());
            assert!(matches!(
                cursor.settle().unwrap().completion,
                purrdf_sparql_eval::eval::SelectCompletion::Complete
            ));
        }
        assert!(cursor.next_row().unwrap().is_none());
    }
    let governors = QueryGovernors::UNBOUNDED.with_max_intermediate_cells(0);
    let GovernedOutcome::BudgetExhausted(expected) = engine
        .query_prepared_governed_view(
            dataset.as_ref(),
            &prepared,
            &[],
            &purrdf_sparql_eval::QueryOptions::EMPTY,
            &governors,
        )
        .unwrap()
    else {
        panic!("the original plan is refused before entering a context");
    };
    let mut cursor = engine
        .open_select_cursor_governed_view(
            dataset.as_ref(),
            &prepared,
            &[],
            &purrdf_sparql_eval::QueryOptions::EMPTY,
            &governors,
        )
        .unwrap();
    assert_eq!(
        cursor
            .schema()
            .vars()
            .iter()
            .map(purrdf_sparql_algebra::Variable::as_str)
            .collect::<Vec<_>>(),
        ["value"]
    );
    assert!(matches!(
        cursor.coordinates(),
        purrdf_sparql_eval::QueryCoordinates::NotStarted
    ));
    assert!(cursor.next_row().unwrap().is_none());
    assert!(!cursor.is_complete());
    let settled = cursor.settle().unwrap();
    assert_eq!(settled.governors, Some(expected.evidence));
    let purrdf_sparql_eval::eval::SelectCompletion::Truncated { tripped, answers } =
        settled.completion
    else {
        panic!("admission refusal cannot masquerade as a complete empty answer");
    };
    assert_eq!(tripped, expected.tripped);
    assert!(answers.is_certain());
    assert_eq!(answers.result().unwrap().rows, 0);

    // Trips preserve the original row bound and positional certificate, including
    // path reach units and the absent endpoint's mandatory zero-length identity.
    let governed_options = purrdf_sparql_eval::QueryOptions::EMPTY
        .with_entropy_seed(67)
        .with_bnode_mint_prefix(Some("cursor_"));
    for text in [
        "SELECT ?value WHERE { ?value <http://example.org/p> ?value }",
        "SELECT ?value WHERE { ?value <http://example.org/p> ?value } ORDER BY ?value",
        "SELECT ?left ?right WHERE { ?left <http://example.org/p>+ ?right }",
        "SELECT ?value WHERE { <http://example.org/absent> <http://example.org/p>* ?value }",
        "SELECT ?value WHERE { VALUES ?value { 1 2 3 } }",
        "SELECT ?value WHERE { { SELECT ?value WHERE { ?value <http://example.org/p> ?value } } { SELECT ?value WHERE { ?value <http://example.org/p> ?value } } }",
        "SELECT ?left ?right WHERE { { SELECT ?left WHERE { ?left <http://example.org/p> ?left } } { SELECT ?right WHERE { ?right <http://example.org/p> ?right } } }",
        "SELECT ?value WHERE { ?value <http://example.org/p> ?value FILTER(?value != <http://example.org/b>) }",
        "SELECT ?value ?draw ?blank WHERE { ?value <http://example.org/p> ?value BIND(RAND() AS ?draw) BIND(BNODE(STR(?value)) AS ?blank) }",
        "SELECT ?value WHERE { { ?value <http://example.org/p> ?value } UNION { ?value <http://example.org/p> ?value } }",
        "SELECT ?value ?missing WHERE { ?value <http://example.org/p> ?value OPTIONAL { ?value <http://example.org/absent> ?missing } }",
    ] {
        let prepared = engine.prepare_query(text, None).unwrap();
        let baseline = engine
            .query_prepared_governed_view(
                dataset.as_ref(),
                &prepared,
                &[],
                &governed_options,
                &QueryGovernors::UNBOUNDED.with_fuel(u64::MAX - 2),
            )
            .unwrap();
        let fuel = baseline
            .evidence()
            .consumed_in(purrdf_core::ResourceDimension::Fuel);
        let GovernedOutcome::Complete {
            result: SparqlResult::Solutions { rows, .. },
            ..
        } = &baseline
        else {
            panic!("non-tripping baseline");
        };
        let count = u64::try_from(rows.len()).unwrap();
        for governors in [
            QueryGovernors::UNBOUNDED.with_fuel(0),
            QueryGovernors::UNBOUNDED.with_fuel(fuel.saturating_sub(1)),
            QueryGovernors::UNBOUNDED.with_fuel(fuel),
            QueryGovernors::UNBOUNDED.with_max_answers(0),
            QueryGovernors::UNBOUNDED.with_max_answers(count.saturating_sub(1)),
            QueryGovernors::UNBOUNDED.with_max_answers(count),
            QueryGovernors::UNBOUNDED.with_fuel(1).with_max_answers(0),
        ] {
            assert_governed_cursor_matches(
                &engine,
                dataset.as_ref(),
                &prepared,
                text,
                &governors,
                &governors,
            );
        }
    }
    let text = "SELECT ?value WHERE { ?value <http://example.org/p> ?value }";
    let prepared = engine.prepare_query(text, None).unwrap();
    let baseline = support::PollCountdown::new(usize::MAX);
    engine
        .query_prepared_governed_view(
            dataset.as_ref(),
            &prepared,
            &[],
            &governed_options,
            &QueryGovernors::UNBOUNDED
                .with_fuel(u64::MAX - 2)
                .with_max_answers(0)
                .with_stop_signal(baseline.clone()),
        )
        .unwrap();
    for quiet in [0, baseline.polls().saturating_sub(1), baseline.polls()] {
        let expected = support::PollCountdown::new(quiet);
        let actual = support::PollCountdown::new(quiet);
        let (expected_receipt, actual_receipt) = assert_governed_cursor_matches(
            &engine,
            dataset.as_ref(),
            &prepared,
            text,
            &QueryGovernors::UNBOUNDED
                .with_fuel(u64::MAX - 2)
                .with_max_answers(0)
                .with_stop_signal(expected.clone()),
            &QueryGovernors::UNBOUNDED
                .with_fuel(u64::MAX - 2)
                .with_max_answers(0)
                .with_stop_signal(actual.clone()),
        );
        assert_eq!(
            actual.polls(),
            expected.polls(),
            "original stop coordinates"
        );
        assert_eq!(
            actual_receipt.consumed_in(purrdf_core::ResourceDimension::Fuel),
            expected_receipt.consumed_in(purrdf_core::ResourceDimension::Fuel)
        );
    }
}

fn assert_governed_cursor_matches(
    engine: &NativeSparqlEngine,
    dataset: &purrdf_core::RdfDataset,
    prepared: &purrdf_sparql_eval::engine::PreparedQuery,
    text: &str,
    expected_governors: &QueryGovernors,
    actual_governors: &QueryGovernors,
) -> (purrdf_core::GovernorEvidence, purrdf_core::GovernorEvidence) {
    let options = purrdf_sparql_eval::QueryOptions::EMPTY
        .with_entropy_seed(67)
        .with_bnode_mint_prefix(Some("cursor_"));
    let expected = engine
        .query_prepared_governed_view(dataset, prepared, &[], &options, expected_governors)
        .unwrap();
    let mut cursor = engine
        .open_select_cursor_governed_view(dataset, prepared, &[], &options, actual_governors)
        .unwrap();
    let mut actual = Vec::new();
    while let Some(row) = cursor.next_row().unwrap() {
        actual.push(
            row.cells()
                .iter()
                .map(|cell| {
                    cell.map(|term| {
                        row.context()
                            .scratch
                            .try_value_of(row.context().dataset, term)
                            .unwrap()
                    })
                })
                .collect::<Vec<_>>(),
        );
    }
    let settled = cursor.settle().unwrap();
    let result = match (&expected, &settled.completion) {
        (
            GovernedOutcome::Complete { result, .. },
            purrdf_sparql_eval::eval::SelectCompletion::Complete,
        ) => Some(result),
        (
            GovernedOutcome::BudgetExhausted(expected),
            purrdf_sparql_eval::eval::SelectCompletion::Truncated { tripped, answers },
        ) => {
            assert_eq!(*tripped, expected.tripped, "{text}");
            assert_eq!(
                answers.is_certain(),
                expected.partial.is_certain(),
                "{text}"
            );
            assert_eq!(answers.barrier(), expected.partial.barrier(), "{text}");
            assert_eq!(
                answers.result().map(|prefix| prefix.positional),
                expected
                    .partial
                    .result()
                    .map(purrdf_sparql_eval::PartialSparqlResult::is_positional_prefix),
                "{text}"
            );
            expected
                .partial
                .result()
                .map(purrdf_sparql_eval::PartialSparqlResult::result)
        }
        _ => panic!("cursor completion differs from original governed answer: {text}"),
    };
    let rows = match result {
        Some(SparqlResult::Solutions { rows, .. }) => rows.as_slice(),
        None => &[],
        _ => panic!("SELECT solutions"),
    };
    assert_eq!(actual, rows, "{text}");
    assert_eq!(settled.delivered, actual.len());
    assert_eq!(settled.established, actual.len());
    assert_eq!(settled.undecided, 0);
    (expected.evidence().clone(), settled.governors.unwrap())
}

fn ex(local: &str) -> String {
    format!("https://example.org/d/{local}")
}

/// A ranked relation of [`HOLDS`] rows, `(entity_i, score_i)`, minted as pulled.
///
/// It reports one unit of work per row it mints, and — when `moves_after` is set —
/// a different generation once it has minted that many rows: an index rebuilt under
/// the read. Every ceiling it is offered is recorded, one entry per open.
struct Ranked {
    moves_after: Option<u64>,
    minted: Arc<AtomicU64>,
    offered: Arc<Mutex<Vec<Option<u64>>>>,
}

impl PropertyFunction for Ranked {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        static MODES: std::sync::OnceLock<Vec<BindingPattern>> = std::sync::OnceLock::new();
        MODES.get_or_init(|| vec![PfArity::new(1, 1).all_free_mode()])
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        HOLDS
    }

    fn open<'cursor>(
        &self,
        _args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor + 'cursor>, EvalError>
    where
        Self: 'cursor,
    {
        self.offered
            .lock()
            .expect("the fixture recorder is never poisoned")
            .push(ceiling);
        Ok(Box::new(RankedCursor {
            next: 0,
            unreported: 0,
            moves_after: self.moves_after,
            minted: Arc::clone(&self.minted),
        }))
    }
}

struct RankedCursor {
    next: u64,
    unreported: u64,
    moves_after: Option<u64>,
    minted: Arc<AtomicU64>,
}

impl PfCursor for RankedCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.next >= HOLDS {
            return Ok(None);
        }
        let row = vec![
            TermValue::iri(ex(&format!("entity{:02}", self.next))),
            TermValue::iri(ex(&format!("score{:02}", self.next))),
        ];
        self.next += 1;
        self.unreported += 1;
        self.minted.fetch_add(1, Ordering::SeqCst);
        Ok(Some(row))
    }

    fn take_work(&mut self) -> u64 {
        std::mem::take(&mut self.unreported)
    }

    fn generation(&self) -> IndexGeneration {
        match self.moves_after {
            Some(after) if self.next >= after => IndexGeneration::declared("gen-8"),
            _ => IndexGeneration::declared("gen-7"),
        }
    }
}

/// The ceilings a fixture relation was offered, one entry per open.
type Offered = Arc<Mutex<Vec<Option<u64>>>>;

/// An environment holding the fixture relation, the counter of rows it minted, and
/// the ceilings it was offered.
fn recording_environment(
    moves_after: Option<u64>,
) -> (ExtensionEnv<'static>, Arc<AtomicU64>, Offered) {
    let minted = Arc::new(AtomicU64::new(0));
    let offered: Offered = Arc::default();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(Ranked {
            moves_after,
            minted: Arc::clone(&minted),
            offered: Arc::clone(&offered),
        }),
    );
    (
        ExtensionEnv::over_relations(registry).expect("the fixture declarations read cleanly"),
        minted,
        offered,
    )
}

/// An environment holding the fixture relation, and the counter of rows it minted.
fn environment(moves_after: Option<u64>) -> (ExtensionEnv<'static>, Arc<AtomicU64>) {
    let (env, minted, _) = recording_environment(moves_after);
    (env, minted)
}

/// The shape a retrieval unit renders: a nested projection with a renaming `BIND`,
/// a `LIMIT` inside and a `LIMIT` outside.
fn nested(limit: u64) -> String {
    format!(
        "SELECT ?candidate WHERE {{\n  {{ SELECT (?c0 AS ?candidate) WHERE {{ ( ?c0 ) <{REL}> \
         ( ?c1 ) }} LIMIT {limit} }}\n}}\nLIMIT {limit}"
    )
}

/// What the governed lane answers for `query`: its variables and rows.
fn governed(query: &str) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    let (variables, rows, _) = governed_offering(query);
    (variables, rows)
}

/// A governed answer's variables and rows, beside the ceilings its relation was
/// offered.
type Offering = (Vec<String>, Vec<Vec<Option<TermValue>>>, Vec<Option<u64>>);

/// What the governed lane answers for `query`, beside the ceilings its relation was
/// offered.
fn governed_offering(query: &str) -> Offering {
    let (env, _, offered) = recording_environment(None);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query_with_options(query, None, &with_env(&env))
        .expect("the query prepares");
    match engine
        .query_prepared_governed_view(
            &*dataset(),
            &prepared,
            &[],
            &with_env(&env),
            &QueryGovernors::UNBOUNDED,
        )
        .expect("the governed lane evaluates")
    {
        GovernedOutcome::Complete {
            result: SparqlResult::Solutions {
                variables, rows, ..
            },
            ..
        } => (
            variables,
            rows,
            offered
                .lock()
                .expect("the fixture recorder is never poisoned")
                .clone(),
        ),
        other => panic!("expected solutions, got {other:?}"),
    }
}

/// Open `query` on demand against an environment whose relation moves as `moves_after`
/// says.
fn open(
    query: &str,
    moves_after: Option<u64>,
) -> (
    Result<purrdf_sparql_eval::CallCursor<'static>, String>,
    Arc<AtomicU64>,
) {
    let (env, minted) = environment(moves_after);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query_with_options(query, None, &with_env(&env))
        .expect("the query prepares");
    (
        engine
            .open_call_cursor(&prepared, &with_env(&env))
            .map_err(|diagnostic| diagnostic.to_string()),
        minted,
    )
}

/// **The same rows, in the same order, as the governed lane — and no more than asked
/// for.**
///
/// Over the retrieval shape and over a plain projection of the call, the cursor
/// drained is the governed answer row for row. Read part-way, it has minted exactly
/// the rows it yielded: the relation produced nothing its consumer did not ask for.
#[test]
fn the_cursor_is_the_governed_answer_produced_one_row_per_pull() {
    for query in [
        nested(HOLDS + 1),
        nested(5),
        format!("SELECT ?b ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }}"),
    ] {
        let (variables, rows) = governed(&query);
        let (cursor, minted) = open(&query, None);
        let mut cursor = cursor.expect("the query is one call under row-for-row operators");
        assert_eq!(cursor.variables(), variables.as_slice(), "{query}");

        let first = cursor
            .next_row(&*dataset())
            .expect("the relation reads")
            .expect("the relation holds rows");
        assert_eq!(Some(&first), rows.first(), "{query}");
        assert_eq!(
            minted.load(Ordering::SeqCst),
            1,
            "one pull minted one row, not the answer — {query}"
        );

        let mut drained = vec![first];
        while let Some(row) = cursor.next_row(&*dataset()).expect("the relation reads") {
            drained.push(row);
        }
        assert_eq!(drained, rows, "{query}");
        assert_eq!(
            cursor.work(),
            minted.load(Ordering::SeqCst),
            "the reported work is summed over every pull — {query}"
        );
    }
}

/// **The witness is built at the stop, and it pins the index.**
///
/// A stable relation settles to one generation, the one it opened on. A relation
/// that reports a different generation after three rows settles, once read past
/// them, to a witness holding both — the fact the sole-witness rule refuses — and,
/// read short of them, to one generation: the witness describes the read that was
/// taken, not one that was not.
#[test]
fn the_witness_is_taken_when_the_read_stops_and_shows_an_index_that_moved() {
    let query = nested(HOLDS + 1);

    let (stable, _) = open(&query, None);
    let mut stable = stable.expect("admitted");
    for _ in 0..6 {
        stable
            .next_row(&*dataset())
            .expect("reads")
            .expect("holds rows");
    }
    let witness = stable.settle().expect("the declarations read cleanly");
    let (_, attested) = witness.iter().next().expect("the call attested");
    assert_eq!(witness.len(), 1);
    assert_eq!(
        attested.generations.len(),
        1,
        "one generation for a stable index"
    );
    assert_eq!(
        stable.opened(),
        &PfAttestation {
            generation: IndexGeneration::declared("gen-7"),
            service: ServiceLevel::Undeclared,
        }
    );

    let (moving, _) = open(&query, Some(3));
    let mut moving = moving.expect("admitted");
    for _ in 0..2 {
        moving
            .next_row(&*dataset())
            .expect("reads")
            .expect("holds rows");
    }
    let short = moving.settle().expect("reads");
    assert_eq!(
        short.iter().next().expect("attested").1.generations.len(),
        1,
        "read short of the move, the read served from one generation"
    );
    for _ in 0..4 {
        moving
            .next_row(&*dataset())
            .expect("reads")
            .expect("holds rows");
    }
    let past = moving.settle().expect("reads");
    assert_eq!(
        past.iter().next().expect("attested").1.generations.len(),
        2,
        "read past the move, the witness holds both generations it served from"
    );
}

/// **A `FILTER` over the call is applied row by row: the governed answer, the rows a
/// `LIMIT` counts where it stands, and a ceiling never offered past the `FILTER`.**
///
/// Four texts over the one relation, each read on demand and by the governed lane:
///
/// * a `FILTER` dropping the relation's first row, under a `LIMIT` of three written
///   *above* it — the answer is the three rows after the dropped one, the read
///   minted four (one dropped, three kept) and stopped without a fifth, and the
///   relation was offered **no** ceiling: three counts rows the `FILTER` keeps, and a
///   relation told three would stop one short;
/// * the same `FILTER` *above* a `LIMIT` of three in a nested `SELECT` — that `LIMIT`
///   counts the relation's own rows, so it is offered as the ceiling, the read mints
///   three, and the `FILTER` leaves two of them;
/// * a `FILTER` dropping the first ten of the twelve rows — the answer is the last
///   two, and every row was minted, because every row had to be looked at;
/// * a `FILTER` reading a name a renaming `BIND` made — it is evaluated over the
///   names visible where it stands.
///
/// Each is the governed lane's answer row for row, and each offered ceiling is the one
/// the governed lane offered the same call. An answer that still held the dropped row
/// would be the `FILTER` skipped; a read that minted more than it needed would be the
/// `FILTER` applied after a drain.
#[test]
fn a_filter_over_the_call_is_applied_row_by_row_and_never_licenses_a_short_read() {
    let dropped = |upto: u32| {
        (0..upto)
            .map(|index| format!("<{}>", ex(&format!("entity{index:02}"))))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let cases = [
        (
            format!(
                "SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(?a NOT IN ({})) }} LIMIT 3",
                dropped(1)
            ),
            vec!["entity01", "entity02", "entity03"],
            4,
            None,
        ),
        (
            format!(
                "SELECT ?a WHERE {{ {{ SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }} LIMIT 3 }} \
                 FILTER(?a NOT IN ({})) }}",
                dropped(1)
            ),
            vec!["entity01", "entity02"],
            3,
            Some(3),
        ),
        (
            format!(
                "SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(?a NOT IN ({})) }}",
                dropped(10)
            ),
            vec!["entity10", "entity11"],
            HOLDS,
            None,
        ),
        (
            format!(
                "SELECT ?x WHERE {{ ( ?a ) <{REL}> ( ?b ) BIND(?a AS ?x) FILTER(?x NOT IN \
                 ({})) }} LIMIT 2",
                dropped(2)
            ),
            vec!["entity02", "entity03"],
            4,
            None,
        ),
    ];
    for (query, expected, minted_rows, ceiling) in cases {
        let (variables, rows, governed_offered) = governed_offering(&query);
        let expected: Vec<Vec<Option<TermValue>>> = expected
            .iter()
            .map(|entity| vec![Some(TermValue::iri(ex(entity)))])
            .collect();
        assert_eq!(rows, expected, "the governed answer — {query}");

        let (env, minted, offered) = recording_environment(None);
        let engine = NativeSparqlEngine::new();
        let prepared = engine
            .prepare_query_with_options(&query, None, &with_env(&env))
            .expect("the query prepares");
        assert!(
            prepared.is_call_read(),
            "a row-by-row FILTER reads on demand — {query}"
        );
        let mut cursor = engine
            .open_call_cursor(&prepared, &with_env(&env))
            .expect("the query is one call under row-for-row operators");
        assert_eq!(cursor.variables(), variables.as_slice(), "{query}");
        let data = dataset();
        let mut drained = Vec::new();
        while let Some(row) = cursor.next_row(&*data).expect("the relation reads") {
            drained.push(row);
        }
        assert_eq!(drained, rows, "the governed answer, row for row — {query}");
        assert_eq!(
            minted.load(Ordering::SeqCst),
            minted_rows,
            "the rows the read had to look at, and not one more — {query}"
        );
        let offered = offered
            .lock()
            .expect("the fixture recorder is never poisoned")
            .clone();
        assert_eq!(offered, vec![ceiling], "the ceiling offered — {query}");
        assert_eq!(
            offered, governed_offered,
            "the ceiling the governed lane offers the same call — {query}"
        );
    }
}

/// **Each refused shape beside the admitted one.**
///
/// The admitted shapes are the ones above. Everything that is not a row-for-row
/// operator between the call and the root is refused by name, because reading it on
/// demand would need that operator read on demand too — and so is a `FILTER` the read
/// cannot evaluate one row at a time, beside the row-by-row `FILTER` above.
#[test]
fn a_shape_that_is_not_one_call_under_row_for_row_operators_is_refused_by_name() {
    let refused = [
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }} OFFSET 1"),
            "an OFFSET",
        ),
        (
            format!("SELECT (STR(?a) AS ?x) WHERE {{ ( ?a ) <{REL}> ( ?b ) }}"),
            "a computed BIND",
        ),
        (
            format!("SELECT DISTINCT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }}"),
            "Distinct",
        ),
        (
            format!(
                "SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER EXISTS {{ ?a <{}> ?b }} }}",
                ex("p")
            ),
            "a FILTER whose predicate embeds EXISTS",
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(RAND() < 2) }}"),
            "a FILTER whose predicate calls a builtin that draws per-query state",
        ),
        (
            format!(
                "SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(<{}>(?a)) }}",
                ex("fn")
            ),
            "a FILTER whose predicate calls a custom function",
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }} ORDER BY ?b"),
            "OrderBy",
        ),
    ];
    for (query, named) in refused {
        let (cursor, minted) = open(&query, None);
        let Err(reason) = cursor else {
            panic!("`{query}` is not one call under row-for-row operators");
        };
        assert!(
            reason.contains(named),
            "the refusal of `{query}` names {named}: {reason}"
        );
        assert_eq!(
            minted.load(Ordering::SeqCst),
            0,
            "a refused shape opened no invocation — {query}"
        );
    }

    // The neighbour: the shape a retrieval unit renders, admitted.
    let (admitted, _) = open(&nested(4), None);
    let mut admitted = admitted.expect("the rendered shape is admitted");
    assert_eq!(
        (0..5)
            .map(|_| admitted.next_row(&*dataset()).expect("reads").is_some())
            .collect::<Vec<_>>(),
        vec![true, true, true, true, false],
        "and it honours its LIMIT"
    );
}

/// **The shape predicate answers exactly what the open admits, and opens nothing.**
///
/// `PreparedQuery::is_call_read` is how a consumer decides, before opening, whether a
/// plan is read on demand or materialised. It must agree with `open_call_cursor` on
/// every shape above — `true` exactly where the open is admitted — and asking it must
/// mint no row. A dataset clause is the one refusal the open makes before the walk,
/// so it is asked here too, beside the same call without one.
#[test]
fn the_shape_predicate_agrees_with_the_open_and_opens_nothing() {
    let cases = [
        (nested(4), true),
        (nested(HOLDS + 1), true),
        (
            format!("SELECT ?b ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }}"),
            true,
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }} OFFSET 1"),
            false,
        ),
        (
            format!("SELECT (STR(?a) AS ?x) WHERE {{ ( ?a ) <{REL}> ( ?b ) }}"),
            false,
        ),
        (
            format!("SELECT DISTINCT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }}"),
            false,
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(?a != ?b) }}"),
            true,
        ),
        (
            format!(
                "SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER EXISTS {{ ?a <{}> ?b }} }}",
                ex("p")
            ),
            false,
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) }} ORDER BY ?b"),
            false,
        ),
        (
            format!(
                "SELECT ?a WHERE {{ ?a <{}> ?c . ( ?a ) <{REL}> ( ?b ) }}",
                ex("p")
            ),
            false,
        ),
        (
            format!(
                "SELECT ?a FROM <{}> WHERE {{ ( ?a ) <{REL}> ( ?b ) }}",
                ex("graph")
            ),
            false,
        ),
    ];
    for (query, admitted) in cases {
        let (env, minted) = environment(None);
        let engine = NativeSparqlEngine::new();
        let prepared = engine
            .prepare_query_with_options(&query, None, &with_env(&env))
            .expect("the query prepares");
        assert_eq!(prepared.is_call_read(), admitted, "{query}");
        assert_eq!(
            minted.load(Ordering::SeqCst),
            0,
            "asking the shape opened no invocation — {query}"
        );
        assert_eq!(
            engine.open_call_cursor(&prepared, &with_env(&env)).is_ok(),
            admitted,
            "the predicate and the open agree — {query}"
        );
    }
}

/// **The shape names the calls each projected column's values come from, through
/// every renaming and every operator that keeps them — and it describes a text before
/// any registry planned it exactly as it describes the plan it becomes.**
///
/// A composition layer asks a column's call a second question — the same invocation
/// with that column's position bound — so it needs the calls every value of the column
/// was emitted by, and the call variable carrying it. Those are asked here of a
/// prepared plan and of the bare parse of the same text; the answers must agree,
/// because the second is how a layer with no registry in hand refuses a text no
/// registry could draw a column from a call for.
///
/// Admitted: a renaming projection and `BIND`, a `FILTER`, and a join of two calls on
/// the column — each call a source, because each binds it in every solution, so the
/// join is one alternative of two calls — and the same two calls under `UNION`, two
/// alternatives of one call each, because every value either branch gives the column
/// is a value that branch's call emitted. Refused beside them: a `UNION` whose other
/// branch binds the column from `VALUES` (that branch's values are no call's), a
/// column only a `VALUES` block binds, and a computed `BIND`. The join and the dataset
/// clause are not read on demand, and say so, while still naming their sources.
#[test]
fn the_shape_names_the_calls_each_columns_values_come_from_before_and_after_planning() {
    use purrdf_sparql_algebra::{ParserOptions, SparqlParser, TermPattern, Variable};
    use purrdf_sparql_eval::CallReadShape;

    let parse = |query: &str| {
        let options = ParserOptions {
            property_fn_iris: vec![REL.to_owned()],
            ..ParserOptions::default()
        };
        SparqlParser::new()
            .parse_query_with(query, &options)
            .expect("the fixture query parses")
    };
    let prepare = |query: &str| {
        let (env, _) = environment(None);
        NativeSparqlEngine::new()
            .prepare_query_with_options(query, None, &with_env(&env))
            .expect("the query prepares")
    };
    // Each column's alternatives, each the argument lists of the calls it is drawn
    // from beside the call variable each carries it in.
    let sources = |shape: &CallReadShape<'_>, column: &str| {
        shape.sources_of(column).map(|alternatives| {
            alternatives
                .into_iter()
                .map(|alternative| {
                    alternative
                        .into_iter()
                        .map(|source| {
                            assert_eq!(source.call().iri, REL);
                            (
                                source.call().object_args.clone(),
                                source.variable().as_str().to_owned(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
    };
    let variable = |name: &str| TermPattern::Variable(Variable::new(name));
    let literal =
        |text: &str| TermPattern::Literal(purrdf_sparql_algebra::Literal::new_simple(text));
    let admitted = [
        (
            format!(
                "SELECT ?x ?b WHERE {{ {{ SELECT (?a AS ?x) ?b WHERE {{ ( ?a ) <{REL}> ( ?b ) }} \
                 LIMIT 3 }} }}"
            ),
            vec![
                ("x", vec![vec![(vec![variable("b")], "a".to_owned())]]),
                ("b", vec![vec![(vec![variable("b")], "b".to_owned())]]),
            ],
            true,
        ),
        (
            format!("SELECT ?y ?b WHERE {{ ( ?a ) <{REL}> ( ?b ) BIND(?a AS ?y) }}"),
            vec![("y", vec![vec![(vec![variable("b")], "a".to_owned())]])],
            true,
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(?a != ?b) }}"),
            vec![("a", vec![vec![(vec![variable("b")], "a".to_owned())]])],
            true,
        ),
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( \"q\" ) . ( ?a ) <{REL}> ( \"r\" ) }}"),
            vec![(
                "a",
                vec![vec![
                    (vec![literal("q")], "a".to_owned()),
                    (vec![literal("r")], "a".to_owned()),
                ]],
            )],
            false,
        ),
        (
            format!(
                "SELECT ?a WHERE {{ {{ ( ?a ) <{REL}> ( \"q\" ) }} UNION {{ ( ?a ) <{REL}> \
                 ( \"r\" ) }} }}"
            ),
            vec![(
                "a",
                vec![
                    vec![(vec![literal("q")], "a".to_owned())],
                    vec![(vec![literal("r")], "a".to_owned())],
                ],
            )],
            false,
        ),
    ];
    for (query, columns, on_demand) in admitted {
        let prepared = prepare(&query);
        let planned = prepared.call_read_shape().expect("a SELECT");
        let raw = parse(&query);
        let unplanned = CallReadShape::of(&raw).expect("a SELECT");
        for shape in [&planned, &unplanned] {
            for (column, expected) in &columns {
                let mut found = sources(shape, column)
                    .unwrap_or_else(|refusal| panic!("?{column} of {query}: {refusal}"));
                // Planning may put the calls of a join in either order; an
                // alternative is a set of calls, compared as one.
                for alternative in &mut found {
                    alternative.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
                }
                assert_eq!(&found, expected, "column ?{column} of {query}");
            }
            assert_eq!(
                shape.read_on_demand().is_ok(),
                on_demand,
                "read on demand — {query}"
            );
        }
    }

    // A variable inside a quoted-triple argument is a call variable too: the call
    // compiles it to a slot and a column exactly as it does a top-level one.
    let quoted = format!(
        "SELECT ?s WHERE {{ ( ?a ) <{REL}> ( <<( ?s <{}> <{}> )>> ) }}",
        ex("p"),
        ex("o")
    );
    let raw = parse(&quoted);
    assert_eq!(
        CallReadShape::of(&raw)
            .expect("a SELECT")
            .sources_of("s")
            .expect("the call binds ?s")
            .into_iter()
            .flatten()
            .map(|source| source.variable().clone())
            .collect::<Vec<_>>(),
        vec![Variable::new("s")],
        "{quoted}"
    );

    let refused = [
        (
            format!(
                "SELECT ?a WHERE {{ {{ ( ?a ) <{REL}> ( \"q\" ) }} UNION {{ VALUES ?a {{ <{}> }} }} }}",
                ex("entity00")
            ),
            "a UNION",
        ),
        (
            format!(
                "SELECT ?a WHERE {{ VALUES ?a {{ <{}> }} ( ?c ) <{REL}> ( ?b ) }}",
                ex("entity00")
            ),
            "no property-function call binds its ?a column",
        ),
        (
            format!("SELECT ?x WHERE {{ ( ?a ) <{REL}> ( ?b ) BIND(STR(?a) AS ?x) }}"),
            "a computed BIND",
        ),
    ];
    for (query, reason) in refused {
        let column = if query.contains("?x") { "x" } else { "a" };
        let raw = parse(&query);
        let refusal = CallReadShape::of(&raw)
            .expect("a SELECT")
            .sources_of(column)
            .expect_err("no call is a source");
        assert!(
            refusal.reason().contains(reason),
            "the bare parse names {reason}: {refusal} — {query}"
        );
        let prepared = prepare(&query);
        let refusal = prepared
            .call_read_shape()
            .expect("a SELECT")
            .sources_of(column)
            .expect_err("no call is a source");
        assert!(
            refusal.reason().contains(reason),
            "and so does the plan: {refusal} — {query}"
        );
    }

    // Not read on demand, by name; each still names its sources.
    for (query, reason) in [
        (
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) . ( ?a ) <{REL}> ( ?c ) }}"),
            "a join of the call with another pattern",
        ),
        (
            format!(
                "SELECT ?a FROM <{}> WHERE {{ ( ?a ) <{REL}> ( ?b ) }}",
                ex("graph")
            ),
            "a query with a dataset clause",
        ),
    ] {
        let raw = parse(&query);
        let prepared = prepare(&query);
        for shape in [
            CallReadShape::of(&raw).expect("a SELECT"),
            prepared.call_read_shape().expect("a SELECT"),
        ] {
            let refusal = shape.read_on_demand().expect_err("not one call");
            assert!(
                refusal.reason().contains(reason),
                "{reason}: {refusal} — {query}"
            );
            assert!(
                shape
                    .sources_of("a")
                    .is_ok_and(|sources| !sources.is_empty()),
                "and the calls ?a is drawn from are still named — {query}"
            );
        }
    }
}
