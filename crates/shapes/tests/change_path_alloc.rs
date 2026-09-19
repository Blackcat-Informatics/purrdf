// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Validating a conforming graph through the SHACL change path allocates a
//! bounded amount, independent of how many focus nodes the change touched.**
//!
//! The change path is the incremental route — [`PreparedValidator::validate_focus_node_ids`]
//! and [`PreparedValidator::validate_focus_nodes`] — that a caller drives after a
//! data change, handing the validator only the focus nodes that change could have
//! affected. Its whole reason to exist is that it is cheaper than revalidating the
//! graph. A per-focus-node allocation term destroys that: the route stays
//! asymptotically correct and quietly costs what it was built to avoid, and
//! nothing in a latency benchmark on a quiet machine says so, because allocator
//! traffic hides inside a number that also moves with cache state and scheduling.
//!
//! So the claim is stated here as an executable contract over the production
//! surfaces, in allocation COUNTS, which are facts about the code rather than
//! about the host.
//!
//! # What each test pins
//!
//! * [`conforming_change_path_allocation_has_no_growth_term_in_focus_count`] —
//!   the headline. One bound dataset, two validations of a **conforming** focus
//!   set differing only in size, through BOTH change-path entry points; the
//!   allocation deltas must be equal.
//! * [`violating_change_path_allocation_scales_with_violations_not_focus_count`]
//!   — the companion that makes the headline mean anything. A conforming-only
//!   assertion is satisfied perfectly by a validator that has stopped validating,
//!   so this one holds the violation count fixed while the conforming population
//!   doubles (cost must not move) and holds the conforming population fixed while
//!   the violation count doubles (cost must move), and checks that the results
//!   are really there.
//! * [`change_path_report_bytes_match_pinned_golden`] — the silent-drop guard.
//!   Allocation counts alone cannot distinguish "cheaper" from "no longer
//!   checking"; this pins the serialized report, byte for byte, for every
//!   constraint kind and path form the other tests drive.
//! * [`bind_allocation_is_independent_of_dataset_size_beyond_the_catalog`] and
//!   [`admit_allocation_is_independent_of_dataset_size`] — the two once-per-
//!   snapshot seams the change path sits on top of. A change path that allocates
//!   nothing per focus node is worth nothing if the bind in front of it is
//!   linear in the graph.
//!
//! `PreparedShapes::bind_dataset` is deliberately not one of the measured seams.
//! It projects the data graph into an owned snapshot before binding, so it is
//! linear in the graph by construction and always will be; asserting otherwise
//! over it would be asserting that a copy is free.
//!
//! # What holds today, and what does not
//!
//! The per-focus-node growth term these tests were written against is gone. A
//! focus node is carried as its interned identity and materialized only where a
//! result is built, the statement projection no longer allocates a dedup table
//! per probe, and every input-sized collection on the route is sized from its
//! input. Measured on this revision, the conforming change path allocates the
//! SAME figure for 2,048 and for 4,096 conforming focus nodes — six or seven
//! allocations for the whole validation, for every constraint kind and path form
//! in [`CASES`] but one — where it allocated 6,171 and 12,320 before. Binding the
//! seam dataset costs 59 either way, where it cost 94 against 97.
//!
//! Two residuals remain, both of them inside third-party crates, and neither of
//! them a per-focus-node term. Each was located by tracing the backtrace of every
//! allocation inside the window rather than by inference, and each is quantified
//! below because "it is probably the runtime" is the sentence a real growth term
//! hides behind.
//!
//! ## `rayon`'s job injector, once every 63 submissions
//!
//! Submitting parallel work from a thread that is not a `rayon` worker pushes one
//! job onto the pool's global injector queue (`rayon_core::Registry::inject`).
//! That queue is a `crossbeam_deque::Injector`, a linked list of fixed blocks
//! whose `BLOCK_CAP` is **63**: every 63rd push allocates the next block, and the
//! other 62 allocate nothing. So one window in a few dozen reads one higher than
//! its neighbours, and WHICH window is not luck — it is the phase of a
//! process-global push counter, which is why the step lands on the same case on
//! every run of a deterministic test binary and looks like a property of that
//! case.
//!
//! Three measurements say it is the injector and not this crate:
//!
//! * With **no PurRDF code in the window at all** — a bare
//!   `(0..4096).collect::<Vec<u64>>().par_chunks(64).map(<[u64]>::len).collect()`
//!   — 600 consecutive windows read 1 allocation except for 10 that read 2, and
//!   the windows that read 2 are spaced **exactly 63 apart**.
//! * Holding the focus count, the chunk count and the data **completely fixed**
//!   and repeating one 4,096-node validation 600 times, 581 windows read 6 and 19
//!   read 7, spaced 31–32 apart — the same 63-push period at the two injections a
//!   validation makes. A constant workload whose measurement still steps cannot be
//!   carrying a growth term in an input it never varied.
//! * The step disappears entirely when the submitting thread IS a worker, which
//!   is the one case `Registry::inject` is not on the path: 400 windows over the
//!   same two focus counts inside a `ThreadPool::install` read 6 and only 6.
//!
//! [`measure_min`] is what keeps it out of the figures, and it removes it by
//! construction rather than by tolerance: see that function.
//!
//! ## The `regex` crate's cache pool, in the `pattern` case only
//!
//! `regex::Regex::is_match` borrows a scratch `Cache` from a pool inside the
//! `regex` crate, sharded by thread. A worker that finds its shard empty builds a
//! fresh one, which costs 43 allocations, and 32 `rayon` workers hammering eight
//! shards do that constantly. Measured over 200 windows at 2,048 conforming
//! `pattern` focus nodes the count takes 36 distinct values from 6 to 3,618 in
//! steps of 43; at 4,096 it takes 72 distinct values from 264 to 4,865 and never
//! once reaches the floor the smaller size reaches. That is not a residual a
//! repetition can floor out, so `pattern` is named in
//! [`ALLOCATION_EXCLUSIONS`] and held out of the two exact-equality assertions —
//! and ONLY out of those; it still runs, still has to conform, still has to
//! produce its violations, and is still pinned byte for byte by the golden.
//!
//! The exclusion is not taken on trust.
//! [`pattern_change_path_allocation_has_no_growth_term_below_the_parallel_threshold`]
//! drives the same `sh:pattern` shape below [`PARALLEL_MIN_FOCUS_NODES`], where
//! the validation stays on one thread and the pool is never contended, and
//! asserts a slope of exactly zero. Measured there, 512 and 1,024 conforming
//! focus nodes both cost 5 allocations in 200 out of 200 windows each. The
//! change path under `sh:pattern` is therefore clean, and what the exclusion
//! excludes is the contended pool and nothing else.
//!
//! # NO-REBLESSING RULE
//!
//! **`fixtures/change-path-report.golden.txt` is written once, here.** It is the
//! guard against the one failure mode an allocation test cannot see: validation
//! that got cheaper by no longer producing results. A later change that finds
//! [`change_path_report_bytes_match_pinned_golden`] failing has found a SEMANTIC
//! CHANGE in SHACL validation and must stop and diagnose it. Regenerating the
//! golden to make the test pass is forbidden, and a regenerated golden is
//! indistinguishable from the bug it exists to catch.
//!
//! # The instrument, and the trap it is threaded around
//!
//! SHACL validation fans focus nodes out over `rayon` above its parallel
//! threshold, and that is precisely the regime whose allocation behaviour matters.
//! A [`purrdf_alloc_probe::CurrentThreadWindow`] cannot see a worker thread, so it
//! would report the parallel path as almost free and every assertion below would
//! be green while measuring nearly nothing. Every measurement here therefore uses
//! a [`WholeProcessWindow`], which counts every thread.
//!
//! That window reads one process-global ledger, and `cargo test` runs a binary's
//! test functions concurrently, so a second test measuring at the same time would
//! land its traffic inside the first one's figures — and so would a test that
//! takes no measurement at all but allocates while someone else's window is open.
//! [`MEASURE_LOCK`] serializes every measured region in this binary AND every
//! test that does enough work to be seen from one; each takes it first and holds
//! it for its whole body. Poisoning is absorbed rather than propagated, so one
//! failing test does not cascade into unrelated failures that hide it.
//!
//! [`FOCUS_NODES`] and its double are both above [`PARALLEL_MIN_FOCUS_NODES`], so
//! the invariant is stated over the parallel path rather than over a
//! single-threaded shadow of it, and [`assert_parallel_path_is_reachable`] refuses
//! to report a result from a build or host where that path cannot be taken.
//!
//! # Warm-up is part of the measurement
//!
//! Plenty of first-touch work in this stack allocates exactly once and would
//! otherwise be charged to whichever measurement happened to run first: `rayon`
//! builds its global thread pool lazily on the first parallel validation, the
//! regex cache and the class-membership index are populated on first use, the
//! SPARQL plan cache fills on first evaluation, and the allocator's own arenas
//! grow. Every measured region below is therefore executed once, with the same
//! arguments, BEFORE the window opens. That is why the warm-up calls look
//! redundant: they are the reason the figures are about steady-state validation
//! and not about process start-up.
//!
//! # Fixtures
//!
//! [`CASES`] is one case per SHACL Core constraint kind — including the recursive
//! ones, `sh:node`, `sh:and`, `sh:or`, `sh:xone`, `sh:not` and
//! `sh:qualifiedValueShape`, where "conforms fast" and "stopped validating" are
//! one refactor apart, and the two RDF 1.2 statement-layer kinds a property shape
//! carries outside the constraint enum, `sh:reificationRequired` and
//! `sh:reifierShape`, and the one kind whose shape is chosen by EVALUATING a node
//! expression against the data per value node, `sh:nodeByExpression` — plus one
//! case per SHACL path form. Tests 1 and 2 are
//! generated over it, so a new constraint kind cannot regress the invariant
//! without a named failure saying which kind broke. Every IRI is under
//! `example.org`: PurRDF mints no vocabulary IRIs, and a test fixture is no more
//! entitled to invent one than a release build is.
//!
//! # Running it
//!
//! ```text
//! cargo test -p purrdf-shapes --test change_path_alloc
//! ```
//!
//! Nothing here is `#[ignore]`d any more, and no assertion carries a tolerance.
//! Weakening one of them to make it pass would remove the only statement of the
//! contract this file exists for; the two residuals named above are properties of
//! `rayon` and of the `regex` crate's cache pool, not of the change path, and
//! each is answered by an instrument that removes a KNOWN third-party artefact
//! whose period or whose absence was measured first — never by widening what
//! counts as equal.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_shapes::engine::{FocusId, PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::term::{NamedNode, Term};

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
/// diagnosis; the lock guards a counter, not an invariant that a panic could have
/// left half-written.
fn measure_lock() -> MutexGuard<'static, ()> {
    MEASURE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Run `operation` inside a whole-process allocation window.
///
/// The caller is responsible for holding [`measure_lock`] and for having warmed
/// the operation first; this helper only brackets it.
fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = WholeProcessWindow::open();
    let value = operation();
    (value, window.close())
}

/// How many times [`measure_min`] executes a region before keeping the smallest
/// allocation count it saw.
///
/// Three, and the three is derived rather than tuned. `rayon`'s global injector
/// queue allocates a fresh block every `crossbeam_deque` `BLOCK_CAP` = **63**
/// pushes; a change-path validation pushes one job per parallel submission, which
/// for every shape in [`CASES`] is a small single-digit number. Three consecutive
/// executions therefore make well under 63 pushes between them, so AT MOST ONE of
/// the three windows can straddle a block boundary and at least two of them
/// cannot. Any `k >= 2` satisfying `k * pushes_per_validation < 63` would do; the
/// third execution is margin, not calibration, and no value of it can hide a
/// per-focus-node term, because a term that is present is present in all three.
const REPETITIONS: usize = 3;

/// At least two executions, or the property the constant is chosen for — that one
/// of them must miss the block boundary — is not available at all.
const _: () = assert!(
    REPETITIONS >= 2,
    "a single execution cannot exclude the injector's block allocation"
);

