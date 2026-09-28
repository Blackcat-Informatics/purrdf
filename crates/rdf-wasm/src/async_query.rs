// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The asynchronous operation runtime: governed Rust operations that suspend on host
//! effects through WebAssembly JavaScript Promise Integration (JSPI).
//!
//! # Two lanes, one evaluator
//!
//! The synchronous surface in [`crate::query`] is the offline lane: it installs no
//! `SERVICE` or `LOAD` source and runs to completion inside one wasm call. This module is
//! the second lane. A JavaScript host starts a *job* ([`QueryEngine::begin_async`]), and
//! the job runs the very same operation — [`OperationInput::execute`], no async Rust
//! anywhere — on a private shadow-stack region. The evaluator already performs every
//! effect through three seams it has always had:
//!
//! | Seam | Effect ([`EffectKind`]) | What the host does |
//! |---|---|---|
//! | `ServiceResolver::resolve` (a `SERVICE` clause) | `Service` = 1, or `AwaitExchange` = 5 when another job already asks the same question | answers the forwarded `SELECT` with SPARQL Results JSON, or a typed failure, to the request's shared exchange |
//! | `GraphResolver::resolve` (a `LOAD`) | `Load` = 2 | answers the IRI with a document and its media type, a redirect, or a typed failure |
//! | `StopSignal::poll` (every governor charge point) | `Yield` = 3 | turns the event loop once, then resumes |
//!
//! Kind 4 is reserved for page faults of a paged dataset. The runner itself knows nothing
//! about SPARQL: an [`Operation`] is a closure over a [`JobRun`], and the SPARQL query,
//! update and EXPLAIN kinds are simply its first operations. SHACL validation, change
//! validation, SHACL-AF entailment and product validation ride the same runner, the same
//! effects and the same import as the SHACL kind (see [`execute_shacl`]); entailment
//! closure and large parses can too.
//!
//! Each effect call writes a *ticket* (a sequence number and a payload) into the job's
//! slots and calls the one suspending import, `purrdf_jspi_suspend(job, ticket, out)`,
//! from `./purrdf_jspi.mjs`. On a JSPI host that import is a `WebAssembly.Suspending`
//! function: the wasm stack is captured as a one-shot continuation, the host takes the
//! ticket ([`AsyncJob::take_effect`]), awaits whatever it likes, delivers an answer
//! ([`AsyncJob::deliver_graph`], [`AsyncJob::deliver_exchange_bindings`] and siblings)
//! and resumes the continuation. The job's
//! entry point is the raw export `purrdf_jspi_run(job)`, driven by
//! `WebAssembly.promising`. On a host without JSPI the synchronous API is untouched and
//! the package root refuses the asynchronous methods before touching wasm.
//!
//! # Stack regions and the `suspend` epilogue invariant
//!
//! A suspended job's frames stay live in linear memory, so two jobs cannot share the one
//! shadow stack. Every job owns a heap-allocated region ([`AsyncJob::stack_top`] /
//! [`AsyncJob::stack_base`], 16-byte aligned, a canary word [`AsyncJob::stack_canary`] at
//! the base). The package's post-link step, `wasm-link`, wraps the job runner so it sets
//! the stack pointer to the job's top before the job's first frame and restores the idle
//! value after the run returns, and routes every call of the suspending import through
//! one linked function that parks the job on the idle pointer for the duration of the
//! import.
//!
//! Parking is not enough on its own: between one job's promise settling and its
//! resumption reaction running, another job's continuation may run and move the global.
//! So the resumed job must put its *own* stack pointer back before anything can allocate
//! a frame. The linked function guarantees that by construction: it saves the frame's
//! stack pointer in a local before the import and writes it back the instruction after
//! the import returns, so the restore is the first stack-relevant thing that happens on
//! resumption, whatever `wasm-opt` inlined. [`suspend`] is a plain call of the import; the
//! linker refuses a module in which the import is reached any other way.
//!
//! Inside the region two guards read one measurement. The run installs the region's base
//! as the stack floor the evaluator's guards measure against, with no walk
//! scope open (`purrdf_stack::replace_context`, which swaps the floor and the evaluator's
//! walk-scope state together), puts the context's own back before every suspension,
//! reinstalls the job's on every resumption (recording the resuming context's as the one
//! to put back next), and puts the context's own back when the run returns — so every
//! stack measurement on the job is against its own region, every one made while it is
//! suspended is against the stack actually running, and a walk scope the job suspends
//! inside (a property path's traversal polls, and so yields) stays the job's: nothing
//! that runs while it waits latches a refusal in it, and the jobs close their scopes in
//! whatever order they are resumed. The evaluator checks that measurement at every
//! recursive step — an operator chain, a nested `EXISTS` walk, a property path's
//! traversal — and refuses with its own typed error
//! (`native-sparql-evaluation-stack-exhausted`) while
//! `purrdf_stack::MARGIN_BYTES` (64 KiB) are still left: the very refusal the synchronous
//! twin gives, with the region's size and `stackBytes` named as the remedy. The
//! [`JspiStopWatch`] is the last resort beneath them, for polling frames no check guards:
//! every poll measures how far its frame is above the region's base and, inside a guard
//! band of half the margin ([`STACK_GUARD_BYTES`]), latches a fault ("asynchronous job
//! stack region exhausted …; raise stackBytes") and stops the job cleanly — before a
//! deeper frame could run off the region into the heap. The band lies below the point
//! every check refuses at, so a frame that checks is always refused, typed, before a poll
//! could see the band; and the band is measured from the base itself, not from the
//! floor, so neither the stack an evaluation reserves above the floor nor the exhausted
//! floor a refusing walk installs while it unwinds turns the evaluator's own refusal
//! into the region's fault. The poll also records the low-water mark, reported as
//! [`AsyncEvidence::stack_high_water_bytes`].
//!
//! Work that neither polls nor is the evaluator's can still recurse past the base between
//! two polls. Beneath every base lies an overrun zone
//! ([`STACK_OVERRUN_ZONE_BYTES`], filled with a known byte) that absorbs such frames
//! inside the job's own allocation. The first poll after one sees the canary it
//! overwrote and stops the job with the same typed exhaustion error; when the run
//! returns, the zone is inspected: an overrun that stayed above its floor
//! ([`STACK_OVERRUN_FLOOR_BYTES`]) is that typed error, and one that reached the floor
//! may have left the allocation, so the run reports status 4 and the host poisons the
//! instance rather than let it run on over memory that may be corrupt. The zone is as
//! large as the synchronous lane's whole stack, so any request the synchronous lane runs
//! without trapping ends here answered or typed, never in corruption.
//!
//! # Faults, statuses, and the rule that nothing crosses a suspended frame
//!
//! A panic or a JavaScript exception that unwinds through a suspended frame bricks the
//! instance. So nothing on the effect path throws or panics: every [`AsyncJob`] method a
//! host calls while a job is suspended returns a status, and every condition that is not
//! an ordinary answer — a malformed delivery, a sequence number that was never issued,
//! stack exhaustion, a host that returned without delivering — is *latched* as the job's
//! fault ([`AsyncJob::fault`]). A host handler that fails to answer one effect (it threw,
//! rejected, returned an unrecognized value or named an unknown failure kind) is not a
//! job fault: it is that effect's failure, delivered as `FailureKind::Fault`, which the
//! evaluator treats as the invocation failing — `SERVICE SILENT` and `LOAD SILENT` answer
//! it as they answer any failed invocation, and a loud clause reports the host's fault. A latched fault makes the job's
//! stop signal fire, the evaluator winds down through its ordinary governor path, and the
//! run stores the fault as the job's error — discarding whatever outcome it reached, and
//! never committing an update. The host's own trap handling (poisoning the instance) is
//! the only answer to a genuine trap. Nothing on this side repairs one: a trap unwinds
//! past the run's restore of the caller's stack context and leaves whatever `RefCell`
//! its frames borrowed still borrowed, so the host bars every entry point of the
//! instance — synchronous ones and objects created before the trap included — for the
//! rest of the JavaScript realm's life.
//!
//! The statuses are exported enums the host reads by name: [`SuspendStatus`] is what
//! `purrdf_jspi_suspend` returns, [`DeliveryStatus`] what every delivery (and `fault` and
//! `cancel`) returns, and [`RunStatus`] what `purrdf_jspi_run` returns.
//!
//! A failure is never flattened into its words: the job keeps the diagnostic
//! ([`crate::operation::JobError`]), and the host reads its code
//! ([`AsyncJob::error_code`]) beside its message.
//!
//! # The failure taxonomy, and the inherited `SILENT` table
//!
//! A host answers a `SERVICE` effect's exchange with bindings, `{kind: "transport"}` or
//! `{kind: "denied"}`, or abandons the effect on the job's signal or at its abandonment
//! instant ([`AsyncJob::expire_effect`]). After resuming, the transport observes the job's
//! stop signal **first**:
//!
//! | After resuming | Returned to the evaluator |
//! |---|---|
//! | signal fired, nothing delivered | `RemoteError::Governed` — the exchange was abandoned, the positional prefix survives |
//! | signal fired, bindings delivered | `RemoteError::GovernedAfterCompletion` — a completed response discarded |
//! | bindings | the SPARQL Results JSON, decoded (bounded by the cell ceiling) natively |
//! | transport failure | `RemoteError::Transport` |
//! | denied | `RemoteError::HostDenied` (the host's own policy, no catalog capability named) — a catalog capability denial never reaches this delivery: it is decided natively before the host is ever asked, and surfaces as `RemoteError::Denied` instead |
//! | nothing delivered | a latched fault ("resolver returned without a delivery") |
//!
//! From there the evaluator's own contract decides (see `purrdf_sparql_eval::service`):
//! `SERVICE SILENT` answers every failed invocation — a transport or decode failure, a
//! denial, an endpoint no handler reaches — with the join identity, and records it on
//! [`AsyncEvidence::silenced`]; a governor trip is never silenced, and neither is a fault,
//! because a fault is not an answer at all. `silent` is handed to the host for
//! information only; an empty answer is not the host's to invent.
//!
//! A `LOAD` source is authorized against the job's catalog before any effect is issued
//! (see [`JspiGraphResolver`]): a refusal is the catalog's [`LoadError::Denied`]
//! (`native-sparql-load-denied`). A `LOAD` effect answered with a transport failure
//! becomes [`LoadError::Transport`] (`native-sparql-load-failed`), a document that does
//! not parse [`LoadError::Decode`] (`native-sparql-load-decode`), and a denial the host
//! delivers [`LoadError::HostDenied`] (`native-sparql-load-host-denied`), whose message
//! reads "the host denied the request: …". A redirect is followed here: its location is
//! re-authorized and fetched as a fresh effect, up to [`MAX_LOAD_REDIRECTS`] hops. Each
//! failure fails the request, and `LOAD SILENT` succeeds over it with nothing loaded,
//! recorded on [`AsyncEvidence::silenced`]. A host fault is latched and fails the job,
//! `SILENT` or not.
//!
//! # Shared `SERVICE` exchanges and the per-job memo
//!
//! A job asks the host at most once for a request it repeats: a successful answer is kept
//! in the job's memo, keyed by the whole request ([`ExchangeKey`]), and a repeat is
//! answered from it with no effect. A failure is never kept. Concurrent jobs share one
//! host call: a job whose request matches an open exchange with a deadline at or after its
//! own waits on it (an `AwaitExchange` effect) rather than asking again; the answer is
//! delivered to the exchange and reaches every job still waiting; a job stopped while it
//! waits leaves, and when the last one leaves the exchange closes and the host aborts its
//! call ([`AsyncJob::exchange_is_open`]).
//!
//! # Abandonment instants
//!
//! Every `SERVICE` and `LOAD` effect carries [`AsyncEffect::abandon_after_ms`]: the
//! request's own timeout (its catalog profile's, or the default) or the job's remaining
//! deadline, whichever falls first. The host abandons an effect still unanswered then
//! ([`AsyncJob::expire_effect`]): at the deadline the effect is abandoned as the
//! deadline's trip; at the timeout it fails as a transport failure `SILENT` absorbs. The
//! deadline instant itself is the job's, read off the clock in Rust.
//!
//! # Yielding: poll-count slicing, and where it happens
//!
//! Every governor poll is counted. Every `yieldEveryPolls` polls (default 65 536; `0`
//! yields at every poll) the watch suspends with a `Yield` ticket and the host turns the
//! event loop once, then resumes. The clock is read only every 1 024 polls and at slice
//! boundaries. Slicing is by count, never by elapsed time, because **on Cloudflare
//! Workers `Date.now()` does not advance during CPU-bound execution** (it advances after
//! I/O): a wall-clock slice would never fire there, and for the same reason a synchronous
//! `deadlineMs` cannot trip mid-evaluation on Workers. The asynchronous lane observes the
//! deadline at every yield and every effect, and an effect still awaited when the
//! deadline falls is abandoned by the host at that instant ([`AsyncJob::expire_effect`]).
//!
//! Yielding happens during *evaluation* only — including an entailment closure, whose
//! reasoner polls the same signal. Freezing the dataset before the job and serializing
//! its result after are linear passes that issue no effects; they run to completion, and
//! [`AsyncEvidence`] reports each phase's time so a host can see them.
//!
//! # Interleaving audit
//!
//! Jobs interleave at every suspension, and a synchronous call on the same
//! [`QueryEngine`] may run while a job is suspended. Anything shared that a job held
//! across a poll or a resolver call would therefore be observed half-held by the next
//! caller — a `RefCell` double borrow panics, and on `wasm32-unknown-unknown` (no
//! threads) a contended `std::sync::Mutex` panics too — and per-thread state a job left
//! set would be read by the next caller as its own.
//!
//! Every `thread_local!` in the workspace is listed in [`crate::interleaving`] with the
//! reason it is safe: the job swaps it as part of its [`JobAmbient`] (the stack context
//! and the SHACL engine's ambient scopes, moved as one value at the run's start, at every
//! suspension, at every resumption and when the run returns), it is borrowed only inside
//! one call that neither polls nor suspends, it is never written while an evaluation
//! runs, or it is not compiled into this package. `scripts/check-thread-locals.py` keeps
//! that ledger equal to the source in both directions. The evaluator itself keeps no
//! per-thread evaluation flag: whether an operation runs its row loops sequentially is a
//! field of its evaluation context, set per evaluation.
//!
//! The shared state that is not a thread-local, and why none of it is observed across a
//! suspension:
//!
//! | State | Held across a poll or resolver call? |
//! |---|---|
//! | `NativeSparqlEngine`'s plan cache (`RefCell<PlanCache>`) | No. Every borrow (`prepare_for`, `prepare_request`, `bind_functions`, `prepare_execution`, the stats accessors) is a temporary inside one statement that parses and admits a plan; planning polls no signal and calls no resolver, and the borrow ends before the returned `Arc<PreparedQuery>` is evaluated. The `engine_is_reentrant_from_a_resolver_and_from_a_poll` test below re-enters one engine from inside its own evaluation to prove it. |
//! | The BGP join-order cache (`Mutex`, `bgp::order_for`) | No. Locked only around the `get` and the `insert`; the cost-based ordering between them runs unlocked and polls nothing. |
//! | `GovernorState` (`poll_stop`, `trip`) | No. It polls the signal *before* touching its `OnceLock`, never inside the initializer. |
//! | Lazily built dataset indexes (`OnceLock` permutations, predecessor and value indexes, path-relation adjacency) | No. Their initializers are pure sorts and scans that poll nothing, so no initialization can be suspended half-done. |
//! | Reasoner state (`purrdf-entail`'s tableau `RefCell`s, the datalog engines) | No sharing: every closure run owns its own. |
//! | This module's own [`JobSlots`] mutexes | No. Each is locked inside one helper that returns owned values, and none is held when [`suspend`] is called or while an exchange's answer is delivered. |

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use purrdf::{JsonLdSerializeOptions, RdfDataset, parse_dataset};
use purrdf_core::SparqlResult;
use purrdf_sparql_eval::protocol::FailureCode;
use purrdf_sparql_eval::remote_http::DEFAULT_TIMEOUT;
use purrdf_sparql_eval::{
    CancellationFlag, GovernorState, GraphResolveRequest, GraphResolver, HttpRemoteQuerySource,
    HttpRequest, HttpTransport, InProcessServiceResolver, LoadError, NativeSparqlEngine,
    QueryGovernors, RemoteError, ResolvedBindings, ServiceCapabilities, ServiceCapability,
    ServiceCatalog as NativeServiceCatalog, ServiceCredential, ServiceProfile, ServiceRequest,
    ServiceResolver, StopCause, StopSignal, TrippedGovernor, WallDeadline,
};
use purrdf_sparql_results::ProvenanceNamespace;
use serde::Deserialize;
use wasm_bindgen::convert::TryFromJsValue;
use wasm_bindgen::prelude::*;

use crate::codec::resolve_media_type;
use crate::dataset::{Dataset, UpdateClaim};
use crate::jsonld::{CompiledJsonLdContext, context_options, decode_options};
use crate::operation::{
    JobError, JobOutcome, JobRun, Lane, OPTIONS_CODE, OperationInput, SHACL_CODE,
    SHACL_REFUSAL_CODE, USAGE_CODE, coded_error, coded_type_error,
};
use crate::query::{
    EntailmentQueryOutcome, GovernorArgs, NegotiatedOutcome, QueryEngine, QueryOutcome,
    QueryResult, UPDATE_REFUSES_MAX_ANSWERS, UpdateOutcome, entailment_query_outcome_from_native,
    kind_mismatch, negotiated_outcome_from_value, query_outcome_from_governed,
    query_result_from_sparql, update_outcome_from_governed,
};
use crate::shacl::{
    ShaclChangeValidation, ShaclProductRefusal, entail_to_ntriples_impl,
    product_validate_expecting_impl, product_validate_impl,
    product_validate_rebuild_expecting_impl, product_validate_rebuild_impl,
    validate_changes_to_sarif_impl, validate_to_sarif_impl,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Polls between yields when the caller names no `yieldEveryPolls`.
const DEFAULT_YIELD_EVERY_POLLS: u32 = 65_536;

/// A job's stack region when the caller names no `stackBytes`: 2 MiB.
const DEFAULT_STACK_BYTES: u32 = 2 * 1024 * 1024;

/// The smallest region a job may run on: 512 KiB, so a job always has at least 448 KiB
/// of stack above the point the evaluator's checks refuse at.
const MIN_STACK_BYTES: u32 = 512 * 1024;

/// The guard band at the base of a region: half the margin the evaluator's checks refuse
/// at. A poll whose frame reaches it stops the job — with the region's
/// fault — before a deeper frame can leave the region.
///
/// It must lie below the margin. Every check measures against the region's base (the
/// floor [`JobInner::run`] installs, raised by whatever the evaluation reserved) and
/// refuses once less than [`purrdf_stack::MARGIN_BYTES`] are left, so a frame that
/// checks is refused with the evaluator's own typed error — the refusal
/// its synchronous twin gives — at least `MARGIN_BYTES - STACK_GUARD_BYTES` above the
/// band. Only frames that poll but never check can reach the band; a band wider than the
/// margin would pre-empt every typed refusal with this fault instead.
const STACK_GUARD_BYTES: usize = purrdf_stack::MARGIN_BYTES / 2;

const _: () = assert!(
    STACK_GUARD_BYTES < purrdf_stack::MARGIN_BYTES,
    "the guard band must lie below the point every stack check refuses at"
);

/// The overrun zone below every region's base: 1 MiB.
///
/// The guard band only stops frames that *poll*, and the evaluator's own stack guards only
/// frames that evaluate. Work that is neither can recurse past the region's base, and
/// below the base lies other heap memory: an overrun there would corrupt it silently. So
/// every region is allocated with this many bytes beneath its base, filled with
/// [`STACK_ZONE_FILL`], and inspected when the run returns (see [`StackRegion::overrun`]):
/// an overrun that stayed above the zone's floor ([`STACK_OVERRUN_FLOOR_BYTES`]) touched
/// nothing but the job's own allocation, and fails the job with the typed exhaustion
/// error; one that reached the floor may have left the allocation, and poisons the
/// instance.
///
/// Sized from the synchronous lane: its whole shadow stack is the linker's 1 MiB, so the
/// non-polling frames of any request it runs without trapping fit in 1 MiB, and on a
/// region of at least [`MIN_STACK_BYTES`] they overrun the base by at most 512 KiB plus
/// the runner's own few frames — well above the floor. A request the synchronous lane
/// can run is therefore never corruption here: it answers, or fails typed.
const STACK_OVERRUN_ZONE_BYTES: usize = 1024 * 1024;

/// The lowest part of the overrun zone. A byte changed here means the frames may have
/// run past the whole zone — out of the job's allocation — so the instance is poisoned.
const STACK_OVERRUN_FLOOR_BYTES: usize = 256 * 1024;

/// The byte the overrun zone is filled with. Not zero: frames routinely write zeros, and
/// an overrun that wrote only the fill would go unseen.
const STACK_ZONE_FILL: u8 = 0xA5;

/// Work between clock reads for the wall deadline, counted as polls are (see
/// [`JspiStopWatch::poll_after_work`]).
const CLOCK_EVERY_POLLS: u64 = 1_024;

/// The word written at the base of every region, checked by the host at every
/// suspension. ASCII `PurD`, little-endian.
const STACK_CANARY: u32 = 0x4472_7550;

/// The most redirect hops one `LOAD` follows: generous enough for a real document
/// mirror, small enough that a redirect loop fails fast. A redirect past it fails the
/// `LOAD` as a transport failure.
pub(crate) const MAX_LOAD_REDIRECTS: u32 = 5;

// ---------------------------------------------------------------------------
// The protocol's statuses
// ---------------------------------------------------------------------------

/// What `purrdf_jspi_run` reports.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// The job's outcome is stored.
    Outcome = 0,
    /// The job's error is stored.
    Error = 1,
    /// No job has this id.
    UnknownJob = 2,
    /// The job had already been started.
    AlreadyStarted = 3,
    /// The job's frames ran past its region's overrun zone. Memory outside the job's
    /// allocation may have been overwritten, so the host must poison the instance; the
    /// job's error names what happened.
    Overran = 4,
}

/// What `purrdf_jspi_suspend` reports when the host resumes the job.
#[wasm_bindgen]
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendStatus {
    /// The effect was answered.
    Answered = 0,
    /// The host abandoned the effect on the job's stop signal.
    Abandoned = 1,
    /// A fault is latched on the job.
    Fault = 2,
}

/// What a delivery (and `fault`, `cancel`) reports.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    /// The delivery answers the outstanding effect.
    Accepted = 0,
    /// The delivery named an effect that was already answered or abandoned — typically a
    /// host promise settling after the job's signal won the race. Ignored; not a fault.
    Stale = 1,
    /// The delivery was malformed (a sequence number never issued, a payload the
    /// outstanding effect cannot take). The job's fault is latched.
    Fault = 2,
    /// The job (or the shared exchange) has already finished; the delivery changes
    /// nothing.
    Finished = 3,
}

// ---------------------------------------------------------------------------
// The raw JSPI import
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "./purrdf_jspi.mjs")]
unsafe extern "C" {
    /// Suspending on JSPI hosts. Returns a [`SuspendStatus`]. The host writes `0` to
    /// `*out` before returning.
    fn purrdf_jspi_suspend(job: u32, ticket: u32, out: *mut u32) -> u32;
}

/// Suspend the running job on its outstanding `ticket`, returning the host's status.
///
/// In the shipped module this call never reaches the import directly: the package's
/// post-link step (`crates/wasm-link`) redirects every call of the import to an injected
/// `$suspend`, which parks this frame's stack pointer in a wasm local, switches the
/// stack pointer to the idle context for the host, and writes the parked pointer back
/// before control returns here. The restore is therefore the linked module's own code,
/// and nothing about this function's shape, frame or inlining carries it.
#[cfg(target_arch = "wasm32")]
fn suspend(job: u32, ticket: u32) -> u32 {
    let mut out = 0u32;
    // SAFETY: `out` is a live, writable word for the whole call, and the host writes only
    // that word. The import has no other preconditions.
    unsafe { purrdf_jspi_suspend(job, ticket, &raw mut out) }
}

/// The JSPI lane exists only on `wasm32`. Nothing on the native build issues an effect:
/// the native tests run operations that complete inside one yield quantum and never
/// deliver through a real suspension.
#[cfg(not(target_arch = "wasm32"))]
fn suspend(_job: u32, _ticket: u32) -> u32 {
    unreachable!("the JSPI lane is wasm32-only")
}

/// Run job `job` on the stack region the host has just switched to.
///
/// The raw export `WebAssembly.promising` drives. Returns a [`RunStatus`].
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_jspi_run(job: u32) -> u32 {
    run_job(job) as u32
}

/// The target-independent body of `purrdf_jspi_run`.
///
/// Reached only through that `wasm32` export (and the native unit tests), so the native
/// library build sees the whole runner as unused.
#[cfg_attr(
    not(target_arch = "wasm32"),
    allow(
        dead_code,
        reason = "the JSPI runner is entered only through the wasm32 export"
    )
)]
fn run_job(id: u32) -> RunStatus {
    // The registry's own `Rc` keeps the job (and so the region this call is running on)
    // alive for the whole run; `finish` refuses to drop it while the job is running.
    let Some(job) = lookup_job(id) else {
        return RunStatus::UnknownJob;
    };
    job.run()
}

// ---------------------------------------------------------------------------
// The clock
// ---------------------------------------------------------------------------

/// Milliseconds since the Unix epoch — the host clock the evidence and the deadline's
/// remaining budget are read from.
#[cfg(target_arch = "wasm32")]
pub(crate) fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Milliseconds since the Unix epoch — the host clock the evidence and the deadline's
/// remaining budget are read from.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64() * 1000.0)
}

/// Lock `mutex`. A poisoned lock means a holder panicked with the job's state half
/// changed; nothing here can reason about that state, so it is a panic too.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .expect("an asynchronous job's state lock is poisoned: a holder panicked mid-update")
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

/// What an effect asks the host to do. The numbering is the protocol's; 4 is reserved
/// for paged datasets.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    /// Answer a forwarded `SERVICE` query: the host calls its `resolveService` once and
    /// delivers the answer to the effect's shared exchange ([`AsyncJob::deliver_exchange_bindings`]
    /// and siblings), which every job waiting on it receives.
    Service = 1,
    /// Fetch a `LOAD` document.
    Load = 2,
    /// Turn the event loop once and resume.
    Yield = 3,
    /// Wait for a `SERVICE` exchange another job opened, whose answer this job receives
    /// when that exchange is delivered.
    AwaitExchange = 5,
}

/// Why an outstanding effect's abandonment instant falls when it does: the job's deadline,
/// or the request's own timeout. See [`AsyncJob::expire_effect`].
#[derive(Debug, Clone, Copy, PartialEq)]
enum Abandon {
    /// The job's deadline falls first: abandoning the effect is the deadline's trip.
    Deadline,
    /// The request's own timeout of this many milliseconds falls first: abandoning the
    /// effect is the request's transport failure.
    Timeout(f64),
}

/// A `SERVICE` effect's request, as the SPARQL Protocol POST the host should issue.
#[derive(Clone)]
struct ServiceEffect {
    endpoint: String,
    query_text: String,
    user_agent: String,
    timeout_ms: f64,
    content_type: String,
    accept: String,
    /// Profile headers then the credential header, in order. May carry a secret.
    headers: Vec<(String, String)>,
    silent: bool,
    max_intermediate_cells: Option<u64>,
    remaining_deadline_ms: Option<f64>,
    /// Whether a shared cache may answer or keep this request's answer: `false` when it
    /// carries a credential.
    cacheable: bool,
}

// Hand-written so a header value — possibly a credential — never reaches a log.
impl fmt::Debug for ServiceEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header_names: Vec<&str> = self.headers.iter().map(|(name, _)| name.as_str()).collect();
        f.debug_struct("ServiceEffect")
            .field("endpoint", &self.endpoint)
            .field("query_text", &self.query_text)
            .field("user_agent", &self.user_agent)
            .field("timeout_ms", &self.timeout_ms)
            .field("headers", &header_names)
            .field("silent", &self.silent)
            .field("max_intermediate_cells", &self.max_intermediate_cells)
            .field("remaining_deadline_ms", &self.remaining_deadline_ms)
            .field("cacheable", &self.cacheable)
            .finish_non_exhaustive()
    }
}

/// A `LOAD` effect's request, as the `GET` the host should issue — one hop: a redirect
/// is delivered back ([`AsyncJob::deliver_redirect`]) and becomes a fresh effect.
#[derive(Clone)]
struct LoadEffect {
    iri: String,
    accept: String,
    user_agent: Option<String>,
    /// The catalog profile's headers then its credential header, in order. May carry a
    /// secret.
    headers: Vec<(String, String)>,
    timeout_ms: f64,
}

// Hand-written so a header value — possibly a credential — never reaches a log.
impl fmt::Debug for LoadEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header_names: Vec<&str> = self.headers.iter().map(|(name, _)| name.as_str()).collect();
        f.debug_struct("LoadEffect")
            .field("iri", &self.iri)
            .field("user_agent", &self.user_agent)
            .field("headers", &header_names)
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}

/// An effect's payload.
#[derive(Debug, Clone)]
enum EffectPayload {
    /// A `SERVICE` request that opens exchange `exchange`.
    Service {
        effect: Box<ServiceEffect>,
        exchange: u64,
    },
    Load(Box<LoadEffect>),
    Yield,
    /// A wait on the open exchange `exchange`.
    AwaitExchange {
        exchange: u64,
    },
}

impl EffectPayload {
    const fn kind(&self) -> EffectKind {
        match self {
            Self::Service { .. } => EffectKind::Service,
            Self::Load(_) => EffectKind::Load,
            Self::Yield => EffectKind::Yield,
            Self::AwaitExchange { .. } => EffectKind::AwaitExchange,
        }
    }
}

/// How an effect failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureKind {
    /// The endpoint or document could not be reached or read.
    Transport,
    /// The host's policy refused the request.
    Denied,
    /// The `LOAD` document arrived and could not be parsed. Decided here, never
    /// delivered by name.
    Decode,
    /// The host's handler failed to answer the effect: it threw, rejected, returned a
    /// value the protocol does not define, or named a failure kind it does not define.
    /// The invocation failed through the host's own defect, not the endpoint's.
    Fault,
}

impl FailureKind {
    /// The delivery for a failure the host named `kind`, with `message`: `"transport"`,
    /// `"denied"` and `"fault"` are the kinds they name; any other name is itself a fault
    /// of the handler, delivered as one and naming the unknown kind for `target`.
    fn delivered(kind: &str, message: String, target: &str) -> Delivered {
        let kind = match kind {
            "transport" => Self::Transport,
            "denied" => Self::Denied,
            "fault" => Self::Fault,
            unknown => {
                return Delivered::Failure {
                    kind: Self::Fault,
                    message: unknown_failure_kind(unknown, target),
                };
            }
        };
        Delivered::Failure { kind, message }
    }
}

/// What the host delivered for an effect.
#[derive(Debug, Clone)]
enum Delivered {
    /// SPARQL Results JSON bytes for a `SERVICE` effect (or a wait on its exchange).
    Bindings(Arc<[u8]>),
    /// A parsed document for a `LOAD` effect.
    Graph(Arc<RdfDataset>),
    /// A redirect for a `LOAD` effect: the `Location` the host was told to go to.
    Redirect(String),
    /// A typed failure.
    Failure { kind: FailureKind, message: String },
    /// The host abandoned the effect on the job's stop signal.
    Governed,
}

impl Delivered {
    /// Whether an outstanding effect of `kind` can take this delivery.
    const fn answers(&self, kind: EffectKind) -> bool {
        matches!(
            (self, kind),
            (
                Self::Bindings(_),
                EffectKind::Service | EffectKind::AwaitExchange
            ) | (Self::Graph(_) | Self::Redirect(_), EffectKind::Load)
                | (
                    Self::Failure { .. } | Self::Governed,
                    EffectKind::Service | EffectKind::Load | EffectKind::AwaitExchange
                )
        )
    }

    const fn label(&self) -> &'static str {
        match self {
            Self::Bindings(_) => "bindings",
            Self::Graph(_) => "a graph",
            Self::Redirect(_) => "a redirect",
            Self::Failure { .. } => "a failure",
            Self::Governed => "an abandonment",
        }
    }
}

/// One effect the host has taken, as JavaScript sees it.
#[wasm_bindgen]
pub struct AsyncEffect {
    seq: u32,
    payload: EffectPayload,
    abandon_after_ms: Option<f64>,
}

