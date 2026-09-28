// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **Configuring a graph-box role vocabulary costs a CONFORMING focus node
//! nothing, and costs a VIOLATING one exactly the roles it reports.**
//!
//! Graph-box roles are report-only provenance: three `Vec<NamedNode>` stamped
//! onto a [`ValidationResult`]. Nothing in validation reads them, so a focus node
//! that produces no result has no use for them — yet computing them is not free.
//! `path_box_roles` scans the data graph for the path predicate's role
//! annotations, materializing owned `(Term, NamedNode, Term)` triples and then
//! sorting and deduping them, and the source roles merge two role slices into a
//! fresh sorted vector. Both once per focus node, per property shape.
//!
//! The vocabulary is CALLER-SUPPLIED — PurRDF mints no vocabulary IRIs — so with
//! none configured the feature is inactive and both computations short-circuit
//! before they touch the allocator. That is the configuration
//! `tests/change_path_alloc.rs` measures, and in it this work is invisible: there
//! is none to do. The cost lives entirely in the configuration a caller who
//! actually wants box-role provenance runs in, which is the configuration this
//! file measures.
//!
//! # What each test pins
//!
//! * [`a_box_role_vocabulary_adds_no_allocation_to_the_conforming_path`] — the
//!   headline, stated DIFFERENTIALLY. One data graph, one shapes text, two
//!   bindings that differ in exactly one bit: whether a [`BoxRoleVocab`] is
//!   configured. Validating the same conforming focus set through both must
//!   allocate the same amount at `N` and at `2N`, so the vocabulary contributes
//!   neither a constant nor a growth term to a conforming focus node.
//! * [`conforming_box_role_validation_has_no_growth_term_in_focus_count`] — the
//!   same claim stated ABSOLUTELY, `delta(2N) == delta(N)`. It was red, and it
//!   was red for a reason that had nothing to do with box roles: the change path
//!   materialized every focus node before any constraint ran. That term is gone,
//!   so this now holds outright rather than only as a difference. Both statements
//!   ship, because the differential one keeps saying what the absolute one
//!   cannot: that the vocabulary contributes nothing even if the route around it
//!   ever acquires a cost again.
//! * [`violating_box_role_validation_still_stamps_every_role`] — the companion
//!   that makes the headline mean anything. "The vocabulary allocates nothing"
//!   is satisfied perfectly by a validator that stopped stamping roles, so this
//!   drives the SAME fixture's violating branch through the SAME change-path
//!   entry point and pins every role vector, by content, on every result.
//! * [`box_roles_are_pinned_for_path_nested_and_reifier_shapes`] — the three
//!   shapes where the role computation is not a single flat lookup: a property
//!   shape whose path predicate carries a role, a nested property shape whose
//!   ancestors contribute theirs, and the reifier path, which is the one caller
//!   of `with_cbox_role` and the one place a role individual is added rather
//!   than found.
//!
//! # What was measured
//!
//! Figures from this file's fixture, one property shape, at `N = 2,048` and
//! `2N = 4,096` conforming focus nodes. The "before" column was taken by forcing
//! both role computations eager again — the behaviour this revision replaced —
//! and re-running the same tests:
//!
//! | configuration | `N` | `2N` | `2N - N` |
//! |---|---|---|---|
//! | vocabulary, roles computed eagerly | 32,782 | 65,554 | 32,772 |
//! | vocabulary, roles computed lazily | 8,206 | 16,402 | 8,196 |
//! | no vocabulary (the feature inactive) | 8,206 | 16,402 | 8,196 |
//!
//! `32,782 - 8,206 = 24,576`, which over 2,048 focus nodes is exactly **12
//! allocations per conforming focus node** that configuring a box-role
//! vocabulary used to cost and never reported — 4,061,184 bytes of allocator
//! traffic at `N = 2,048`, and very nearly a quadrupling of the conforming
//! change path's allocation count. It now costs exactly zero, to the single
//! allocation: the configured and unconfigured rows are identical at both sizes.
//!
//! The growth term both rows used to carry was the change path's own focus
//! materialization, not box roles, and it has since been removed —
//! [`conforming_box_role_validation_has_no_growth_term_in_focus_count`] now holds
//! absolutely as well as differentially.
//!
//! # The instrument, and the traps it is threaded around
//!
//! Identical to `tests/change_path_alloc.rs`, and for the same reasons:
//!
//! * SHACL validation fans focus nodes over `rayon` above its parallel
//!   threshold, so a [`purrdf_alloc_probe::CurrentThreadWindow`] would miss every
//!   worker and report a stable figure that measured almost nothing. Every
//!   measurement here uses a [`WholeProcessWindow`].
//! * That window reads one process-global ledger and `cargo test` runs a
//!   binary's tests concurrently, so [`MEASURE_LOCK`] serializes every measured
//!   region in this binary. Poisoning is absorbed, so one failure does not
//!   cascade into unrelated ones that hide it.
//! * [`FOCUS_NODES`] and its double both sit above
//!   [`PARALLEL_MIN_FOCUS_NODES`], checked when the file compiles, and
//!   [`assert_parallel_path_is_reachable`] refuses to report a figure from a host
//!   whose pool cannot take that path.
//! * Every measured region is executed once, with the same arguments, BEFORE its
//!   window opens. `rayon`'s global pool, the class-membership index and the
//!   allocator's own arenas are all first-touch lazies, and charging them to
//!   whichever measurement ran first would make these figures about start-up.
//!
//! One trap is specific to a differential measurement: the two bindings must
//! differ in NOTHING but the configuration bit. They therefore share one frozen
//! [`RdfDataset`] — including its `meta:graphBoxRole` annotation triples, which
//! the unconfigured binding simply never looks for — and one shapes document,
//! whose `meta:graphBoxRole` statements the unconfigured parse ignores. Two
//! fixtures built from two sources could differ in interning order or quad count,
//! and an allocation difference would then be attributable to the fixtures rather
//! than to the feature.
//!
//! # Fixtures
//!
//! Every IRI is under `example.org`, including the box-role vocabulary itself.
//! PurRDF mints no vocabulary IRIs and a test fixture is no more entitled to
//! invent one than a release build is; [`BoxRoleVocab::for_namespace`] derives the
//! six term IRIs from the fixture's own namespace.
//!
//! # Running it
//!
//! ```text
//! cargo test -p purrdf-shapes --test box_role_alloc
//! ```

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_shapes::engine::{FocusId, PreparedShapes, PreparedValidator, parse_shapes_with_config};
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::report::{ValidationReport, ValidationResult};
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
///
/// The caller holds [`measure_lock`] and has warmed the operation; this helper
/// only brackets it.
fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = WholeProcessWindow::open();
    let value = operation();
    (value, window.close())
}

