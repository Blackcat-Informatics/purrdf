// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public-surface tests for governed SPARQL **UPDATE**.
//!
//! Every test here drives the **public** API only — `NativeSparqlEngine::update_governed`,
//! the ungoverned `SparqlEngine::update` it is the sibling of, and the outcome types they
//! return. A mutation governor that can only be reached from inside the crate governs
//! nothing a consumer can act on, so these are written from exactly the vantage a consumer
//! has.
//!
//! The load-bearing property is negative and therefore easy to claim and hard to notice
//! losing: **a tripped request applies nothing**. It is asserted here by comparing a
//! deterministic byte image of the store across the trip, and by asserting that the
//! caller's `Arc` handle is the *same* handle — not an equal one — because "rebuilt to an
//! equal value" and "never written" are different facts and only the second is the contract.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use purrdf_core::{
    DatasetMut, GraphMatchValue, MutableDataset, RdfDataset, RdfDatasetBuilder, RdfDiagnostic,
    ResourceDimension, SparqlEngine, SparqlRequest, StopCause, TrippedGovernor,
};
use purrdf_sparql_eval::{
    CancellationFlag, GovernedUpdateOutcome, GraphResolver, NativeSparqlEngine, QueryGovernors,
    StopSignal,
};

/// The number of `ex:p` edges in the fixture store.
const EDGES: usize = 8;

const PREFIX: &str = "PREFIX ex: <http://example.org/>\n";

/// `EDGES` subjects, each with one `ex:p` edge in the default graph, plus one quad in a
/// named graph so the graph-scoped operations (`CLEAR`, `ADD`, `COPY`, `MOVE`) have
/// something to move around.
fn fixture() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    for index in 0..EDGES {
        let s = builder.intern_iri(&format!("http://example.org/s{index}"));
        let o = builder.intern_iri(&format!("http://example.org/o{index}"));
        builder.push_quad(s, p, o, None);
    }
    let g = builder.intern_iri("http://example.org/g");
    let gs = builder.intern_iri("http://example.org/gs");
    let go = builder.intern_iri("http://example.org/go");
    builder.push_quad(gs, p, go, Some(g));
    builder.freeze().expect("freeze fixture")
}

/// A deterministic byte image of everything the store holds.
///
/// Every effective quad, rendered in value space and sorted, so the comparison is over the
/// store's *content* rather than over an interner layout that a rebuild would legitimately
/// change. Two stores with this image are indistinguishable to every query.
fn store_image(dataset: &Arc<RdfDataset>) -> String {
    let view = MutableDataset::new(Arc::clone(dataset));
    let mut lines: Vec<String> = view
        .quads_for_pattern(None, None, None, GraphMatchValue::Any)
        .iter()
        .map(|quad| format!("{:?} {:?} {:?} {:?}\n", quad.s, quad.p, quad.o, quad.g))
        .collect();
    lines.sort_unstable();
    lines.concat()
}

fn request(update: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query: update,
        base_iri: None,
        substitutions: &[],
    }
}

/// Run `update` over a fresh fixture under `governors`, asserting only that it is not an
/// error — a trip is an outcome, never an `Err`.
fn governed(update: &str, governors: &QueryGovernors) -> (Arc<RdfDataset>, GovernedUpdateOutcome) {
    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let outcome = engine
        .update_governed(&mut dataset, request(update), governors)
        .expect("a tripped governor is an outcome, not an update error");
    (dataset, outcome)
}

/// The fuel one run of `update` costs, measured rather than guessed.
///
/// Every ceiling in this file is derived from a `METERED` run of the very request under
/// test, so a test says "one unit less than this request needs" instead of pinning a
/// literal that a schedule change would silently turn into a different assertion.
fn metered_fuel(update: &str) -> u64 {
    let (_, outcome) = governed(update, &QueryGovernors::METERED);
    assert!(
        outcome.is_applied(),
        "the metering run must complete: {outcome:?}"
    );
    // Asserted on every measurement in this file, because it is the invariant the `Applied`
    // arm claims: a request that applied is a request no governor stopped. An applied
    // outcome beside a latched trip would be a store published under a governor that had
    // already fired.
    assert_eq!(
        outcome.evidence().tripped,
        None,
        "an applied request reported a trip"
    );
    outcome.evidence().consumed.get(ResourceDimension::Fuel)
}

/// The fuel a governed **query** of `select` costs, for comparison against an UPDATE whose
/// `WHERE` is the same pattern.
fn metered_query_fuel(select: &str) -> u64 {
    let outcome = NativeSparqlEngine::new()
        .query_governed(&fixture(), request(select), &QueryGovernors::METERED)
        .expect("the metering query runs");
    outcome.evidence().consumed.get(ResourceDimension::Fuel)
}