// The payload's own `Debug` already redacts header values.
impl fmt::Debug for AsyncEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncEffect")
            .field("seq", &self.seq)
            .field("payload", &self.payload)
            .field("abandon_after_ms", &self.abandon_after_ms)
            .finish()
    }
}

impl AsyncEffect {
    const fn service(&self) -> Option<&ServiceEffect> {
        match &self.payload {
            EffectPayload::Service { effect, .. } => Some(effect),
            _ => None,
        }
    }

    const fn load(&self) -> Option<&LoadEffect> {
        match &self.payload {
            EffectPayload::Load(effect) => Some(effect),
            _ => None,
        }
    }
}

/// Flatten `(name, value)` pairs into `[name, value, name, value, …]`.
fn flatten_headers(headers: &[(String, String)]) -> Vec<String> {
    headers
        .iter()
        .flat_map(|(name, value)| [name.clone(), value.clone()])
        .collect()
}

#[wasm_bindgen]
impl AsyncEffect {
    /// The sequence number every delivery for this effect must name.
    #[wasm_bindgen(getter)]
    pub fn seq(&self) -> u32 {
        self.seq
    }

    /// What the effect asks for.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> EffectKind {
        self.payload.kind()
    }

    /// When the host abandons the effect if it is still unanswered, in milliseconds from
    /// now: the request's own timeout or the job's remaining deadline, whichever is
    /// sooner. The host calls [`AsyncJob::expire_effect`] when it passes. `undefined` for
    /// a yield.
    #[wasm_bindgen(getter, js_name = abandonAfterMs)]
    pub fn abandon_after_ms(&self) -> Option<f64> {
        self.abandon_after_ms
    }

    /// `SERVICE` and a wait on an exchange: the shared exchange the answer is delivered
    /// to.
    #[wasm_bindgen(getter, js_name = exchangeId)]
    pub fn exchange_id(&self) -> Option<f64> {
        match &self.payload {
            EffectPayload::Service { exchange, .. } | EffectPayload::AwaitExchange { exchange } => {
                Some(*exchange as f64)
            }
            _ => None,
        }
    }

    /// `SERVICE`: the endpoint IRI exactly as the query wrote it.
    #[wasm_bindgen(getter)]
    pub fn endpoint(&self) -> Option<String> {
        self.service().map(|effect| effect.endpoint.clone())
    }

    /// `SERVICE`: the complete forwarded `SELECT * WHERE { … }` text — the request body.
    #[wasm_bindgen(getter, js_name = queryText)]
    pub fn query_text(&self) -> Option<String> {
        self.service().map(|effect| effect.query_text.clone())
    }

    /// `SERVICE`: whether the clause was written `SERVICE SILENT`. Informational only:
    /// an empty answer is never the host's to invent because of it.
    #[wasm_bindgen(getter)]
    pub fn silent(&self) -> bool {
        self.service().is_some_and(|effect| effect.silent)
    }

    /// `SERVICE`: whether a shared cache may answer the request or keep its answer —
    /// `false` when it carries a credential, whose answer belongs to the credential's
    /// holder.
    #[wasm_bindgen(getter)]
    pub fn cacheable(&self) -> bool {
        self.service().is_some_and(|effect| effect.cacheable)
    }

    /// `SERVICE`: the `Accept` header value (`application/sparql-results+json`); `LOAD`:
    /// the media type of every RDF syntax a `LOAD` parses.
    #[wasm_bindgen(getter)]
    pub fn accept(&self) -> Option<String> {
        self.service()
            .map(|effect| effect.accept.clone())
            .or_else(|| self.load().map(|effect| effect.accept.clone()))
    }

    /// `SERVICE`: the `Content-Type` header value (`application/sparql-query`).
    #[wasm_bindgen(getter, js_name = contentType)]
    pub fn content_type(&self) -> Option<String> {
        self.service().map(|effect| effect.content_type.clone())
    }

    /// `SERVICE`: the `User-Agent` the service's profile names, or the engine default;
    /// `LOAD`: the one the source's catalog profile names, if it names one.
    #[wasm_bindgen(getter, js_name = userAgent)]
    pub fn user_agent(&self) -> Option<String> {
        self.service()
            .map(|effect| effect.user_agent.clone())
            .or_else(|| self.load().and_then(|effect| effect.user_agent.clone()))
    }

    /// `SERVICE` and `LOAD`: the per-request timeout the catalog profile names, or the
    /// default, in milliseconds.
    #[wasm_bindgen(getter, js_name = timeoutMs)]
    pub fn timeout_ms(&self) -> Option<f64> {
        self.service()
            .map(|effect| effect.timeout_ms)
            .or_else(|| self.load().map(|effect| effect.timeout_ms))
    }

    /// `SERVICE` and `LOAD`: the extra request headers as flattened `[name, value, name,
    /// value, …]` pairs — the catalog profile's headers, then its credential header — in
    /// the order they must be sent. Append each pair; a repeated name is legal and must
    /// not be merged. Empty when no catalog is configured.
    pub fn headers(&self) -> Vec<String> {
        match &self.payload {
            EffectPayload::Service { effect, .. } => flatten_headers(&effect.headers),
            EffectPayload::Load(effect) => flatten_headers(&effect.headers),
            EffectPayload::Yield | EffectPayload::AwaitExchange { .. } => Vec::new(),
        }
    }

    /// `SERVICE`: the query's inclusive intermediate-cell ceiling, when the caller
    /// actually configured one. `undefined` on an ungoverned query and on a governed one
    /// that set no cell ceiling — never the metering bookkeeping sentinel a caller never
    /// asked for.
    #[wasm_bindgen(getter, js_name = maxIntermediateCells)]
    pub fn max_intermediate_cells(&self) -> Option<u64> {
        self.service()
            .and_then(|effect| effect.max_intermediate_cells)
    }

    /// `SERVICE`: milliseconds left before the job's deadline, when one is set.
    #[wasm_bindgen(getter, js_name = remainingDeadlineMs)]
    pub fn remaining_deadline_ms(&self) -> Option<f64> {
        self.service()
            .and_then(|effect| effect.remaining_deadline_ms)
    }

    /// `LOAD`: the IRI of this hop — the source, or the location a redirect named.
    #[wasm_bindgen(getter)]
    pub fn iri(&self) -> Option<String> {
        self.load().map(|effect| effect.iri.clone())
    }
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

/// Add `ms` to an `f64` accumulated in an atomic's bits.
pub(crate) fn add_ms(cell: &AtomicU64, ms: f64) {
    let ms = if ms.is_finite() { ms.max(0.0) } else { 0.0 };
    // A single-threaded instance never contends; the loop is the plain CAS shape.
    let _ = cell.try_update(Ordering::Relaxed, Ordering::Relaxed, |bits| {
        Some((f64::from_bits(bits) + ms).to_bits())
    });
}

fn read_ms(cell: &AtomicU64) -> f64 {
    f64::from_bits(cell.load(Ordering::Relaxed))
}

/// A job's evidence counters, written as it runs.
#[derive(Debug)]
pub(crate) struct AsyncCounters {
    polls: AtomicU64,
    yields: AtomicU64,
    service_effects: AtomicU64,
    load_effects: AtomicU64,
    /// The lowest stack address a poll observed; `usize::MAX` until one does.
    stack_low_water: AtomicUsize,
    service_wait_ms: AtomicU64,
    load_wait_ms: AtomicU64,
    yield_wait_ms: AtomicU64,
    freeze_ms: AtomicU64,
    pub(crate) evaluate_ms: AtomicU64,
    pub(crate) serialize_ms: AtomicU64,
    /// Every invocation a `SILENT` clause absorbed, across the job's evaluations.
    silenced: Mutex<Vec<purrdf_core::SilencedInvocation>>,
}

impl Default for AsyncCounters {
    fn default() -> Self {
        Self {
            polls: AtomicU64::new(0),
            yields: AtomicU64::new(0),
            service_effects: AtomicU64::new(0),
            load_effects: AtomicU64::new(0),
            stack_low_water: AtomicUsize::new(usize::MAX),
            service_wait_ms: AtomicU64::new(0),
            load_wait_ms: AtomicU64::new(0),
            yield_wait_ms: AtomicU64::new(0),
            freeze_ms: AtomicU64::new(0),
            evaluate_ms: AtomicU64::new(0),
            serialize_ms: AtomicU64::new(0),
            silenced: Mutex::new(Vec::new()),
        }
    }
}

impl AsyncCounters {
    /// Count one completed suspension of `kind` that waited `waited_ms`.
    fn record_suspension(&self, kind: EffectKind, waited_ms: f64) {
        let (count, wait) = match kind {
            EffectKind::Service | EffectKind::AwaitExchange => {
                (&self.service_effects, &self.service_wait_ms)
            }
            EffectKind::Load => (&self.load_effects, &self.load_wait_ms),
            EffectKind::Yield => (&self.yields, &self.yield_wait_ms),
        };
        count.fetch_add(1, Ordering::Relaxed);
        add_ms(wait, waited_ms);
    }

    pub(crate) fn snapshot(&self, stack_top: usize) -> AsyncEvidence {
        let low = self.stack_low_water.load(Ordering::Relaxed);
        let high_water = if low == usize::MAX {
            0
        } else {
            stack_top.saturating_sub(low)
        };
        AsyncEvidence {
            polls: self.polls.load(Ordering::Relaxed) as f64,
            yields: self.yields.load(Ordering::Relaxed) as f64,
            service_effects: self.service_effects.load(Ordering::Relaxed) as f64,
            load_effects: self.load_effects.load(Ordering::Relaxed) as f64,
            stack_high_water_bytes: high_water as f64,
            service_wait_ms: read_ms(&self.service_wait_ms),
            load_wait_ms: read_ms(&self.load_wait_ms),
            yield_wait_ms: read_ms(&self.yield_wait_ms),
            freeze_ms: read_ms(&self.freeze_ms),
            evaluate_ms: read_ms(&self.evaluate_ms),
            serialize_ms: read_ms(&self.serialize_ms),
            silenced: crate::query::silenced_records(&lock(&self.silenced)),
        }
    }

    /// Keep the silenced invocations one evaluation's `evidence` recorded.
    pub(crate) fn record_silenced(&self, evidence: &purrdf_core::GovernorEvidence) {
        if evidence.silenced().is_empty() {
            return;
        }
        lock(&self.silenced).extend_from_slice(evidence.silenced());
    }
}

/// What one asynchronous job cost: counts and phase times, all measured in Rust.
///
/// `evaluateMs` is the wall time of the evaluation phase and so *includes* the waits the
/// three `…WaitMs` fields report separately; `freezeMs` and `serializeMs` are the phases
/// that run to completion without yielding.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct AsyncEvidence {
    polls: f64,
    yields: f64,
    service_effects: f64,
    load_effects: f64,
    stack_high_water_bytes: f64,
    service_wait_ms: f64,
    load_wait_ms: f64,
    yield_wait_ms: f64,
    freeze_ms: f64,
    evaluate_ms: f64,
    serialize_ms: f64,
    silenced: Vec<crate::query::SilencedInvocation>,
}

#[wasm_bindgen]
impl AsyncEvidence {
    /// Stop-signal polls the evaluator made.
    #[wasm_bindgen(getter)]
    pub fn polls(&self) -> f64 {
        self.polls
    }

    /// Times the job gave the event loop back.
    #[wasm_bindgen(getter)]
    pub fn yields(&self) -> f64 {
        self.yields
    }

    /// `SERVICE` effects — requests the job issued, and waits on another job's
    /// identical request — the host answered or abandoned. A request the job repeats is
    /// answered from its own memo and issues no effect.
    #[wasm_bindgen(getter, js_name = serviceEffects)]
    pub fn service_effects(&self) -> f64 {
        self.service_effects
    }

    /// `LOAD` effects the host answered or abandoned.
    #[wasm_bindgen(getter, js_name = loadEffects)]
    pub fn load_effects(&self) -> f64 {
        self.load_effects
    }

    /// The deepest the job's stack reached at a poll, in bytes below its region's top.
    /// `0` when the job never ran on a region (the native build).
    #[wasm_bindgen(getter, js_name = stackHighWaterBytes)]
    pub fn stack_high_water_bytes(&self) -> f64 {
        self.stack_high_water_bytes
    }

    /// Milliseconds spent suspended on `SERVICE` effects.
    #[wasm_bindgen(getter, js_name = serviceWaitMs)]
    pub fn service_wait_ms(&self) -> f64 {
        self.service_wait_ms
    }

    /// Milliseconds spent suspended on `LOAD` effects.
    #[wasm_bindgen(getter, js_name = loadWaitMs)]
    pub fn load_wait_ms(&self) -> f64 {
        self.load_wait_ms
    }

    /// Milliseconds spent suspended on yields.
    #[wasm_bindgen(getter, js_name = yieldWaitMs)]
    pub fn yield_wait_ms(&self) -> f64 {
        self.yield_wait_ms
    }

    /// Milliseconds freezing the dataset snapshot before the job started.
    #[wasm_bindgen(getter, js_name = freezeMs)]
    pub fn freeze_ms(&self) -> f64 {
        self.freeze_ms
    }

    /// Milliseconds of the evaluation phase, waits included.
    #[wasm_bindgen(getter, js_name = evaluateMs)]
    pub fn evaluate_ms(&self) -> f64 {
        self.evaluate_ms
    }

    /// Milliseconds serializing a raw result.
    #[wasm_bindgen(getter, js_name = serializeMs)]
    pub fn serialize_ms(&self) -> f64 {
        self.serialize_ms
    }

    /// Every invocation a `SERVICE SILENT` or `LOAD SILENT` absorbed during the job, in
    /// the order its evaluations recorded them. Empty when nothing failed.
    #[wasm_bindgen(getter)]
    pub fn silenced(&self) -> Vec<crate::query::SilencedInvocation> {
        self.silenced.clone()
    }
}

// ---------------------------------------------------------------------------
// Job slots: the state a host and a suspended job share
// ---------------------------------------------------------------------------

/// The effect a job is suspended on, as the host must answer it.
#[derive(Debug, Clone, Copy)]
struct Outstanding {
    seq: u32,
    kind: EffectKind,
    /// Why the host's abandonment instant falls when it does; `None` for a yield.
    abandon: Option<Abandon>,
}

/// The ticket exchange between a job and its host.
#[derive(Debug, Default)]
struct Tickets {
    /// The last sequence number issued; `0` before the first.
    last_issued: u32,
    /// The effect the host has not taken yet.
    posted: Option<AsyncEffect>,
    /// The effect awaiting an answer.
    outstanding: Option<Outstanding>,
    /// The answer, once delivered.
    delivered: Option<(u32, Delivered)>,
}

/// What a `SERVICE` request carried that the HTTP request does not: recorded by
/// [`HostServiceSource`] and read by [`JspiTransport`].
#[derive(Debug, Clone, Copy)]
struct ServiceContext {
    silent: bool,
    max_intermediate_cells: Option<u64>,
}

/// A region's address range, `[base, top)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StackBounds {
    base: usize,
    top: usize,
}

impl StackBounds {
    /// Whether a frame at `sp` may keep running: inside the region and above its guard
    /// band. The error is the fault to latch.
    ///
    /// The band is measured from the region's base itself, not from
    /// [`purrdf_stack::remaining`]: the floor that reads against is raised by the stack
    /// an evaluation reserves, and replaced by `purrdf_stack::EXHAUSTED` while a refusing
    /// walk unwinds — both are the evaluator's business, and reading either here would
    /// turn the evaluator's own typed refusal into this fault.
    fn check(self, sp: usize) -> Result<(), String> {
        if sp < self.base || sp > self.top {
            return Err(format!(
                "asynchronous job is not running on its stack region (stack pointer {sp:#x} \
                 outside [{:#x}, {:#x}]); the host did not switch the stack pointer",
                self.base, self.top
            ));
        }
        if sp - self.base < STACK_GUARD_BYTES {
            return Err(self.exhausted());
        }
        Ok(())
    }

    /// The typed error of a job that ran out of its region.
    fn exhausted(self) -> String {
        format!(
            "asynchronous job stack region exhausted ({} bytes); raise stackBytes",
            self.top - self.base
        )
    }
}

/// Everything a job shares with its host and its resolvers. `Sync` through mutexes and
/// atomics; no lock here is ever held across [`suspend`].
#[derive(Debug)]
struct JobSlots {
    job: u32,
    tickets: Mutex<Tickets>,
    service_context: Mutex<Option<ServiceContext>>,
    /// The shared `SERVICE` exchange the outstanding effect waits on, until the job
    /// leaves it.
    waiting_on: Mutex<Option<u64>>,
    /// The fatal job condition, first writer wins.
    fault: OnceLock<String>,
    /// The job's deadline expired while an effect was outstanding
    /// ([`AsyncJob::expire_effect`]).
    deadline_tripped: AtomicBool,
    /// The run has returned; deliveries are refused.
    finished: AtomicBool,
    /// Whether polls may compare the stack pointer with [`Self::bounds`]: set only while
    /// the job runs on its region, which is only ever on `wasm32`.
    region_armed: AtomicBool,
    bounds: StackBounds,
    /// The per-thread ambient state ([`JobAmbient`]) of the context the job was started
    /// or last resumed from, put back whenever the job leaves its region: at every
    /// suspension and when the run returns. Only ever written on `wasm32`, where the job
    /// runs on its region; locked only to move the value in or out, never across
    /// [`suspend`].
    outer: Mutex<JobAmbient>,
    counters: AsyncCounters,
}

impl JobSlots {
    fn new(job: u32, bounds: StackBounds) -> Self {
        Self {
            job,
            tickets: Mutex::new(Tickets::default()),
            service_context: Mutex::new(None),
            waiting_on: Mutex::new(None),
            fault: OnceLock::new(),
            deadline_tripped: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            region_armed: AtomicBool::new(false),
            bounds,
            outer: Mutex::new(JobAmbient::fresh(0)),
            counters: AsyncCounters::default(),
        }
    }

    /// Latch `message` as the job's fault. The first fault wins; later ones are dropped
    /// because they are consequences of it.
    fn latch_fault(&self, message: impl Into<String>) {
        let _ = self.fault.set(message.into());
    }

    fn fault(&self) -> Option<&str> {
        self.fault.get().map(String::as_str)
    }

    /// Post an effect for the host and make it the outstanding one, to be abandoned per
    /// `abandon` (`(milliseconds from now, why)`). `None` (with a fault latched) when
    /// another effect is still outstanding — which only a bridge that resumed a job
    /// without answering it could cause — or when the job has issued every sequence
    /// number there is.
    fn issue(&self, payload: EffectPayload, abandon: Option<(f64, Abandon)>) -> Option<u32> {
        let kind = payload.kind();
        let mut tickets = lock(&self.tickets);
        if let Some(outstanding) = tickets.outstanding {
            drop(tickets);
            self.latch_fault(format!(
                "an effect was issued while effect {} was still outstanding",
                outstanding.seq
            ));
            return None;
        }
        let Some(seq) = tickets.last_issued.checked_add(1) else {
            drop(tickets);
            self.latch_fault(format!(
                "the job issued every effect sequence number there is ({})",
                u32::MAX
            ));
            return None;
        };
        tickets.last_issued = seq;
        tickets.posted = Some(AsyncEffect {
            seq,
            payload,
            abandon_after_ms: abandon.map(|(ms, _)| ms),
        });
        tickets.outstanding = Some(Outstanding {
            seq,
            kind,
            abandon: abandon.map(|(_, why)| why),
        });
        tickets.delivered = None;
        drop(tickets);
        Some(seq)
    }

    /// Hand the posted effect to the host, once.
    fn take_effect(&self) -> Option<AsyncEffect> {
        lock(&self.tickets).posted.take()
    }

    /// The outstanding effect, when it is `seq`.
    fn outstanding(&self, seq: u32) -> Option<Outstanding> {
        lock(&self.tickets)
            .outstanding
            .filter(|outstanding| outstanding.seq == seq)
    }

    /// Accept `value` for effect `seq`, or say why not.
    fn deliver(&self, seq: u32, value: Delivered) -> DeliveryStatus {
        if self.finished.load(Ordering::Relaxed) {
            return DeliveryStatus::Finished;
        }
        let mut tickets = lock(&self.tickets);
        let fault = match tickets.outstanding {
            Some(outstanding) if outstanding.seq == seq => {
                if value.answers(outstanding.kind) {
                    tickets.outstanding = None;
                    tickets.posted = None;
                    tickets.delivered = Some((seq, value));
                    return DeliveryStatus::Accepted;
                }
                format!(
                    "{} was delivered to effect {seq}, which is a {:?} effect",
                    value.label(),
                    outstanding.kind
                )
            }
            _ if seq != 0 && seq <= tickets.last_issued => return DeliveryStatus::Stale,
            Some(outstanding) => format!(
                "a delivery named effect {seq}, but the outstanding effect is {}",
                outstanding.seq
            ),
            None => format!(
                "a delivery named effect {seq}, but no effect is outstanding (the last issued \
                 was {})",
                tickets.last_issued
            ),
        };
        drop(tickets);
        self.latch_fault(fault);
        DeliveryStatus::Fault
    }

    /// After resuming from effect `seq`: take its answer, and close it — and leave any
    /// shared exchange it waited on — whether or not one arrived.
    fn resume(&self, seq: u32) -> Option<Delivered> {
        self.leave_exchange();
        let mut tickets = lock(&self.tickets);
        if tickets
            .outstanding
            .is_some_and(|outstanding| outstanding.seq == seq)
        {
            tickets.outstanding = None;
        }
        tickets.posted = None;
        match tickets.delivered.take() {
            Some((delivered_seq, value)) if delivered_seq == seq => Some(value),
            other => {
                tickets.delivered = other;
                None
            }
        }
    }

    /// Whether an answer for effect `seq` is waiting to be resumed with.
    fn has_delivery(&self, seq: u32) -> bool {
        lock(&self.tickets)
            .delivered
            .as_ref()
            .is_some_and(|(delivered, _)| *delivered == seq)
    }

    /// Record that the outstanding effect waits on shared exchange `exchange`.
    fn wait_on(&self, exchange: u64) {
        *lock(&self.waiting_on) = Some(exchange);
    }

    /// Stop waiting on the shared exchange the outstanding effect waited on, if any: the
    /// exchange closes when no job is left waiting on it.
    fn leave_exchange(&self) {
        let exchange = lock(&self.waiting_on).take();
        if let Some(exchange) = exchange {
            leave_exchange(exchange, self.job);
        }
    }

    fn set_service_context(&self, context: Option<ServiceContext>) {
        *lock(&self.service_context) = context;
    }

    fn take_service_context(&self) -> Option<ServiceContext> {
        lock(&self.service_context).take()
    }
}

// ---------------------------------------------------------------------------
// The stop watch
// ---------------------------------------------------------------------------

/// The per-thread ambient state one job owns, moved as one value: its stack context (the
/// floor its guards measure against, its walk-scope state and the stack it reserved) and
/// the SHACL engine's ambient scopes (a validation's governors, `SERVICE`/`LOAD` sources,
/// registries and call depth). A job installs a fresh one when its run starts, puts the
/// outer context's back at every suspension and reinstalls its own at every resumption,
/// so whatever runs while it waits sees its own state and nothing of the job's — see
/// [`JspiStopWatch::leave_region`]. Every swapped thread-local in the workspace is one of
/// these two contexts' statics ([`crate::interleaving`]).
#[derive(Debug)]
struct JobAmbient {
    stack: purrdf_stack::Context,
    shacl: purrdf_shapes::sparql::AmbientContext,
}

// The ledger's swapped entries are exactly the statics these two contexts move: the five
// of `purrdf_stack::Context` and the eight of the SHACL `AmbientContext`.
const _: () = assert!(crate::interleaving::swapped_count() == 5 + 8);

impl JobAmbient {
    /// The state a job starts its run with: `floor` (its region's base) as the stack
    /// floor, no walk scope open, nothing reserved, and no SHACL scope installed.
    fn fresh(floor: usize) -> Self {
        Self {
            stack: purrdf_stack::Context::on_floor(floor),
            shacl: purrdf_shapes::sparql::AmbientContext::default(),
        }
    }

    /// Install this state on the thread, returning the state that was installed.
    fn install(self) -> Self {
        Self {
            stack: purrdf_stack::replace_context(self.stack),
            shacl: purrdf_shapes::sparql::replace_ambient_context(self.shacl),
        }
    }
}

/// The job's [`StopSignal`]: cancellation, the wall deadline (read off the clock, and
/// latched when an effect outlives it), the fault latch, the stack guard — and the poll
/// counter that slices evaluation into yields.
#[derive(Debug)]
struct JspiStopWatch {
    slots: Arc<JobSlots>,
    cancel: CancellationFlag,
    deadline: Option<WallDeadline>,
    /// When the deadline falls, on [`now_ms`]'s clock: the instant every effect's
    /// abandonment is bounded by and a shared exchange's deadline is compared against.
    deadline_at_ms: Option<f64>,
    /// Work per yield, counted in polls; `0` yields at every poll.
    quantum: u32,
    /// Work since the last yield. A poll counts as the work its checkpoint stands for
    /// ([`StopSignal::poll_after_work`]): one for a checkpoint the evaluator passes per
    /// item, a whole interval for one it passes once an interval of charged fuel.
    work_since_yield: AtomicU64,
    /// Work since the job began, for the clock-read cadence.
    work: AtomicU64,
    /// The resolved cause, written once; a fired signal stays fired.
    latched: OnceLock<StopCause>,
}

impl JspiStopWatch {
    fn new(slots: Arc<JobSlots>, deadline_ms: Option<u64>, quantum: u32) -> Self {
        Self {
            slots,
            cancel: CancellationFlag::new(),
            deadline: deadline_ms.map(|ms| WallDeadline::after(Duration::from_millis(ms))),
            deadline_at_ms: deadline_ms.map(|ms| now_ms() + ms as f64),
            quantum,
            work_since_yield: AtomicU64::new(0),
            work: AtomicU64::new(0),
            latched: OnceLock::new(),
        }
    }

    fn latch(&self, cause: StopCause) -> StopCause {
        *self.latched.get_or_init(|| cause)
    }

    /// Every source that needs no clock read, ranked as the kernel ranks them: a
    /// cancellation (or a fault, which stops the job the same way) ahead of a deadline.
    fn peek(&self) -> Option<StopCause> {
        if let Some(&cause) = self.latched.get() {
            return Some(cause);
        }
        let cause = if self.cancel.is_cancelled() || self.slots.fault().is_some() {
            StopCause::Cancelled
        } else if self.slots.deadline_tripped.load(Ordering::Relaxed)
            || self
                .deadline
                .as_ref()
                .is_some_and(WallDeadline::has_expired)
        {
            StopCause::Deadline
        } else {
            return None;
        };
        Some(self.latch(cause))
    }

    /// Every source, the clock included — the check after a resumption. Never yields.
    fn observe_now(&self) -> Option<StopCause> {
        self.peek().or_else(|| {
            self.deadline
                .as_ref()
                .and_then(StopSignal::poll)
                .map(|cause| self.latch(cause))
        })
    }

    /// The stack guard, while the job runs on its region.
    fn stack_check(&self) -> Result<(), String> {
        if !self.slots.region_armed.load(Ordering::Relaxed) {
            return Ok(());
        }
        // A frame that never polled may have run past the base since the last poll (see
        // `STACK_OVERRUN_ZONE_BYTES`); the canary it overwrote on the way stops the job at
        // the first poll after, before anything suspends on a damaged region.
        // SAFETY: the region is armed only while the job runs on it, and the job's
        // `StackRegion` — which owns the word at `base` — outlives every run.
        let canary = unsafe { core::ptr::read_volatile(self.slots.bounds.base as *const u32) };
        if canary != STACK_CANARY {
            return Err(self.slots.bounds.exhausted());
        }
        // The evaluator's own measurement of this frame's depth, read against the region's
        // bounds.
        let sp = purrdf_stack::stack_pointer();
        self.slots
            .counters
            .stack_low_water
            .fetch_min(sp, Ordering::Relaxed);
        self.slots.bounds.check(sp)
    }

    /// Suspend on the already-issued effect `seq`, count it, and return the host's
    /// status (an unknown status is latched as a fault and reported as one).
    fn suspend_on(&self, seq: u32, kind: EffectKind) -> SuspendStatus {
        let started = now_ms();
        // Leaving the region: whatever runs while the job is suspended runs on the
        // context's own stack, so it gets that stack's context back — its floor, and its
        // own walk scopes rather than the one the job may have open — and its own SHACL
        // scopes rather than the governors, sources and registries of a validation the
        // job is inside; the job gets its own back on resumption, from whichever context
        // resumed it (that context's are the ones to put back at the next suspension).
        let region = self.leave_region();
        let status = suspend(self.slots.job, seq);
        self.enter_region(region);
        self.slots
            .counters
            .record_suspension(kind, now_ms() - started);
        // Any suspension gave the event loop back, so the slice starts again.
        self.work_since_yield.store(0, Ordering::Relaxed);
        match status {
            0 => SuspendStatus::Answered,
            1 => SuspendStatus::Abandoned,
            2 => SuspendStatus::Fault,
            other => {
                self.slots.latch_fault(format!(
                    "the host bridge returned status {other} for effect {seq}"
                ));
                SuspendStatus::Fault
            }
        }
    }

    /// Put the outer context's per-thread state back before the job leaves its region,
    /// and return the job's own ([`JobAmbient`]: its region's floor, its open walk scopes
    /// and reserve, and the SHACL scopes installed by guards the job is still inside) to
    /// reinstall on the way back in. `None` off `wasm32`, where the job never runs on its
    /// region and nothing runs while it is suspended.
    fn leave_region(&self) -> Option<JobAmbient> {
        if !cfg!(target_arch = "wasm32") {
            return None;
        }
        let outer = std::mem::replace(&mut *lock(&self.slots.outer), JobAmbient::fresh(0));
        Some(outer.install())
    }

    /// Reinstall the job's per-thread state on resumption, recording the state of the
    /// context that resumed the job as the one to put back next. A no-op off `wasm32`.
    fn enter_region(&self, ambient: Option<JobAmbient>) {
        let Some(ambient) = ambient else {
            return;
        };
        *lock(&self.slots.outer) = ambient.install();
    }

    /// Give the event loop back once.
    fn yield_now(&self) {
        let Some(seq) = self.slots.issue(EffectPayload::Yield, None) else {
            return;
        };
        match self.suspend_on(seq, EffectKind::Yield) {
            SuspendStatus::Answered | SuspendStatus::Fault => {}
            // A yield has nothing to abandon: a host that reports one broke the protocol.
            SuspendStatus::Abandoned => self
                .slots
                .latch_fault(format!("the host abandoned yield effect {seq}")),
        }
        // A yield is answered by resuming; nothing is delivered for it.
        if let Some(delivered) = self.slots.resume(seq) {
            self.slots.latch_fault(format!(
                "{} was delivered for yield effect {seq}",
                delivered.label()
            ));
        }
    }

    /// Milliseconds left before the deadline, when one is set.
    fn remaining_deadline_ms(&self) -> Option<f64> {
        self.deadline_at_ms.map(|at| (at - now_ms()).max(0.0))
    }

    /// When a request with its own timeout of `timeout_ms` is abandoned if unanswered:
    /// after the timeout, or at the deadline when that falls first. A whole number of
    /// milliseconds, rounded up — a JavaScript timer takes nothing else — so the instant
    /// is never before the one it stands for.
    fn abandonment(&self, timeout_ms: f64) -> (f64, Abandon) {
        match self.remaining_deadline_ms() {
            Some(remaining) if remaining <= timeout_ms => (remaining.ceil(), Abandon::Deadline),
            _ => (timeout_ms.ceil(), Abandon::Timeout(timeout_ms)),
        }
    }
}

impl StopSignal for JspiStopWatch {
    fn poll(&self) -> Option<StopCause> {
        self.poll_after_work(1)
    }

    /// One poll, standing for `work` units of evaluation. The evidence counts polls; the
    /// clock read and the yield are paced by work, so a charged row loop — which polls
    /// once per interval of fuel rather than once per row — is sliced into turns of the
    /// same size as a scan that polls at every candidate.
    fn poll_after_work(&self, work: u64) -> Option<StopCause> {
        let work = work.max(1);
        self.slots.counters.polls.fetch_add(1, Ordering::Relaxed);
        if let Some(cause) = self.peek() {
            return Some(cause);
        }
        if let Err(fault) = self.stack_check() {
            self.slots.latch_fault(fault);
            return Some(self.latch(StopCause::Cancelled));
        }
        let before = self.work.fetch_add(work, Ordering::Relaxed);
        if before / CLOCK_EVERY_POLLS != before.saturating_add(work) / CLOCK_EVERY_POLLS
            && let Some(cause) = self.deadline.as_ref().and_then(StopSignal::poll)
        {
            return Some(self.latch(cause));
        }
        let since = self
            .work_since_yield
            .fetch_add(work, Ordering::Relaxed)
            .saturating_add(work);
        if self.quantum == 0 || since >= u64::from(self.quantum) {
            self.yield_now();
            return self.observe_now();
        }
        None
    }
}

