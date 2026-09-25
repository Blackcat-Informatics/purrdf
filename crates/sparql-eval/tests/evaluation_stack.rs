// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The evaluator's stack guard, end to end: a request the parser admits but the thread
//! evaluating it has too little stack for is the typed
//! `native-sparql-evaluation-stack-exhausted` diagnostic — never an aborted process — and
//! the same request on a thread with room answers with the rows its semantics give.
//!
//! Each deep request is prepared on a roomy thread and evaluated on a small one. The
//! parser bounds its own recursion by level count, and parsing the deepest nested forms
//! takes more native stack than the small thread has, so preparing there would test the
//! parser rather than the evaluator; a prepared plan is exactly what a host that parses
//! once and evaluates on worker threads hands them. The flat `OPTIONAL` spine parses in a
//! few KiB, so it also goes through the whole request path on the small thread.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    EvalError, GovernedOutcome, InProcessServiceResolver, NativeSparqlEngine, PreparedQuery,
    QueryGovernors, QueryOptions, StopCause, StopSignal,
};

const EX: &str = "http://example.org/";

/// Small enough that none of the deep requests below fits: 256 KiB left.
const SMALL: usize = 256 * 1024;

/// Large enough that every one of them does: 64 MiB left.
const LARGE: usize = 64 * 1024 * 1024;

/// `<s1>` … `<s4>` each `<p>` an object, and only `<s1>` also `<q>` one.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for n in 1..=4 {
        let s = builder.intern_iri(&format!("{EX}s{n}"));
        let o = builder.intern_iri(&format!("{EX}o{n}"));
        builder.push_quad(s, p, o, None);
        if n == 1 {
            builder.push_quad(s, q, o, None);
        }
    }
    builder.freeze().expect("the fixture dataset")
}

/// `open` written `n` times around `core`, closed by `close` written `n` times.
fn nested(open: &str, core: &str, close: &str, n: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ {}{core}{} }}",
        open.repeat(n),
        close.repeat(n)
    )
}

/// `n` nested `FILTER NOT EXISTS` groups around `?s <q> ?z`.
fn not_exists(n: usize) -> String {
    nested(
        &format!("?s <{EX}p> ?o FILTER NOT EXISTS {{ "),
        &format!("?s <{EX}q> ?z"),
        " }",
        n,
    )
}

/// `n` nested `FILTER EXISTS` groups around `?s <q> ?z`.
fn exists(n: usize) -> String {
    nested(
        &format!("?s <{EX}p> ?o FILTER EXISTS {{ "),
        &format!("?s <{EX}q> ?z"),
        " }",
        n,
    )
}

/// `n` nested `LATERAL` groups around `?s <q> ?z`.
fn lateral(n: usize) -> String {
    nested(
        &format!("?s <{EX}p> ?o LATERAL {{ "),
        &format!("?s <{EX}q> ?z"),
        " }",
        n,
    )
}

/// `n` sibling `OPTIONAL` groups after `?s <p> ?o` — no nesting at all, but an algebra
/// spine `n` levels tall.
fn optional_spine(n: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o {} }}",
        format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(n)
    )
}

