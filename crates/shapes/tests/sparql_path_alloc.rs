// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **What one conforming focus node costs when the shape that validates it is
//! backed by SPARQL.**
//!
//! `tests/change_path_alloc.rs` pins the conforming change path at a constant six
//! allocations across thirty-nine constraint and path cases — and not one of those
//! cases contains a SPARQL-bearing shape. That is not an oversight in the counting;
//! it is a hole in the COVERAGE, and it is precisely the hole a SHACL-SPARQL
//! surface hides in. Every figure in that file can be perfect while `sh:sparql`
//! charges a focus node for a whole query egress, because that file never runs one.
//!
//! This file runs the four surfaces that do:
//!
//! * `sh:sparql` — a SHACL-SPARQL constraint on a node shape (SELECT, per focus
//!   node);
//! * a custom constraint component with a `sh:ask` validator (per VALUE NODE, so
//!   the measured fixture gives every focus two of them);
//! * a custom constraint component with a `sh:select` validator (per focus node);
//! * a SHACL-AF `sh:expression` whose body is a function call, which routes
//!   through the scalar-expression seam once per tuple of the cartesian product
//!   over its argument value-sets.
//!
//! # What is claimed, and what is not
//!
//! The headline of the change-path work is `delta(2N) == delta(N)` — no growth
//! term in the focus count at all. **That does not hold here, and this file says
//! so in its assertions rather than around them.** A SHACL-SPARQL shape runs one
//! SPARQL query PER FOCUS NODE, and a query evaluation is not allocation-free:
//! the pre-binding rewrite clones the prepared algebra and rebuilds it, and every
//! pre-bound term crosses as an owned `TermValue` whose IRI is a fresh `String`.
//! Those are properties of the evaluator's query interface, not of the change
//! path.
//!
//! So what is pinned here is the closed form, which CAN be pinned and which is
//! where the defect actually lived:
//!
//! ```text
//! allocations(N) == CHANGE_PATH_CONSTANT + per_focus_node * N
//! ```
//!
//! [`CHANGE_PATH_CONSTANT`] is the six allocations `change_path_alloc.rs` already
//! pins, and [`SparqlCase::per_focus_node`] is the SPARQL surface's marginal cost.
//! Both are asserted EXACTLY, at `N` and at `2N`: an allocation count is a fact
//! about the code, not about the host, so a tolerance here would be a place for a
//! regression to hide. The form held to the single allocation at both sizes for
//! all four surfaces when it was written.
//!
//! # What was measured
//!
//! Marginal cost per conforming focus node, from this file's fixtures at
//! `N = 1,280` and `2N = 2,560`. The "before" column is this same file, unchanged,
//! run against the revision this one replaced:
//!
//! | surface | allocations before | after | requested bytes before | after |
//! |---|---|---|---|---|
//! | `sh:sparql` constraint | 2,695 | 54 | 1,277,672 | 3,518 |
//! | custom `sh:ask` component (2 value nodes) | 350 | 122 | 16,156 | 7,460 |
//! | custom `sh:select` component | 2,738 | 67 | 1,278,972 | 4,093 |
//! | SHACL-AF `sh:expression` call (2 tuples) | 236 | 131 | 13,393 | 7,456 |
//!
//! The "after" column is the figure pinned below, which is a live number rather
//! than a historical one: it moves whenever the evaluator's per-query setup gets
//! cheaper, and the pins move with it.
//!
//! # The most recent drop: a prepared execution now retains its SUBSTITUTED plan
//!
//! The four surfaces most recently dropped by 14, 22, 14 and 20 allocations
//! respectively (from 68, 144, 81 and 151) when a prepared execution started
//! retaining the REWRITTEN algebra across runs — `purrdf_sparql_eval::prebind_memo`
//! — instead of only the admitted plan the rewrite starts from. Every run before
//! this cloned the admitted algebra and walked it again to push each pre-bound
//! constant into the patterns that carry it; a [`PrebindMemo`](
//! purrdf_sparql_eval::execution) instead writes each run's values directly into
//! the cells of a tree built once, so a run costs a handful of refcount bumps
//! where it used to cost a whole clone-and-walk.
//!
//! Measuring this required a second change alongside it, not a testing nicety: the
//! memo carries a differential oracle (`PreparedExecution::substituted`, gated
//! `#[cfg(debug_assertions)]`) that re-runs the very rewrite the memo exists to
//! avoid on every hit, to prove the memo never answers differently from it. `cargo
//! test` always builds with `debug_assertions` on, so without a way to turn that
//! check off, this file would have measured the oracle's cost instead of the
//! memo's — indistinguishable from the memo saving nothing at all. So this file
//! brackets its measured window with
//! `purrdf_sparql_eval::set_memo_verification_enabled(false)`, broadcast to every
//! `rayon` worker (see [`without_memo_verification`]) and restored immediately
//! after; every OTHER debug test in the workspace, and every call these figures
//! don't bracket, still runs the oracle. An instrumented run of this same fixture
//! confirmed the shape the numbers imply: inside the measured window every one of
//! `sh:sparql`'s 11,520 calls into `substituted` (`REPETITIONS` × (`N` + `2N`)) was
//! a memo HIT — building happens once per case, entirely inside
//! [`warm_every_worker`]'s second broadcast, ahead of the window — so the whole
//! marginal drop is the avoided oracle recompute, at 10–14 allocations per hit
//! across the four surfaces (the `sh:ask` and `sh:expression` figures divide their
//! drop by two, because those two surfaces call `substituted` twice per focus
//! node).
//!
//! The four surfaces before that dropped by 2, 0, 1 and 13 allocations respectively
//! (from 70, 144, 82 and 164) when SHACL stopped reaching the evaluator through query
//! TEXT. Every one of these surfaces runs the same query over and over, changing
//! only the terms pre-bound into it, so each surface now holds a prepared execution
//! — parsed and admitted once, its parameter names interned once, its bindings
//! written into slots — checked out of a per-worker table keyed by query text and
//! parameter list. Three changes made that pay rather than cost:
//!
//! * the two algebra soundness walks the run path performed (a full `validate` and
//!   a graph-pattern depth check, each allocating a traversal stack and growing it
//!   with the query) are properties of the PLAN, so a prepared execution runs them
//!   once at preparation instead of once per run;
//! * the replanning walk that checks a plan against the supplied registries returns
//!   immediately when no registry is configured on either side, which is decidable
//!   without walking anything and is the configuration a SHACL host normally has;
//! * `$PATH` substitution returns `Cow`, so the two pass-through cases — a node
//!   shape, and a property shape whose query does not mention the placeholder —
//!   stop copying the whole query text per focus node.
//!
//! The `sh:ask` figure is unchanged rather than improved, and that is the honest
//! reading: its validator already hoisted its pre-binding list out of the value-node
//! loop, so what the handle removed there (the plan-cache probe and the name
//! interning per value node) is matched by what it added (the parameter-name list
//! per focus node). What the handle does NOT touch on any surface is the dominant
//! term — the pre-binding rewrite still clones the admitted algebra and rewrites it
//! per run, because a prepared execution caches the plan, not the substituted plan.
//!
//! Before that, the four surfaces dropped by 26, 70, 34 and 30 allocations
//! respectively (from 96, 214, 116 and 194) through six changes: the pre-binding rewrite stopped grounding every pre-bound value twice,
//! once for the `VALUES` seed and again for the expression-position walk; it
//! stopped rebuilding the algebra in order to rewrite it, both halves now mutating
//! a clone of the prepared plan in place so a visited node costs no fresh `Box`;
//! the empty variable schema, a constant reached on every execution, became a
//! process-wide shared one; a variable schema narrower than nine columns stopped
//! building a hash index it can answer by scanning; the `Variable` for a
//! pre-binding name, which is shape text and constant across every focus node in a
//! run, is interned per worker instead of rebuilt from a borrow twice over; and a
//! column layout, which is a plan constant reached through a node that is a fresh
//! heap temporary, is interned per worker by its CONTENT rather than rebuilt.
//!
//! # The two big rows and the two small ones are two different findings
//!
//! **The two SELECT surfaces were scanning the graph.** That is what the 1.28 MB
//! is, and it is not the honest price of running a query — it is a planning
//! defect, now fixed in `purrdf_sparql_eval`'s pre-binding rewrite. Pre-binding
//! `$this` used to bind it ONLY by joining a single-row `VALUES` seed onto the
//! core pattern, leaving the triple pattern's subject a VARIABLE; the join
//! evaluates both operands in full, so every focus node enumerated every
//! `ex:name` quad in the dataset and then discarded all but its own. The
//! deciding measurement holds the focus count fixed at 64 and varies the data
//! graph: before, the per-focus-node cost tracked the graph exactly (383
//! allocations over 768 quads, 8,324 over 24,576); after, it is FLAT across every
//! one of those sizes (at 100, the figure pinned when that measurement was taken).
//! The constant is pushed into the pattern now, so the bound position is an index
//! probe.
//!
//! **The two non-SELECT surfaces were never scanning**, which is why that step
//! moved their bytes by nothing at all — 16,156 and 13,393 on both sides of it:
//! an ASK materializes no rows on any path, and the expression call's body has no
//! BGP with a pre-bound term in it. (The later per-query-setup work above does
//! show in their bytes, because it removed whole buffers rather than rows.) Their
//! allocation savings come from the pre-binding rewrite getting cheaper rather
//! than narrower — one combined `VALUES` seed carrying every pre-bound variable
//! instead of one seed, one `Join` and one whole rebuild of the
//! solution-modifier stack PER variable, and a pre-binding list whose variable
//! names are borrowed from the shapes graph instead of re-allocated per focus
//! node.
//!
//! # The instrument, and the traps it is threaded around
//!
//! Identical to `tests/change_path_alloc.rs` and `tests/box_role_alloc.rs`, and
//! for the same reasons:
//!
//! * SHACL validation fans focus nodes over `rayon` above its parallel threshold,
//!   so a per-thread window would miss every worker. Every measurement here uses a
//!   [`WholeProcessWindow`].
//! * That window reads one process-global ledger and `cargo test` runs a binary's
//!   tests concurrently, so [`MEASURE_LOCK`] serializes every measured region in
//!   this binary. Poisoning is absorbed, so one failure does not cascade.
//! * [`FOCUS_NODES`] and its double both sit above [`PARALLEL_MIN_FOCUS_NODES`],
//!   checked when the file compiles, and [`assert_parallel_path_is_reachable`]
//!   refuses to report a figure from a host whose pool cannot take that path.
//! * Every measured region is executed once, with the same arguments, BEFORE its
//!   window opens: `rayon`'s global pool, the thread-local SPARQL engine's plan
//!   cache and the allocator's arenas are all first-touch lazies.
//! * [`measure_min`] keeps the smallest of [`REPETITIONS`] executions, because
//!   `rayon`'s work-stealing deque allocates a block every 63 pushes and a single
//!   sample can read one extra. A minimum is not a tolerance: a real per-focus
//!   term is charged to every execution and so to the minimum too.
//! * Both the `N` and the `2N` warm-up run TWICE, because an `N`-sized fan-out
//!   reaches a different set of workers than a `2N`-sized one and the engine's
//!   plan cache is a thread-local. See the warm-up block in the headline test.
//!
//! One trap is specific to this file. The engine memoizes query PLANS in a
//! thread-local cache keyed on query text, and a first-touch parse is a large,
//! one-off allocation burst. It is charged to whichever measurement runs first
//! unless the warm-up has already taken it — which is why the warm-up here runs
//! the exact case, on the exact focus slice, before the window opens, exactly as
//! the two sibling files do.
//!
//! # Fixtures
//!
//! Every IRI is under `example.org`; PurRDF mints no vocabulary IRIs. The query
//! bodies spell their predicates as absolute IRIs rather than relying on a
//! `sh:prefixes` declaration, so what each query matches is readable without
//! resolving a prefix map.
//!
//! # Running it
//!
//! ```text
//! cargo test -p purrdf-shapes --test sparql_path_alloc
//! ```

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_shapes::engine::{FocusId, PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::term::NamedNode;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Serializes every measured region in this binary.
///
/// [`WholeProcessWindow`] reads one process-global ledger and `cargo test` runs
/// test functions concurrently, so two measurements in flight at once would each
/// report the union of both regions while appearing to report their own.
static MEASURE_LOCK: Mutex<()> = Mutex::new(());

/// Take [`MEASURE_LOCK`], absorbing poison.
///
/// A panicking assertion inside a measured region poisons the mutex. Propagating
/// that would turn one real failure into a cascade of unrelated ones and bury the
/// diagnosis; the lock guards a counter, not an invariant a panic could have left
/// half-written.
fn measure_lock() -> MutexGuard<'static, ()> {
    MEASURE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Run `operation` inside a whole-process allocation window.
fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = WholeProcessWindow::open();
    let value = operation();
    (value, window.close())
}