/// Execute a measured region [`REPETITIONS`] times and keep the smallest.
///
/// This is the same kind of instrument as the warm-up calls beside it, aimed at a
/// different once-in-a-while cost. A warm-up removes first-touch work by making
/// sure it has already happened; this removes `rayon`'s injector block allocation
/// by making sure at least one execution falls between two of them. Both remove a
/// cost that is NOT the measured code's, and neither changes what is compared: the
/// figures that come out are still exact allocation counts, still compared with
/// `==`, and still fail on a difference of one.
///
/// What it deliberately is not is a tolerance. A tolerance would let a real
/// per-focus-node term of the same magnitude through; a minimum cannot, because
/// such a term is charged to every execution and so to the minimum as well. It
/// also cannot mask a term that is merely intermittent in the CODE — the smallest
/// figure is still a figure the code really produced.
///
/// The value returned is the one produced by the execution the reported
/// measurement came from, so a caller's assertions about the report and its
/// assertions about the count describe the same run.
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
const NS: &str = "http://example.org/purrdf/change-path#";

/// `rdf:type`, spelled out because the fixtures are built id-natively.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// `rdfs:subClassOf`, which the seam fixture needs to reach the class-membership
/// index at all; see [`seam_dataset`].
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

/// `xsd:integer`, for the numeric-range and datatype cases.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The class every generated focus node carries and every generated shape targets.
const FOCUS_CLASS: &str = "Focus";

/// Mirrors `PARALLEL_MIN_FOCUS_NODES` in `crates/shapes/src/parallel.rs`.
///
/// That constant is `pub(crate)`, so an integration test cannot read it. It is
/// mirrored rather than approximated because the whole point of [`FOCUS_NODES`]
/// is to sit above it; [`assert_parallel_path_is_reachable`] is what turns the
/// mirror into a checked claim about the size relationship, and a threshold that
/// moved UP past [`FOCUS_NODES`] would show up as tests 1 and 2 suddenly
/// measuring a serial path — which is why the sizes carry a wide margin rather
/// than sitting one node above the line.
const PARALLEL_MIN_FOCUS_NODES: usize = 1_024;

/// The focus-node count `N` in the `delta(2N) == delta(N)` assertions.
///
/// Chosen so that both `N` and `2N` are comfortably above
/// [`PARALLEL_MIN_FOCUS_NODES`]: the invariant is being stated over the parallel
/// path, not over the serial path a smaller `N` would quietly select.
const FOCUS_NODES: usize = 2_048;

/// The violation count `V` in the violation-scaling assertions.
///
/// Small and fixed on purpose: it is the quantity that must drive the cost, so it
/// has to be independently variable from the focus count, and it has to stay far
/// enough below it that a per-focus-node term cannot masquerade as a per-violation
/// one.
const VIOLATIONS: usize = 8;

/// Focus nodes per dataset in the bind and admit size-independence tests.
const SEAM_FOCUS_NODES: usize = 4_096;

/// How many allocations one `bind_shared_dataset` costs over the
/// [`SEAM_FOCUS_NODES`]-focus-node seam dataset.
///
/// MEASURED on this revision, not chosen: binding that dataset makes 59
/// allocations, and binding twice as much instance data makes 59 as well.
///
/// The figure used to be 94, against 97 for twice the instance data, and the
/// companion size-independence assertion beside this one was red because of those
/// three. Two changes account for both numbers:
///
/// * the statement projection no longer dedups a probe's rows when the carrier
///   cannot produce a duplicate row in the first place — it used to allocate a
///   hash table on the first quad EVERY probe matched, and to grow that table for
///   a wide probe, so binding paid per probe and paid more for more data;
/// * the class-membership index sizes its `rdf:type` row buffer from a count
///   taken on the pass it already makes, rather than doubling its way up to the
///   instance count.
///
/// Neither weakens what this pins. The assertion is still exact equality against
/// a figure measured on this revision; the figure moved because binding got
/// cheaper and stopped tracking the graph, which is what the test beside it asks
/// for.
///
/// A determinism pin, NOT a timing threshold. An allocation count is a fact about
/// the code: the same revision produces the same number on every host, under any
/// load, at any core count, because nothing in binding consults a clock, a source
/// of randomness or the scheduler. A host-sensitive figure would have no business
/// being asserted; this one has no business being merely logged.
const BIND_ALLOC_CONST: u64 = 59;

/// How many allocations one prepared-product `admit` costs.
///
/// The same kind of pin as [`BIND_ALLOC_CONST`], over the other once-per-snapshot
/// seam, and here the figure really is constant: admission makes 282 allocations
/// with either seam dataset bound.
const ADMIT_ALLOC_CONST: u64 = 282;

/// Conforming focus nodes per case in the golden fixture.
const GOLDEN_CONFORMING: usize = 2;

/// Violating focus nodes per case in the golden fixture.
const GOLDEN_VIOLATING: usize = 2;

/// The pinned report text. See the NO-REBLESSING RULE in this module's docs.
const GOLDEN: &str = include_str!("fixtures/change-path-report.golden.txt");

/// Turtle prefixes every generated shapes graph opens with.
const PREFIXES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "@prefix ex: <http://example.org/purrdf/change-path#> .\n",
);

// ---------------------------------------------------------------------------
// The case table
// ---------------------------------------------------------------------------

/// The data-graph writer handed to a case, scoped to one focus node.
///
/// Every case builds its focus node's data through this, so a case is a few lines
/// of "what this node has" rather than a transcription of the builder API, and
/// the conforming and violating shapes of one constraint sit next to each other
/// where a reader can see that they really differ.
#[derive(Debug)]
struct Emit<'builder> {
    /// The dataset under construction.
    builder: &'builder mut RdfDatasetBuilder,
    /// The focus node this call is populating.
    focus: TermId,
    /// The focus node's local name, used to derive per-node auxiliary IRIs so
    /// that no two focus nodes share a target, parent or sibling node.
    scope: String,
    /// The focus node's ordinal, for cases whose values vary with it.
    index: usize,
}

impl Emit<'_> {
    /// Intern `{NS}{local}`.
    fn iri(&mut self, local: &str) -> TermId {
        let iri = format!("{NS}{local}");
        self.builder.intern_iri(&iri)
    }

    /// Intern an auxiliary node belonging to this focus node alone.
    fn scoped(&mut self, suffix: &str) -> TermId {
        let iri = format!("{NS}{}-{suffix}", self.scope);
        self.builder.intern_iri(&iri)
    }

    /// Intern a literal.
    fn lit(&mut self, literal: RdfLiteral) -> TermId {
        self.builder.intern_literal(literal)
    }

    /// Push `subject predicate object` into the default graph.
    fn quad(&mut self, subject: TermId, predicate: &str, object: TermId) {
        let predicate = self.iri(predicate);
        self.builder.push_quad(subject, predicate, object, None);
    }

    /// Push a triple whose subject is the focus node.
    fn prop(&mut self, predicate: &str, object: TermId) {
        let subject = self.focus;
        self.quad(subject, predicate, object);
    }

    /// Attach a simple literal to the focus node.
    fn text(&mut self, predicate: &str, value: &str) {
        let object = self.lit(RdfLiteral::simple(value));
        self.prop(predicate, object);
    }

    /// Attach an `xsd:integer` to the focus node.
    fn integer(&mut self, predicate: &str, value: i64) {
        let object = self.lit(RdfLiteral::typed(value.to_string(), XSD_INTEGER));
        self.prop(predicate, object);
    }

    /// Attach a language-tagged literal to the focus node.
    fn tagged(&mut self, predicate: &str, value: &str, language: &str) {
        let object = self.lit(RdfLiteral::language_tagged(value, language));
        self.prop(predicate, object);
    }

    /// Type `subject` as `{NS}{class}`.
    ///
    /// `rdf:type` is the one predicate that does not live under [`NS`], so it has
    /// its own method rather than a case spelling an absolute IRI inline.
    fn classify(&mut self, subject: TermId, class: &str) {
        let class = self.iri(class);
        let predicate = self.builder.intern_iri(RDF_TYPE);
        self.builder.push_quad(subject, predicate, class, None);
    }

    /// Declare a per-focus-node reifier for the statement
    /// `focus predicate object`, returning the reifier so a case can say more
    /// about it.
    ///
    /// The declaration goes through [`RdfDatasetBuilder::push_reifier`], which
    /// puts it in the RDF 1.2 reifier SIDE-TABLE — the table `sh:reifierShape`
    /// and `sh:reificationRequired` read. Writing a plain
    /// `(stmt, rdf:reifies, <<( s p o )>>)` row into the quad table instead would
    /// land it somewhere neither constraint consults, and both branches of a case
    /// built on it would violate, which is a fixture that measures the wrong thing
    /// while looking like it measures the right one.
    fn reify(&mut self, predicate: &str, object: TermId) -> TermId {
        let predicate = self.iri(predicate);
        let subject = self.focus;
        let triple = self.builder.intern_triple(subject, predicate, object);
        let reifier = self.scoped("stmt");
        self.builder.push_reifier(reifier, triple);
        reifier
    }

    /// Attach a per-focus-node target node typed `ex:Target`, returning it.
    fn target(&mut self, predicate: &str) -> TermId {
        let target = self.scoped("target");
        self.classify(target, "Target");
        self.prop(predicate, target);
        target
    }
}

