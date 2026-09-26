// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Triple terms nested past anything a dataset holds are valid SPARQL 1.2, answered
//! end to end through [`NativeSparqlEngine`] rather than refused at parse:
//!
//! * a pattern whose triple term nests deeper than any the dataset holds matches
//!   nothing — `SELECT` answers no rows, `ASK` false, `COUNT` zero — while the same
//!   shape nested as deep as the stored term matches it;
//! * a deep triple term written as a value (`VALUES`, `BIND`) is an answer like any
//!   other, returned whole;
//! * writing one into a dataset (`INSERT DATA`, an `INSERT` template, a `CONSTRUCT`
//!   graph) is refused with the dataset's own limit, `rdf-ir-triple-nesting-limit`,
//!   past 16 levels, and admitted at 16;
//! * how deep they may nest is the stack of the thread answering, and past it the
//!   answer is the typed stack refusal, never an abort — however deep the evaluation
//!   stands when it walks the term.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{
    DatasetView, GraphMatch, QuadIds, QuadProbePlan, QuadRef, RdfDataset, RdfDatasetBuilder,
    RdfDiagnostic, RdfStoreCapabilities, SparqlEngine, SparqlRequest, SparqlResult, TermId,
    TermRef, TermValue,
};
use purrdf_sparql_eval::{
    Arity, BoundFunctionRegistry, EvalError, ExtensionEnv, NativeSparqlEngine, QueryOptions,
    UserFunctionRegistry, Volatility,
};

const EX: &str = "http://example.org/";

/// The code a frozen dataset refuses a triple term nested past 16 levels with.
const DATASET_LIMIT: &str = "rdf-ir-triple-nesting-limit";

/// The deepest triple term a frozen dataset holds.
const DATASET_DEPTH: usize = 16;

/// `<<( <s> <p> … core … )>>`, `levels` triple terms deep, `core` the innermost object.
fn chain(levels: usize, core: &str) -> String {
    format!(
        "{}{core}{}",
        format!("<<( <{EX}s> <{EX}p> ").repeat(levels),
        " )>>".repeat(levels)
    )
}

/// The value of [`chain`] with an IRI or literal `core`.
fn chain_value(levels: usize, core: TermValue) -> TermValue {
    let mut term = core;
    for _ in 0..levels {
        term = TermValue::Triple {
            s: Box::new(TermValue::iri(format!("{EX}s"))),
            p: Box::new(TermValue::iri(format!("{EX}p"))),
            o: Box::new(term),
        };
    }
    term
}

/// How many triple terms `value`'s object chain holds, counted without recursion.
fn nesting(value: &TermValue) -> usize {
    let mut levels = 0;
    let mut term = value;
    while let TermValue::Triple { o, .. } = term {
        levels += 1;
        term = o;
    }
    levels
}

/// Drop a deep value level by level, so a test thread's stack never holds its drop.
fn drop_flat(value: TermValue) {
    let mut term = value;
    while let TermValue::Triple { o, .. } = term {
        term = *o;
    }
}

