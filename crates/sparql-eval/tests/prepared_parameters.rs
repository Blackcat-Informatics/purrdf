// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A prepare that is told which variables an execution will substitute, and the
//! promise that makes it sound.
//!
//! A property-function call is admitted at PREPARE time, in the access pattern the
//! query text shows. A caller that prepares once and runs the plan many times —
//! substituting a different term for the same variable each time — has a variable that
//! is free in the text and bound in every execution, and no reading of the text can see
//! that. Admitted as free, the call is matched against whatever mode the relation
//! declares for a free position, and the relation is then handed a bound argument in a
//! pattern it never declared.
//!
//! [`NativeSparqlEngine::prepare_query_with_parameters`] closes that gap by taking the
//! declaration explicitly. This file holds it to both halves of the bargain:
//!
//! * **It buys something.** A call no mode serves with the variable free is admitted
//!   with it declared, and the relation receives it bound.
//! * **It is enforced.** A plan admitted on the declaration is refused — by name, with
//!   a typed diagnostic code — when an execution does not supply one of the names. Both
//!   directions are executed here, and so is the neighbouring case that must NOT be
//!   refused: a plan that declared nothing still runs with nothing supplied.
//!
//! The fixture relation declares exactly one access mode, `bf`, so "admitted" and
//! "refused" are decided by the declaration alone rather than by anything the relation
//! chooses to tolerate at run time. Fixture IRIs are `example.org`; PurRDF mints none.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, ExtensionEnv, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryOptions, Volatility,
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

/// The rows a `SELECT` answered with.
fn rows(result: SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        other => panic!("a SELECT answers with solutions, got {other:?}"),
    }
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
/// plan is admitted.
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

    let undeclared = engine.prepare_query_with_options(&free_subject_query(), None, options);
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
        .prepare_query_with_parameters(&free_subject_query(), None, options, &["subject"])
        .expect("the declared parameter makes the call feasible");
    assert_eq!(
        declared.parameters(),
        ["subject".to_owned()],
        "the plan carries the declaration it was admitted on"
    );
}

/// **Declared and supplied runs; declared and omitted is refused by name.**
///
/// The two directions of the promise, over ONE plan, so nothing but the substitution
/// list differs between them. The run that supplies the parameter answers a row and the
/// relation really was invoked with the subject bound — which is the thing the
/// declaration promised and the reason the plan was admitted at all. The run that omits
/// it is a typed diagnostic naming `subject`, and the relation is not invoked again, so
/// the refusal happened before evaluation rather than inside it.
#[test]
fn a_declared_parameter_must_be_supplied_and_the_refusal_names_it() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = empty_dataset();
    let prepared = engine
        .prepare_query_with_parameters(&free_subject_query(), None, options, &["subject"])
        .expect("the declared parameter makes the call feasible");

    // Supplied: the plan runs, and the relation received the promised binding.
    let answered = engine
        .query_prepared(
            &dataset,
            &prepared,
            &[("subject".to_owned(), iri("a"))],
            options,
        )
        .expect("a plan whose declaration is honoured evaluates");
    assert_eq!(rows(answered), 1, "the relation answered its one row");
    assert_eq!(
        relation.bound_invocations(),
        1,
        "and it was invoked with the subject bound, which is what the declaration promised"
    );

    // Omitted: refused, by name, before anything runs.
    let refused = engine
        .query_prepared(&dataset, &prepared, &[], options)
        .expect_err("a plan whose declaration is not honoured must not evaluate");
    assert_eq!(
        refused.code, "native-sparql-prepared-parameter",
        "the refusal is typed rather than a generic evaluation failure: {refused}"
    );
    assert!(
        refused.to_string().contains("subject"),
        "and it names the parameter that was not supplied: {refused}"
    );
    assert_eq!(
        relation.bound_invocations(),
        1,
        "the refused execution invoked nothing: the count is still the supplied run's"
    );

    // A DIFFERENT name is not the declared one, and is refused the same way — the check
    // is on the promise, not on the presence of some substitution or other.
    let wrong = engine
        .query_prepared(
            &dataset,
            &prepared,
            &[("object".to_owned(), iri("a"))],
            options,
        )
        .expect_err("substituting some other variable does not honour the declaration");
    assert_eq!(wrong.code, "native-sparql-prepared-parameter");
    assert!(wrong.to_string().contains("subject"), "{wrong}");
}

