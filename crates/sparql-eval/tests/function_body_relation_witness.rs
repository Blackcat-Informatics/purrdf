// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A SPARQL-bodied user function whose body invokes a registered relation: the call
//! is recognised, and what the relation attested reaches the CALLING query's
//! governed receipt.
//!
//! # Why this file exists
//!
//! There is no second receipt for a function body. A body runs in a child evaluation
//! context, and everything that context learned about the relations it invoked has to
//! survive the return boundary or it is lost — silently, because a relation that
//! served a query and told nobody is indistinguishable from one that was never asked.
//! `eval_user_function` merges the child's witness back on both of its exits for
//! exactly that reason.
//!
//! That merge existed before this test did, and was unpinned: nothing anywhere
//! combined a `UserFunction` body with a relation call, so the channel worked by
//! construction and by nobody's assertion. A channel that only lines up until someone
//! refactors it is not a channel.
//!
//! # Both halves, executed
//!
//! The relation reached from a function body, and a body naming no relation
//! unchanged. The fixture is built so the two cannot be confused: the data graph
//! holds the nodes' types and nothing else, and the single fact that distinguishes
//! them is a row only the relation can supply.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_algebra::ParserOptions;
use purrdf_sparql_eval::{
    AggregateRegistry, BindingPattern, EvalError, ExprFnCall, ExtensionEnv, GovernedOutcome,
    GovernorState, IndexGeneration, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryGovernors, QueryOptions, TypeConstraint,
    UserFnBody, UserFnParam, UserFunction, UserFunctionRegistry, Volatility,
};

const EX: &str = "http://example.org/ns#";

/// The relation the function body calls.
const REL: &str = "http://example.org/rel/flagged";

/// The function whose body calls it.
const FN_IRI: &str = "http://example.org/ns#isFlagged";

/// The generation the relation's cursor declares, so the receipt carries something
/// identifiable rather than merely a count.
const GENERATION: &str = "flag-index@7";

/// A relation over one fixed row, counting how many times the engine opened it.
#[derive(Debug)]
struct FlagRelation {
    modes: [BindingPattern; 1],
    opens: Arc<AtomicU64>,
}

#[derive(Debug)]
struct FlagCursor {
    rows: std::vec::IntoIter<PfRow>,
}

impl PfCursor for FlagCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }

    fn generation(&self) -> IndexGeneration {
        IndexGeneration::declared(GENERATION)
    }
}

impl PropertyFunction for FlagRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.opens.fetch_add(1, Ordering::Relaxed);
        let rows = vec![vec![
            TermValue::Iri(format!("{EX}ada")),
            TermValue::Iri(format!("{EX}yes")),
        ]];
        Ok(Box::new(FlagCursor {
            rows: rows.into_iter(),
        }))
    }
}

fn relations() -> (PropertyFunctionRegistry, Arc<AtomicU64>) {
    let opens = Arc::new(AtomicU64::new(0));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(FlagRelation {
            modes: [BindingPattern::from_code("ff")],
            opens: Arc::clone(&opens),
        }),
    );
    (registry, opens)
}

/// Two typed nodes, and nothing that could tell them apart.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let thing = builder.intern_iri(&format!("{EX}Thing"));
    for local in ["ada", "brian"] {
        let s = builder.intern_iri(&format!("{EX}{local}"));
        builder.push_quad(s, rdf_type, thing, None);
    }
    builder.freeze().expect("freeze fixture")
}

/// A registry holding one SPARQL-bodied function whose body names `predicate`.
fn functions(predicate: &str) -> UserFunctionRegistry {
    let mut registry = UserFunctionRegistry::new();
    registry.insert(
        FN_IRI,
        UserFunction {
            params: vec![UserFnParam {
                var: "node".to_owned(),
                constraint: TypeConstraint::default(),
            }],
            required: 1,
            // Text, not algebra. Which of this body's predicate IRIs is a call is
            // decided by the environment it is bound against, below — not here.
            body: Arc::from(format!("ASK {{ ?node <{predicate}> ?why }}")),
            kind: UserFnBody::Ask,
            return_constraint: TypeConstraint::default(),
        },
    );
    registry
}