/// `<a> <q> T` with `T` a [`DATASET_DEPTH`]-deep triple term over `<o>`.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri(&format!("{EX}a"));
    let q = builder.intern_iri(&format!("{EX}q"));
    let s = builder.intern_iri(&format!("{EX}s"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let mut o = builder.intern_iri(&format!("{EX}o"));
    for _ in 0..DATASET_DEPTH {
        o = builder.intern_triple(s, p, o);
    }
    builder.push_quad(a, q, o, None);
    builder
        .freeze()
        .expect("a 16-deep triple term is within the dataset limit")
}

/// A view over [`dataset`] that vouches for a chosen triple-term nesting bound and counts
/// the triple-term values it is asked to resolve: the observing oracle for whether a
/// pattern was answered before it was walked.
struct CountingView {
    inner: Arc<RdfDataset>,
    bound: Option<usize>,
    triple_lookups: AtomicUsize,
}

impl CountingView {
    fn new(bound: Option<usize>) -> Self {
        Self {
            inner: dataset(),
            bound,
            triple_lookups: AtomicUsize::new(0),
        }
    }
}

impl DatasetView for CountingView {
    type Id = TermId;
    type ProbePlan = QuadProbePlan;

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads(&*self.inner)
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        DatasetView::quad_refs(&*self.inner)
    }

    fn resolve(&self, id: TermId) -> TermRef<'_> {
        DatasetView::resolve(&*self.inner, id)
    }

    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads_for_pattern(&*self.inner, s, p, o, g)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        if matches!(value, TermValue::Triple { .. }) {
            self.triple_lookups.fetch_add(1, Ordering::Relaxed);
        }
        DatasetView::term_id_by_value(&*self.inner, value)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        DatasetView::capabilities(&*self.inner)
    }

    fn triple_term_nesting_bound(&self) -> Option<usize> {
        self.bound
    }

    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch,
    ) -> QuadProbePlan {
        DatasetView::probe_plan(&*self.inner, s_bound, p_bound, o_bound, g)
    }

    fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads_for_pattern_with_plan(&*self.inner, plan, s, p, o, g)
    }

    fn term_count(&self) -> usize {
        DatasetView::term_count(&*self.inner)
    }
}

fn query_on<D: DatasetView + Sync>(
    dataset: &D,
    query: &str,
) -> Result<SparqlResult, RdfDiagnostic> {
    NativeSparqlEngine::new().query_with_options_view(
        dataset,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
    )
}

/// The variables and rows of a `SELECT`.
fn solutions(result: SparqlResult) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        other => panic!("a SELECT answers with solutions, got {other:?}"),
    }
}

fn iri(local: &str) -> Option<TermValue> {
    Some(TermValue::iri(format!("{EX}{local}")))
}

/// The same shape at the stored depth and past it: the stored depth matches the stored
/// term (binding its innermost object, so the match is real and not a coincidence), and
/// 200 levels match nothing — with the variables still in scope. Both a view that vouches
/// for the dataset's bound and one that vouches for none answer the same; the one that
/// vouches answers the deep pattern without resolving a single triple-term value, the
/// other resolves it the long way.
#[test]
fn a_pattern_nested_past_the_dataset_matches_nothing_and_its_neighbour_matches() {
    for bound in [Some(DATASET_DEPTH), None] {
        let view = CountingView::new(bound);
        let select = |levels: usize| {
            format!(
                "SELECT ?a ?o WHERE {{ ?a <{EX}q> {} }}",
                chain(levels, "?o")
            )
        };
        let (variables, rows) = solutions(query_on(&view, &select(DATASET_DEPTH)).expect("16"));
        assert_eq!(variables, ["a", "o"]);
        assert_eq!(
            rows,
            [vec![iri("a"), iri("o")]],
            "{bound:?}: the stored depth matches"
        );
        let (variables, rows) = solutions(query_on(&view, &select(200)).expect("200"));
        assert_eq!(
            variables,
            ["a", "o"],
            "{bound:?}: the variables stay in scope"
        );
        assert!(
            rows.is_empty(),
            "{bound:?}: 200 levels match nothing: {rows:?}"
        );

        let ground = |levels: usize| {
            format!(
                "SELECT ?a WHERE {{ ?a <{EX}q> {} }}",
                chain(levels, &format!("<{EX}o>"))
            )
        };
        view.triple_lookups.store(0, Ordering::Relaxed);
        let (_, rows) = solutions(query_on(&view, &ground(DATASET_DEPTH)).expect("16"));
        assert_eq!(
            rows,
            [vec![iri("a")]],
            "{bound:?}: the stored ground term matches"
        );
        assert!(
            view.triple_lookups.load(Ordering::Relaxed) > 0,
            "resolved to match it"
        );
        view.triple_lookups.store(0, Ordering::Relaxed);
        let (_, rows) = solutions(query_on(&view, &ground(200)).expect("200"));
        assert!(
            rows.is_empty(),
            "{bound:?}: a 200-deep ground term matches nothing"
        );
        let lookups = view.triple_lookups.load(Ordering::Relaxed);
        if bound.is_some() {
            assert_eq!(lookups, 0, "answered before the term was resolved");
        } else {
            assert!(lookups > 0, "with no bound it is resolved the long way");
        }
    }
}