/// The sorted local names `?s` binds, or the diagnostic.
fn subjects(result: Result<SparqlResult, RdfDiagnostic>) -> Result<Vec<String>, RdfDiagnostic> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result?
    else {
        panic!("a SELECT answers with solutions");
    };
    let column = variables
        .iter()
        .position(|name| name == "s")
        .expect("?s is projected");
    let mut names: Vec<String> = rows
        .into_iter()
        .map(|row| match &row[column] {
            Some(TermValue::Iri(iri)) => iri.trim_start_matches(EX).to_owned(),
            other => panic!("?s binds an IRI, got {other:?}"),
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Run `body` with exactly `bytes` of stack left below it (to within one 4 KiB frame).
///
/// The thread is spawned larger and `body` runs beneath frames that eat down to `bytes`,
/// measured with the guard's own [`purrdf_stack::remaining`]: a thread's
/// real stack can be larger than it asked for (the C library reuses a cached stack of up
/// to several times the requested size), and a test that trusted the request would
/// then pass or fail with the order the harness ran it in.
fn on_stack<T: Send + 'static>(bytes: usize, body: impl FnOnce() -> T + Send + 'static) -> T {
    fn descend<T>(bytes: usize, body: impl FnOnce() -> T) -> T {
        if purrdf_stack::remaining() <= bytes {
            return body();
        }
        let frame = core::hint::black_box([0u8; 4096]);
        let value = descend(bytes, body);
        core::hint::black_box(&frame);
        value
    }
    std::thread::Builder::new()
        .stack_size(bytes + 1024 * 1024)
        .spawn(move || descend(bytes, body))
        .expect("spawn")
        .join()
        .expect("the evaluating thread returned rather than aborting")
}

/// Prepare `query` on a roomy thread.
fn prepare(query: &str) -> Arc<PreparedQuery> {
    let query = query.to_owned();
    on_stack(LARGE, move || {
        NativeSparqlEngine::new()
            .prepare_query(&query, None)
            .expect("the parser admits the request")
    })
}

/// Evaluate a prepared plan on a thread with `bytes` of stack.
fn evaluate(prepared: &Arc<PreparedQuery>, bytes: usize) -> Result<Vec<String>, RdfDiagnostic> {
    let prepared = Arc::clone(prepared);
    on_stack(bytes, move || {
        subjects(NativeSparqlEngine::new().query_prepared(
            &dataset(),
            &prepared,
            &[],
            QueryOptions::EMPTY,
        ))
    })
}

/// Run a whole request — parse and evaluation — on a thread with `bytes` of stack.
fn request(query: &str, bytes: usize) -> Result<Vec<String>, RdfDiagnostic> {
    let query = query.to_owned();
    on_stack(bytes, move || {
        subjects(NativeSparqlEngine::new().query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        ))
    })
}

/// Assert `refused` is the stack guard's diagnostic.
fn assert_stack_refusal(refused: &Result<Vec<String>, RdfDiagnostic>, what: &str) {
    let diagnostic = refused
        .as_ref()
        .expect_err(&format!("{what} does not fit the small stack"));
    assert_eq!(
        diagnostic.code,
        EvalError::STACK_EXHAUSTED_CODE,
        "{what}: {diagnostic:?}"
    );
    assert!(
        diagnostic.message.contains("evaluation stack exhausted"),
        "{what}: {diagnostic:?}"
    );
}