// ---------------------------------------------------------------------------
// The resolvers
// ---------------------------------------------------------------------------

/// Map what a host delivered for a `SERVICE` effect onto the evaluator's seam, given
/// the host's status and whether the job's stop signal had fired by the time it
/// resumed. See the module documentation's taxonomy table.
fn service_answer(
    endpoint: &str,
    status: SuspendStatus,
    fired: Option<StopCause>,
    delivered: Option<Delivered>,
    slots: &JobSlots,
) -> Result<Arc<[u8]>, RemoteError> {
    if let Some(cause) = fired {
        let trip = TrippedGovernor::Stopped { cause };
        return Err(match delivered {
            Some(Delivered::Bindings(_)) => RemoteError::GovernedAfterCompletion(trip),
            _ => RemoteError::Governed(trip),
        });
    }
    let cancelled = RemoteError::Governed(TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    });
    match (status, delivered) {
        // The fault already latched is the job's error; the stop it fires ends the job.
        (SuspendStatus::Fault, _) => Err(cancelled),
        (SuspendStatus::Answered, Some(Delivered::Bindings(bytes))) => Ok(bytes),
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Transport | FailureKind::Decode,
                message,
            }),
        ) => Err(RemoteError::Transport(message)),
        // The host's handler failed to answer this effect: the invocation's failure, with
        // the host — not the endpoint — as its cause.
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Fault,
                message,
            }),
        ) => Err(RemoteError::HostFault {
            endpoint: endpoint.to_owned(),
            message,
        }),
        // A host resolver's own policy refusal, reached only when no native catalog
        // already denied the request — see `HttpRemoteQuerySource::resolve`, which
        // applies an installed `ServiceCatalog` (and returns `RemoteError::Denied`, a real
        // withheld capability) before this transport is ever reached. Whatever the host
        // reports here is therefore its own decision, not a capability this engine's
        // catalog withheld — see `RemoteError::HostDenied`.
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                message,
            }),
        ) => Err(RemoteError::HostDenied {
            endpoint: endpoint.to_owned(),
            message,
        }),
        (SuspendStatus::Abandoned, _) | (SuspendStatus::Answered, Some(Delivered::Governed)) => {
            slots.latch_fault(format!(
                "the host abandoned the SERVICE <{endpoint}> effect, but the job's stop signal \
                 had not fired"
            ));
            Err(cancelled)
        }
        (SuspendStatus::Answered, Some(other @ (Delivered::Graph(_) | Delivered::Redirect(_)))) => {
            slots.latch_fault(format!(
                "{} was delivered for SERVICE <{endpoint}>",
                other.label()
            ));
            Err(cancelled)
        }
        (SuspendStatus::Answered, None) => {
            slots.latch_fault(format!(
                "resolver returned without a delivery for SERVICE <{endpoint}>"
            ));
            Err(cancelled)
        }
    }
}

/// One hop of a `LOAD`, as its delivery decided it.
#[derive(Debug)]
enum LoadStep {
    /// The document.
    Graph(Arc<RdfDataset>),
    /// The host was redirected to this `Location`.
    Redirect(String),
}

/// Map what a host delivered for one hop of a `LOAD` onto the `LOAD` seam.
///
/// A fired signal is the job's own stop, [`LoadError::Governed`]: the evaluator reports
/// the trip, `SILENT` or not.
fn load_answer(
    iri: &str,
    status: SuspendStatus,
    fired: Option<StopCause>,
    delivered: Option<Delivered>,
    slots: &JobSlots,
) -> Result<LoadStep, LoadError> {
    if let Some(cause) = fired {
        return Err(LoadError::Governed(TrippedGovernor::Stopped { cause }));
    }
    let message = match (status, delivered) {
        (SuspendStatus::Answered, Some(Delivered::Graph(dataset))) => {
            return Ok(LoadStep::Graph(dataset));
        }
        (SuspendStatus::Answered, Some(Delivered::Redirect(location))) => {
            return Ok(LoadStep::Redirect(location));
        }
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Transport,
                message,
            }),
        ) => return Err(LoadError::Transport(message)),
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Decode,
                message,
            }),
        ) => return Err(LoadError::Decode(message)),
        // The catalog's own refusal never reaches the host (see `JspiGraphResolver`), so
        // a denial delivered here is the host's own decision.
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                message,
            }),
        ) => return Err(LoadError::HostDenied(message)),
        // The host's handler failed to answer this fetch: the fetch's failure, not the
        // job's fault.
        (
            SuspendStatus::Answered,
            Some(Delivered::Failure {
                kind: FailureKind::Fault,
                message,
            }),
        ) => return Err(LoadError::Fault(message)),
        // The fault already latched names the cause.
        (SuspendStatus::Fault, _) => {
            return Err(LoadError::Fault(slots.fault().map_or_else(
                || "the host latched a fault".to_owned(),
                str::to_owned,
            )));
        }
        (SuspendStatus::Abandoned, _) | (SuspendStatus::Answered, Some(Delivered::Governed)) => {
            format!(
                "the host abandoned the LOAD <{iri}> effect, but the job's stop signal had not \
                 fired"
            )
        }
        (SuspendStatus::Answered, Some(Delivered::Bindings(_))) => {
            format!("bindings were delivered for LOAD <{iri}>")
        }
        (SuspendStatus::Answered, None) => {
            format!("resolver returned without a delivery for LOAD <{iri}>")
        }
    };
    slots.latch_fault(message.clone());
    Err(LoadError::Fault(message))
}

/// The header names whose presence makes a request user-specific: a shared cache must
/// never answer one, because the answer is the credential holder's.
const CREDENTIAL_HEADERS: [&str; 3] = ["authorization", "cookie", "proxy-authorization"];

/// The [`HttpTransport`] that suspends the job on a `SERVICE` effect.
///
/// Wrapped in an [`HttpRemoteQuerySource`], so the catalog policy, the request shape and
/// the bounded SPARQL Results JSON decode are all the native ones; this type moves the
/// request to the host and the answer back. A request the job already had answered is
/// answered again from the job's memo, and a request another job has in flight is
/// waited on rather than sent twice (see [`ExchangeKey`]).
#[derive(Debug)]
struct JspiTransport {
    watch: Arc<JspiStopWatch>,
    /// The catalog requests are authorized against, for whether a request carries a
    /// credential.
    catalog: Option<NativeServiceCatalog>,
    /// The identity of the host's `resolveService` handler: only requests to the same
    /// handler share an exchange.
    handler: u32,
    /// The answers this job already received, by request. Successful bindings only: a
    /// failure is never replayed to a later request.
    memo: Mutex<BTreeMap<ExchangeKey, Arc<[u8]>>>,
}

impl JspiTransport {
    /// Whether a shared cache may answer `request` or keep its answer.
    fn cacheable(&self, request: &HttpRequest<'_>) -> bool {
        let credentialed_profile = self.catalog.as_ref().is_some_and(|catalog| {
            catalog
                .profile_for(request.endpoint)
                .is_some_and(|profile| profile.credential().is_some())
        });
        let credential_header = request.headers.iter().any(|(name, _)| {
            CREDENTIAL_HEADERS
                .iter()
                .any(|credential| name.eq_ignore_ascii_case(credential))
        });
        !(credentialed_profile || credential_header)
    }
}

impl HttpTransport for JspiTransport {
    fn post(&self, request: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError> {
        let slots = &self.watch.slots;
        let context = slots.take_service_context();
        let effect = ServiceEffect {
            endpoint: request.endpoint.to_owned(),
            query_text: request.query_text.to_owned(),
            user_agent: request.user_agent.to_owned(),
            timeout_ms: request.timeout.as_secs_f64() * 1000.0,
            content_type: request.content_type.to_owned(),
            accept: request.accept.to_owned(),
            headers: request.headers.to_vec(),
            silent: context.is_some_and(|context| context.silent),
            max_intermediate_cells: context.and_then(|context| context.max_intermediate_cells),
            remaining_deadline_ms: self.watch.remaining_deadline_ms(),
            cacheable: self.cacheable(&request),
        };
        let key = ExchangeKey::new(self.handler, &effect);
        if let Some(bytes) = lock(&self.memo).get(&key) {
            return Ok(bytes.to_vec());
        }
        let deadline_at_ms = self.watch.deadline_at_ms;
        let abandon = self.watch.abandonment(effect.timeout_ms);
        let joined = find_exchange(&key, deadline_at_ms);
        let exchange = joined.unwrap_or_else(next_exchange_id);
        let payload = match joined {
            Some(_) => EffectPayload::AwaitExchange { exchange },
            None => EffectPayload::Service {
                effect: Box::new(effect),
                exchange,
            },
        };
        let kind = payload.kind();
        let Some(seq) = slots.issue(payload, Some(abandon)) else {
            let fired = self.watch.observe_now();
            return service_answer(request.endpoint, SuspendStatus::Fault, fired, None, slots)
                .map(|bytes| bytes.to_vec());
        };
        let waiter = (Arc::clone(slots), seq);
        match joined {
            Some(_) => add_waiter(exchange, waiter),
            None => open_exchange(exchange, key.clone(), deadline_at_ms, waiter),
        }
        slots.wait_on(exchange);
        let status = self.watch.suspend_on(seq, kind);
        let delivered = slots.resume(seq);
        // `request.stop` is this same watch; observing it directly avoids a second poll
        // that could slice into a yield before the answer is even mapped.
        let fired = self.watch.observe_now();
        let bytes = service_answer(request.endpoint, status, fired, delivered, slots)?;
        lock(&self.memo).insert(key, Arc::clone(&bytes));
        Ok(bytes.to_vec())
    }
}

/// The source a job's host-resolved `SERVICE` clauses reach: the native HTTP adapter over
/// [`JspiTransport`], recording what the HTTP request cannot carry (`SILENT`, the cell
/// ceiling) before delegating.
#[derive(Debug)]
struct HostServiceSource {
    slots: Arc<JobSlots>,
    source: HttpRemoteQuerySource<JspiTransport>,
}

impl ServiceResolver for HostServiceSource {
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        self.slots.set_service_context(Some(ServiceContext {
            silent: request.silent,
            max_intermediate_cells: request.max_intermediate_cells,
        }));
        let resolved = self.source.resolve(request);
        // A denial or an earlier stop never reaches the transport; clear what it would
        // have taken so the next request starts clean.
        self.slots.set_service_context(None);
        resolved
    }
}

/// The source a job with local services but no host `SERVICE` handler sends every other
/// endpoint to: it fails exactly as the offline lane's missing source does
/// ([`RemoteError::Unconfigured`]) — an error, and under `SILENT` the join identity with
/// a silenced record.
#[derive(Debug)]
struct UnhandledServiceSource;

impl ServiceResolver for UnhandledServiceSource {
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        if let Some(trip) = request.stop_trip() {
            return Err(trip);
        }
        Err(RemoteError::Unconfigured(
            "no remote query source configured: the endpoint is not a local service and no \
             resolveService handler was supplied"
                .to_owned(),
        ))
    }
}

/// A job's `SERVICE` source: its local services, answered in process, and every other
/// endpoint handed to the host's handler — or, with no handler, refused exactly as the
/// offline lane's missing source is.
///
/// Owned, so an operation can install it where a validation reads its sources (see
/// [`JobRun`]). A job with local services routes each request exactly as a
/// [`purrdf_sparql_eval::ServiceRouter`] with one route per local endpoint and the host's
/// source for every other does — a fired signal prevents the exchange, a local endpoint
/// is answered in process, every other endpoint goes to the host — without building one
/// per request: a router borrows its resolvers, and this owns them. A job without local
/// services hands every request to the host's source directly.
#[derive(Debug)]
struct JobServiceSource {
    /// The in-process resolver and the endpoints it serves.
    local: Option<(InProcessServiceResolver, Vec<String>)>,
    host: Option<HostServiceSource>,
}

impl ServiceResolver for JobServiceSource {
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        let remote: &(dyn ServiceResolver + Sync) = match &self.host {
            Some(host) => host,
            None => &UnhandledServiceSource,
        };
        let Some((local, endpoints)) = &self.local else {
            return remote.resolve(request);
        };
        // The router's own sequence. Its no-route denial cannot arise: every endpoint
        // that is not local has the host's source (or its refusal).
        if let Some(trip) = request.stop_trip() {
            return Err(trip);
        }
        if endpoints
            .iter()
            .any(|endpoint| endpoint == request.endpoint)
        {
            local.resolve(request)
        } else {
            remote.resolve(request)
        }
    }
}

/// The `Accept` header of a `LOAD` fetch: every parseable RDF syntax's media type.
fn load_accept() -> String {
    purrdf::NativeRdfFormat::all()
        .map(purrdf::NativeRdfFormat::media_type)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `LOAD` source that suspends the job on a `Load` effect.
///
/// With a catalog installed, every hop's IRI — the source and every location a redirect
/// names — is authorized against it before the host is asked: a `LOAD` needs the
/// `network` capability (and `credentials` for a credential), exactly the policy a
/// `SERVICE` meets, and a refusal is the catalog's [`LoadError::Denied`] with nothing
/// fetched. The authorized profile supplies the hop's headers, credential, user agent and
/// timeout, so a redirect to another origin carries only what that origin's own profile
/// grants. A redirect is followed here, never by the host: its location is resolved
/// against the hop's IRI and fetched as a fresh effect, up to [`MAX_LOAD_REDIRECTS`] hops.
#[derive(Debug)]
struct JspiGraphResolver {
    watch: Arc<JspiStopWatch>,
    catalog: Option<NativeServiceCatalog>,
}

impl JspiGraphResolver {
    /// The effect that fetches `iri`, or the catalog's refusal.
    fn authorize(&self, iri: &str) -> Result<LoadEffect, LoadError> {
        let default_timeout_ms = DEFAULT_TIMEOUT.as_secs_f64() * 1000.0;
        let Some(catalog) = &self.catalog else {
            return Ok(LoadEffect {
                iri: iri.to_owned(),
                accept: load_accept(),
                user_agent: None,
                headers: Vec::new(),
                timeout_ms: default_timeout_ms,
            });
        };
        let profile = catalog
            .authorize(
                iri,
                ServiceCapabilities::granting([ServiceCapability::Network]),
            )
            .map_err(LoadError::Denied)?;
        Ok(LoadEffect {
            iri: iri.to_owned(),
            accept: load_accept(),
            user_agent: profile.user_agent().map(str::to_owned),
            headers: profile.request_headers(),
            timeout_ms: profile
                .timeout()
                .map_or(default_timeout_ms, |timeout| timeout.as_secs_f64() * 1000.0),
        })
    }
}

impl GraphResolver for JspiGraphResolver {
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
        let slots = &self.watch.slots;
        let mut iri = request.iri.to_owned();
        let mut hops = 0_u32;
        loop {
            let effect = self.authorize(&iri)?;
            let abandon = self.watch.abandonment(effect.timeout_ms);
            let Some(seq) = slots.issue(EffectPayload::Load(Box::new(effect)), Some(abandon))
            else {
                // Issuing refused, with the job's fault latched: the stop that fault
                // fires is what the evaluator reports.
                return Err(match self.watch.observe_now() {
                    Some(cause) => LoadError::Governed(TrippedGovernor::Stopped { cause }),
                    None => LoadError::Fault(format!("the LOAD <{iri}> effect was not issued")),
                });
            };
            let status = self.watch.suspend_on(seq, EffectKind::Load);
            let delivered = slots.resume(seq);
            let fired = self.watch.observe_now();
            match load_answer(&iri, status, fired, delivered, slots)? {
                LoadStep::Graph(dataset) => return Ok(dataset),
                LoadStep::Redirect(location) => {
                    hops += 1;
                    if hops > MAX_LOAD_REDIRECTS {
                        return Err(LoadError::Transport(format!(
                            "exceeded {MAX_LOAD_REDIRECTS} redirect hops; the last, from <{iri}>, \
                             named {location:?}"
                        )));
                    }
                    iri = redirect_target(&iri, &location).map_err(LoadError::Transport)?;
                }
            }
        }
    }
}

/// The IRI a redirect from `from` to `location` names: the location resolved against
/// the IRI it redirected (RFC 3986 §5.2).
fn redirect_target(from: &str, location: &str) -> Result<String, String> {
    purrdf::iri::parse(from)
        .and_then(|base| base.resolve(location))
        .map(|target| target.as_str().to_owned())
        .map_err(|error| {
            format!("<{from}> redirected to {location:?}, which is not a resolvable IRI: {error}")
        })
}

// ---------------------------------------------------------------------------
// Shared SERVICE exchanges
// ---------------------------------------------------------------------------

/// What identifies a `SERVICE` request to the host: everything the host can observe but
/// the deadline and the signal. Two effects with equal keys ask the host the same
/// question, so a job answers a repeat from its memo, and concurrent jobs share one host
/// call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExchangeKey {
    handler: u32,
    endpoint: String,
    query_text: String,
    accept: String,
    content_type: String,
    user_agent: String,
    timeout_bits: u64,
    headers: Vec<(String, String)>,
    silent: bool,
    max_intermediate_cells: Option<u64>,
}

impl ExchangeKey {
    fn new(handler: u32, effect: &ServiceEffect) -> Self {
        Self {
            handler,
            endpoint: effect.endpoint.clone(),
            query_text: effect.query_text.clone(),
            accept: effect.accept.clone(),
            content_type: effect.content_type.clone(),
            user_agent: effect.user_agent.clone(),
            timeout_bits: effect.timeout_ms.to_bits(),
            headers: effect.headers.clone(),
            silent: effect.silent,
            max_intermediate_cells: effect.max_intermediate_cells,
        }
    }
}

/// One host call in flight, and every job waiting on its answer.
///
/// A job joins an open exchange only when its key matches and the exchange's deadline is
/// no earlier than the job's own (a job without a deadline joins only an exchange without
/// one): the host was told the opening job's remaining deadline, so a host that bounds its
/// work by it gives up no earlier than it would on the joining job's own call, and a
/// failure the shared call reports by running out of time is one the job's own call would
/// have reported too. The answer — rows, a failure, or a fault — is delivered to every
/// waiting job as it stands. A job that is stopped while it waits leaves; when the last
/// waiter leaves, the exchange closes and the host aborts its call.
#[derive(Debug)]
struct OpenExchange {
    key: ExchangeKey,
    deadline_at_ms: Option<f64>,
    waiters: Vec<(Arc<JobSlots>, u32)>,
}

impl OpenExchange {
    /// Whether a job whose deadline falls at `deadline_at_ms` may wait on this exchange.
    fn covers(&self, deadline_at_ms: Option<f64>) -> bool {
        match (self.deadline_at_ms, deadline_at_ms) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(exchange), Some(job)) => exchange >= job,
        }
    }
}

thread_local! {
    /// Every open shared exchange, by id. Borrowed only inside the helpers below, never
    /// across a delivery or a suspension.
    static EXCHANGES: RefCell<BTreeMap<u64, OpenExchange>> = const { RefCell::new(BTreeMap::new()) };
    /// The last exchange id handed out.
    static LAST_EXCHANGE_ID: Cell<u64> = const { Cell::new(0) };
}

/// A fresh exchange id.
fn next_exchange_id() -> u64 {
    LAST_EXCHANGE_ID.with(|last| {
        let id = last.get() + 1;
        last.set(id);
        id
    })
}

/// The open exchange a job with `key` and `deadline_at_ms` may wait on.
fn find_exchange(key: &ExchangeKey, deadline_at_ms: Option<f64>) -> Option<u64> {
    EXCHANGES.with(|exchanges| {
        exchanges
            .borrow()
            .iter()
            .find(|(_, open)| open.key == *key && open.covers(deadline_at_ms))
            .map(|(&id, _)| id)
    })
}

fn open_exchange(
    id: u64,
    key: ExchangeKey,
    deadline_at_ms: Option<f64>,
    waiter: (Arc<JobSlots>, u32),
) {
    EXCHANGES.with(|exchanges| {
        exchanges.borrow_mut().insert(
            id,
            OpenExchange {
                key,
                deadline_at_ms,
                waiters: vec![waiter],
            },
        )
    });
}

fn add_waiter(id: u64, waiter: (Arc<JobSlots>, u32)) {
    EXCHANGES.with(|exchanges| {
        if let Some(open) = exchanges.borrow_mut().get_mut(&id) {
            open.waiters.push(waiter);
        }
    });
}

/// Job `job` stops waiting on exchange `id`; the exchange closes when no job is left.
fn leave_exchange(id: u64, job: u32) {
    let closed = EXCHANGES.with(|exchanges| {
        let mut exchanges = exchanges.borrow_mut();
        let open = exchanges.get_mut(&id)?;
        open.waiters.retain(|(slots, _)| slots.job != job);
        open.waiters
            .is_empty()
            .then(|| exchanges.remove(&id))
            .flatten()
    });
    drop(closed);
}

/// Close exchange `id`, returning its waiters; `None` when it is not open.
fn close_exchange(id: u64) -> Option<Vec<(Arc<JobSlots>, u32)>> {
    EXCHANGES
        .with(|exchanges| exchanges.borrow_mut().remove(&id))
        .map(|open| open.waiters)
}

/// Deliver `value` to every job waiting on exchange `id`, closing it.
fn settle_exchange(id: u64, value: &Delivered) -> DeliveryStatus {
    let Some(waiters) = close_exchange(id) else {
        return DeliveryStatus::Finished;
    };
    for (slots, seq) in waiters {
        // A waiter that already left or finished takes nothing; that is not the other
        // waiters' concern.
        let _ = slots.deliver(seq, value.clone());
    }
    DeliveryStatus::Accepted
}

// ---------------------------------------------------------------------------
// Operations and outcomes
// ---------------------------------------------------------------------------

/// Which operation a job runs. Each is the asynchronous twin of a synchronous entry.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOperationKind {
    /// `query`/`select`/`ask`/`construct`/`describe`: a typed result.
    Query = 0,
    /// `queryRaw`/`queryRawConfigured`/`Dataset.query`: serialized bytes.
    Raw = 1,
    /// `queryRawWithContext`: bytes serialized under a compiled JSON-LD context.
    RawWithContext = 2,
    /// `queryGoverned`: a governed outcome.
    Governed = 3,
    /// `queryEntailmentGoverned`: a two-phase governed outcome.
    EntailmentGoverned = 4,
    /// `update`: applied through [`AsyncJob::commit_update`].
    Update = 5,
    /// `updateGoverned`: a governed outcome, applied through [`AsyncJob::commit_update`].
    UpdateGoverned = 6,
    /// `queryGovernedNegotiatedAsync`: a governed outcome whose complete answer is
    /// serialized in the format negotiated from an `Accept` header, once the result's
    /// shape is known. The asynchronous lane's own: a SPARQL Protocol endpoint's query.
    Negotiated = 7,
    /// `explainQuery`: the rendered charge ledger, taken through
    /// [`AsyncJob::take_raw_bytes`]. EXPLAIN evaluates the query to measure it, so its
    /// measuring run suspends on `SERVICE`, yields and stops exactly as a query does.
    Explain = 8,
    /// A SHACL surface — validation to SARIF, change validation, SHACL-AF entailment, or
    /// validation with a prepared product ([`ShaclAsyncOperation`] names which). Its
    /// `sh:SPARQLTarget` queries, SHACL-SPARQL constraints, node expressions and rules
    /// run under the job's signal and reach its `SERVICE` and `LOAD` sources, and the
    /// signal is polled between focus nodes too, so a validation with no SPARQL in it
    /// still yields and stops. Started through `AsyncJob.beginShacl`.
    Shacl = 9,
}

impl AsyncOperationKind {
    const fn name(self) -> &'static str {
        self.spec().name
    }
}

/// A job's work: a closure over the run's signal and sources.
type Operation = Box<dyn FnOnce(&JobRun<'_>) -> JobOutcome>;

/// The job's work for a SPARQL operation: the one implementation both lanes run.
fn sparql_operation(input: OperationInput<'static>) -> Operation {
    Box::new(move |run| input.execute(run).unwrap_or_else(JobOutcome::Failed))
}

// ---------------------------------------------------------------------------
// SHACL operations
// ---------------------------------------------------------------------------

/// Why `beginAsync` refuses the SHACL kind.
pub(crate) const SHACL_STARTS_ELSEWHERE: &str =
    "a shacl operation reads no dataset and no SPARQL text; it starts through AsyncJob.beginShacl";

/// Which SHACL surface a [`AsyncOperationKind::Shacl`] job runs: one per synchronous
/// entry point that evaluates SPARQL, each returning exactly what that entry returns.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaclAsyncOperation {
    /// `shaclValidateToSarif`: a SARIF log, taken through [`AsyncJob::take_raw_bytes`].
    ValidateToSarif = 0,
    /// `shaclValidateChangesToSarif`: a [`ShaclChangeValidation`], taken through
    /// [`AsyncJob::take_shacl_change_validation`].
    ValidateChangesToSarif = 1,
    /// `shaclEntail`: canonical N-Triples, taken through [`AsyncJob::take_raw_bytes`].
    Entail = 2,
    /// `shaclProductValidateToSarif`.
    ProductValidateToSarif = 3,
    /// `shaclProductValidateToSarifRebuild`.
    ProductValidateToSarifRebuild = 4,
    /// `shaclProductValidateToSarifExpecting`.
    ProductValidateToSarifExpecting = 5,
    /// `shaclProductValidateToSarifRebuildExpecting`.
    ProductValidateToSarifRebuildExpecting = 6,
}

impl ShaclAsyncOperation {
    const fn name(self) -> &'static str {
        match self {
            Self::ValidateToSarif => "shaclValidateToSarif",
            Self::ValidateChangesToSarif => "shaclValidateChangesToSarif",
            Self::Entail => "shaclEntail",
            Self::ProductValidateToSarif => "shaclProductValidateToSarif",
            Self::ProductValidateToSarifRebuild => "shaclProductValidateToSarifRebuild",
            Self::ProductValidateToSarifExpecting => "shaclProductValidateToSarifExpecting",
            Self::ProductValidateToSarifRebuildExpecting => {
                "shaclProductValidateToSarifRebuildExpecting"
            }
        }
    }
}

/// A SHACL job's arguments, exactly those of the synchronous entry it twins.
#[derive(Debug)]
enum ShaclRequest {
    Validate {
        shapes: String,
        shapes_base: Option<String>,
        data: String,
    },
    ValidateChanges {
        shapes: String,
        shapes_base: Option<String>,
        data: String,
        added: Option<String>,
        removed: Option<String>,
    },
    Entail {
        shapes: String,
        shapes_base: Option<String>,
        data: String,
    },
    Product {
        product: Vec<u8>,
        data: String,
        rebuild: bool,
        expect_identity: Option<String>,
    },
}

/// The arguments `AsyncJob.beginShacl` was handed, before they are matched to an operation.
#[derive(Debug, Default)]
struct ShaclArguments {
    shapes: Option<String>,
    shapes_base: Option<String>,
    data: String,
    added: Option<String>,
    removed: Option<String>,
    product: Option<Vec<u8>>,
    expect_identity: Option<String>,
}

impl ShaclRequest {
    /// Match `arguments` to `operation`, refusing a missing argument and one the
    /// operation would ignore, by name.
    fn build(operation: ShaclAsyncOperation, arguments: ShaclArguments) -> Result<Self, String> {
        let ShaclArguments {
            shapes,
            shapes_base,
            data,
            added,
            removed,
            product,
            expect_identity,
        } = arguments;
        let op = operation.name();
        let refuse = |name: &str| format!("{op} takes no {name}");
        let is_product = matches!(
            operation,
            ShaclAsyncOperation::ProductValidateToSarif
                | ShaclAsyncOperation::ProductValidateToSarifRebuild
                | ShaclAsyncOperation::ProductValidateToSarifExpecting
                | ShaclAsyncOperation::ProductValidateToSarifRebuildExpecting
        );
        let expecting = matches!(
            operation,
            ShaclAsyncOperation::ProductValidateToSarifExpecting
                | ShaclAsyncOperation::ProductValidateToSarifRebuildExpecting
        );
        if operation != ShaclAsyncOperation::ValidateChangesToSarif
            && (added.is_some() || removed.is_some())
        {
            return Err(refuse("change document"));
        }
        if !expecting && expect_identity.is_some() {
            return Err(refuse("expected identity"));
        }
        if is_product {
            if shapes.is_some() {
                return Err(refuse("shapes graph: a product carries its own"));
            }
            if shapes_base.is_some() {
                return Err(refuse("shapesBase: a product records its own"));
            }
            let product = product.ok_or_else(|| format!("{op} needs a product"))?;
            if expecting && expect_identity.is_none() {
                return Err(format!("{op} needs an expected identity"));
            }
            return Ok(Self::Product {
                product,
                data,
                rebuild: matches!(
                    operation,
                    ShaclAsyncOperation::ProductValidateToSarifRebuild
                        | ShaclAsyncOperation::ProductValidateToSarifRebuildExpecting
                ),
                expect_identity,
            });
        }
        if product.is_some() {
            return Err(refuse("product"));
        }
        let shapes = shapes.ok_or_else(|| format!("{op} needs a shapes graph"))?;
        Ok(match operation {
            ShaclAsyncOperation::ValidateToSarif => Self::Validate {
                shapes,
                shapes_base,
                data,
            },
            ShaclAsyncOperation::ValidateChangesToSarif => Self::ValidateChanges {
                shapes,
                shapes_base,
                data,
                added,
                removed,
            },
            _ => Self::Entail {
                shapes,
                shapes_base,
                data,
            },
        })
    }

    /// Run the synchronous entry's own body. Every one reaches the engine through the
    /// ambient scopes, so the execution scope [`execute_shacl`] installs around this is
    /// what governs it: nothing here is a second implementation of any surface.
    fn run(self) -> JobOutcome {
        let text = |result: Result<String, String>| match result {
            Ok(text) => JobOutcome::Raw(text),
            Err(message) => JobOutcome::Failed(JobError::message(SHACL_CODE, message)),
        };
        let refusable = |result: Result<String, ShaclProductRefusal>| match result {
            Ok(text) => JobOutcome::Raw(text),
            Err(refusal) => JobOutcome::Refused(refusal),
        };
        match self {
            Self::Validate {
                shapes,
                shapes_base,
                data,
            } => text(validate_to_sarif_impl(
                &shapes,
                shapes_base.as_deref(),
                &data,
            )),
            Self::ValidateChanges {
                shapes,
                shapes_base,
                data,
                added,
                removed,
            } => match validate_changes_to_sarif_impl(
                &shapes,
                shapes_base.as_deref(),
                &data,
                added.as_deref(),
                removed.as_deref(),
            ) {
                Ok((sarif, scope)) => {
                    JobOutcome::ShaclChange(ShaclChangeValidation::new(sarif, scope))
                }
                Err(message) => JobOutcome::Failed(JobError::message(SHACL_CODE, message)),
            },
            Self::Entail {
                shapes,
                shapes_base,
                data,
            } => text(entail_to_ntriples_impl(
                &shapes,
                shapes_base.as_deref(),
                &data,
            )),
            Self::Product {
                product,
                data,
                rebuild,
                expect_identity,
            } => refusable(match (rebuild, expect_identity.as_deref()) {
                (false, None) => {
                    product_validate_impl(&product, &data).map_err(ShaclProductRefusal::from)
                }
                (true, None) => product_validate_rebuild_impl(&product, &data)
                    .map_err(ShaclProductRefusal::from),
                (false, Some(expected)) => {
                    product_validate_expecting_impl(&product, &data, expected)
                }
                (true, Some(expected)) => {
                    product_validate_rebuild_expecting_impl(&product, &data, expected)
                }
            }),
        }
    }
}

/// Run `request` under the job: its signal and its sources installed as the SHACL
/// engine's execution scope, so every SPARQL query the surface runs — a
/// `sh:SPARQLTarget`, a SHACL-SPARQL constraint, a node expression, a rule — reaches the
/// job's `SERVICE` and `LOAD` sources and polls its signal, and the signal is polled
/// between focus nodes as well.
///
/// The scope is metered and never bounded, as `queryAsync`'s is: the synchronous twins
/// take no ceiling, so neither does this. A stop is the job's error — a validation cut
/// short has no report, and its partial one is never offered — read back off the state
/// that latched it rather than out of the error text the stop unwound with.
fn execute_shacl(request: ShaclRequest, run: &JobRun<'_>) -> JobOutcome {
    let state = Arc::new(GovernorState::new(&run.governors(QueryGovernors::METERED)));
    let outcome = {
        let _scope =
            purrdf_shapes::sparql::enter_execution_scope(Arc::clone(&state), run.sources());
        run.evaluate(|| request.run())
    };
    run.record_silenced(&state.evidence());
    match state.tripped() {
        Some(tripped @ TrippedGovernor::Stopped { .. }) => {
            JobOutcome::Failed(JobError::stopped(tripped))
        }
        _ => outcome,
    }
}