/// Run the fixture query on the GOVERNED lane — the only lane that carries a receipt,
/// and the only one that arms the witness channel at all.
fn run(predicate: &str) -> (GovernedOutcome, Arc<AtomicU64>) {
    let (relations, opens) = relations();
    let engine = NativeSparqlEngine::new();
    let env = ExtensionEnv::new(
        ParserOptions::default(),
        relations,
        AggregateRegistry::EMPTY,
    )
    .expect("the declarations read cleanly");
    let bound = engine
        .bind_functions(functions(predicate), &env)
        .expect("the body parses and admits against this environment");

    let options = QueryOptions {
        functions: &bound,
        env: &env,
        ..QueryOptions::EMPTY
    };
    let query =
        format!("SELECT ?s WHERE {{ ?s a <{EX}Thing> . FILTER(<{FN_IRI}>(?s)) }} ORDER BY ?s");
    let dataset = dataset();
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let outcome = engine
        .query_governed_in_operation(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            options,
            &state,
        )
        .expect("a governed run of a valid query is an outcome, never an error");
    (outcome, opens)
}

/// The bound subjects a completed outcome projected, in row order.
fn subjects(outcome: &GovernedOutcome) -> Vec<String> {
    let GovernedOutcome::Complete { result, .. } = outcome else {
        panic!("the fixture query completes under UNBOUNDED governors");
    };
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions");
    };
    rows.iter()
        .map(|row| match row[0].as_ref().expect("bound subject") {
            TermValue::Iri(iri) => iri.clone(),
            other => panic!("expected an IRI subject, got {other:?}"),
        })
        .collect()
}

/// The headline: the body's relation call is recognised, it decides the answer, and
/// its attestation lands on the calling query's receipt.
#[test]
fn a_function_body_s_relation_attests_on_the_calling_query_s_receipt() {
    let (outcome, opens) = run(REL);

    assert!(
        opens.load(Ordering::Relaxed) > 0,
        "the body never reached the relation"
    );
    assert_eq!(
        subjects(&outcome),
        vec![format!("{EX}ada")],
        "exactly the subject the relation flagged; the data graph cannot tell the two apart"
    );

    let attested = outcome
        .relations()
        .witness
        .get(REL)
        .expect("the relation the body invoked attests on the caller's receipt");
    assert_eq!(
        attested.invocations,
        opens.load(Ordering::Relaxed),
        "the receipt counts exactly the invocations the relation saw"
    );
    assert!(
        attested
            .generations
            .contains(&IndexGeneration::declared(GENERATION)),
        "the generation the cursor declared survives the function-call boundary: {:?}",
        attested.generations
    );
}

/// The neighbouring valid case: a body naming a predicate nobody registered is
/// ordinary data. It answers over the base graph, invokes nothing, and attests
/// nothing — unchanged.
#[test]
fn a_body_naming_no_registered_relation_is_ordinary_data_and_attests_nothing() {
    let (outcome, opens) = run(&format!("{EX}plainEdge"));

    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "a body naming an unregistered predicate must not invoke the relation"
    );
    assert!(
        subjects(&outcome).is_empty(),
        "no node has the plain edge, so the ASK is false for every subject"
    );
    assert!(
        outcome.relations().witness.get(REL).is_none(),
        "a relation that was never called attests nothing"
    );
}

/// The prefix-vs-exact guard at the seam that decides it: registering
/// `…/rel/flagged` must leave `…/rel/flaggedElsewhere` an ordinary predicate. An
/// over-refusal here would break a working query with a diagnostic naming the wrong
/// cause.
#[test]
fn a_sibling_iri_sharing_a_registered_prefix_stays_ordinary_data() {
    let (outcome, opens) = run(&format!("{REL}Elsewhere"));

    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "a merely-same-prefixed sibling must not resolve to the registered relation"
    );
    assert_eq!(
        subjects(&outcome),
        Vec::<String>::new(),
        "the sibling is an ordinary predicate that matches nothing"
    );
    assert!(outcome.relations().witness.get(REL).is_none());
}

// ── The expression-bodied door ──────────────────────────────────────────────────