/// **The neighbouring valid cases still run, which is what keeps the refusal from being
/// an over-refusal.**
///
/// Three of them, all executed:
///
/// * a plan that declared NOTHING runs with nothing supplied — the ordinary prepare is
///   untouched by any of this;
/// * a plan that declared a parameter runs when the parameter is supplied ALONGSIDE
///   other substitutions it never declared, because the declaration is a floor and not a
///   whitelist;
/// * the same plan runs again, with a different term, which is the whole point of
///   preparing once and substituting per call.
#[test]
fn declaring_nothing_still_runs_and_declaring_something_admits_extra_substitutions() {
    let (registry, relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let dataset = empty_dataset();

    // A plan that promises nothing, run with nothing.
    let constant =
        format!("SELECT ?object WHERE {{ ( <https://example.org/d/a> ) <{RELATED}> ( ?object ) }}");
    let undeclared = engine
        .prepare_query_with_options(&constant, None, options)
        .expect("a constant subject is bound without any declaration");
    assert!(
        undeclared.parameters().is_empty(),
        "an ordinary prepare declares nothing"
    );
    assert_eq!(
        rows(
            engine
                .query_prepared(&dataset, &undeclared, &[], options)
                .expect("and it runs with no substitutions at all")
        ),
        1
    );

    // A plan that promises one name, run with that name and a spare.
    let declared = engine
        .prepare_query_with_parameters(&free_subject_query(), None, options, &["subject"])
        .expect("the declared parameter makes the call feasible");
    assert_eq!(
        rows(
            engine
                .query_prepared(
                    &dataset,
                    &declared,
                    &[
                        ("subject".to_owned(), iri("b")),
                        ("unrelated".to_owned(), iri("c")),
                    ],
                    options,
                )
                .expect("a substitution the plan never declared does not make it unrunnable")
        ),
        1
    );

    // And again, with a different term: one prepare, many executions.
    assert_eq!(
        rows(
            engine
                .query_prepared(
                    &dataset,
                    &declared,
                    &[("subject".to_owned(), iri("d"))],
                    options,
                )
                .expect("the same plan answers for another candidate")
        ),
        1
    );
    assert_eq!(
        relation.bound_invocations(),
        3,
        "three executions, three bound invocations, and not one of them free"
    );
}

/// **A plan prepared under a declaration is not the plan prepared without one.**
///
/// The two are cached separately, because they are admitted differently: the declared
/// one ordered a call the undeclared one cannot even admit. A cache key that ignored the
/// declaration would hand the second caller the first caller's plan, and the promise
/// would then be carried by a plan that never made it — or, worse, a plan that made it
/// would be handed to a caller who promises nothing and is not checked.
///
/// Asserted by preparing the same text under two different declarations and reading each
/// plan's own declaration back.
#[test]
fn the_declaration_is_part_of_the_plan_identity() {
    let (registry, _relation) = registry();
    let engine = NativeSparqlEngine::new();
    let env = environment(&registry);
    let options = QueryOptions {
        env: &env,
        ..QueryOptions::EMPTY
    };
    let text = format!(
        "SELECT ?object WHERE {{ ( ?subject ) <{RELATED}> ( ?object ) . ( ?other ) <{RELATED}> ( \
         ?fourth ) }}"
    );

    let one = engine
        .prepare_query_with_parameters(&text, None, options, &["subject", "other"])
        .expect("both subjects declared");
    let two = engine
        .prepare_query_with_parameters(&text, None, options, &["other", "subject"])
        .expect("the same declaration, spelled in the other order");
    assert_eq!(
        one.parameters(),
        two.parameters(),
        "the declaration is normalized, so one order is one plan"
    );
    assert!(
        Arc::ptr_eq(&one, &two),
        "and the normalized declaration reaches the same cache entry"
    );
}