/// One constraint kind or path form: the shape that states it, and the data that
/// makes a focus node satisfy or break it.
struct ConstraintCase {
    /// Appears in every assertion message, so a failure says which kind broke.
    name: &'static str,
    /// The shapes graph body, appended to [`PREFIXES`].
    shapes: &'static str,
    /// Writes one focus node's data; `violating` selects which shape it takes.
    emit: fn(&mut Emit<'_>, bool),
}

impl std::fmt::Debug for ConstraintCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstraintCase")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// Every constraint kind and path form the change path has to keep cheap AND keep
/// checking.
///
/// A case is a triple of claims: the shape states one constraint, the conforming
/// branch really conforms, and the violating branch really violates. All three are
/// executed — the conforming branch by test 1, which asserts the report conforms,
/// and the violating branch by test 2, which asserts the results are produced and
/// scale, and by the golden, which pins what they say.
const CASES: &[ConstraintCase] = &[
    ConstraintCase {
        name: "class",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:class ex:Target ] .",
        emit: |emit, violating| {
            if violating {
                let stray = emit.scoped("stray");
                emit.classify(stray, "Other");
                emit.prop("ref", stray);
            } else {
                emit.target("ref");
            }
        },
    },
    ConstraintCase {
        name: "datatype",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:datatype xsd:integer ] .",
        emit: |emit, violating| {
            if violating {
                emit.text("count", "not-a-number");
            } else {
                let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
                emit.integer("count", index);
            }
        },
    },
    ConstraintCase {
        name: "node_kind",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:nodeKind sh:IRI ] .",
        emit: |emit, violating| {
            if violating {
                emit.text("ref", "a literal is not an IRI");
            } else {
                emit.target("ref");
            }
        },
    },
    ConstraintCase {
        name: "min_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                let index = emit.index;
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "max_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:maxCount 1 ] .",
        emit: |emit, violating| {
            let index = emit.index;
            emit.text("name", &format!("item-{index}"));
            if violating {
                emit.text("name", &format!("item-{index}-again"));
            }
        },
    },
    ConstraintCase {
        name: "min_inclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:minInclusive 0 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { -1 } else { index });
        },
    },
    ConstraintCase {
        name: "max_inclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:maxInclusive 1000000 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { 1_000_001 } else { index });
        },
    },
    ConstraintCase {
        name: "min_exclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:minExclusive -1 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { -1 } else { index });
        },
    },
    ConstraintCase {
        name: "max_exclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:maxExclusive 1000000 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { 1_000_000 } else { index });
        },
    },
    ConstraintCase {
        name: "min_length",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:minLength 4 ] .",
        emit: |emit, violating| {
            let index = emit.index;
            if violating {
                emit.text("name", "ab");
            } else {
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "max_length",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:maxLength 32 ] .",
        emit: |emit, violating| {
            let index = emit.index;
            if violating {
                emit.text(
                    "name",
                    "this label is far longer than the thirty-two characters allowed",
                );
            } else {
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "pattern",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:pattern \"^item-[0-9]+$\" ] .",
        emit: |emit, violating| {
            let index = emit.index;
            if violating {
                emit.text("name", "does-not-match");
            } else {
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "language_in",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:label ; sh:languageIn ( \"en\" ) ] .",
        emit: |emit, violating| {
            if violating {
                emit.tagged("label", "bonjour", "fr");
            } else {
                emit.tagged("label", "hello", "en");
            }
        },
    },
    ConstraintCase {
        name: "unique_lang",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:label ; sh:uniqueLang true ] .",
        emit: |emit, violating| {
            emit.tagged("label", "hello", "en");
            if violating {
                emit.tagged("label", "hello again", "en");
            }
        },
    },
    ConstraintCase {
        name: "equals",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:equals ex:alias ] .",
        emit: |emit, violating| {
            let index = emit.index;
            emit.text("name", &format!("item-{index}"));
            if violating {
                emit.text("alias", &format!("other-{index}"));
            } else {
                emit.text("alias", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "disjoint",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:disjoint ex:alias ] .",
        emit: |emit, violating| {
            let index = emit.index;
            emit.text("name", &format!("item-{index}"));
            if violating {
                emit.text("alias", &format!("item-{index}"));
            } else {
                emit.text("alias", &format!("other-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "less_than",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:lessThan ex:limit ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", index);
            emit.integer("limit", if violating { index } else { index + 1 });
        },
    },
    ConstraintCase {
        name: "less_than_or_equals",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:lessThanOrEquals ex:limit ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", index);
            emit.integer("limit", if violating { index - 1 } else { index });
        },
    },
    ConstraintCase {
        name: "not",
        // The negated shape is spelled as a NODE shape wrapping a property
        // shape. Measured on this build, the anonymous PROPERTY-shape spelling
        // — `sh:not [ sh:path ex:flag ; sh:minCount 1 ]`, with or without an
        // explicit `a sh:PropertyShape` — reports a violation for EVERY focus
        // node, including nodes carrying no `ex:flag` at all, which is the
        // opposite of what SHACL states for `sh:not`. That behaviour is a
        // question about the engine, not about allocation, so the fixture uses
        // the spelling whose conforming and violating branches really are what
        // their names say; a case that reported every node as violating would
        // make the conforming half of this file untestable.
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:not [ a sh:NodeShape ; sh:property [ sh:path ex:flag ; sh:minCount 1 ] ] .",
        emit: |emit, violating| {
            if violating {
                emit.text("flag", "present");
            }
        },
    },
    ConstraintCase {
        name: "and",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:and (
                [ a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 ]
                [ a sh:PropertyShape ; sh:path ex:count ; sh:minCount 1 ]
            ) .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.text("name", "present");
            if !violating {
                emit.integer("count", index);
            }
        },
    },
    ConstraintCase {
        name: "or",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:or (
                [ a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 ]
                [ a sh:PropertyShape ; sh:path ex:count ; sh:minCount 1 ]
            ) .",
        emit: |emit, violating| {
            if !violating {
                emit.text("name", "present");
            }
        },
    },
    ConstraintCase {
        name: "xone",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:xone (
                [ a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 ]
                [ a sh:PropertyShape ; sh:path ex:count ; sh:minCount 1 ]
            ) .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.text("name", "present");
            if violating {
                emit.integer("count", index);
            }
        },
    },
    ConstraintCase {
        name: "node",
        shapes: "ex:TargetShape a sh:NodeShape ;
            sh:property [ sh:path ex:key ; sh:minCount 1 ] .
        ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:node ex:TargetShape ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                let key = emit.lit(RdfLiteral::simple("present"));
                emit.quad(target, "key", key);
            }
        },
    },
    ConstraintCase {
        name: "property",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:node [
                a sh:NodeShape ;
                sh:property [ sh:path ex:key ; sh:minLength 4 ]
            ] ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            let key = emit.lit(RdfLiteral::simple(if violating { "ab" } else { "present" }));
            emit.quad(target, "key", key);
        },
    },
    ConstraintCase {
        name: "qualified_min_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [
                sh:path ex:ref ;
                sh:qualifiedValueShape [ a sh:PropertyShape ; sh:path ex:key ; sh:minCount 1 ] ;
                sh:qualifiedMinCount 1
            ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                let key = emit.lit(RdfLiteral::simple("present"));
                emit.quad(target, "key", key);
            }
        },
    },
    ConstraintCase {
        name: "qualified_max_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [
                sh:path ex:ref ;
                sh:qualifiedValueShape [ a sh:PropertyShape ; sh:path ex:key ; sh:minCount 1 ] ;
                sh:qualifiedMaxCount 1
            ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            let key = emit.lit(RdfLiteral::simple("present"));
            emit.quad(target, "key", key);
            if violating {
                let second = emit.scoped("second");
                emit.classify(second, "Target");
                let other = emit.lit(RdfLiteral::simple("also present"));
                emit.quad(second, "key", other);
                emit.prop("ref", second);
            }
        },
    },
    ConstraintCase {
        name: "closed",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:closed true ;
            sh:ignoredProperties ( rdf:type ) ;
            sh:property [ sh:path ex:name ] .",
        emit: |emit, violating| {
            emit.text("name", "present");
            if violating {
                emit.text("stray", "not permitted by the closed shape");
            }
        },
    },
    ConstraintCase {
        name: "has_value",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:tag ; sh:hasValue \"alpha\" ] .",
        emit: |emit, violating| {
            emit.text("tag", if violating { "gamma" } else { "alpha" });
        },
    },
    ConstraintCase {
        name: "in",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:tag ; sh:in ( \"alpha\" \"beta\" ) ] .",
        emit: |emit, violating| {
            emit.text("tag", if violating { "gamma" } else { "alpha" });
        },
    },
    ConstraintCase {
        // The RDF 1.2 statement layer, asked the cheapest question it answers:
        // does ANY reifier exist for `<< focus ex:ref target >>`. The conforming
        // branch declares one, the violating branch declares none, and the
        // statement itself is present on both — so what separates them is the
        // statement-layer read and nothing on the property path.
        name: "reification_required",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:reificationRequired true ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                emit.reify("ref", target);
            }
        },
    },
    ConstraintCase {
        // The other half of the same layer: a reifier EXISTS on both branches, so
        // emptiness cannot be what decides the verdict — the reifier is submitted
        // to an inner shape, which is what judges it. That makes this the case
        // that fails if the reifier identities stop reaching the inner shape, and
        // `reification_required` above the one that fails if the existence probe
        // stops seeing them.
        name: "reifier_shapes",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:reifierShape [
                a sh:NodeShape ;
                sh:property [ sh:path ex:source ; sh:minCount 1 ]
            ] ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            let reifier = emit.reify("ref", target);
            if !violating {
                let source = emit.lit(RdfLiteral::simple("attested"));
                emit.quad(reifier, "source", source);
            }
        },
    },
    ConstraintCase {
        // SHACL 1.2 Node Expressions §7.2: the shape every value node is checked
        // against is COMPUTED out of the data graph, per value node, so — alone
        // among the kinds here — it cannot be resolved to a fixed shape when the
        // constraint is lowered. The value node names its own shape through
        // `ex:kind`, and BOTH branches name the same existing shape, so what
        // separates them is the conformance check the constraint performs and not
        // whether the produced IRI resolved.
        name: "node_by_expression",
        shapes: "@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
            ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
                sh:nodeByExpression [ shnex:pathValues ex:kind ] .
            ex:KindShape a sh:NodeShape ;
                sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            let kind = emit.iri("KindShape");
            emit.prop("kind", kind);
            if !violating {
                emit.text("name", "present");
            }
        },
    },
    ConstraintCase {
        name: "path_predicate",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                emit.text("name", "present");
            }
        },
    },
    ConstraintCase {
        name: "path_inverse",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:inversePath ex:child ] ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                let parent = emit.scoped("parent");
                let focus = emit.focus;
                emit.quad(parent, "child", focus);
            }
        },
    },
    ConstraintCase {
        name: "path_sequence",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ( ex:ref ex:key ) ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                let key = emit.lit(RdfLiteral::simple("present"));
                emit.quad(target, "key", key);
            }
        },
    },
    ConstraintCase {
        name: "path_alternative",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [
                sh:path [ sh:alternativePath ( ex:name ex:alias ) ] ;
                sh:minCount 1
            ] .",
        emit: |emit, violating| {
            if !violating {
                emit.text("alias", "present");
            }
        },
    },
    ConstraintCase {
        name: "path_zero_or_more",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:zeroOrMorePath ex:next ] ; sh:nodeKind sh:IRI ] .",
        emit: |emit, violating| {
            // A zero-or-more path always reaches the focus node itself, so the
            // constraint has to be one the REACHED nodes can break: a literal
            // successor is what makes this violate rather than a missing one.
            if violating {
                emit.text("next", "a literal is not an IRI");
            } else {
                emit.target("next");
            }
        },
    },
    ConstraintCase {
        name: "path_one_or_more",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:oneOrMorePath ex:next ] ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                emit.target("next");
            }
        },
    },
    ConstraintCase {
        name: "path_zero_or_one",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:zeroOrOnePath ex:next ] ; sh:nodeKind sh:IRI ] .",
        emit: |emit, violating| {
            // Same reason as `path_zero_or_more`: the zero-length leg always
            // matches, so only a bad successor can break it.
            if violating {
                emit.text("next", "a literal is not an IRI");
            } else {
                emit.target("next");
            }
        },
    },
];

/// Cases held out of the EXACT-EQUALITY allocation assertions, each with the
/// third-party cause that puts it there.
///
/// An exclusion list is a hole unless three things are true of it, and all three
/// are checked rather than asserted in prose:
///
/// * the cause is third-party and named — see the second half of this module's
///   documentation for the measurements behind the entry below;
/// * the exclusion is narrow. An excluded case still builds, still validates,
///   still has to conform on its conforming branch and produce exactly its
///   violations on its violating one, and is still pinned byte for byte by
///   [`change_path_report_bytes_match_pinned_golden`]. Only the two
///   `allocations(N) == allocations(2N)` comparisons skip it;
/// * the change path itself is still known clean under that constraint, proved by
///   a companion test that reaches the same constraint by a route the third-party
///   cause cannot be on. For `sh:pattern` that companion is
///   [`pattern_change_path_allocation_has_no_growth_term_below_the_parallel_threshold`].
///
/// [`every_allocation_exclusion_names_a_real_case`] keeps the list from
/// silently disabling an assertion for a case that no longer exists or never did.
const ALLOCATION_EXCLUSIONS: &[(&str, &str)] = &[(
    "pattern",
    "`regex::Regex::is_match` borrows a scratch `Cache` from a thread-sharded pool inside the \
     `regex` crate, and a worker that finds its shard empty builds a fresh one for 43 \
     allocations. With 32 rayon workers over eight shards that happens constantly and \
     nondeterministically: measured over 200 windows, 2,048 conforming pattern focus nodes take \
     36 distinct counts from 6 to 3,618 and 4,096 take 72 distinct counts from 264 to 4,865. The \
     cost is the regex crate's pool, not the change path — see \
     `pattern_change_path_allocation_has_no_growth_term_below_the_parallel_threshold`, which \
     drives the same shape on one thread and measures a slope of exactly zero.",
)];

/// The reason `case` is excluded from the exact-equality assertions, if it is.
fn allocation_exclusion(case: &str) -> Option<&'static str> {
    ALLOCATION_EXCLUSIONS
        .iter()
        .find(|(name, _)| *name == case)
        .map(|(_, reason)| *reason)
}

/// **Every excluded name is a case that exists.**
///
/// An exclusion keyed by a name no case carries is invisible: it excludes
/// nothing, reads as if it excludes something, and would just as happily survive
/// the case being renamed out from under the assertion it was meant to skip.
#[test]
fn every_allocation_exclusion_names_a_real_case() {
    let _guard = measure_lock();
    for (name, reason) in ALLOCATION_EXCLUSIONS {
        assert!(
            CASES.iter().any(|case| case.name == *name),
            "allocation exclusion {name:?} names no case in CASES, so it silently excludes \
             nothing while reading as though it excludes something"
        );
        assert!(
            !reason.trim().is_empty(),
            "allocation exclusion {name:?} carries no stated cause"
        );
    }
}

// ---------------------------------------------------------------------------
// Fixture construction
// ---------------------------------------------------------------------------