/// An expression-bodied function whose body RE-ENTERS the evaluator, invoking the
/// relation through a fresh governed run, reaches the calling query's receipt.
///
/// This door differs from the SPARQL-bodied one above in how it gets back into the
/// evaluator: `eval_user_function` builds a CHILD context and merges its witness on
/// return, while an expression body calls the engine again and produces its own,
/// separate governed outcome. That outcome's evidence dies with it unless the body
/// hands it back — so before `ExprFnCall::record_relations` existed, a relation
/// invoked through this seam served the query and told nobody, which is
/// indistinguishable from never having been asked.
#[test]
fn an_expression_body_s_relation_attests_on_the_calling_query_s_receipt() {
    let (relations, opens) = relations();
    let env = ExtensionEnv::over_relations(relations).expect("the declarations read cleanly");
    let engine = NativeSparqlEngine::new();

    // The body: a whole nested governed query over the focus graph, calling the
    // relation. Its witness is handed back through the call rather than discarded.
    let nested_env = ExtensionEnv::over_relations(relations_only(Arc::clone(&opens)))
        .expect("the declarations read cleanly");
    let body: purrdf_sparql_eval::ExprFnBody = Arc::new(move |call: &ExprFnCall<'_>| {
        let bound = NativeSparqlEngine::new()
            .bind_functions(UserFunctionRegistry::new(), &nested_env)
            .expect("an empty registry has no body to bind");
        let inner = NativeSparqlEngine::new();
        let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
        let outcome = inner
            .query_governed_in_operation(
                &**call.focus_graph,
                SparqlRequest {
                    query: &format!("SELECT ?w WHERE {{ <{EX}ada> <{REL}> ?w }}"),
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    functions: &bound,
                    env: &nested_env,
                    ..QueryOptions::EMPTY
                },
                &state,
            )
            .map_err(|e| EvalError::function(e.to_string()))?;

        // THE POINT OF THIS TEST: hand the nested run's evidence back.
        call.record_relations(outcome.relations().witness.clone());

        Ok(Some(TermValue::iri(format!("{EX}done"))))
    });

    let mut functions = UserFunctionRegistry::new();
    functions.register_expr(FN_IRI, purrdf_sparql_eval::Arity::Exact(0), body);
    let bound = engine
        .bind_functions(functions, &env)
        .expect("an expression-bodied function has no SPARQL body to bind");

    let dataset = dataset();
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let outcome = engine
        .query_governed_in_operation(
            &*dataset,
            SparqlRequest {
                query: &format!("SELECT (<{FN_IRI}>() AS ?v) WHERE {{}}"),
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &bound,
                env: &env,
                focus_graph: Some(&dataset),
                ..QueryOptions::EMPTY
            },
            &state,
        )
        .expect("a governed run of a valid query is an outcome, never an error");

    assert!(
        opens.load(Ordering::Relaxed) > 0,
        "the nested run never reached the relation"
    );
    let attested = outcome
        .relations()
        .witness
        .get(REL)
        .expect("the relation the expression body invoked attests on the CALLER's receipt");
    assert!(
        attested
            .generations
            .contains(&IndexGeneration::declared(GENERATION)),
        "the generation the nested run's cursor declared survives the call boundary: {:?}",
        attested.generations
    );
}