/// `ASK` is false and `COUNT` is zero past the stored depth; at the stored depth they are
/// true and one.
#[test]
fn ask_and_count_answer_a_pattern_nested_past_the_dataset() {
    let data = dataset();
    for (levels, asked, counted) in [(DATASET_DEPTH, true, "1"), (200, false, "0")] {
        let ask = format!("ASK {{ ?a <{EX}q> {} }}", chain(levels, "?o"));
        match query_on(&data, &ask).expect("ASK answers") {
            SparqlResult::Boolean(answer) => assert_eq!(answer, asked, "ASK at {levels}"),
            other => panic!("ASK answers a boolean, got {other:?}"),
        }
        let count = format!(
            "SELECT (COUNT(*) AS ?n) WHERE {{ ?a <{EX}q> {} }}",
            chain(levels, "?o")
        );
        let (_, rows) = solutions(query_on(&data, &count).expect("COUNT answers"));
        match rows.as_slice() {
            [row] => match row.as_slice() {
                [Some(TermValue::Literal { lexical_form, .. })] => {
                    assert_eq!(lexical_form, counted, "COUNT at {levels}");
                }
                other => panic!("COUNT binds one literal, got {other:?}"),
            },
            other => panic!("COUNT answers one row, got {other:?}"),
        }
    }
}

/// A 200-deep triple term written in `VALUES` and built by `BIND` is an answer, returned
/// whole: equal to the term written, 200 levels deep, and unchanged through the SPARQL
/// JSON results serialization and back.
#[test]
fn a_deep_triple_term_written_as_a_value_is_returned_whole() {
    let data = dataset();
    let literal = TermValue::Literal {
        lexical_form: "1".to_owned(),
        datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
        language: None,
        direction: None,
    };
    for query in [
        format!("SELECT ?x WHERE {{ VALUES ?x {{ {} }} }}", chain(200, "1")),
        format!("SELECT ?x WHERE {{ BIND({} AS ?x) }}", chain(200, "1")),
    ] {
        let result = query_on(&data, &query).expect("a deep value answers");
        let serialized = purrdf_sparql_results::to_json(
            &result,
            &purrdf_sparql_results::ResultProvenance::default(),
            None,
        )
        .expect("serializes");
        let parsed = purrdf_sparql_results::from_json(&serialized.bytes).expect("parses back");
        let (variables, mut rows) = solutions(result);
        assert_eq!(variables, ["x"]);
        let value = rows
            .pop()
            .and_then(|mut row| row.pop().flatten())
            .expect("one row binding ?x");
        assert_eq!(
            nesting(&value),
            200,
            "{query:.40}: every level is in the answer"
        );
        let written = chain_value(200, literal.clone());
        assert_eq!(value, written, "the answer is the term written");
        assert_eq!(parsed.variables, ["x"]);
        assert_eq!(
            parsed.rows,
            [vec![Some(value.clone())]],
            "the JSON round trip holds it unchanged"
        );
        drop_flat(value);
        drop_flat(written);
        for row in parsed.rows {
            row.into_iter().flatten().for_each(drop_flat);
        }
    }
}

fn update(dataset: &mut Arc<RdfDataset>, text: &str) -> Result<(), RdfDiagnostic> {
    NativeSparqlEngine::new().update(
        dataset,
        SparqlRequest {
            query: text,
            base_iri: None,
            substitutions: &[],
        },
    )
}

