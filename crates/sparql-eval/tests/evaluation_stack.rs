// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The evaluator's stack guard, end to end: a request the parser admits, evaluated on a
//! thread with little stack, answers with the rows its semantics give or is the typed
//! `native-sparql-evaluation-stack-exhausted` diagnostic — never an aborted process — and
//! the same request on a thread with room answers with those rows.
//!
//! Each deep request is prepared on a roomy thread and evaluated on a small one: a
//! prepared plan is exactly what a host that parses once and evaluates on worker threads
//! hands them. The flat `OPTIONAL` spine parses in a few KiB,
//! so it also goes through the whole request path on the small thread. And whole requests
//! nested far past every count this workspace used to enforce — calls, negations, groups,
//! operator levels, path groups, spines, and a property-function plan under them — are
//! swept across every stack from the margin to one that holds them: each answers or is a
//! typed refusal, never an abort.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    EvalError, EvalOptions, GovernedOutcome, InProcessServiceResolver, NativeSparqlEngine,
    PreparedQuery, QueryGovernors, QueryOptions, StopCause, StopSignal,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const EX: &str = "http://example.org/";

/// A small stack for the deep requests below: 256 KiB left.
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

/// Assert `outcome` answers exactly `expected`, or is the stack guard's diagnostic.
fn assert_answer_or_stack_refusal(
    outcome: &Result<Vec<String>, RdfDiagnostic>,
    expected: &[&str],
    what: &str,
) {
    let diagnostic = match outcome {
        Ok(answered) => {
            assert_eq!(answered, expected, "{what} answers what it computes");
            return;
        }
        Err(diagnostic) => diagnostic,
    };
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
fn a_deep_request_on_a_small_stack_answers_or_is_the_typed_refusal_and_answers_on_a_large_one() {
    for (what, query, expected) in deep_forms() {
        let prepared = prepare(&query);
        assert_answer_or_stack_refusal(&evaluate(&prepared, SMALL), expected, what);
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
fn the_whole_request_path_answers_or_refuses_a_flat_spine_on_a_small_stack() {
    // The flat spine parses in a few KiB, so the small thread parses it, admits it and
    // plans it, and then evaluates it or refuses its evaluation, typed.
    assert_answer_or_stack_refusal(
        &request(&optional_spine(126), SMALL),
        &["s1", "s2", "s3", "s4"],
        "126 sibling OPTIONAL",
    );
    assert_eq!(
        request(&optional_spine(126), LARGE).expect("room for it"),
        ["s1", "s2", "s3", "s4"]
    );
}

#[test]
fn the_small_stack_still_answers_a_shallow_request_after_a_deep_one() {
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
    assert_answer_or_stack_refusal(&refused, &["s2", "s3", "s4"], "63 nested FILTER NOT EXISTS");
    // Three negations: the same rows as one.
    assert_eq!(
        shallow.expect("a shallow request answers"),
        ["s2", "s3", "s4"]
    );
}

#[test]
fn an_update_on_a_small_stack_applies_whole_or_is_refused_and_applies_nothing() {
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
    let (small, count) = run(SMALL);
    match small {
        Ok(()) => assert_eq!(count, 4, "an applied update deletes the one `<q>` edge"),
        Err(diagnostic) => {
            assert_eq!(
                diagnostic.code,
                EvalError::STACK_EXHAUSTED_CODE,
                "{diagnostic:?}"
            );
            assert_eq!(count, 5, "a refused update applies nothing");
        }
    }
    // The valid neighbour: with room, the one `<q>` edge is deleted.
    let (applied, after) = run(LARGE);
    applied.expect("the update applies on a large stack");
    assert_eq!(after, 4);
}

#[test]
fn an_in_process_service_on_a_small_stack_answers_or_its_refusal_is_not_silenced() {
    // The forwarded body is itself a deep spine, evaluated by the in-process source under
    // an outer spine, with 320 KiB of stack. It answers `s1` alone, or its evaluation is
    // the typed refusal. `SILENT` tolerates an endpoint that cannot answer — it must not
    // turn "this host's stack cannot evaluate the body" into the join identity, which
    // here would answer every subject as though the service had imposed nothing.
    let body = format!(
        "?s <{EX}p> ?o {}",
        format!("OPTIONAL {{ ?s <{EX}q> ?z }} ").repeat(90)
    );
    let query = format!(
        "SELECT ?s WHERE {{ ?s <{EX}p> ?o {} SERVICE SILENT <{EX}svc> {{ {body} FILTER(?s = <{EX}s1>) }} }}",
        format!("OPTIONAL {{ ?s <{EX}q> ?y }} ").repeat(60)
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
    assert_answer_or_stack_refusal(&run(320 * 1024), &["s1"], "a deep SERVICE SILENT body");
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
            QueryOptions::new().with_remote(Some(&resolver)),
        ))
    })
}

