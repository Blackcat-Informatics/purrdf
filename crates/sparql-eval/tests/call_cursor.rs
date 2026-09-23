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

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, GovernedOutcome, IndexGeneration, NativeSparqlEngine,
    PfArgs, PfArity, PfAttestation, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    QueryGovernors, QueryOptions, ServiceLevel, Volatility,
};

/// The relation IRI every query calls; host configuration, never minted vocabulary.
const REL: &str = "https://example.org/rel/ranked";

/// How many rows the fixture relation holds.
const HOLDS: u64 = 12;

fn ex(local: &str) -> String {
    format!("https://example.org/d/{local}")
}

/// A ranked relation of [`HOLDS`] rows, `(entity_i, score_i)`, minted as pulled.
///
/// It reports one unit of work per row it mints, and — when `moves_after` is set —
/// a different generation once it has minted that many rows: an index rebuilt under
/// the read.
struct Ranked {
    moves_after: Option<u64>,
    minted: Arc<AtomicU64>,
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

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
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

/// An environment holding the fixture relation, and the counter of rows it minted.
fn environment(moves_after: Option<u64>) -> (ExtensionEnv, Arc<AtomicU64>) {
    let minted = Arc::new(AtomicU64::new(0));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(Ranked {
            moves_after,
            minted: Arc::clone(&minted),
        }),
    );
    (
        ExtensionEnv::over_relations(registry).expect("the fixture declarations read cleanly"),
        minted,
    )
}

fn dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid")
}

fn options(env: &ExtensionEnv) -> QueryOptions<'_> {
    QueryOptions {
        env,
        ..QueryOptions::EMPTY
    }
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
    let (env, _) = environment(None);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query_with_options(query, None, options(&env))
        .expect("the query prepares");
    match engine
        .query_prepared_governed_view(
            &*dataset(),
            &prepared,
            &[],
            options(&env),
            &QueryGovernors::UNBOUNDED,
        )
        .expect("the governed lane evaluates")
    {
        GovernedOutcome::Complete {
            result: SparqlResult::Solutions {
                variables, rows, ..
            },
            ..
        } => (variables, rows),
        other => panic!("expected solutions, got {other:?}"),
    }
}

/// Open `query` on demand against an environment whose relation moves as `moves_after`
/// says.
fn open(
    query: &str,
    moves_after: Option<u64>,
) -> (
    Result<purrdf_sparql_eval::CallCursor, String>,
    Arc<AtomicU64>,
) {
    let (env, minted) = environment(moves_after);
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query_with_options(query, None, options(&env))
        .expect("the query prepares");
    (
        engine
            .open_call_cursor(&prepared, options(&env))
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
            .next_row()
            .expect("the relation reads")
            .expect("the relation holds rows");
        assert_eq!(Some(&first), rows.first(), "{query}");
        assert_eq!(
            minted.load(Ordering::SeqCst),
            1,
            "one pull minted one row, not the answer — {query}"
        );

        let mut drained = vec![first];
        while let Some(row) = cursor.next_row().expect("the relation reads") {
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
        stable.next_row().expect("reads").expect("holds rows");
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
        moving.next_row().expect("reads").expect("holds rows");
    }
    let short = moving.settle().expect("reads");
    assert_eq!(
        short.iter().next().expect("attested").1.generations.len(),
        1,
        "read short of the move, the read served from one generation"
    );
    for _ in 0..4 {
        moving.next_row().expect("reads").expect("holds rows");
    }
    let past = moving.settle().expect("reads");
    assert_eq!(
        past.iter().next().expect("attested").1.generations.len(),
        2,
        "read past the move, the witness holds both generations it served from"
    );
}

/// **Each refused shape beside the admitted one.**
///
/// The admitted shapes are the ones above. Everything that is not a row-for-row
/// operator between the call and the root is refused by name, because reading it on
/// demand would need that operator read on demand too.
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
            format!("SELECT ?a WHERE {{ ( ?a ) <{REL}> ( ?b ) FILTER(?a != ?b) }}"),
            "Filter",
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
            .map(|_| admitted.next_row().expect("reads").is_some())
            .collect::<Vec<_>>(),
        vec![true, true, true, true, false],
        "and it honours its LIMIT"
    );
}