/// An expression body that never re-enters the evaluator records nothing, and the
/// caller's receipt stays empty — the neighbouring case that shows the channel costs
/// a body which does not use it exactly nothing.
#[test]
fn an_expression_body_that_invokes_no_relation_attests_nothing() {
    let (relations, opens) = relations();
    let env = ExtensionEnv::over_relations(relations).expect("the declarations read cleanly");
    let engine = NativeSparqlEngine::new();

    let body: purrdf_sparql_eval::ExprFnBody =
        Arc::new(|_call: &ExprFnCall<'_>| Ok(Some(TermValue::iri(format!("{EX}done")))));
    let mut functions = UserFunctionRegistry::new();
    functions.register_expr(FN_IRI, purrdf_sparql_eval::Arity::Exact(0), body);
    let bound = engine
        .bind_functions(functions, &env)
        .expect("an expression-bodied function has no SPARQL body to bind");

    let dataset = dataset();
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let outcome = engine
        .query_governed_in_operation(
            &*dataset,
            SparqlRequest {
                query: &format!("SELECT (<{FN_IRI}>() AS ?v) WHERE {{}}"),
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &bound,
                env: &env,
                focus_graph: Some(&dataset),
                ..QueryOptions::EMPTY
            },
            &state,
        )
        .expect("a governed run of a valid query is an outcome, never an error");

    assert_eq!(opens.load(Ordering::Relaxed), 0);
    assert!(
        outcome.relations().witness.get(REL).is_none(),
        "a body that invoked nothing attests nothing"
    );
}

/// A second registry over the same relation, for the nested run to hold, SHARING
/// `opens` with the outer fixture.
///
/// A fresh registry instance because the nested query prepares and evaluates against
/// its own environment — but the same counter, because the assertion is that the
/// nested run really invoked the relation, and a counter the nested run does not
/// share would leave that unobservable.
fn relations_only(opens: Arc<AtomicU64>) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(FlagRelation {
            modes: [BindingPattern::from_code("ff")],
            opens,
        }),
    );
    registry
}

// ── Bound once per (text, environment) ─────────────────────────────────────────

/// A function body is PARSED once per (body text, environment), not once per bind.
///
/// Binding is cheap to repeat on purpose: it goes through the engine's own plan
/// cache, keyed on the body text and the environment's identity. That is what makes
/// it safe for `purrdf-shapes` to bind at each of several validation entry points
/// rather than having to thread one bound registry through all of them — the second
/// bind of a body is a cache HIT and reuses the first's prepared, feasibility-ordered
/// plan.
///
/// Asserted on the cache's own production counters rather than on a test-only probe.
/// A `#[cfg(test)]` counter would be conditional behaviour compiled differently in
/// test and production, which is the thing this workspace refuses everywhere else;
/// and a counter that merely correlated with binding today would stop correlating the
/// moment anything else touched the same path.
#[test]
fn a_body_is_parsed_once_per_text_and_environment() {
    let (relations, _) = relations();
    let engine = NativeSparqlEngine::new();
    let env = ExtensionEnv::over_relations(relations).expect("the declarations read cleanly");

    let before = engine.plan_cache_stats();
    engine
        .bind_functions(functions(REL), &env)
        .expect("the body parses and admits");
    let first = engine.plan_cache_stats();
    assert_eq!(
        first.misses - before.misses,
        1,
        "the first bind parses the body exactly once"
    );

    // Bind AGAIN, same text, same environment.
    engine
        .bind_functions(functions(REL), &env)
        .expect("the body parses and admits");
    let second = engine.plan_cache_stats();
    assert_eq!(
        second.misses, first.misses,
        "the second bind parsed nothing: same text, same environment, cache hit"
    );
    assert_eq!(
        second.hits - first.hits,
        1,
        "and it was served from the cache rather than skipped"
    );
}

/// A DIFFERENT environment is a different key, so the same text is parsed again.
///
/// The other half of the invariant above, and the one that makes it correct rather
/// than merely cheap: if the environment were not part of the key, the second bind
/// would reuse a plan in which the relation IRI is an ordinary triple pattern — the
/// defect this whole branch exists to fix, arriving through the cache.
#[test]
fn a_different_environment_reparses_the_same_body() {
    let (relations, _) = relations();
    let engine = NativeSparqlEngine::new();

    let wired = ExtensionEnv::over_relations(relations).expect("the declarations read cleanly");
    engine
        .bind_functions(functions(REL), &wired)
        .expect("the body parses and admits");
    let after_wired = engine.plan_cache_stats();

    // The SAME body text, against an environment that registers nothing.
    engine
        .bind_functions(functions(REL), ExtensionEnv::empty())
        .expect("the body parses and admits");
    let after_bare = engine.plan_cache_stats();
    assert_eq!(
        after_bare.misses - after_wired.misses,
        1,
        "a different environment is a different cache key, so the text is parsed again; \
         reusing the first plan would mean the relation call had silently become an \
         ordinary triple pattern"
    );
}