/// A `GraphResolver` that counts how many times the host was actually asked to fetch.
///
/// The counter is the whole point: "the request was refused before the I/O" and "the I/O
/// happened and its result was thrown away" are indistinguishable from the store, and only
/// the first is a governor.
#[derive(Debug)]
struct CountingResolver {
    calls: AtomicUsize,
    document: Arc<RdfDataset>,
}

impl CountingResolver {
    fn new() -> Arc<Self> {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/loaded");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/value");
        builder.push_quad(s, p, o, None);
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            document: builder.freeze().expect("freeze the loadable document"),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl GraphResolver for CountingResolver {
    fn resolve(
        &self,
        _request: purrdf_sparql_eval::GraphResolveRequest<'_>,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(Arc::clone(&self.document))
    }
}

#[derive(Debug)]
struct CancellingResolver {
    calls: AtomicUsize,
    flag: CancellationFlag,
    document: Arc<RdfDataset>,
}

impl CancellingResolver {
    fn new(flag: CancellationFlag) -> Arc<Self> {
        let base = CountingResolver::new();
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            flag,
            document: Arc::clone(&base.document),
        })
    }
}

impl GraphResolver for CancellingResolver {
    fn resolve(
        &self,
        request: purrdf_sparql_eval::GraphResolveRequest<'_>,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert!(request.stop.is_some(), "LOAD must carry the stop signal");
        // Complete the fetch after firing cancellation: the engine's post-return poll,
        // not resolver cooperation, must prevent publication.
        self.flag.cancel();
        Ok(Arc::clone(&self.document))
    }
}

/// A stop signal that reports clear for its first `quiet` polls and `Cancelled` for every
/// poll after that.
///
/// It honours the latching contract — once it says `Some`, it says `Some` forever — while
/// letting a test place the firing edge at a chosen poll. That is what makes it possible to
/// prove the `LOAD` seam does its *own* poll: with `quiet = 1` the per-operation poll sees
/// a clear signal and only the poll inside `load` can stop the request.
#[derive(Debug)]
struct QuietThenCancelled {
    quiet: usize,
    polls: AtomicUsize,
    latched: AtomicBool,
}

impl QuietThenCancelled {
    fn new(quiet: usize) -> Arc<Self> {
        Arc::new(Self {
            quiet,
            polls: AtomicUsize::new(0),
            latched: AtomicBool::new(false),
        })
    }
}

impl StopSignal for QuietThenCancelled {
    fn poll(&self) -> Option<StopCause> {
        if self.latched.load(Ordering::Relaxed) {
            return Some(StopCause::Cancelled);
        }
        if self.polls.fetch_add(1, Ordering::Relaxed) < self.quiet {
            return None;
        }
        self.latched.store(true, Ordering::Relaxed);
        Some(StopCause::Cancelled)
    }
}

fn cancelled_governors() -> QueryGovernors {
    let flag = CancellationFlag::new();
    flag.cancel();
    QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag))
}

// ── the guarantee ────────────────────────────────────────────────────────────

/// A three-operation request whose third operation trips leaves the store byte-identical.
///
/// The first two operations are ordinary mutations that succeed, so this asserts the whole
/// *request* rolls back rather than only the operation the governor stopped. The control
/// run establishes that those first two operations really do change the store, which is
/// what makes the byte-identity a fact about the rollback rather than about a request that
/// happened to be a no-op.
#[test]
fn update_governed_trip_leaves_the_store_byte_identical() {
    let prefix_ops = format!(
        "{PREFIX}\
         INSERT DATA {{ ex:added ex:p ex:one }} ;\n\
         DELETE DATA {{ ex:s0 ex:p ex:o0 }}"
    );
    let whole = format!(
        "{prefix_ops} ;\n\
         DELETE {{ ?s ex:p ?o }} INSERT {{ ?s ex:q ?o }} WHERE {{ ?s ex:p ?o }}"
    );

    // Control: ungoverned, the request applies and the store changes — including the two
    // operations the governed run is about to discard.
    let (applied, outcome) = governed(&whole, &QueryGovernors::UNBOUNDED);
    assert!(outcome.is_applied());
    let before = fixture();
    assert_ne!(
        store_image(&applied),
        store_image(&before),
        "the control run must actually mutate, or byte-identity proves nothing"
    );
    let (after_two, _) = governed(&prefix_ops, &QueryGovernors::UNBOUNDED);
    assert_ne!(
        store_image(&after_two),
        store_image(&before),
        "operations 1 and 2 must be real mutations on their own"
    );

    // Exactly enough fuel for the first two operations and not for the third: measured, so
    // the ceiling stays meaningful if the charge schedule ever moves.
    let ceiling = metered_fuel(&prefix_ops);
    assert!(ceiling > 0, "the first two operations must charge");
    assert!(
        metered_fuel(&whole) > ceiling,
        "the third operation must charge more than the first two"
    );

    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&whole),
            &QueryGovernors::UNBOUNDED.with_fuel(ceiling),
        )
        .expect("a tripped governor is an outcome, not an update error");

    assert!(
        !outcome.is_applied(),
        "the request must report that it did not apply: {outcome:?}"
    );
    assert!(matches!(
        outcome.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            ..
        })
    ));
    assert_eq!(
        store_image(&dataset),
        image_before,
        "a tripped request left mutations behind"
    );
    assert!(
        Arc::ptr_eq(&dataset, &handle_before),
        "the caller's handle was republished; a trip must not write it at all"
    );
}