/// The fixture's data namespace. Caller-supplied and `example.org` by rule.
const NS: &str = "http://example.org/purrdf/box-role#";

/// The fixture's box-role vocabulary namespace.
///
/// A SECOND namespace, not a corner of [`NS`], because a vocabulary is
/// configuration a caller brings and the code must not be able to guess it from
/// the data.
const META: &str = "http://example.org/purrdf/box-role/meta#";

/// `rdf:type`, spelled out because the measured fixture is built id-natively.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Mirrors `PARALLEL_MIN_FOCUS_NODES` in `crates/shapes/src/parallel.rs`.
///
/// That constant is `pub(crate)`, so an integration test cannot read it. It is
/// mirrored rather than approximated because the whole point of [`FOCUS_NODES`]
/// is to sit above it, and [`assert_parallel_path_is_reachable`] is what turns
/// the mirror into a checked claim about the host.
const PARALLEL_MIN_FOCUS_NODES: usize = 1_024;

/// The focus-node count `N` in every `N` versus `2N` comparison.
const FOCUS_NODES: usize = 2_048;

/// How many violating focus nodes the measured fixture carries.
///
/// Small and fixed: the violating branch is what
/// [`violating_box_role_validation_still_stamps_every_role`] reads role contents
/// off, and every one of them is inspected individually.
const VIOLATIONS: usize = 8;

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

/// The shapes document both bindings parse.
///
/// The `meta:graphBoxRole` statements are present in BOTH parses. With no
/// vocabulary configured the parser never asks for that predicate, so they are
/// inert data rather than a difference between the two fixtures — which is what
/// makes the configuration bit the only thing that differs.
const SHAPES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/purrdf/box-role#> .\n",
    "@prefix meta: <http://example.org/purrdf/box-role/meta#> .\n",
    "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;\n",
    "    meta:graphBoxRole meta:boxTBox ;\n",
    "    sh:property ex:NameShape .\n",
    "ex:NameShape sh:path ex:name ; sh:minCount 1 ;\n",
    "    meta:graphBoxRole meta:boxABox .\n",
);

