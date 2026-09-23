// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A prepared execution's declared parameters are counted as bound when its property
//! -function calls are admitted, and the refusal to run one unbound is what makes that
//! sound.
//!
//! A property-function call is admitted at PREPARE time, in the access pattern the
//! query text shows. A caller that prepares once and runs the plan many times —
//! binding a different term to the same variable each time — has a variable that is
//! free in the text and bound in every execution, and no reading of the text can see
//! that. Admitted as free, the call is matched against whatever mode the relation
//! declares for a free position, and the relation is then handed a bound argument in a
//! pattern it never declared.
//!
//! [`NativeSparqlEngine::prepare_execution`] closes that gap: the parameters it is
//! given are exactly the variables every run binds, so admission counts them as bound.
//! This file holds it to both halves of the bargain:
//!
//! * **It buys something.** A call no mode serves with the variable free is admitted
//!   with it declared, and the relation receives it bound.
//! * **It is enforced.** An execution refuses to run — by name, with a typed
//!   diagnostic code — while a declared parameter is unbound, so a call admitted on a
//!   parameter being bound is never invoked with it free. Both directions are executed
//!   here, and so is the neighbouring case that must NOT be refused: an execution that
//!   declared nothing still runs with nothing bound.
//!
//! The fixture relation declares exactly one access mode, `bf`, so "admitted" and
//! "refused" are decided by the declaration alone rather than by anything the relation
//! chooses to tolerate at run time. Fixture IRIs are `example.org`; PurRDF mints none.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, InternedOutcome, NativeSparqlEngine, PfArgs, PfArity,
    PfCursor, PfRow, PreparedExecution, PropertyFunction, PropertyFunctionRegistry, QueryOptions,
    ShaclPrebinding, Volatility,
};

/// The predicate **this fixture** calls the relation by.
const RELATED: &str = "https://example.org/pf/related";

/// The one access mode the relation declares: the subject is an input, the object an
/// output. Nothing serves a free subject.
const RELATED_MODE: &str = "bf";

/// The query whose admission the declaration decides: both positions are variables, so
/// the text alone shows `ff`.
fn free_subject_query() -> String {
    format!("SELECT ?object WHERE {{ ( ?subject ) <{RELATED}> ( ?object ) }}")
}

fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("https://example.org/d/{local}"))
}

fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty dataset freezes")
}

// ---------------------------------------------------------------------------
// The fixture relation
// ---------------------------------------------------------------------------

/// A one-in, one-out relation that can only be invoked with its subject bound.
///
/// It emits exactly one row per invocation — the subject, and a term derived from it —
/// and it REFUSES a free subject rather than enumerating. That refusal is what makes the
/// invocation counter below meaningful: if a plan ever reached this relation with the
/// subject free, the error would surface as a failed evaluation rather than as a quietly
/// different answer.
#[derive(Debug)]
struct Related {
    /// The one declared mode, materialized so `modes` can hand out a slice.
    modes: [BindingPattern; 1],
    /// How many invocations arrived with the subject bound.
    bound_invocations: AtomicU64,
}

impl Related {
    fn new() -> Self {
        Self {
            modes: [BindingPattern::from_code(RELATED_MODE)],
            bound_invocations: AtomicU64::new(0),
        }
    }

    fn bound_invocations(&self) -> u64 {
        self.bound_invocations.load(Ordering::Relaxed)
    }
}

impl PropertyFunction for Related {
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
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let Some(TermValue::Iri(subject)) = args.get(0) else {
            return Err(EvalError::function(
                "the subject at position 0 is free or not an IRI; this relation is declared \
                 `bf` and cannot enumerate subjects"
                    .to_owned(),
            ));
        };
        self.bound_invocations.fetch_add(1, Ordering::Relaxed);
        let row: PfRow = vec![
            TermValue::iri(subject.as_str()),
            TermValue::iri(format!("{}/related", subject.as_str())),
        ];
        Ok(Box::new(OneRow { row: Some(row) }))
    }
}

/// The cursor [`Related::open`] returns: one row, then exhaustion.
struct OneRow {
    row: Option<PfRow>,
}

impl PfCursor for OneRow {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.row.take())
    }
}