// ---------------------------------------------------------------------------
// Jobs and the registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobState {
    Pending,
    Running,
    Done,
}

/// A job's stack region: owned heap memory the job's frames live in while it runs, above
/// an overrun zone of [`STACK_OVERRUN_ZONE_BYTES`].
struct StackRegion {
    /// The allocation: alignment padding, the overrun zone, then the region. While the
    /// job runs its frames own the region's bytes; Rust reads the zone only after a run
    /// returns ([`Self::overrun`]).
    memory: Box<[u8]>,
    /// Where the overrun zone starts inside `memory`.
    zone_offset: usize,
    bounds: StackBounds,
}

/// How far a finished run's frames reached below its region's base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overrun {
    /// Never past the base: the canary and the whole zone are intact.
    None,
    /// Past the base, but never into the zone's floor: nothing outside the job's own
    /// allocation was touched.
    Absorbed,
    /// Into the zone's floor: the frames may have left the allocation.
    Escaped,
}

impl fmt::Debug for StackRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StackRegion")
            .field("bounds", &self.bounds)
            .finish_non_exhaustive()
    }
}

impl StackRegion {
    /// A zeroed region of `bytes` rounded down to 16, its base and top aligned to 16,
    /// the canary written at the base and the filled overrun zone beneath it. The
    /// region's size depends only on `bytes`, never on where the allocator put it.
    fn new(bytes: usize) -> Self {
        let usable = bytes & !15;
        let mut memory = vec![0u8; 15 + STACK_OVERRUN_ZONE_BYTES + usable].into_boxed_slice();
        let start = memory.as_ptr() as usize;
        let zone_offset = start.next_multiple_of(16) - start;
        let base_offset = zone_offset + STACK_OVERRUN_ZONE_BYTES;
        memory[zone_offset..base_offset].fill(STACK_ZONE_FILL);
        memory[base_offset..base_offset + 4].copy_from_slice(&STACK_CANARY.to_le_bytes());
        let base = start + base_offset;
        Self {
            memory,
            zone_offset,
            bounds: StackBounds {
                base,
                top: base + usable,
            },
        }
    }

    /// How far the finished run's frames reached below the base. Read only once the run
    /// has returned, when no frame lives in the region any more.
    fn overrun(&self) -> Overrun {
        let zone = &self.memory[self.zone_offset..self.zone_offset + STACK_OVERRUN_ZONE_BYTES];
        let (floor, upper) = zone.split_at(STACK_OVERRUN_FLOOR_BYTES);
        if floor.iter().any(|&byte| byte != STACK_ZONE_FILL) {
            return Overrun::Escaped;
        }
        let base_offset = self.zone_offset + STACK_OVERRUN_ZONE_BYTES;
        let canary = &self.memory[base_offset..base_offset + 4];
        if canary != STACK_CANARY.to_le_bytes() || upper.iter().any(|&byte| byte != STACK_ZONE_FILL)
        {
            return Overrun::Absorbed;
        }
        Overrun::None
    }
}

/// A registered job.
struct JobInner {
    id: u32,
    kind: AsyncOperationKind,
    watch: Arc<JspiStopWatch>,
    operation: RefCell<Option<Operation>>,
    /// The identity of the host's `resolveService` handler, when it supplied one.
    service_handler: Option<u32>,
    load_handler: bool,
    catalog: Option<NativeServiceCatalog>,
    local_services: Vec<(String, Arc<RdfDataset>)>,
    dataset_id: u64,
    dataset_generation: u64,
    /// An update job's claim on its dataset: no other asynchronous update of it may begin
    /// until [`AsyncJob::finish`] releases it.
    update_claim: RefCell<Option<UpdateClaim>>,
    region: StackRegion,
    state: Cell<JobState>,
    outcome: RefCell<Option<JobOutcome>>,
    error: RefCell<Option<JobError>>,
    /// The frozen result an applied update offers `commitUpdate`, until it is taken.
    pending_commit: RefCell<Option<Arc<RdfDataset>>>,
    /// A SHACL product job's refusal, beside the error it also stored, until it is
    /// taken ([`AsyncJob::take_shacl_refusal`]).
    refusal: RefCell<Option<ShaclProductRefusal>>,
}

impl fmt::Debug for JobInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JobInner")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("state", &self.state.get())
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

impl JobInner {
    /// The lane the job's failures are rendered for.
    const fn lane(&self) -> Lane {
        Lane::Async {
            region_bytes: self.region.bounds.top - self.region.bounds.base,
        }
    }

    fn run(&self) -> RunStatus {
        if self.state.get() != JobState::Pending {
            return RunStatus::AlreadyStarted;
        }
        self.state.set(JobState::Running);
        let operation = self.operation.borrow_mut().take();
        let slots = &self.watch.slots;
        // Only on wasm32 is the job actually running on its region (the host switched the
        // stack pointer before the promising call); the native build never arms the guard.
        slots
            .region_armed
            .store(cfg!(target_arch = "wasm32"), Ordering::Relaxed);
        // The region's base is the floor every stack measurement on this job reads — the
        // poll-time guard band and the evaluator's own guard alike — and the job starts
        // with no walk scope open and no SHACL scope installed, whatever the context that
        // started it has open or installed, until it suspends or returns (see
        // `JspiStopWatch::leave_region`).
        if cfg!(target_arch = "wasm32") {
            *lock(&slots.outer) = JobAmbient::fresh(self.region.bounds.base).install();
        }
        let outcome = match (operation, self.watch.stack_check()) {
            (_, Err(fault)) => {
                slots.latch_fault(fault);
                JobOutcome::Failed(JobError::fault("the job did not start"))
            }
            (Some(operation), Ok(())) => self.execute(operation),
            (None, Ok(())) => {
                slots.latch_fault("the job's operation was already taken");
                JobOutcome::Failed(JobError::fault("the job had no operation"))
            }
        };
        slots.region_armed.store(false, Ordering::Relaxed);
        if cfg!(target_arch = "wasm32") {
            // Every guard the operation installed has dropped by now, so the job's own
            // state is idle and is discarded as the outer one goes back.
            let outer = std::mem::replace(&mut *lock(&slots.outer), JobAmbient::fresh(0));
            drop(outer.install());
        }
        match self.region.overrun() {
            Overrun::None => {}
            // Frames that never polled ran past the base into the zone: nothing outside
            // the job's allocation was touched, so this is the job's typed exhaustion.
            Overrun::Absorbed => slots.latch_fault(self.region.bounds.exhausted()),
            Overrun::Escaped => {
                slots.finished.store(true, Ordering::Relaxed);
                *self.error.borrow_mut() = Some(JobError::fault(format!(
                    "asynchronous job {} ran more than {} bytes past the base of its stack \
                     region ({} bytes), so memory outside it may be overwritten; raise \
                     stackBytes",
                    self.id,
                    STACK_OVERRUN_ZONE_BYTES - STACK_OVERRUN_FLOOR_BYTES,
                    self.region.bounds.top - self.region.bounds.base
                )));
                self.state.set(JobState::Done);
                return RunStatus::Overran;
            }
        }
        slots.finished.store(true, Ordering::Relaxed);
        // A latched fault outranks whatever the operation reached: that outcome was
        // produced by a job whose effects cannot be trusted, and an update is never
        // committed from it.
        let outcome = match slots.fault() {
            Some(fault) => JobOutcome::Failed(JobError::fault(fault)),
            None => outcome,
        };
        let status = match outcome {
            JobOutcome::Failed(error) => {
                *self.error.borrow_mut() = Some(error);
                RunStatus::Error
            }
            JobOutcome::Refused(refusal) => {
                *self.error.borrow_mut() = Some(JobError::message(
                    SHACL_REFUSAL_CODE,
                    refusal.to_js_string(),
                ));
                *self.refusal.borrow_mut() = Some(refusal);
                RunStatus::Error
            }
            JobOutcome::Updated(frozen) => {
                *self.pending_commit.borrow_mut() = Some(frozen);
                RunStatus::Outcome
            }
            JobOutcome::UpdateGoverned { outcome, frozen } => {
                *self.pending_commit.borrow_mut() = frozen;
                *self.outcome.borrow_mut() = Some(JobOutcome::UpdateGoverned {
                    outcome,
                    frozen: None,
                });
                RunStatus::Outcome
            }
            other => {
                *self.outcome.borrow_mut() = Some(other);
                RunStatus::Outcome
            }
        };
        self.state.set(JobState::Done);
        status
    }

    /// Build the job's effect sources and run `operation` over them.
    fn execute(&self, operation: Operation) -> JobOutcome {
        let local = (!self.local_services.is_empty()).then(|| {
            let resolver = self.local_services.iter().fold(
                InProcessServiceResolver::new(),
                |resolver, (endpoint, dataset)| {
                    resolver.with_endpoint(endpoint.clone(), Arc::clone(dataset))
                },
            );
            let endpoints = self
                .local_services
                .iter()
                .map(|(endpoint, _)| endpoint.clone())
                .collect();
            (resolver, endpoints)
        });
        let host = self.service_handler.map(|handler| {
            let transport = JspiTransport {
                watch: Arc::clone(&self.watch),
                catalog: self.catalog.clone(),
                handler,
                memo: Mutex::new(BTreeMap::new()),
            };
            let source = match &self.catalog {
                Some(catalog) => {
                    HttpRemoteQuerySource::new(transport).with_catalog(catalog.clone())
                }
                None => HttpRemoteQuerySource::new(transport),
            };
            HostServiceSource {
                slots: Arc::clone(&self.watch.slots),
                source,
            }
        });
        let remote: Option<Arc<dyn ServiceResolver + Send + Sync>> =
            (local.is_some() || host.is_some()).then(|| {
                Arc::new(JobServiceSource { local, host }) as Arc<dyn ServiceResolver + Send + Sync>
            });
        let load = self.load_handler.then(|| {
            Arc::new(JspiGraphResolver {
                watch: Arc::clone(&self.watch),
                catalog: self.catalog.clone(),
            }) as Arc<dyn GraphResolver + Send + Sync>
        });
        let stop: Arc<dyn StopSignal> = Arc::clone(&self.watch) as Arc<dyn StopSignal>;
        let run = JobRun {
            stop: Some(stop),
            remote,
            load,
            counters: Some(&self.watch.slots.counters),
        };
        operation(&run)
    }

    fn require_done(&self, what: &str) -> Result<(), JobError> {
        match self.state.get() {
            JobState::Done => Ok(()),
            _ => Err(JobError::message(
                USAGE_CODE,
                format!("{what} is not available until the asynchronous job has finished"),
            )),
        }
    }

    fn require_kind(&self, what: &str, kinds: &[AsyncOperationKind]) -> Result<(), JobError> {
        if kinds.contains(&self.kind) {
            Ok(())
        } else {
            Err(JobError::message(
                USAGE_CODE,
                format!("{what} is not available on a {} job", self.kind.name()),
            ))
        }
    }

    /// The stored outcome, for a `take*` that expects this job's kind.
    fn take_outcome(
        &self,
        what: &str,
        kinds: &[AsyncOperationKind],
    ) -> Result<JobOutcome, JobError> {
        self.require_kind(what, kinds)?;
        self.require_done(what)?;
        if let Some(error) = self.error.borrow().as_ref() {
            return Err(error.clone());
        }
        self.outcome.borrow_mut().take().ok_or_else(|| {
            JobError::message(USAGE_CODE, format!("{what}: the outcome was already taken"))
        })
    }

    /// Commit an applied update into `dataset`, refusing any commit that could
    /// overwrite a mutation the update never saw.
    fn commit_into(&self, dataset: &mut Dataset) -> Result<(), JobError> {
        self.require_kind(
            "commitUpdate",
            &[
                AsyncOperationKind::Update,
                AsyncOperationKind::UpdateGoverned,
            ],
        )?;
        self.require_done("commitUpdate")?;
        if let Some(error) = self.error.borrow().as_ref() {
            return Err(error.clone());
        }
        if self.pending_commit.borrow().is_none() {
            return Err(JobError::message(
                USAGE_CODE,
                "nothing to commit: the update was not applied, or was already committed",
            ));
        }
        if dataset.identity() != self.dataset_id {
            return Err(JobError::message(
                USAGE_CODE,
                format!(
                    "commit targets a different dataset (the update read dataset {}, this is \
                 dataset {}); the update was not applied",
                    self.dataset_id,
                    dataset.identity()
                ),
            ));
        }
        // A mutation the update never saw is the conflict an update in flight is refused
        // for, found at the commit rather than at the start.
        if dataset.current_generation() != self.dataset_generation {
            return Err(JobError::message(
                FailureCode::UpdateInFlight.code(),
                format!(
                    "dataset mutated while an asynchronous update was in flight (generation {} → \
                 {}); the update was not applied",
                    self.dataset_generation,
                    dataset.current_generation()
                ),
            ));
        }
        if let Some(frozen) = self.pending_commit.borrow_mut().take() {
            dataset.replace(frozen);
        }
        Ok(())
    }
}

thread_local! {
    /// Every job not yet finished, by id. Borrowed only for an insert, a lookup or a
    /// removal — never across a run.
    static JOBS: RefCell<BTreeMap<u32, Rc<JobInner>>> = const { RefCell::new(BTreeMap::new()) };
    /// The last job id handed out.
    static LAST_JOB_ID: Cell<u32> = const { Cell::new(0) };
}

/// Register the job `build` makes for a fresh id.
fn register_job(build: impl FnOnce(u32) -> JobInner) -> Rc<JobInner> {
    let id = JOBS.with(|jobs| {
        let jobs = jobs.borrow();
        let mut candidate = LAST_JOB_ID.with(Cell::get);
        loop {
            candidate = candidate.wrapping_add(1).max(1);
            if !jobs.contains_key(&candidate) {
                break candidate;
            }
        }
    });
    LAST_JOB_ID.with(|last| last.set(id));
    let job = Rc::new(build(id));
    JOBS.with(|jobs| jobs.borrow_mut().insert(id, Rc::clone(&job)));
    job
}

fn lookup_job(id: u32) -> Option<Rc<JobInner>> {
    JOBS.with(|jobs| jobs.borrow().get(&id).cloned())
}

fn unregister_job(id: u32) {
    let removed = JOBS.with(|jobs| jobs.borrow_mut().remove(&id));
    drop(removed);
}

// ---------------------------------------------------------------------------
// The JavaScript job handle
// ---------------------------------------------------------------------------

/// One asynchronous operation, driven by the package root's JSPI scheduler.
///
/// Every method is safe to call while the job's frames are suspended, and none of them
/// throws for a protocol condition: deliveries return a status and latch a fault instead.
/// The `take*` methods and `commitUpdate` are ordinary synchronous calls made after the
/// job has finished; they throw for a caller bug (the wrong kind, a job not yet finished)
/// and for the job's own error.
///
/// A job stays registered — and its stack region allocated — until [`Self::finish`] is
/// called, whether or not this handle has been freed: the host calls it once the run
/// has settled.
#[wasm_bindgen]
pub struct AsyncJob {
    inner: Rc<JobInner>,
}

impl fmt::Debug for AsyncJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncJob")
            .field("inner", &self.inner)
            .finish()
    }
}

/// A usage refusal of the job handle, thrown as the error carrying its code.
fn usage_error(message: &str) -> JsValue {
    coded_error(message, USAGE_CODE)
}

#[wasm_bindgen]
impl AsyncJob {
    /// The id `purrdf_jspi_run` and `purrdf_jspi_suspend` name this job by.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> u32 {
        self.inner.id
    }

    /// The operation this job runs.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> AsyncOperationKind {
        self.inner.kind
    }

    /// The region's top: the stack pointer to install before the promising call.
    #[wasm_bindgen(getter, js_name = stackTop)]
    pub fn stack_top(&self) -> u32 {
        u32::try_from(self.inner.region.bounds.top).unwrap_or(u32::MAX)
    }

    /// The region's lowest usable address, where [`Self::stack_canary`] is written.
    #[wasm_bindgen(getter, js_name = stackBase)]
    pub fn stack_base(&self) -> u32 {
        u32::try_from(self.inner.region.bounds.base).unwrap_or(u32::MAX)
    }

    /// The little-endian `u32` at [`Self::stack_base`] while the region is intact.
    #[wasm_bindgen(getter, js_name = stackCanary)]
    pub fn stack_canary(&self) -> u32 {
        STACK_CANARY
    }

    /// Whether the run has returned.
    #[wasm_bindgen(getter, js_name = isFinished)]
    pub fn is_finished(&self) -> bool {
        self.inner.state.get() == JobState::Done
    }

    /// How the job failed — `"error"`, `"cancelled"`, `"deadline"`, `"fault"` or
    /// `"not-acceptable"` — or `undefined` when it did not (or has not finished).
    #[wasm_bindgen(getter, js_name = errorKind)]
    pub fn error_kind(&self) -> Option<String> {
        self.inner
            .error
            .borrow()
            .as_ref()
            .map(|error| error.kind().name().to_owned())
    }

    /// The stable code the job's failure is reported under — an engine diagnostic's own
    /// code, or the code of a stop, a fault or a refusal — or `undefined` when it did not
    /// fail (or has not finished).
    #[wasm_bindgen(getter, js_name = errorCode)]
    pub fn error_code(&self) -> Option<String> {
        self.inner
            .error
            .borrow()
            .as_ref()
            .map(|error| error.code().to_owned())
    }

    /// The job's failure in words, or `undefined` when it did not fail (or has not
    /// finished).
    #[wasm_bindgen(getter, js_name = errorMessage)]
    pub fn error_message(&self) -> Option<String> {
        let lane = self.inner.lane();
        self.inner
            .error
            .borrow()
            .as_ref()
            .map(|error| error.rendered(lane))
    }

    /// The effect the job is suspended on, once; `undefined` when there is none.
    #[wasm_bindgen(js_name = takeEffect)]
    pub fn take_effect(&self) -> Option<AsyncEffect> {
        self.inner.watch.slots.take_effect()
    }

    /// Fail effect `seq`: `kind` is `"transport"` (unreachable, or unreadable), `"denied"`
    /// (the host's policy refused it) or `"fault"` (the host's handler threw, rejected or
    /// returned something the protocol does not define). Any other kind is delivered as a
    /// fault naming it. Each is the effect's own failure, answered by the clause that
    /// issued it; none is the job's fault.
    #[wasm_bindgen(js_name = deliverFailure)]
    pub fn deliver_failure(&self, seq: u32, kind: &str, message: String) -> DeliveryStatus {
        self.inner.watch.slots.deliver(
            seq,
            FailureKind::delivered(kind, message, &format!("effect {seq}")),
        )
    }

    /// Answer `LOAD` effect `seq` with a document. It is parsed here, by media type (or
    /// any format name `Dataset.parse` accepts); a document that cannot be parsed is the
    /// `LOAD`'s decode failure, never a fault.
    #[wasm_bindgen(js_name = deliverGraph)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn deliver_graph(
        &self,
        seq: u32,
        bytes: &[u8],
        media_type: &str,
        base: Option<String>,
    ) -> DeliveryStatus {
        let delivered = match parse_document(bytes, media_type, base.as_deref()) {
            Ok(dataset) => Delivered::Graph(dataset),
            Err(message) => Delivered::Failure {
                kind: FailureKind::Decode,
                message,
            },
        };
        self.inner.watch.slots.deliver(seq, delivered)
    }

    /// Answer `LOAD` effect `seq` with a snapshot of an existing dataset.
    #[wasm_bindgen(js_name = deliverGraphDataset)]
    pub fn deliver_graph_dataset(&self, seq: u32, dataset: &Dataset) -> DeliveryStatus {
        let delivered = match dataset.view().freeze() {
            Ok(frozen) => Delivered::Graph(frozen),
            Err(diagnostic) => Delivered::Failure {
                kind: FailureKind::Decode,
                message: diagnostic.to_string(),
            },
        };
        self.inner.watch.slots.deliver(seq, delivered)
    }

    /// Answer `LOAD` effect `seq` with a redirect to `location`, the response's
    /// `Location` as sent. The job resolves it against the effect's IRI, authorizes the
    /// target against its catalog and fetches it as a fresh effect, up to its redirect
    /// limit.
    #[wasm_bindgen(js_name = deliverRedirect)]
    pub fn deliver_redirect(&self, seq: u32, location: String) -> DeliveryStatus {
        self.inner
            .watch
            .slots
            .deliver(seq, Delivered::Redirect(location))
    }

    /// Record that the host abandoned effect `seq` on the job's stop signal. When that
    /// signal is not already latched, this cancels the job. A job waiting on a shared
    /// exchange leaves it; see [`Self::exchange_is_open`].
    #[wasm_bindgen(js_name = deliverGoverned)]
    pub fn deliver_governed(&self, seq: u32) -> DeliveryStatus {
        let watch = &self.inner.watch;
        if watch.slots.finished.load(Ordering::Relaxed) {
            return DeliveryStatus::Finished;
        }
        if watch.peek().is_none() {
            watch.cancel.cancel();
        }
        watch.slots.leave_exchange();
        watch.slots.deliver(seq, Delivered::Governed)
    }

    /// Effect `seq`'s abandonment instant ([`AsyncEffect::abandon_after_ms`]) has
    /// passed with no answer. When it was the job's deadline, the deadline is latched and
    /// the effect abandoned (`Abandoned`); when it was the request's own timeout, the
    /// effect fails as a transport failure the `SILENT` forms absorb (`Answered`). A job
    /// waiting on a shared exchange leaves it. An effect already answered is left as it
    /// is (`Answered`), and a fault is reported as one (`Fault`).
    #[wasm_bindgen(js_name = expireEffect)]
    pub fn expire_effect(&self, seq: u32) -> SuspendStatus {
        let slots = &self.inner.watch.slots;
        if slots.fault().is_some() {
            return SuspendStatus::Fault;
        }
        let Some(outstanding) = slots.outstanding(seq) else {
            if slots.has_delivery(seq) {
                return SuspendStatus::Answered;
            }
            slots.latch_fault(format!(
                "effect {seq} expired, but it is not the outstanding effect"
            ));
            return SuspendStatus::Fault;
        };
        slots.leave_exchange();
        match outstanding.abandon {
            Some(Abandon::Deadline) => {
                slots.deadline_tripped.store(true, Ordering::Relaxed);
                match slots.deliver(seq, Delivered::Governed) {
                    DeliveryStatus::Accepted => SuspendStatus::Abandoned,
                    DeliveryStatus::Stale | DeliveryStatus::Fault | DeliveryStatus::Finished => {
                        SuspendStatus::Fault
                    }
                }
            }
            Some(Abandon::Timeout(ms)) => {
                let failure = Delivered::Failure {
                    kind: FailureKind::Transport,
                    message: format!("no answer within {ms} ms"),
                };
                match slots.deliver(seq, failure) {
                    DeliveryStatus::Accepted => SuspendStatus::Answered,
                    DeliveryStatus::Stale | DeliveryStatus::Fault | DeliveryStatus::Finished => {
                        SuspendStatus::Fault
                    }
                }
            }
            None => {
                slots.latch_fault(format!(
                    "effect {seq} expired, but a {:?} effect has no abandonment instant",
                    outstanding.kind
                ));
                SuspendStatus::Fault
            }
        }
    }

    /// The status to resume effect `seq` with once the shared exchange it waited on has
    /// settled: `Answered` when the exchange's answer was delivered to it, `Fault` when a
    /// fault is latched (a settled exchange that delivered nothing to a job still waiting
    /// on it latches one).
    #[wasm_bindgen(js_name = settledStatus)]
    pub fn settled_status(&self, seq: u32) -> SuspendStatus {
        let slots = &self.inner.watch.slots;
        if slots.fault().is_some() {
            return SuspendStatus::Fault;
        }
        if slots.has_delivery(seq) {
            return SuspendStatus::Answered;
        }
        slots.latch_fault(format!(
            "the exchange effect {seq} waited on settled without an answer for it"
        ));
        SuspendStatus::Fault
    }

    /// Answer every job waiting on shared exchange `exchange` with SPARQL Results JSON
    /// bytes, and close the exchange. `Finished` when no such exchange is open — its
    /// last waiter left, or it was already answered.
    #[wasm_bindgen(js_name = deliverExchangeBindings)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn deliver_exchange_bindings(exchange: f64, bytes: Vec<u8>) -> DeliveryStatus {
        let Some(id) = exchange_id(exchange) else {
            return DeliveryStatus::Finished;
        };
        settle_exchange(id, &Delivered::Bindings(Arc::from(bytes)))
    }

    /// Fail every job waiting on shared exchange `exchange`: `kind` is `"transport"`,
    /// `"denied"` or `"fault"`; any other kind is delivered as a fault naming it. Each
    /// waiting job's effect fails as its own invocation's failure. Closes the exchange.
    #[wasm_bindgen(js_name = deliverExchangeFailure)]
    pub fn deliver_exchange_failure(exchange: f64, kind: &str, message: String) -> DeliveryStatus {
        let Some(id) = exchange_id(exchange) else {
            return DeliveryStatus::Finished;
        };
        settle_exchange(
            id,
            &FailureKind::delivered(kind, message, &format!("exchange {id}")),
        )
    }

    /// Fail every job waiting on shared exchange `exchange` with the host handler's fault
    /// `message` — it threw, rejected or returned something the protocol does not define
    /// — and close the exchange. Each waiter's effect fails as its own invocation's
    /// failure; no job's fault is latched.
    #[wasm_bindgen(js_name = faultExchange)]
    pub fn fault_exchange(exchange: f64, message: &str) -> DeliveryStatus {
        let Some(id) = exchange_id(exchange) else {
            return DeliveryStatus::Finished;
        };
        settle_exchange(
            id,
            &Delivered::Failure {
                kind: FailureKind::Fault,
                message: message.to_owned(),
            },
        )
    }

    /// Whether shared exchange `exchange` is still open: some job waits on its answer.
    /// Once it closes without an answer, the host aborts its call.
    #[wasm_bindgen(js_name = exchangeIsOpen)]
    pub fn exchange_is_open(exchange: f64) -> bool {
        exchange_id(exchange)
            .is_some_and(|id| EXCHANGES.with(|exchanges| exchanges.borrow().contains_key(&id)))
    }

    /// Latch `message` as the job's fault: a host bug the job cannot continue past. The
    /// job's signal fires, it winds down, and the fault becomes its error.
    pub fn fault(&self, message: String) -> DeliveryStatus {
        let slots = &self.inner.watch.slots;
        if slots.finished.load(Ordering::Relaxed) {
            return DeliveryStatus::Finished;
        }
        slots.latch_fault(message);
        DeliveryStatus::Accepted
    }

    /// Cancel the job. It observes the cancellation at its next poll, yield or effect.
    pub fn cancel(&self) -> DeliveryStatus {
        if self.inner.watch.slots.finished.load(Ordering::Relaxed) {
            return DeliveryStatus::Finished;
        }
        self.inner.watch.cancel.cancel();
        DeliveryStatus::Accepted
    }

    /// A `query` job's typed result. `expect` — `"select"`, `"ask"`, `"construct"`,
    /// `"describe"` (or `"graph"`) — refuses any other result kind with the synchronous
    /// twin's message; `undefined` accepts any.
    #[wasm_bindgen(js_name = takeQueryResult)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn take_query_result(&self, expect: Option<String>) -> Result<QueryResult, JsValue> {
        let expected = match expect.as_deref() {
            None => None,
            Some("select") => Some("SELECT solutions"),
            Some("ask") => Some("ASK boolean"),
            Some("construct" | "describe" | "graph") => Some("CONSTRUCT/DESCRIBE graph"),
            Some(other) => {
                return Err(usage_error(&format!(
                    "unknown expected result kind {other:?} (expected select, ask, construct, \
                     describe or graph)"
                )));
            }
        };
        let JobOutcome::Query(result) =
            self.take("takeQueryResult", &[AsyncOperationKind::Query])?
        else {
            return Err(usage_error(
                "takeQueryResult: the job holds no query result",
            ));
        };
        if let Some(expected) = expected {
            let matches = matches!(
                (expected, &result),
                ("SELECT solutions", SparqlResult::Solutions { .. })
                    | ("ASK boolean", SparqlResult::Boolean(_))
                    | ("CONSTRUCT/DESCRIBE graph", SparqlResult::Graph(_))
            );
            if !matches {
                return Err(kind_mismatch(expected, &result).into());
            }
        }
        Ok(query_result_from_sparql(result)?)
    }

    /// A raw job's serialized bytes, an explain job's rendered ledger, or a SHACL job's
    /// SARIF log or entailed N-Triples (UTF-8 text).
    #[wasm_bindgen(js_name = takeRawBytes)]
    pub fn take_raw_bytes(&self) -> Result<Vec<u8>, JsValue> {
        match self.take(
            "takeRawBytes",
            &[
                AsyncOperationKind::Raw,
                AsyncOperationKind::RawWithContext,
                AsyncOperationKind::Explain,
                AsyncOperationKind::Shacl,
            ],
        )? {
            JobOutcome::Raw(text) => Ok(text.into_bytes()),
            _ => Err(usage_error("takeRawBytes: the job holds no raw result")),
        }
    }

    /// A governed query job's outcome.
    #[wasm_bindgen(js_name = takeQueryOutcome)]
    pub fn take_query_outcome(&self) -> Result<QueryOutcome, JsValue> {
        match self.take("takeQueryOutcome", &[AsyncOperationKind::Governed])? {
            JobOutcome::Governed(outcome) => Ok(query_outcome_from_governed(*outcome)?),
            _ => Err(usage_error(
                "takeQueryOutcome: the job holds no governed outcome",
            )),
        }
    }

    /// A negotiated query job's outcome: the serialized complete answer, or the trip.
    #[wasm_bindgen(js_name = takeNegotiatedOutcome)]
    pub fn take_negotiated_outcome(&self) -> Result<NegotiatedOutcome, JsValue> {
        match self.take("takeNegotiatedOutcome", &[AsyncOperationKind::Negotiated])? {
            JobOutcome::Negotiated(value) => Ok(negotiated_outcome_from_value(*value)?),
            _ => Err(usage_error(
                "takeNegotiatedOutcome: the job holds no negotiated outcome",
            )),
        }
    }

    /// A governed entailment job's outcome.
    #[wasm_bindgen(js_name = takeEntailmentOutcome)]
    pub fn take_entailment_outcome(&self) -> Result<EntailmentQueryOutcome, JsValue> {
        match self.take(
            "takeEntailmentOutcome",
            &[AsyncOperationKind::EntailmentGoverned],
        )? {
            JobOutcome::Entailment(outcome) => Ok(entailment_query_outcome_from_native(*outcome)?),
            _ => Err(usage_error(
                "takeEntailmentOutcome: the job holds no entailment outcome",
            )),
        }
    }

    /// A governed update job's outcome. Whether it applied is on the outcome; applying it
    /// to the dataset is [`Self::commit_update`].
    #[wasm_bindgen(js_name = takeUpdateOutcome)]
    pub fn take_update_outcome(&self) -> Result<UpdateOutcome, JsValue> {
        match self.take("takeUpdateOutcome", &[AsyncOperationKind::UpdateGoverned])? {
            JobOutcome::UpdateGoverned { outcome, .. } => {
                Ok(update_outcome_from_governed(&outcome))
            }
            _ => Err(usage_error(
                "takeUpdateOutcome: the job holds no update outcome",
            )),
        }
    }

    /// A SHACL change-validation job's outcome: the SARIF log and the scope it
    /// describes, exactly as `shaclValidateChangesToSarif` returns them.
    #[wasm_bindgen(js_name = takeShaclChangeValidation)]
    pub fn take_shacl_change_validation(&self) -> Result<ShaclChangeValidation, JsValue> {
        match self.take("takeShaclChangeValidation", &[AsyncOperationKind::Shacl])? {
            JobOutcome::ShaclChange(validation) => Ok(validation),
            _ => Err(usage_error(
                "takeShaclChangeValidation: the job holds no change validation",
            )),
        }
    }

    /// A SHACL product job's refusal, once — the `ShaclProductRefusal` the synchronous
    /// twin throws — or `undefined` when the job was not refused.
    #[wasm_bindgen(js_name = takeShaclRefusal)]
    pub fn take_shacl_refusal(&self) -> Option<ShaclProductRefusal> {
        self.inner.refusal.borrow_mut().take()
    }

    /// A snapshot of the job's evidence. Callable at any time, as often as wanted.
    #[wasm_bindgen(js_name = takeEvidence)]
    pub fn take_evidence(&self) -> AsyncEvidence {
        self.inner
            .watch
            .slots
            .counters
            .snapshot(self.inner.region.bounds.top)
    }

    /// Apply a finished update's result to `dataset` — refused, with the dataset left
    /// untouched, when `dataset` is not the one the update read or has been mutated since
    /// it started.
    #[wasm_bindgen(js_name = commitUpdate)]
    pub fn commit_update(&self, dataset: &mut Dataset) -> Result<(), JsValue> {
        self.inner
            .commit_into(dataset)
            .map_err(|error| error.to_js(self.inner.lane()))
    }

    /// Remove the job from the registry once it has finished, and release an update's
    /// claim on its dataset: returns 0, or 1 (and does nothing) while it is still
    /// running, because the run is standing on the job's region.
    pub fn finish(&self) -> u32 {
        if self.inner.state.get() == JobState::Running {
            return 1;
        }
        drop(self.inner.update_claim.borrow_mut().take());
        unregister_job(self.inner.id);
        0
    }
}