/// Every deep form, and the rows it answers with on a stack with room for it.
fn deep_forms() -> [(&'static str, String, &'static [&'static str]); 4] {
    [
        // An odd number of negations of "`?s` has a `<q>`": everything but `s1`.
        (
            "63 nested FILTER NOT EXISTS",
            not_exists(63),
            &["s2", "s3", "s4"],
        ),
        // Only `s1` has a `<q>`, at every level.
        ("63 nested FILTER EXISTS", exists(63), &["s1"]),
        ("126 nested LATERAL", lateral(126), &["s1"]),
        // Every subject, once: each `OPTIONAL` keeps its row.
        (
            "126 sibling OPTIONAL",
            optional_spine(126),
            &["s1", "s2", "s3", "s4"],
        ),
    ]
}

#[test]
fn a_deep_request_on_a_small_stack_is_the_typed_refusal_and_answers_on_a_large_one() {
    for (what, query, expected) in deep_forms() {
        let prepared = prepare(&query);
        assert_stack_refusal(&evaluate(&prepared, SMALL), what);
        // The valid neighbour: the very same plan, on a thread with room for it.
        assert_eq!(
            evaluate(&prepared, LARGE).expect(what),
            expected,
            "{what} answers on a large stack"
        );
    }
}

/// The large-stack answers above are the right ones, not merely some rows: each deep form
/// answers exactly as its one-level twin, whose answer the depth cannot change (an odd
/// number of negations of the same test is that test negated once).
#[test]
fn the_deep_answers_equal_their_shallow_twins() {
    for (deep, shallow) in [
        (not_exists(63), not_exists(1)),
        (exists(63), exists(1)),
        (lateral(126), lateral(1)),
        (optional_spine(126), optional_spine(1)),
    ] {
        assert_eq!(
            evaluate(&prepare(&deep), LARGE).expect("deep"),
            request(&shallow, SMALL).expect("the shallow twin answers on the small stack"),
        );
    }
}

#[test]
fn the_whole_request_path_refuses_a_flat_spine_on_a_small_stack() {
    // The flat spine parses in a few KiB, so the small thread parses it, admits it and
    // plans it, and only its evaluation does not fit.
    assert_stack_refusal(
        &request(&optional_spine(126), SMALL),
        "126 sibling OPTIONAL",
    );
    assert_eq!(
        request(&optional_spine(126), LARGE).expect("room for it"),
        ["s1", "s2", "s3", "s4"]
    );
}

#[test]
fn the_small_stack_still_answers_a_shallow_request_after_refusing_a_deep_one() {
    let prepared = prepare(&not_exists(63));
    let (refused, shallow) = on_stack(SMALL, move || {
        let engine = NativeSparqlEngine::new();
        let refused =
            subjects(engine.query_prepared(&dataset(), &prepared, &[], QueryOptions::EMPTY));
        let shallow = subjects(engine.query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: &not_exists(3),
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        ));
        (refused, shallow)
    });
    assert_stack_refusal(&refused, "63 nested FILTER NOT EXISTS");
    // Three negations: the same rows as one.
    assert_eq!(
        shallow.expect("a shallow request answers"),
        ["s2", "s3", "s4"]
    );
}

#[test]
fn an_update_whose_where_does_not_fit_is_refused_and_applies_nothing() {
    // DELETE every `<q>` edge, guarded by a WHERE that is a 120-deep OPTIONAL spine.
    let update = format!(
        "DELETE {{ ?s <{EX}q> ?z }} WHERE {{ ?s <{EX}p> ?o {} }}",
        format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(120)
    );
    let run = |bytes: usize| {
        let update = update.clone();
        on_stack(bytes, move || {
            let mut data = dataset();
            let outcome = NativeSparqlEngine::new().update_with_options(
                &mut data,
                SparqlRequest {
                    query: &update,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
            );
            (outcome, data.quad_count())
        })
    };
    let (refused, untouched) = run(SMALL);
    let diagnostic = refused.expect_err("the WHERE does not fit the small stack");
    assert_eq!(
        diagnostic.code,
        EvalError::STACK_EXHAUSTED_CODE,
        "{diagnostic:?}"
    );
    assert_eq!(untouched, 5, "a refused update applies nothing");
    // The valid neighbour: with room, the one `<q>` edge is deleted.
    let (applied, after) = run(LARGE);
    applied.expect("the update applies on a large stack");
    assert_eq!(after, 4);
}

#[test]
fn a_stack_refusal_inside_an_in_process_service_is_not_silenced() {
    // The forwarded body is itself a deep spine, evaluated by the in-process source; the
    // outer spine before it leaves the body too little of 320 KiB of stack. `SILENT`
    // tolerates an endpoint that cannot answer — it must not turn "this host's stack
    // cannot evaluate the body" into the join identity, which here would answer every
    // subject as though the service had imposed nothing.
    let body = format!(
        "?s <{EX}p> ?o {}",
        format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(60)
    );
    let query = format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o {} SERVICE SILENT <{EX}svc> {{ {body} FILTER(?s = <{EX}s1>) }} }}",
        format!("OPTIONAL {{ ?s <{EX}q> ?y }} ").repeat(40)
    );
    let run = |bytes: usize| {
        let query = query.clone();
        on_stack(bytes, move || {
            let resolver =
                InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), dataset());
            subjects(NativeSparqlEngine::new().query_with_source(
                &dataset(),
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &resolver,
                QueryOptions::EMPTY,
            ))
        })
    };
    assert_stack_refusal(&run(320 * 1024), "a deep SERVICE SILENT body");
    // The valid neighbour: with room, the service answers `s1` only, and the join keeps it.
    assert_eq!(run(LARGE).expect("room for it"), ["s1"]);
}

/// A `SERVICE SILENT` whose body keeps `?s` only when `depth` nested `STR` calls over
/// its object equal `<o1>`'s string: only `s1` passes, at any depth.
fn service_with_nested_calls(depth: usize) -> String {
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o SERVICE SILENT <{EX}svc> {{ ?s <{EX}p> ?w \
         FILTER({} = \"{EX}o1\") }} }}",
        nested("STR(", "?w", ")", depth)
            .trim_start_matches("SELECT ?s WHERE { ")
            .trim_end_matches(" }")
    )
}

/// Evaluate a prepared plan on a thread with `bytes` of stack, resolving its `SERVICE`
/// in process against [`dataset`].
fn evaluate_with_service(
    prepared: &Arc<PreparedQuery>,
    bytes: usize,
) -> Result<Vec<String>, RdfDiagnostic> {
    let prepared = Arc::clone(prepared);
    on_stack(bytes, move || {
        let resolver = InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), dataset());
        subjects(NativeSparqlEngine::new().query_prepared(
            &dataset(),
            &prepared,
            &[],
            QueryOptions {
                remote: Some(&resolver),
                ..QueryOptions::EMPTY
            },
        ))
    })
}

