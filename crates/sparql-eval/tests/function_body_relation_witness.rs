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
    AggregateRegistry, BindingPattern, EvalError, ExtensionEnv, GovernedOutcome, GovernorState,
    IndexGeneration, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, QueryGovernors, QueryOptions, TypeConstraint, UserFnBody,
    UserFnParam, UserFunction, UserFunctionRegistry, Volatility,
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