impl AsyncJob {
    /// The stored outcome, for a `take*` that expects this job's kind, or the job's own
    /// failure as the error carrying its code.
    fn take(&self, what: &str, kinds: &[AsyncOperationKind]) -> Result<JobOutcome, JsValue> {
        self.inner
            .take_outcome(what, kinds)
            .map_err(|error| error.to_js(self.inner.lane()))
    }
}

/// An exchange id as JavaScript passes it back: the `number` [`AsyncEffect::exchange_id`]
/// gave it. `None` for any other number, which names no exchange.
fn exchange_id(exchange: f64) -> Option<u64> {
    (exchange.is_finite() && exchange.fract() == 0.0 && (1.0..=2f64.powi(53)).contains(&exchange))
        .then_some(exchange as u64)
}

/// The fault for a delivered failure kind the protocol does not define.
fn unknown_failure_kind(kind: &str, target: &str) -> String {
    format!(
        "unknown failure kind {kind:?} for {target} (expected \"transport\", \"denied\" or \
         \"fault\")"
    )
}

/// Parse a `LOAD` document delivered by the host.
fn parse_document(
    bytes: &[u8],
    media_type: &str,
    base: Option<&str>,
) -> Result<Arc<RdfDataset>, String> {
    let media_type = resolve_media_type(media_type)?;
    parse_dataset(bytes, media_type, base).map_err(|diagnostic| diagnostic.to_string())
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// How an option's value is read from the options object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionShape {
    /// A string.
    Text,
    /// A governor ceiling: an integer as a `number`, a `bigint` or an integral string.
    Ceiling,
    /// A count: a `number`, whose range is checked when the job begins.
    Count,
    /// `{ prefix, iri }`, two strings.
    Provenance,
    /// A `ServiceCatalog`.
    Catalog,
    /// An object mapping endpoint IRIs to `Dataset`s.
    LocalServices,
}

/// Every option key an asynchronous operation reads, and its shape.
const OPTION_SHAPES: [(&str, OptionShape); 17] = [
    ("base", OptionShape::Text),
    ("format", OptionShape::Text),
    ("provenanceNamespace", OptionShape::Provenance),
    ("optionsJson", OptionShape::Text),
    ("yamlSchemaUrl", OptionShape::Text),
    ("aggregateNamespace", OptionShape::Text),
    ("program", OptionShape::Text),
    ("accept", OptionShape::Text),
    ("fuel", OptionShape::Ceiling),
    ("deadlineMs", OptionShape::Ceiling),
    ("maxAnswers", OptionShape::Ceiling),
    ("maxIntermediateCells", OptionShape::Ceiling),
    ("maxScratchBytes", OptionShape::Ceiling),
    ("maxRemoteRequests", OptionShape::Ceiling),
    ("yieldEveryPolls", OptionShape::Count),
    ("stackBytes", OptionShape::Count),
    ("catalog", OptionShape::Catalog),
];

/// The shape of option `key`, or `None` for a key no operation reads.
fn option_shape(key: &str) -> Option<OptionShape> {
    if key == "localServices" {
        return Some(OptionShape::LocalServices);
    }
    OPTION_SHAPES
        .iter()
        .find(|(name, _)| *name == key)
        .map(|&(_, shape)| shape)
}

/// The keys every asynchronous operation accepts beside its own: the job's host options.
const HOST_KEYS: [&str; 4] = ["yieldEveryPolls", "stackBytes", "catalog", "localServices"];

/// The keys the package root reads itself and never passes on: the host's handlers and
/// its signal. Named in a refusal's list of accepted keys.
const PACKAGE_KEYS: [&str; 3] = ["resolveService", "resolveLoad", "signal"];

/// The governor ceilings, in the order the synchronous governed entries name them.
const CEILING_KEYS: [&str; 6] = [
    "fuel",
    "deadlineMs",
    "maxAnswers",
    "maxIntermediateCells",
    "maxScratchBytes",
    "maxRemoteRequests",
];

/// The keys every governed operation accepts.
const GOVERNED_KEYS: [&str; 8] = [
    "base",
    "aggregateNamespace",
    "fuel",
    "deadlineMs",
    "maxAnswers",
    "maxIntermediateCells",
    "maxScratchBytes",
    "maxRemoteRequests",
];

/// One operation kind's options: the keys it accepts beside [`HOST_KEYS`], whether it
/// enforces governors, and the positional argument its twin takes, if any. The one table
/// an options object is validated against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OperationSpec {
    /// The operation's name, as refusals spell it.
    pub(crate) name: &'static str,
    /// The operation's own keys.
    pub(crate) keys: &'static [&'static str],
    /// Whether the operation enforces the governor ceilings.
    pub(crate) governed: bool,
    /// The name of the positional argument the twin passes (`regime`, `format`).
    pub(crate) argument: Option<&'static str>,
}

impl AsyncOperationKind {
    /// This kind's options.
    pub(crate) const fn spec(self) -> OperationSpec {
        const fn spec(
            name: &'static str,
            keys: &'static [&'static str],
            governed: bool,
            argument: Option<&'static str>,
        ) -> OperationSpec {
            OperationSpec {
                name,
                keys,
                governed,
                argument,
            }
        }
        match self {
            Self::Query => spec("query", &["base"], false, None),
            Self::Raw => spec(
                "raw",
                &["base", "format", "provenanceNamespace", "optionsJson"],
                false,
                None,
            ),
            Self::RawWithContext => spec(
                "rawWithContext",
                &["base", "yamlSchemaUrl"],
                false,
                Some("format"),
            ),
            Self::Governed => spec("governed", &GOVERNED_KEYS, true, None),
            Self::EntailmentGoverned => spec(
                "entailmentGoverned",
                &[
                    "base",
                    "aggregateNamespace",
                    "fuel",
                    "deadlineMs",
                    "maxAnswers",
                    "maxIntermediateCells",
                    "maxScratchBytes",
                    "maxRemoteRequests",
                    "program",
                ],
                true,
                Some("regime"),
            ),
            Self::Update => spec("update", &["base"], false, None),
            // `maxAnswers` is accepted so it is refused by name when the job begins,
            // exactly as the synchronous twin refuses it.
            Self::UpdateGoverned => spec("updateGoverned", &GOVERNED_KEYS, true, None),
            Self::Negotiated => spec(
                "negotiated",
                &[
                    "base",
                    "aggregateNamespace",
                    "fuel",
                    "deadlineMs",
                    "maxAnswers",
                    "maxIntermediateCells",
                    "maxScratchBytes",
                    "maxRemoteRequests",
                    "accept",
                ],
                true,
                None,
            ),
            // EXPLAIN measures a run that is metered and never bounded: no ceiling.
            Self::Explain => spec("explain", &["base"], false, None),
            // The SHACL twins take their synchronous twin's arguments positionally, and
            // no ceiling: only the host options.
            Self::Shacl => spec("shacl", &[], false, None),
        }
    }

    fn accepts(self, key: &str) -> bool {
        self.spec().keys.contains(&key) || HOST_KEYS.contains(&key)
    }
}

/// One option as it was read off the options object.
#[derive(Debug, Clone)]
pub(crate) enum OptionValue {
    Text(String),
    Ceiling(i64),
    Count(f64),
    Provenance {
        prefix: String,
        iri: String,
    },
    Catalog(NativeServiceCatalog),
    LocalServices(Vec<(String, Arc<RdfDataset>)>),
    /// A key no operation reads; refused by name.
    Unknown,
}

/// Why an options object was refused: a key or value of the wrong type is a `TypeError`,
/// every other refusal an `Error`; both carry [`OPTIONS_CODE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionsError {
    message: String,
    type_error: bool,
}

impl OptionsError {
    fn type_error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            type_error: true,
        }
    }

    fn refused(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            type_error: false,
        }
    }

    fn to_js(&self) -> JsValue {
        if self.type_error {
            coded_type_error(&self.message, OPTIONS_CODE)
        } else {
            coded_error(&self.message, OPTIONS_CODE)
        }
    }
}

impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// The configuration of one asynchronous job: an options object read and validated
/// against its operation's [`OperationSpec`] ([`Self::from_js`]).
///
/// Every option a kind would ignore is refused by name rather than dropped: an option a
/// caller believes applies and that nothing enforces is the silent hole this surface
/// exists to close. The ceilings are 64-bit integers exactly as the synchronous governed
/// entries take them; `yieldEveryPolls` and `stackBytes` are refused unless they are
/// integers in range, so a negative never wraps into a huge value.
#[wasm_bindgen]
#[derive(Debug, Default, Clone)]
pub struct AsyncJobOptions {
    kind: Option<AsyncOperationKind>,
    base: Option<String>,
    format: Option<String>,
    options_json: Option<String>,
    yaml_schema_url: Option<String>,
    provenance: Option<(String, String)>,
    aggregate_namespace: Option<String>,
    regime: Option<String>,
    program: Option<String>,
    accept: Option<String>,
    ceilings: GovernorArgs,
    quantum: u32,
    stack_bytes: u32,
    catalog: Option<NativeServiceCatalog>,
    local_services: Vec<(String, Arc<RdfDataset>)>,
    service_handler: Option<u32>,
    load_handler: bool,
}

/// Read an integer option in `[min, u32::MAX]` from a JS `number`.
fn count_option(name: &str, value: Option<f64>, min: u32, default: u32) -> Result<u32, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    if value.is_finite()
        && value.fract() == 0.0
        && value >= f64::from(min)
        && value <= f64::from(u32::MAX)
    {
        Ok(value as u32)
    } else {
        Err(format!(
            "{name} must be an integer from {min} to {} inclusive, got {value}",
            u32::MAX
        ))
    }
}

#[wasm_bindgen]
extern "C" {
    /// A value an option is read from.
    type OptionSource;

    #[wasm_bindgen(js_namespace = Object, js_name = keys)]
    fn object_keys(value: &JsValue) -> Vec<String>;

    #[wasm_bindgen(js_namespace = Array, js_name = isArray)]
    fn is_array(value: &JsValue) -> bool;

    #[wasm_bindgen(js_namespace = Reflect, js_name = get, catch)]
    fn reflect_get(target: &JsValue, key: &str) -> Result<JsValue, JsValue>;

    /// `ServiceCatalog#copy`, called on whatever was passed as the catalog.
    #[wasm_bindgen(method, catch, js_name = copy)]
    fn copy(this: &OptionSource) -> Result<JsValue, JsValue>;

    /// `Dataset#snapshot`, called on whatever was passed as a local service.
    #[wasm_bindgen(method, catch, js_name = snapshot)]
    fn snapshot(this: &OptionSource) -> Result<JsValue, JsValue>;
}

/// Whether `value` is `undefined` or `null`: an option left unset.
fn is_unset(value: &JsValue) -> bool {
    value.is_undefined() || value.is_null()
}

/// Read option `key` of shape `shape` from `value`.
fn read_option(
    key: &str,
    shape: OptionShape,
    value: &JsValue,
) -> Result<OptionValue, OptionsError> {
    match shape {
        OptionShape::Text => value.as_string().map(OptionValue::Text).ok_or_else(|| {
            OptionsError::type_error(format!("query option {key} must be a string when supplied"))
        }),
        OptionShape::Count => value.as_f64().map(OptionValue::Count).ok_or_else(|| {
            OptionsError::type_error(format!("query option {key} must be a number when supplied"))
        }),
        OptionShape::Ceiling => read_ceiling(key, value).map(OptionValue::Ceiling),
        OptionShape::Provenance => {
            if !value.is_object() {
                return Err(OptionsError::type_error(
                    "query option provenanceNamespace must be an object ({ prefix, iri }) when \
                     supplied",
                ));
            }
            let part = |name: &str| {
                reflect_get(value, name)
                    .ok()
                    .and_then(|part| part.as_string())
            };
            match (part("prefix"), part("iri")) {
                (Some(prefix), Some(iri)) => Ok(OptionValue::Provenance { prefix, iri }),
                _ => Err(OptionsError::type_error(
                    "query option provenanceNamespace must supply both a string `prefix` and \
                     a string `iri`",
                )),
            }
        }
        OptionShape::Catalog => {
            let refused = || {
                OptionsError::type_error(
                    "query option catalog must be a ServiceCatalog when supplied",
                )
            };
            if !value.is_object() {
                return Err(refused());
            }
            let copy = value
                .unchecked_ref::<OptionSource>()
                .copy()
                .map_err(|_| refused())?;
            ServiceCatalog::try_from_js_value(copy)
                .map(|catalog| OptionValue::Catalog(catalog.inner))
                .map_err(|_| refused())
        }
        OptionShape::LocalServices => {
            if !value.is_object() || is_array(value) {
                return Err(OptionsError::type_error(
                    "query option localServices must be an object mapping endpoint IRIs to \
                     Datasets",
                ));
            }
            let mut services = Vec::new();
            for endpoint in object_keys(value) {
                let refused = || {
                    OptionsError::type_error(format!(
                        "query option localServices must map every endpoint to a Dataset; \
                         {endpoint:?} does not"
                    ))
                };
                let dataset = reflect_get(value, &endpoint).map_err(|_| refused())?;
                if !dataset.is_object() {
                    return Err(refused());
                }
                let snapshot = dataset
                    .unchecked_ref::<OptionSource>()
                    .snapshot()
                    .map_err(|_| refused())?;
                let snapshot = Dataset::try_from_js_value(snapshot).map_err(|_| refused())?;
                let frozen = snapshot
                    .view()
                    .freeze()
                    .map_err(|diagnostic| OptionsError::refused(diagnostic.to_string()))?;
                services.push((endpoint, frozen));
            }
            Ok(OptionValue::LocalServices(services))
        }
    }
}

/// Read a governor ceiling: an integer as a `bigint`, a safe-integer `number` or an
/// integral string. Its sign is checked when the job begins, with the synchronous
/// entries' own message.
fn read_ceiling(key: &str, value: &JsValue) -> Result<i64, OptionsError> {
    let refused = || {
        OptionsError::type_error(format!(
            "query option {key} must be an integer (number, bigint, or integral string)"
        ))
    };
    if value.is_bigint() {
        return i64::try_from(value.clone()).map_err(|_| refused());
    }
    if let Some(number) = value.as_f64() {
        const MAX_SAFE: f64 = 9_007_199_254_740_991.0;
        return (number.fract() == 0.0 && number.abs() <= MAX_SAFE)
            .then_some(number as i64)
            .ok_or_else(refused);
    }
    if let Some(text) = value.as_string() {
        let text = text.trim();
        let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
        if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return text
                .strip_prefix('+')
                .unwrap_or(text)
                .parse::<i64>()
                .map_err(|_| refused());
        }
    }
    Err(refused())
}

#[wasm_bindgen]
impl AsyncJobOptions {
    /// Read and validate the options object of an operation of `kind`.
    ///
    /// `options` is the twin's options object without the keys the package root reads
    /// itself (`resolveService`, `resolveLoad`, `signal`); `serviceHandler` identifies
    /// the host's `resolveService` handler when it supplied one (only requests to the
    /// same handler share a host call), and `loadHandler` says whether it supplied
    /// `resolveLoad`. `argument` is the twin's positional argument — the regime of
    /// `entailmentGoverned`, the format of `rawWithContext` — and must be absent for
    /// every other kind.
    ///
    /// # Errors
    ///
    /// A `TypeError` for a key the operation does not take or a value of the wrong type,
    /// and an `Error` for a value it cannot honor; both carry the code
    /// `purrdf-wasm-options`.
    #[wasm_bindgen(js_name = fromJs)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn from_js(
        kind: AsyncOperationKind,
        options: JsValue,
        service_handler: Option<u32>,
        load_handler: bool,
        argument: Option<String>,
    ) -> Result<Self, JsValue> {
        let entries = Self::read(&options).map_err(|error| error.to_js())?;
        Self::from_entries(kind, entries, service_handler, load_handler, argument)
            .map_err(|error| error.to_js())
    }
}

impl AsyncJobOptions {
    /// Read the set keys of `options` by their shapes; a key no operation reads is
    /// [`OptionValue::Unknown`].
    fn read(options: &JsValue) -> Result<Vec<(String, OptionValue)>, OptionsError> {
        if is_unset(options) {
            return Ok(Vec::new());
        }
        if !options.is_object() || is_array(options) {
            return Err(OptionsError::type_error(
                "query options must be an object when supplied",
            ));
        }
        let mut entries = Vec::new();
        for key in object_keys(options) {
            let value = reflect_get(options, &key).map_err(|_| {
                OptionsError::type_error(format!("query option {key} could not be read"))
            })?;
            if is_unset(&value) {
                continue;
            }
            let read = match option_shape(&key) {
                Some(shape) => read_option(&key, shape, &value)?,
                None => OptionValue::Unknown,
            };
            entries.push((key, read));
        }
        Ok(entries)
    }

    /// Validate `entries` for `kind`: every key the kind does not take is refused by
    /// name, then every value the kind cannot honor.
    pub(crate) fn from_entries(
        kind: AsyncOperationKind,
        entries: Vec<(String, OptionValue)>,
        service_handler: Option<u32>,
        load_handler: bool,
        argument: Option<String>,
    ) -> Result<Self, OptionsError> {
        let spec = kind.spec();
        let op = spec.name;
        for (key, _) in &entries {
            if kind.accepts(key) {
                continue;
            }
            if key == "cancel" {
                return Err(OptionsError::type_error(
                    "query option cancel is not accepted by an asynchronous call; pass an \
                     AbortSignal as signal instead",
                ));
            }
            if !spec.governed && CEILING_KEYS.contains(&key.as_str()) {
                return Err(OptionsError::type_error(format!(
                    "query option {key} is an execution governor and is enforced only by \
                     queryGovernedAsync/queryEntailmentGovernedAsync/updateGovernedAsync; this \
                     call would ignore it entirely"
                )));
            }
            if !spec.governed && key == "aggregateNamespace" {
                return Err(OptionsError::type_error(
                    "query option aggregateNamespace registers the statistical-aggregate \
                     registry and is honored only by queryGovernedAsync/\
                     queryEntailmentGovernedAsync/updateGovernedAsync; this call would ignore \
                     it entirely",
                ));
            }
            let accepted: Vec<&str> = spec
                .keys
                .iter()
                .chain(HOST_KEYS.iter())
                .chain(PACKAGE_KEYS.iter())
                .copied()
                .collect();
            return Err(OptionsError::type_error(format!(
                "unknown query option {key:?} (this call accepts {})",
                accepted.join(", ")
            )));
        }
        match (spec.argument, &argument) {
            (Some(_), Some(_)) | (None, None) => {}
            (Some(name), None) => {
                return Err(OptionsError::refused(format!(
                    "a {op} operation needs a {name}"
                )));
            }
            (None, Some(_)) => {
                return Err(OptionsError::refused(format!(
                    "a {op} operation takes no positional argument"
                )));
            }
        }
        let mut options = Self {
            kind: Some(kind),
            service_handler,
            load_handler,
            ..Self::default()
        };
        match spec.argument {
            Some("regime") => options.regime = argument,
            Some(_) => options.format = argument,
            None => {}
        }
        let mut ceilings: BTreeMap<&str, i64> = BTreeMap::new();
        let mut counts: BTreeMap<&str, f64> = BTreeMap::new();
        for (key, value) in entries {
            match (key.as_str(), value) {
                ("base", OptionValue::Text(text)) => options.base = Some(text),
                ("format", OptionValue::Text(text)) => options.format = Some(text),
                ("optionsJson", OptionValue::Text(text)) => options.options_json = Some(text),
                ("yamlSchemaUrl", OptionValue::Text(text)) => options.yaml_schema_url = Some(text),
                ("aggregateNamespace", OptionValue::Text(text)) => {
                    options.aggregate_namespace = Some(text);
                }
                ("program", OptionValue::Text(text)) => options.program = Some(text),
                ("accept", OptionValue::Text(text)) => options.accept = Some(text),
                ("provenanceNamespace", OptionValue::Provenance { prefix, iri }) => {
                    options.provenance = Some((prefix, iri));
                }
                ("catalog", OptionValue::Catalog(catalog)) => options.catalog = Some(catalog),
                ("localServices", OptionValue::LocalServices(services)) => {
                    for (endpoint, frozen) in services {
                        options
                            .add_local_frozen(endpoint, frozen)
                            .map_err(OptionsError::refused)?;
                    }
                }
                (key, OptionValue::Ceiling(value)) => {
                    let name = CEILING_KEYS
                        .iter()
                        .find(|name| **name == key)
                        .ok_or_else(|| {
                            OptionsError::type_error(format!("{key} is not a ceiling"))
                        })?;
                    ceilings.insert(name, value);
                }
                (key, OptionValue::Count(value)) => {
                    let name = HOST_KEYS
                        .iter()
                        .find(|name| **name == key)
                        .ok_or_else(|| OptionsError::type_error(format!("{key} is not a count")))?;
                    counts.insert(name, value);
                }
                (key, _) => {
                    return Err(OptionsError::type_error(format!(
                        "query option {key} was read as a value it cannot hold"
                    )));
                }
            }
        }
        if kind == AsyncOperationKind::UpdateGoverned && ceilings.contains_key("maxAnswers") {
            return Err(OptionsError::refused(UPDATE_REFUSES_MAX_ANSWERS));
        }
        options.ceilings = GovernorArgs::decode(
            ceilings.get("fuel").copied(),
            ceilings.get("deadlineMs").copied(),
            ceilings.get("maxAnswers").copied(),
            ceilings.get("maxIntermediateCells").copied(),
            ceilings.get("maxScratchBytes").copied(),
            ceilings.get("maxRemoteRequests").copied(),
        )
        .map_err(OptionsError::refused)?;
        options.quantum = count_option(
            "yieldEveryPolls",
            counts.get("yieldEveryPolls").copied(),
            0,
            DEFAULT_YIELD_EVERY_POLLS,
        )
        .map_err(OptionsError::refused)?;
        options.stack_bytes = count_option(
            "stackBytes",
            counts.get("stackBytes").copied(),
            MIN_STACK_BYTES,
            DEFAULT_STACK_BYTES,
        )
        .map_err(OptionsError::refused)?;
        options.check()?;
        Ok(options)
    }

    fn add_local_frozen(
        &mut self,
        endpoint: String,
        frozen: Arc<RdfDataset>,
    ) -> Result<(), String> {
        if self
            .local_services
            .iter()
            .any(|(existing, _)| *existing == endpoint)
        {
            return Err(format!("local service <{endpoint}> is declared twice"));
        }
        self.local_services.push((endpoint, frozen));
        Ok(())
    }

    /// The refusals no key table can express: combinations of options, and a catalog
    /// with nothing to govern.
    fn check(&self) -> Result<(), OptionsError> {
        if self.options_json.is_some() && self.format.is_none() {
            return Err(OptionsError::refused(
                "optionsJson needs a format: a configured serialization names the JSON-LD or \
                 YAML-LD format it configures",
            ));
        }
        if self.options_json.is_some() && self.provenance.is_some() {
            return Err(OptionsError::refused(
                "provenanceNamespace applies to SPARQL results documents, not to a configured \
                 JSON-LD graph serialization",
            ));
        }
        if self.catalog.is_some() && self.service_handler.is_none() && !self.load_handler {
            return Err(OptionsError::refused(
                "a service catalog governs host-resolved SERVICE and LOAD requests, and neither \
                 a resolveService nor a resolveLoad handler was supplied; it would govern \
                 nothing",
            ));
        }
        Ok(())
    }

    /// The kind these options were validated for; refused for another.
    fn require_kind(&self, kind: AsyncOperationKind) -> Result<(), String> {
        match self.kind {
            Some(validated) if validated == kind => Ok(()),
            Some(validated) => Err(format!(
                "these options were validated for a {} operation, not a {} one",
                validated.name(),
                kind.name()
            )),
            None => Err("these options were never validated for an operation".to_owned()),
        }
    }
}

// ---------------------------------------------------------------------------
// The service catalog
// ---------------------------------------------------------------------------