#[test]
fn a_forwarded_body_too_deep_to_re_parse_is_the_typed_refusal_under_silent() {
    // The body's 120 nested calls are written again in the text the SERVICE forwards, and
    // the in-process source re-parses that text at the depth the SERVICE is evaluated at.
    // With 320 KiB left there, the re-parse runs out of stack: the typed refusal, naming
    // the parser's construct, and not silenced — `SILENT` answering it with the join
    // identity would pass every subject as though the service had imposed nothing.
    let deep = prepare(&service_with_nested_calls(120));
    let refused = evaluate_with_service(&deep, 320 * 1024);
    assert_stack_refusal(&refused, "a SERVICE SILENT body 120 calls deep");
    let message = refused.expect_err("refused").message;
    assert!(
        message.contains("function argument list"),
        "the parser's construct is named: {message}"
    );
    // The valid neighbours: the same plan with room for the re-parse answers `s1` alone,
    // and so does a shallow body on the very same small stack.
    assert_eq!(
        evaluate_with_service(&deep, LARGE).expect("room for the re-parse"),
        ["s1"]
    );
    assert_eq!(
        evaluate_with_service(&prepare(&service_with_nested_calls(3)), 320 * 1024)
            .expect("a shallow body answers on the small stack"),
        ["s1"]
    );
}

/// How a [`SuspendingHost`] switches away from the evaluation it suspends.
#[derive(Debug, Clone, Copy)]
enum Switch {
    /// The whole [`purrdf_stack::Context`]: the floor and the walk-scope state.
    Context,
    /// The floor alone, as a host that knew nothing of walk scopes would.
    FloorOnly,
}

/// A stop signal that stands in for a host suspending the evaluation at every poll — as
/// the wasm package's asynchronous lane does under `yieldEveryPolls: 0` — and running
/// another computation while it waits: one whose stack is nearly gone (a floor 1 KiB
/// below the poll's frame), and which asks an unscoped walk whether to stop. A property
/// path's traversal polls inside its walk scope, so some of these suspensions happen
/// with that scope open.
#[derive(Debug)]
struct SuspendingHost {
    switch: Switch,
    polls: AtomicUsize,
    /// Suspensions whose waiting walk was told to stop — a refusal latched somewhere.
    waiting_refused: AtomicUsize,
    /// Suspensions that left the waiting computation's own floor changed.
    waiting_disturbed: AtomicUsize,
    /// The first changed floor the waiting computation was left with.
    disturbed_floor: Mutex<Option<usize>>,
}

impl SuspendingHost {
    fn new(switch: Switch) -> Arc<Self> {
        Arc::new(Self {
            switch,
            polls: AtomicUsize::new(0),
            waiting_refused: AtomicUsize::new(0),
            waiting_disturbed: AtomicUsize::new(0),
            disturbed_floor: Mutex::new(None),
        })
    }
}

impl StopSignal for SuspendingHost {
    fn poll(&self) -> Option<StopCause> {
        self.polls.fetch_add(1, Ordering::Relaxed);
        let waiting_floor = purrdf_stack::stack_pointer() - 1024;
        let (refused, left_with) = match self.switch {
            Switch::Context => {
                let job =
                    purrdf_stack::replace_context(purrdf_stack::Context::on_floor(waiting_floor));
                let refused = purrdf_stack::walk_is_low("the waiting walk");
                let waiting = purrdf_stack::replace_context(job);
                let left_with = if waiting == purrdf_stack::Context::on_floor(waiting_floor) {
                    waiting_floor
                } else {
                    // Whatever the waiting walk left behind, it is not what it was given.
                    usize::MAX
                };
                (refused, left_with)
            }
            Switch::FloorOnly => {
                let job = purrdf_stack::replace_floor(waiting_floor);
                let refused = purrdf_stack::walk_is_low("the waiting walk");
                (refused, purrdf_stack::replace_floor(job))
            }
        };
        if refused {
            self.waiting_refused.fetch_add(1, Ordering::Relaxed);
        }
        if left_with != waiting_floor {
            self.waiting_disturbed.fetch_add(1, Ordering::Relaxed);
            self.disturbed_floor
                .lock()
                .expect("the observation lock")
                .get_or_insert(left_with);
        }
        None
    }
}