/// Intern `{META}{local}`.
fn meta(local: &str) -> String {
    format!("{META}{local}")
}

/// The fixture's vocabulary.
fn vocabulary() -> BoxRoleVocab {
    BoxRoleVocab::for_namespace(META)
}

/// A role vector as plain strings, for content comparison against an expected
/// list that a reader can check by eye.
fn role_iris(roles: &[NamedNode]) -> Vec<String> {
    roles.iter().map(|role| role.as_str().to_owned()).collect()
}

/// The measured fixture: one dataset, two bindings differing only in whether a
/// vocabulary is configured, and the focus ids to drive them with.
#[derive(Debug)]
struct Fixture {
    /// The binding whose shapes carry the configured [`BoxRoleVocab`].
    with_vocab: PreparedValidator,
    /// The binding whose shapes were parsed with the feature inactive.
    without_vocab: PreparedValidator,
    /// Focus ids for [`Fixture::with_vocab`]: the conforming population, then
    /// the violating one.
    ///
    /// A [`FocusId`] is only valid against the binding that minted it, and the
    /// two bindings here are two separate carriers over one dataset, so each
    /// gets its own pair. The ids are resolved in [`Fixture::build`], outside
    /// every measurement window, exactly as the bare ids were.
    with_vocab_ids: FocusSets,
    /// The same two populations minted by [`Fixture::without_vocab`].
    without_vocab_ids: FocusSets,
}

/// One binding's conforming and violating focus ids, in construction order.
#[derive(Debug)]
struct FocusSets {
    /// Focus nodes that satisfy the shape.
    conforming: Vec<FocusId>,
    /// Focus nodes that break it.
    violating: Vec<FocusId>,
}

/// Build the shared data graph.
///
/// The `meta:graphBoxRole` annotation on `ex:name` is what gives the path a role
/// to find; without it the configured run would take the vocabulary's branch and
/// still find nothing, and "the feature costs nothing" would be measured over a
/// feature with no work to do.
fn build_dataset(conforming: usize, violating: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let focus_class = builder.intern_iri(&format!("{NS}Focus"));
    let name = builder.intern_iri(&format!("{NS}name"));
    let graph_box_role = builder.intern_iri(&meta("graphBoxRole"));
    let box_rbox = builder.intern_iri(&meta("boxRBox"));
    builder.push_quad(name, graph_box_role, box_rbox, None);

    for index in 0..conforming {
        let focus = builder.intern_iri(&format!("{NS}c{index}"));
        builder.push_quad(focus, rdf_type, focus_class, None);
        let label = builder.intern_literal(RdfLiteral::simple(format!("item-{index}")));
        builder.push_quad(focus, name, label, None);
    }
    for index in 0..violating {
        let focus = builder.intern_iri(&format!("{NS}v{index}"));
        builder.push_quad(focus, rdf_type, focus_class, None);
    }
    builder.freeze().expect("the box-role fixture must freeze")
}

/// Bind [`SHAPES`] over `dataset`, with the box-role feature active or inactive.
fn bind(dataset: &Arc<RdfDataset>, vocab: Option<BoxRoleVocab>) -> PreparedValidator {
    let configured = vocab.is_some();
    let shapes =
        parse_shapes_with_config(SHAPES, None, vocab, &purrdf_shapes::ShapesImports::new())
            .unwrap_or_else(|error| panic!("the box-role shapes must parse: {error}"));
    PreparedShapes::new(Arc::new(shapes))
        .bind_shared_dataset(Arc::clone(dataset))
        .unwrap_or_else(|error| {
            panic!("the box-role fixture must bind (vocabulary configured: {configured}): {error}")
        })
}