/// The same guarantee where the trip lands inside a **bulk** operation, which does mutate
/// as it iterates.
///
/// `delete_insert` collects its whole mutation before touching the store, so a trip there
/// could never have left a partial write even without the branch. `CLEAR ALL` can: it
/// removes quads one at a time. This is the case the branch-drop rollback exists for.
#[test]
fn a_trip_inside_a_bulk_operation_still_leaves_the_store_byte_identical() {
    let update = format!("{PREFIX}INSERT DATA {{ ex:added ex:p ex:one }} ;\nCLEAR ALL");
    let full = metered_fuel(&update);
    assert!(full > 2, "the request must charge more than a token amount");

    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            // Enough for the INSERT DATA and part of the CLEAR, never enough for all of it.
            &QueryGovernors::UNBOUNDED.with_fuel(full - 1),
        )
        .expect("a tripped governor is an outcome");

    assert!(!outcome.is_applied(), "{outcome:?}");
    assert_eq!(store_image(&dataset), image_before);
    assert!(Arc::ptr_eq(&dataset, &handle_before));
}

// ── LOAD: the host seam ──────────────────────────────────────────────────────

/// A latched stop signal must stop the request **before** the host is asked to fetch.
///
/// The resolver counts calls, so this distinguishes "refused before the I/O" from "the I/O
/// ran and its result was discarded". Only the first is a governor: a fetch that has
/// already been issued has already spent the network and the wait the deadline existed to
/// bound.
#[test]
fn load_is_not_issued_when_the_stop_signal_is_already_latched() {
    let resolver = CountingResolver::new();
    let engine = NativeSparqlEngine::new().with_resolver(Arc::clone(&resolver) as Arc<_>);
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&format!(
                "{PREFIX}INSERT DATA {{ ex:added ex:p ex:one }} ;\nLOAD ex:doc"
            )),
            &cancelled_governors(),
        )
        .expect("a tripped governor is an outcome, not an update error");

    assert_eq!(
        resolver.calls(),
        0,
        "the host was asked to fetch under a latched stop signal"
    );
    assert_eq!(
        outcome.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
    assert_eq!(store_image(&dataset), image_before);
    assert!(Arc::ptr_eq(&dataset, &handle_before));
}

/// The `LOAD` seam does its **own** poll, not merely the per-operation one.
///
/// The signal is clear for the first poll — the one `apply_operation` makes on the way into
/// the single `LOAD` — and fires on the second, which only the check inside `load` can
/// make. Deleting that check would turn this test red while leaving the already-latched
/// test above green, which is exactly the regression it exists to catch.
#[test]
fn load_is_not_issued_when_the_stop_signal_latches_at_the_host_seam() {
    let resolver = CountingResolver::new();
    let engine = NativeSparqlEngine::new().with_resolver(Arc::clone(&resolver) as Arc<_>);
    let mut dataset = fixture();
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&format!("{PREFIX}LOAD ex:doc")),
            &QueryGovernors::UNBOUNDED.with_stop_signal(QuietThenCancelled::new(1)),
        )
        .expect("a tripped governor is an outcome");

    assert_eq!(
        resolver.calls(),
        0,
        "the fetch was issued after the signal had fired"
    );
    assert_eq!(
        outcome.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
    assert_eq!(store_image(&dataset), image_before);
}

/// `SILENT` says the *source* may be missing. It says nothing about the caller's budget, so
/// it must not turn a governor trip into a no-op success.
#[test]
fn load_silent_does_not_launder_a_governor_trip_into_a_noop_success() {
    let resolver = CountingResolver::new();
    let engine = NativeSparqlEngine::new().with_resolver(Arc::clone(&resolver) as Arc<_>);
    let mut dataset = fixture();

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&format!("{PREFIX}LOAD SILENT ex:doc")),
            &QueryGovernors::UNBOUNDED.with_stop_signal(QuietThenCancelled::new(1)),
        )
        .expect("a tripped governor is an outcome");

    assert_eq!(resolver.calls(), 0);
    assert!(
        !outcome.is_applied(),
        "SILENT reported a stopped request as applied: {outcome:?}"
    );
}