/// A case's dataset, its prepared validator, and the focus ids to drive it with.
#[derive(Debug)]
struct CaseFixture {
    /// The bound validator; the change path under measurement hangs off this.
    validator: PreparedValidator,
    /// Focus nodes that satisfy the case's shape, in construction order.
    ///
    /// Minted by [`Self::validator`], because a [`FocusId`] is only valid against
    /// the binding that issued it.
    conforming: Vec<FocusId>,
    /// Focus nodes that break it, in construction order.
    violating: Vec<FocusId>,
    /// The same conforming population in the term key space, for the term-keyed
    /// change path. Built here so that constructing the argument is never part of
    /// a measured region.
    conforming_terms: Vec<Term>,
}

/// Build and bind one case's dataset.
///
/// Conforming and violating focus nodes live in the SAME dataset, because the
/// change path validates only the ids it is handed: keeping both populations in
/// one graph is what lets the violation count and the focus count be varied
/// independently without rebuilding or rebinding anything, which in turn is what
/// keeps the two figures each test compares measurements of one preparation
/// rather than of two.
fn build_case(case: &ConstraintCase, conforming: usize, violating: usize) -> CaseFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let focus_class = builder.intern_iri(&format!("{NS}{FOCUS_CLASS}"));

    let mut conforming_names = Vec::with_capacity(conforming);
    let mut violating_names = Vec::with_capacity(violating);
    for (prefix, count, names) in [
        ('c', conforming, &mut conforming_names),
        ('v', violating, &mut violating_names),
    ] {
        for index in 0..count {
            let scope = format!("{prefix}{index}");
            let focus = builder.intern_iri(&format!("{NS}{scope}"));
            builder.push_quad(focus, rdf_type, focus_class, None);
            let mut emit = Emit {
                builder: &mut builder,
                focus,
                scope: scope.clone(),
                index,
            };
            (case.emit)(&mut emit, prefix == 'v');
            names.push(scope);
        }
    }

    let dataset = builder
        .freeze()
        .unwrap_or_else(|error| panic!("case {} must freeze: {error}", case.name));
    let shapes = parse_shapes(&format!("{PREFIXES}{}", case.shapes), None)
        .unwrap_or_else(|error| panic!("case {} shapes must parse: {error}", case.name));
    let validator = PreparedShapes::new(Arc::new(shapes))
        .bind_shared_dataset(Arc::clone(&dataset))
        .unwrap_or_else(|error| panic!("case {} must bind: {error}", case.name));

    let resolve = |names: &[String]| -> Vec<FocusId> {
        names
            .iter()
            .map(|scope| {
                validator
                    .term_id(&NamedNode::new_unchecked(format!("{NS}{scope}")).into_term())
                    .unwrap_or_else(|| panic!("case {} focus {scope} must be interned", case.name))
            })
            .collect()
    };
    let (conforming_ids, violating_ids) = (resolve(&conforming_names), resolve(&violating_names));
    CaseFixture {
        conforming: conforming_ids,
        violating: violating_ids,
        conforming_terms: conforming_names
            .iter()
            .map(|scope| NamedNode::new_unchecked(format!("{NS}{scope}")).into_term())
            .collect(),
        validator,
    }
}

impl CaseFixture {
    /// A focus set of `violations` violating ids followed by `conforming`
    /// conforming ones.
    ///
    /// The two populations are dimensioned INDEPENDENTLY rather than as a share
    /// of one total, and that is load-bearing. Swapping conforming nodes for
    /// violating ones to keep a total fixed changes two things at once, and the
    /// conforming and violating branches of a case do not cost the same: measured
    /// on this build, exchanging eight conforming `pattern` nodes for eight
    /// violating ones LOWERS the allocation count, because a name that fails the
    /// regex at its first character is cheaper to reject than a matching one is to
    /// accept. An assertion that "more violations cost more" written over that
    /// swap is simply false, and would have been softened into meaninglessness to
    /// make it pass. Holding the conforming population identical and ADDING
    /// violating nodes makes the difference attributable to the violations and
    /// nothing else.
    fn focus_set(&self, violations: usize, conforming: usize) -> Vec<FocusId> {
        let mut ids = Vec::with_capacity(violations + conforming);
        ids.extend_from_slice(&self.violating[..violations]);
        ids.extend_from_slice(&self.conforming[..conforming]);
        ids
    }
}

/// The shapes graph both once-per-snapshot seam tests use.
///
/// Deliberately more than one shape and more than one class: the bind test's
/// claim is that binding costs the CLASS CATALOG and not the graph, and a
/// single-class catalog would let a figure that was really "one class" pass for
/// "constant".
fn seam_shapes_source() -> String {
    format!(
        "{PREFIXES}
ex:FocusShape a sh:NodeShape ; sh:targetClass ex:Focus ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
    sh:property [ sh:path ex:count ; sh:datatype xsd:integer ] ;
    sh:property [ sh:path ex:ref ; sh:class ex:Target ] .

ex:TargetShape a sh:NodeShape ; sh:targetClass ex:Target ;
    sh:property [ sh:path ex:key ; sh:minCount 1 ] .
"
    )
}

/// One focus node in every `SEAM_VIOLATION_PERIOD` omits `ex:name`.
///
/// A seam fixture where everything conformed would let a binding or an admission
/// that produced a HUSK — a validator holding no shapes, a preparation that lost
/// its constraints — report exactly what a working one reports, and "cheap" and
/// "empty" would be the same measurement. Both seam tests check the violation
/// count outside their windows for that reason.
///
/// The period divides both seam sizes, so the violating population doubles with
/// the graph and the quad count still doubles exactly.
const SEAM_VIOLATION_PERIOD: usize = 512;

/// How many results a seam dataset of `focus_nodes` focus nodes must produce.
const fn seam_violations(focus_nodes: usize) -> usize {
    focus_nodes / SEAM_VIOLATION_PERIOD
}

/// How many classes the seam fixture's subclass hierarchy holds.
///
/// This IS "the catalog" the bind test's name excludes: it is the part of the
/// data graph that describes the schema rather than the instances, it is the same
/// size at both measured sizes, and binding is allowed to cost it.
const SEAM_HIERARCHY_CLASSES: usize = 8;

/// The seam fixture's fixed schema preamble, in quads.
const SEAM_HIERARCHY_QUADS: usize = SEAM_HIERARCHY_CLASSES;

/// A data graph of `SEAM_HIERARCHY_QUADS + 6 * focus_nodes -
/// seam_violations(focus_nodes)` quads.
///
/// Outside the fixed hierarchy nothing is shared between focus nodes — no shared
/// target, no shared literal — so doubling `focus_nodes` doubles the instance
/// data exactly, and the size-independence tests assert that rather than assume
/// it.
///
/// # Why there is a subclass hierarchy at all
///
/// The class-membership index is the most graph-proportional thing binding does,
/// and it short-circuits to nothing when the data graph interns no
/// `rdfs:subClassOf` — so a seam fixture with a flat type hierarchy never reaches
/// it and reports a constant bind cost by DODGING the work rather than by not
/// doing it. A fixture that passes because it avoided the expensive path is worse
/// than no fixture: it says the invariant holds when nobody checked. Measured on
/// this build, the flat variant binds in 6 allocations at every size, and the
/// variant below in tens — the whole question the test asks lives in the
/// difference.
///
/// The hierarchy is FIXED at [`SEAM_HIERARCHY_CLASSES`] while the instances scale,
/// which is what makes "independent of dataset size beyond the catalog" the
/// claim being tested. A fixture that gave every focus node its own class would
/// scale the schema along with the data and could then only report that binding
/// costs something per class, which nobody disputes.
fn seam_dataset(focus_nodes: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let subclass_of = builder.intern_iri(RDFS_SUBCLASS_OF);
    let focus_class = builder.intern_iri(&format!("{NS}{FOCUS_CLASS}"));
    let target_class = builder.intern_iri(&format!("{NS}Target"));
    let name = builder.intern_iri(&format!("{NS}name"));
    let count = builder.intern_iri(&format!("{NS}count"));
    let reference = builder.intern_iri(&format!("{NS}ref"));
    let key = builder.intern_iri(&format!("{NS}key"));

    let subclasses: Vec<_> = (0..SEAM_HIERARCHY_CLASSES)
        .map(|class| builder.intern_iri(&format!("{NS}seam-k{class}")))
        .collect();
    for &subclass in &subclasses {
        builder.push_quad(subclass, subclass_of, focus_class, None);
    }

    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{NS}seam-c{index}"));
        let target = builder.intern_iri(&format!("{NS}seam-t{index}"));
        let number = builder.intern_literal(RdfLiteral::typed(index.to_string(), XSD_INTEGER));
        let target_key = builder.intern_literal(RdfLiteral::simple(format!("key-{index}")));
        builder.push_quad(
            focus,
            rdf_type,
            subclasses[index % SEAM_HIERARCHY_CLASSES],
            None,
        );
        if index % SEAM_VIOLATION_PERIOD != 0 {
            let label = builder.intern_literal(RdfLiteral::simple(format!("item-{index}")));
            builder.push_quad(focus, name, label, None);
        }
        builder.push_quad(focus, count, number, None);
        builder.push_quad(focus, reference, target, None);
        builder.push_quad(target, rdf_type, target_class, None);
        builder.push_quad(target, key, target_key, None);
    }
    builder.freeze().expect("the seam dataset must freeze")
}

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

/// Refuse to report a change-path figure from a run where the parallel path
/// cannot be taken.
///
/// Both sizes under measurement are above the threshold at compile time, but a
/// single-threaded `rayon` pool keeps validation serial whatever the sizes are,
/// and a serial figure asserted under a parallel test name is exactly the
/// "green because it measured almost nothing" failure this file is built to
/// avoid. That is a property of the host, so it is checked at run time.
fn assert_parallel_path_is_reachable() {
    assert!(
        rayon::current_num_threads() > 1,
        "SHACL validation stays serial on a single-threaded rayon pool, so this host cannot \
         exercise the parallel change path these assertions are written about"
    );
}

/// Require a bound validator to actually validate the seam dataset it is bound
/// to, producing exactly the violations that dataset carries.
///
/// Both seam tests assert an allocation count is CONSTANT. A validator that bound
/// no shapes, or a preparation that was restored without its constraints, would
/// satisfy a constant just as well as a correct one — and more easily. This is
/// what separates the two, and it runs outside every measurement window so it
/// costs the figures nothing.
fn assert_bound_validator_is_live(validator: &PreparedValidator, focus_nodes: usize) {
    let expected = seam_violations(focus_nodes);
    let report = validator.validate().expect("the seam dataset validates");
    assert_eq!(
        report.results.len(),
        expected,
        "a seam dataset of {focus_nodes} focus nodes must produce {expected} results; {} means \
         the binding under measurement is not the binding the assertions describe",
        report.results.len()
    );
}

/// Which of the two change-path entry points a measurement drives.
///
/// There are two, they are documented as twins, and only one of them is named in
/// most of the discussion of this work — which is exactly the situation in which
/// one of them quietly keeps a cost the other sheds. Both are measured.
#[derive(Clone, Copy, Debug)]
enum ChangePath {
    /// [`PreparedValidator::validate_focus_node_ids`] — the id-native route.
    Ids,
    /// [`PreparedValidator::validate_focus_nodes`] — the term-keyed route.
    Terms,
}

impl ChangePath {
    /// The name that appears in an assertion message.
    const fn label(self) -> &'static str {
        match self {
            Self::Ids => "validate_focus_node_ids",
            Self::Terms => "validate_focus_nodes",
        }
    }
}

/// Validate the first `focus_nodes` conforming nodes of `fixture` through
/// `path`, requiring the report to conform.
///
/// Both argument slices are prepared by [`build_case`], so the only thing inside
/// a window built around this call is the validation itself.
fn validate_conforming(
    fixture: &CaseFixture,
    path: ChangePath,
    focus_nodes: usize,
    case: &str,
) -> ValidationReport {
    let report = match path {
        ChangePath::Ids => fixture
            .validator
            .validate_focus_node_ids(&fixture.conforming[..focus_nodes]),
        ChangePath::Terms => fixture
            .validator
            .validate_focus_nodes(&fixture.conforming_terms[..focus_nodes]),
    }
    .unwrap_or_else(|error| panic!("case {case}/{} must validate: {error}", path.label()));
    assert!(
        report.conforms,
        "case {case}/{}: the conforming population must conform, or the allocation figure below \
         describes a workload that never reached the constraint ({} result(s))",
        path.label(),
        report.results.len()
    );
    report
}