/// The per-service policy host-resolved `SERVICE` requests and `LOAD` fetches are
/// authorized against, before the host is ever called: deny by default, one profile per
/// endpoint (or `LOAD` source), an optional fallback. Each profile is JSON:
///
/// ```json
/// {
///   "capabilities": ["query", "network", "credentials"],
///   "headers": [["X-Tenant", "a"], ["Accept-Language", "en"]],
///   "credential": { "header": "Authorization", "value": "Bearer …" },
///   "userAgent": "my-worker/1.0",
///   "timeoutMs": 5000
/// }
/// ```
///
/// `capabilities` is required; a host-resolved `SERVICE` request needs `query` and
/// `network`, a `LOAD` fetch `network`, and a `credential` needs `credentials` too (the
/// native rule: a credential that may not be sent refuses the request rather than sending
/// it without). `timeoutMs` bounds each request to the service: the host abandons one
/// unanswered after it, or at the job's deadline when that falls first. `headers` is an array of
/// `[name, value]` pairs because order and repeated names are significant. Unknown keys
/// and capability names are refused.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct ServiceCatalog {
    inner: NativeServiceCatalog,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ProfileJson {
    capabilities: Vec<String>,
    #[serde(default)]
    headers: Option<serde_json::Value>,
    #[serde(default)]
    credential: Option<CredentialJson>,
    #[serde(default)]
    user_agent: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialJson {
    header: String,
    value: String,
}

// Hand-written so the secret never reaches a log.
impl fmt::Debug for CredentialJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialJson")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

/// Whether `name` is an HTTP field name (RFC 9110 §5.1: a `token`).
fn is_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

/// Refuse a header a host `fetch` would throw on, with the reason, before any request.
fn check_header(name: &str, value: &str) -> Result<(), String> {
    if !is_header_name(name) {
        return Err(format!("header name {name:?} is not an HTTP token"));
    }
    if value.contains(['\r', '\n', '\0']) {
        return Err(format!(
            "the value of header {name:?} contains a line break or NUL"
        ));
    }
    Ok(())
}

/// Parse one service profile document.
fn parse_profile(json: &str) -> Result<ServiceProfile, String> {
    let profile: ProfileJson =
        serde_json::from_str(json).map_err(|error| format!("service profile: {error}"))?;
    let mut capabilities = ServiceCapabilities::NONE;
    for name in &profile.capabilities {
        let capability = match name.as_str() {
            "query" => ServiceCapability::Query,
            "network" => ServiceCapability::Network,
            "credentials" => ServiceCapability::Credentials,
            other => {
                return Err(format!(
                    "service profile: unknown capability {other:?} (expected query, network \
                     or credentials)"
                ));
            }
        };
        capabilities = capabilities.grant(capability);
    }
    let mut out = ServiceProfile::new(capabilities);
    match profile.headers {
        None => {}
        Some(serde_json::Value::Array(pairs)) => {
            for pair in pairs {
                let (name, value) = match pair.as_array().map(Vec::as_slice) {
                    Some(
                        [
                            serde_json::Value::String(name),
                            serde_json::Value::String(value),
                        ],
                    ) => (name.clone(), value.clone()),
                    _ => {
                        return Err(format!(
                            "service profile: every header must be a [name, value] pair of \
                             strings, got {pair}"
                        ));
                    }
                };
                check_header(&name, &value).map_err(|error| format!("service profile: {error}"))?;
                out = out.with_header(name, value);
            }
        }
        Some(_) => {
            return Err(
                "service profile: headers must be an array of [name, value] pairs (order and \
                 repeated names are significant, which an object cannot express)"
                    .to_owned(),
            );
        }
    }
    if let Some(credential) = profile.credential {
        check_header(&credential.header, &credential.value)
            .map_err(|error| format!("service profile credential: {error}"))?;
        out = out.with_credential(ServiceCredential::Header {
            name: credential.header,
            value: credential.value,
        });
    }
    if let Some(user_agent) = profile.user_agent {
        check_header("User-Agent", &user_agent)
            .map_err(|error| format!("service profile userAgent: {error}"))?;
        out = out.with_user_agent(user_agent);
    }
    if let Some(ms) = profile.timeout_ms {
        if ms == 0 || ms > MAX_TIMEOUT_MS {
            return Err(format!(
                "service profile: timeoutMs must be an integer from 1 to {MAX_TIMEOUT_MS} (the \
                 longest delay a JavaScript timer honours), got {ms}"
            ));
        }
        out = out.with_timeout(Duration::from_millis(ms));
    }
    Ok(out)
}

/// The longest request timeout a profile may name: the longest delay, in milliseconds, a
/// JavaScript timer honours (a signed 32-bit count). The host abandons an unanswered
/// request with such a timer, and a longer delay fires at once.
const MAX_TIMEOUT_MS: u64 = (1 << 31) - 1;

#[wasm_bindgen]
impl ServiceCatalog {
    /// An empty catalog: every service is denied.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the profile `profileJson` for the service IRI `endpoint`, replacing any
    /// earlier one.
    #[wasm_bindgen(js_name = addService)]
    pub fn add_service(&mut self, endpoint: String, profile_json: &str) -> Result<(), JsValue> {
        self.add_service_message(endpoint, profile_json)
            .map_err(|message| coded_error(&message, OPTIONS_CODE))
    }

    /// Apply the profile `profileJson` to every service with no entry of its own — the
    /// explicit opt-out of deny-by-default.
    #[wasm_bindgen(js_name = setFallback)]
    pub fn set_fallback(&mut self, profile_json: &str) -> Result<(), JsValue> {
        self.set_fallback_message(profile_json)
            .map_err(|message| coded_error(&message, OPTIONS_CODE))
    }

    /// An independent copy of this catalog: later changes to either leave the other as
    /// it is. An asynchronous operation's options take a copy of the catalog they name,
    /// so the caller's own stays usable for later operations.
    #[must_use]
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

impl ServiceCatalog {
    fn add_service_message(&mut self, endpoint: String, profile_json: &str) -> Result<(), String> {
        let profile = parse_profile(profile_json)?;
        self.inner = std::mem::take(&mut self.inner).with_service(endpoint, profile);
        Ok(())
    }

    fn set_fallback_message(&mut self, profile_json: &str) -> Result<(), String> {
        let profile = parse_profile(profile_json)?;
        self.inner = std::mem::take(&mut self.inner).with_fallback(profile);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Starting a job
// ---------------------------------------------------------------------------

/// Freeze `dataset`, build the operation and register the job. An update claims the
/// dataset first: a second asynchronous update of it cannot begin while this one is in
/// flight.
fn begin_job(
    engine: &Rc<NativeSparqlEngine>,
    dataset: &Dataset,
    kind: AsyncOperationKind,
    sparql: String,
    options: &AsyncJobOptions,
    jsonld: Option<JsonLdSerializeOptions>,
) -> Result<AsyncJob, JobError> {
    options
        .require_kind(kind)
        .map_err(|message| JobError::message(OPTIONS_CODE, message))?;
    let provenance = match &options.provenance {
        None => None,
        Some((prefix, iri)) => Some(
            ProvenanceNamespace::new(prefix.clone(), iri.clone())
                .map_err(|error| JobError::message(OPTIONS_CODE, error.to_string()))?,
        ),
    };
    let update_claim = match kind {
        AsyncOperationKind::Update | AsyncOperationKind::UpdateGoverned => {
            Some(dataset.claim_update().ok_or_else(|| {
                JobError::message(
                    FailureCode::UpdateInFlight.code(),
                    format!(
                        "an asynchronous update of dataset {} is already in flight; await it \
                         before beginning another",
                        dataset.identity()
                    ),
                )
            })?)
        }
        _ => None,
    };
    let freeze_started = now_ms();
    let frozen = dataset.view().freeze().map_err(JobError::diagnostic)?;
    let freeze_ms = now_ms() - freeze_started;
    let input = OperationInput {
        kind,
        engine: Rc::clone(engine),
        frozen,
        sparql: Cow::Owned(sparql),
        base: options.base.clone().map(Cow::Owned),
        aggregate_namespace: options.aggregate_namespace.clone(),
        ceilings: options.ceilings,
        format: options.format.clone(),
        provenance,
        jsonld,
        regime: options.regime.clone(),
        program: options.program.clone(),
        accept: options.accept.clone(),
    };
    Ok(register_operation(
        kind,
        sparql_operation(input),
        options,
        freeze_ms,
        (dataset.identity(), dataset.current_generation()),
        update_claim,
    ))
}

/// Register a job of `kind` that runs `operation` under `options`. `dataset` is the
/// identity and generation an update's commit is checked against (`(0, 0)` for an
/// operation that reads no dataset).
fn register_operation(
    kind: AsyncOperationKind,
    operation: Operation,
    options: &AsyncJobOptions,
    freeze_ms: f64,
    dataset: (u64, u64),
    update_claim: Option<UpdateClaim>,
) -> AsyncJob {
    let region = StackRegion::new(options.stack_bytes as usize);
    let inner = register_job(|id| {
        let slots = Arc::new(JobSlots::new(id, region.bounds));
        add_ms(&slots.counters.freeze_ms, freeze_ms);
        let watch = Arc::new(JspiStopWatch::new(
            slots,
            options.ceilings.deadline_ms(),
            options.quantum,
        ));
        JobInner {
            id,
            kind,
            watch,
            operation: RefCell::new(Some(operation)),
            service_handler: options.service_handler,
            load_handler: options.load_handler,
            catalog: options.catalog.clone(),
            local_services: options.local_services.clone(),
            dataset_id: dataset.0,
            dataset_generation: dataset.1,
            update_claim: RefCell::new(update_claim),
            region,
            state: Cell::new(JobState::Pending),
            outcome: RefCell::new(None),
            error: RefCell::new(None),
            pending_commit: RefCell::new(None),
            refusal: RefCell::new(None),
        }
    });
    AsyncJob { inner }
}

/// Match the arguments to `operation` and register the SHACL job `options` were
/// validated for. Native-testable core of [`AsyncJob::begin_shacl`].
fn begin_shacl_job(
    operation: ShaclAsyncOperation,
    arguments: ShaclArguments,
    options: &AsyncJobOptions,
) -> Result<AsyncJob, JobError> {
    options
        .require_kind(AsyncOperationKind::Shacl)
        .map_err(|message| JobError::message(OPTIONS_CODE, message))?;
    let request = ShaclRequest::build(operation, arguments)
        .map_err(|message| JobError::message(OPTIONS_CODE, message))?;
    Ok(register_operation(
        AsyncOperationKind::Shacl,
        Box::new(move |run| execute_shacl(request, run)),
        options,
        0.0,
        (0, 0),
        None,
    ))
}

/// A refusal to begin a job, thrown before any job exists.
fn begin_refused(error: &JobError) -> JsValue {
    error.to_js(Lane::Async {
        region_bytes: DEFAULT_STACK_BYTES as usize,
    })
}

#[wasm_bindgen]
impl AsyncJob {
    /// Start an asynchronous SHACL operation: the twin of the synchronous entry
    /// `operation` names, over the same arguments, under `options` (from
    /// `AsyncJobOptions.fromJs` for the `shacl` kind).
    ///
    /// `shapesTtl`, `shapesBase`, `addedNt` and `removedNt` are the text-shapes entries'
    /// arguments and `product` and `expectIdentity` the product entries'; an argument the
    /// operation does not take is refused by name. The documents are parsed when the job
    /// runs, so a parse error — like a product refusal — is the job's error, with the
    /// synchronous twin's words.
    ///
    /// A static constructor rather than a free function: it is the package root's
    /// plumbing, reached through its `shacl…Async` twins, never a consumer entry point.
    ///
    /// # Errors
    ///
    /// Options validated for another kind, or an argument the operation does not take or
    /// is missing.
    #[wasm_bindgen(js_name = beginShacl)]
    #[allow(clippy::too_many_arguments)] // one argument per synchronous twin's argument
    pub fn begin_shacl(
        operation: ShaclAsyncOperation,
        options: &AsyncJobOptions,
        data_nt: String,
        shapes_ttl: Option<String>,
        shapes_base: Option<String>,
        added_nt: Option<String>,
        removed_nt: Option<String>,
        product: Option<Vec<u8>>,
        expect_identity: Option<String>,
    ) -> Result<Self, JsValue> {
        begin_shacl_job(
            operation,
            ShaclArguments {
                shapes: shapes_ttl,
                shapes_base,
                data: data_nt,
                added: added_nt,
                removed: removed_nt,
                product,
                expect_identity,
            },
            options,
        )
        .map_err(|error| begin_refused(&error))
    }
}

#[wasm_bindgen]
impl QueryEngine {
    /// Start an asynchronous operation of `kind` over a snapshot of `dataset`, under
    /// `options` (from `AsyncJobOptions.fromJs` for the same kind).
    ///
    /// The dataset is frozen now: a query sees this snapshot whatever happens to the
    /// dataset afterwards, and an update captures its identity and generation so
    /// `commitUpdate` can refuse to overwrite a mutation made while it ran. An update
    /// also claims the dataset until the job is finished: a second asynchronous update of
    /// it is refused (`native-sparql-update-in-flight`) rather than queued. The SPARQL
    /// text is parsed when the job runs, so a parse error is the job's error, with the
    /// synchronous twin's words. A `rawWithContext` operation starts through
    /// `beginAsyncWithContext` instead.
    ///
    /// # Errors
    ///
    /// Options validated for another kind, a malformed JSON-LD options document or
    /// provenance namespace, an update already in flight on `dataset`, or a dataset that
    /// cannot be frozen.
    #[wasm_bindgen(js_name = beginAsync)]
    pub fn begin_async(
        &self,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: String,
        options: &AsyncJobOptions,
    ) -> Result<AsyncJob, JsValue> {
        let refused = |message: &str| begin_refused(&JobError::message(OPTIONS_CODE, message));
        if kind == AsyncOperationKind::RawWithContext {
            return Err(refused(
                "a rawWithContext operation starts through beginAsyncWithContext",
            ));
        }
        if kind == AsyncOperationKind::Shacl {
            return Err(refused(SHACL_STARTS_ELSEWHERE));
        }
        let jsonld = match (kind, options.options_json.as_deref()) {
            (AsyncOperationKind::Raw, Some(json)) => Some(decode_options(json)?),
            _ => None,
        };
        begin_job(self.engine(), dataset, kind, sparql, options, jsonld)
            .map_err(|error| begin_refused(&error))
    }

    /// Start a `rawWithContext` operation: a CONSTRUCT/DESCRIBE serialized under a
    /// compiled JSON-LD context (and, for YAML-LD, the options' `yamlSchemaUrl`), in the
    /// format the options were validated with.
    ///
    /// # Errors
    ///
    /// As [`Self::begin_async`], plus a kind other than `rawWithContext` and an invalid
    /// `yamlSchemaUrl`.
    #[wasm_bindgen(js_name = beginAsyncWithContext)]
    pub fn begin_async_with_context(
        &self,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: String,
        options: &AsyncJobOptions,
        context: &CompiledJsonLdContext,
    ) -> Result<AsyncJob, JsValue> {
        if kind != AsyncOperationKind::RawWithContext {
            return Err(begin_refused(&JobError::message(
                OPTIONS_CODE,
                format!(
                    "beginAsyncWithContext starts only a rawWithContext operation, not {}",
                    kind.name()
                ),
            )));
        }
        let mut jsonld = context_options(context);
        if let Some(url) = &options.yaml_schema_url {
            jsonld = jsonld.with_yaml_schema_url(url).map_err(|error| {
                begin_refused(&JobError::message(OPTIONS_CODE, error.to_string()))
            })?;
        }
        begin_job(self.engine(), dataset, kind, sparql, options, Some(jsonld))
            .map_err(|error| begin_refused(&error))
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::SparqlEngine as _;
    use purrdf_sparql_eval::{GovernedOutcome, QueryOptions};

    use super::*;
    use crate::query::sparql_request;

    const SEED_NT: &str = concat!(
        "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
        "<http://example.org/s2> <http://example.org/p> <http://example.org/o2> .\n",
    );
    const REMOTE_NT: &str =
        "<http://example.org/o> <http://example.org/q> <http://example.org/x> .";
    const ENDPOINT: &str = "http://example.org/sparql";
    const SRJ: &[u8] = br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"uri","value":"http://example.org/r"}}]}}"#;

    fn seed() -> Dataset {
        Dataset::parse(SEED_NT, "ntriples", None).expect("seed parses")
    }

    /// An options object as a twin passes it: entries by key, the handlers and the
    /// positional argument.
    #[derive(Debug, Default, Clone)]
    struct Opts {
        entries: Vec<(String, OptionValue)>,
        service_handler: Option<u32>,
        load_handler: bool,
        argument: Option<String>,
    }

    impl Opts {
        fn with(mut self, key: &str, value: OptionValue) -> Self {
            self.entries.push((key.to_owned(), value));
            self
        }

        fn text(self, key: &str, value: &str) -> Self {
            self.with(key, OptionValue::Text(value.to_owned()))
        }

        fn ceiling(self, key: &str, value: i64) -> Self {
            self.with(key, OptionValue::Ceiling(value))
        }

        fn count(self, key: &str, value: f64) -> Self {
            self.with(key, OptionValue::Count(value))
        }

        fn handlers(mut self, service: bool, load: bool) -> Self {
            self.service_handler = service.then_some(1);
            self.load_handler = load;
            self
        }

        fn argument(mut self, argument: &str) -> Self {
            self.argument = Some(argument.to_owned());
            self
        }

        fn set(&mut self, key: &str, value: OptionValue) {
            self.entries.push((key.to_owned(), value));
        }

        fn set_fuel(&mut self, value: Option<i64>) {
            if let Some(value) = value {
                self.set("fuel", OptionValue::Ceiling(value));
            }
        }

        fn set_deadline_ms(&mut self, value: Option<i64>) {
            if let Some(value) = value {
                self.set("deadlineMs", OptionValue::Ceiling(value));
            }
        }

        fn set_max_answers(&mut self, value: Option<i64>) {
            if let Some(value) = value {
                self.set("maxAnswers", OptionValue::Ceiling(value));
            }
        }

        fn set_accept(&mut self, value: Option<String>) {
            if let Some(value) = value {
                self.set("accept", OptionValue::Text(value));
            }
        }

        /// Serve `endpoint` from `frozen`, in the one `localServices` entry.
        fn add_local_frozen(
            &mut self,
            endpoint: String,
            frozen: Arc<RdfDataset>,
        ) -> Result<(), String> {
            let existing = self
                .entries
                .iter_mut()
                .find_map(|(key, value)| match value {
                    OptionValue::LocalServices(services) if key == "localServices" => {
                        Some(services)
                    }
                    _ => None,
                });
            match existing {
                Some(services) => services.push((endpoint, frozen)),
                None => self.set(
                    "localServices",
                    OptionValue::LocalServices(vec![(endpoint, frozen)]),
                ),
            }
            Ok(())
        }

        fn validate(self, kind: AsyncOperationKind) -> Result<AsyncJobOptions, String> {
            AsyncJobOptions::from_entries(
                kind,
                self.entries,
                self.service_handler,
                self.load_handler,
                self.argument,
            )
            .map_err(|error| error.to_string())
        }
    }

    fn options() -> Opts {
        Opts::default()
    }

    /// Validate and begin, as `beginAsync` does, without the wasm-only reading.
    fn begin(
        engine: &QueryEngine,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: &str,
        options: impl std::borrow::Borrow<Opts>,
    ) -> AsyncJob {
        let options = options
            .borrow()
            .clone()
            .validate(kind)
            .expect("options are valid");
        begin_job(
            engine.engine(),
            dataset,
            kind,
            sparql.to_owned(),
            &options,
            None,
        )
        .expect("the job begins")
    }

    fn raw_text(job: &AsyncJob) -> String {
        String::from_utf8(job.take_raw_bytes().expect("raw bytes")).expect("UTF-8")
    }

    fn slots() -> JobSlots {
        JobSlots::new(
            1,
            StackBounds {
                base: 0x10_0000,
                top: 0x30_0000,
            },
        )
    }

    fn catalog(entries: &[(&str, &str)]) -> ServiceCatalog {
        let mut catalog = ServiceCatalog::new();
        for (endpoint, profile) in entries {
            catalog
                .add_service_message((*endpoint).to_owned(), profile)
                .expect("profile parses");
        }
        catalog
    }

    // ── Options: every refusal beside the valid neighbour it must not catch ─────────

    #[test]
    fn yield_every_polls_refuses_a_negative_and_accepts_zero() {
        let error = options()
            .count("yieldEveryPolls", -1.0)
            .validate(AsyncOperationKind::Query)
            .expect_err("a negative quantum is refused");
        assert!(error.contains("yieldEveryPolls"), "{error}");
        assert!(
            options()
                .count("yieldEveryPolls", 1.5)
                .validate(AsyncOperationKind::Query)
                .is_err()
        );
        let zero = options()
            .count("yieldEveryPolls", 0.0)
            .validate(AsyncOperationKind::Query)
            .expect("zero yields at every poll");
        assert_eq!(zero.quantum, 0);
        let default = options()
            .validate(AsyncOperationKind::Query)
            .expect("defaults are valid");
        assert_eq!(default.quantum, DEFAULT_YIELD_EVERY_POLLS);
    }

    #[test]
    fn stack_bytes_refuses_a_region_below_the_minimum_and_accepts_the_minimum() {
        let error = options()
            .count("stackBytes", 4096.0)
            .validate(AsyncOperationKind::Query)
            .expect_err("a 4 KiB region is refused");
        assert!(
            error.contains("stackBytes") && error.contains("524288"),
            "{error}"
        );
        let minimum = options()
            .count("stackBytes", 524_288.0)
            .validate(AsyncOperationKind::Query)
            .expect("the minimum is accepted");
        assert_eq!(minimum.stack_bytes, 524_288);
        assert_eq!(
            options()
                .validate(AsyncOperationKind::Query)
                .expect("defaults")
                .stack_bytes,
            DEFAULT_STACK_BYTES
        );
    }

    #[test]
    fn an_ungoverned_operation_refuses_a_ceiling_the_governed_one_enforces() {
        let error = options()
            .ceiling("fuel", 10)
            .validate(AsyncOperationKind::Query)
            .expect_err("an ungoverned query would ignore fuel");
        assert!(
            error.contains("fuel") && error.contains("execution governor"),
            "{error}"
        );
        let governed = options()
            .ceiling("fuel", 10)
            .validate(AsyncOperationKind::Governed)
            .expect("a governed query enforces fuel");
        assert!(format!("{:?}", governed.ceilings).contains("fuel: Some(10)"));

        assert!(
            options()
                .text("aggregateNamespace", "http://example.org/agg#")
                .validate(AsyncOperationKind::Raw)
                .expect_err("an ungoverned raw query would ignore it")
                .contains("aggregateNamespace")
        );
        assert!(
            options()
                .text("aggregateNamespace", "http://example.org/agg#")
                .validate(AsyncOperationKind::Governed)
                .is_ok()
        );
    }

    #[test]
    fn a_negative_ceiling_is_refused_and_zero_is_a_ceiling() {
        let error = options()
            .ceiling("maxAnswers", -1)
            .validate(AsyncOperationKind::Governed)
            .expect_err("negative");
        assert!(error.contains("maxAnswers"), "{error}");
        assert!(
            options()
                .ceiling("maxAnswers", 0)
                .validate(AsyncOperationKind::Governed)
                .is_ok()
        );
    }

    #[test]
    fn a_governed_update_refuses_max_answers_and_a_governed_query_takes_it() {
        assert_eq!(
            options()
                .ceiling("maxAnswers", 5)
                .validate(AsyncOperationKind::UpdateGoverned)
                .expect_err("an UPDATE has no answer sequence"),
            UPDATE_REFUSES_MAX_ANSWERS
        );
        assert!(
            options()
                .ceiling("maxAnswers", 5)
                .validate(AsyncOperationKind::Governed)
                .is_ok()
        );
        assert!(
            options()
                .ceiling("fuel", 5)
                .validate(AsyncOperationKind::UpdateGoverned)
                .is_ok()
        );
    }

    #[test]
    fn a_key_the_operation_does_not_take_is_refused_by_name() {
        let error = options()
            .text("format", "json")
            .validate(AsyncOperationKind::Query)
            .expect_err("a typed result has no format");
        assert!(error.contains("unknown query option \"format\""), "{error}");
        assert!(
            error.contains("resolveService"),
            "the accepted keys are named: {error}"
        );
        assert!(
            options()
                .text("format", "json")
                .validate(AsyncOperationKind::Raw)
                .is_ok()
        );
        assert!(
            options()
                .with("retries", OptionValue::Unknown)
                .validate(AsyncOperationKind::Raw)
                .expect_err("a key no operation reads")
                .contains("retries")
        );
        assert!(
            options()
                .with("cancel", OptionValue::Unknown)
                .validate(AsyncOperationKind::Governed)
                .expect_err("a cancellation token")
                .contains("pass an AbortSignal as signal instead")
        );
    }

    #[test]
    fn the_positional_argument_is_required_by_its_kinds_and_refused_by_the_rest() {
        assert!(
            options()
                .validate(AsyncOperationKind::RawWithContext)
                .expect_err("a context serialization needs its format")
                .contains("needs a format")
        );
        let with_format = options()
            .argument("jsonld")
            .validate(AsyncOperationKind::RawWithContext)
            .expect("the format is the argument");
        assert_eq!(with_format.format.as_deref(), Some("jsonld"));
        assert!(
            options()
                .validate(AsyncOperationKind::EntailmentGoverned)
                .expect_err("an entailment query needs its regime")
                .contains("needs a regime")
        );
        let with_regime = options()
            .argument("rdfs")
            .validate(AsyncOperationKind::EntailmentGoverned)
            .expect("the regime is the argument");
        assert_eq!(with_regime.regime.as_deref(), Some("rdfs"));
        assert!(
            options()
                .argument("rdfs")
                .validate(AsyncOperationKind::Governed)
                .expect_err("a governed query takes no argument")
                .contains("takes no positional argument")
        );
    }

    #[test]
    fn options_json_needs_a_format_and_a_raw_operation() {
        assert!(
            options()
                .text("optionsJson", "{}")
                .validate(AsyncOperationKind::Raw)
                .is_err()
        );
        assert!(
            options()
                .text("optionsJson", "{}")
                .validate(AsyncOperationKind::Governed)
                .is_err()
        );
        assert!(
            options()
                .text("optionsJson", "{}")
                .text("format", "jsonld")
                .validate(AsyncOperationKind::Raw)
                .is_ok()
        );
    }

    #[test]
    fn provenance_is_refused_outside_a_raw_operation() {
        let provenance = || {
            options().with(
                "provenanceNamespace",
                OptionValue::Provenance {
                    prefix: "prov".to_owned(),
                    iri: "http://example.org/prov#".to_owned(),
                },
            )
        };
        assert!(
            provenance()
                .validate(AsyncOperationKind::Governed)
                .expect_err("a governed outcome has no results document")
                .contains("provenanceNamespace")
        );
        assert!(provenance().validate(AsyncOperationKind::Raw).is_ok());
    }

    #[test]
    fn a_catalog_without_a_handler_is_refused() {
        let catalog = || {
            OptionValue::Catalog(
                catalog(&[(ENDPOINT, r#"{"capabilities":["query","network"]}"#)]).inner,
            )
        };
        assert!(
            options()
                .with("catalog", catalog())
                .validate(AsyncOperationKind::Query)
                .expect_err("a catalog with nothing to govern")
                .contains("resolveService")
        );
        assert!(
            options()
                .with("catalog", catalog())
                .handlers(true, false)
                .validate(AsyncOperationKind::Query)
                .is_ok()
        );
        assert!(
            options()
                .with("catalog", catalog())
                .handlers(false, true)
                .validate(AsyncOperationKind::Update)
                .is_ok(),
            "a catalog governs LOAD too"
        );
    }

    #[test]
    fn a_local_service_declared_twice_is_refused() {
        let frozen = seed().view().freeze().expect("freeze");
        let services = |endpoints: &[&str]| {
            OptionValue::LocalServices(
                endpoints
                    .iter()
                    .map(|endpoint| ((*endpoint).to_owned(), Arc::clone(&frozen)))
                    .collect(),
            )
        };
        assert!(
            options()
                .with(
                    "localServices",
                    services(&[ENDPOINT, "http://example.org/other"])
                )
                .validate(AsyncOperationKind::Query)
                .is_ok()
        );
        assert!(
            options()
                .with("localServices", services(&[ENDPOINT, ENDPOINT]))
                .validate(AsyncOperationKind::Query)
                .expect_err("duplicate")
                .contains("twice")
        );
    }

    // ── The service catalog ────────────────────────────────────────────────────────

    #[test]
    fn a_profile_with_an_unknown_key_is_refused() {
        let error =
            parse_profile(r#"{"capabilities":["query"],"retries":3}"#).expect_err("unknown key");
        assert!(error.contains("retries"), "{error}");
        assert!(parse_profile(r#"{"capabilities":["query"],"timeoutMs":3}"#).is_ok());
        assert!(
            parse_profile("{}")
                .expect_err("capabilities are required")
                .contains("capabilities")
        );
    }

    /// A timeout the host's timer could not honour is refused; the longest one it can is
    /// accepted, and so is the shortest.
    #[test]
    fn a_profile_timeout_past_the_host_timer_range_is_refused() {
        for refused in ["0", "2147483648", "18446744073709551615"] {
            let error = parse_profile(&format!(
                r#"{{"capabilities":["query"],"timeoutMs":{refused}}}"#
            ))
            .expect_err("out of range");
            assert!(error.contains("timeoutMs"), "{refused}: {error}");
        }
        for accepted in [1_u64, MAX_TIMEOUT_MS] {
            let profile = parse_profile(&format!(
                r#"{{"capabilities":["query"],"timeoutMs":{accepted}}}"#
            ))
            .expect("in range");
            assert_eq!(profile.timeout(), Some(Duration::from_millis(accepted)));
        }
    }

    #[test]
    fn a_profile_with_an_unknown_capability_is_refused() {
        let error =
            parse_profile(r#"{"capabilities":["query","teleport"]}"#).expect_err("unknown name");
        assert!(error.contains("teleport"), "{error}");
        let profile = parse_profile(r#"{"capabilities":["query","network","credentials"]}"#)
            .expect("known names");
        for capability in [
            ServiceCapability::Query,
            ServiceCapability::Network,
            ServiceCapability::Credentials,
        ] {
            assert!(profile.capabilities().allows(capability));
        }
    }

    #[test]
    fn headers_must_be_ordered_pairs_of_valid_fields() {
        assert!(
            parse_profile(r#"{"capabilities":["query"],"headers":{"X-A":"1"}}"#)
                .expect_err("an object loses order")
                .contains("pairs")
        );
        assert!(
            parse_profile(r#"{"capabilities":["query"],"headers":[["Bad Name","1"]]}"#).is_err()
        );
        assert!(
            parse_profile(r#"{"capabilities":["query"],"headers":[["X-A","1\r\nX-B: 2"]]}"#)
                .is_err()
        );
        let profile = parse_profile(
            r#"{"capabilities":["query","network","credentials"],
                "headers":[["X-B","2"],["X-A","1"],["X-B","3"]],
                "credential":{"header":"Authorization","value":"Bearer t"},
                "userAgent":"worker/1","timeoutMs":250}"#,
        )
        .expect("valid profile");
        assert_eq!(
            profile.request_headers(),
            vec![
                ("X-B".to_owned(), "2".to_owned()),
                ("X-A".to_owned(), "1".to_owned()),
                ("X-B".to_owned(), "3".to_owned()),
                ("Authorization".to_owned(), "Bearer t".to_owned()),
            ],
            "profile headers in order, repeats kept, then the credential"
        );
        assert_eq!(profile.user_agent(), Some("worker/1"));
        assert_eq!(profile.timeout(), Some(Duration::from_millis(250)));
    }

    #[test]
    fn a_catalog_denies_by_default_and_authorizes_a_listed_service() {
        let mut catalog = ServiceCatalog::new();
        catalog
            .add_service_message(
                ENDPOINT.to_owned(),
                r#"{"capabilities":["query","network"]}"#,
            )
            .expect("profile parses");
        let needs =
            ServiceCapabilities::granting([ServiceCapability::Query, ServiceCapability::Network]);
        assert!(catalog.inner.authorize(ENDPOINT, needs).is_ok());
        assert!(
            catalog
                .inner
                .authorize("http://example.org/unlisted", needs)
                .is_err()
        );
        catalog
            .set_fallback_message(r#"{"capabilities":["query","network"]}"#)
            .expect("fallback parses");
        assert!(
            catalog
                .inner
                .authorize("http://example.org/unlisted", needs)
                .is_ok()
        );
    }

    // ── Effect mapping ──────────────────────────────────────────────────────────────

    const ANSWERED: SuspendStatus = SuspendStatus::Answered;

    fn bindings() -> Delivered {
        Delivered::Bindings(Arc::from(SRJ))
    }

    fn service(
        status: SuspendStatus,
        fired: Option<StopCause>,
        delivered: Option<Delivered>,
        slots: &JobSlots,
    ) -> Result<Vec<u8>, RemoteError> {
        service_answer(ENDPOINT, status, fired, delivered, slots).map(|bytes| bytes.to_vec())
    }

    #[test]
    fn a_fired_signal_is_reported_before_whatever_was_delivered() {
        let slots = slots();
        let abandoned = service(
            SuspendStatus::Abandoned,
            Some(StopCause::Deadline),
            None,
            &slots,
        );
        assert_eq!(
            abandoned,
            Err(RemoteError::Governed(TrippedGovernor::Stopped {
                cause: StopCause::Deadline
            })),
            "an abandoned exchange keeps the positional prefix"
        );
        let completed = service(
            ANSWERED,
            Some(StopCause::Cancelled),
            Some(bindings()),
            &slots,
        );
        assert_eq!(
            completed,
            Err(RemoteError::GovernedAfterCompletion(
                TrippedGovernor::Stopped {
                    cause: StopCause::Cancelled
                }
            )),
            "a completed, discarded response withdraws it"
        );
        let failed = service(
            ANSWERED,
            Some(StopCause::Cancelled),
            Some(Delivered::Failure {
                kind: FailureKind::Transport,
                message: "down".to_owned(),
            }),
            &slots,
        );
        assert!(
            matches!(failed, Err(RemoteError::Governed(_))),
            "a stop outranks a transport failure SILENT could swallow"
        );
        assert!(slots.fault().is_none(), "none of these is a fault");
    }

    #[test]
    fn deliveries_map_onto_the_remote_seam() {
        let slots = slots();
        assert_eq!(
            service(ANSWERED, None, Some(bindings()), &slots),
            Ok(SRJ.to_vec())
        );
        assert_eq!(
            service(
                ANSWERED,
                None,
                Some(Delivered::Failure {
                    kind: FailureKind::Transport,
                    message: "HTTP 503".to_owned()
                }),
                &slots
            ),
            Err(RemoteError::Transport("HTTP 503".to_owned()))
        );
        // A host `"denied"` delivery is the host's own policy, never a catalog capability
        // this engine withheld — see `RemoteError::HostDenied`'s docs. Only the native
        // catalog gate in `HttpRemoteQuerySource::resolve` produces `RemoteError::Denied`,
        // and that path never reaches `service_answer` at all.
        let denied = service(
            ANSWERED,
            None,
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                message: "not on the list".to_owned(),
            }),
            &slots,
        );
        assert_eq!(
            denied,
            Err(RemoteError::HostDenied {
                endpoint: ENDPOINT.to_owned(),
                message: "not on the list".to_owned(),
            })
        );
        assert!(slots.fault().is_none());
    }

    #[test]
    fn a_resolver_that_returns_without_a_delivery_is_a_fault_not_an_answer() {
        let slots = slots();
        let result = service(ANSWERED, None, None, &slots);
        assert!(
            matches!(result, Err(RemoteError::Governed(_))),
            "{result:?}"
        );
        let fault = slots.fault().expect("a fault is latched");
        assert!(
            fault.contains("resolver returned without a delivery"),
            "{fault}"
        );
        // The neighbour: a host that reports its own fault latches nothing more here.
        let faulted = slots_with_fault("the host's own words");
        assert!(matches!(
            service(SuspendStatus::Fault, None, None, &faulted),
            Err(RemoteError::Governed(_))
        ));
        assert_eq!(faulted.fault(), Some("the host's own words"));
    }

    fn slots_with_fault(message: &str) -> JobSlots {
        let slots = slots();
        slots.latch_fault(message);
        slots
    }

    fn load(
        status: SuspendStatus,
        delivered: Option<Delivered>,
        slots: &JobSlots,
    ) -> Result<LoadStep, LoadError> {
        load_answer("http://example.org/doc", status, None, delivered, slots)
    }

    #[test]
    fn load_deliveries_map_onto_the_load_seam() {
        let slots = slots();
        let graph = parse_document(REMOTE_NT.as_bytes(), "text/turtle", None).expect("parses");
        assert!(matches!(
            load(ANSWERED, Some(Delivered::Graph(graph)), &slots),
            Ok(LoadStep::Graph(_))
        ));
        assert!(matches!(
            load(
                ANSWERED,
                Some(Delivered::Redirect("/moved".to_owned())),
                &slots
            ),
            Ok(LoadStep::Redirect(location)) if location == "/moved"
        ));
        let failed = load(
            ANSWERED,
            Some(Delivered::Failure {
                kind: FailureKind::Transport,
                message: "HTTP 404".to_owned(),
            }),
            &slots,
        )
        .expect_err("a failure");
        assert_eq!(failed, LoadError::Transport("HTTP 404".to_owned()));
        assert_eq!(failed.code(), "native-sparql-load-failed");
        let undecodable = load(
            ANSWERED,
            Some(Delivered::Failure {
                kind: FailureKind::Decode,
                message: "not Turtle".to_owned(),
            }),
            &slots,
        )
        .expect_err("a decode failure");
        assert_eq!(undecodable.code(), "native-sparql-load-decode");
        let denied = load(
            ANSWERED,
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                message: "policy".to_owned(),
            }),
            &slots,
        )
        .expect_err("a denial");
        assert_eq!(denied.code(), "native-sparql-load-host-denied");
        assert_eq!(
            denied,
            LoadError::HostDenied("policy".to_owned()),
            "a LOAD denial the host delivers is the host's own decision"
        );
        assert_eq!(denied.to_string(), "the host denied the request: policy");
        assert!(slots.fault().is_none(), "failures are answers, not faults");
        let missing = load(ANSWERED, None, &slots).expect_err("no delivery");
        assert_eq!(missing.code(), "native-sparql-load-fault");
        assert!(slots.fault().is_some(), "a missing delivery is a fault");
    }

    /// With a catalog, a `LOAD` source the catalog does not grant `network` is the
    /// catalog's own denial, decided before any effect; a granted one is fetched with its
    /// profile's headers, user agent and timeout; without a catalog nothing is refused.
    #[test]
    fn the_catalog_decides_a_load_before_the_host_is_asked() {
        let catalog = catalog(&[
            (
                "http://example.org/doc",
                r#"{"capabilities":["network"],"headers":[["X-A","1"]],"userAgent":"w/1","timeoutMs":250}"#,
            ),
            (
                "http://example.org/query-only",
                r#"{"capabilities":["query"]}"#,
            ),
        ]);
        let resolver = JspiGraphResolver {
            watch: Arc::new(JspiStopWatch::new(Arc::new(slots()), None, u32::MAX)),
            catalog: Some(catalog.inner),
        };
        let allowed = resolver
            .authorize("http://example.org/doc")
            .expect("granted network");
        assert_eq!(allowed.headers, [("X-A".to_owned(), "1".to_owned())]);
        assert_eq!(allowed.user_agent.as_deref(), Some("w/1"));
        assert!((allowed.timeout_ms - 250.0).abs() < f64::EPSILON);
        assert!(allowed.accept.contains("text/turtle"));
        let withheld = resolver
            .authorize("http://example.org/query-only")
            .expect_err("no network");
        assert_eq!(withheld.code(), "native-sparql-load-denied");
        assert!(withheld.to_string().contains("network"), "{withheld}");
        assert!(matches!(
            resolver.authorize("http://example.org/unlisted"),
            Err(LoadError::Denied(_))
        ));
        assert!(
            resolver.watch.slots.take_effect().is_none(),
            "a denial issues no effect"
        );
        // Without a catalog, every source is fetched, with the default timeout.
        let open = JspiGraphResolver {
            watch: Arc::clone(&resolver.watch),
            catalog: None,
        };
        let effect = open
            .authorize("http://example.org/unlisted")
            .expect("no catalog, no gate");
        assert_eq!(effect.headers, Vec::<(String, String)>::new());
        assert_eq!(
            effect.timeout_ms.to_bits(),
            (DEFAULT_TIMEOUT.as_secs_f64() * 1000.0).to_bits()
        );
    }

    #[test]
    fn a_redirect_resolves_against_the_iri_it_redirected() {
        assert_eq!(
            redirect_target("http://example.org/a/doc", "/moved").as_deref(),
            Ok("http://example.org/moved")
        );
        assert_eq!(
            redirect_target("http://example.org/a/doc", "next").as_deref(),
            Ok("http://example.org/a/next")
        );
        assert_eq!(
            redirect_target("http://example.org/a/doc", "https://example.org/x").as_deref(),
            Ok("https://example.org/x")
        );
        assert!(
            redirect_target("http://example.org/a/doc", "http://exa mple.org/")
                .expect_err("not an IRI")
                .contains("not a resolvable IRI")
        );
    }

    // ── The ticket exchange ─────────────────────────────────────────────────────────

    fn service_effect() -> ServiceEffect {
        ServiceEffect {
            endpoint: ENDPOINT.to_owned(),
            query_text: "SELECT * WHERE { ?s ?p ?o }".to_owned(),
            user_agent: "test".to_owned(),
            timeout_ms: 1.0,
            content_type: "application/sparql-query".to_owned(),
            accept: "application/sparql-results+json".to_owned(),
            headers: vec![("Authorization".to_owned(), "Bearer secret".to_owned())],
            silent: true,
            max_intermediate_cells: Some(10),
            remaining_deadline_ms: None,
            cacheable: false,
        }
    }

    fn service_payload() -> EffectPayload {
        EffectPayload::Service {
            effect: Box::new(service_effect()),
            exchange: 1,
        }
    }

    fn load_payload(iri: &str) -> EffectPayload {
        EffectPayload::Load(Box::new(LoadEffect {
            iri: iri.to_owned(),
            accept: load_accept(),
            user_agent: None,
            headers: vec![("X-Key".to_owned(), "secret".to_owned())],
            timeout_ms: 30_000.0,
        }))
    }

    #[test]
    fn a_delivery_must_name_the_outstanding_effect() {
        let slots = slots();
        let seq = slots
            .issue(load_payload("http://example.org/doc"), None)
            .expect("issued");
        let effect = slots.take_effect().expect("posted");
        assert_eq!(effect.seq(), seq);
        assert_eq!(effect.kind(), EffectKind::Load);
        assert_eq!(effect.iri().as_deref(), Some("http://example.org/doc"));
        assert!(
            effect
                .accept()
                .expect("a LOAD accept")
                .contains("text/turtle")
        );
        assert_eq!(effect.headers(), ["X-Key", "secret"]);
        assert!(
            !format!("{effect:?}").contains("secret"),
            "a log never shows a header value"
        );
        assert!(slots.take_effect().is_none(), "an effect is taken once");

        // Bindings cannot answer a LOAD: a fault, and the effect stays outstanding.
        assert_eq!(slots.deliver(seq, bindings()), DeliveryStatus::Fault);
        assert!(slots.fault().expect("latched").contains("Load"));
    }

    #[test]
    fn stale_future_and_late_deliveries_are_told_apart() {
        let slots = slots();
        let first = slots.issue(service_payload(), None).expect("issued");
        assert_eq!(slots.deliver(first, bindings()), DeliveryStatus::Accepted);
        assert!(matches!(slots.resume(first), Some(Delivered::Bindings(_))));
        let second = slots.issue(service_payload(), None).expect("issued");
        // A promise settling for the first effect after it was answered: stale, no fault.
        assert_eq!(slots.deliver(first, bindings()), DeliveryStatus::Stale);
        assert!(slots.fault().is_none());
        // A sequence number never issued is a bridge bug.
        assert_eq!(slots.deliver(second + 5, bindings()), DeliveryStatus::Fault);
        let fault = slots.fault().expect("latched");
        assert!(
            fault.contains(&(second + 5).to_string()) && fault.contains(&second.to_string()),
            "the fault names both sequence numbers: {fault}"
        );
        slots.finished.store(true, Ordering::Relaxed);
        assert_eq!(slots.deliver(second, bindings()), DeliveryStatus::Finished);
    }

    /// A job that has issued every sequence number there is latches a fault rather than
    /// wrapping around to a number a stale delivery could name; the number before it is
    /// still issued.
    #[test]
    fn the_last_sequence_number_is_issued_and_the_next_one_latches_a_fault() {
        let slots = slots();
        lock(&slots.tickets).last_issued = u32::MAX - 1;
        let last = slots
            .issue(EffectPayload::Yield, None)
            .expect("the last number");
        assert_eq!(last, u32::MAX);
        assert!(slots.fault().is_none());
        drop(slots.resume(last));
        assert_eq!(slots.issue(EffectPayload::Yield, None), None);
        assert!(
            slots
                .fault()
                .expect("latched")
                .contains("every effect sequence number")
        );
    }

    #[test]
    fn a_service_effect_exposes_its_request_and_hides_its_secret_from_logs() {
        let effect = AsyncEffect {
            seq: 7,
            payload: service_payload(),
            abandon_after_ms: Some(1.0),
        };
        assert_eq!(effect.kind(), EffectKind::Service);
        assert_eq!(effect.endpoint().as_deref(), Some(ENDPOINT));
        assert_eq!(
            effect.content_type().as_deref(),
            Some("application/sparql-query")
        );
        assert_eq!(
            effect.accept().as_deref(),
            Some("application/sparql-results+json")
        );
        assert!(effect.silent());
        assert!(!effect.cacheable());
        assert_eq!(effect.exchange_id(), Some(1.0));
        assert_eq!(effect.abandon_after_ms(), Some(1.0));
        assert_eq!(effect.max_intermediate_cells(), Some(10));
        assert_eq!(effect.headers(), vec!["Authorization", "Bearer secret"]);
        assert!(effect.iri().is_none());
        let logged = format!("{effect:?}");
        assert!(!logged.contains("secret"), "{logged}");
    }

    /// A failure kind the protocol does not define fails the effect it answers — as the
    /// host's fault, naming the kind — and latches nothing on the job; `"fault"` by name
    /// is the same failure with the host's own message; a denial is an answer.
    #[test]
    fn an_unknown_failure_kind_fails_the_effect_and_denied_is_an_answer() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        let slots = &job.inner.watch.slots;
        let seq = slots
            .issue(load_payload("http://example.org/doc"), None)
            .expect("issued");
        assert_eq!(
            job.deliver_failure(seq, "nope", "x".to_owned()),
            DeliveryStatus::Accepted
        );
        assert!(slots.fault().is_none());
        match slots.resume(seq) {
            Some(Delivered::Failure {
                kind: FailureKind::Fault,
                message,
            }) => assert_eq!(
                message,
                "unknown failure kind \"nope\" for effect 1 (expected \"transport\", \"denied\" \
                 or \"fault\")"
            ),
            other => panic!("the effect fails with the host's fault, got {other:?}"),
        }
        job.finish();

        let named = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        let slots = &named.inner.watch.slots;
        let seq = slots
            .issue(load_payload("http://example.org/doc"), None)
            .expect("issued");
        assert_eq!(
            named.deliver_failure(seq, "fault", "resolveLoad threw: boom".to_owned()),
            DeliveryStatus::Accepted
        );
        assert!(slots.fault().is_none());
        match slots.resume(seq) {
            Some(Delivered::Failure {
                kind: FailureKind::Fault,
                message,
            }) => assert_eq!(message, "resolveLoad threw: boom"),
            other => panic!("the effect fails with the host's own message, got {other:?}"),
        }
        named.finish();

        let neighbour = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        let slots = &neighbour.inner.watch.slots;
        let seq = slots
            .issue(load_payload("http://example.org/doc"), None)
            .expect("issued");
        assert_eq!(
            neighbour.deliver_failure(seq, "denied", "policy".to_owned()),
            DeliveryStatus::Accepted
        );
        assert!(slots.fault().is_none());
        assert!(matches!(
            slots.resume(seq),
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                ..
            })
        ));
        neighbour.finish();
    }

    /// A document that arrives and does not parse — an unknown media type, or bytes that
    /// are not the syntax named — is the `LOAD`'s decode failure; the neighbour parses.
    #[test]
    fn an_unparseable_load_document_is_a_decode_failure_not_a_fault() {
        let engine = QueryEngine::new();
        let dataset = seed();
        for (media_type, bytes, parses) in [
            ("text/x-unknown", REMOTE_NT, false),
            ("text/turtle", "<http://example.org/s> <", false),
            ("text/turtle", REMOTE_NT, true),
        ] {
            let job = begin(
                &engine,
                &dataset,
                AsyncOperationKind::Update,
                "CLEAR ALL",
                options(),
            );
            let slots = &job.inner.watch.slots;
            let seq = slots
                .issue(load_payload("http://example.org/doc"), None)
                .expect("issued");
            assert_eq!(
                job.deliver_graph(seq, bytes.as_bytes(), media_type, None),
                DeliveryStatus::Accepted
            );
            assert!(slots.fault().is_none(), "{media_type}");
            match (slots.resume(seq), parses) {
                (Some(Delivered::Graph(graph)), true) => assert_eq!(graph.quads().count(), 1),
                (
                    Some(Delivered::Failure {
                        kind: FailureKind::Decode,
                        ..
                    }),
                    false,
                ) => {}
                (other, _) => panic!("{media_type}: unexpected {other:?}"),
            }
            job.finish();
        }
    }

    // ── Abandonment ─────────────────────────────────────────────────────────────────

    /// An effect abandoned at the job's deadline is the deadline's trip; one abandoned at
    /// its own, earlier timeout is a transport failure `SILENT` absorbs. The instant is
    /// whichever falls first.
    #[test]
    fn an_effect_expires_as_the_deadline_or_as_its_own_timeout() {
        let short = JspiStopWatch::new(Arc::new(slots()), Some(1_000), u32::MAX);
        let (after, why) = short.abandonment(30_000.0);
        assert_eq!(why, Abandon::Deadline);
        assert!(after <= 1_000.0);
        assert_eq!(after.fract(), 0.0, "a timer takes whole milliseconds");
        let long = JspiStopWatch::new(Arc::new(slots()), Some(60_000), u32::MAX);
        assert_eq!(long.abandonment(0.25), (1.0, Abandon::Timeout(0.25)));
        assert_eq!(
            long.abandonment(30_000.0),
            (30_000.0, Abandon::Timeout(30_000.0))
        );
        let unbounded = JspiStopWatch::new(Arc::new(slots()), None, u32::MAX);
        assert_eq!(
            unbounded.abandonment(250.0),
            (250.0, Abandon::Timeout(250.0))
        );

        let engine = QueryEngine::new();
        let dataset = seed();
        // At the deadline: the effect is abandoned and the trip reads as a deadline.
        let deadline = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        let slots = &deadline.inner.watch.slots;
        let seq = slots
            .issue(
                load_payload("http://example.org/doc"),
                Some((0.0, Abandon::Deadline)),
            )
            .expect("issued");
        assert_eq!(deadline.expire_effect(seq), SuspendStatus::Abandoned);
        assert_eq!(
            deadline.inner.watch.observe_now(),
            Some(StopCause::Deadline)
        );
        assert!(matches!(slots.resume(seq), Some(Delivered::Governed)));
        deadline.finish();
        // At its own timeout: a transport failure, and the job's signal stays quiet.
        let timeout = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        let slots = &timeout.inner.watch.slots;
        let seq = slots
            .issue(
                load_payload("http://example.org/doc"),
                Some((250.0, Abandon::Timeout(250.0))),
            )
            .expect("issued");
        assert_eq!(timeout.expire_effect(seq), SuspendStatus::Answered);
        assert_eq!(timeout.inner.watch.observe_now(), None);
        match slots.resume(seq) {
            Some(Delivered::Failure {
                kind: FailureKind::Transport,
                message,
            }) => assert_eq!(message, "no answer within 250 ms"),
            other => panic!("expected a transport failure, got {other:?}"),
        }
        // An effect already answered stays answered.
        let seq = slots
            .issue(
                load_payload("http://example.org/doc"),
                Some((250.0, Abandon::Timeout(250.0))),
            )
            .expect("issued");
        assert_eq!(
            timeout.deliver_failure(seq, "transport", "down".to_owned()),
            DeliveryStatus::Accepted
        );
        assert_eq!(timeout.expire_effect(seq), SuspendStatus::Answered);
        assert!(slots.fault().is_none());
        timeout.finish();
    }

    // ── Shared SERVICE exchanges ────────────────────────────────────────────────────

    fn key(handler: u32, silent: bool) -> ExchangeKey {
        let mut effect = service_effect();
        effect.silent = silent;
        ExchangeKey::new(handler, &effect)
    }

    /// A job joins an open exchange only for the same request, to the same handler,
    /// under a deadline no later than the exchange's; every other job opens its own.
    #[test]
    fn a_job_joins_only_an_exchange_asking_the_same_question_by_its_deadline() {
        let owner = Arc::new(slots());
        let id = next_exchange_id();
        open_exchange(id, key(1, false), Some(60_000.0), (Arc::clone(&owner), 1));
        assert_eq!(find_exchange(&key(1, false), Some(30_000.0)), Some(id));
        assert_eq!(find_exchange(&key(1, false), Some(60_000.0)), Some(id));
        assert_eq!(find_exchange(&key(1, false), Some(90_000.0)), None);
        assert_eq!(find_exchange(&key(1, false), None), None, "no deadline");
        assert_eq!(find_exchange(&key(1, true), Some(30_000.0)), None, "SILENT");
        assert_eq!(
            find_exchange(&key(2, false), Some(30_000.0)),
            None,
            "handler"
        );
        leave_exchange(id, owner.job);
        assert!(!AsyncJob::exchange_is_open(id as f64));

        let unbounded = next_exchange_id();
        open_exchange(unbounded, key(1, false), None, (Arc::clone(&owner), 2));
        assert_eq!(find_exchange(&key(1, false), None), Some(unbounded));
        assert_eq!(
            find_exchange(&key(1, false), Some(90_000.0)),
            Some(unbounded)
        );
        leave_exchange(unbounded, owner.job);
    }

    /// An exchange's answer reaches every job still waiting on it; a job that left takes
    /// nothing; the exchange stays open while any job waits and closes with the last.
    #[test]
    fn an_exchange_fans_its_answer_out_and_closes_with_its_last_waiter() {
        let first = Arc::new(JobSlots::new(101, slots().bounds));
        let second = Arc::new(JobSlots::new(102, slots().bounds));
        let first_seq = first.issue(service_payload(), None).expect("issued");
        let second_seq = second.issue(service_payload(), None).expect("issued");
        let id = next_exchange_id();
        open_exchange(id, key(1, false), None, (Arc::clone(&first), first_seq));
        add_waiter(id, (Arc::clone(&second), second_seq));
        assert_eq!(
            AsyncJob::deliver_exchange_bindings(id as f64, SRJ.to_vec()),
            DeliveryStatus::Accepted
        );
        assert!(matches!(
            first.resume(first_seq),
            Some(Delivered::Bindings(_))
        ));
        assert!(matches!(
            second.resume(second_seq),
            Some(Delivered::Bindings(_))
        ));
        assert!(!AsyncJob::exchange_is_open(id as f64));
        assert_eq!(
            AsyncJob::deliver_exchange_bindings(id as f64, SRJ.to_vec()),
            DeliveryStatus::Finished,
            "a closed exchange takes nothing"
        );

        let staying = Arc::new(JobSlots::new(103, slots().bounds));
        let leaving = Arc::new(JobSlots::new(104, slots().bounds));
        let staying_seq = staying.issue(service_payload(), None).expect("issued");
        let leaving_seq = leaving.issue(service_payload(), None).expect("issued");
        let id = next_exchange_id();
        open_exchange(id, key(1, false), None, (Arc::clone(&staying), staying_seq));
        add_waiter(id, (Arc::clone(&leaving), leaving_seq));
        leaving.wait_on(id);
        drop(leaving.resume(leaving_seq));
        assert!(AsyncJob::exchange_is_open(id as f64), "one job still waits");
        assert_eq!(
            AsyncJob::deliver_exchange_failure(id as f64, "transport", "down".to_owned()),
            DeliveryStatus::Accepted
        );
        assert!(matches!(
            staying.resume(staying_seq),
            Some(Delivered::Failure {
                kind: FailureKind::Transport,
                ..
            })
        ));
        assert!(leaving.fault().is_none(), "the job that left took nothing");

        let only = Arc::new(JobSlots::new(105, slots().bounds));
        let seq = only.issue(service_payload(), None).expect("issued");
        let id = next_exchange_id();
        open_exchange(id, key(1, false), None, (Arc::clone(&only), seq));
        only.wait_on(id);
        drop(only.resume(seq));
        assert!(
            !AsyncJob::exchange_is_open(id as f64),
            "the last waiter left"
        );
    }

    /// A host fault in the shared call — the handler threw, or answered with a kind the
    /// protocol does not define — fails the effect of every job waiting on it, as that
    /// invocation's failure, and latches no job's fault.
    #[test]
    fn a_fault_in_a_shared_call_fails_every_waiting_jobs_effect() {
        let first = Arc::new(JobSlots::new(111, slots().bounds));
        let second = Arc::new(JobSlots::new(112, slots().bounds));
        let first_seq = first.issue(service_payload(), None).expect("issued");
        let second_seq = second.issue(service_payload(), None).expect("issued");
        let id = next_exchange_id();
        open_exchange(id, key(1, false), None, (Arc::clone(&first), first_seq));
        add_waiter(id, (Arc::clone(&second), second_seq));
        assert_eq!(
            AsyncJob::deliver_exchange_failure(id as f64, "nope", "?".to_owned()),
            DeliveryStatus::Accepted
        );
        for (slots, seq) in [(&first, first_seq), (&second, second_seq)] {
            assert!(slots.fault().is_none());
            match slots.resume(seq) {
                Some(Delivered::Failure {
                    kind: FailureKind::Fault,
                    message,
                }) => assert_eq!(
                    message,
                    format!(
                        "unknown failure kind \"nope\" for exchange {id} (expected \
                         \"transport\", \"denied\" or \"fault\")"
                    )
                ),
                other => panic!("every waiter's effect fails with the fault, got {other:?}"),
            }
        }
        let third = Arc::new(JobSlots::new(113, slots().bounds));
        let seq = third.issue(service_payload(), None).expect("issued");
        let id = next_exchange_id();
        open_exchange(id, key(1, false), None, (Arc::clone(&third), seq));
        assert_eq!(
            AsyncJob::fault_exchange(id as f64, "resolveService threw: boom"),
            DeliveryStatus::Accepted
        );
        assert!(third.fault().is_none());
        match third.resume(seq) {
            Some(Delivered::Failure {
                kind: FailureKind::Fault,
                message,
            }) => assert_eq!(message, "resolveService threw: boom"),
            other => panic!("the waiter's effect fails with the handler's fault, got {other:?}"),
        }
    }

    /// Only a request that could not have been shared with another's host call can be
    /// cached by a shared cache: one carrying a credential — in its catalog profile or
    /// in a credential header — is not cacheable; the neighbour without one is.
    #[test]
    fn a_credentialed_request_is_never_cacheable() {
        let catalog = catalog(&[
            (
                "http://example.org/secret",
                r#"{"capabilities":["query","network","credentials"],"credential":{"header":"X-Key","value":"k"}}"#,
            ),
            (ENDPOINT, r#"{"capabilities":["query","network"]}"#),
        ]);
        let transport = JspiTransport {
            watch: Arc::new(JspiStopWatch::new(Arc::new(slots()), None, u32::MAX)),
            catalog: Some(catalog.inner),
            handler: 1,
            memo: Mutex::new(BTreeMap::new()),
        };
        let request = |endpoint: &'static str, headers: &'static [(String, String)]| HttpRequest {
            endpoint,
            query_text: "SELECT * WHERE { ?s ?p ?o }",
            user_agent: "test",
            timeout: Duration::from_secs(1),
            content_type: "application/sparql-query",
            accept: "application/sparql-results+json",
            headers,
            stop: None,
        };
        assert!(!transport.cacheable(&request("http://example.org/secret", &[])));
        let cookie: &'static [(String, String)] =
            Box::leak(Box::new([("Cookie".to_owned(), "a=b".to_owned())]));
        assert!(!transport.cacheable(&request(ENDPOINT, cookie)));
        let plain: &'static [(String, String)] =
            Box::leak(Box::new([("X-Tenant".to_owned(), "a".to_owned())]));
        assert!(transport.cacheable(&request(ENDPOINT, plain)));
        assert!(transport.cacheable(&request(ENDPOINT, &[])));
    }

    /// A request the job already had answered with rows is answered from its memo: the
    /// bytes, and no effect issued. (A request not in the memo suspends, which only a
    /// JSPI host can resume; the package's own tests observe that neighbour.)
    #[test]
    fn a_repeated_request_is_answered_from_the_jobs_memo() {
        let transport = JspiTransport {
            watch: Arc::new(JspiStopWatch::new(Arc::new(slots()), None, u32::MAX)),
            catalog: None,
            handler: 1,
            memo: Mutex::new(BTreeMap::new()),
        };
        let request = HttpRequest {
            endpoint: ENDPOINT,
            query_text: "SELECT * WHERE { ?s ?p ?o }",
            user_agent: "test",
            timeout: Duration::from_millis(1),
            content_type: "application/sparql-query",
            accept: "application/sparql-results+json",
            headers: &[],
            stop: None,
        };
        let mut effect = service_effect();
        effect.headers = Vec::new();
        effect.silent = false;
        effect.max_intermediate_cells = None;
        effect.cacheable = true;
        lock(&transport.memo).insert(ExchangeKey::new(1, &effect), Arc::from(SRJ));
        assert_eq!(transport.post(request), Ok(SRJ.to_vec()));
        assert!(
            transport.watch.slots.take_effect().is_none(),
            "a memo hit issues no effect"
        );
        assert_eq!(lock(&transport.watch.slots.tickets).last_issued, 0);
    }

    // ── The stop watch ──────────────────────────────────────────────────────────────

    fn watch(deadline_ms: Option<u64>) -> JspiStopWatch {
        JspiStopWatch::new(Arc::new(slots()), deadline_ms, u32::MAX)
    }

    #[test]
    fn the_watch_counts_polls_and_reads_the_clock_every_1024() {
        let expired = watch(Some(0));
        for poll in 1..CLOCK_EVERY_POLLS {
            assert_eq!(expired.poll(), None, "poll {poll} reads no clock");
        }
        assert_eq!(
            expired.poll(),
            Some(StopCause::Deadline),
            "the 1024th poll reads the clock"
        );
        assert_eq!(
            expired.slots.counters.polls.load(Ordering::Relaxed),
            CLOCK_EVERY_POLLS
        );
        // A resumption reads the clock at once.
        assert_eq!(watch(Some(0)).observe_now(), Some(StopCause::Deadline));
        assert_eq!(watch(Some(60_000)).observe_now(), None);
    }

    #[test]
    fn cancellation_faults_and_an_expired_effect_stop_the_watch_and_stay_stopped() {
        let cancelled = watch(None);
        assert_eq!(cancelled.poll(), None);
        cancelled.cancel.cancel();
        assert_eq!(cancelled.poll(), Some(StopCause::Cancelled));

        let faulted = watch(None);
        faulted.slots.latch_fault("host bug");
        assert_eq!(faulted.poll(), Some(StopCause::Cancelled));

        let timed_out = watch(None);
        timed_out
            .slots
            .deadline_tripped
            .store(true, Ordering::Relaxed);
        assert_eq!(timed_out.poll(), Some(StopCause::Deadline));
        timed_out.cancel.cancel();
        assert_eq!(
            timed_out.poll(),
            Some(StopCause::Deadline),
            "a fired signal stays fired with its first cause"
        );
    }

    #[test]
    fn deliver_governed_cancels_only_a_watch_that_has_not_fired() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        let seq = job
            .inner
            .watch
            .slots
            .issue(load_payload("http://example.org/doc"), None)
            .expect("issued");
        assert_eq!(job.deliver_governed(seq), DeliveryStatus::Accepted);
        assert_eq!(job.inner.watch.observe_now(), Some(StopCause::Cancelled));
        job.finish();

        let timed_out = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            options(),
        );
        timed_out
            .inner
            .watch
            .slots
            .deadline_tripped
            .store(true, Ordering::Relaxed);
        let seq = timed_out
            .inner
            .watch
            .slots
            .issue(load_payload("http://example.org/doc"), None)
            .expect("issued");
        assert_eq!(timed_out.deliver_governed(seq), DeliveryStatus::Accepted);
        assert_eq!(
            timed_out.inner.watch.observe_now(),
            Some(StopCause::Deadline),
            "the deadline latched first, so the trip is a deadline"
        );
        timed_out.finish();
    }

    #[test]
    fn the_stack_guard_stops_a_frame_in_the_guard_band() {
        let bounds = StackBounds {
            base: 0x10_0000,
            top: 0x18_0000,
        };
        let check = |sp: usize| bounds.check(sp);
        assert!(check(bounds.top - 64).is_ok());
        assert!(
            check(bounds.base + STACK_GUARD_BYTES).is_ok(),
            "the first byte above the guard band is usable"
        );
        let exhausted =
            check(bounds.base + STACK_GUARD_BYTES - 1).expect_err("inside the guard band");
        assert_eq!(
            exhausted,
            "asynchronous job stack region exhausted (524288 bytes); raise stackBytes"
        );
        // The band lies below the point the evaluator's checks refuse at: a frame the
        // margin has just run out under is still above it.
        assert!(check(bounds.base + purrdf_stack::MARGIN_BYTES - 1).is_ok());
        assert!(
            check(bounds.top + 16)
                .expect_err("outside the region")
                .contains("not running on its stack region")
        );
    }

    #[test]
    fn a_region_is_aligned_and_carries_its_canary() {
        let region = StackRegion::new(524_288 + 7);
        assert_eq!(region.bounds.base % 16, 0);
        assert_eq!(region.bounds.top % 16, 0);
        assert_eq!(region.bounds.top - region.bounds.base, 524_288);
        let offset = region.bounds.base - region.memory.as_ptr() as usize;
        assert_eq!(
            region.memory[offset..offset + 4],
            STACK_CANARY.to_le_bytes(),
            "the canary sits at the base"
        );
    }

    #[test]
    fn a_region_size_and_its_exhaustion_message_depend_only_on_the_request() {
        // Allocations land at different alignments; the region (and so the message a
        // host sees) must not follow them.
        let regions: Vec<StackRegion> = (0..8).map(|_| StackRegion::new(524_288 + 7)).collect();
        for region in &regions {
            assert_eq!(region.bounds.top - region.bounds.base, 524_288);
            assert_eq!(
                region.bounds.exhausted(),
                "asynchronous job stack region exhausted (524288 bytes); raise stackBytes"
            );
        }
        let larger = StackRegion::new(4 * 1024 * 1024);
        assert_eq!(
            larger.bounds.exhausted(),
            "asynchronous job stack region exhausted (4194304 bytes); raise stackBytes",
            "a different request is a different message"
        );
    }

    #[test]
    fn an_overrun_the_zone_absorbs_is_told_apart_from_one_that_escapes() {
        let base_offset = |region: &StackRegion| region.zone_offset + STACK_OVERRUN_ZONE_BYTES;

        // The valid neighbour: frames anywhere inside the region leave no trace below it.
        let mut inside = StackRegion::new(524_288);
        let offset = base_offset(&inside);
        inside.memory[offset + 4..offset + 4096].fill(0);
        assert_eq!(inside.overrun(), Overrun::None);

        // A frame that overwrote only the canary.
        let mut canary = StackRegion::new(524_288);
        let offset = base_offset(&canary);
        canary.memory[offset] ^= 0xFF;
        assert_eq!(canary.overrun(), Overrun::Absorbed);

        // Frames down to the floor, the canary rewritten by chance.
        let mut upper = StackRegion::new(524_288);
        let offset = base_offset(&upper);
        upper.memory[offset - (STACK_OVERRUN_ZONE_BYTES - STACK_OVERRUN_FLOOR_BYTES)..offset]
            .fill(0);
        assert_eq!(upper.overrun(), Overrun::Absorbed);

        // One byte in the floor: the frames may have left the allocation.
        let mut floor = StackRegion::new(524_288);
        let zone = floor.zone_offset;
        floor.memory[zone + STACK_OVERRUN_FLOOR_BYTES - 1] = 0;
        assert_eq!(floor.overrun(), Overrun::Escaped);
        let mut bottom = StackRegion::new(524_288);
        let zone = bottom.zone_offset;
        bottom.memory[zone] = 0;
        assert_eq!(bottom.overrun(), Overrun::Escaped);
    }

    #[test]
    fn a_poll_after_frames_overwrote_the_canary_stops_with_the_typed_exhaustion() {
        let region = StackRegion::new(524_288);
        let slots = Arc::new(JobSlots::new(1, region.bounds));
        slots.region_armed.store(true, Ordering::Relaxed);
        let watch = JspiStopWatch::new(Arc::clone(&slots), None, u32::MAX);
        // The valid neighbour: the canary is intact, so the guard goes on to the frame's
        // own address — which on the native build is never inside the region.
        assert!(
            watch
                .stack_check()
                .expect_err("a native frame is outside the region")
                .contains("not running on its stack region")
        );
        let mut region = region;
        let offset = region.zone_offset + STACK_OVERRUN_ZONE_BYTES;
        region.memory[offset..offset + 4].fill(0);
        assert_eq!(
            watch.stack_check().expect_err("the canary is gone"),
            "asynchronous job stack region exhausted (524288 bytes); raise stackBytes"
        );
        drop(watch);
        drop(region);
    }

    // ── Jobs, end to end on the native build (no effect is issued) ───────────────────

    const SELECT: &str = "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o } ORDER BY ?s";

    #[test]
    fn a_raw_job_answers_what_the_synchronous_twin_answers() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Raw,
            SELECT,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        assert!(job.is_finished());
        let expected = engine
            .query_raw(&dataset, SELECT, None, None, None, None)
            .expect("sync twin");
        assert_eq!(raw_text(&job), expected);
        let evidence = job.take_evidence();
        assert!(
            evidence.polls() > 0.0,
            "the evaluator polled the job's watch"
        );
        assert_eq!(evidence.yields(), 0.0);
        assert_eq!(evidence.service_effects(), 0.0);
        assert!(evidence.evaluate_ms() >= 0.0 && evidence.serialize_ms() >= 0.0);
        assert_eq!(
            job.inner
                .take_outcome("takeRawBytes", &[AsyncOperationKind::Raw])
                .expect_err("an outcome is taken once")
                .rendered(Lane::Sync),
            "takeRawBytes: the outcome was already taken"
        );
        assert_eq!(run_job(job.id()), RunStatus::AlreadyStarted);
        assert_eq!(job.finish(), 0);
        assert_eq!(run_job(job.id()), RunStatus::UnknownJob);
    }

    #[test]
    fn a_query_job_answers_the_typed_result() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            SELECT,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let mut result = job
            .take_query_result(Some("select".to_owned()))
            .expect("a SELECT result");
        let select = result.take_select().expect("select rows");
        assert_eq!(select.row_count(), 2);
        job.finish();
    }

    #[test]
    fn a_parse_error_is_the_jobs_error_in_the_synchronous_words() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "SELEC",
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("error"));
        let frozen = dataset.view().freeze().expect("freeze");
        let expected = NativeSparqlEngine::new()
            .query(&frozen, sparql_request("SELEC", None))
            .expect_err("the sync twin refuses it too")
            .to_string();
        assert_eq!(job.error_message().as_deref(), Some(expected.as_str()));
        job.finish();
    }

    #[test]
    fn service_without_a_handler_fails_like_the_offline_lane_unless_silent() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let query = format!("SELECT * WHERE {{ ?s ?p ?o SERVICE <{ENDPOINT}> {{ ?o ?q ?x }} }}");
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Raw,
            &query,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Error);
        let error = job.error_message().expect("an error");
        assert!(
            error.contains(&format!(
                "no remote query source configured for SERVICE <{ENDPOINT}>"
            )),
            "{error}"
        );
        assert!(job.take_evidence().silenced().is_empty());
        job.finish();

        // With nowhere to send the request the invocation fails, so SILENT is the join
        // identity on both lanes, and the job's evidence records the endpoint.
        let silent = query.replace("SERVICE <", "SERVICE SILENT <");
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Raw,
            &silent,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let expected = engine
            .query_raw(&dataset, &silent, None, None, None, None)
            .expect("sync twin");
        assert_eq!(
            raw_text(&job),
            expected,
            "SILENT is the join identity on both lanes"
        );
        let silenced = job.take_evidence().silenced();
        assert_eq!(silenced.len(), 1);
        assert_eq!(silenced[0].endpoint().as_deref(), Some(ENDPOINT));
        assert_eq!(silenced[0].kind(), "unconfigured");
        job.finish();
    }

    #[test]
    fn a_local_service_answers_in_process_and_an_unlisted_one_still_fails() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let remote = Dataset::parse(REMOTE_NT, "ntriples", None).expect("remote parses");
        let mut local = options();
        local
            .add_local_frozen(ENDPOINT.to_owned(), remote.view().freeze().expect("freeze"))
            .expect("declared");
        let query = format!(
            "SELECT ?s ?x WHERE {{ ?s <http://example.org/p> ?o \
             SERVICE <{ENDPOINT}> {{ ?o <http://example.org/q> ?x }} }}"
        );
        let job = begin(&engine, &dataset, AsyncOperationKind::Raw, &query, &local);
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let text = raw_text(&job);
        assert!(
            text.contains("http://example.org/x") && text.contains("http://example.org/s"),
            "the local endpoint's row joined: {text}"
        );
        assert!(!text.contains("http://example.org/s2"), "{text}");
        job.finish();

        let unlisted = query.replace(ENDPOINT, "http://example.org/elsewhere");
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Raw,
            &unlisted,
            &local,
        );
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert!(
            job.error_message()
                .expect("an error")
                .contains("no remote query source configured")
        );
        job.finish();
        // Under SILENT the unlisted endpoint is the join identity, as offline, recorded
        // as an endpoint no handler reaches.
        let silent = unlisted.replace("SERVICE <", "SERVICE SILENT <");
        let job = begin(&engine, &dataset, AsyncOperationKind::Raw, &silent, &local);
        assert_eq!(
            run_job(job.id()),
            RunStatus::Outcome,
            "silenced, as offline"
        );
        let text = raw_text(&job);
        assert!(!text.contains("http://example.org/x"), "{text}");
        let silenced = job.take_evidence().silenced();
        assert_eq!(silenced.len(), 1);
        assert_eq!(
            silenced[0].endpoint().as_deref(),
            Some("http://example.org/elsewhere")
        );
        assert_eq!(silenced[0].kind(), "unconfigured");
        job.finish();
        // The valid neighbour: SILENT naming the listed endpoint answers with its row.
        let silent_listed = query.replace("SERVICE <", "SERVICE SILENT <");
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Raw,
            &silent_listed,
            &local,
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let text = raw_text(&job);
        assert!(text.contains("http://example.org/x"), "{text}");
        job.finish();
    }

    #[test]
    fn a_cancelled_ungoverned_job_errors_as_cancelled() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            SELECT,
            options(),
        );
        assert_eq!(job.cancel(), DeliveryStatus::Accepted);
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("cancelled"));
        assert_eq!(job.cancel(), DeliveryStatus::Finished);
        job.finish();
    }

    #[test]
    fn an_explain_job_answers_what_the_synchronous_twin_answers() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Explain,
            SELECT,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let expected = engine
            .explain_query(&dataset, SELECT, None)
            .expect("sync twin");
        assert_eq!(raw_text(&job), expected);
        assert!(
            job.take_evidence().polls() > 0.0,
            "the measuring run polled the job's watch"
        );
        assert_eq!(
            job.inner
                .take_outcome("takeQueryResult", &[AsyncOperationKind::Query])
                .expect_err("a take of the wrong kind is refused")
                .rendered(Lane::Sync),
            "takeQueryResult is not available on a explain job"
        );
        job.finish();
    }

    #[test]
    fn an_explain_job_measures_a_service_join_the_offline_lane_cannot() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let remote = Dataset::parse(REMOTE_NT, "ntriples", None).expect("remote parses");
        let mut local = options();
        local
            .add_local_frozen(ENDPOINT.to_owned(), remote.view().freeze().expect("freeze"))
            .expect("declared");
        let query = format!(
            "SELECT ?s ?x WHERE {{ ?s <http://example.org/p> ?o \
             SERVICE <{ENDPOINT}> {{ ?o <http://example.org/q> ?x }} }}"
        );
        // The offline lane refuses it by name.
        let frozen = dataset.view().freeze().expect("freeze");
        let refused = NativeSparqlEngine::new()
            .explain_query(&frozen, &query, None)
            .expect_err("no SERVICE source offline")
            .to_string();
        assert!(
            refused.contains("no remote query source configured"),
            "{refused}"
        );
        // The job explains it, with the endpoint's row joined into the measured run.
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Explain,
            &query,
            &local,
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let text = raw_text(&job);
        // One seed row joins the endpoint's one row: the SERVICE node materialised the
        // endpoint's row and the projection the joined one.
        let node = |label: &str| {
            text.lines()
                .find(|line| line.split_whitespace().nth(1) == Some(label))
                .unwrap_or_else(|| panic!("the ledger has a {label} node: {text}"))
                .to_owned()
        };
        assert!(node("Service").contains(" rows=1 "), "{text}");
        assert!(node("Project").contains(" rows=1 "), "{text}");
        job.finish();
    }

    #[test]
    fn a_cancelled_explain_job_errors_as_cancelled() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Explain,
            SELECT,
            options(),
        );
        assert_eq!(job.cancel(), DeliveryStatus::Accepted);
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("cancelled"));
        job.finish();
    }

    #[test]
    fn an_explain_operation_refuses_a_ceiling_it_would_not_enforce() {
        let mut ceiling = options();
        ceiling.set_deadline_ms(Some(10));
        assert!(
            ceiling
                .validate(AsyncOperationKind::Explain)
                .expect_err("explain is metered, never bounded")
                .contains("deadlineMs is an execution governor")
        );
        assert!(options().validate(AsyncOperationKind::Explain).is_ok());
    }

    #[test]
    fn a_governed_job_reports_a_trip_as_an_outcome() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let mut governed = options();
        governed.set_fuel(Some(0));
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Governed,
            SELECT,
            &governed,
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let mut outcome = job.take_query_outcome().expect("an outcome");
        assert!(!outcome.is_complete());
        assert!(outcome.take_tripped().is_some());
        assert_eq!(
            job.inner
                .take_outcome("takeRawBytes", &[AsyncOperationKind::Raw])
                .expect_err("a take of the wrong kind is refused")
                .rendered(Lane::Sync),
            "takeRawBytes is not available on a governed job"
        );
        job.finish();
    }

    #[test]
    fn a_latched_fault_discards_the_outcome() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            "INSERT DATA { <http://example.org/n> <http://example.org/p> <http://example.org/o> }",
            options(),
        );
        assert_eq!(job.fault("host bug".to_owned()), DeliveryStatus::Accepted);
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("fault"));
        let before = dataset.current_generation();
        assert_eq!(
            job.inner
                .commit_into(&mut dataset)
                .expect_err("nothing to commit")
                .rendered(Lane::Sync),
            "host bug"
        );
        assert_eq!(dataset.current_generation(), before);
        assert_eq!(dataset.size(), 2);
        job.finish();
    }

    const INSERT: &str =
        "INSERT DATA { <http://example.org/n> <http://example.org/p> <http://example.org/o> }";

    #[test]
    fn load_silent_without_a_handler_loads_nothing_and_records_it() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        // Without SILENT the job fails, as the synchronous lane does.
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            "LOAD <http://example.org/doc>",
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert!(
            job.error_message()
                .expect("an error")
                .contains("native-sparql-load-no-resolver")
        );
        job.finish();
        // With SILENT it applies, loads nothing, and the evidence names the source.
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            &format!("LOAD SILENT <http://example.org/doc> ; {INSERT}"),
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let silenced = job.take_evidence().silenced();
        assert_eq!(silenced.len(), 1);
        assert_eq!(silenced[0].target(), "load");
        assert_eq!(silenced[0].iri().as_deref(), Some("http://example.org/doc"));
        assert_eq!(silenced[0].endpoint(), None);
        assert_eq!(silenced[0].kind(), "unconfigured");
        job.inner.commit_into(&mut dataset).expect("commits");
        assert_eq!(
            dataset.size(),
            3,
            "the INSERT beside it applied, nothing was loaded"
        );
        job.finish();
    }

    #[test]
    fn an_update_commits_into_the_dataset_it_read() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            INSERT,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        assert_eq!(dataset.size(), 2, "nothing lands before the commit");
        job.inner.commit_into(&mut dataset).expect("commits");
        assert_eq!(dataset.size(), 3);
        assert_eq!(dataset.current_generation(), 1);
        assert_eq!(
            job.inner
                .commit_into(&mut dataset)
                .expect_err("a second commit")
                .rendered(Lane::Sync),
            "nothing to commit: the update was not applied, or was already committed"
        );
        assert_eq!(dataset.size(), 3);
        job.finish();
    }

    #[test]
    fn an_update_refuses_to_commit_over_a_concurrent_mutation() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            INSERT,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        engine
            .update(
                &mut dataset,
                "INSERT DATA { <http://example.org/m> <http://example.org/p> <http://example.org/o> }",
                None,
            )
            .expect("a synchronous mutation meanwhile");
        let error = job
            .inner
            .commit_into(&mut dataset)
            .expect_err("stale")
            .rendered(Lane::Sync);
        assert_eq!(
            error,
            "dataset mutated while an asynchronous update was in flight (generation 0 → 1); \
             the update was not applied"
        );
        assert_eq!(dataset.size(), 3, "only the synchronous insert landed");
        job.finish();
    }

    #[test]
    fn an_update_refuses_to_commit_into_a_different_dataset() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let mut other = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            INSERT,
            options(),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let error = job
            .inner
            .commit_into(&mut other)
            .expect_err("wrong dataset")
            .rendered(Lane::Sync);
        assert!(
            error.starts_with("commit targets a different dataset"),
            "{error}"
        );
        assert_eq!(other.size(), 2);
        job.finish();
    }

    #[test]
    fn a_tripped_governed_update_offers_nothing_to_commit() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        let mut governed = options();
        governed.set_fuel(Some(0));
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::UpdateGoverned,
            INSERT,
            &governed,
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let outcome = job.take_update_outcome().expect("an outcome");
        assert!(!outcome.is_applied());
        assert!(
            job.inner
                .commit_into(&mut dataset)
                .expect_err("not applied")
                .rendered(Lane::Sync)
                .starts_with("nothing to commit")
        );
        assert_eq!(dataset.size(), 2);
        job.finish();

        let applied = begin(
            &engine,
            &dataset,
            AsyncOperationKind::UpdateGoverned,
            INSERT,
            options(),
        );
        assert_eq!(run_job(applied.id()), RunStatus::Outcome);
        assert!(applied.take_update_outcome().expect("outcome").is_applied());
        applied.inner.commit_into(&mut dataset).expect("commits");
        assert_eq!(dataset.size(), 3);
        applied.finish();
    }

    // ── Interleaving: the engine is re-entrant mid-evaluation ─────────────────────────

    thread_local! {
        static SHARED: RefCell<Option<(Rc<NativeSparqlEngine>, Arc<RdfDataset>)>> =
            const { RefCell::new(None) };
    }

    /// Run a query and an update on the shared engine — what another job or a
    /// synchronous call does while a suspended job's frames are on the stack.
    fn reenter() {
        let (engine, frozen) = SHARED
            .with(|shared| shared.borrow().clone())
            .expect("the shared engine is installed");
        let inner = engine
            .query(&frozen, sparql_request(SELECT, None))
            .expect("the re-entrant query answers");
        assert!(matches!(inner, SparqlResult::Solutions { .. }));
        let other = engine
            .query(&frozen, sparql_request("ASK { ?s ?p ?o }", None))
            .expect("a different text through the plan cache");
        assert!(matches!(other, SparqlResult::Boolean(true)));
        let mut target = Arc::clone(&frozen);
        engine
            .update(&mut target, sparql_request(INSERT, None))
            .expect("the re-entrant update applies");
    }

    #[derive(Debug, Default)]
    struct ReentrantSignal {
        polls: AtomicU64,
    }

    impl StopSignal for ReentrantSignal {
        fn poll(&self) -> Option<StopCause> {
            if self.polls.fetch_add(1, Ordering::Relaxed).is_multiple_of(3) {
                reenter();
            }
            None
        }
    }

    /// The executable half of the interleaving audit: a query whose resolver and whose
    /// stop signal both run further queries and an update on the SAME engine while the
    /// outer evaluation is on the stack. A plan-cache borrow, an order-cache lock or a
    /// memo borrow held across either seam would panic (or deadlock) here.
    #[test]
    fn engine_is_reentrant_from_a_resolver_and_from_a_poll() {
        let engine = Rc::new(NativeSparqlEngine::new());
        let frozen = seed().view().freeze().expect("freeze");
        SHARED
            .with(|shared| *shared.borrow_mut() = Some((Rc::clone(&engine), Arc::clone(&frozen))));
        let source = HttpRemoteQuerySource::new(|_request: HttpRequest<'_>| {
            reenter();
            Ok(SRJ.to_vec())
        });
        let signal = Arc::new(ReentrantSignal::default());
        let governors =
            QueryGovernors::METERED.with_stop_signal(Arc::clone(&signal) as Arc<dyn StopSignal>);
        let query = format!(
            "SELECT ?s ?x WHERE {{ ?s <http://example.org/p> ?o SERVICE <{ENDPOINT}> {{ ?o ?q ?x }} }}"
        );
        for _ in 0..2 {
            let outcome = engine
                .query_governed(
                    &frozen,
                    sparql_request(&query, None),
                    QueryOptions::new().with_remote(Some(&source)),
                    &governors,
                )
                .expect("the outer query evaluates");
            let GovernedOutcome::Complete {
                result: SparqlResult::Solutions { rows, .. },
                ..
            } = outcome
            else {
                panic!("the outer query completes with solutions");
            };
            assert_eq!(rows.len(), 2, "both local rows joined the remote binding");
        }
        assert!(signal.polls.load(Ordering::Relaxed) > 0);
        SHARED.with(|shared| *shared.borrow_mut() = None);
    }

    const GRAPH_CONSTRUCT: &str = "CONSTRUCT { GRAPH <http://example.org/out> { ?s ?p ?o } } \
                                   WHERE { ?s ?p ?o }";

    fn negotiated(accept: Option<&str>) -> Opts {
        let mut options = options();
        options.set_accept(accept.map(str::to_owned));
        options
    }

    #[test]
    fn a_negotiated_graph_carrying_named_graphs_defaults_to_trig_and_a_plain_one_to_turtle() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            GRAPH_CONSTRUCT,
            negotiated(None),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let mut outcome = job.take_negotiated_outcome().expect("an outcome");
        assert!(outcome.is_complete());
        assert_eq!(outcome.format().as_deref(), Some("trig"));
        assert_eq!(outcome.media_type().as_deref(), Some("application/trig"));
        let body = String::from_utf8(outcome.take_body().expect("a body")).expect("UTF-8");
        assert!(body.contains("<http://example.org/out>"), "{body}");
        assert!(outcome.take_evidence().is_some());
        job.finish();

        // The neighbour without a named graph in its template is plain Turtle.
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            "CONSTRUCT WHERE { ?s ?p ?o }",
            negotiated(None),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let outcome = job.take_negotiated_outcome().expect("an outcome");
        assert_eq!(outcome.format().as_deref(), Some("turtle"));
        job.finish();
    }

    #[test]
    fn an_explicit_triples_only_accept_refuses_named_graphs_but_serves_a_plain_graph() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            GRAPH_CONSTRUCT,
            negotiated(Some("text/turtle")),
        );
        assert_eq!(run_job(job.id()), RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("not-acceptable"));
        let error = job.error_message().expect("an error");
        assert!(error.contains("application/trig, application/n-quads, application/ld+json"));
        job.finish();

        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            "CONSTRUCT WHERE { ?s ?p ?o }",
            negotiated(Some("text/turtle")),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let outcome = job.take_negotiated_outcome().expect("an outcome");
        assert_eq!(outcome.format().as_deref(), Some("turtle"));
        job.finish();

        // A client that prefers Turtle but accepts N-Quads is sent N-Quads for the dataset.
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            GRAPH_CONSTRUCT,
            negotiated(Some("text/turtle, application/n-quads;q=0.5")),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let outcome = job.take_negotiated_outcome().expect("an outcome");
        assert_eq!(outcome.format().as_deref(), Some("nquads"));
        job.finish();
    }

    #[test]
    fn a_negotiated_solutions_result_honours_accept_and_a_trip_is_an_outcome_without_a_body() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            "SELECT ?s WHERE { ?s ?p ?o }",
            negotiated(Some("text/csv")),
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let mut outcome = job.take_negotiated_outcome().expect("an outcome");
        assert_eq!(outcome.format().as_deref(), Some("csv"));
        let body = String::from_utf8(outcome.take_body().expect("a body")).expect("UTF-8");
        assert!(body.starts_with("s\r\n"), "{body:?}");
        job.finish();

        let mut capped = negotiated(None);
        capped.set_max_answers(Some(1));
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            "SELECT ?s WHERE { ?s ?p ?o }",
            &capped,
        );
        assert_eq!(run_job(job.id()), RunStatus::Outcome);
        let mut outcome = job.take_negotiated_outcome().expect("an outcome");
        assert!(!outcome.is_complete());
        assert!(outcome.take_body().is_none());
        assert!(outcome.format().is_none());
        assert!(outcome.take_tripped().is_some());
        assert!(outcome.take_partial().is_some());
        job.finish();
    }

    #[test]
    fn accept_is_refused_on_every_operation_but_the_negotiated_one() {
        let error = negotiated(Some("text/csv"))
            .validate(AsyncOperationKind::Governed)
            .expect_err("a governed query would ignore accept");
        assert!(error.contains("unknown query option \"accept\""), "{error}");
        assert!(
            negotiated(Some("text/csv"))
                .validate(AsyncOperationKind::Negotiated)
                .is_ok()
        );
        assert!(
            options()
                .text("format", "json")
                .validate(AsyncOperationKind::Negotiated)
                .is_err()
        );
        assert!(
            options()
                .text("format", "json")
                .validate(AsyncOperationKind::Raw)
                .is_ok()
        );
    }

    /// Beginning an asynchronous update while another update of the same dataset is in
    /// flight is refused with its own code; once the first is finished, the next begins,
    /// and a query never claims the dataset at all.
    #[test]
    fn a_second_update_in_flight_is_refused_and_a_sequential_one_begins() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        let first = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            INSERT,
            options(),
        );
        let validated = options()
            .validate(AsyncOperationKind::UpdateGoverned)
            .expect("valid");
        let refused = begin_job(
            engine.engine(),
            &dataset,
            AsyncOperationKind::UpdateGoverned,
            INSERT.to_owned(),
            &validated,
            None,
        )
        .expect_err("an update is in flight");
        assert_eq!(refused.code(), "native-sparql-update-in-flight");
        assert_eq!(
            FailureCode::from_diagnostic_code(refused.code()),
            FailureCode::UpdateInFlight
        );
        // A query reads a snapshot and claims nothing.
        let query = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            SELECT,
            options(),
        );
        assert_eq!(run_job(query.id()), RunStatus::Outcome);
        query.finish();
        // Another dataset is not claimed by the first update.
        let other = seed();
        let elsewhere = begin(
            &engine,
            &other,
            AsyncOperationKind::Update,
            INSERT,
            options(),
        );
        elsewhere.finish();

        assert_eq!(run_job(first.id()), RunStatus::Outcome);
        first.inner.commit_into(&mut dataset).expect("commits");
        assert_eq!(first.finish(), 0);
        // The neighbour: the next update, once the first is finished, begins and applies.
        let second = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            "INSERT DATA { <http://example.org/m> <http://example.org/p> <http://example.org/o> }",
            options(),
        );
        assert_eq!(run_job(second.id()), RunStatus::Outcome);
        second.inner.commit_into(&mut dataset).expect("commits");
        second.finish();
        assert_eq!(dataset.size(), 4);
    }

    // ── SHACL jobs ──────────────────────────────────────────────────────────────────

    const SHACL_PREFIXES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n";

    /// Core shapes: every person's age is an integer.
    fn core_shapes() -> String {
        format!(
            "{SHACL_PREFIXES}ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;\n\
             sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n"
        )
    }

    /// A SPARQL target that asks `ENDPOINT` which people are banned, and a constraint
    /// every one of them violates.
    fn service_target_shapes() -> String {
        format!(
            "{SHACL_PREFIXES}ex:BannedShape a sh:NodeShape ;\n\
             sh:target [ a sh:SPARQLTarget ; sh:select \"\"\"SELECT ?this WHERE {{ \
             ?this a <http://example.org/Person> . SERVICE <{ENDPOINT}> \
             {{ ?this <http://example.org/status> \"banned\" }} }}\"\"\" ] ;\n\
             sh:property [ sh:path ex:clearance ; sh:minCount 1 ] .\n"
        )
    }

    const PEOPLE_NT: &str = concat!(
        "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n",
        "<http://example.org/alice> <http://example.org/age> \"old\" .\n",
        "<http://example.org/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n",
        "<http://example.org/bob> <http://example.org/age> \"40\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    );

    fn shacl_arguments(shapes: Option<String>, data: &str) -> ShaclArguments {
        ShaclArguments {
            shapes,
            data: data.to_owned(),
            ..ShaclArguments::default()
        }
    }

    /// Begin a SHACL job and run it to completion on the native build.
    fn run_shacl(
        operation: ShaclAsyncOperation,
        arguments: ShaclArguments,
        options: impl std::borrow::Borrow<Opts>,
    ) -> (AsyncJob, RunStatus) {
        let options = options
            .borrow()
            .clone()
            .validate(AsyncOperationKind::Shacl)
            .expect("options are valid");
        let job = begin_shacl_job(operation, arguments, &options).expect("the job begins");
        let status = run_job(job.id());
        (job, status)
    }

    #[test]
    fn a_shacl_job_answers_what_each_synchronous_entry_answers() {
        let shapes = core_shapes();
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ValidateToSarif,
            shacl_arguments(Some(shapes.clone()), PEOPLE_NT),
            options(),
        );
        assert_eq!(status, RunStatus::Outcome);
        let expected = validate_to_sarif_impl(&shapes, None, PEOPLE_NT).expect("sync entry");
        assert!(
            expected.contains("DatatypeConstraintComponent"),
            "the fixture violates"
        );
        assert_eq!(raw_text(&job), expected);
        assert!(
            job.take_evidence().polls() > 0.0,
            "the validation polled the job's watch between focus nodes"
        );
        job.finish();

        let added = "<http://example.org/bob> <http://example.org/age> \"x\" .\n";
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ValidateChangesToSarif,
            ShaclArguments {
                added: Some(added.to_owned()),
                ..shacl_arguments(Some(shapes.clone()), PEOPLE_NT)
            },
            options(),
        );
        assert_eq!(status, RunStatus::Outcome);
        let (sarif, scope) =
            validate_changes_to_sarif_impl(&shapes, None, PEOPLE_NT, Some(added), None)
                .expect("sync entry");
        let changed = job
            .take_shacl_change_validation()
            .expect("a change validation");
        assert_eq!(changed.sarif(), sarif);
        assert_eq!(changed.bounded(), scope.is_bounded());
        assert_eq!(changed.focus_nodes(), Some(1));
        job.finish();

        let rules = format!(
            "{SHACL_PREFIXES}ex:R a sh:NodeShape ; sh:targetClass ex:Person ;\n\
             sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:adult ; \
             sh:object ex:yes ] .\n"
        );
        let (job, status) = run_shacl(
            ShaclAsyncOperation::Entail,
            shacl_arguments(Some(rules.clone()), PEOPLE_NT),
            options(),
        );
        assert_eq!(status, RunStatus::Outcome);
        let entailed = entail_to_ntriples_impl(&rules, None, PEOPLE_NT).expect("sync entry");
        assert!(entailed.contains("<http://example.org/adult>"));
        assert_eq!(raw_text(&job), entailed);
        job.finish();

        let product = crate::shacl::pack_product_impl(&shapes, None).expect("packs");
        for operation in [
            ShaclAsyncOperation::ProductValidateToSarif,
            ShaclAsyncOperation::ProductValidateToSarifRebuild,
        ] {
            let (job, status) = run_shacl(
                operation,
                ShaclArguments {
                    product: Some(product.clone()),
                    ..shacl_arguments(None, PEOPLE_NT)
                },
                options(),
            );
            assert_eq!(status, RunStatus::Outcome, "{operation:?}");
            assert_eq!(raw_text(&job), expected, "{operation:?}");
            job.finish();
        }
    }

    #[test]
    fn a_shacl_job_reaches_its_local_service_where_the_synchronous_entry_has_no_source() {
        let shapes = service_target_shapes();
        let refused = validate_to_sarif_impl(&shapes, None, PEOPLE_NT)
            .expect_err("no SERVICE source offline");
        assert!(
            refused.contains("no remote query source configured"),
            "{refused}"
        );
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ValidateToSarif,
            shacl_arguments(Some(shapes.clone()), PEOPLE_NT),
            options(),
        );
        assert_eq!(
            status,
            RunStatus::Error,
            "a job with no source refuses it the same way"
        );
        assert!(
            job.error_message()
                .expect("an error")
                .contains("no remote query source configured")
        );
        job.finish();

        // Two registries: the report's focus node is the one each bans.
        for (banned, other) in [("alice", "bob"), ("bob", "alice")] {
            let registry = Dataset::parse(
                &format!(
                    "<http://example.org/{banned}> <http://example.org/status> \"banned\" .\n"
                ),
                "ntriples",
                None,
            )
            .expect("registry parses");
            let mut local = options();
            local
                .add_local_frozen(
                    ENDPOINT.to_owned(),
                    registry.view().freeze().expect("freeze"),
                )
                .expect("declared");
            let (job, status) = run_shacl(
                ShaclAsyncOperation::ValidateToSarif,
                shacl_arguments(Some(shapes.clone()), PEOPLE_NT),
                &local,
            );
            assert_eq!(status, RunStatus::Outcome);
            let sarif = raw_text(&job);
            assert!(
                sarif.contains(&format!("http://example.org/{banned}")),
                "{sarif}"
            );
            assert!(
                !sarif.contains(&format!("http://example.org/{other}")),
                "{sarif}"
            );
            job.finish();
        }
    }

    #[test]
    fn a_cancelled_shacl_job_errors_as_cancelled_and_an_uncancelled_one_answers() {
        let (job, status) = {
            let job = begin_shacl_job(
                ShaclAsyncOperation::ValidateToSarif,
                shacl_arguments(Some(core_shapes()), PEOPLE_NT),
                &options()
                    .validate(AsyncOperationKind::Shacl)
                    .expect("valid"),
            )
            .expect("begins");
            assert_eq!(job.cancel(), DeliveryStatus::Accepted);
            let status = run_job(job.id());
            (job, status)
        };
        assert_eq!(status, RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("cancelled"));
        job.finish();
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ValidateToSarif,
            shacl_arguments(Some(core_shapes()), PEOPLE_NT),
            options(),
        );
        assert_eq!(status, RunStatus::Outcome);
        job.finish();
    }

    #[test]
    fn a_refused_product_job_keeps_the_refusal_class_and_a_valid_product_answers() {
        let product = crate::shacl::pack_product_impl(&core_shapes(), None).expect("packs");
        let mut corrupted = product.clone();
        let middle = corrupted.len() / 2;
        corrupted[middle] ^= 0xff;
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ProductValidateToSarif,
            ShaclArguments {
                product: Some(corrupted),
                ..shacl_arguments(None, PEOPLE_NT)
            },
            options(),
        );
        assert_eq!(status, RunStatus::Error);
        assert_eq!(job.error_kind().as_deref(), Some("error"));
        let refusal = job.take_shacl_refusal().expect("the refusal is kept");
        assert_eq!(refusal.dimension().as_deref(), Some("section-digest"));
        assert!(job.take_shacl_refusal().is_none(), "taken once");
        job.finish();

        let (job, status) = run_shacl(
            ShaclAsyncOperation::ProductValidateToSarifExpecting,
            ShaclArguments {
                product: Some(product.clone()),
                expect_identity: Some("not-hex".to_owned()),
                ..shacl_arguments(None, PEOPLE_NT)
            },
            options(),
        );
        assert_eq!(status, RunStatus::Error);
        let refusal = job.take_shacl_refusal().expect("the refusal is kept");
        assert_eq!(refusal.dimension(), None, "no product was inspected");
        job.finish();

        let (job, status) = run_shacl(
            ShaclAsyncOperation::ProductValidateToSarif,
            ShaclArguments {
                product: Some(product),
                ..shacl_arguments(None, PEOPLE_NT)
            },
            options(),
        );
        assert_eq!(status, RunStatus::Outcome);
        assert!(job.take_shacl_refusal().is_none());
        job.finish();
    }

    #[test]
    fn a_shacl_operation_refuses_what_it_would_ignore_and_takes_what_its_twin_takes() {
        assert!(
            options()
                .ceiling("fuel", 10)
                .validate(AsyncOperationKind::Shacl)
                .expect_err("no SHACL entry takes a ceiling")
                .contains("fuel is an execution governor")
        );
        assert!(
            options()
                .text("base", "http://example.org/")
                .validate(AsyncOperationKind::Shacl)
                .expect_err("a SPARQL base would be ignored")
                .contains("unknown query option \"base\"")
        );
        assert!(options().validate(AsyncOperationKind::Shacl).is_ok());

        let shapes = || Some(core_shapes());
        let product = || Some(vec![0_u8; 4]);
        for (operation, arguments, refusal) in [
            (
                ShaclAsyncOperation::ValidateToSarif,
                shacl_arguments(None, PEOPLE_NT),
                "needs a shapes graph",
            ),
            (
                ShaclAsyncOperation::ValidateToSarif,
                ShaclArguments {
                    product: product(),
                    ..shacl_arguments(shapes(), PEOPLE_NT)
                },
                "takes no product",
            ),
            (
                ShaclAsyncOperation::Entail,
                ShaclArguments {
                    added: Some(String::new()),
                    ..shacl_arguments(shapes(), PEOPLE_NT)
                },
                "takes no change document",
            ),
            (
                ShaclAsyncOperation::ProductValidateToSarif,
                shacl_arguments(shapes(), PEOPLE_NT),
                "takes no shapes graph",
            ),
            (
                ShaclAsyncOperation::ProductValidateToSarif,
                ShaclArguments {
                    expect_identity: Some("00".to_owned()),
                    ..shacl_arguments(None, PEOPLE_NT)
                },
                "takes no expected identity",
            ),
            (
                ShaclAsyncOperation::ProductValidateToSarifExpecting,
                ShaclArguments {
                    product: product(),
                    ..shacl_arguments(None, PEOPLE_NT)
                },
                "needs an expected identity",
            ),
            (
                ShaclAsyncOperation::ProductValidateToSarifRebuild,
                shacl_arguments(None, PEOPLE_NT),
                "needs a product",
            ),
        ] {
            let error = ShaclRequest::build(operation, arguments).expect_err("refused");
            assert!(error.contains(refusal), "{operation:?}: {error}");
            assert!(error.starts_with(operation.name()), "{error}");
        }
        // The valid neighbour of each shape of refusal.
        for (operation, arguments) in [
            (
                ShaclAsyncOperation::ValidateToSarif,
                shacl_arguments(shapes(), PEOPLE_NT),
            ),
            (
                ShaclAsyncOperation::ValidateChangesToSarif,
                ShaclArguments {
                    added: Some(String::new()),
                    removed: Some(String::new()),
                    ..shacl_arguments(shapes(), PEOPLE_NT)
                },
            ),
            (
                ShaclAsyncOperation::ProductValidateToSarif,
                ShaclArguments {
                    product: product(),
                    ..shacl_arguments(None, PEOPLE_NT)
                },
            ),
            (
                ShaclAsyncOperation::ProductValidateToSarifRebuildExpecting,
                ShaclArguments {
                    product: product(),
                    expect_identity: Some("00".to_owned()),
                    ..shacl_arguments(None, PEOPLE_NT)
                },
            ),
        ] {
            assert!(
                ShaclRequest::build(operation, arguments).is_ok(),
                "{operation:?}"
            );
        }
    }
}