/// Every `(?s, ?o)` pair one or more `<p>` steps apart, as `s|o` local names, sorted.
const PATH_PAIRS: &str = "SELECT ?s ?o WHERE { ?s <http://example.org/p>+ ?o }";

/// The sorted `s|o` local-name pairs of a `PATH_PAIRS` answer.
fn pairs(result: &SparqlResult) -> Vec<String> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions");
    };
    let name = |cell: &Option<TermValue>| match cell {
        Some(TermValue::Iri(iri)) => iri.trim_start_matches(EX).to_owned(),
        other => panic!("an IRI, got {other:?}"),
    };
    let mut pairs: Vec<String> = rows
        .iter()
        .map(|row| format!("{}|{}", name(&row[0]), name(&row[1])))
        .collect();
    pairs.sort();
    pairs
}

/// Run `PATH_PAIRS` under `host`, then an ordinary query on the same thread, on a roomy
/// thread: the governed outcome, and whether the query after it answered.
fn path_under(host: Arc<SuspendingHost>) -> (Result<GovernedOutcome, RdfDiagnostic>, bool) {
    on_stack(LARGE, move || {
        let engine = NativeSparqlEngine::new();
        let request = SparqlRequest {
            query: PATH_PAIRS,
            base_iri: None,
            substitutions: &[],
        };
        let outcome = engine.query_governed(
            &dataset(),
            request,
            QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_stop_signal(host),
        );
        let after = engine
            .query_with_options_view(&*dataset(), request, QueryOptions::EMPTY)
            .is_ok();
        (outcome, after)
    })
}

/// A suspension inside a property path's walk scope leaves the scope with the suspended
/// evaluation: the computation that runs while it waits has no scope of its own open, so
/// its low walk carries on and latches nothing, the evaluation it interrupted answers
/// exactly, and every context keeps the floor it had. The control switches the floor
/// alone: the waiting walk then latches its refusal in the suspended scope, the waiting
/// computation is left with the exhausted floor, and the evaluation — which never ran low
/// — is refused for the waiting walk, and leaves the thread refusing the next query too.
#[test]
fn a_suspension_inside_a_path_s_walk_scope_leaves_the_scope_with_the_evaluation() {
    let engine = NativeSparqlEngine::new();
    let baseline = engine
        .query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: PATH_PAIRS,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .expect("the ungoverned path answers");
    assert_eq!(pairs(&baseline), ["s1|o1", "s2|o2", "s3|o3", "s4|o4"]);

    let host = SuspendingHost::new(Switch::Context);
    let (outcome, after) = path_under(Arc::clone(&host));
    let GovernedOutcome::Complete { result, .. } = outcome.expect("a suspended evaluation answers")
    else {
        panic!("no governor trips");
    };
    assert_eq!(
        pairs(&result),
        pairs(&baseline),
        "the suspended evaluation's answer"
    );
    assert!(after, "the thread answers the next query");
    assert!(
        host.polls.load(Ordering::Relaxed) > 0,
        "the traversal polled"
    );
    assert_eq!(host.waiting_refused.load(Ordering::Relaxed), 0);
    assert_eq!(host.waiting_disturbed.load(Ordering::Relaxed), 0);

    let control = SuspendingHost::new(Switch::FloorOnly);
    let (outcome, after) = path_under(Arc::clone(&control));
    assert!(
        control.waiting_refused.load(Ordering::Relaxed) > 0,
        "a suspension happened inside the path's walk scope"
    );
    assert_eq!(
        *control
            .disturbed_floor
            .lock()
            .expect("the observation lock"),
        Some(purrdf_stack::EXHAUSTED),
        "the waiting computation was left exhausted"
    );
    let refused = outcome.expect_err("the evaluation is refused for the waiting walk");
    assert_eq!(refused.code, EvalError::STACK_EXHAUSTED_CODE, "{refused:?}");
    assert!(refused.message.contains("the waiting walk"), "{refused:?}");
    assert!(!after, "and the thread refuses the next query");
}