// ---------------------------------------------------------------------------
// 1. The headline
// ---------------------------------------------------------------------------

/// **Validating a conforming focus set costs the same whether it holds N nodes or
/// 2N.**
///
/// One dataset, one binding, two validations of the same production surface
/// differing only in how many conforming focus nodes they are handed. Equality is
/// asserted EXACTLY: an allocation count is a fact about the code — measured
/// eight times in a row on this build, the same validation reports the same
/// figure every time — so a change path with no per-focus-node allocation term
/// produces the same number twice, and any tolerance here would be a place for
/// exactly the term this test exists to forbid to hide.
///
/// Both change-path entry points are driven, and the failure message names which.
#[test]
fn conforming_change_path_allocation_has_no_growth_term_in_focus_count() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    for case in CASES {
        let fixture = build_case(case, 2 * FOCUS_NODES, 0);
        for path in [ChangePath::Ids, ChangePath::Terms] {
            // Warm-up, OUTSIDE every window and with the exact arguments measured
            // below: rayon's global pool, the regex cache, the class-membership
            // index and the SPARQL plan cache are all first-touch lazies, and
            // charging them to whichever measurement ran first would make the
            // comparison a statement about start-up rather than about focus-node
            // count.
            drop(validate_conforming(&fixture, path, FOCUS_NODES, case.name));
            drop(validate_conforming(
                &fixture,
                path,
                2 * FOCUS_NODES,
                case.name,
            ));

            let (half_report, half_measured) =
                measure_min(|| validate_conforming(&fixture, path, FOCUS_NODES, case.name));
            drop(half_report);
            let (full_report, full_measured) =
                measure_min(|| validate_conforming(&fixture, path, 2 * FOCUS_NODES, case.name));
            drop(full_report);

            // The conforming branch has run at both sizes and been required to
            // conform above, whatever happens below: an excluded case is excluded
            // from the COMPARISON, never from the work.
            if allocation_exclusion(case.name).is_some() {
                continue;
            }

            assert_eq!(
                half_measured.allocations,
                full_measured.allocations,
                "case {}/{}: the change path allocated {} for {FOCUS_NODES} conforming focus \
                 nodes and {} for {}, so its cost carries a growth term in the focus \
                 count\n  N  = {half_measured:?}\n  2N = {full_measured:?}",
                case.name,
                path.label(),
                half_measured.allocations,
                full_measured.allocations,
                2 * FOCUS_NODES,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. The companion that makes the headline mean anything
// ---------------------------------------------------------------------------

/// **Validating a violating focus set costs what the violations cost, not what
/// the focus count costs.**
///
/// [`conforming_change_path_allocation_has_no_growth_term_in_focus_count`] is
/// satisfied exactly as well by a validator that has stopped validating as by one
/// that got cheaper, so it cannot stand alone. This test drives every case's
/// VIOLATING branch and asserts three things:
///
/// 1. the results are really produced, and doubling the violating population
///    doubles them — a validator that quietly skipped a constraint fails here
///    first;
/// 2. holding the violation count fixed while doubling the CONFORMING population
///    does not move the allocation count, which is the same invariant as test 1
///    stated over a non-vacuous workload;
/// 3. holding the conforming population fixed while doubling the VIOLATING one
///    DOES move it, which is what says the remaining cost is the reporting work
///    and not a constant that stopped tracking anything.
///
/// The two populations are varied one at a time and never traded against each
/// other; [`CaseFixture::focus_set`] says why that distinction is not cosmetic.
#[test]
fn violating_change_path_allocation_scales_with_violations_not_focus_count() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    for case in CASES {
        let fixture = build_case(case, 2 * FOCUS_NODES, 2 * VIOLATIONS);
        let few_small = fixture.focus_set(VIOLATIONS, FOCUS_NODES);
        let few_large = fixture.focus_set(VIOLATIONS, 2 * FOCUS_NODES);
        let many_small = fixture.focus_set(2 * VIOLATIONS, FOCUS_NODES);

        let validate = |ids: &[FocusId]| {
            fixture
                .validator
                .validate_focus_node_ids(ids)
                .unwrap_or_else(|error| panic!("case {} must validate: {error}", case.name))
        };

        // Warm-up for all three regions, outside every window; see test 1.
        drop(validate(&few_small));
        drop(validate(&few_large));
        drop(validate(&many_small));

        let (few_small_report, few_small_measured) = measure_min(|| validate(&few_small));
        let (few_large_report, few_large_measured) = measure_min(|| validate(&few_large));
        let (many_small_report, many_small_measured) = measure_min(|| validate(&many_small));

        let few = few_small_report.results.len();
        let many = many_small_report.results.len();
        assert!(
            few > 0,
            "case {}: the violating population produced no results at all, so every allocation \
             figure in this file is describing a validator that is not validating",
            case.name
        );
        assert!(
            !few_small_report.conforms && !few_large_report.conforms,
            "case {}: a focus set containing {VIOLATIONS} violating nodes reported conformance",
            case.name
        );
        assert_eq!(
            few_large_report.results.len(),
            few,
            "case {}: adding conforming focus nodes changed the result count",
            case.name
        );
        assert_eq!(
            many,
            2 * few,
            "case {}: doubling the violating population must double the results ({few} -> {many})",
            case.name
        );

        // Every semantic assertion above applies to every case. Only the two
        // allocation comparisons below are skipped for an excluded one, and only
        // for the reason recorded beside its name.
        if allocation_exclusion(case.name).is_some() {
            continue;
        }

        assert_eq!(
            few_small_measured.allocations,
            few_large_measured.allocations,
            "case {}: with {VIOLATIONS} violations held fixed, the change path allocated {} for \
             {FOCUS_NODES} conforming focus nodes and {} for {}, so its cost still tracks the \
             focus count\n  N  = {few_small_measured:?}\n  2N = {few_large_measured:?}",
            case.name,
            few_small_measured.allocations,
            few_large_measured.allocations,
            2 * FOCUS_NODES,
        );
        assert!(
            many_small_measured.allocations > few_small_measured.allocations,
            "case {}: doubling the violations from {VIOLATIONS} to {} did not increase the \
             allocation count ({} -> {}), which means the reporting cost has stopped tracking \
             the results it is supposed to be building\n  V  = {few_small_measured:?}\n  2V = {many_small_measured:?}",
            case.name,
            2 * VIOLATIONS,
            few_small_measured.allocations,
            many_small_measured.allocations,
        );
    }
}

// ---------------------------------------------------------------------------
// 2b. What the one exclusion is, and is not, about
// ---------------------------------------------------------------------------

/// Focus nodes in the single-threaded `sh:pattern` companion.
///
/// Chosen so that BOTH this and its double stay at or below
/// [`PARALLEL_MIN_FOCUS_NODES`] — `should_parallelize` is a strict `>`, so 1,024
/// is still the serial path. That is the whole point of the fixture: on one
/// thread the `regex` crate's cache pool is never contended, so whatever slope is
/// measured here is the change path's and not the pool's.
const SERIAL_FOCUS_NODES: usize = 512;

/// Both measured sizes stay serial, checked when the file compiles.
const _: () = assert!(
    2 * SERIAL_FOCUS_NODES <= PARALLEL_MIN_FOCUS_NODES,
    "2n must not cross the parallel threshold, or this companion measures the contended pool it \
     exists to hold constant"
);

/// **`sh:pattern`'s change path has no growth term either — the excluded cost is
/// the `regex` crate's contended cache pool and nothing else.**
///
/// [`ALLOCATION_EXCLUSIONS`] holds `pattern` out of the two exact-equality
/// assertions. An exclusion with nothing behind it is indistinguishable from a
/// case quietly dropped because it failed, so this runs the SAME shape, over the
/// same fixture builder, through the same production entry point, at a size where
/// the excluded cause cannot be present — one thread, one pool user, no shard
/// contention — and asserts the slope is exactly zero.
///
/// Measured on this revision: 512 and 1,024 conforming `sh:pattern` focus nodes
/// each cost 5 allocations, in 200 out of 200 windows apiece, with no other value
/// observed. The regex pool costs nothing when it is not contended, which is what
/// makes the parallel figures attributable to it.
///
/// The non-vacuity check is the same one the rest of the file uses, and it
/// matters more here than anywhere: a `sh:pattern` that never ran would have a
/// beautifully flat slope.
#[test]
fn pattern_change_path_allocation_has_no_growth_term_below_the_parallel_threshold() {
    let _guard = measure_lock();

    let name = "pattern";
    let case = CASES
        .iter()
        .find(|case| case.name == name)
        .expect("the excluded case must exist; see every_allocation_exclusion_names_a_real_case");
    assert!(
        allocation_exclusion(name).is_some(),
        "this companion exists to justify an exclusion; if {name} is no longer excluded it should \
         be asserted exactly like every other case instead"
    );

    let fixture = build_case(case, 2 * SERIAL_FOCUS_NODES, VIOLATIONS);

    // Non-vacuity, outside every window: the regex really runs and really
    // rejects, so the flat slope below is about a pattern that is being matched.
    let violating = fixture.focus_set(VIOLATIONS, SERIAL_FOCUS_NODES);
    let report = fixture
        .validator
        .validate_focus_node_ids(&violating)
        .expect("the serial pattern fixture must validate");
    assert_eq!(
        report.results.len(),
        VIOLATIONS,
        "the serial pattern fixture must report its {VIOLATIONS} violations, or a flat allocation \
         slope is only saying that nothing was matched"
    );

    // Warm-up, outside every window, with the exact arguments measured below.
    drop(validate_conforming(
        &fixture,
        ChangePath::Ids,
        SERIAL_FOCUS_NODES,
        name,
    ));
    drop(validate_conforming(
        &fixture,
        ChangePath::Ids,
        2 * SERIAL_FOCUS_NODES,
        name,
    ));

    let (half_report, half_measured) =
        measure_min(|| validate_conforming(&fixture, ChangePath::Ids, SERIAL_FOCUS_NODES, name));
    drop(half_report);
    let (full_report, full_measured) = measure_min(|| {
        validate_conforming(&fixture, ChangePath::Ids, 2 * SERIAL_FOCUS_NODES, name)
    });
    drop(full_report);

    assert_eq!(
        half_measured.allocations,
        full_measured.allocations,
        "the serial sh:pattern change path allocated {} for {SERIAL_FOCUS_NODES} conforming focus \
         nodes and {} for {}. On one thread the regex cache pool is never contended, so this \
         difference is the change path's own and the exclusion in ALLOCATION_EXCLUSIONS no longer \
         describes what is being excluded\n  n  = {half_measured:?}\n  2n = {full_measured:?}",
        half_measured.allocations,
        full_measured.allocations,
        2 * SERIAL_FOCUS_NODES,
    );
}

// ---------------------------------------------------------------------------
// 2c. The expansion in front of the change path
// ---------------------------------------------------------------------------

/// Focus nodes in the change-expansion fixture.
///
/// Every focus node owns a target of its own, so a changed row on a target reaches
/// exactly one focus node and the expansion's size tracks the change rather than
/// the graph.
const EXPANSION_FOCUS_NODES: usize = 1_024;

/// The changed-row counts the expansion's closed form is checked at.
///
/// Powers of two, and four of them, because the form carries a `log2` term: two
/// points can be fitted by any line and would let a per-row term hide inside the
/// intercept. Every size is a real measurement taken on this revision.
const EXPANSION_CHANGE_SIZES: [usize; 4] = [32, 64, 128, 256];

/// Every measured change size fits inside the fixture's targets, checked when the
/// file compiles: a change that ran off the end would measure fewer rows than it
/// names.
const _: () = assert!(
    EXPANSION_CHANGE_SIZES[EXPANSION_CHANGE_SIZES.len() - 1] <= EXPANSION_FOCUS_NODES,
    "the largest measured change must still name a target of the expansion fixture"
);

/// The shapes graph the expansion fixture is bound with.
///
/// A SEQUENCE path, because that is what gives the footprint a trigger with a
/// non-empty CHAIN: the read of `ex:key` happens one `ex:ref` hop away from the
/// focus node, so a changed `(target, ex:key, …)` row has to be walked back along
/// `^ex:ref` to reach the focus node whose verdict it moves. A shapes graph of bare
/// predicate paths would leave every chain empty and would measure the one branch of
/// the expansion that evaluates no path at all — which is what
/// [`EXPANSION_EMPTY_CHAIN_SHAPES`] is for.
const EXPANSION_SHAPES: &str = "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
    sh:property [ sh:path ( ex:ref ex:key ) ; sh:minCount 1 ] .";

/// The same shapes graph with the sequence flattened to a bare predicate, so the
/// trigger that matches a changed `ex:key` row has an EMPTY chain and the expansion
/// walks no path.
///
/// The control for [`EXPANSION_PER_ROW`]: what a changed row costs when nothing
/// evaluates a path for it. See
/// [`change_expansion_with_no_chain_to_walk_costs_no_path_probe`].
const EXPANSION_EMPTY_CHAIN_SHAPES: &str = "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
    sh:property [ sh:path ex:key ; sh:minCount 1 ] .";

/// The expansion's fixed cost, independent of how many rows changed.
const EXPANSION_CONST: u64 = 2;

/// What ONE changed row costs the expansion once it has a chain to walk back.
///
/// TWO, and both are the projected view's probe rather than the walk: a
/// delta-backed [`ShaclDatasetView`] type-erases its probe iterator
/// (`crates/shapes/src/data_view.rs`, `raw_probe` and `raw_overlay_probe`), so each
/// pattern lookup costs one `Box` for the source rows and one for the RDF 1.2
/// overlay rows. The erasure is deliberate and documented where it is done — a
/// delta probe nests several indexed source alternatives, and keeping them generic
/// would grow the stack frame of every caller that composes probes, which a
/// recursive path evaluator does at every step. A NATIVE probe keeps its
/// allocation-free representation, which is why the whole-validation figures in
/// [`CASES`] have no per-focus-node term at all.
///
/// It used to be FOUR. Two of the four were the change expansion's own: it walked
/// each trigger's chain with the `Path`-driven evaluator, which built a `HashSet`
/// frontier table on its first insert — once per changed row, per trigger — and
/// rebuilt the reversed `Path` rather than reading the lowering beside it. The
/// expansion now runs the LOWERED evaluator the validation next to it runs, whose
/// frontier dedup answers from the accumulator it is already building.
const EXPANSION_PER_ROW: u64 = 2;

/// What DOUBLING the changed-row count costs in reallocation, on top of the terms
/// above.
///
/// Three collections on the expansion grow by doubling, because none of them can be
/// sized in advance: `DeltaDatasetView::changed_quads` is an iterator with no
/// length, so the change set the expansion maps into its own id space is built by
/// pushing, and the affected-id vector and its dedup set are bounded by the
/// expansion's own output — which a closure chain can make larger than the change,
/// so the change's size is a hint and not a bound. Three doubling series is `3` per
/// doubling, and [`EXPANSION_CHANGE_SIZES`] is four points precisely so this term
/// cannot be confused with a per-row one.
const EXPANSION_PER_DOUBLING: u64 = 3;

/// The expansion's measured cost for `changes` changed rows.
///
/// `EXPANSION_CONST + EXPANSION_PER_ROW * N + EXPANSION_PER_DOUBLING * log2(N)`.
/// Exact at every size in [`EXPANSION_CHANGE_SIZES`], with no tolerance: an
/// allocation count is a fact about the code.
fn expansion_alloc_model(changes: usize) -> u64 {
    let doublings = u64::from(changes.ilog2());
    EXPANSION_CONST + EXPANSION_PER_ROW * changes as u64 + EXPANSION_PER_DOUBLING * doublings
}

/// A delta-bound validator over [`EXPANSION_FOCUS_NODES`] focus nodes whose mutation
/// inserts `changes` rows, each of which reaches one focus node.
fn expansion_fixture(
    shapes: &str,
    changes: usize,
) -> (Arc<purrdf::ir::DeltaDatasetView>, PreparedValidator, usize) {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let focus_class = builder.intern_iri(&format!("{NS}{FOCUS_CLASS}"));
    let reference = builder.intern_iri(&format!("{NS}ref"));
    for index in 0..EXPANSION_FOCUS_NODES {
        let focus = builder.intern_iri(&format!("{NS}x{index}"));
        let target = builder.intern_iri(&format!("{NS}xt{index}"));
        builder.push_quad(focus, rdf_type, focus_class, None);
        builder.push_quad(focus, reference, target, None);
    }
    let base = builder.freeze().expect("the expansion base freezes");

    let mut mutation = purrdf::MutableDataset::new(base);
    for index in 0..changes {
        let row = purrdf::QuadValues {
            s: purrdf::TermValue::iri(format!("{NS}xt{index}")),
            p: purrdf::TermValue::iri(format!("{NS}key")),
            o: purrdf::TermValue::simple_literal(format!("k{index}")),
            g: None,
        };
        assert!(
            purrdf::DatasetMut::insert(&mut mutation, row).expect("the expansion insert applies"),
            "expansion row {index} changed nothing, so the measurement names more rows than it \
             makes"
        );
    }
    let snapshot = Arc::new(mutation.snapshot_view().expect("the expansion snapshots"));
    let validator = PreparedShapes::new(Arc::new(
        parse_shapes(&format!("{PREFIXES}{shapes}"), None).expect("the expansion shapes parse"),
    ))
    .bind_delta_with_shapes_graph(
        Arc::clone(&snapshot),
        None,
        purrdf::ir::ViewLimits::default(),
    )
    .expect("the expansion delta binds");
    (snapshot, validator, changes)
}

/// One expansion fixture: its snapshot, its validator, and the row count it was
/// built with.
type ExpansionFixture = (Arc<purrdf::ir::DeltaDatasetView>, PreparedValidator, usize);

/// Expand `fixture`'s change, requiring the answer to be bounded and to name
/// `expected` focus nodes.
///
/// The count check is the non-vacuity guard, and it is what an allocation figure
/// alone cannot give: an expansion that stopped walking the chain would report the
/// changed rows' own subjects — the TARGETS, not the focus nodes — and would be
/// both cheaper and silently unsound.
fn expand(fixture: &ExpansionFixture, expected: usize) -> usize {
    let (snapshot, validator, changes) = fixture;
    let expansion = validator
        .affected_focus_node_ids(snapshot)
        .expect("the expansion succeeds");
    let ids = expansion.ids().unwrap_or_else(|| {
        panic!(
            "the expansion must be bounded, not TOP ({:?})",
            expansion.reason()
        )
    });
    assert_eq!(
        ids.len(),
        expected,
        "a change of {changes} rows must expand to {expected} focus node(s); {} means the \
         expansion is not walking the chain this fixture was built around",
        ids.len()
    );
    ids.len()
}

/// **Expanding a change costs `2 + 2N + 3·log2(N)` allocations for `N` changed
/// rows — and the `2N` is the delta view's type-erased probe, not the walk.**
///
/// `affected_focus_node_ids` is the surface the incremental soundness claim rests
/// on, and until this test it was the one surface on the change path with no
/// allocation coverage at all. That is how it came to walk every trigger's chain
/// with the `Path`-driven evaluator — rebuilding the reversed path per trigger and
/// building a `HashSet` frontier table per changed row — while the validation
/// beside it ran the lowered one. Measured on this revision before that was fixed,
/// the same fixture cost `4 + 4N + 3·log2(N)`: 278 allocations for 64 changed rows
/// and 537 for 128, against 148 and 279 now.
///
/// A closed form rather than a flat `alloc(2N) == alloc(N)`, for the reason
/// `tests/sparql_path_alloc.rs` states for its three surfaces: the residual terms
/// are real, they are named and attributed above, and asserting they are absent
/// would be asserting something false. Nothing here carries a tolerance — the form
/// is exact at all four sizes.
///
/// The graph is held FIXED while the change doubles, so every term measured is a
/// term in the CHANGE.
///
/// [`ShaclDatasetView`]: ../src/data_view.rs
#[test]
fn change_expansion_allocation_matches_its_pinned_closed_form() {
    let _guard = measure_lock();

    for changes in EXPANSION_CHANGE_SIZES {
        let fixture = expansion_fixture(EXPANSION_SHAPES, changes);
        // Warm-up, outside the window and with the exact arguments measured below.
        assert_eq!(expand(&fixture, changes), changes);

        let (_, measured) = measure_min(|| expand(&fixture, changes));
        assert_eq!(
            measured.allocations,
            expansion_alloc_model(changes),
            "expanding {changes} changed rows allocated {}, and the pinned closed form \
             ({EXPANSION_CONST} + {EXPANSION_PER_ROW}N + {EXPANSION_PER_DOUBLING}·log2 N) says \
             {}. A HIGHER per-row coefficient means the expansion has gone back to walking a \
             chain it rebuilds and re-resolves per changed row; a lower one is welcome and should \
             move these constants\n  {measured:?}",
            measured.allocations,
            expansion_alloc_model(changes),
        );
    }
}

/// **A trigger with no chain to walk back charges a changed row nothing.**
///
/// The control for [`EXPANSION_PER_ROW`]. Its two allocations are attributed to the
/// delta view's type-erased pattern probe rather than to the expansion's walk, and
/// that attribution is only worth anything if it can be turned off: this drives the
/// same graph, the same change and the same entry point through a shapes graph whose
/// matching trigger is anchored AT the changed row's subject, so the expansion takes
/// the branch that performs no pattern lookup at all.
///
/// Measured on this revision the per-row term is then exactly zero — 16 allocations
/// for 32 changed rows and 25 for 256, which is `1 + 3·log2(N)`, the three doubling
/// series and nothing else. So the `2N` above really is one probe per row, and a
/// per-row term that ever appeared HERE would be the expansion's own.
#[test]
fn change_expansion_with_no_chain_to_walk_costs_no_path_probe() {
    let _guard = measure_lock();

    let mut measured = Vec::with_capacity(EXPANSION_CHANGE_SIZES.len());
    for changes in EXPANSION_CHANGE_SIZES {
        let fixture = expansion_fixture(EXPANSION_EMPTY_CHAIN_SHAPES, changes);
        assert_eq!(expand(&fixture, changes), changes);
        let (_, sample) = measure_min(|| expand(&fixture, changes));
        measured.push((changes, sample));
    }

    for (changes, sample) in &measured {
        let doublings = u64::from(changes.ilog2());
        assert_eq!(
            sample.allocations,
            1 + EXPANSION_PER_DOUBLING * doublings,
            "expanding {changes} changed rows through an EMPTY chain allocated {}, not the \
             1 + {EXPANSION_PER_DOUBLING}·log2 N this control is pinned at. A per-row term here \
             is the expansion's own, and it would mean the {EXPANSION_PER_ROW} charged per row \
             in change_expansion_allocation_matches_its_pinned_closed_form is no longer the \
             probe it is attributed to\n  {sample:?}",
            sample.allocations,
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The silent-drop guard
// ---------------------------------------------------------------------------

/// **The change path's report text is pinned, byte for byte, for every constraint
/// kind and path form.**
///
/// See the NO-REBLESSING RULE in this module's documentation. It exists so that
/// "the change path got cheaper" can never be satisfied by "the change path
/// stopped saying anything".
///
/// It takes no measurement of its own and still takes [`MEASURE_LOCK`], for the
/// other half of that lock's job. [`WholeProcessWindow`] counts EVERY thread, and
/// `cargo test` runs this binary's test functions concurrently, so this test's
/// own allocation traffic would otherwise land inside a sibling's window and be
/// reported as the change path's cost. Serializing it out is not a tolerance: it
/// removes traffic that provably is not the measured region's, and it makes the
/// figures the other tests print smaller and more stable rather than more
/// permissive.
#[test]
fn change_path_report_bytes_match_pinned_golden() {
    let _guard = measure_lock();
    let rendered = render_golden();
    assert_eq!(
        rendered, GOLDEN,
        "the SHACL change path's report text changed. This is a SEMANTIC CHANGE, not a golden \
         that needs refreshing: diagnose what moved. Regenerating \
         fixtures/change-path-report.golden.txt is forbidden."
    );
}

/// Render every case's change-path report into the golden's exact text.
fn render_golden() -> String {
    let mut rendered = String::with_capacity(64 * 1024);
    for case in CASES {
        let fixture = build_case(case, GOLDEN_CONFORMING, GOLDEN_VIOLATING);
        let ids = fixture.focus_set(GOLDEN_VIOLATING, GOLDEN_CONFORMING);
        let report = fixture
            .validator
            .validate_focus_node_ids(&ids)
            .unwrap_or_else(|error| panic!("case {} must validate: {error}", case.name));
        assert!(
            !report.conforms && !report.results.is_empty(),
            "case {}: the golden fixture must produce results, or it pins nothing",
            case.name
        );
        writeln!(rendered, "### case {}", case.name).expect("writing to a String cannot fail");
        writeln!(rendered, "### results {}", report.results.len())
            .expect("writing to a String cannot fail");
        rendered.push_str(&report.to_ntriples());
        rendered.push('\n');
    }

    // The term-keyed change path is the same validation reached through a
    // different key space, so it belongs in the same pin: a refactor that moved
    // the two apart would otherwise only be caught by whichever one had a test.
    let case = &CASES[0];
    let fixture = build_case(case, GOLDEN_CONFORMING, GOLDEN_VIOLATING);
    let terms: Vec<_> = (0..GOLDEN_VIOLATING)
        .map(|index| NamedNode::new_unchecked(format!("{NS}v{index}")).into_term())
        .collect();
    let report = fixture
        .validator
        .validate_focus_nodes(&terms)
        .expect("the term-keyed change path must validate");
    writeln!(rendered, "### case {}/validate_focus_nodes", case.name)
        .expect("writing to a String cannot fail");
    writeln!(rendered, "### results {}", report.results.len())
        .expect("writing to a String cannot fail");
    rendered.push_str(&report.to_ntriples());
    rendered.push('\n');
    rendered
}

// ---------------------------------------------------------------------------
// 4 and 5. The once-per-snapshot seams
// ---------------------------------------------------------------------------

/// **Binding a dataset costs the class catalog, not the graph.**
///
/// The change path's whole value is that a caller binds once and then validates
/// small focus sets many times. A bind that is linear in the graph moves the cost
/// rather than removing it, and it does so invisibly, because the bind happens
/// once and nobody times it.
///
/// [`BIND_ALLOC_CONST`] pins the constant term. It is a DETERMINISM pin, not a
/// timing threshold: an allocation count is a fact about the code, reproducible on
/// any host at any core count, so asserting it costs nothing in flakiness and buys
/// a build failure the day binding starts allocating per triple.
#[test]
fn bind_allocation_is_independent_of_dataset_size_beyond_the_catalog() {
    let _guard = measure_lock();

    let shapes = Arc::new(parse_shapes(&seam_shapes_source(), None).expect("seam shapes parse"));
    let prepared = PreparedShapes::new(shapes);
    let small = seam_dataset(SEAM_FOCUS_NODES);
    let large = seam_dataset(2 * SEAM_FOCUS_NODES);
    assert_eq!(
        large.quad_count() - SEAM_HIERARCHY_QUADS,
        2 * (small.quad_count() - SEAM_HIERARCHY_QUADS),
        "the two seam datasets must hold exactly twice the instance data, or 'independent of \
         dataset size' is being asserted over sizes that are not what the names say. The fixed \
         {SEAM_HIERARCHY_QUADS}-quad hierarchy is excluded because it IS the catalog this test's \
         name sets aside: it is identical in both, so it cannot be what a difference measures."
    );

    // Warm-up, outside every window: first-touch lazies and allocator arenas.
    // Deliberately over DIFFERENT dataset instances of the same two sizes. Warming
    // with the very snapshots about to be measured would leave any per-snapshot
    // memoization already populated, and the test would then be reporting a cache
    // hit twice instead of what binding a fresh snapshot costs.
    drop(prepared.bind_shared_dataset(seam_dataset(SEAM_FOCUS_NODES)));
    drop(prepared.bind_shared_dataset(seam_dataset(2 * SEAM_FOCUS_NODES)));

    let (small_bound, small_measured) =
        measure(|| prepared.bind_shared_dataset(Arc::clone(&small)));
    let small_bound = small_bound.expect("the small seam dataset binds");
    let (large_bound, large_measured) =
        measure(|| prepared.bind_shared_dataset(Arc::clone(&large)));
    let large_bound = large_bound.expect("the large seam dataset binds");

    // Non-vacuity, outside every window: a bind that produced a husk would be
    // beautifully constant too.
    assert_bound_validator_is_live(&small_bound, SEAM_FOCUS_NODES);
    assert_bound_validator_is_live(&large_bound, 2 * SEAM_FOCUS_NODES);

    assert_eq!(
        small_measured.allocations, BIND_ALLOC_CONST,
        "bind's pinned figure moved from {BIND_ALLOC_CONST} to {}; this is a determinism pin, so \
         the question is what changed in binding, not what the number should be now",
        small_measured.allocations,
    );
    assert_eq!(
        small_measured.allocations,
        large_measured.allocations,
        "bind allocated {} for {} quads and {} for {}, so binding carries a term in the instance \
         count\n  M  = {small_measured:?}\n  2M = {large_measured:?}",
        small_measured.allocations,
        small.quad_count(),
        large_measured.allocations,
        large.quad_count(),
    );
}

/// **Admitting a prepared product costs the product, not the dataset.**
///
/// `admit` takes no dataset — it restores a preparation from carried bytes — so
/// this reads as a tautology, and that is exactly what it is pinning. The seam is
/// one refactor away from folding dataset-derived state (a resolved class table, a
/// membership index) into admission, at which point the once-per-preparation cost
/// silently becomes a once-per-snapshot cost and the product stops being the cheap
/// path it is sold as. The measurement is therefore taken with a bound validator
/// over each dataset size held live, so a figure that started depending on the
/// data in scope would move.
#[test]
fn admit_allocation_is_independent_of_dataset_size() {
    let _guard = measure_lock();

    let shapes = Arc::new(parse_shapes(&seam_shapes_source(), None).expect("seam shapes parse"));
    let prepared = PreparedShapes::new(shapes);
    let product = prepared
        .to_product(&ShapesProfile::CORE)
        .expect("the seam shapes graph encodes as a product");
    let host = HostBindings::empty();

    let mut measured = Vec::with_capacity(2);
    for focus_nodes in [SEAM_FOCUS_NODES, 2 * SEAM_FOCUS_NODES] {
        let data = seam_dataset(focus_nodes);
        let bound = prepared
            .bind_shared_dataset(Arc::clone(&data))
            .expect("the seam dataset binds");
        // Warm-up, outside the window. `admit` consumes its view, so the warm-up
        // and the measured call each open their own; both opens stay outside the
        // window, so the figure is admission and not admission plus the
        // structural tier.
        let warm = ShapesProduct::open(&product).expect("the product opens");
        drop(warm.admit(&ShapesProfile::CORE, &host));

        let view = ShapesProduct::open(&product).expect("the product opens");
        let (admitted, sample) = measure(|| view.admit(&ShapesProfile::CORE, &host));
        let admitted = admitted.expect("the product admits");
        measured.push(sample);

        // Non-vacuity, outside the window: the admitted preparation has to be able
        // to do the job the original one does, or a constant admission cost is
        // just the cost of restoring nothing.
        let restored = admitted
            .bind_shared_dataset(Arc::clone(&data))
            .expect("the admitted preparation binds");
        assert_bound_validator_is_live(&restored, focus_nodes);
        drop(bound);
    }

    let (small, large) = (measured[0], measured[1]);
    assert_eq!(
        small.allocations,
        large.allocations,
        "admit allocated {} with a {SEAM_FOCUS_NODES}-focus-node snapshot bound and {} with a \
         {}-focus-node one, so admission has started depending on the data\n  M  = {small:?}\n  2M = {large:?}",
        small.allocations,
        large.allocations,
        2 * SEAM_FOCUS_NODES,
    );
    assert_eq!(
        small.allocations, ADMIT_ALLOC_CONST,
        "admit's constant term moved from {ADMIT_ALLOC_CONST} to {}; this is a determinism pin, so \
         the question is what changed in admission, not what the number should be now",
        small.allocations,
    );
}

// ---------------------------------------------------------------------------
// 6. The regression guard: every test in this binary holds MEASURE_LOCK first
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
    let source = include_str!("change_path_alloc.rs");
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

// ---------------------------------------------------------------------------
// 7. The coverage guard: every constraint kind has an allocation case
// ---------------------------------------------------------------------------

/// Every variant identifier declared on `enum PlannedConstraint` in
/// `crates/shapes/src/plan.rs`, in source order.
///
/// [`PlannedConstraint`] is `pub(crate)` — deliberately: widening it just so an
/// integration test could `match` over it would be the wrong trade. So this
/// test cannot import the type at all; the only way it can enumerate the
/// variants is to read the declaration itself, exactly as the measure-lock
/// guard above reads this file's own source and `product_model_census.rs`
/// reads the crate's model types: a source scan with `syn`, already a
/// dev-dependency of this crate for that reason.
///
/// [`PlannedConstraint`]: ../src/plan.rs
#[derive(Default)]
struct PlannedConstraintVariants(Vec<String>);

impl<'ast> syn::visit::Visit<'ast> for PlannedConstraintVariants {
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        if item.ident == "PlannedConstraint" {
            self.0 = item
                .variants
                .iter()
                .map(|variant| variant.ident.to_string())
                .collect();
        }
        syn::visit::visit_item_enum(self, item);
    }
}

/// Convert a `PascalCase` variant identifier to the `snake_case` kind name this
/// file's [`CASES`] are named with (e.g. `NodeKind` -> `node_kind`).
///
/// Every `PlannedConstraint` variant identifier is plain ASCII words with no
/// adjacent capitals (no acronyms), so "insert `_` before an interior capital,
/// then lowercase everything" is exact for the whole enum; it is not offered as
/// a general-purpose converter.
fn to_kind_name(variant: &str) -> String {
    let mut out = String::with_capacity(variant.len() + 4);
    for (index, ch) in variant.char_indices() {
        if ch.is_ascii_uppercase() {
            if index != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// The two RDF 1.2 constraint surfaces `PropertyShape` carries OUTSIDE
/// `PlannedConstraint`: `sh:reifierShape` and `sh:reificationRequired`
/// (`crates/shapes/src/shapes.rs` fields `reifier_shapes` and
/// `reification_required`). Both produce violations
/// (`crates/shapes/src/constraints.rs::eval_reifier_shapes`), and neither is
/// ever wrapped in a `PlannedConstraint` variant — `plan.rs` reads
/// `property.reifier_shapes` straight off the property shape — so a scan of
/// the enum alone would miss both. They are added to the checked kind set by
/// name here instead.
const REIFICATION_KIND_NAMES: &[&str] = &["reifier_shapes", "reification_required"];

/// Every kind name this coverage guard requires a case (or an accounted-for
/// absence) for: every `PlannedConstraint` variant, snake-cased, plus the two
/// reification kinds above.
fn all_constraint_kind_names() -> Vec<String> {
    let source = include_str!("../src/plan.rs");
    let parsed = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("plan.rs must parse as Rust: {error}"));
    let mut collector = PlannedConstraintVariants::default();
    syn::visit::Visit::visit_file(&mut collector, &parsed);
    assert!(
        !collector.0.is_empty(),
        "the scan found no variants on `enum PlannedConstraint` in plan.rs, so this guard is \
         reading nothing"
    );

    let mut names: Vec<String> = collector
        .0
        .iter()
        .map(|variant| to_kind_name(variant))
        .collect();
    names.extend(REIFICATION_KIND_NAMES.iter().map(|name| (*name).to_owned()));
    names
}

/// Kinds whose mechanical `snake_case` name (the direct conversion of the
/// `PlannedConstraint` variant name above) matches no single [`CASES`] entry,
/// even though the kind genuinely IS exercised there — split, across entries
/// named for the sub-constraints it composes, because those names are more
/// informative than the kind's own. Renaming the existing entries to match is
/// out of scope here (an existing `CASES` entry must not change), so the
/// mapping is recorded instead.
///
/// `sh:qualifiedValueShape` is the one case: `PlannedConstraint::QualifiedValueShape`
/// carries the shape, its siblings, both counts and the disjointness flag as
/// ONE constraint, but `CASES` exercises it through two scenarios named for
/// which count it violates.
const KIND_NAME_ALIASES: &[(&str, &[&str])] = &[(
    "qualified_value_shape",
    &["qualified_min_count", "qualified_max_count"],
)];

/// One [`SIBLING_FILE_COVERAGE`] entry: a kind, the sibling file that measures
/// it (repository-relative to this crate, i.e. relative to `CARGO_MANIFEST_DIR`),
/// the exact `CASES` name(s) there that measure it, and the reason a `CASES`
/// entry HERE would be the wrong place for it.
///
/// The file and case names are DATA, not prose, precisely so
/// [`every_sibling_file_coverage_entry_names_a_real_case`] can check them
/// against the sibling file's actual source rather than trusting a sentence
/// that nothing re-reads.
type SiblingCoverageEntry = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
);

/// Kinds validated for allocation behaviour in a SIBLING allocation-measuring
/// file rather than in THIS file's [`CASES`].
///
/// `tests/sparql_path_alloc.rs`'s own module documentation states plainly why
/// these three do not belong among `CASES`: `sh:sparql`, a SHACL-SPARQL custom
/// constraint component, and a SHACL-AF `sh:expression` function call each
/// charge a focus node a real, nonzero marginal allocation cost per query
/// evaluation or per expression tuple TODAY, so the `delta(2N) == delta(N)`
/// claim `CASES` exists to pin does NOT hold for them yet — asserting it here
/// would be asserting something false. The true, closed-form claim
/// (`allocations(N) == CONSTANT + per_focus_node * N`) is pinned in that file
/// instead.
///
/// This is a recorded COST, not a permanent exemption: the per-focus-node
/// charge on these three surfaces is itself a defect this same effort means to
/// eliminate. When it is, these entries should move OUT of
/// `SIBLING_FILE_COVERAGE` and become genuine zero-growth entries in `CASES`
/// below — at which point [`every_constraint_kind_is_covered_by_an_allocation_case`]
/// stays green through the move, having lost nothing it was checking before.
const SIBLING_FILE_COVERAGE: &[SiblingCoverageEntry] = &[
    (
        "sparql",
        "tests/sparql_path_alloc.rs",
        &["sh:sparql"],
        "a SHACL-SPARQL SELECT constraint on a node shape runs one query per focus node, pinned \
         there in closed form (a constant plus a per-focus-node marginal cost), not as a \
         zero-growth case — see that file's module documentation for why the zero-growth claim \
         does not hold for it today.",
    ),
    (
        "component",
        "tests/sparql_path_alloc.rs",
        &["sh:ask component", "sh:select component"],
        "a custom SHACL-SPARQL constraint component runs a query per value node (ASK) or per \
         focus node (SELECT), pinned there in closed form, not as a zero-growth case — see that \
         file's module documentation for why the zero-growth claim does not hold for it today.",
    ),
    (
        "expression",
        "tests/sparql_path_alloc.rs",
        &["sh:expression call"],
        "a SHACL-AF node-expression function call routes through the scalar-expression seam once \
         per tuple of its argument value-sets, pinned there in closed form, not as a zero-growth \
         case — see that file's module documentation for why the zero-growth claim does not hold \
         for it today.",
    ),
];

/// Every `name:` string literal found inside a `const CASES` item, in source
/// order.
///
/// Scoped to the `CASES` declaration specifically, via [`CasesConstFinder`],
/// rather than scanning the whole file for any struct literal with a `name`
/// field: a sibling file's `CASES` names the measured cases, and nothing else
/// in it should be mistaken for one.
#[derive(Default)]
struct CaseNamesInExpr(Vec<String>);

impl<'ast> syn::visit::Visit<'ast> for CaseNamesInExpr {
    fn visit_expr_struct(&mut self, item: &'ast syn::ExprStruct) {
        for field in &item.fields {
            if let syn::Member::Named(ident) = &field.member
                && ident == "name"
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(literal),
                    ..
                }) = &field.expr
            {
                self.0.push(literal.value());
            }
        }
        syn::visit::visit_expr_struct(self, item);
    }
}

/// Locates `const CASES` in a parsed file and collects the `name:` literals of
/// every struct literal inside its initializer.
#[derive(Default)]
struct CasesConstFinder(Vec<String>);

impl<'ast> syn::visit::Visit<'ast> for CasesConstFinder {
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if item.ident == "CASES" {
            let mut names = CaseNamesInExpr::default();
            syn::visit::Visit::visit_expr(&mut names, &item.expr);
            self.0 = names.0;
        }
        syn::visit::visit_item_const(self, item);
    }
}

/// Every `name:` string literal `const CASES` carries in the file at
/// `CARGO_MANIFEST_DIR`-relative `path`.
///
/// # Panics
///
/// Panics — naming `path` and the underlying error — when `path` does not read
/// as a file or does not parse as Rust: a typo'd sibling-file path must fail
/// loudly here rather than silently matching nothing.
fn sibling_case_names(path: &str) -> Vec<String> {
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let source = std::fs::read_to_string(&full_path).unwrap_or_else(|error| {
        panic!(
            "SIBLING_FILE_COVERAGE names {path:?}, which does not read as a file at {}: {error}",
            full_path.display()
        )
    });
    let parsed = syn::parse_file(&source).unwrap_or_else(|error| {
        panic!("SIBLING_FILE_COVERAGE's {path:?} must parse as Rust: {error}")
    });
    let mut finder = CasesConstFinder::default();
    syn::visit::Visit::visit_file(&mut finder, &parsed);
    finder.0
}

/// **Every [`SIBLING_FILE_COVERAGE`] entry names a file that exists, parses,
/// and really carries every case it claims.**
///
/// [`every_allocation_exclusion_names_a_real_case`] is the analogous check for
/// `ALLOCATION_EXCLUSIONS`, but it can lean on the compiler: `CASES` is right
/// there in the same translation unit, so a renamed or deleted case is already
/// a compile error via `case.name`. A SIBLING file's `CASES` is NOT in this
/// translation unit, so nothing else would notice a case renamed or deleted
/// there, or a path typo'd here: the coverage hatch would keep silently
/// satisfying [`every_constraint_kind_is_covered_by_an_allocation_case`] while
/// the kind it claims to cover quietly went unmeasured — exactly the failure
/// shape this whole guard exists to rule out.
#[test]
fn every_sibling_file_coverage_entry_names_a_real_case() {
    let _guard = measure_lock();
    for (kind, path, case_names, reason) in SIBLING_FILE_COVERAGE {
        assert!(
            !reason.trim().is_empty(),
            "SIBLING_FILE_COVERAGE entry {kind:?} carries no stated reason"
        );
        let found = sibling_case_names(path);
        assert!(
            !found.is_empty(),
            "SIBLING_FILE_COVERAGE entry {kind:?} names {path:?}, but no `const CASES` with any \
             `name:` field was found there"
        );
        let mut missing: Vec<&str> = case_names
            .iter()
            .copied()
            .filter(|case_name| !found.iter().any(|name| name == case_name))
            .collect();
        missing.sort_unstable();
        assert!(
            missing.is_empty(),
            "SIBLING_FILE_COVERAGE entry {kind:?} claims case(s) {missing:?} in {path:?}, but that \
             file's CASES carries no entry with that name"
        );
    }
}

/// Every kind name this file (together with its documented aliases and its
/// documented, source-verified sibling-file coverage) accounts for, by ANY of:
/// a [`CASES`] entry, an [`ALLOCATION_EXCLUSIONS`] entry, a
/// [`KIND_NAME_ALIASES`] entry whose target names ARE a `CASES` entry, or a
/// [`SIBLING_FILE_COVERAGE`] entry.
fn covered_kind_names() -> std::collections::HashSet<String> {
    let mut covered: std::collections::HashSet<String> =
        CASES.iter().map(|case| case.name.to_owned()).collect();
    covered.extend(
        ALLOCATION_EXCLUSIONS
            .iter()
            .map(|(name, _)| (*name).to_owned()),
    );
    covered.extend(
        SIBLING_FILE_COVERAGE
            .iter()
            .map(|(kind, ..)| (*kind).to_owned()),
    );
    for (kind, aliases) in KIND_NAME_ALIASES {
        if aliases.iter().any(|alias| covered.contains(*alias)) {
            covered.insert((*kind).to_owned());
        }
    }
    covered
}

/// **Every SHACL constraint kind `PlannedConstraint` can express, plus the two
/// RDF 1.2 reification surfaces `PropertyShape` carries outside it, is covered
/// by an allocation case** — this file's [`CASES`], its
/// [`ALLOCATION_EXCLUSIONS`], or the documented sibling-file coverage above.
///
/// This is the totality half of the guard the per-variant allocation audit
/// promised: the compiler-enforced match in `plan.rs` (`kind_name`,
/// `#[cfg(test)] mod tests`) makes it impossible to add a `PlannedConstraint`
/// variant without naming it; this test makes it impossible for a named kind
/// to carry no allocation coverage anywhere without that absence being stated
/// and argued, rather than merely never noticed.
///
/// # The three kinds this test used to be `#[ignore]`d for
///
/// It ran red, and was held out, because three kinds were covered NOWHERE in
/// any allocation-measuring file in this crate (`change_path_alloc.rs`,
/// `sparql_path_alloc.rs`, `box_role_alloc.rs`). All three were FIXED rather
/// than excluded, and each is now an ordinary [`CASES`] entry asserted by exact
/// equality and pinned byte for byte by the golden:
///
/// * the two RDF 1.2 reification surfaces (`reification_required` and
///   `reifier_shapes`). Driving `sh:reificationRequired true` over a value
///   triple present on both branches cost `6 + 18 * focus_nodes` allocations —
///   36,870 at 2,048 conforming focus nodes and 73,734 at 4,096 — a first-party
///   growth term contributed by the reification arm of `eval_property_shape`
///   (`crates/shapes/src/constraints.rs`), which materialized each value node
///   and the focus node as owned terms, built the quoted triple term, and then
///   answered an EXISTENCE question by building a deduplicated, canonically
///   sorted vector of owned reifier terms. That arm is now id-native end to end
///   and the slope is exactly zero;
/// * `node_by_expression` — `sh:nodeByExpression`, which selects the shape to
///   validate a value node against by EVALUATING a node expression, per value
///   node. It cost `6 + 6 * focus_nodes`: 12,294 at 2,048 conforming focus
///   nodes and 24,582 at 4,096. Three of the six were a `String` rendered
///   purely to key the shape index, which is now keyed by the shape's identity
///   TERM; the other three were the owned focus term and the owned result
///   vector the general node-expression evaluator needs, which a production
///   that stays in identity space no longer builds. The slope is exactly zero.
///
/// `box_role_alloc.rs` DOES drive a shape with `sh:reificationRequired true`
/// (case `reifier_cbox`), but only to pin the graph-box-role VECTOR the
/// reifier path adds; it asserts nothing about the change path's allocation
/// behaviour for that constraint, so it does not count as coverage here.
///
/// With all three closed there is no kind left unmeasured, so this test is
/// ACTIVE — and being active is the point of it: it is what makes "every
/// constraint kind carries an allocation measurement" enforceable from here on,
/// rather than a claim someone has to remember to re-check. A future kind that
/// arrives without one fails HERE, by name.
#[test]
fn every_constraint_kind_is_covered_by_an_allocation_case() {
    let _guard = measure_lock();
    let covered = covered_kind_names();
    let mut uncovered: Vec<String> = all_constraint_kind_names()
        .into_iter()
        .filter(|name| !covered.contains(name))
        .collect();
    uncovered.sort_unstable();
    uncovered.dedup();
    assert!(
        uncovered.is_empty(),
        "constraint kinds with no allocation coverage anywhere in this crate: {uncovered:?}"
    );
}