/// How many times a measured region is executed, keeping the smallest figure.
///
/// Three, for the reason `tests/change_path_alloc.rs` states at length: `rayon`'s
/// injector allocates a block every 63 pushes, so one execution in a run can read
/// one allocation more than the code performed. Two executions are the minimum
/// that can miss a block boundary; the third is margin.
const REPETITIONS: usize = 3;

/// At least two executions, or the property the constant is chosen for is not
/// available at all.
const _: () = assert!(
    REPETITIONS >= 2,
    "a single execution cannot exclude the injector's block allocation"
);

/// Execute a measured region [`REPETITIONS`] times and keep the smallest.
///
/// Not a tolerance: a genuine per-focus-node term is charged to every execution
/// and therefore to the minimum as well. It removes a cost that is not the
/// measured code's, exactly as the warm-up beside it does.
fn measure_min<T>(mut operation: impl FnMut() -> T) -> (T, Measurement) {
    let mut best: Option<(T, Measurement)> = None;
    for _ in 0..REPETITIONS {
        let (value, measured) = measure(&mut operation);
        if best
            .as_ref()
            .is_none_or(|(_, seen)| measured.allocations < seen.allocations)
        {
            best = Some((value, measured));
        }
    }
    best.expect("REPETITIONS is non-zero, so at least one execution was measured")
}