impl Fixture {
    /// Build one dataset and both bindings over it.
    fn build(conforming: usize, violating: usize) -> Self {
        let dataset = build_dataset(conforming, violating);
        let with_vocab = bind(&dataset, Some(vocabulary()));
        let without_vocab = bind(&dataset, None);
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
        let sets = |validator: &PreparedValidator| FocusSets {
            conforming: resolve(validator, 'c', conforming),
            violating: resolve(validator, 'v', violating),
        };
        Self {
            with_vocab_ids: sets(&with_vocab),
            without_vocab_ids: sets(&without_vocab),
            with_vocab,
            without_vocab,
        }
    }

    /// The binding under test and the focus ids it minted.
    fn binding(&self, configured: bool) -> (&PreparedValidator, &FocusSets, &'static str) {
        if configured {
            (&self.with_vocab, &self.with_vocab_ids, "with a vocabulary")
        } else {
            (&self.without_vocab, &self.without_vocab_ids, "with none")
        }
    }

    /// Validate the first `focus_nodes` conforming ids through one binding,
    /// requiring the report to conform.
    fn validate_conforming(
        &self,
        configured: bool,
        focus_nodes: usize,
    ) -> (ValidationReport, &'static str) {
        let (validator, ids, label) = self.binding(configured);
        let report = validator
            .validate_focus_node_ids(&ids.conforming[..focus_nodes])
            .unwrap_or_else(|error| panic!("the conforming set must validate {label}: {error}"));
        assert!(
            report.conforms,
            "the conforming population must conform {label}, or the allocation figure describes a \
             workload that never reached the constraint ({} result(s))",
            report.results.len()
        );
        (report, label)
    }

    /// Validate every violating id through one binding, requiring one result per
    /// violating focus node.
    fn validate_violating(&self, configured: bool) -> ValidationReport {
        let (validator, ids, _) = self.binding(configured);
        let report = validator
            .validate_focus_node_ids(&ids.violating)
            .unwrap_or_else(|error| panic!("the violating set must validate: {error}"));
        assert_eq!(
            report.results.len(),
            ids.violating.len(),
            "each of the {} violating focus nodes must produce exactly one result; {} means the \
             fixture is not exercising the branch the role assertions are written about",
            ids.violating.len(),
            report.results.len()
        );
        report
    }
}

/// Refuse to report a figure from a run where the parallel path cannot be taken.
///
/// Both sizes are above the threshold at compile time, but a single-threaded
/// `rayon` pool keeps validation serial whatever the sizes are, and a serial
/// figure asserted under a parallel test name is the "green because it measured
/// almost nothing" failure this instrument exists to avoid. That is a property of
/// the host, so it is checked at run time.
fn assert_parallel_path_is_reachable() {
    assert!(
        rayon::current_num_threads() > 1,
        "SHACL validation stays serial on a single-threaded rayon pool, so this host cannot \
         exercise the parallel path these assertions are written about"
    );
}

/// The roles a violating result of the measured fixture must carry.
///
/// `boxTBox` comes from the node shape, `boxABox` from the property shape and
/// `boxRBox` from the `ex:name` predicate's annotation in the DATA graph — one
/// role from each of the three sources, so a stamp that lost any one of them
/// fails here by name.
fn assert_measured_fixture_roles(result: &ValidationResult) {
    assert_eq!(
        role_iris(&result.source_box_roles),
        vec![meta("boxABox"), meta("boxTBox")],
        "a violating result must carry the property shape's role and its parent node shape's"
    );
    assert_eq!(
        role_iris(&result.path_box_roles),
        vec![meta("boxRBox")],
        "a violating result must carry the path predicate's role from the data graph"
    );
    assert_eq!(
        role_iris(&result.result_box_roles),
        vec![meta("boxABox"), meta("boxRBox"), meta("boxTBox")],
        "a violating result's result roles must be the deduplicated union of the other two"
    );
}

// ---------------------------------------------------------------------------
// 1. The headline
// ---------------------------------------------------------------------------