/// Writing a triple term into a dataset is held to the dataset's own limit: 16 levels are
/// inserted and then matched by a query, 17 are refused with the dataset's typed error
/// and leave it as it was — through `INSERT DATA` and through an `INSERT` template alike.
#[test]
fn inserting_a_triple_term_is_held_to_the_dataset_limit() {
    for form in ["INSERT DATA", "INSERT WHERE"] {
        let text = |levels: usize| match form {
            "INSERT DATA" => format!(
                "INSERT DATA {{ <{EX}b> <{EX}q> {} }}",
                chain(levels, &format!("<{EX}o>"))
            ),
            _ => format!(
                "INSERT {{ <{EX}b> <{EX}q> {} }} WHERE {{}}",
                chain(levels, &format!("<{EX}o>"))
            ),
        };
        let mut data = RdfDatasetBuilder::new().freeze().expect("empty");
        let refused = update(&mut data, &text(DATASET_DEPTH + 1))
            .expect_err("17 levels exceed the dataset limit");
        assert_eq!(refused.code, DATASET_LIMIT, "{form}: {refused:?}");
        assert_eq!(data.quads().count(), 0, "{form}: the refusal wrote nothing");

        update(&mut data, &text(DATASET_DEPTH)).expect("16 levels are inserted");
        let select = format!(
            "SELECT ?b ?o WHERE {{ ?b <{EX}q> {} }}",
            chain(DATASET_DEPTH, "?o")
        );
        let (_, rows) = solutions(query_on(&data, &select).expect("queryable"));
        assert_eq!(
            rows,
            [vec![iri("b"), iri("o")]],
            "{form}: the insert is matched"
        );
    }
}

/// A `CONSTRUCT` builds a dataset, so its graph is held to the same limit: a template
/// nesting 17 levels is refused with the dataset's typed error, one nesting 16 builds the
/// statement.
#[test]
fn a_constructed_graph_is_held_to_the_dataset_limit() {
    let data = dataset();
    let construct = |levels: usize| {
        format!(
            "CONSTRUCT {{ ?a <{EX}r> {} }} WHERE {{ ?a <{EX}q> ?t }}",
            chain(levels, "?t")
        )
    };
    // The stored term is 16 deep, so a template nesting one level around it is 17 deep.
    let refused = query_on(&data, &construct(1)).expect_err("17 levels are refused");
    assert_eq!(refused.code, DATASET_LIMIT, "{refused:?}");
    let construct_flat = format!(
        "CONSTRUCT {{ ?a <{EX}r> {} }} WHERE {{ ?a <{EX}q> ?t }}",
        chain(DATASET_DEPTH, &format!("<{EX}o>"))
    );
    match query_on(&data, &construct_flat).expect("16 levels are built") {
        SparqlResult::Graph(graph) => assert_eq!(graph.quads().count(), 1),
        other => panic!("CONSTRUCT answers a graph, got {other:?}"),
    }
}

/// Whether `diagnostic` is a stack refusal: the parser's or the evaluator's.
fn is_stack_refusal(diagnostic: &RdfDiagnostic) -> bool {
    diagnostic.code == EvalError::STACK_EXHAUSTED_CODE
        || (diagnostic.code == "native-sparql-query-parse"
            && diagnostic.message.contains("SPARQL parse stack exhausted"))
}

/// A hundred thousand nested triple terms on a test thread are the typed stack refusal,
/// in a pattern and in `VALUES` alike — never an abort — and the next request answers.
#[test]
fn triple_terms_nested_past_the_stack_are_the_typed_refusal() {
    let data = dataset();
    for query in [
        format!("SELECT * WHERE {{ ?a <{EX}q> {} }}", chain(100_000, "?o")),
        format!(
            "SELECT * WHERE {{ VALUES ?x {{ {} }} }}",
            chain(100_000, "1")
        ),
    ] {
        let refused = query_on(&data, &query).expect_err("past the stack");
        assert!(is_stack_refusal(&refused), "{refused:?}");
    }
    let (_, rows) = solutions(
        query_on(&data, &format!("SELECT ?a WHERE {{ ?a <{EX}q> ?t }}"))
            .expect("the next request answers"),
    );
    assert_eq!(rows, [vec![iri("a")]]);
}