/// The fixture namespace. Caller-supplied and `example.org` by rule.
const NS: &str = "http://example.org/purrdf/sparql-path#";

/// `rdf:type`, spelled out because the fixture is built id-natively.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Mirrors `PARALLEL_MIN_FOCUS_NODES` in `crates/shapes/src/parallel.rs`.
///
/// That constant is `pub(crate)`, so an integration test cannot read it. It is
/// mirrored rather than approximated because the whole point of [`FOCUS_NODES`]
/// is to sit above it, and [`assert_parallel_path_is_reachable`] is what turns the
/// mirror into a checked claim about the host.
const PARALLEL_MIN_FOCUS_NODES: u64 = 1_024;

/// The focus-node count `N` in every `N` versus `2N` comparison.
///
/// Above the parallel threshold, and no higher than it needs to be: every focus
/// node here runs a whole SPARQL query, so this file's work per node is orders of
/// magnitude above the sibling files' and the size is chosen to keep a debug-build
/// run of the suite finite.
const FOCUS_NODES: u64 = 1_280;

/// `multiple * N` as a slice length.
///
/// The counts are `u64` because that is what a [`Measurement`] reports and every
/// arithmetic statement this file makes is between a count and a measurement; the
/// conversion happens here, once, where a slice needs one.
fn focus_nodes(multiple: u64) -> usize {
    usize::try_from(FOCUS_NODES * multiple).expect("the focus-node count fits a machine word")
}

/// What validating a conforming focus set costs BEFORE the first focus node.
///
/// Six, which is not a number this file discovered: it is the constant
/// `tests/change_path_alloc.rs` pins across all thirty-nine of its constraint and
/// path cases, and it is the entry cost of the change path itself — the same six
/// whether the population is one focus node or four thousand. It appears here as
/// the intercept of the closed form, so what this file's per-case constants
/// measure is the SPARQL surface's marginal cost and nothing else.
///
/// If it ever moves, it moves in that file first, and a failure here that quotes
/// this constant is pointing at the change path rather than at SPARQL.
const CHANGE_PATH_CONSTANT: u64 = 6;

/// How many violating focus nodes the fixture carries.
///
/// Small and fixed: the violating branch exists to prove each shape really
/// evaluates, and every result it produces is inspected.
const VIOLATIONS: usize = 4;

/// N crosses the parallel threshold, checked when the file compiles.
const _: () = assert!(
    FOCUS_NODES > PARALLEL_MIN_FOCUS_NODES,
    "N must exceed the parallel threshold, or the invariant is stated over the serial path"
);

/// And so does 2N, or the two measurements would be of two different schedulers.
const _: () = assert!(
    2 * FOCUS_NODES > PARALLEL_MIN_FOCUS_NODES,
    "2N must exceed the parallel threshold too, or the two measurements are of different code"
);

/// The Turtle prefixes every case's shapes graph opens with.
const PREFIXES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "@prefix ex: <http://example.org/purrdf/sparql-path#> .\n",
);

/// One SPARQL-bearing surface, measured on its own.
struct SparqlCase {
    /// Appears in every assertion message, so a failure names the surface.
    name: &'static str,
    /// The shapes graph body, appended to [`PREFIXES`].
    shapes: &'static str,
    /// The allocations one conforming focus node costs this surface.
    ///
    /// Pinned per case rather than shared: the four surfaces do genuinely
    /// different amounts of work (the ASK component runs once per value node and
    /// the fixture gives every focus two; the expression call runs the scalar seam
    /// once per tuple of its cartesian product), so one shared number would either
    /// be wrong for three of them or be a bound loose enough to pin nothing.
    per_focus_node: u64,
    /// How many results one VIOLATING focus node must produce.
    ///
    /// Also per case: the ASK component reports per failing value node, the others
    /// report once.
    results_per_violation: usize,
}