#[test]
fn load_returning_after_cancellation_is_discarded_before_publication() {
    for silent in [false, true] {
        let flag = CancellationFlag::new();
        let resolver = CancellingResolver::new(flag.clone());
        let engine = NativeSparqlEngine::new().with_resolver(Arc::clone(&resolver) as Arc<_>);
        let mut dataset = fixture();
        let handle_before = Arc::clone(&dataset);
        let image_before = store_image(&dataset);
        let silent = if silent { " SILENT" } else { "" };

        let outcome = engine
            .update_governed(
                &mut dataset,
                request(&format!("{PREFIX}LOAD{silent} ex:doc")),
                &QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag)),
            )
            .expect("a post-return stop is an outcome");

        assert_eq!(resolver.calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            outcome.tripped(),
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            })
        );
        assert_eq!(store_image(&dataset), image_before);
        assert!(Arc::ptr_eq(&dataset, &handle_before));
    }
}

/// A `LOAD` whose document is larger than the caller's remaining fuel trips on the
/// document, not on the request text.
///
/// The one mutation whose size the request cannot reveal: the host chooses how many quads
/// come back. A budget that could not bound it would be bounded by whatever the host felt
/// like returning.
#[test]
fn load_charges_for_the_document_it_ingests() {
    let resolver = CountingResolver::new();
    let engine = NativeSparqlEngine::new().with_resolver(Arc::clone(&resolver) as Arc<_>);
    let update = format!("{PREFIX}LOAD ex:doc");

    let mut metered = fixture();
    let applied = engine
        .update_governed(&mut metered, request(&update), &QueryGovernors::METERED)
        .expect("the metering run");
    assert!(applied.is_applied());
    assert_eq!(
        applied.evidence().consumed.get(ResourceDimension::Fuel),
        2,
        "one host request issued plus one quad ingested"
    );
    assert_eq!(
        applied
            .evidence()
            .consumed
            .get(ResourceDimension::RemoteRequests),
        1,
        "a LOAD is one request issued to a host"
    );

    // Fuel exhausted before the request charge: the fetch never happens.
    let mut dataset = fixture();
    let image_before = store_image(&dataset);
    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_fuel(0),
        )
        .expect("a tripped governor is an outcome");
    assert!(!outcome.is_applied(), "{outcome:?}");
    assert_eq!(store_image(&dataset), image_before);
    assert_eq!(
        resolver.calls(),
        1,
        "fuel that cannot pay for the request itself must stop before the fetch"
    );

    // Fuel enough for the request and not for the document: the fetch happens — its size
    // was not knowable in advance — and the ingest is refused rather than truncated.
    let mut dataset = fixture();
    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_fuel(1),
        )
        .expect("a tripped governor is an outcome");
    assert!(!outcome.is_applied(), "{outcome:?}");
    assert_eq!(
        store_image(&dataset),
        image_before,
        "a partially-ingested document is still a partial mutation"
    );
    assert_eq!(
        resolver.calls(),
        2,
        "the document's size is the host's answer, so bounding it is necessarily after \
         the fetch"
    );
}

/// A `LOAD` is a request issued to a host, so the remote-request ceiling bounds it — and
/// bounds it **before** the network is touched.
///
/// This is the only ceiling that can. The document's size is the host's answer, so the
/// per-quad ingest charge is necessarily after the fetch has already happened; how many
/// fetches a request may make is knowable in advance, and is therefore taken in advance.
/// A caller who bounded a federated query's endpoint count means the same thing by that
/// number here.
#[test]
fn a_remote_request_ceiling_bounds_load_before_the_fetch() {
    let resolver = CountingResolver::new();
    let engine = NativeSparqlEngine::new().with_resolver(Arc::clone(&resolver) as Arc<_>);
    let mut dataset = fixture();
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&format!("{PREFIX}LOAD ex:doc")),
            &QueryGovernors::UNBOUNDED.with_max_remote_requests(0),
        )
        .expect("a tripped governor is an outcome");

    assert_eq!(
        resolver.calls(),
        0,
        "the host was asked to fetch past a remote-request ceiling of zero"
    );
    assert_eq!(
        outcome.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::RemoteRequests,
            limit: 0,
            consumed: 1,
        })
    );
    assert_eq!(store_image(&dataset), image_before);
}

// ── the ungoverned path is untouched ─────────────────────────────────────────

