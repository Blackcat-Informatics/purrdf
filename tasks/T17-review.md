# T17 independent review (Addendum A Task 17, G-05, A-R5)

Scope: uncommitted Task 17 diff on top of 9dece9ec3 (16 files): `ShapesError::Processor(ProcessorFailure)`,
`FailureClass`, `ShapesError::failure_class`, `sparql_call_result` in `shapes/parser/functions.rs`, typed
conversion of the guard/governor/stop/query string errors, host mappings (capi, wasm, python, cli, validate),
and the new tests. No source was modified by this review.

## Commands run (all green)

- `cargo test --locked -p purrdf-shapes --test custom_node_expressions --test governor_inheritance --test dated_execution`: 32 / 5 / 16 passed.
- `cargo test --locked -p purrdf-shapes --lib functions`: 4 passed (includes `only_an_evaluation_failure_becomes_the_calls_sparql_error`).
- `cargo test --locked -p purrdf-validate -p purrdf-capi`: all passed (109, 58, 6, 4, ... 0 failed).
- `cargo test --locked -p purrdf-cli --test shacl_cli --test validate_cli`: passed (29, 62, 62).
- clippy `-D warnings` on shapes, validate, cli, capi, wasm, python (`--all-targets`): clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p purrdf-shapes`: clean.

## Findings

### F1 (BLOCKING) The governor-trip neighbour test does not discriminate

`a_governor_trip_inside_a_function_body_still_fails_the_validation`
(`crates/shapes/tests/governor_inheritance.rs`) asserts `GovernedValidation::BudgetExhausted`.
But `governed_outcome` (`crates/shapes/src/engine.rs` ~3430) is
`match (outcome, state.tripped()) { (_, Some(tripped)) => Ok(BudgetExhausted{..}), ... }`: the trip is read
from the latched `GovernorState`, and wins over the outcome whether the outcome is `Ok(report)` or `Err`.
The doc on `certify_governed`/`run_query_view` says the same ("the governed validation entry recovers the typed
trip from the shared state, which latched it"). So if the Governor failure were wrongly classed Evaluation
and swallowed to `Ok(None)` (unbound), validation would complete with a report, the state would still be
tripped, and this test would still pass. A-R5 ("the fuel trip still fails") is therefore not evidenced by an
integration test. The unit test `only_an_evaluation_failure_becomes_the_calls_sparql_error` does pin the
classification of hand-built `ProcessorFailure` values (it deliberately omits `Governor`), but it does not
show that real runtime failures reach `sparql_call_result` as `Processor`.

Required: a neighbour that fails when a processor failure is swallowed and does not rely on a latched
governor. Two options:
1. Non-governed processor failure through the SPARQL call path with `validate`/`validate_with` and
   `expect_err`: a function whose body is a `sh:select` that calls the function again (the existing
   `a_cycle_through_sparql_query_text_is_bounded_too`, `custom_node_expressions.rs` ~866, is this shape; the
   existing `a_recursive_function_makes_validation_fail_rather_than_conform` covers the non-SPARQL path). Keep
   it, but make the assertion name the typed cause (e.g. `ProcessorFailure::CallDepth` via the error's
   `as_processor()`/message "re-entry depth exceeded"), not only the text "depth bound", so it is tied to
   the Processor class. Verify it fails when `FailureClass::Processor` for `CallDepth` is changed to Evaluation.
2. For the governed case, assert the swallow is impossible by making the trip observable only through the
   result: run with a stop signal / cancel (`StopCause`) that fires inside the body and assert the typed
   cause, or assert on the surrounding query's behaviour (e.g. a constraint after the call that would
   produce a result row if the call were unbound, and assert `BudgetExhausted` AND that evidence shows the
   query aborted, not completed). The simplest rigorous form is option 1 plus a mutation check recorded in
   the task notes.

Whichever is chosen, the task record should say the mutation (move `Governor`/`CallDepth` to Evaluation)
was run and the test failed.

### F2 (MAJOR, fix with F1) Internal-defect strings are classed Evaluation

`Invalid` is also the landing type of every `String` error via `From<String> for ShapesError`. Reachable
during body evaluation and now turned into a silent unbound result:
- `CustomFunction::body()` "internal error: ... has no installed sh:bodyExpression" (expression.rs ~749).
- `LoweredExpr::path/shape/operand/index/scalar_query/body_iri` and `expr_defect`
  "internal validation-plan defect: ..." (plan.rs ~1983-2065), all `Result<_, String>`, all reached from
  `eval_node_expr_at_depth` and `eval_custom_call` via `?`.
- `ShaclExecution::prepare` / query-parse failures route through `query_error`, which returns `Invalid`
  unless the diagnostic code requires propagation; and `run_query_view` still builds
  `format!("query evaluation error: {error}")` for the query-law `prepare_interned_request` failure (not
  through `query_error`), so a propagation-required diagnostic there is also `Invalid`.

The doc on `ShapesError::failure_class` claims "`Invalid` is the evaluator's own channel for an evaluation
failure ... every failure of the processor is `Storage` or `Processor`". That is false for the first two
bullets: they are invariant violations of the processor, which CLAUDE.md requires to be hard failures. Before
this change they failed validation; now `BIND (ex:f() AS ?r)` hides them. Fix: give them a typed
`ProcessorFailure::Internal { what }` (or `PlanDefect`) and convert at the source (`body()`, `expr_defect`
and the `index`/`scalar_query`/`body_iri` messages return `ShapesError`), and route the query-law
`prepare_interned_request` error through `query_error`. Add a unit case per new variant to
`only_an_evaluation_failure_...`.

Mitigated, not a finding: a dated-law admission refusal of a body `sh:select` (`law.admit(...)?` returns a
`String`, so `Invalid`) is also recorded in the query-law runtime; `profile.rs::run` turns a recorded failure
into `Err(failure.into_public())` even when the outcome was `Ok`, so it still fails on profiled runs. Not
reproduced by running a probe (the review made no source changes); reasoned from `profile.rs` ~372-384.

### F3 (MINOR) Stale docs
`eval_custom_function_call` still says "Returns `Err(String)`" (expression.rs ~2570); the doc on
`invoke_native_list_function` says "more than one is an error" - say "the call's SPARQL error". `RecursionGuard`
doc comments for `enter*` were reflowed into a very long line (rustfmt passes; style only).

## Answers to the requested checks

1. Classification: `failure_class` is an exhaustive match with no wildcard (a new variant must choose).
   `Storage`, `Processor` -> Processor; `Imports`, `ShaclJs`, `IllFormed`, `Prebinding`, `UnsupportedTarget`,
   `SparqlTargetDisagreement` -> Refusal; `Invalid` -> Evaluation. The variant mapping is correct; the claim
   that `Invalid` contains only evaluation failures is not (F2). `sparql_call_result` maps only Evaluation to
   `Ok(None)`; Processor and Refusal keep `EvalError::function("{what}: {error}")`.
2. Processor failures: governor trip, stop, incomplete relation, call depth, expression depth, re-entry cycle
   and propagation-required query errors are now typed and reach `sparql_call_result` as Processor (by types,
   `?` conversions in `enter_call`, `enter_node_expr`, `enter`, `certify_governed`, `query_error`). Message
   text is preserved verbatim in `ProcessorFailure::fmt` (compared line by line with the removed `format!`s,
   including "validation budget exhausted: {tripped}; a conformance verdict ...", the stop text, the
   incomplete-relation text, "re-entry depth exceeded ({limit})", "node expression nesting depth exceeded
   ({depth} > {limit})", "recursive node expression detected"). `Query` renders `message` unchanged and
   keeps the `capture(Cause::Query)` side effect. Existing host/CLI/capi/validate tests pass unchanged.
   Integration evidence is weak (F1).
3. Built-in list functions: `invoke_native_list_function` now uses `sparql_call_result` too. The in-repo
   rationale is that a declared built-in is evaluated by the same evaluator as a custom call, and §7.3 ("Evaluation of
   Custom SPARQL Functions": "If the list of output nodes rs has exactly one member, then return that node.
   Otherwise, or if the evaluation produces an evaluation failure, the result of the SPARQL function call is
   an error") is the only text in the repo for a node-expression function called from SPARQL. I could not
   quote a separate sentence for built-ins; applying the same rule is consistent and in scope for the
   `shnex:conformsToShape(?x, ex:Missing)` case, which is tested. It also changes behavior: a built-in with
   more than one output node was a hard error and is now unbound. That follows the same §7.3 "otherwise"
   clause but has no test (add a built-in >1-output neighbour, or state it as not covered).
4. Governor-trip test: does not discriminate (F1). The unit test is evidence of classification only, not of
   propagation, so it is not sufficient alone.
5. `report.rs`/`profile.rs`: `execution()` maps `Processor` to `Execution(failure.to_string())`, the same
   string `Invalid` produced; the `into_parts`/`profile.rs::run` arms add `Processor` beside `Invalid` and
   so preserve the prior projection (recorded-failure replacement, XPath cause promotion). `engine.rs`
   ~1914 (`complete_data` -> `SourceContext`) only sees load-time `Invalid`, so no change is needed.
   `into_shapes` and the validate/cli `other => Invalid(to_string())` fallbacks only wrap non-Shapes errors.
6. Host completeness: capi `error.rs`, wasm `shapes_error_message`, python `shapes_error`, cli
   `shapes_source::shapes_error`, validate `product.rs::message` all handle `Processor` with the same status
   and text as `Invalid` (capi `ParseError`, python `ValueError`, cli `Runtime`). `wasm::shapes_rejection`,
   `validate::complete`, `sparql_conformance::community` use wildcards and are unchanged in behavior.
   New public items `FailureClass`, `ProcessorFailure` are exported from `purrdf_shapes`. No host is missing.

VERDICT: BLOCKED

---

# Recheck of F1-F3 (single scoped pass)

Run: `custom_node_expressions` 35, `governor_inheritance` 5, `dated_execution` 17 passed; full
`cargo test --locked -p purrdf-shapes --lib` passed; `-p purrdf-validate -p purrdf-capi -p purrdf-cli`
passed (60 result blocks, 0 failed); clippy `-D warnings` on shapes/validate/cli/capi/wasm/python clean.

**F1 - resolved.** `a_processor_failure_inside_a_sparql_called_body_fails_validation_with_a_typed_cause`
(custom_node_expressions.rs) uses a self-calling `sh:select` body; the engine's function-depth bound (32)
fails at the innermost level and arrives as `ProcessorFailure::Query` with the function-operational code. It
asserts `expect_err`, `failure_class() == Processor`, the typed `Query` variant, code and message. This does
not depend on a latched governor state: if `Query` were read as Evaluation, each level would return
`Ok(None)` and the chain would complete with a report, so the test fails. `body_level_processor_failures_are_typed`
pins `CallDepth` and `Internal` directly. The governor test now documents that it observes the latch only.
I judged the discrimination by reading the control flow (`query_error` -> `sparql_call_result` at each
level); I did not rerun the mutation myself (no source changes allowed in this review) and rely on the
coordinator's recorded mutation evidence.

**F2 - resolved.** `ProcessorFailure::Internal` and `::Admission` exist and are covered in the unit test;
`CustomFunction::body`, all `plan.rs` helpers (`slot_defect`, `lowering_defect`, `expr_defect`, `pair*`,
`index/scalar_query/body_iri/constant_class_id`) and the conformsToShape "index never filled" case are
`ShapesError::internal`; `law.admit` returns `Admission`; the query-law `prepare_interned_request` error now
goes through `query_error`. `a_dated_admission_refusal_inside_a_function_body_still_fails_the_validation`
(dated_execution.rs) runs both dated profiles and asserts the typed `CompleteValidationError::Admission`
with a `VALUES`-admitted neighbour. Doc on `failure_class` now states the audit scope and defers to the
T3-triage audit. Remaining `Invalid` sources (cardinality, unresolvable shape, node count, non-propagating
query diagnostics) are evaluation failures.

**F3 - resolved.** Docs on `eval_custom_function_call`, `body`, the guard `enter*` methods and
`invoke_native_list_function` are corrected.

**Built-in "cannot yield more than one node" claim - accepted.** Every `sparql:` form
`sparql_ns_lowering` returns is a scalar operator or function call (aggregates and `filter-exists` are
refused), arguments are single constants, and the call lowers to a single-row scalar SELECT; `conformsToShape`
yields one boolean. So the >1 arm is not constructible from a built-in, and the new test
`a_declared_builtin_follows_the_same_value_and_error_rule` covers value and error. Note, not blocking:
`form.render(..)` / arity errors in `invoke_native_list_function` still raise a hard `EvalError::function`
(unchanged behaviour; arguably an evaluation failure but not part of G-05's probe).

No new defects found in the diff.

VERDICT: PASS