/// **Configuring a box-role vocabulary adds nothing at all to a conforming focus
/// node, at `N` focus nodes and at `2N`.**
///
/// The claim is stated as a DIFFERENCE between two bindings rather than as an
/// absolute figure, and that is not a softening. Whatever the route around the
/// feature costs — it used to carry a per-focus-node materialization term, and
/// could acquire another — it is identical on both sides here, so it cancels
/// exactly. What survives the subtraction is the box-role feature's entire
/// contribution, and the assertion is that it is ZERO: not small, not bounded,
/// but not a single allocation, at either size.
///
/// Equality is asserted EXACTLY. An allocation count is a fact about the code,
/// not about the host, so two runs of the same work produce the same number and
/// any tolerance here would be a place for the term this test exists to forbid to
/// hide.
#[test]
fn a_box_role_vocabulary_adds_no_allocation_to_the_conforming_path() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    let fixture = Fixture::build(2 * FOCUS_NODES, VIOLATIONS);

    // Non-vacuity, outside every window. "The vocabulary costs nothing" is
    // satisfied trivially by a vocabulary that does nothing, so the configured
    // binding is required to really produce roles and the unconfigured one is
    // required to really produce none, BEFORE any figure below is believed.
    for result in &fixture.validate_violating(true).results {
        assert_measured_fixture_roles(result);
    }
    for result in &fixture.validate_violating(false).results {
        assert!(
            result.source_box_roles.is_empty()
                && result.path_box_roles.is_empty()
                && result.result_box_roles.is_empty(),
            "with no vocabulary configured the box-role feature is inactive and every role list \
             must stay empty; PurRDF mints no vocabulary IRIs"
        );
    }

    let mut measured = Vec::with_capacity(4);
    for configured in [true, false] {
        for focus_nodes in [FOCUS_NODES, 2 * FOCUS_NODES] {
            // Warm-up, OUTSIDE the window and with the exact arguments measured:
            // rayon's global pool, the class-membership index and the allocator's
            // arenas are first-touch lazies, and charging them to whichever
            // measurement ran first would make this a statement about start-up.
            drop(fixture.validate_conforming(configured, focus_nodes));
            let (report, sample) = measure(|| fixture.validate_conforming(configured, focus_nodes));
            drop(report);
            measured.push(sample);
        }
    }
    let (vocab_n, vocab_2n) = (measured[0], measured[1]);
    let (plain_n, plain_2n) = (measured[2], measured[3]);

    assert_eq!(
        vocab_n.allocations, plain_n.allocations,
        "at {FOCUS_NODES} conforming focus nodes the configured binding allocated {} and the \
         unconfigured one {}, so the box-role feature charges a conforming focus node for \
         provenance it never reports\n  vocabulary = {vocab_n:?}\n  none       = {plain_n:?}",
        vocab_n.allocations, plain_n.allocations,
    );
    assert_eq!(
        vocab_2n.allocations,
        plain_2n.allocations,
        "at {} conforming focus nodes the configured binding allocated {} and the unconfigured \
         one {}\n  vocabulary = {vocab_2n:?}\n  none       = {plain_2n:?}",
        2 * FOCUS_NODES,
        vocab_2n.allocations,
        plain_2n.allocations,
    );
    // Implied by the two equalities above, and stated anyway: "no growth term in
    // the focus count" is the claim, and a reader should find it written down
    // rather than have to derive it from a pair of absolute comparisons.
    assert_eq!(
        vocab_2n.allocations - vocab_n.allocations,
        plain_2n.allocations - plain_n.allocations,
        "doubling the conforming focus count cost the configured binding {} extra allocations and \
         the unconfigured one {}, so the box-role feature carries a growth term in the focus count",
        vocab_2n.allocations - vocab_n.allocations,
        plain_2n.allocations - plain_n.allocations,
    );
}

// ---------------------------------------------------------------------------
// 2. The same claim, stated absolutely
// ---------------------------------------------------------------------------