/// The relation registered under the fixture predicate, and the handle a test reads its
/// invocation count back from.
fn registry() -> (PropertyFunctionRegistry, Arc<Related>) {
    let relation = Arc::new(Related::new());
    let registered: Arc<dyn PropertyFunction> = Arc::<Related>::clone(&relation);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(RELATED, registered);
    (registry, relation)
}

fn environment(registry: &PropertyFunctionRegistry) -> ExtensionEnv {
    ExtensionEnv::over_relations(registry.clone()).expect("the fixture declaration reads cleanly")
}

/// Run `execution` and count the rows its `SELECT` answered with.
fn run(
    engine: &NativeSparqlEngine,
    execution: &mut PreparedExecution,
    dataset: &RdfDataset,
    options: QueryOptions<'_>,
) -> Result<usize, purrdf_core::RdfDiagnostic> {
    engine.execute(execution, dataset, options, |outcome| match outcome {
        InternedOutcome::Solutions(solutions) => solutions.len(),
        InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
            panic!("a SELECT answers with solutions")
        }
    })
}

// ---------------------------------------------------------------------------
// What the declaration buys
// ---------------------------------------------------------------------------

/// **The same text is infeasible without the declaration and admitted with it.**
///
/// Both preparations are executed. Without the declaration the subject is a free
/// variable, the invocation pattern is `ff`, and the one declared mode does not subsume
/// it — so admission fails at prepare, which is where an infeasible chain is supposed to
/// fail. With the declaration the pattern is `bf`, which is the mode itself, and the
/// execution is admitted.
///
/// The refusal is asserted to name the mode it could not serve, so a future change that
/// made the prepare fail for some unrelated reason would not be read as this one.
#[test]
fn the_declaration_is_what_admits_a_call_whose_argument_the_text_leaves_free() {
    let (registry, _relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };

    let undeclared = engine.prepare_execution(&free_subject_query(), None, &[], options);
    let diagnostic = undeclared.expect_err("a free subject reaches no declared mode");
    assert_eq!(
        diagnostic.code, "native-sparql-property-function",
        "the admission failure is the property-function seam's: {diagnostic}"
    );
    assert!(
        diagnostic.to_string().contains(RELATED_MODE),
        "the refusal names the mode the relation declares: {diagnostic}"
    );

    let declared = engine
        .prepare_execution(&free_subject_query(), None, &["subject"], options)
        .expect("the declared parameter makes the call feasible");
    assert_eq!(
        declared.slot("subject"),
        Some(0),
        "the execution carries the declaration it was admitted on"
    );
}

/// **Declared and bound runs; declared and unbound is refused by name.**
///
/// The two directions of the promise, over ONE execution, so nothing but the binding
/// differs between them. The run with the parameter bound answers a row and the
/// relation really was invoked with the subject bound — which is the thing the
/// declaration promised and the reason the call was admitted at all. The run with it
/// unbound is a typed diagnostic naming `subject`, and the relation is not invoked
/// again, so the refusal happened before evaluation rather than inside it.
#[test]
fn a_declared_parameter_must_be_bound_and_the_refusal_names_it() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = empty_dataset();
    let mut execution = engine
        .prepare_execution(&free_subject_query(), None, &["subject"], options)
        .expect("the declared parameter makes the call feasible");

    // Unbound: refused, by name, before anything runs.
    let refused = run(&engine, &mut execution, &dataset, options)
        .expect_err("an execution whose declaration is not honoured must not evaluate");
    assert_eq!(
        refused.code, "native-sparql-execution-parameter",
        "the refusal is typed rather than a generic evaluation failure: {refused}"
    );
    assert!(
        refused.to_string().contains("subject"),
        "and it names the parameter that was not bound: {refused}"
    );
    assert_eq!(
        relation.bound_invocations(),
        0,
        "the refused execution invoked nothing"
    );

    // A DIFFERENT name is not the declared one: binding it is refused, and it leaves the
    // declared parameter unbound, so the run is still refused.
    let wrong = execution
        .bind_named("object", iri("a"))
        .expect_err("binding some other variable does not honour the declaration");
    assert_eq!(wrong.code, "native-sparql-execution-parameter");
    assert!(
        run(&engine, &mut execution, &dataset, options).is_err(),
        "the declared parameter is still unbound"
    );

    // Bound: the execution runs, and the relation received the promised binding.
    execution
        .bind_named("subject", iri("a"))
        .expect("the declared parameter binds");
    assert_eq!(
        run(&engine, &mut execution, &dataset, options)
            .expect("an execution whose declaration is honoured evaluates"),
        1,
        "the relation answered its one row"
    );
    assert_eq!(
        relation.bound_invocations(),
        1,
        "and it was invoked with the subject bound, which is what the declaration promised"
    );
}