/// The four SPARQL-bearing surfaces.
///
/// Every query body spells its predicates as absolute IRIs, so no `sh:prefixes`
/// declaration stands between the reader and what the query matches.
const CASES: &[SparqlCase] = &[
    SparqlCase {
        // SHACL-SPARQL §5.3: a SELECT whose solutions ARE the violations. A
        // conforming focus node produces the empty solution bag, which is exactly
        // the shape of result the interned egress exists to stop paying for.
        name: "sh:sparql",
        shapes: concat!(
            "ex:SparqlShape a sh:NodeShape ; sh:targetClass ex:Focus ;\n",
            "    sh:sparql [ a sh:SPARQLConstraint ; sh:select \"\"\"\n",
            "        SELECT $this WHERE {\n",
            "          $this <http://example.org/purrdf/sparql-path#name> ?n .\n",
            "          FILTER(!isLiteral(?n))\n",
            "        }\"\"\" ] .\n",
        ),
        per_focus_node: 54,
        results_per_violation: 1,
    },
    SparqlCase {
        // A custom constraint component with an ASK validator, on a PROPERTY
        // shape: the validator runs once per value node, and the fixture gives
        // every focus node two. This is the surface the per-value-node
        // substitution rebuild was charged to.
        name: "sh:ask component",
        shapes: concat!(
            "ex:AskComponent a sh:ConstraintComponent ;\n",
            "    sh:parameter [ sh:path ex:askParam ] ;\n",
            "    sh:validator [ a sh:SPARQLAskValidator ;\n",
            "        sh:ask \"ASK { FILTER(isLiteral($value) && $askParam) }\" ] .\n",
            "ex:AskShape a sh:NodeShape ; sh:targetClass ex:Focus ;\n",
            "    sh:property [ sh:path ex:name ; ex:askParam true ] .\n",
        ),
        per_focus_node: 122,
        results_per_violation: 1,
    },
    SparqlCase {
        // A custom constraint component with a SELECT validator, on a node shape:
        // one query per focus node, solutions are violations.
        name: "sh:select component",
        shapes: concat!(
            "ex:SelectComponent a sh:ConstraintComponent ;\n",
            "    sh:parameter [ sh:path ex:selectParam ] ;\n",
            // `sh:validator` admits only ASK validators; a SELECT one is declared
            // for the shape kind it applies to, and this component is on a node
            // shape.
            "    sh:nodeValidator [ a sh:SPARQLSelectValidator ; sh:select \"\"\"\n",
            "        SELECT $this ?value WHERE {\n",
            "          $this <http://example.org/purrdf/sparql-path#name> ?value .\n",
            "          FILTER(!isLiteral(?value))\n",
            "        }\"\"\" ] .\n",
            "ex:SelectShape a sh:NodeShape ; sh:targetClass ex:Focus ;\n",
            "    ex:selectParam true .\n",
        ),
        per_focus_node: 67,
        results_per_violation: 1,
    },
    SparqlCase {
        // A SHACL-AF node expression whose body is a FUNCTION CALL. The callee is
        // a keyword-only SPARQL builtin named by its XPath IRI, so it lowers to
        // `CONTAINS(?a0, ?a1)` and routes through the scalar-expression seam once
        // per tuple of the cartesian product over the argument value-sets — twice
        // per focus node here, because `ex:name` carries two values.
        name: "sh:expression call",
        shapes: concat!(
            "ex:ExprShape a sh:NodeShape ; sh:targetClass ex:Focus ;\n",
            "    sh:expression [ <http://www.w3.org/2005/xpath-functions#contains>\n",
            "        ( [ shnex:pathValues ex:name ] \"item\" ) ] .\n",
        ),
        per_focus_node: 131,
        results_per_violation: 1,
    },
];

/// The measured fixture: one dataset and one prepared validator per case.
struct Fixture {
    /// One bound validator per entry of [`CASES`], in the same order.
    validators: Vec<PreparedValidator>,
    /// Focus nodes that satisfy every case's shape, in construction order, per
    /// entry of [`validators`] and in the same order.
    ///
    /// A [`FocusId`] names the binding that minted it, and each case gets its
    /// own binding over the shared dataset, so the ids are resolved once per
    /// case. That happens in [`Fixture::build`], outside every measured window,
    /// exactly as the bare ids were.
    ///
    /// [`validators`]: Fixture::validators
    conforming: Vec<Vec<FocusId>>,
    /// Focus nodes that break every case's shape, in the same per-case shape as
    /// [`Fixture::conforming`].
    violating: Vec<Vec<FocusId>>,
}

/// Build the shared data graph.
///
/// A conforming focus node carries TWO literal `ex:name` values, both containing
/// `item`: two values is what makes the per-value-node surfaces (the ASK
/// validator, and the expression call's cartesian product) run more than once, and
/// both containing `item` is what keeps the expression's unioned result the single
/// term `true` rather than `{true, false}`.
///
/// A violating focus node carries one `ex:name` value that is an IRI. That single
/// fact breaks all four shapes at once — a non-literal solution for the two
/// SELECTs, a false ASK, and a CONTAINS type error (so no value, so not `true`)
/// for the expression — which is what lets one violating population witness every
/// case without four differently-broken fixtures.
fn build_dataset(conforming: usize, violating: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let focus_class = builder.intern_iri(&format!("{NS}Focus"));
    let name = builder.intern_iri(&format!("{NS}name"));

    for index in 0..conforming {
        let focus = builder.intern_iri(&format!("{NS}c{index}"));
        builder.push_quad(focus, rdf_type, focus_class, None);
        for label in [format!("item-{index}"), format!("item-alt-{index}")] {
            let literal = builder.intern_literal(RdfLiteral::simple(label));
            builder.push_quad(focus, name, literal, None);
        }
    }
    for index in 0..violating {
        let focus = builder.intern_iri(&format!("{NS}v{index}"));
        builder.push_quad(focus, rdf_type, focus_class, None);
        let iri_value = builder.intern_iri(&format!("{NS}notALiteral{index}"));
        builder.push_quad(focus, name, iri_value, None);
    }
    builder
        .freeze()
        .expect("the SPARQL-path fixture must freeze")
}

impl Fixture {
    /// Build one dataset and one bound validator per case over it.
    fn build(conforming: usize, violating: usize) -> Self {
        let dataset = build_dataset(conforming, violating);
        let validators: Vec<PreparedValidator> = CASES
            .iter()
            .map(|case| {
                let mut ttl = String::from(PREFIXES);
                ttl.push_str(case.shapes);
                let shapes = parse_shapes(&ttl, None).unwrap_or_else(|error| {
                    panic!("case {}: the shapes graph must parse: {error}", case.name)
                });
                PreparedShapes::new(Arc::new(shapes))
                    .bind_shared_dataset(Arc::clone(&dataset))
                    .unwrap_or_else(|error| {
                        panic!("case {}: the fixture must bind: {error}", case.name)
                    })
            })
            .collect();
        let resolve = |validator: &PreparedValidator, prefix: char, count: usize| -> Vec<FocusId> {
            (0..count)
                .map(|index| {
                    let iri = format!("{NS}{prefix}{index}");
                    validator
                        .term_id(&NamedNode::new_unchecked(iri.clone()).into_term())
                        .unwrap_or_else(|| panic!("focus {iri} must be interned"))
                })
                .collect()
        };
        Self {
            conforming: validators
                .iter()
                .map(|validator| resolve(validator, 'c', conforming))
                .collect(),
            violating: validators
                .iter()
                .map(|validator| resolve(validator, 'v', violating))
                .collect(),
            validators,
        }
    }