/// Run `body` with about `bytes` of stack left below it (to within one 4 KiB frame),
/// measured with the guard's own [`purrdf_stack::remaining`] rather than trusted from a
/// thread's requested size.
fn with_stack_left<T>(bytes: usize, body: impl FnOnce() -> T) -> T {
    if purrdf_stack::remaining() <= bytes {
        return body();
    }
    let frame = core::hint::black_box([0u8; 4096]);
    let value = with_stack_left(bytes, body);
    core::hint::black_box(&frame);
    value
}

/// `nesting` nested `FILTER EXISTS` around a group that writes the deep term `term` in
/// `VALUES` again and compares it with the outer `VALUES` binding. An `EXISTS` level
/// costs its evaluation several times the stack its height charge takes, so evaluation
/// reaches the end of the stack well before the term does, and the innermost group then
/// converts, copies and compares the whole term there. It answers the one row, `?x`
/// bound to the whole term, only if every level held.
fn exists_around_a_deep_term(nesting: usize, term: &str) -> String {
    format!(
        "SELECT ?x WHERE {{ VALUES ?x {{ {term} }} {}VALUES ?y {{ {term} }} FILTER(sameTerm(?y, ?x)){} }}",
        "FILTER EXISTS { ".repeat(nesting),
        " }".repeat(nesting)
    )
}

/// A deep triple term walked where the evaluation stands deepest: a plan prepared with
/// room to spare, evaluated with less and less stack left until the evaluation of its
/// `EXISTS` nesting is refused. With the least stack that still answers, the innermost
/// level passed its check with barely the margin left and then walked a term thousands of
/// levels deep, far more than the margin holds: it answers whole because the stack those
/// walks take was reserved when the evaluation started, so no check can leave less. A
/// stack one step smaller is the typed refusal. Without the reserve the walk at that edge
/// runs past the margin, and this process aborts rather than failing an assertion.
#[test]
fn a_deep_term_walked_at_the_deepest_evaluation_fits_the_stack_left() {
    const LEVELS: usize = 3_000;
    let answers = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let data = dataset();
            let engine = NativeSparqlEngine::new();
            let prepared = engine
                .prepare_query(&exists_around_a_deep_term(200, &chain(LEVELS, "1")), None)
                .expect("parses with room to spare");
            let answer = |bytes: usize| {
                with_stack_left(bytes, || {
                    engine
                        .query_prepared(&data, &prepared, &[], QueryOptions::EMPTY)
                        .map(|result| {
                            let (_, mut rows) = solutions(result);
                            let value = rows.pop().and_then(|mut row| row.pop().flatten());
                            let depth = value.as_ref().map_or(0, nesting);
                            if let Some(value) = value {
                                drop_flat(value);
                            }
                            (rows.len(), depth)
                        })
                })
            };
            let (mut refused, mut answered) = (purrdf_stack::MARGIN_BYTES, 64 << 20);
            assert_eq!(answer(answered).expect("answers with room"), (0, LEVELS));
            let refusal = answer(refused).expect_err("the margin alone holds nothing");
            assert!(is_stack_refusal(&refusal), "{refusal:?}");
            while answered - refused > 4096 {
                let mid = refused.midpoint(answered);
                match answer(mid) {
                    Ok(found) => {
                        assert_eq!(found, (0, LEVELS), "{mid} bytes: the term is whole");
                        answered = mid;
                    }
                    Err(refusal) => {
                        assert!(is_stack_refusal(&refusal), "{mid} bytes: {refusal:?}");
                        refused = mid;
                    }
                }
            }
            (answered, refused)
        })
        .expect("spawn")
        .join()
        .expect("the thread returned rather than aborting");
    eprintln!(
        "a {LEVELS}-deep term under 200 nested EXISTS answers with {} bytes left, refused with {}",
        answers.0, answers.1
    );
}