/// An ungoverned UPDATE is the execution it was before governors existed.
///
/// Two halves, because "ungoverned" is two claims. The first is that the *result* is
/// unchanged: the ungoverned `SparqlEngine::update` seam and an `UNBOUNDED` governed run
/// produce byte-identical stores from the same request. The second is that no accounting
/// happened at all — `UNBOUNDED` reports zero on every dimension, which is only possible if
/// no charge site was entered, because every charge site that runs increments something.
/// `METERED` over the same request reports non-zero, which is what makes the zero a
/// short-circuit rather than an absence of work.
#[test]
fn an_ungoverned_update_is_the_execution_it_was_before_governors_existed() {
    let update = format!(
        "{PREFIX}\
         INSERT DATA {{ ex:added ex:p ex:one }} ;\n\
         DELETE {{ ?s ex:p ?o }} INSERT {{ ?s ex:q ?o }} WHERE {{ ?s ex:p ?o }} ;\n\
         COPY DEFAULT TO GRAPH ex:copy ;\n\
         CLEAR GRAPH ex:g"
    );

    let engine = NativeSparqlEngine::new();
    let mut ungoverned = fixture();
    engine
        .update(&mut ungoverned, request(&update))
        .expect("the ungoverned seam applies the request");

    let (bounded, outcome) = governed(&update, &QueryGovernors::UNBOUNDED);
    assert!(outcome.is_applied());
    assert_eq!(
        store_image(&bounded),
        store_image(&ungoverned),
        "governing a request with no ceilings changed what it did"
    );

    for dimension in ResourceDimension::ALL {
        // The recursion guard carries a fixed build ceiling on every execution, governed or
        // not, so it is the one dimension whose *limit* is finite here; its consumption is
        // still zero because nothing recursed.
        assert_eq!(
            outcome.evidence().consumed.get(dimension),
            0,
            "an UNBOUNDED request charged {dimension:?}, so a charge site ran"
        );
    }
    assert_eq!(outcome.evidence().tripped, None);

    let (metered, metered_outcome) = governed(&update, &QueryGovernors::METERED);
    assert!(metered_outcome.is_applied());
    assert!(
        metered_outcome
            .evidence()
            .consumed
            .get(ResourceDimension::Fuel)
            > 0,
        "METERED must observe the work UNBOUNDED declines to count"
    );
    assert_eq!(
        store_image(&metered),
        store_image(&ungoverned),
        "metering a request changed what it did"
    );
}

// ── the WHERE clause is charged as a query ───────────────────────────────────

/// A `DELETE/INSERT … WHERE` charges its `WHERE` exactly as the same pattern inside a
/// governed `SELECT` does.
///
/// Compared as the **marginal** cost of the pattern rather than as a raw total, because the
/// two requests are not the same algebra: a `SELECT` carries a `Project` the UPDATE has no
/// counterpart for, and a `DELETE/INSERT` carries a mutation the `SELECT` has no
/// counterpart for. What must agree — and what would disagree the moment the `WHERE`
/// stopped being evaluated under the request's governors — is the cost of the pattern
/// itself, isolated here by differencing two `WHERE`s that differ by one triple pattern.
#[test]
fn a_where_clause_charges_exactly_as_the_same_pattern_in_a_select() {
    let one = "{ ?s ex:p ?o }";
    let two = "{ ?s ex:p ?o . ?s ex:p ?o2 }";

    let select_marginal = metered_query_fuel(&format!("{PREFIX}SELECT * WHERE {two}"))
        - metered_query_fuel(&format!("{PREFIX}SELECT * WHERE {one}"));
    // `DELETE WHERE` deletes exactly what it matched, so the mutation half of the two
    // requests differs too; `INSERT { }`-free `DELETE { }` with a fixed, ground template
    // keeps the mutation identical across both and leaves only the pattern varying.
    let update_marginal = metered_fuel(&format!("{PREFIX}DELETE {{ ex:a ex:b ex:c }} WHERE {two}"))
        - metered_fuel(&format!("{PREFIX}DELETE {{ ex:a ex:b ex:c }} WHERE {one}"));

    assert!(select_marginal > 0, "the extra pattern must cost something");
    assert_eq!(
        update_marginal, select_marginal,
        "an UPDATE's WHERE is charged differently from the same pattern in a SELECT"
    );
}

/// A `WHERE` that would exceed the fuel ceiling stops the request, and the mutation the
/// truncated solutions would have produced is never applied.
#[test]
fn a_where_clause_stops_on_the_fuel_ceiling_and_applies_nothing() {
    let update = format!("{PREFIX}DELETE {{ ?s ex:p ?o }} WHERE {{ ?s ex:p ?o }}");
    let full = metered_fuel(&update);

    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let image_before = store_image(&dataset);
    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_fuel(full / 2),
        )
        .expect("a tripped governor is an outcome");

    assert!(!outcome.is_applied(), "{outcome:?}");
    assert_eq!(store_image(&dataset), image_before);
}