/// **The neighbouring valid cases still run, which is what keeps the refusal from being
/// an over-refusal.**
///
/// Three of them, all executed:
///
/// * an execution that declared NOTHING runs with nothing bound — the ordinary prepare
///   is untouched by any of this;
/// * an execution that declared a parameter the text never uses beside one it does
///   runs once both are bound, because a declaration counts a variable as bound and
///   never makes an unmentioned one matter;
/// * the same execution runs again, and again, with a different term each time — which
///   is the whole point of preparing once and binding per call, and which takes the
///   retained substituted plan from the third run on.
#[test]
fn declaring_nothing_still_runs_and_one_execution_answers_many_bindings() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = empty_dataset();

    // An execution that declares nothing, run with nothing.
    let constant =
        format!("SELECT ?object WHERE {{ ( <https://example.org/d/a> ) <{RELATED}> ( ?object ) }}");
    let mut undeclared = engine
        .prepare_execution(&constant, None, &[], options)
        .expect("a constant subject is bound without any declaration");
    assert!(
        undeclared.parameters().is_empty(),
        "an ordinary prepare declares nothing"
    );
    assert_eq!(
        run(&engine, &mut undeclared, &dataset, options).expect("and it runs with nothing bound"),
        1
    );

    // A declaration naming the used parameter and a spare, both bound.
    let mut spare = engine
        .prepare_execution(
            &free_subject_query(),
            None,
            &["subject", "unrelated"],
            options,
        )
        .expect("the declared parameter makes the call feasible");
    spare.bind_named("subject", iri("b")).expect("declared");
    spare.bind_named("unrelated", iri("c")).expect("declared");
    assert_eq!(
        run(&engine, &mut spare, &dataset, options)
            .expect("a declared parameter the text never uses does not make it unrunnable"),
        1
    );

    // One execution, many bindings.
    let mut declared = engine
        .prepare_execution(&free_subject_query(), None, &["subject"], options)
        .expect("the declared parameter makes the call feasible");
    let slot = declared.slot("subject").expect("declared");
    for local in ["d", "e", "f", "g"] {
        declared.bind(slot, iri(local)).expect("declared");
        assert_eq!(
            run(&engine, &mut declared, &dataset, options)
                .expect("the same execution answers for another term"),
            1
        );
    }
    assert_eq!(
        relation.bound_invocations(),
        6,
        "six executions, six bound invocations, and not one of them free"
    );
}

/// **A plan admitted under a declaration is not the plan admitted without one.**
///
/// The two are cached separately, because they are admitted differently: the declared
/// one admitted a call the undeclared one cannot. A cache key that ignored the
/// declaration would hand the second caller the first caller's plan — a plan that is
/// only sound behind a handle that refuses to run with its parameter unbound, handed to
/// a caller that binds nothing.
///
/// Asserted by preparing the declared execution FIRST and then asking for the same text
/// with no declaration: that must still be refused, which it could not be if the cache
/// had answered it with the declared plan. And one declaration spelled in two orders
/// is one entry, read off the engine's own plan count.
#[test]
fn the_declaration_is_part_of_the_plan_identity() {
    let (registry, _relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };

    engine
        .prepare_execution(&free_subject_query(), None, &["subject"], options)
        .expect("the declared parameter makes the call feasible");
    assert!(
        engine
            .prepare_query_with_options(&free_subject_query(), None, options)
            .is_err(),
        "the undeclared prepare of the same text is not answered with the declared plan"
    );
    assert!(
        engine
            .prepare_execution(&free_subject_query(), None, &[], options)
            .is_err(),
        "nor is an execution that declares nothing"
    );

    let text = format!(
        "SELECT ?object WHERE {{ ( ?subject ) <{RELATED}> ( ?object ) . ( ?other ) <{RELATED}> ( \
         ?fourth ) }}"
    );
    let before = engine.cached_plan_count();
    engine
        .prepare_execution(&text, None, &["subject", "other"], options)
        .expect("both subjects declared");
    assert_eq!(engine.cached_plan_count(), before + 1, "one new plan");
    engine
        .prepare_execution(&text, None, &["other", "subject"], options)
        .expect("the same declaration, spelled in the other order");
    assert_eq!(
        engine.cached_plan_count(),
        before + 1,
        "the declaration is normalized, so one order and the other reach one cache entry"
    );
}