/// **With a box-role vocabulary configured, validating a conforming focus set
/// costs the same whether it holds `N` nodes or `2N`.**
///
/// This is the invariant a reader arrives looking for. It was red on the change
/// path's own per-focus-node materialization term, under either configuration,
/// and it went green with no edit to this file when that term was removed —
/// which is exactly what an executable specification is supposed to do.
/// [`a_box_role_vocabulary_adds_no_allocation_to_the_conforming_path`] states the
/// same thing with whatever the surrounding route costs cancelled rather than
/// assumed away, and the two are kept side by side for that reason.
#[test]
fn conforming_box_role_validation_has_no_growth_term_in_focus_count() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    let fixture = Fixture::build(2 * FOCUS_NODES, VIOLATIONS);
    for result in &fixture.validate_violating(true).results {
        assert_measured_fixture_roles(result);
    }

    // Warm-up for both regions, outside every window; see the headline test.
    drop(fixture.validate_conforming(true, FOCUS_NODES));
    drop(fixture.validate_conforming(true, 2 * FOCUS_NODES));

    let (half, half_measured) = measure(|| fixture.validate_conforming(true, FOCUS_NODES));
    drop(half);
    let (full, full_measured) = measure(|| fixture.validate_conforming(true, 2 * FOCUS_NODES));
    drop(full);

    assert_eq!(
        half_measured.allocations,
        full_measured.allocations,
        "with a vocabulary configured the change path allocated {} for {FOCUS_NODES} conforming \
         focus nodes and {} for {}, so its cost carries a growth term in the focus \
         count\n  N  = {half_measured:?}\n  2N = {full_measured:?}",
        half_measured.allocations,
        full_measured.allocations,
        2 * FOCUS_NODES,
    );
}

// ---------------------------------------------------------------------------
// 3. The companion that makes the headline mean anything
// ---------------------------------------------------------------------------

/// **A violating focus node still carries every role, through the same entry
/// point the measurement drives.**
///
/// "The box-role feature allocates nothing on the conforming path" is satisfied
/// exactly as well by a validator that computes roles for nobody as by one that
/// computes them lazily. This takes the measured fixture's VIOLATING branch
/// through [`PreparedValidator::validate_focus_node_ids`] — the surface the
/// figures above are taken over, not a convenience wrapper beside it — and pins
/// all three role vectors by content on every result.
///
/// It also drives the unconfigured binding over the same nodes and requires the
/// same violations with empty role lists, so a regression that started minting
/// roles without a configured vocabulary fails here too. PurRDF mints no
/// vocabulary IRIs, and an inactive feature that quietly acquired a default would
/// be the same defect read from the other side.
#[test]
fn violating_box_role_validation_still_stamps_every_role() {
    let _guard = measure_lock();
    let fixture = Fixture::build(FOCUS_NODES, VIOLATIONS);

    let configured = fixture.validate_violating(true);
    assert!(!configured.conforms, "the violating set must not conform");
    for result in &configured.results {
        assert_measured_fixture_roles(result);
    }

    let plain = fixture.validate_violating(false);
    assert_eq!(
        plain.results.len(),
        configured.results.len(),
        "configuring a vocabulary must change the provenance on a result, never whether the \
         result exists"
    );
    for result in &plain.results {
        assert!(
            result.source_box_roles.is_empty()
                && result.path_box_roles.is_empty()
                && result.result_box_roles.is_empty(),
            "with no vocabulary configured every role list must stay empty"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The shapes where the role computation is not a flat lookup
// ---------------------------------------------------------------------------

/// One shape whose violating result's roles are pinned by content.
struct RoleCase {
    /// Appears in every assertion message, so a failure says which shape broke.
    name: &'static str,
    /// The shapes graph, Turtle.
    shapes: &'static str,
    /// The data graph, N-Triples.
    data: &'static str,
    /// The expected `source_box_roles`, as local names under [`META`].
    source: &'static [&'static str],
    /// The expected `path_box_roles`, as local names under [`META`].
    path: &'static [&'static str],
}

/// Turtle prefixes every case's shapes graph opens with.
const CASE_PREFIXES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/purrdf/box-role#> .\n",
    "@prefix meta: <http://example.org/purrdf/box-role/meta#> .\n",
);