    /// Validate the first `focus_nodes` conforming ids through one case,
    /// requiring the report to conform.
    fn validate_conforming(&self, case: usize, focus_nodes: usize) -> ValidationReport {
        let name = CASES[case].name;
        let report = self.validators[case]
            .validate_focus_node_ids(&self.conforming[case][..focus_nodes])
            .unwrap_or_else(|error| {
                panic!("case {name}: the conforming set must validate: {error}")
            });
        assert!(
            report.conforms,
            "case {name}: the conforming population must conform, or the allocation figure \
             describes a workload that never reached the constraint ({} result(s), first: {:?})",
            report.results.len(),
            report.results.first().map(|r| r.message.clone()),
        );
        report
    }

    /// Validate every violating id through one case, requiring the shape to
    /// really fire.
    fn validate_violating(&self, case: usize) -> ValidationReport {
        let SparqlCase {
            name,
            results_per_violation,
            ..
        } = CASES[case];
        let report = self.validators[case]
            .validate_focus_node_ids(&self.violating[case])
            .unwrap_or_else(|error| {
                panic!("case {name}: the violating set must validate: {error}")
            });
        assert!(
            !report.conforms,
            "case {name}: the violating set must not conform"
        );
        assert_eq!(
            report.results.len(),
            self.violating[case].len() * results_per_violation,
            "case {name}: each of the {} violating focus nodes must produce {results_per_violation} \
             result(s); a different count means the fixture is not exercising the surface these \
             figures are written about",
            self.violating[case].len(),
        );
        report
    }
}

/// Initialise the per-worker lazies for `case` on EVERY thread of the pool.
///
/// The engine — and therefore the query plan cache it memoizes parses in — is a
/// THREAD-LOCAL, so its first touch is charged once per worker per query text, not
/// once per process. Measured here, that first touch is exactly 56 allocations,
/// and this host's pool has 32 workers: the first `N`-sized pass of a fresh case
/// reached 20 of them (1,120 allocations of excess) and the first `2N`-sized pass
/// reached the other 12 (672), after which every subsequent pass was exact.
///
/// Warming by running the workload is therefore a RACE against which workers a
/// chunking happens to reach, and it is one this file kept losing: a worker still
/// cold when the window opened added its 56 to that sample, and [`measure_min`]'s
/// minimum only removes it if at least one of the [`REPETITIONS`] samples found
/// every worker warm. Left to chance it failed about half of a twelve-run stress
/// on a 32-thread pool.
///
/// [`rayon::broadcast`] does not leave it to chance: it runs the closure ONCE ON
/// EVERY THREAD in the pool, so no worker can still be cold afterwards. Each
/// invocation validates a single focus node, which is below
/// [`PARALLEL_MIN_FOCUS_NODES`] and so stays serial on the worker that runs it —
/// exactly the per-worker initialisation wanted, with no nested fan-out.
///
/// This is not a tolerance. It changes which threads are warm BEFORE the window
/// opens; it does not change, soften, or exclude anything counted once it is open.
///
/// # The second broadcast, and the race it closes
///
/// A prepared execution's substituted-plan memo (`purrdf_sparql_eval::prebind_memo`)
/// builds on the SECOND consecutive sighting of one lane and value-shape list — so
/// one broadcast call leaves every worker on its FIRST sighting only, with the
/// build (several extra rewrites, charged once) still ahead of it. Left there, that
/// build is the SAME kind of race this function already exists to close: it lands
/// on whichever worker a later chunked pass happens to schedule its SECOND focus
/// node onto, and `rayon`'s work-stealing does not guarantee that is every worker
/// before the window opens — an unlucky schedule at the smaller `N` size left a
/// straggler's build inside the MEASURED region often enough to redden this file's
/// headline assertion nondeterministically. Calling [`rayon::broadcast`] a second
/// time gives every worker its second, consecutive, UNMEASURED sighting, so the
/// build happens here or not at all.
fn warm_every_worker(fixture: &Fixture, case: usize) {
    for _ in 0..2 {
        rayon::broadcast(|_| {
            drop(fixture.validate_conforming(case, 1));
        });
    }
}