// ---------------------------------------------------------------------------
// Where the declaration reaches
// ---------------------------------------------------------------------------

/// A data predicate, distinct from the relation's.
const LINKED: &str = "https://example.org/pf/linked";

/// One `LINKED` triple, so a data atom ahead of a call yields exactly one row.
fn linked_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/d/s");
    let predicate = builder.intern_iri(LINKED);
    let object = builder.intern_iri("https://example.org/d/o");
    builder.push_quad(subject, predicate, object, None);
    builder.freeze().expect("the fixture dataset freezes")
}

/// **A call that follows a data atom is admitted on the declaration AND invoked bound.**
///
/// The data atom is scheduled first, so the call is the right operand of the `LATERAL`
/// the planner rebuilds the group into — the shape every call written after another
/// atom has. Admission counted the parameter as bound there, so a run must really
/// hand it to the relation bound; the relation refuses a free subject, so a run that
/// did not would fail rather than answer.
#[test]
fn a_call_after_a_data_atom_is_invoked_with_its_declared_parameter_bound() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = linked_dataset();
    let text = format!("SELECT ?x WHERE {{ ?s <{LINKED}> ?o . ( ?subject ) <{RELATED}> ( ?x ) }}");
    let mut execution = engine
        .prepare_execution(&text, None, &["subject"], options)
        .expect("the declared parameter makes the call feasible after the data atom");
    let slot = execution.slot("subject").expect("declared");
    for (runs, local) in (1_u64..).zip(["a", "b", "c"]) {
        execution.bind(slot, iri(local)).expect("declared");
        assert_eq!(
            run(&engine, &mut execution, &dataset, options)
                .expect("the call is invoked with the declared parameter bound"),
            1,
            "one data row, one related term"
        );
        assert_eq!(relation.bound_invocations(), runs);
    }
}

/// **A declared parameter does not reach an `OPTIONAL`'s right arm, so a call there that
/// needs it is refused at prepare — and the neighbouring calls it does reach are not.**
///
/// A run binds a parameter by joining it onto the core pattern and writing it into the
/// leaves it may restrict, and an `OPTIONAL`'s right arm is not one of them: restricting
/// it would change which left rows survive. A call there is invoked with the parameter
/// free on every run, so admitting it on the promise would only move the refusal from
/// prepare to every execution. All three are executed: the call on `?subject` inside the
/// `OPTIONAL` is refused; the same call in the group itself is admitted and runs; and a
/// call inside the `OPTIONAL` on a variable the arm binds for itself is admitted and runs
/// too, so the refusal is about the parameter's reach and not about the arm.
#[test]
fn a_declared_parameter_does_not_reach_an_optional_arm_and_its_neighbours_still_run() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = linked_dataset();

    let unreached = format!(
        "SELECT ?o WHERE {{ ?s <{LINKED}> ?o OPTIONAL {{ ( ?subject ) <{RELATED}> ( ?x ) }} }}"
    );
    let refused = engine
        .prepare_execution(&unreached, None, &["subject"], options)
        .expect_err("the declaration does not bind a parameter inside an OPTIONAL arm");
    assert_eq!(
        refused.code, "native-sparql-property-function",
        "refused as the infeasible call it is: {refused}"
    );
    assert!(refused.to_string().contains(RELATED_MODE), "{refused}");

    let in_group =
        format!("SELECT ?o WHERE {{ ?s <{LINKED}> ?o . ( ?subject ) <{RELATED}> ( ?x ) }}");
    let mut execution = engine
        .prepare_execution(&in_group, None, &["subject"], options)
        .expect("the declaration reaches a call in the group itself");
    execution.bind_named("subject", iri("a")).expect("declared");
    assert_eq!(
        run(&engine, &mut execution, &dataset, options).expect("and it runs bound"),
        1
    );

    let own_binding = format!(
        "SELECT ?o WHERE {{ ?s <{LINKED}> ?o \
         OPTIONAL {{ ?t <{LINKED}> ?u . ( ?t ) <{RELATED}> ( ?x ) }} }}"
    );
    let mut execution = engine
        .prepare_execution(&own_binding, None, &["subject"], options)
        .expect("a call on a variable its own arm binds is feasible");
    execution.bind_named("subject", iri("a")).expect("declared");
    assert_eq!(
        run(&engine, &mut execution, &dataset, options)
            .expect("and it runs, driven by the arm's own row"),
        1
    );
    assert_eq!(relation.bound_invocations(), 2);
}