// ── Triple terms built at run time ───────────────────────────────────────────────────

/// The IRI of the native function [`deep_functions`] registers: `<deep>(n)` returns the
/// `n`-level chain over `<o>`, built by a loop as a host would build it.
fn deep_fn() -> String {
    format!("{EX}deep")
}

/// A bound registry holding [`deep_fn`].
fn deep_functions() -> BoundFunctionRegistry {
    let mut functions = UserFunctionRegistry::new();
    functions.register_native(
        deep_fn(),
        Arity::Exact(1),
        Volatility::Stable,
        Arc::new(|args: &[&TermValue]| {
            let levels = match args[0] {
                TermValue::Literal { lexical_form, .. } => lexical_form.parse().unwrap_or(0),
                _ => 0,
            };
            Ok(Some(chain_value(levels, TermValue::iri(format!("{EX}o")))))
        }),
    );
    NativeSparqlEngine::new()
        .bind_functions(functions, ExtensionEnv::empty())
        .expect("a native-only registry binds")
}

/// Run the test named `name` in a child process of this test binary, and assert that the
/// child exited normally. A walk off the end of a thread's stack aborts the whole process
/// — it is no panic a test can catch — so the tests that probe the edge of the stack run
/// where an abort is reported as this test's failure (naming the signal) rather than
/// ending the run. Returns `true` in the child, which then runs the body.
fn in_child_process(name: &str) -> bool {
    const CHILD: &str = "PURRDF_DEEP_TRIPLE_TERMS_CHILD";
    if std::env::var(CHILD).is_ok_and(|child| child == name) {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([name, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, name)
        .output()
        .expect("the child runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{name} ended with {:?} (an abort is a walk that ran off the stack):\n{stderr}",
        output.status
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{name} did not run in the child"
    );
    eprint!("{stderr}");
    false
}

/// `nesting` nested `FILTER EXISTS` around a group that BUILDS a triple term at run time
/// — `TRIPLE(<s>, <p>, <deep>(levels))`, one level more than the host function returns —
/// and compares it with the same term built at the top. No triple term is written in the
/// request, so nothing is reserved for it when the evaluation starts; the innermost level
/// builds and walks the whole term where the evaluation stands deepest. It answers the
/// one row, `?x` bound to the whole term, only if every level held.
fn exists_around_a_built_term(nesting: usize, levels: usize) -> String {
    let built = format!("TRIPLE(<{EX}s>, <{EX}p>, <{}>({levels}))", deep_fn());
    format!(
        "SELECT ?x WHERE {{ BIND({built} AS ?x) {}BIND({built} AS ?y) FILTER(sameTerm(?y, ?x)){} }}",
        "FILTER EXISTS { ".repeat(nesting),
        " }".repeat(nesting)
    )
}

/// A triple term built at run time, far deeper than the margin holds, walked where the
/// evaluation stands deepest: evaluated with less and less stack left, every run answers
/// the whole term or is the typed stack refusal — never an abort. The term is admitted
/// when the top-level `BIND` builds it: the evaluation keeps stack for walks over it from
/// then on, so the innermost level — which passed its check with barely the margin left —
/// still has room to build, hash and compare it. Without that admission the walk at the
/// edge runs off the stack, and the child process aborts.
#[test]
fn a_term_built_at_run_time_answers_whole_or_is_refused_never_aborts() {
    const LEVELS: usize = 5_000;
    if !in_child_process("a_term_built_at_run_time_answers_whole_or_is_refused_never_aborts") {
        return;
    }
    let edge = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let data = dataset();
            let engine = NativeSparqlEngine::new();
            let functions = deep_functions();
            let prepared = engine
                .prepare_query(&exists_around_a_built_term(200, LEVELS), None)
                .expect("parses with room to spare");
            let answer = |bytes: usize| {
                with_stack_left(bytes, || {
                    engine
                        .query_prepared(
                            &data,
                            &prepared,
                            &[],
                            QueryOptions::new().with_functions(&functions),
                        )
                        .map(|result| {
                            let (_, mut rows) = solutions(result);
                            let value = rows.pop().and_then(|mut row| row.pop().flatten());
                            let depth = value.as_ref().map_or(0, nesting);
                            if let Some(value) = value {
                                drop_flat(value);
                            }
                            (rows.len(), depth)
                        })
                })
            };
            let (mut refused, mut answered) = (purrdf_stack::MARGIN_BYTES, 64 << 20);
            assert_eq!(
                answer(answered).expect("answers with room"),
                (0, LEVELS + 1),
                "one row, the whole term"
            );
            let refusal = answer(refused).expect_err("the margin alone holds nothing");
            assert!(is_stack_refusal(&refusal), "{refusal:?}");
            while answered - refused > 4096 {
                let mid = refused.midpoint(answered);
                match answer(mid) {
                    Ok(found) => {
                        assert_eq!(found, (0, LEVELS + 1), "{mid} bytes: the term is whole");
                        answered = mid;
                    }
                    Err(refusal) => {
                        assert!(is_stack_refusal(&refusal), "{mid} bytes: {refusal:?}");
                        refused = mid;
                    }
                }
            }
            (answered, refused)
        })
        .expect("spawn")
        .join()
        .expect("the thread returned");
    eprintln!(
        "a {}-deep term built under 200 nested EXISTS answers with {} bytes left, refused with {}",
        LEVELS + 1,
        edge.0,
        edge.1
    );
}