/// A governor that trips **inside** the `WHERE` crosses as a typed governor, on an `Ok`
/// outcome — not as a diagnostic and not as an error.
///
/// This is the arm that used to be unreachable: the `WHERE` was evaluated ungoverned, so
/// the truncation branch that rendered a trip into a formatted `RdfDiagnostic` was code no
/// execution could enter. The request here mutates nothing when it completes, so the
/// outcome is attributable to the pattern evaluation and to nothing else.
#[test]
fn a_governor_trip_inside_the_where_crosses_as_a_typed_governor_not_a_diagnostic() {
    let update = format!(
        "{PREFIX}DELETE {{ ?s ex:p ?o }} WHERE {{ ?s ex:p ?o . ?x ex:p ?y FILTER(false) }}"
    );

    // The request applies nothing even when it completes, so every unit of fuel it spends
    // is the WHERE's.
    let (applied, outcome) = governed(&update, &QueryGovernors::METERED);
    assert!(outcome.is_applied());
    assert_eq!(
        store_image(&applied),
        store_image(&fixture()),
        "the fixture request must be mutation-free for the charge to be attributable"
    );
    let fuel = outcome.evidence().consumed.get(ResourceDimension::Fuel);
    assert!(fuel > 1, "the WHERE must do real work: {fuel}");

    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_fuel(fuel - 1),
        )
        .expect("a WHERE-clause trip is an outcome, not a diagnostic");

    assert!(matches!(
        outcome.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::Fuel,
            ..
        })
    ));
    assert_eq!(store_image(&dataset), store_image(&fixture()));
}

/// A `WHERE` that is stopped part-way through applies **no** mutation — not the mutation
/// its partial solutions would have produced.
///
/// This is the sharpest form of the guarantee and the one no other test in this file
/// reaches. The configuration is deliberate: a stop signal and **no fuel ceiling**. Under a
/// fuel ceiling every later charge re-reports the latched trip, so an implementation that
/// quietly accepted the partial solutions would still be stopped by the next charge and
/// would look correct. With fuel unengaged there is no such backstop — the mutation charge
/// short-circuits before it can consult the trip — so the only thing standing between the
/// caller and a store built from "some of the answers" is the `WHERE`'s own truncation
/// being treated as a refusal. Accepting those rows here would delete a subset the caller
/// never asked for and report the request as applied.
///
/// The signal is clear for the first poll (the per-operation one) and fires afterwards,
/// inside pattern evaluation.
#[test]
fn a_where_stopped_part_way_applies_none_of_the_mutation_it_had_computed() {
    let update = format!(
        "{PREFIX}DELETE {{ ?s ex:p ?o }} WHERE {{ {{ ?s ex:p ?o }} UNION {{ ?s ex:p ?o }} }}"
    );

    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            // No ceiling on any dimension: a stop signal, and nothing else.
            &QueryGovernors::UNBOUNDED.with_stop_signal(QuietThenCancelled::new(1)),
        )
        .expect("a stopped WHERE is an outcome, not an update error");

    assert_eq!(
        outcome.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }),
        "{outcome:?}"
    );
    assert!(!outcome.is_applied(), "{outcome:?}");
    assert_eq!(
        store_image(&dataset),
        image_before,
        "a stopped WHERE applied the mutation its partial solutions had computed"
    );
    assert!(Arc::ptr_eq(&dataset, &handle_before));
}

/// The intermediate-cell ceiling refuses an UPDATE's `WHERE` whose predicted peak bag
/// breaches it, exactly as it refuses a query's.
///
/// Reported as [`TrippedGovernor::Refused`], which carries an *estimate*: the request never
/// ran, so there is no consumption to report and none is invented.
#[test]
fn an_intermediate_cell_ceiling_refuses_an_update_whose_where_would_breach_it() {
    let update =
        format!("{PREFIX}DELETE {{ ?a ex:p ?b }} WHERE {{ ?a ex:p ?b . ?c ex:p ?d . ?e ex:p ?f }}");
    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(4),
        )
        .expect("a refusal is an outcome, not an update error");

    assert!(
        matches!(
            outcome.tripped(),
            Some(TrippedGovernor::Refused {
                dimension: ResourceDimension::IntermediateCells,
                ..
            })
        ),
        "{outcome:?}"
    );
    assert_eq!(store_image(&dataset), image_before);
}