/// **A `FILTER EXISTS` over the core sees the declared parameter bound, because the rows
/// it is evaluated over carry it.**
///
/// The filter sits above the core the parameter is joined onto, so every row it tests
/// already binds the parameter, and a correlated `EXISTS` hands that value to the call
/// inside it. Executed rather than only admitted: the relation refuses a free subject,
/// so a run that reached it free would fail.
#[test]
fn a_filter_exists_over_the_core_invokes_its_call_with_the_declared_parameter_bound() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let text = format!(
        "SELECT ?o WHERE {{ ?s <{LINKED}> ?o FILTER EXISTS {{ ( ?subject ) <{RELATED}> ( ?x ) }} }}"
    );
    let mut execution = engine
        .prepare_execution(&text, None, &["subject"], options)
        .expect("the rows the filter tests carry the declared parameter");
    execution.bind_named("subject", iri("a")).expect("declared");
    assert_eq!(
        run(&engine, &mut execution, &linked_dataset(), options)
            .expect("the call inside the EXISTS is invoked bound"),
        1
    );
    assert_eq!(relation.bound_invocations(), 1);
}

// ---------------------------------------------------------------------------
// What a call is admitted against is what it is invoked with
// ---------------------------------------------------------------------------

/// **A call is admitted against the variables its evaluation actually hands it: an
/// `OPTIONAL` arm's call does not see the enclosing group's bindings, and is refused at
/// prepare rather than on every run — while a call in a nested group does.**
///
/// An `OPTIONAL` evaluates its right operand on its own and matches it against the
/// left afterwards, so a call inside it is invoked with nothing the left binds. A plan
/// that admitted it against the left's bindings would be refused by the relation on
/// every run instead — the same question, answered later and once per execution.
///
/// A nested group is different: `{ A . { call } }` is `Join(A, call)`, and a join is
/// associative and commutative, so the planner joins the nested group's call into the
/// enclosing chain and drives it with `A`'s rows exactly as it drives a call written
/// into the group itself. Both are executed, and both answer with the relation invoked
/// bound.
#[test]
fn a_call_is_admitted_against_what_its_evaluation_hands_it() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = linked_dataset();

    let refused_text =
        format!("SELECT ?o WHERE {{ ?s <{LINKED}> ?o OPTIONAL {{ ( ?s ) <{RELATED}> ( ?x ) }} }}");
    let refused = engine
        .prepare_execution(&refused_text, None, &[], options)
        .expect_err("the call's subject is free where it is evaluated");
    assert_eq!(
        refused.code, "native-sparql-property-function",
        "an OPTIONAL arm: refused as the infeasible call it is: {refused}"
    );
    assert!(
        refused.to_string().contains(RELATED_MODE),
        "an OPTIONAL arm: {refused}"
    );

    for (admitted, neighbour) in [
        (
            1,
            format!("SELECT ?x WHERE {{ ?s <{LINKED}> ?o . ( ?s ) <{RELATED}> ( ?x ) }}"),
        ),
        (
            2,
            format!("SELECT ?x WHERE {{ ?s <{LINKED}> ?o . {{ ( ?s ) <{RELATED}> ( ?x ) }} }}"),
        ),
    ] {
        let mut execution = engine
            .prepare_execution(&neighbour, None, &[], options)
            .expect("a call joined into the group is driven by the rows before it");
        assert_eq!(
            run(&engine, &mut execution, &dataset, options).expect("and it answers"),
            1
        );
        assert_eq!(relation.bound_invocations(), admitted, "{neighbour}");
    }
}

// ---------------------------------------------------------------------------
// A parameter bound to a dataset blank node
// ---------------------------------------------------------------------------