/// Turn `PreparedExecution`'s memo differential oracle off on every worker, run
/// `operation`, then turn it back on — even if `operation` panics.
///
/// # Why the harness needs this at all
///
/// `purrdf_sparql_eval::set_memo_verification_enabled` exists because the oracle it
/// gates re-runs the FULL, un-memoized rewrite on every memo hit and compares it
/// against the memo's answer, so the oracle's own cost — a whole clone-and-walk of
/// the admitted algebra — is exactly the allocation `PrebindMemo` exists to remove.
/// `cargo test` always builds with `debug_assertions` on, so without this the
/// figures pinned below would measure the oracle, not the memo, and the memo's
/// saving would never show up in the one instrument built to see it. See that
/// function's rustdoc in `crates/sparql-eval/src/execution.rs` for the full case.
///
/// # Why a second `rayon::broadcast`, not the flag alone
///
/// The flag is a thread-local: setting it on the calling (test) thread does nothing
/// for the pool workers the parallel validation path actually runs on. This reuses
/// exactly the mechanism [`warm_every_worker`] already uses to reach every worker
/// deterministically before a window opens — a `rayon::broadcast` call runs its
/// closure once on every thread in the pool, so no worker is left with the oracle
/// on (or, afterwards, left with it off) by chance of which workers a chunked pass
/// happened to schedule onto.
///
/// # Scope: exactly this window, exactly these workers
///
/// The flag is restored with a second broadcast before returning, including on a
/// panicking `operation` (`std::panic::catch_unwind` plus a resume), so a failing
/// assertion inside `operation` cannot leave a later, unrelated test's oracle
/// silently disabled. Nothing outside the broadcasts is touched: a thread that is
/// never a worker in this pool (there is only one process-wide `rayon` pool, so
/// that means no other thread at all) never sees the flag change.
fn without_memo_verification<T>(operation: impl FnOnce() -> T) -> T {
    rayon::broadcast(|_| purrdf_sparql_eval::set_memo_verification_enabled(false));
    // `AssertUnwindSafe`, not a real `UnwindSafe` bound: `operation` only reads the
    // fixture (validation takes `&self`) and this function never inspects any state
    // `operation` touched after a panic, it only resumes the payload unchanged — the
    // hazard `UnwindSafe` exists to flag (observing a value a panic left
    // half-written) does not apply to a caller that never looks.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    rayon::broadcast(|_| purrdf_sparql_eval::set_memo_verification_enabled(true));
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Refuse to report a figure from a run where the parallel path cannot be taken.
fn assert_parallel_path_is_reachable() {
    assert!(
        rayon::current_num_threads() > 1,
        "SHACL validation stays serial on a single-threaded rayon pool, so this host cannot \
         exercise the parallel path these assertions are written about"
    );
}

// ---------------------------------------------------------------------------
// 1. The headline
// ---------------------------------------------------------------------------

/// **Every SPARQL-bearing surface costs exactly
/// `CHANGE_PATH_CONSTANT + per_focus_node * N` allocations, at `N` and at `2N`.**
///
/// A closed form, not a ratio. The whole cost of validating `N` conforming focus
/// nodes through one of these shapes is the change path's own constant plus a
/// fixed number per focus node — asserted as an EQUALITY against both pinned
/// numbers, at both sizes, with no tolerance. A ratio alone ("2N costs twice N")
/// is satisfied by a surface that doubled; a closed form is not, and it also
/// separates the two things a reader wants separated: the entry cost of a
/// validation, and the marginal cost of one more focus node.
///
/// This is deliberately NOT the `delta(2N) == delta(N)` that
/// `tests/change_path_alloc.rs` holds to. A SHACL-SPARQL shape runs one SPARQL
/// query per focus node and a query evaluation is not allocation-free; see this
/// file's module documentation for what remains and why removing it was out of
/// reach here. Stating the achievable invariant exactly beats stating the
/// unachievable one loosely.
#[test]
fn every_sparql_surface_costs_a_constant_per_conforming_focus_node() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    let fixture = Fixture::build(focus_nodes(2), VIOLATIONS);

    // Non-vacuity, outside every window: a surface that stopped evaluating would
    // satisfy any allocation claim perfectly.
    for case in 0..CASES.len() {
        drop(fixture.validate_violating(case));
    }

    let mut report = String::new();
    let mut failures = String::new();
    for (case, spec) in CASES.iter().enumerate() {
        // Warm-up, OUTSIDE the window and with the exact arguments measured: the
        // thread-local engine's plan cache, rayon's global pool and the
        // allocator's arenas are first-touch lazies, and charging them to whichever
        // measurement ran first would make this a statement about start-up.
        //
        // [`warm_every_worker`] first, because the SPARQL engine and its query plan
        // cache are THREAD-LOCALS: their first touch is charged per worker, and
        // running the workload only warms the workers that workload's chunking
        // happened to reach. The broadcast reaches all of them by construction; see
        // that function for the measurement that made the difference visible.
        //
        // Then one pass at each size, on the exact arguments measured, to take the
        // process-wide first-touch costs the broadcast's single-focus-node runs do
        // not exercise: rayon's global pool at THIS fan-out width, and the
        // allocator's arenas at this working-set size.
        warm_every_worker(&fixture, case);
        drop(fixture.validate_conforming(case, focus_nodes(1)));
        drop(fixture.validate_conforming(case, focus_nodes(2)));

        // The measured window itself: the memo's differential oracle comes OFF here
        // and only here, on every worker, via `without_memo_verification` — see that
        // function for why a debug build's own correctness check would otherwise be
        // exactly the allocation these figures are trying to see past.
        let (half_measured, full_measured) = without_memo_verification(|| {
            let (half, half_measured) =
                measure_min(|| fixture.validate_conforming(case, focus_nodes(1)));
            drop(half);
            let (full, full_measured) =
                measure_min(|| fixture.validate_conforming(case, focus_nodes(2)));
            drop(full);
            (half_measured, full_measured)
        });

        let name = spec.name;
        let (n, two_n) = (half_measured.allocations, full_measured.allocations);
        // Requested BYTES beside the allocation count, because the two answer
        // different questions and the second one is what a reader wants when a
        // figure moves: an allocation count says how many times the code went to
        // the allocator, and the byte figure says how much traffic that was. The
        // scan this file's constants used to include was visible in both, and far
        // more dramatic in bytes.
        let (n_bytes, two_n_bytes) = (half_measured.requested_bytes, full_measured.requested_bytes);
        let _ = writeln!(
            report,
            "  {name}: N = {n}, 2N = {two_n}, marginal = {} allocations and {} requested bytes \
             per focus node, entry = {}",
            (two_n - n) / FOCUS_NODES,
            (two_n_bytes - n_bytes) / FOCUS_NODES,
            n.saturating_sub((two_n - n) / FOCUS_NODES * FOCUS_NODES),
        );
        for (label, measured, population) in [
            ("N", half_measured, FOCUS_NODES),
            ("2N", full_measured, 2 * FOCUS_NODES),
        ] {
            let expected = CHANGE_PATH_CONSTANT + spec.per_focus_node * population;
            if measured.allocations != expected {
                let _ = writeln!(
                    failures,
                    "  {name} at {label} = {population} conforming focus nodes: allocated {}, not \
                     the pinned {CHANGE_PATH_CONSTANT} + {} * {population} = {expected}\n    \
                     {measured:?}",
                    measured.allocations, spec.per_focus_node,
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the SPARQL-bearing surfaces' cost moved:\n{failures}\nall measurements:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// 2. The companion that makes the headline mean anything
// ---------------------------------------------------------------------------

/// **Every surface still reports the violation it is there to report, through the
/// same entry point the figures are taken over.**
///
/// "The SPARQL path allocates a constant" is satisfied exactly as well by a
/// validator that stopped running the query. This drives each case's VIOLATING
/// population through [`PreparedValidator::validate_focus_node_ids`] — the surface
/// the measurement uses, not a convenience wrapper beside it — and pins the focus
/// node and the reported value of every result.
#[test]
fn every_sparql_surface_still_reports_its_violations() {
    let _guard = measure_lock();
    let fixture = Fixture::build(focus_nodes(1), VIOLATIONS);
    let violating: Vec<String> = (0..VIOLATIONS).map(|i| format!("{NS}v{i}")).collect();

    for (case, spec) in CASES.iter().enumerate() {
        let report = fixture.validate_violating(case);
        let mut reported: Vec<String> = report
            .results
            .iter()
            .map(|result| result.focus_node.to_string())
            .collect();
        reported.sort_unstable();
        reported.dedup();
        let expected: Vec<String> = violating.iter().map(|iri| format!("<{iri}>")).collect();
        assert_eq!(
            reported, expected,
            "case {}: every violating focus node must appear in the report",
            spec.name
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The fixture really exercises what the figures are attributed to
// ---------------------------------------------------------------------------

/// **The measured fixture gives every conforming focus node two value nodes.**
///
/// Two of the four surfaces are charged PER VALUE NODE — the ASK validator runs
/// once per value of `ex:name`, and the expression call's cartesian product has
/// one tuple per value. With one value apiece the per-value-node work in this
/// file's figures would be invisible, and the largest of the savings this revision
/// makes is exactly there. So the fixture's own shape is asserted rather than
/// assumed: this is the fact the ASK row's attribution rests on.
#[test]
fn the_fixture_gives_every_focus_node_two_value_nodes() {
    let _guard = measure_lock();
    let dataset = build_dataset(4, 1);
    let name = dataset
        .term_id_by_iri(&format!("{NS}name"))
        .expect("the fixture's path predicate must be interned");
    for index in 0..4 {
        let iri = format!("{NS}c{index}");
        let focus = dataset
            .term_id_by_iri(&iri)
            .unwrap_or_else(|| panic!("focus {iri} must be interned"));
        let values = dataset
            .quads()
            .filter(|quad| quad.s == focus && quad.p == name)
            .count();
        assert_eq!(
            values, 2,
            "conforming focus {iri} must carry two ex:name values, or the per-value-node \
             surfaces are measured over a single-valued path and their figures describe \
             different work than they claim"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The regression guard: every test in this binary holds MEASURE_LOCK first
// ---------------------------------------------------------------------------

/// Whether `attrs` carries a bare `#[test]` attribute.
///
/// Matches by attribute PATH, not by scanning the source text for the word
/// "test": a doc comment or a code comment that happens to contain that word
/// must never be read as marking a function.
fn is_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

/// Whether `block`'s FIRST statement is a `let` binding whose initializer is a
/// call to `measure_lock()`.
///
/// Not "somewhere in the body": [`WholeProcessWindow`] reads one process-global
/// ledger for the whole test, so a lock taken after even one allocation has
/// already let that allocation land unguarded. It must also be a `let` binding
/// and not a bare `measure_lock();` statement — the returned [`MutexGuard`] is a
/// temporary that drops at the end of a bare statement, which releases the lock
/// immediately rather than holding it for the test.
fn first_statement_holds_measure_lock(block: &syn::Block) -> bool {
    let Some(syn::Stmt::Local(local)) = block.stmts.first() else {
        return false;
    };
    let Some(init) = &local.init else {
        return false;
    };
    matches!(
        init.expr.as_ref(),
        syn::Expr::Call(call)
            if matches!(
                call.func.as_ref(),
                syn::Expr::Path(path) if path.path.is_ident("measure_lock")
            )
    )
}

/// Every `#[test]` function declared anywhere in the scanned file, in source
/// order.
///
/// Walks the whole file rather than only its top-level items, so a `#[test]`
/// nested inside a `mod` block cannot go unseen.
#[derive(Default)]
struct TestFns(Vec<syn::ItemFn>);

impl<'ast> syn::visit::Visit<'ast> for TestFns {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if is_test_attr(&item.attrs) {
            self.0.push(item.clone());
        }
        syn::visit::visit_item_fn(self, item);
    }
}

/// `text` with comment markers and line breaks flattened away, so a claim that
/// wraps across lines is one searchable string.
fn flattened(text: &str) -> String {
    text.lines()
        .map(|line| {
            line.trim_start()
                .trim_start_matches("///")
                .trim_start_matches("//!")
                .trim()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// **Every prose site this guard knows about quotes the CURRENT figures.**
///
/// The four constants below are assertion-bearing: a stale one fails loudly. The
/// SENTENCES that quote them are not — nothing anywhere re-reads them, so a
/// reduction that moved the constants and forgot a README would leave the crate
/// stating a number it no longer measures. That is not hypothetical: an
/// unqualified version of this claim shipped false once already, and the figures
/// moved six times in the change that added this test — and a version of it
/// covering only two of the six sites below shipped a SECOND time, when
/// `docs/design/purrdf-change-path-allocations.md` drifted for two more
/// reductions before a *different* gate (`scripts/check-doc-claims.py`) caught
/// it, one that this test never read.
///
/// So every prose site known to restate these four numbers is read here and
/// checked against the constants rather than against a reader's memory. Five
/// sites, not "every" one — the name says what this test actually sweeps rather
/// than promising a guarantee it cannot keep for a sixth site nobody has
/// registered yet:
///
/// * `crates/shapes/README.md` — the long and short prose forms;
/// * `crates/shapes/src/engine.rs` — the same two forms, on the rustdoc of
///   `validate_focus_nodes` and `validate_focus_node_ids`;
/// * `docs/design/purrdf-change-path-allocations.md` — the per-focus-node
///   table's four rows, the same four `scripts/check-doc-claims.py` checks
///   independently;
/// * this file's OWN module documentation — the `after` column of the
///   before/after table near the top of the file;
/// * `crates/shapes/tests/change_path_alloc.rs` — its `SIBLING_FILE_COVERAGE`
///   doc comment restates these as "the current" figure, in the same sentence
///   as the HISTORICAL baseline (96/214/116/194) the decomposition in the
///   design document was taken against. Only the "current" restatement is a
///   live claim; the historical baseline is a fixed label for a past
///   measurement and must not be swept as though it tracked a constant it no
///   longer describes. Distinguished explicitly below, not by the two numbers
///   simply never colliding.
///
/// Whitespace and comment markers are flattened first, because the same
/// sentence is wrapped differently in Markdown and in rustdoc.
#[test]
fn every_registered_prose_site_quotes_the_measured_figures() {
    let _guard = measure_lock();

    let readme = flattened(include_str!("../README.md"));
    let rustdoc = flattened(include_str!("../src/engine.rs"));
    let design_doc = flattened(include_str!(
        "../../../docs/design/purrdf-change-path-allocations.md"
    ));
    let own_module_doc = flattened(include_str!("sparql_path_alloc.rs"));
    let change_path_alloc = flattened(include_str!("change_path_alloc.rs"));

    // The long form, spelled identically in the crate README and in
    // `validate_focus_nodes`' rustdoc.
    let long = format!(
        "**{}** for a `sh:sparql` SELECT constraint, **{}** for a custom `sh:ask` \
         component over a two-valued path, **{}** for a custom `sh:select` component, \
         and **{}** for a `sh:expression` function call over two argument tuples.",
        CASES[0].per_focus_node,
        CASES[1].per_focus_node,
        CASES[2].per_focus_node,
        CASES[3].per_focus_node,
    );
    let long = flattened(&long);
    assert!(
        readme.contains(&long),
        "crates/shapes/README.md no longer quotes the measured figures.\n  expected: \
         {long}"
    );
    assert!(
        rustdoc.contains(&long),
        "validate_focus_nodes' rustdoc no longer quotes the measured figures.\n  \
         expected: {long}"
    );

    // The short form on `validate_focus_node_ids`.
    let short = format!(
        "{} / {} / {} / {} allocations by",
        CASES[0].per_focus_node,
        CASES[1].per_focus_node,
        CASES[2].per_focus_node,
        CASES[3].per_focus_node,
    );
    assert!(
        rustdoc.contains(&short),
        "validate_focus_node_ids' rustdoc no longer quotes the measured figures.\n  \
         expected: {short}"
    );

    // The design document's per-focus-node table, row by row — the same four
    // rows `scripts/check-doc-claims.py` checks, read here too so a regression in
    // that Python gate is not the only thing standing between the document and
    // these constants.
    const DESIGN_DOC_ROWS: [&str; 4] = [
        "`sh:sparql` constraint",
        "custom `sh:ask` component",
        "custom `sh:select` component",
        "`sh:expression` function call",
    ];
    for (index, label) in DESIGN_DOC_ROWS.iter().enumerate() {
        let row = format!("| {label} | {} |", CASES[index].per_focus_node);
        assert!(
            design_doc.contains(&row),
            "docs/design/purrdf-change-path-allocations.md no longer quotes the measured \
             figure for {label}.\n  expected row: {row}"
        );
    }

    // `crates/shapes/tests/conformance_memo.rs` was a sixth site here and is no
    // longer one. It quoted the `sh:sparql` figure to justify an ALLOCATION
    // inequality between its two arms, and that oracle has been retired: an
    // allocation total is a sum every layer beneath the SHACL conformance memo also
    // moves, and it inverted once a lower layer (`PreparedExecution`'s prebind memo)
    // landed under it while the conformance memo was still firing. That file now
    // counts how many times the inner shape's constraint is EVALUATED, which is the
    // quantity the memo changes and the only one it changes, so it restates no
    // figure from this file and nothing there can drift against these constants.

    // This file's own module documentation states an "after" figure per surface,
    // beside a frozen "before" figure and two frozen byte counts from the same
    // historical run. Only the live "after" allocation count is checked here; the
    // "before" figures and the byte counts describe one specific past
    // measurement and restate nothing asserted elsewhere.
    const OWN_TABLE_ROWS: [(&str, &str, &str, &str); 4] = [
        ("`sh:sparql` constraint", "2,695", "1,277,672", "3,518"),
        (
            "custom `sh:ask` component (2 value nodes)",
            "350",
            "16,156",
            "7,460",
        ),
        (
            "custom `sh:select` component",
            "2,738",
            "1,278,972",
            "4,093",
        ),
        (
            "SHACL-AF `sh:expression` call (2 tuples)",
            "236",
            "13,393",
            "7,456",
        ),
    ];
    for (index, (label, before, bytes_before, bytes_after)) in OWN_TABLE_ROWS.iter().enumerate() {
        let row = format!(
            "| {label} | {before} | {} | {bytes_before} | {bytes_after} |",
            CASES[index].per_focus_node
        );
        assert!(
            own_module_doc.contains(&row),
            "this file's own module documentation no longer quotes its measured \"after\" \
             figure for {label}.\n  expected row: {row}"
        );
    }

    // `change_path_alloc.rs` restates these four as "the current" figure inside
    // `SIBLING_FILE_COVERAGE`'s doc comment, explicitly distinct from the
    // HISTORICAL baseline (96/214/116/194) the same sentence names — that
    // baseline is a fixed label for the measurement the decomposition in
    // `docs/design/purrdf-change-path-allocations.md` was taken against, not a
    // live constant, and must not be swept here.
    let sibling_claim = format!(
        "the current {}/{}/{}/{}",
        CASES[0].per_focus_node,
        CASES[1].per_focus_node,
        CASES[2].per_focus_node,
        CASES[3].per_focus_node,
    );
    assert!(
        change_path_alloc.contains(&sibling_claim),
        "crates/shapes/tests/change_path_alloc.rs's `SIBLING_FILE_COVERAGE` doc comment no \
         longer quotes the measured figures as \"the current\" ones.\n  expected: \
         {sibling_claim}"
    );
    assert!(
        change_path_alloc.contains("historical baseline term of 96/214/116/194"),
        "crates/shapes/tests/change_path_alloc.rs no longer labels 96/214/116/194 as the \
         historical baseline; if that sentence moved, this guard's distinction between the \
         live figure and the historical one needs to move with it rather than start silently \
         matching the wrong number"
    );
}

/// **Every `#[test]` function in this binary takes [`MEASURE_LOCK`] as the FIRST
/// statement of its body.**
///
/// This file's own documentation states that rule; this is what enforces it. A
/// binary-wide [`WholeProcessWindow`] reads one process-global ledger, and
/// `cargo test` runs a binary's test functions CONCURRENTLY, so a test that
/// allocates without holding the lock — whether or not it takes a measurement of
/// its own — can land its traffic inside a sibling test's open window and shift
/// a pinned constant nondeterministically. A future test that omits the lock is
/// exactly the contamination source this file exists to rule out, so it must
/// fail loudly and name itself rather than show up as an occasional,
/// unattributed shift in someone else's figure.
///
/// A source scan rather than a runtime check: nothing observable at runtime
/// distinguishes "this test forgot to take the lock" from "this test never
/// needed it", so the only place the distinction is visible is the source
/// itself.
#[test]
fn every_test_in_this_binary_takes_the_measure_lock_first() {
    let _guard = measure_lock();
    let source = include_str!("sparql_path_alloc.rs");
    let parsed = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("this file must parse as Rust: {error}"));
    let mut collector = TestFns::default();
    syn::visit::Visit::visit_file(&mut collector, &parsed);

    assert!(
        !collector.0.is_empty(),
        "the scan found no #[test] function in this file at all, so this guard is reading \
         nothing"
    );

    let mut offenders: Vec<String> = collector
        .0
        .iter()
        .filter(|item| !first_statement_holds_measure_lock(&item.block))
        .map(|item| item.sig.ident.to_string())
        .collect();
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "these #[test] functions do not take MEASURE_LOCK as the first statement of their \
         body, so they can allocate concurrently with another test's WholeProcessWindow and \
         silently shift its pinned figure: {offenders:?}"
    );
}