/// Past the stack of the thread answering, a term built at run time is the typed stack
/// refusal naming triple terms — on a small thread a host function's 100 000-level chain
/// is — and the valid neighbour, the same call at 100 levels, answers the whole term.
#[test]
fn a_term_built_past_the_stack_is_the_typed_refusal() {
    if !in_child_process("a_term_built_past_the_stack_is_the_typed_refusal") {
        return;
    }
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            let data = dataset();
            let functions = deep_functions();
            let run = |levels: usize| {
                NativeSparqlEngine::new().query_with_options_view(
                    &*data,
                    SparqlRequest {
                        query: &format!(
                            "SELECT ?x WHERE {{ BIND(TRIPLE(<{EX}s>, <{EX}p>, <{}>({levels})) AS ?x) }}",
                            deep_fn()
                        ),
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::new().with_functions(&functions),
                )
            };
            let refused = run(100_000).expect_err("past the stack");
            assert_eq!(refused.code, EvalError::STACK_EXHAUSTED_CODE, "{refused:?}");
            assert!(refused.message.contains("triple term"), "{refused:?}");
            let (_, mut rows) = solutions(run(100).expect("the neighbour answers"));
            let value = rows
                .pop()
                .and_then(|mut row| row.pop().flatten())
                .expect("one row binding ?x");
            assert_eq!(
                value,
                chain_value(101, TermValue::iri(format!("{EX}o"))),
                "the whole term"
            );
        })
        .expect("spawn")
        .join()
        .expect("the thread returned");
}