/// The predicate **this fixture** calls the blank-accepting relation by.
const OWNS: &str = "https://example.org/pf/owns";

/// A one-in, one-out relation, declared `bf` only, whose subjects are blank nodes.
///
/// Each subject it knows owns one distinct literal, keyed by the blank node's label, so
/// the row an invocation answers with says WHICH blank node it was handed — an oracle
/// that tells "bound to this blank" from "bound to another blank" and from "invoked
/// free". A free subject is refused rather than enumerated, so a run that reached the
/// relation with the parameter free fails instead of answering.
#[derive(Debug)]
struct Owns {
    modes: [BindingPattern; 1],
    /// Every subject label an invocation was handed, in order.
    seen: std::sync::Mutex<Vec<String>>,
}

impl Owns {
    fn new() -> Self {
        Self {
            modes: [BindingPattern::from_code(RELATED_MODE)],
            seen: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<String> {
        self.seen.lock().expect("unpoisoned").clone()
    }

    /// The literal `label` owns, or `None` for a blank node this relation holds nothing
    /// for.
    fn owned(label: &str) -> Option<&'static str> {
        match label {
            "b1" => Some("owned by the first"),
            "b2" => Some("owned by the second"),
            _ => None,
        }
    }
}

impl PropertyFunction for Owns {
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
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let Some(subject) = args.get(0) else {
            return Err(EvalError::function(
                "the subject at position 0 is free; this relation is declared `bf` and cannot \
                 enumerate subjects"
                    .to_owned(),
            ));
        };
        let TermValue::Blank { label, .. } = subject else {
            return Err(EvalError::function(format!(
                "the subject at position 0 is {subject:?}, not a blank node"
            )));
        };
        self.seen.lock().expect("unpoisoned").push(label.clone());
        let row = Self::owned(label)
            .map(|owned| -> PfRow { vec![subject.clone(), TermValue::simple_literal(owned)] });
        Ok(Box::new(OneRow { row }))
    }
}

/// Two blank subjects, each with one `LINKED` triple, so the dataset holds both blank
/// nodes as terms and a data atom over `LINKED` yields two rows.
fn blank_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(LINKED);
    for (label, object) in [("b1", "o1"), ("b2", "o2")] {
        let subject = builder.intern_blank(label, purrdf_core::BlankScope::DEFAULT);
        let object = builder.intern_iri(&format!("https://example.org/d/{object}"));
        builder.push_quad(subject, predicate, object, None);
    }
    builder.freeze().expect("the fixture dataset freezes")
}

/// The dataset's own id for the blank node `label`, through the only value-to-id door a
/// dataset view has, so the id handed to `bind_id` is the dataset's and not invented.
fn blank_id(dataset: &RdfDataset, label: &str) -> purrdf_core::TermId {
    dataset
        .term_id_by_value(&TermValue::blank(label))
        .expect("the fixture dataset holds the blank node")
}

/// Run `execution` and render column `column` of every answer row.
fn answers(
    engine: &NativeSparqlEngine,
    execution: &mut PreparedExecution,
    dataset: &RdfDataset,
    options: QueryOptions<'_>,
    column: usize,
) -> Result<Vec<String>, purrdf_core::RdfDiagnostic> {
    engine.execute(execution, dataset, options, |outcome| match outcome {
        InternedOutcome::Solutions(solutions) => solutions
            .rows()
            .iter()
            .map(|row| format!("{:?}", solutions.cell(row, column)))
            .collect(),
        InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
            panic!("a SELECT answers with solutions")
        }
    })
}