#[test]
fn an_intermediate_cell_ceiling_bounds_update_staging_before_limit_plus_one() {
    // One one-cell WHERE row fits easily. Its template expands to three quads, so an
    // eight-cell staging ceiling admits two four-position quads and refuses the third
    // before either staging vector grows. UPDATE remains all-or-nothing: even the admitted
    // prefix is not published.
    let update = format!(
        "{PREFIX}INSERT {{
           ex:new ex:a ?value .
           ex:new ex:b ?value .
           ex:new ex:c ?value
         }} WHERE {{ VALUES ?value {{ ex:value }} }}"
    );
    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let image_before = store_image(&dataset);

    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(8),
        )
        .expect("a staging trip is an outcome, not an update error");

    assert_eq!(
        outcome.tripped(),
        Some(TrippedGovernor::Budget {
            dimension: ResourceDimension::IntermediateCells,
            limit: 8,
            consumed: 12,
        })
    );
    assert!(!outcome.is_applied());
    assert_eq!(store_image(&dataset), image_before);
    assert!(Arc::ptr_eq(&dataset, &handle_before));
}

/// The answer cap does not apply to an UPDATE, and is not silently approximated by capping
/// something else.
///
/// `with_max_answers` bounds the answer sequence a caller receives, and a mutation has
/// none. The honest behaviour is that the cap governs nothing here — which is exactly what
/// makes it worth a test, because the tempting alternative is to reinterpret it as a bound
/// on the `WHERE`'s solution rows. That would silently truncate the mutation a caller
/// asked for and report it as applied: the same request under the same cap would delete a
/// different subset depending on a number the caller believed bounded their *result set*.
/// A cap far below the solution count must therefore change nothing at all.
#[test]
fn the_answer_cap_does_not_bound_a_mutation() {
    let update = format!("{PREFIX}DELETE {{ ?s ex:p ?o }} WHERE {{ ?s ex:p ?o }}");

    let (uncapped, uncapped_outcome) = governed(&update, &QueryGovernors::UNBOUNDED);
    assert!(uncapped_outcome.is_applied());

    let (capped, capped_outcome) =
        governed(&update, &QueryGovernors::UNBOUNDED.with_max_answers(1));
    assert!(
        capped_outcome.is_applied(),
        "an answer cap stopped a mutation: {capped_outcome:?}"
    );
    assert_eq!(
        store_image(&capped),
        store_image(&uncapped),
        "an answer cap was reinterpreted as a bound on the WHERE's solutions, so the \
         mutation was truncated"
    );
    assert_ne!(
        store_image(&capped),
        store_image(&fixture()),
        "the request must actually mutate, or the comparison proves nothing"
    );
}

// ── every operation kind honours the governor ────────────────────────────────

/// Every SPARQL UPDATE operation kind that mutates the store charges the governor, and
/// stops on a ceiling it cannot meet without applying anything.
///
/// A table rather than eight tests, because the property is one property and the failure it
/// guards against is a *hole*: a governor accepted at the door and honoured on some
/// operations but not others is worse than no governor, since the caller has been told
/// their mutation is bounded. `CREATE` is deliberately absent — graph existence is implicit
/// here, so it mutates nothing and charging it would be charging for a no-op.
#[test]
fn every_mutating_operation_kind_charges_and_stops() {
    let cases: [(&str, &str); 8] = [
        ("INSERT DATA", "INSERT DATA { ex:a ex:p ex:b }"),
        ("DELETE DATA", "DELETE DATA { ex:s0 ex:p ex:o0 }"),
        (
            "DELETE/INSERT WHERE",
            "DELETE { ?s ex:p ?o } INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }",
        ),
        ("CLEAR", "CLEAR ALL"),
        ("DROP", "DROP GRAPH ex:g"),
        ("ADD", "ADD DEFAULT TO GRAPH ex:target"),
        ("COPY", "COPY DEFAULT TO GRAPH ex:target"),
        ("MOVE", "MOVE DEFAULT TO GRAPH ex:target"),
    ];

    for (kind, body) in cases {
        let update = format!("{PREFIX}{body}");
        let fuel = metered_fuel(&update);
        assert!(
            fuel > 0,
            "{kind} reported zero consumption under METERED, so its governor is dead"
        );

        let engine = NativeSparqlEngine::new();
        let mut dataset = fixture();
        let image_before = store_image(&dataset);
        let handle_before = Arc::clone(&dataset);
        let outcome = engine
            .update_governed(
                &mut dataset,
                request(&update),
                &QueryGovernors::UNBOUNDED.with_fuel(fuel - 1),
            )
            .unwrap_or_else(|error| panic!("{kind} reported a trip as an error: {error:?}"));

        assert!(
            !outcome.is_applied(),
            "{kind} applied under a ceiling one unit below its measured cost: {outcome:?}"
        );
        assert_eq!(
            store_image(&dataset),
            image_before,
            "{kind} left a mutation behind after tripping"
        );
        assert!(
            Arc::ptr_eq(&dataset, &handle_before),
            "{kind} republished the caller's handle after tripping"
        );
    }
}