/// `TRIPLE` calls feeding each other through a chain of `LATERAL` levels build a term one
/// level deeper per level, each one where the evaluation stands one level deeper: a
/// moderate chain of 20 answers exactly the term it builds, and one of 300 — past the
/// 128 levels the margin holds, with no triple term written in the request and no host
/// code — answers whole too, the evaluation having kept stack for it as it grew.
#[test]
fn triple_calls_chained_at_run_time_answer_the_whole_term() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let data = dataset();
            for levels in [20, 300] {
                let mut body = format!("BIND(?t{levels} AS ?t)");
                for level in (1..=levels).rev() {
                    body = format!(
                        "LATERAL {{ BIND(TRIPLE(<{EX}s>, <{EX}p>, ?t{}) AS ?t{level}) {body} }}",
                        level - 1
                    );
                }
                let query = format!("SELECT ?t WHERE {{ BIND(<{EX}o> AS ?t0) {body} }}");
                let (variables, mut rows) =
                    solutions(query_on(&data, &query).expect("the chain answers"));
                assert_eq!(variables, ["t"]);
                let value = rows
                    .pop()
                    .and_then(|mut row| row.pop().flatten())
                    .expect("one row binding ?t");
                assert_eq!(rows.len(), 0, "one row only");
                let expected = chain_value(levels, TermValue::iri(format!("{EX}o")));
                assert_eq!(nesting(&value), levels);
                assert_eq!(value, expected, "{levels} levels: the term built");
                drop_flat(value);
                drop_flat(expected);
            }
        })
        .expect("spawn")
        .join()
        .expect("the thread returned");
}

/// A `FILTER` over more rows than the evaluator forks for, whose expression builds a term
/// deeper than the margin holds: a fork-join worker cannot keep stack for it past its own
/// work, so it refuses the term, and the filter runs again on the evaluating thread —
/// every row answers, exactly as the same filter over a handful of rows does. The
/// observing neighbour: at 100 levels, within the margin, the host function is called off
/// the evaluating thread (so the filter really forks) and every row answers too.
#[test]
fn a_term_too_deep_for_a_worker_is_built_on_the_evaluating_thread() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let mut builder = RdfDatasetBuilder::new();
            let p = builder.intern_iri(&format!("{EX}p"));
            let o = builder.intern_iri(&format!("{EX}o"));
            for index in 0..4_000 {
                let s = builder.intern_iri(&format!("{EX}s{index}"));
                builder.push_quad(s, p, o, None);
            }
            let data = builder.freeze().expect("freezes");
            let evaluating = std::thread::current().id();
            let elsewhere = Arc::new(AtomicUsize::new(0));
            let mut functions = UserFunctionRegistry::new();
            let counted = Arc::clone(&elsewhere);
            functions.register_native(
                deep_fn(),
                Arity::Exact(1),
                Volatility::Stable,
                Arc::new(move |args: &[&TermValue]| {
                    if std::thread::current().id() != evaluating {
                        counted.fetch_add(1, Ordering::Relaxed);
                    }
                    let levels = match args[0] {
                        TermValue::Literal { lexical_form, .. } => {
                            lexical_form.parse().unwrap_or(0)
                        }
                        _ => 0,
                    };
                    Ok(Some(chain_value(levels, TermValue::iri(format!("{EX}o")))))
                }),
            );
            let functions = NativeSparqlEngine::new()
                .bind_functions(functions, ExtensionEnv::empty())
                .expect("binds");
            let count = |levels: usize, limit: usize| {
                let query = format!(
                    "SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER(isTRIPLE(TRIPLE(?s, <{EX}p>, <{}>({levels})))) }} LIMIT {limit}",
                    deep_fn()
                );
                let result = NativeSparqlEngine::new()
                    .query_with_options_view(
                        &*data,
                        SparqlRequest {
                            query: &query,
                            base_iri: None,
                            substitutions: &[],
                        },
                        QueryOptions::new().with_functions(&functions),
                    )
                    .unwrap_or_else(|refusal| panic!("{levels} levels: {refusal:?}"));
                solutions(result).1.len()
            };
            assert_eq!(count(100, 10_000), 4_000, "within the margin: every row");
            assert!(
                elsewhere.swap(0, Ordering::Relaxed) > 0,
                "the filter forked: the function ran off the evaluating thread"
            );
            assert_eq!(count(500, 10_000), 4_000, "past it: every row, never a refusal");
            assert_eq!(count(500, 3), 3, "a handful of rows");
        })
        .expect("spawn")
        .join()
        .expect("the thread returned");
}