/// **A declared parameter bound to a dataset blank node reaches the call bound — through
/// the id door and through the value door — and each blank node answers with its own
/// row.**
///
/// A blank node written into a query PATTERN is an anonymous variable, which is why the
/// rewrite never writes a blank value into a matched position. A bound VALUE that is a
/// dataset blank node is a term identity instead, and a call's argument is an invocation
/// input: the relation must be handed that blank node, bound, exactly as it is handed an
/// IRI. The relation below serves only a bound subject and answers per blank node, so
/// every run tells three outcomes apart — refused (invoked free), the first blank's
/// row, and the second blank's row.
///
/// Two shapes are run: the call alone, and the call after a data atom that does not
/// mention the parameter (the right operand of a `LATERAL`, driven once per data row).
/// In both, the parameter is bound only by the run, never by the text.
///
/// The neighbouring refusal is executed too: the same text prepared WITHOUT the
/// declaration is refused at prepare, naming the one mode it could not serve.
#[test]
fn a_parameter_bound_to_a_dataset_blank_node_reaches_the_call_bound() {
    let relation = Arc::new(Owns::new());
    let registered: Arc<dyn PropertyFunction> = Arc::<Owns>::clone(&relation);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(OWNS, registered);
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = blank_dataset();

    let alone = format!("SELECT ?x WHERE {{ ( ?subject ) <{OWNS}> ( ?x ) }}");
    let after_atom =
        format!("SELECT ?x ?o WHERE {{ ?s <{LINKED}> ?o . ( ?subject ) <{OWNS}> ( ?x ) }}");

    let refused = engine
        .prepare_execution(&alone, None, &[], options)
        .expect_err("undeclared, the subject is free and no declared mode serves it");
    assert_eq!(refused.code, "native-sparql-property-function", "{refused}");
    assert!(refused.to_string().contains(RELATED_MODE), "{refused}");

    for (text, rows_per_run) in [(&alone, 1), (&after_atom, 2)] {
        let mut execution = engine
            .prepare_execution(text, None, &["subject"], options)
            .expect("the declared parameter makes the call feasible");
        let slot = execution.slot("subject").expect("declared");
        for (label, owned) in [
            ("b1", "owned by the first"),
            ("b2", "owned by the second"),
            ("b1", "owned by the first"),
        ] {
            for door in ["id", "value"] {
                let before = relation.seen().len();
                match door {
                    "id" => execution
                        .bind_id(slot, &*dataset, blank_id(&dataset, label))
                        .expect("the id door binds a blank node"),
                    _ => execution
                        .bind(slot, TermValue::blank(label))
                        .expect("the value door binds a blank node"),
                }
                let answered = answers(&engine, &mut execution, &dataset, options, 0)
                    .unwrap_or_else(|diagnostic| {
                        panic!(
                            "{text} bound to _:{label} through the {door} door must run with \
                             the call's subject bound: {diagnostic}"
                        )
                    });
                assert_eq!(
                    answered.len(),
                    rows_per_run,
                    "{text}, _:{label}, {door} door: {answered:?}"
                );
                assert!(
                    answered.iter().all(|cell| cell.contains(owned)),
                    "{text}, _:{label}, {door} door: every row is the one _:{label} owns, \
                     and no other blank's: {answered:?}"
                );
                assert_eq!(
                    relation.seen()[before..],
                    vec![label.to_owned(); rows_per_run],
                    "{text}, _:{label}, {door} door: one invocation per row that drives the \
                     call, each handed exactly that blank node"
                );
            }
        }
    }
}

/// **A blank node the dataset does not hold is bound, reaches the call, and answers
/// nothing — rather than being refused or answered as another blank.**
///
/// The value door takes a term with no dataset behind it. The relation holds nothing
/// for `_:b9`, so the honest answer is zero rows from exactly one bound invocation;
/// the neighbouring `_:b2` through the same handle answers its own row, so the empty
/// answer is not the handle having stopped answering.
#[test]
fn a_blank_node_the_dataset_does_not_hold_is_still_bound_into_the_call() {
    let relation = Arc::new(Owns::new());
    let registered: Arc<dyn PropertyFunction> = Arc::<Owns>::clone(&relation);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(OWNS, registered);
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = blank_dataset();
    let text = format!("SELECT ?x WHERE {{ ( ?subject ) <{OWNS}> ( ?x ) }}");
    let mut execution = engine
        .prepare_execution(&text, None, &["subject"], options)
        .expect("the declared parameter makes the call feasible");

    execution
        .bind_named("subject", TermValue::blank("b9"))
        .expect("declared");
    let answered = answers(&engine, &mut execution, &dataset, options, 0)
        .expect("a blank node the dataset does not hold is still bound into the call");
    assert!(answered.is_empty(), "{answered:?}");
    assert_eq!(relation.seen(), ["b9".to_owned()]);

    execution
        .bind_named("subject", TermValue::blank("b2"))
        .expect("declared");
    let answered = answers(&engine, &mut execution, &dataset, options, 0)
        .expect("the neighbouring held blank node answers");
    assert_eq!(answered.len(), 1, "{answered:?}");
    assert!(answered[0].contains("owned by the second"), "{answered:?}");
    assert_eq!(relation.seen(), ["b9".to_owned(), "b2".to_owned()]);
}