/// The three shapes where the role computation does more than read one shape's
/// annotation.
const ROLE_CASES: &[RoleCase] = &[
    RoleCase {
        // A path whose predicate carries a role in the DATA graph: the case that
        // reaches `path_box_roles`' graph scan, which is the expensive half of
        // what the conforming path must not pay for.
        name: "path_predicate",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetNode ex:n ;
                     meta:graphBoxRole meta:boxTBox ;
                     sh:property ex:NameShape .
                 ex:NameShape sh:path ex:name ; sh:minCount 1 ;
                     meta:graphBoxRole meta:boxABox .",
        data: "<http://example.org/purrdf/box-role#name> \
               <http://example.org/purrdf/box-role/meta#graphBoxRole> \
               <http://example.org/purrdf/box-role/meta#boxRBox> .",
        source: &["boxABox", "boxTBox"],
        path: &["boxRBox"],
    },
    RoleCase {
        // A nested property shape: the source roles are the merge of the whole
        // ancestor chain, which is what the lazy initializer has to reproduce
        // exactly rather than approximately.
        name: "nested_property",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetNode ex:n ; sh:property ex:Outer ;
                     meta:graphBoxRole meta:boxTBox .
                 ex:Outer sh:path ex:outer ; sh:property ex:Inner ;
                     meta:graphBoxRole meta:boxConfigBox .
                 ex:Inner sh:path ex:required ; sh:minCount 1 ;
                     meta:graphBoxRole meta:boxABox .",
        data: "<http://example.org/purrdf/box-role#n> \
               <http://example.org/purrdf/box-role#outer> \
               <http://example.org/purrdf/box-role#child> .\n\
               <http://example.org/purrdf/box-role#required> \
               <http://example.org/purrdf/box-role/meta#graphBoxRole> \
               <http://example.org/purrdf/box-role/meta#boxRBox> .",
        source: &["boxABox", "boxConfigBox", "boxTBox"],
        path: &["boxRBox"],
    },
    RoleCase {
        // The reifier path: the ONE caller of `with_cbox_role`, and the one place
        // a role individual is ADDED to the source roles rather than read off a
        // shape or a predicate. `boxCBox` appears in no graph here — it comes
        // from the configured vocabulary alone.
        name: "reifier_cbox",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetNode ex:n ; sh:property ex:Property ;
                     meta:graphBoxRole meta:boxTBox .
                 ex:Property sh:path ex:p ; sh:reificationRequired true ;
                     meta:graphBoxRole meta:boxABox .",
        data: "<http://example.org/purrdf/box-role#n> \
               <http://example.org/purrdf/box-role#p> \
               <http://example.org/purrdf/box-role#value> .\n\
               <http://example.org/purrdf/box-role#p> \
               <http://example.org/purrdf/box-role/meta#graphBoxRole> \
               <http://example.org/purrdf/box-role/meta#boxRBox> .",
        source: &["boxABox", "boxCBox", "boxTBox"],
        path: &["boxRBox"],
    },
];

/// **Every role vector is pinned by content on the three shapes where the
/// computation is not a single flat lookup.**
///
/// The laziness moved WHEN the roles are computed. This is the statement that it
/// did not move WHAT they are: a path predicate's role read out of the data
/// graph, an ancestor chain's roles merged down through a nested property shape,
/// and the CBox individual the reifier path adds from the vocabulary. Counts
/// would not catch a merge that dropped an ancestor or an initializer that
/// captured the wrong slice, so the contents are asserted, sorted and complete.
#[test]
fn box_roles_are_pinned_for_path_nested_and_reifier_shapes() {
    let _guard = measure_lock();
    for case in ROLE_CASES {
        let shapes = format!("{CASE_PREFIXES}{}", case.shapes);
        let report = purrdf_shapes::engine::validate_graphs_with_config(
            case.data,
            &shapes,
            None,
            Some(vocabulary()),
            &purrdf_shapes::ShapesImports::new(),
        )
        .unwrap_or_else(|error| panic!("case {} must validate: {error}", case.name));
        assert_eq!(
            report.results.len(),
            1,
            "case {}: the fixture must produce exactly one result, or it pins nothing",
            case.name
        );
        let result = &report.results[0];
        let expected_source: Vec<String> = case.source.iter().copied().map(meta).collect();
        let expected_path: Vec<String> = case.path.iter().copied().map(meta).collect();
        let mut expected_result = expected_source.clone();
        expected_result.extend(expected_path.iter().cloned());
        expected_result.sort_unstable();
        expected_result.dedup();

        assert_eq!(
            role_iris(&result.source_box_roles),
            expected_source,
            "case {}: source roles",
            case.name
        );
        assert_eq!(
            role_iris(&result.path_box_roles),
            expected_path,
            "case {}: path roles",
            case.name
        );
        assert_eq!(
            role_iris(&result.result_box_roles),
            expected_result,
            "case {}: result roles must be the deduplicated union of the other two",
            case.name
        );
    }
}

// ---------------------------------------------------------------------------
// 5. The regression guard: every test in this binary holds MEASURE_LOCK first
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
    let source = include_str!("box_role_alloc.rs");
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