/// A cancelled request stops at the operation boundary and applies nothing, whichever
/// operation kind it was about to run.
#[test]
fn a_latched_stop_signal_stops_every_operation_kind_before_it_runs() {
    for body in [
        "INSERT DATA { ex:a ex:p ex:b }",
        "DELETE DATA { ex:s0 ex:p ex:o0 }",
        "DELETE { ?s ex:p ?o } WHERE { ?s ex:p ?o }",
        "CLEAR ALL",
        "ADD DEFAULT TO GRAPH ex:target",
        "COPY DEFAULT TO GRAPH ex:target",
        "MOVE DEFAULT TO GRAPH ex:target",
        "CREATE GRAPH ex:target",
    ] {
        let update = format!("{PREFIX}{body}");
        let engine = NativeSparqlEngine::new();
        let mut dataset = fixture();
        let image_before = store_image(&dataset);
        let outcome = engine
            .update_governed(&mut dataset, request(&update), &cancelled_governors())
            .expect("a tripped governor is an outcome");
        assert_eq!(
            outcome.tripped(),
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            }),
            "{body} ran under a latched cancellation"
        );
        assert_eq!(store_image(&dataset), image_before, "{body}");
    }
}

// ── errors stay errors ───────────────────────────────────────────────────────

/// A malformed request is an `Err`, never a budget-exhausted outcome.
///
/// The distinction is the reason the governed surface exists at all: "your budget was too
/// small" tells a caller to raise it and retry, and a request that has no execution must
/// never be dressed up as one.
#[test]
fn a_parse_failure_is_an_error_not_an_exhausted_budget() {
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let error = NativeSparqlEngine::new()
        .update_governed(
            &mut dataset,
            request("INSERT DATA { this is not sparql"),
            &QueryGovernors::METERED,
        )
        .expect_err("a malformed request is an error");
    assert_eq!(error.code, "native-sparql-update-parse");
    assert!(Arc::ptr_eq(&dataset, &handle_before));
}

/// A `LOAD` with no host resolver is an `Err`, under governors exactly as without them.
#[test]
fn a_missing_host_seam_is_an_error_not_an_exhausted_budget() {
    let mut dataset = fixture();
    let handle_before = Arc::clone(&dataset);
    let error = NativeSparqlEngine::new()
        .update_governed(
            &mut dataset,
            request(&format!("{PREFIX}LOAD ex:doc")),
            &QueryGovernors::METERED,
        )
        .expect_err("a LOAD with no resolver is an error");
    assert_eq!(error.code, "native-sparql-load-no-resolver");
    assert!(Arc::ptr_eq(&dataset, &handle_before));
}

// ── RDF 1.2 shapes ───────────────────────────────────────────────────────────

/// A governed UPDATE over RDF 1.2 triple terms and reifiers is governed like any other:
/// the mutation is charged, a ceiling stops it, and nothing is applied.
///
/// Triple terms are a term kind, not a special case, so the interesting assertion is that
/// they are *not* special — the same measured-ceiling-minus-one recipe trips them, and the
/// same byte image survives.
#[test]
fn a_governed_update_over_triple_terms_charges_and_rolls_back() {
    let update = format!(
        "{PREFIX}\
         INSERT DATA {{ ex:r ex:reifies <<( ex:s0 ex:p ex:o0 )>> }} ;\n\
         INSERT {{ ?s ex:saw <<( ?s ex:p ?o )>> }} WHERE {{ ?s ex:p ?o }}"
    );

    let (applied, outcome) = governed(&update, &QueryGovernors::UNBOUNDED);
    assert!(outcome.is_applied(), "{outcome:?}");
    assert_ne!(
        store_image(&applied),
        store_image(&fixture()),
        "the triple-term request must actually mutate"
    );

    let fuel = metered_fuel(&update);
    assert!(
        fuel > u64::try_from(EDGES).expect("the edge count fits"),
        "the mutation charge must count every minted quad, triple terms included"
    );

    let engine = NativeSparqlEngine::new();
    let mut dataset = fixture();
    let image_before = store_image(&dataset);
    let outcome = engine
        .update_governed(
            &mut dataset,
            request(&update),
            &QueryGovernors::UNBOUNDED.with_fuel(fuel - 1),
        )
        .expect("a tripped governor is an outcome");
    assert!(!outcome.is_applied(), "{outcome:?}");
    assert_eq!(store_image(&dataset), image_before);
}