// ---------------------------------------------------------------------------
// Admitted under the SHACL pre-binding rewrite
// ---------------------------------------------------------------------------

/// **Prepared for the SHACL pre-binding rewrite, a call in an `OPTIONAL` arm is admitted
/// with the parameter bound and invoked bound — for a blank node too — and the handle
/// refuses to run under the ordinary rewrite, which would invoke it free.**
///
/// The SHACL rewrite binds a parameter in every property-function call in the query,
/// an `OPTIONAL`'s right arm included; the ordinary rewrite stops at that arm. So the
/// same text is refused when prepared for the ordinary rewrite (it would be invoked
/// free on every run) and admitted when prepared for the SHACL one. Every run then
/// answers with the row the bound blank node owns, and a different blank node answers
/// differently. The handle's one refusal is executed beside its neighbours: run under
/// the ordinary rewrite it is refused, by the parameter code; run under the SHACL one
/// it answers; and a handle prepared for the ordinary rewrite still runs under the
/// SHACL one, whose reach contains it.
#[test]
fn a_call_in_an_optional_arm_is_bound_under_the_shacl_rewrite_and_only_there() {
    let relation = Arc::new(Owns::new());
    let registered: Arc<dyn PropertyFunction> = Arc::<Owns>::clone(&relation);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(OWNS, registered);
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let ordinary = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let shacl = QueryOptions {
        prebinding: ShaclPrebinding::Applied,
        ..ordinary
    };
    let dataset = blank_dataset();
    let text = format!("SELECT ?x WHERE {{ OPTIONAL {{ ( ?subject ) <{OWNS}> ( ?x ) }} }}");

    let refused = engine
        .prepare_execution(&text, None, &["subject"], ordinary)
        .expect_err("the ordinary rewrite does not bind a parameter inside an OPTIONAL arm");
    assert_eq!(refused.code, "native-sparql-property-function", "{refused}");
    assert!(refused.to_string().contains(RELATED_MODE), "{refused}");

    let mut execution = engine
        .prepare_execution(&text, None, &["subject"], shacl)
        .expect("the SHACL rewrite binds the parameter in the OPTIONAL arm's call");
    let slot = execution.slot("subject").expect("declared");
    for (label, owned) in [
        ("b1", "owned by the first"),
        ("b2", "owned by the second"),
        ("b1", "owned by the first"),
    ] {
        execution
            .bind_id(slot, &*dataset, blank_id(&dataset, label))
            .expect("the id door binds a blank node");
        let answered = answers(&engine, &mut execution, &dataset, shacl, 0)
            .unwrap_or_else(|diagnostic| panic!("_:{label} under the SHACL rewrite: {diagnostic}"));
        assert_eq!(answered.len(), 1, "_:{label}: {answered:?}");
        assert!(answered[0].contains(owned), "_:{label}: {answered:?}");
        assert_eq!(relation.seen().last(), Some(&label.to_owned()));
    }
    let invoked = relation.seen().len();

    let wrong_lane = answers(&engine, &mut execution, &dataset, ordinary, 0)
        .expect_err("a handle admitted for the SHACL rewrite does not run under the other one");
    assert_eq!(
        wrong_lane.code, "native-sparql-execution-parameter",
        "{wrong_lane}"
    );
    assert_eq!(
        relation.seen().len(),
        invoked,
        "the refused run never reached the relation"
    );

    let core = format!("SELECT ?x WHERE {{ ( ?subject ) <{OWNS}> ( ?x ) }}");
    let mut plain = engine
        .prepare_execution(&core, None, &["subject"], ordinary)
        .expect("a call at the core is bound under the ordinary rewrite");
    plain
        .bind(0, TermValue::blank("b2"))
        .expect("the value door binds a blank node");
    let answered = answers(&engine, &mut plain, &dataset, shacl, 0)
        .expect("a handle prepared for the ordinary rewrite runs under the SHACL one");
    assert_eq!(answered.len(), 1, "{answered:?}");
    assert!(answered[0].contains("owned by the second"), "{answered:?}");
}