#[test]
fn a_deep_forwarded_body_answers_or_is_the_typed_refusal_under_silent() {
    // The body's 120 nested calls are written again in the text the SERVICE forwards, and
    // the in-process source re-parses and evaluates that text at the depth the SERVICE is
    // evaluated at. With 320 KiB left there, the body answers `s1` alone, or its
    // evaluation is the typed refusal naming the evaluator's construct, and not silenced —
    // `SILENT` answering it with the join identity would pass every subject as though the
    // service had imposed nothing.
    let deep = prepare(&service_with_nested_calls(120));
    let small = evaluate_with_service(&deep, 320 * 1024);
    assert_answer_or_stack_refusal(&small, &["s1"], "a SERVICE SILENT body 120 calls deep");
    if let Err(refused) = small {
        assert!(
            refused.message.contains("expression"),
            "the evaluator's construct is named: {}",
            refused.message
        );
    }
    // The valid neighbours: the same plan with room for the body answers `s1` alone,
    // and so does a shallow body on the very same small stack.
    assert_eq!(
        evaluate_with_service(&deep, LARGE).expect("room for the body"),
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
/// below the poll's frame), and which asks an unscoped walk whether to stop. It records
/// whether that walk latched a refusal and whether the waiting computation was left with
/// a floor other than the one it was given.
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

/// A property path is traversed as a flat program over explicit work lists, with no
/// walk scope open, so a host that suspends the traversal at its polls suspends no
/// scope: the computation that runs while it waits latches nothing whether the host
/// swaps the whole [`purrdf_stack::Context`] or the floor alone, the evaluation it
/// interrupted answers exactly, and the waiting computation keeps the floor it was
/// given. (That a scope a suspension DOES interrupt stays with its own context is
/// `purrdf-stack`'s `a_suspended_scope_stays_with_its_context`.)
#[test]
fn a_suspension_inside_a_path_traversal_leaves_the_evaluation_and_the_waiting_computation_intact() {
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

    for switch in [Switch::Context, Switch::FloorOnly] {
        let host = SuspendingHost::new(switch);
        let (outcome, after) = path_under(Arc::clone(&host));
        let GovernedOutcome::Complete { result, .. } =
            outcome.expect("a suspended evaluation answers")
        else {
            panic!("no governor trips");
        };
        assert_eq!(
            pairs(&result),
            pairs(&baseline),
            "{switch:?}: the suspended evaluation's answer"
        );
        assert!(after, "{switch:?}: the thread answers the next query");
        assert!(
            host.polls.load(Ordering::Relaxed) > 0,
            "{switch:?}: the traversal polled"
        );
        assert_eq!(
            host.waiting_refused.load(Ordering::Relaxed),
            0,
            "{switch:?}: no scope was open for the waiting walk to latch in"
        );
        assert_eq!(
            host.waiting_disturbed.load(Ordering::Relaxed),
            0,
            "{switch:?}: the waiting computation keeps its floor"
        );
    }
}

/// A relation `<rel:tag>` binding `<s1>` to `<t>`, for a request whose property-function
/// call sends its plan through the feasibility pass, which walks the whole pattern.
fn tag_relation() -> purrdf_sparql_eval::ExtensionEnv {
    let mut registry = purrdf_sparql_eval::PropertyFunctionRegistry::new();
    registry.register(
        format!("{EX}rel/tag"),
        Arc::new(
            purrdf_sparql_eval::MemoryRelation::new(
                1,
                1,
                vec![vec![
                    TermValue::iri(format!("{EX}s1")),
                    TermValue::iri(format!("{EX}t")),
                ]],
            )
            .expect("every row is two values wide"),
        ),
    );
    purrdf_sparql_eval::ExtensionEnv::over_relations(registry)
        .expect("the fixture declaration reads cleanly")
}

/// Run a whole request with `bytes` of stack left, with [`tag_relation`] registered.
fn request_with_relation(query: &str, bytes: usize) -> Result<Vec<String>, RdfDiagnostic> {
    let query = query.to_owned();
    on_stack(bytes, move || {
        let env = tag_relation();
        subjects(NativeSparqlEngine::new().query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new().with_env(&env),
        ))
    })
}

/// Whether `result` is the evaluator's typed stack refusal.
fn is_stack_refusal(result: &Result<Vec<String>, RdfDiagnostic>) -> bool {
    result
        .as_ref()
        .is_err_and(|diagnostic| diagnostic.code == EvalError::STACK_EXHAUSTED_CODE)
}

/// Requests nested far past the removed counts (128 recursive levels, 512 levels of
/// tree height, 128 levels of graph pattern, 2 048 spine nodes), each with the only
/// subjects its nesting computes, and whether it calls [`tag_relation`].
fn tall_requests() -> Vec<(&'static str, String, &'static [&'static str], bool)> {
    let o1 = format!("<{EX}o1>");
    let mut chain = format!("?o = {o1}");
    for _ in 0..150 {
        chain = format!("(?o != <{EX}zz> && {chain})");
    }
    vec![
        (
            "300 nested STR calls",
            format!(
                "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}?o{} = \"{EX}o1\") }}",
                "STR(".repeat(300),
                ")".repeat(300)
            ),
            &["s1"],
            false,
        ),
        (
            "2 000 negations",
            format!(
                "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({}(?o = {o1})) }}",
                "!".repeat(2_000)
            ),
            &["s1"],
            false,
        ),
        (
            "400 nested groups",
            format!(
                "SELECT ?s WHERE {{ {}?s <{EX}q> ?z{} }}",
                "{ ".repeat(400),
                " }".repeat(400)
            ),
            &["s1"],
            false,
        ),
        (
            "150 bracket levels of operators",
            format!("SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER({chain}) }}"),
            &["s1"],
            false,
        ),
        (
            "300 nested path groups",
            format!(
                "SELECT ?s WHERE {{ ?s {}<{EX}p>{} {o1} }}",
                "(".repeat(300),
                ")".repeat(300)
            ),
            &["s1"],
            false,
        ),
        ("200 nested LATERAL", lateral(200), &["s1"], false),
        (
            "2 500 sibling OPTIONAL",
            optional_spine(2_500),
            &["s1", "s2", "s3", "s4"],
            false,
        ),
        (
            "a property-function call under 300 nested groups",
            format!(
                "SELECT ?s WHERE {{ ?s <{EX}q> ?z {}?s <{EX}rel/tag> ?t{} }}",
                "{ ".repeat(300),
                " }".repeat(300)
            ),
            &["s1"],
            true,
        ),
    ]
}

/// Wherever in a request the stack runs out — its parse, the walks over its plan before
/// the first operator, the property-function planning pass, its evaluation — the request
/// returns: it answers exactly (a stack with room) or it is a typed stack refusal (one
/// without), never an abort, which would take this test process down. Once it answers on
/// a stack it answers on every larger one, and the largest holds every request.
#[test]
fn every_stack_from_the_margin_up_answers_or_refuses_typed_far_past_the_removed_counts() {
    let floor = purrdf_stack::MARGIN_BYTES;
    for (what, query, expected, relation) in tall_requests() {
        let mut answered_at = None;
        for step in 0..40 {
            let bytes = floor + step * 160 * 1024;
            let outcome = if relation {
                request_with_relation(&query, bytes)
            } else {
                request(&query, bytes)
            };
            match outcome {
                Ok(names) => {
                    assert_eq!(names, expected, "{what} with {bytes} bytes left");
                    answered_at.get_or_insert(bytes);
                }
                refused => {
                    assert!(
                        is_stack_refusal(&refused),
                        "{what} with {bytes} bytes left: {refused:?}"
                    );
                    assert!(
                        answered_at.is_none(),
                        "{what}: refused with {bytes} bytes after answering with less"
                    );
                }
            }
        }
        assert!(
            answered_at.is_some(),
            "{what}: answers within {} KiB",
            (floor + 39 * 160 * 1024) / 1024
        );
    }
}

/// The peak live heap a thousand nested `LATERAL` levels may use, parse and evaluation
/// together. Measured at 6 MB. An evaluation that copied the whole chain below every
/// level, for every left row, passed this at a few dozen levels: its peak grew with the
/// cube of the depth, 80 MB at a hundred levels and 624 MB at two hundred.
const THOUSAND_LATERALS_PEAK_BYTES: i64 = 24 * 1024 * 1024;

/// The heap a thousand nested `LATERAL` levels may request in all. Measured at 33 MB; the
/// copying evaluation requested 346 MB at a hundred levels and 2.6 GB at two hundred.
const THOUSAND_LATERALS_REQUESTED_BYTES: u64 = 128 * 1024 * 1024;

/// A thousand nested `LATERAL` levels, parsed and evaluated whole on a thread with room for
/// them, answer the row the shape has, inside a heap ceiling linear in the depth — and on
/// the small stack the same request answers or is the typed refusal, never an abort.
#[test]
fn a_thousand_nested_laterals_answer_inside_an_allocation_ceiling() {
    let query = lateral(1000);
    let text = query.clone();
    let (answered, measured) = on_stack(LARGE, move || {
        // Sequential, so every allocation the evaluation makes is this thread's own.
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential: true,
            ..EvalOptions::default()
        });
        let data = dataset();
        let window = CurrentThreadWindow::open();
        let answered = subjects(engine.query_with_options_view(
            &*data,
            SparqlRequest {
                query: &text,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        ));
        (answered, window.close())
    });
    assert_eq!(
        answered.expect("a thousand nested LATERAL levels answer on a large stack"),
        ["s1"]
    );
    assert!(
        measured.peak_working_bytes < THOUSAND_LATERALS_PEAK_BYTES,
        "a thousand nested LATERAL levels peaked at {} bytes: {measured:?}",
        measured.peak_working_bytes
    );
    assert!(
        measured.requested_bytes < THOUSAND_LATERALS_REQUESTED_BYTES,
        "a thousand nested LATERAL levels requested {} bytes: {measured:?}",
        measured.requested_bytes
    );
    assert_answer_or_stack_refusal(&request(&query, SMALL), &["s1"], "1000 nested LATERAL");
}
