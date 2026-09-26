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
//! the job runs the very same synchronous evaluator — no async Rust anywhere — on a
//! private shadow-stack region. The evaluator already performs every effect through three
//! seams it has always had:
//!
//! | Seam | Effect ([`AsyncEffectKind`]) | What the host does |
//! |---|---|---|
//! | `ServiceResolver::resolve` (a `SERVICE` clause) | `Service` = 1 | answers the forwarded `SELECT` with SPARQL Results JSON, or a typed failure |
//! | `GraphResolver::resolve` (a `LOAD`) | `Load` = 2 | answers the IRI with a document and its media type, or a typed failure |
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
//! ([`AsyncJob::deliver_bindings`] and siblings) and resumes the continuation. The job's
//! entry point is the raw export `purrdf_jspi_run(job)`, driven by
//! `WebAssembly.promising`. On a host without JSPI the synchronous API is untouched and
//! the package root refuses the asynchronous methods before touching wasm.
//!
//! # Stack regions and the `suspend` epilogue invariant
//!
//! A suspended job's frames stay live in linear memory, so two jobs cannot share the one
//! shadow stack. Every job owns a heap-allocated region ([`AsyncJob::stack_top`] /
//! [`AsyncJob::stack_base`], 16-byte aligned, a canary word [`AsyncJob::stack_canary`] at
//! the base). The host sets the stack pointer to the job's top before the promising call,
//! switches back to its own saved pointer at every suspension, and restores its idle value
//! after completion.
//!
//! The host's restore is not enough on its own: between one job's promise settling and
//! its resumption reaction running, another job's continuation may run and move the
//! global. So the resumed job must put its *own* stack pointer back before anything can
//! allocate a frame. That is what [`suspend`] guarantees: it takes the address of a local
//! (so it has a shadow-stack frame), is `#[inline(never)]`, and makes no call after the
//! import returns — only a read of its own frame — so its epilogue, which writes the
//! frame's stack pointer back to `__stack_pointer`, is the first stack-relevant thing that
//! happens on resumption. `wasm-opt` inlines it anyway; the inlined code keeps its own
//! frame and the same property, which is why the build gate checks it structurally (the
//! first stack-pointer instruction after every call of the import is its restore), never
//! by counting call sites.
//!
//! Inside the region two guards read one measurement. The run installs the region's base
//! as the stack floor the parser's and evaluator's guards measure against, with no walk
//! scope open (`purrdf_stack::replace_context`, which swaps the floor and the evaluator's
//! walk-scope state together), puts the context's own back before every suspension,
//! reinstalls the job's on every resumption (recording the resuming context's as the one
//! to put back next), and puts the context's own back when the run returns — so every
//! stack measurement on the job is against its own region, every one made while it is
//! suspended is against the stack actually running, and a walk scope the job suspends
//! inside (a property path's traversal polls, and so yields) stays the job's: nothing
//! that runs while it waits latches a refusal in it, and the jobs close their scopes in
//! whatever order they are resumed. The [`JspiStopWatch`] guards polling frames:
//! every poll measures the stack left above the base and, inside a guard band of
//! 128 KiB, latches a fault ("asynchronous job stack region exhausted …; raise
//! stackBytes") and stops the job cleanly — before a deeper frame could run off the
//! region into the heap. It also records the low-water mark, reported as
//! [`AsyncEvidence::stack_high_water_bytes`]. The evaluator guards the frames that never
//! poll — an operator chain, a nested `EXISTS` walk — by checking the same measurement at
//! every recursive step and refusing with its own typed error
//! (`native-sparql-evaluation-stack-exhausted`) while 64 KiB are still left.
//!
//! Work that neither polls nor is the evaluator's — parsing, serializing — can still
//! recurse past the base between two polls. Beneath every base lies an overrun zone
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
//! an ordinary answer — a malformed delivery, an unknown failure kind, a sequence number
//! that was never issued, stack exhaustion, a host that returned without delivering — is
//! *latched* as the job's fault ([`AsyncJob::fault`]). A latched fault makes the job's
//! stop signal fire, the evaluator winds down through its ordinary governor path, and the
//! run stores the fault as the job's error — discarding whatever outcome it reached, and
//! never committing an update. The host's own trap handling (poisoning the instance) is
//! the only answer to a genuine trap. Nothing on this side repairs one: a trap unwinds
//! past the run's restore of the caller's stack context and leaves whatever `RefCell`
//! its frames borrowed still borrowed, so the host bars every entry point of the
//! instance — synchronous ones and objects created before the trap included — for the
//! rest of the JavaScript realm's life.
//!
//! Statuses returned by `purrdf_jspi_suspend`: 0 the effect was answered, 1 the host
//! abandoned it on the job's stop signal, 2 a fault was latched. Statuses of the delivery
//! methods (and of `fault`, `tripDeadline` and `cancel`): 0 accepted; 1 stale — the
//! effect named was already answered or abandoned, typically a host promise settling
//! after the job's signal won the race, and not a fault; 2 malformed, with the job's
//! fault latched; 3 the job has already finished and nothing changed. Statuses of
//! `purrdf_jspi_run`: 0 an outcome is stored, 1 an error is stored, 2 no such job, 3 the
//! job had already been started, 4 the job overran its region's overrun zone (an error is
//! stored and the host poisons the instance).
//!
//! # The failure taxonomy, and the inherited `SILENT` table
//!
//! A host answers a `SERVICE` effect with bindings, `{kind: "transport"}` or
//! `{kind: "denied"}`, or abandons it on the job's signal. After resuming, the transport
//! observes the job's stop signal **first**:
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
//! From there the evaluator's own contract decides, unchanged (see
//! `purrdf_sparql_eval::service`): `SERVICE SILENT` swallows a transport or decode
//! failure to the join identity and nothing else — a denial and a governor trip are never
//! silenced, and neither is a fault, because a fault is not an answer at all. `silent` is
//! handed to the host for information only; an empty answer is not the host's to invent.
//!
//! A `LOAD` effect answered with a transport failure becomes the evaluator's
//! `native-sparql-load-failed`, which `LOAD SILENT` swallows exactly as it swallows an
//! unreachable document. A denial becomes
//! [`LOAD_DENIED`](purrdf_sparql_eval::LOAD_DENIED), whose message reads "the host denied
//! the request: …" (there is no native catalog gate for `LOAD` the way there is for
//! `SERVICE`, so every `LOAD` denial is, from this evaluator's point of view, the host's
//! own decision — see [`load_answer`]); it fails the request even under `LOAD SILENT`, as
//! a denied `SERVICE` does. A host fault is latched and so trips the request.
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
//! deadline at every yield and every effect, and the host's own deadline timer latches it
//! through [`AsyncJob::trip_deadline`].
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
//! set would be read by the next caller as its own. Every candidate on the evaluation
//! path, every `thread_local!` compiled into this package, and why none is observed
//! across a suspension:
//!
//! | State | Held across a poll or resolver call? |
//! |---|---|
//! | The stack floor (`purrdf-stack`'s thread-local `FLOOR`) | Yes — every frame of a suspended job is measured against its region's base. Swapped: it is half of the [`purrdf_stack::Context`] the job installs when its run starts, puts back at every suspension, reinstalls at every resumption and puts back when the run returns (`JspiStopWatch::leave_region` / `enter_region`, `JobInner::run`). |
//! | The evaluator's walk-scope state (`purrdf-stack`'s thread-local `WALK`: no scope, a scope open, a refusal latched with the floor it replaced) | Yes — a property path's traversal runs inside a walk scope and polls the stop signal at every step, so a job yields with that scope open. Swapped with the floor as the other half of the same `Context`: a job starts with no scope open, a context that runs while the job waits sees its own scopes rather than the job's (so its walks never latch a refusal in the job's scope, nor install the exhausted floor on a context that never refused), and jobs resumed in any order close their own scopes. A refusal latched in a scope never reaches a suspension in any case: it installs the exhausted floor, and the next poll's stack guard stops the job before it could yield. No other walk scope contains a poll or a resolver call — `CONSTRUCT` and update template instantiation, `EXISTS` preparation, the per-row substitution copy, a `SERVICE` body's analysis and serialization, a function body's copy and rewrite all run between polls. |
//! | `NativeSparqlEngine`'s plan cache (`RefCell<PlanCache>`) | No. Every borrow (`prepare_for`, `prepare_request`, `bind_functions`, `prepare_execution`, the stats accessors) is a temporary inside one statement that parses and admits a plan; planning polls no signal and calls no resolver, and the borrow ends before the returned `Arc<PreparedQuery>` is evaluated. The `engine_is_reentrant_from_a_resolver_and_from_a_poll` test below re-enters one engine from inside its own evaluation to prove it. |
//! | The BGP join-order cache (`Mutex`, `bgp::order_for`) | No. Locked only around the `get` and the `insert`; the cost-based ordering between them runs unlocked and polls nothing. |
//! | Plan-memory observers (`plan_memory::interner_memory_observer`'s thread-local `OBSERVER`, `PlanCharge`) | No. Locked only to add or credit a byte total. |
//! | Interned schemas and variables (`solution::INTERNED_SCHEMAS`, `substitute::INTERNED_VARIABLES`, thread-local `RefCell`s) | No. Borrowed only inside a pure memo lookup or insert. |
//! | `GovernorState` (`poll_stop`, `trip`) | No. It polls the signal *before* touching its `OnceLock`, never inside the initializer. |
//! | Lazily built dataset indexes (`OnceLock` permutations, predecessor and value indexes, path-relation adjacency) | No. Their initializers are pure sorts and scans that poll nothing, so no initialization can be suspended half-done. |
//! | Reasoner state (`purrdf-entail`'s tableau `RefCell`s, the datalog engines) | No sharing: every closure run owns its own. |
//! | SHACL's ambient scopes (`purrdf-shapes`'s `CURRENT_GOVERNORS`, `CURRENT_SOURCES`, `CURRENT_FUNCTIONS`, `CURRENT_PROPERTY_FUNCTIONS`, `CURRENT_AGGREGATES`, `CURRENT_PARSER_OPTIONS`, `CURRENT_CALL_DEPTH`) and the extension environment memoized from them (`CACHED_ENV`) | Yes — a SHACL job installs its governors and sources with `enter_execution_scope`, and the engine its registries, through guards that stay open across every query and every poll between focus nodes, so a job yields inside them. Swapped: the lot is one [`purrdf_shapes::sparql::AmbientContext`], taken off the thread and put back beside the stack context at every suspension and resumption, and a job starts with none installed. A context that runs while the job waits sees its own scopes — a synchronous validation stays ungoverned, polls no suspended job's signal and has no `SERVICE` source — and each job's guards restore values it installed, in whatever order the jobs finish. Without the swap a synchronous validation run during a suspension polled the suspended job's signal and was stopped by it. |
//! | SHACL's per-thread engine (`purrdf-shapes`'s `SPARQL_ENGINE`, a `NativeSparqlEngine` whose plan cache is a `RefCell`) | No, for the reason the evaluator's own plan cache is not: every borrow is a temporary inside one planning statement that polls nothing and calls no resolver. |
//! | SHACL's prepared handles (`purrdf-shapes`'s `PREPARED_EXECUTIONS`, a `RefCell` map of cached `PreparedExecution`s) | No. The map is borrowed only to take a handle out or put one back, never across a run; a running validation holds its handle checked OUT, so a caller that runs while it waits finds no handle for that query and prepares its own, and nothing it does to the map reaches the suspended one. A handle put back by either is re-prepared at the next checkout if its environment is not the one then in force, so a handle prepared under another context's registries is never run. |
//! | This module's own [`JobSlots`] mutexes and the job registry (thread-local `JOBS` and `LAST_JOB_ID`) | No. Each is locked or borrowed inside one helper that returns owned values, and none is held when [`suspend`] is called. |
//! | `purrdf-sparql-eval`'s memo-verification switch (`MEMO_VERIFICATION_ENABLED`) | Absent from release builds (it exists only under `debug_assertions`), and never written during an evaluation: only a test harness sets it. |
//! | Test instrumentation thread-locals — `purrdf-sparql-eval`'s counters and strategy overrides (`LEVEL_ADVANCES`, `POWER_EXPANSIONS`, `NUMERIC_FOLD_TRACE`, `FORCE_PARALLEL`, `FORCE_CHUNK_SIZE`, `MERGE_COUNT`, `INDEX_OF_CALLS`, `FORCE_EXISTS_STRATEGY`, `SUPPRESS_FIRST_WITNESS_WRAP`, `PREPARED_EXISTS_BUILD_COUNT`), `purrdf-core`'s distance-kernel hooks (`BYPASSED`, `HIDDEN`), `purrdf-hnsw`'s `LACKING`, `purrdf-shapes`'s scheduler overrides (`FORCE_PARALLEL`, `FORCE_CHUNK_SIZE`) and class-index counter (`THREAD_INDEX_BUILDS`) | Not compiled into this package: every one is `#[cfg(test)]`. |
//!
//! `FORCE_SEQUENTIAL_OPERATION` (`purrdf_sparql_eval::parallel`) is a last-in-first-out
//! guard that interleaving would break, but it is set only by the fallible lazy-view
//! entries, which no operation here calls — and on `wasm32` rayon runs every fork inline
//! and sequentially (see `purrdf-sparql-eval`'s manifest), so the flag cannot change the
//! evaluation order there in any case.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;

use purrdf::{
    ClosureRelations, GovernedEntailment, JsonLdSerializeOptions, QueryEntailmentPlan, RdfDataset,
    RdfDiagnostic, parse_dataset, query_with_entailment_governed,
};
use purrdf_core::SparqlResult;
use purrdf_sparql_eval::protocol::negotiate;
use purrdf_sparql_eval::{
    CancellationFlag, GovernedOutcome, GovernedUpdateOutcome, GovernorState, GraphResolveRequest,
    GraphResolver, HttpRemoteQuerySource, HttpRequest, HttpTransport, InProcessServiceResolver,
    NativeSparqlEngine, QueryGovernors, QueryOptions, RemoteError, ResolvedBindings,
    ServiceCapabilities, ServiceCapability, ServiceCatalog as NativeServiceCatalog,
    ServiceCredential, ServiceProfile, ServiceRequest, ServiceResolver, StopCause, StopSignal,
    TrippedGovernor, WallDeadline,
};
use purrdf_sparql_results::ProvenanceNamespace;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::codec::resolve_media_type;
use crate::dataset::{Dataset, diag_to_err};
use crate::jsonld::{CompiledJsonLdContext, context_options, decode_options};
use crate::protocol::not_acceptable_message;
use crate::query::{
    EntailmentQueryOutcome, GovernorArgs, NegotiatedOutcome, NegotiatedValue, QueryEngine,
    QueryOutcome, QueryResult, UPDATE_REFUSES_MAX_ANSWERS, UpdateOutcome, aggregate_env_message,
    build_aggregates, build_provenance_namespace, entailment_query_outcome_from_native,
    kind_mismatch, negotiable_result_kind, negotiated_outcome_from_value,
    query_outcome_from_governed, query_result_from_sparql, serialize_configured_graph,
    serialize_query_result, sparql_request, update_outcome_from_governed,
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

/// The smallest region a job may run on: 512 KiB, four times the guard band, so a job
/// always has at least 384 KiB of usable stack.
const MIN_STACK_BYTES: u32 = 512 * 1024;

/// The guard band at the base of a region. A poll whose frame reaches it stops the job
/// before a deeper frame can leave the region.
const STACK_GUARD_BYTES: usize = 128 * 1024;

/// The overrun zone below every region's base: 1 MiB.
///
/// The guard band only stops frames that *poll*, and the parser's and evaluator's own
/// stack guards only frames that parse or evaluate. Work that is none of these —
/// serializing the answer, the walks over a parsed tree — can recurse past the region's
/// base, and below the base lies other heap memory: an overrun there would corrupt it silently. So
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

/// Polls between clock reads for the wall deadline.
const CLOCK_EVERY_POLLS: u64 = 1_024;

/// The word written at the base of every region, checked by the host at every
/// suspension. ASCII `PurD`, little-endian.
const STACK_CANARY: u32 = 0x4472_7550;

/// A delivery was accepted: it answers the outstanding effect.
const DELIVERY_ACCEPTED: u32 = 0;
/// A delivery named an effect that was already answered or abandoned — typically a host
/// promise settling after the job's signal won the race. Ignored; not a fault.
const DELIVERY_STALE: u32 = 1;
/// A delivery was malformed (a sequence number never issued, a payload the outstanding
/// effect cannot take, an unknown failure kind). The job's fault is latched.
const DELIVERY_FAULT: u32 = 2;
/// The job has already finished; the delivery changes nothing.
const DELIVERY_FINISHED: u32 = 3;

/// `purrdf_jspi_run`: the job's outcome is stored.
const RUN_OUTCOME: u32 = 0;
/// `purrdf_jspi_run`: the job's error is stored.
const RUN_ERROR: u32 = 1;
/// `purrdf_jspi_run`: no job has this id.
const RUN_UNKNOWN_JOB: u32 = 2;
/// `purrdf_jspi_run`: the job had already been started.
const RUN_ALREADY_STARTED: u32 = 3;
/// `purrdf_jspi_run`: the job's frames ran past its region's overrun zone. Memory outside
/// the job's allocation may have been overwritten, so the host must poison the instance;
/// the job's error names what happened.
const RUN_OVERRAN: u32 = 4;

/// `purrdf_jspi_suspend`: the effect was answered.
const SUSPEND_ANSWERED: u32 = 0;
/// `purrdf_jspi_suspend`: the host abandoned the effect on the job's stop signal.
const SUSPEND_ABANDONED: u32 = 1;
/// `purrdf_jspi_suspend`: a fault was latched on the job.
const SUSPEND_FAULT: u32 = 2;

// ---------------------------------------------------------------------------
// The raw JSPI import and the one frame function
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "./purrdf_jspi.mjs")]
unsafe extern "C" {
    /// Suspending on JSPI hosts. Returns a status: 0 delivered, 1 governed (the host
    /// aborted the effect on the job's stop signal), 2 fault latched on the job. The host
    /// writes `0` to `out[0]` before returning.
    fn purrdf_jspi_suspend(job: u32, ticket: u32, out: *mut u32) -> u32;
}

/// Suspend the running job on its outstanding `ticket`, returning the host's status.
///
/// The one function that calls the suspending import. See the module documentation for
/// the invariant its shape exists to keep: an address-taken local gives it a
/// shadow-stack frame, it is never inlined by rustc, and nothing is called after the
/// import returns, so its epilogue restores the job's own stack pointer before anything
/// else on resumption can move it.
#[cfg(target_arch = "wasm32")]
#[inline(never)]
fn suspend(job: u32, ticket: u32) -> u32 {
    let mut out = [0u32; 2];
    // SAFETY: `out` is a live, writable two-word local for the whole call; the host
    // writes only `out[0]`. The import has no other preconditions.
    let status = unsafe { purrdf_jspi_suspend(job, ticket, out.as_mut_ptr()) };
    // SAFETY: `out` is initialized and in scope; the volatile read keeps the frame (and
    // so the epilogue's restore) and adds no call after the import.
    unsafe { core::ptr::read_volatile(out.as_ptr()) }.wrapping_add(status)
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
/// The raw export `WebAssembly.promising` drives. Returns 0 when an outcome is stored, 1
/// when an error is stored, 2 when no job has this id, 3 when the job had already been
/// started, and 4 when the job's frames ran past its region's overrun zone (an error is
/// stored, and the host must poison the instance).
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_jspi_run(job: u32) -> u32 {
    run_job(job)
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
fn run_job(id: u32) -> u32 {
    // The registry's own `Rc` keeps the job (and so the region this call is running on)
    // alive for the whole run; `finish` refuses to drop it while the job is running.
    let Some(job) = lookup_job(id) else {
        return RUN_UNKNOWN_JOB;
    };
    job.run()
}

// ---------------------------------------------------------------------------
// The clock
// ---------------------------------------------------------------------------

/// Milliseconds since the Unix epoch — the host clock the evidence and the deadline's
/// remaining budget are read from.
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Milliseconds since the Unix epoch — the host clock the evidence and the deadline's
/// remaining budget are read from.
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64() * 1000.0)
}

/// Lock `mutex`, taking the value even if a previous holder panicked. On `wasm32` a
/// panic aborts the instance, so poisoning cannot be observed there; on the native build
/// a poisoned slot is still the best information there is.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

/// What an effect asks the host to do. The numbering is the protocol's; 4 is reserved
/// for paged datasets.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncEffectKind {
    /// Answer a forwarded `SERVICE` query.
    Service = 1,
    /// Fetch a `LOAD` document.
    Load = 2,
    /// Turn the event loop once and resume.
    Yield = 3,
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
            .finish_non_exhaustive()
    }
}

/// An effect's payload.
#[derive(Debug, Clone)]
enum EffectPayload {
    Service(Box<ServiceEffect>),
    Load { iri: String },
    Yield,
}

impl EffectPayload {
    const fn kind(&self) -> AsyncEffectKind {
        match self {
            Self::Service(_) => AsyncEffectKind::Service,
            Self::Load { .. } => AsyncEffectKind::Load,
            Self::Yield => AsyncEffectKind::Yield,
        }
    }
}

/// How a host failed an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureKind {
    /// The endpoint or document could not be reached or read.
    Transport,
    /// The host's policy refused the request.
    Denied,
}

impl FailureKind {
    fn parse(kind: &str) -> Option<Self> {
        match kind {
            "transport" => Some(Self::Transport),
            "denied" => Some(Self::Denied),
            _ => None,
        }
    }
}

/// What the host delivered for an effect.
#[derive(Debug)]
enum Delivered {
    /// SPARQL Results JSON bytes for a `SERVICE` effect.
    Bindings(Vec<u8>),
    /// A parsed document for a `LOAD` effect.
    Graph(Arc<RdfDataset>),
    /// A typed failure for either.
    Failure { kind: FailureKind, message: String },
    /// The host abandoned the effect on the job's stop signal.
    Governed,
}

impl Delivered {
    /// Whether an outstanding effect of `kind` can take this delivery.
    const fn answers(&self, kind: AsyncEffectKind) -> bool {
        matches!(
            (self, kind),
            (Self::Bindings(_), AsyncEffectKind::Service)
                | (Self::Graph(_), AsyncEffectKind::Load)
                | (
                    Self::Failure { .. } | Self::Governed,
                    AsyncEffectKind::Service | AsyncEffectKind::Load
                )
        )
    }

    const fn label(&self) -> &'static str {
        match self {
            Self::Bindings(_) => "bindings",
            Self::Graph(_) => "a graph",
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
}

// The payload's own `Debug` already redacts header values.
impl fmt::Debug for AsyncEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncEffect")
            .field("seq", &self.seq)
            .field("payload", &self.payload)
            .finish()
    }
}

impl AsyncEffect {
    const fn service(&self) -> Option<&ServiceEffect> {
        match &self.payload {
            EffectPayload::Service(effect) => Some(effect),
            _ => None,
        }
    }
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
    pub fn kind(&self) -> AsyncEffectKind {
        self.payload.kind()
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

    /// `SERVICE`: the `Accept` header value (`application/sparql-results+json`).
    #[wasm_bindgen(getter)]
    pub fn accept(&self) -> Option<String> {
        self.service().map(|effect| effect.accept.clone())
    }

    /// `SERVICE`: the `Content-Type` header value (`application/sparql-query`).
    #[wasm_bindgen(getter, js_name = contentType)]
    pub fn content_type(&self) -> Option<String> {
        self.service().map(|effect| effect.content_type.clone())
    }

    /// `SERVICE`: the `User-Agent` the service's profile names, or the engine default.
    #[wasm_bindgen(getter, js_name = userAgent)]
    pub fn user_agent(&self) -> Option<String> {
        self.service().map(|effect| effect.user_agent.clone())
    }

    /// `SERVICE`: the per-request timeout the service's profile names, or the default,
    /// in milliseconds.
    #[wasm_bindgen(getter, js_name = timeoutMs)]
    pub fn timeout_ms(&self) -> Option<f64> {
        self.service().map(|effect| effect.timeout_ms)
    }

    /// `SERVICE`: the extra request headers as flattened `[name, value, name, value, …]`
    /// pairs — the service profile's headers, then its credential header — in the order
    /// they must be sent. Append each pair; a repeated name is legal and must not be
    /// merged. Empty when no catalog is configured.
    pub fn headers(&self) -> Vec<String> {
        self.service().map_or_else(Vec::new, |effect| {
            effect
                .headers
                .iter()
                .flat_map(|(name, value)| [name.clone(), value.clone()])
                .collect()
        })
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

    /// `LOAD`: the source IRI.
    #[wasm_bindgen(getter)]
    pub fn iri(&self) -> Option<String> {
        match &self.payload {
            EffectPayload::Load { iri } => Some(iri.clone()),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

/// Add `ms` to an `f64` accumulated in an atomic's bits.
fn add_ms(cell: &AtomicU64, ms: f64) {
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
struct AsyncCounters {
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
    evaluate_ms: AtomicU64,
    serialize_ms: AtomicU64,
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
        }
    }
}

impl AsyncCounters {
    /// Count one completed suspension of `kind` that waited `waited_ms`.
    fn record_suspension(&self, kind: AsyncEffectKind, waited_ms: f64) {
        let (count, wait) = match kind {
            AsyncEffectKind::Service => (&self.service_effects, &self.service_wait_ms),
            AsyncEffectKind::Load => (&self.load_effects, &self.load_wait_ms),
            AsyncEffectKind::Yield => (&self.yields, &self.yield_wait_ms),
        };
        count.fetch_add(1, Ordering::Relaxed);
        add_ms(wait, waited_ms);
    }

    fn snapshot(&self, stack_top: usize) -> AsyncEvidence {
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
        }
    }
}

/// What one asynchronous job cost: counts and phase times, all measured in Rust.
///
/// `evaluateMs` is the wall time of the evaluation phase and so *includes* the waits the
/// three `…WaitMs` fields report separately; `freezeMs` and `serializeMs` are the phases
/// that run to completion without yielding.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
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

    /// `SERVICE` effects the host answered or abandoned.
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
}

// ---------------------------------------------------------------------------
// Job slots: the state a host and a suspended job share
// ---------------------------------------------------------------------------

/// The ticket exchange between a job and its host.
#[derive(Debug, Default)]
struct Exchange {
    /// The last sequence number issued; `0` before the first.
    last_issued: u32,
    /// The effect the host has not taken yet.
    posted: Option<(u32, EffectPayload)>,
    /// The effect awaiting an answer, and its kind.
    outstanding: Option<(u32, AsyncEffectKind)>,
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
    /// Whether a frame at `sp`, with `left` bytes of stack below it, may keep running:
    /// inside the region and above its guard band. The error is the fault to latch.
    ///
    /// `left` is [`purrdf_stack::remaining`], measured against the floor
    /// [`JobInner::run`] installs — this region's base — so the guard band and the
    /// parser's and evaluator's own stack guards read one measurement rather than two.
    fn check(self, sp: usize, left: usize) -> Result<(), String> {
        if sp < self.base || sp > self.top {
            return Err(format!(
                "asynchronous job is not running on its stack region (stack pointer {sp:#x} \
                 outside [{:#x}, {:#x}]); the host did not switch the stack pointer",
                self.base, self.top
            ));
        }
        if left < STACK_GUARD_BYTES {
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
    exchange: Mutex<Exchange>,
    service_context: Mutex<Option<ServiceContext>>,
    /// The fatal job condition, first writer wins.
    fault: OnceLock<String>,
    /// The host's deadline timer fired ([`AsyncJob::trip_deadline`]).
    deadline_tripped: AtomicBool,
    /// The run has returned; deliveries are refused.
    finished: AtomicBool,
    /// Whether polls may compare the stack pointer with [`Self::bounds`]: set only while
    /// the job runs on its region, which is only ever on `wasm32`.
    region_armed: AtomicBool,
    bounds: StackBounds,
    /// The stack context ([`purrdf_stack::replace_context`]: the stack floor and the
    /// walk-scope state) of the context the job was started or last resumed from, put
    /// back whenever the job leaves its region: at every suspension and when the run
    /// returns. Only ever written on `wasm32`, where the job runs on its region; locked
    /// only to read or write the value, never across [`suspend`].
    outer_context: Mutex<purrdf_stack::Context>,
    /// The SHACL engine's ambient scopes ([`purrdf_shapes::sparql::AmbientContext`]:
    /// its governors, `SERVICE`/`LOAD` sources, registries and call depth) of the context
    /// the job was started or last resumed from, put back with [`Self::outer_context`]
    /// and on the same occasions. Only ever written on `wasm32`; locked only to move the
    /// value in or out, never across [`suspend`].
    outer_ambient: Mutex<purrdf_shapes::sparql::AmbientContext>,
    counters: AsyncCounters,
}

impl JobSlots {
    fn new(job: u32, bounds: StackBounds) -> Self {
        Self {
            job,
            exchange: Mutex::new(Exchange::default()),
            service_context: Mutex::new(None),
            fault: OnceLock::new(),
            deadline_tripped: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            region_armed: AtomicBool::new(false),
            bounds,
            outer_context: Mutex::new(purrdf_stack::Context::on_floor(0)),
            outer_ambient: Mutex::new(purrdf_shapes::sparql::AmbientContext::default()),
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

    /// Post an effect for the host and make it the outstanding one. `None` (with a fault
    /// latched) when another effect is still outstanding, which only a bridge that
    /// resumed a job without answering it could cause.
    fn issue(&self, payload: EffectPayload) -> Option<u32> {
        let kind = payload.kind();
        let mut exchange = lock(&self.exchange);
        if let Some((outstanding, _)) = exchange.outstanding {
            drop(exchange);
            self.latch_fault(format!(
                "an effect was issued while effect {outstanding} was still outstanding"
            ));
            return None;
        }
        let seq = exchange.last_issued.wrapping_add(1).max(1);
        exchange.last_issued = seq;
        exchange.posted = Some((seq, payload));
        exchange.outstanding = Some((seq, kind));
        exchange.delivered = None;
        drop(exchange);
        Some(seq)
    }

    /// Hand the posted effect to the host, once.
    fn take_effect(&self) -> Option<AsyncEffect> {
        let posted = lock(&self.exchange).posted.take();
        posted.map(|(seq, payload)| AsyncEffect { seq, payload })
    }

    /// Accept `value` for effect `seq`, or say why not.
    fn deliver(&self, seq: u32, value: Delivered) -> u32 {
        if self.finished.load(Ordering::Relaxed) {
            return DELIVERY_FINISHED;
        }
        let mut exchange = lock(&self.exchange);
        let fault = match exchange.outstanding {
            Some((outstanding, kind)) if outstanding == seq => {
                if value.answers(kind) {
                    exchange.outstanding = None;
                    exchange.posted = None;
                    exchange.delivered = Some((seq, value));
                    return DELIVERY_ACCEPTED;
                }
                format!(
                    "{} was delivered to effect {seq}, which is a {kind:?} effect",
                    value.label()
                )
            }
            _ if seq != 0 && seq <= exchange.last_issued => return DELIVERY_STALE,
            Some((outstanding, _)) => format!(
                "a delivery named effect {seq}, but the outstanding effect is {outstanding}"
            ),
            None => format!(
                "a delivery named effect {seq}, but no effect is outstanding (the last issued \
                 was {})",
                exchange.last_issued
            ),
        };
        drop(exchange);
        self.latch_fault(fault);
        DELIVERY_FAULT
    }

    /// After resuming from effect `seq`: take its answer, and close it whether or not
    /// one arrived.
    fn resume(&self, seq: u32) -> Option<Delivered> {
        let mut exchange = lock(&self.exchange);
        if exchange
            .outstanding
            .is_some_and(|(outstanding, _)| outstanding == seq)
        {
            exchange.outstanding = None;
        }
        exchange.posted = None;
        match exchange.delivered.take() {
            Some((delivered_seq, value)) if delivered_seq == seq => Some(value),
            other => {
                exchange.delivered = other;
                None
            }
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

/// A suspended job's per-thread state, held in its own frame while it waits: its stack
/// context and the SHACL engine's ambient scopes. See [`JspiStopWatch::leave_region`].
struct RegionContext {
    stack: purrdf_stack::Context,
    ambient: purrdf_shapes::sparql::AmbientContext,
}

/// The job's [`StopSignal`]: cancellation, the wall deadline, the host's deadline latch,
/// the fault latch, the stack guard — and the poll counter that slices evaluation into
/// yields.
#[derive(Debug)]
struct JspiStopWatch {
    slots: Arc<JobSlots>,
    cancel: CancellationFlag,
    deadline: Option<WallDeadline>,
    /// When the deadline falls, on [`now_ms`]'s clock, for the remaining budget handed
    /// to the host.
    deadline_at_ms: Option<f64>,
    /// Polls per yield; `0` yields at every poll.
    quantum: u32,
    polls_since_yield: AtomicU32,
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
            polls_since_yield: AtomicU32::new(0),
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
        // The evaluator's own measurement of this frame's depth and of the stack left
        // below it, against the region's base (see `JobInner::run`).
        let sp = purrdf_stack::stack_pointer();
        self.slots
            .counters
            .stack_low_water
            .fetch_min(sp, Ordering::Relaxed);
        self.slots.bounds.check(sp, purrdf_stack::remaining())
    }

    /// Suspend on the already-issued effect `seq`, count it, and return the host's
    /// status (an unknown status is latched as a fault and reported as one).
    fn suspend_on(&self, seq: u32, kind: AsyncEffectKind) -> u32 {
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
        self.polls_since_yield.store(0, Ordering::Relaxed);
        match status {
            SUSPEND_ANSWERED | SUSPEND_ABANDONED | SUSPEND_FAULT => status,
            other => {
                self.slots.latch_fault(format!(
                    "the host bridge returned status {other} for effect {seq}"
                ));
                SUSPEND_FAULT
            }
        }
    }

    /// Put the outer context's per-thread state back before the job leaves its region,
    /// and return the job's own to reinstall on the way back in: its stack context (its
    /// region's floor and its open walk scopes) and the SHACL engine's ambient scopes (a
    /// validation's governors, sources, registries and call depth, installed by guards
    /// the job is still inside). `None` off `wasm32`, where the job never runs on its
    /// region and nothing runs while it is suspended.
    fn leave_region(&self) -> Option<RegionContext> {
        if !cfg!(target_arch = "wasm32") {
            return None;
        }
        let outer_stack = *lock(&self.slots.outer_context);
        let outer_ambient = std::mem::take(&mut *lock(&self.slots.outer_ambient));
        Some(RegionContext {
            stack: purrdf_stack::replace_context(outer_stack),
            ambient: purrdf_shapes::sparql::replace_ambient_context(outer_ambient),
        })
    }

    /// Reinstall the job's per-thread state on resumption, recording the state of the
    /// context that resumed the job as the one to put back next. A no-op off `wasm32`.
    fn enter_region(&self, region: Option<RegionContext>) {
        let Some(region) = region else {
            return;
        };
        let outer_stack = purrdf_stack::replace_context(region.stack);
        *lock(&self.slots.outer_context) = outer_stack;
        let outer_ambient = purrdf_shapes::sparql::replace_ambient_context(region.ambient);
        *lock(&self.slots.outer_ambient) = outer_ambient;
    }

    /// Give the event loop back once.
    fn yield_now(&self) {
        let Some(seq) = self.slots.issue(EffectPayload::Yield) else {
            return;
        };
        let _ = self.suspend_on(seq, AsyncEffectKind::Yield);
        let _ = self.slots.resume(seq);
    }

    /// Milliseconds left before the deadline, when one is set.
    fn remaining_deadline_ms(&self) -> Option<f64> {
        self.deadline_at_ms.map(|at| (at - now_ms()).max(0.0))
    }
}

impl StopSignal for JspiStopWatch {
    fn poll(&self) -> Option<StopCause> {
        let total = self.slots.counters.polls.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some(cause) = self.peek() {
            return Some(cause);
        }
        if let Err(fault) = self.stack_check() {
            self.slots.latch_fault(fault);
            return Some(self.latch(StopCause::Cancelled));
        }
        if total.is_multiple_of(CLOCK_EVERY_POLLS)
            && let Some(cause) = self.deadline.as_ref().and_then(StopSignal::poll)
        {
            return Some(self.latch(cause));
        }
        let since = self.polls_since_yield.fetch_add(1, Ordering::Relaxed) + 1;
        if self.quantum == 0 || since >= self.quantum {
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
/// whether the job's stop signal had fired by the time it resumed. See the module
/// documentation's taxonomy table.
fn service_answer(
    endpoint: &str,
    fired: Option<StopCause>,
    delivered: Option<Delivered>,
    slots: &JobSlots,
) -> Result<Vec<u8>, RemoteError> {
    if let Some(cause) = fired {
        let trip = TrippedGovernor::Stopped { cause };
        return Err(match delivered {
            Some(Delivered::Bindings(_)) => RemoteError::GovernedAfterCompletion(trip),
            _ => RemoteError::Governed(trip),
        });
    }
    let cancelled = TrippedGovernor::Stopped {
        cause: StopCause::Cancelled,
    };
    match delivered {
        Some(Delivered::Bindings(bytes)) => Ok(bytes),
        Some(Delivered::Failure {
            kind: FailureKind::Transport,
            message,
        }) => Err(RemoteError::Transport(message)),
        // A host resolver's own policy refusal, reached only when no NATIVE catalog
        // already denied the request — see `HttpRemoteQuerySource::resolve`, which
        // applies an installed `ServiceCatalog` (and returns `RemoteError::Denied`, a real
        // withheld capability) BEFORE this transport is ever reached. Whatever the host
        // reports here is therefore its own decision, not a capability this engine's
        // catalog withheld, so it must not be reported as one — see `RemoteError::HostDenied`.
        Some(Delivered::Failure {
            kind: FailureKind::Denied,
            message,
        }) => Err(RemoteError::HostDenied {
            endpoint: endpoint.to_owned(),
            message,
        }),
        Some(Delivered::Governed) => {
            slots.latch_fault(format!(
                "the host abandoned the SERVICE <{endpoint}> effect, but the job's stop signal \
                 had not fired"
            ));
            Err(RemoteError::Governed(cancelled))
        }
        Some(Delivered::Graph(_)) => {
            slots.latch_fault(format!("a graph was delivered for SERVICE <{endpoint}>"));
            Err(RemoteError::Governed(cancelled))
        }
        None => {
            slots.latch_fault(format!(
                "resolver returned without a delivery for SERVICE <{endpoint}>"
            ));
            Err(RemoteError::Governed(cancelled))
        }
    }
}

/// Map what a host delivered for a `LOAD` effect onto the `LOAD` seam.
///
/// A fired signal is reported as a diagnostic here, but it is not what the request
/// reports: the evaluator polls the same signal the moment this returns and aborts the
/// request with the trip, `SILENT` or not.
fn load_answer(
    iri: &str,
    fired: Option<StopCause>,
    delivered: Option<Delivered>,
    slots: &JobSlots,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    if let Some(cause) = fired {
        return Err(RdfDiagnostic::error(
            "native-sparql-load-stopped",
            format!("LOAD <{iri}>: {}", TrippedGovernor::Stopped { cause }),
        ));
    }
    match delivered {
        Some(Delivered::Graph(dataset)) => Ok(dataset),
        Some(Delivered::Failure {
            kind: FailureKind::Transport,
            message,
        }) => Err(RdfDiagnostic::error(
            "native-sparql-load-failed",
            format!("LOAD <{iri}>: {message}"),
        )),
        // Unlike a `SERVICE` denial, there is no native catalog gate for `LOAD` (see this
        // module's doc comment): `ServiceCatalog::authorizeLoad` is a check the HOST makes
        // of its own accord, before it ever calls back here, so whatever this delivery
        // reports is, from this evaluator's point of view, always the host's own decision
        // — reported as such rather than assumed to be a catalog-capability cause.
        Some(Delivered::Failure {
            kind: FailureKind::Denied,
            message,
        }) => Err(RdfDiagnostic::error(
            purrdf_sparql_eval::LOAD_DENIED,
            format!("LOAD <{iri}>: the host denied the request: {message}"),
        )),
        other => {
            let message = match other {
                Some(Delivered::Governed) => format!(
                    "the host abandoned the LOAD <{iri}> effect, but the job's stop signal had \
                     not fired"
                ),
                Some(Delivered::Bindings(_)) => format!("bindings were delivered for LOAD <{iri}>"),
                _ => format!("resolver returned without a delivery for LOAD <{iri}>"),
            };
            slots.latch_fault(message.clone());
            Err(RdfDiagnostic::error("native-sparql-load-fault", message))
        }
    }
}

/// The [`HttpTransport`] that suspends the job on a `SERVICE` effect.
///
/// Wrapped in an [`HttpRemoteQuerySource`], so the catalog policy, the request shape and
/// the bounded SPARQL Results JSON decode are all the native ones; this type only moves
/// the request to the host and the answer back.
#[derive(Debug)]
struct JspiTransport {
    watch: Arc<JspiStopWatch>,
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
        };
        let Some(seq) = slots.issue(EffectPayload::Service(Box::new(effect))) else {
            return service_answer(request.endpoint, self.watch.observe_now(), None, slots);
        };
        let _ = self.watch.suspend_on(seq, AsyncEffectKind::Service);
        let delivered = slots.resume(seq);
        // `request.stop` is this same watch; observing it directly avoids a second poll
        // that could slice into a yield before the answer is even mapped.
        let fired = self.watch.observe_now();
        service_answer(request.endpoint, fired, delivered, slots)
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

/// The fallback for a job with local services but no host `SERVICE` handler: an endpoint
/// that is not local fails exactly as the offline lane's missing source does —
/// silenceable, and otherwise a hard error.
#[derive(Debug)]
struct UnhandledServiceSource;

impl ServiceResolver for UnhandledServiceSource {
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        if let Some(trip) = request.stop_trip() {
            return Err(trip);
        }
        Err(RemoteError::Transport(
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
/// [`purrdf_sparql_eval::ServiceRouter`] with one route per local endpoint and the host's source as its
/// fallback does — a fired signal prevents the exchange, a local endpoint is answered in
/// process, every other endpoint goes to the fallback — without building one per
/// request: a router borrows its resolvers, and this owns them. A job without local
/// services hands every request to the host's source directly, as it always has.
#[derive(Debug)]
struct JobServiceSource {
    /// The in-process resolver and the endpoints it serves.
    local: Option<(InProcessServiceResolver, Vec<String>)>,
    host: Option<HostServiceSource>,
}

impl ServiceResolver for JobServiceSource {
    fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
        let fallback: &(dyn ServiceResolver + Sync) = match &self.host {
            Some(host) => host,
            None => &UnhandledServiceSource,
        };
        let Some((local, endpoints)) = &self.local else {
            return fallback.resolve(request);
        };
        // The router's own sequence. Its no-route denial cannot arise: there is always a
        // fallback.
        if let Some(trip) = request.stop_trip() {
            return Err(trip);
        }
        if endpoints
            .iter()
            .any(|endpoint| endpoint == request.endpoint)
        {
            local.resolve(request)
        } else {
            fallback.resolve(request)
        }
    }
}

/// The `LOAD` source that suspends the job on a `Load` effect.
#[derive(Debug)]
struct JspiGraphResolver {
    watch: Arc<JspiStopWatch>,
}

impl GraphResolver for JspiGraphResolver {
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let slots = &self.watch.slots;
        let payload = EffectPayload::Load {
            iri: request.iri.to_owned(),
        };
        let Some(seq) = slots.issue(payload) else {
            let fired = self.watch.observe_now();
            return load_answer(request.iri, fired, None, slots);
        };
        let _ = self.watch.suspend_on(seq, AsyncEffectKind::Load);
        let delivered = slots.resume(seq);
        let fired = self.watch.observe_now();
        load_answer(request.iri, fired, delivered, slots)
    }
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
        match self {
            Self::Query => "query",
            Self::Raw => "raw",
            Self::RawWithContext => "rawWithContext",
            Self::Governed => "governed",
            Self::EntailmentGoverned => "entailmentGoverned",
            Self::Update => "update",
            Self::UpdateGoverned => "updateGoverned",
            Self::Negotiated => "negotiated",
            Self::Explain => "explain",
            Self::Shacl => "shacl",
        }
    }

    const fn is_governed(self) -> bool {
        matches!(
            self,
            Self::Governed | Self::EntailmentGoverned | Self::UpdateGoverned | Self::Negotiated
        )
    }
}

/// How a job failed, for the host to decide what to reject with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobErrorKind {
    /// A parse or evaluation failure, or a refused result shape.
    Error,
    /// An ungoverned operation's cancellation.
    Cancelled,
    /// An ungoverned operation's deadline (latched by the host).
    Deadline,
    /// A latched fault.
    Fault,
    /// A negotiated query's result, whose shape (a graph carrying named graphs) no format
    /// the `Accept` header allows can carry.
    NotAcceptable,
}

impl JobErrorKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Cancelled => "cancelled",
            Self::Deadline => "deadline",
            Self::Fault => "fault",
            Self::NotAcceptable => "not-acceptable",
        }
    }
}

#[derive(Debug, Clone)]
struct JobError {
    kind: JobErrorKind,
    message: String,
}

impl JobError {
    fn error(message: impl Into<String>) -> Self {
        Self {
            kind: JobErrorKind::Error,
            message: message.into(),
        }
    }

    /// A negotiated query whose result no acceptable format can carry.
    const fn not_acceptable(message: String) -> Self {
        Self {
            kind: JobErrorKind::NotAcceptable,
            message,
        }
    }

    /// An ungoverned operation stopped by its signal. It has no outcome to carry a
    /// truncation in, so the stop is its error.
    fn stopped(tripped: TrippedGovernor) -> Self {
        match tripped {
            TrippedGovernor::Stopped {
                cause: StopCause::Cancelled,
            } => Self {
                kind: JobErrorKind::Cancelled,
                message: "the asynchronous operation was cancelled".to_owned(),
            },
            TrippedGovernor::Stopped {
                cause: StopCause::Deadline,
            } => Self {
                kind: JobErrorKind::Deadline,
                message: "the asynchronous operation's deadline expired".to_owned(),
            },
            other => Self::error(format!("the asynchronous operation stopped: {other}")),
        }
    }
}

/// What a finished operation left for the host to take.
enum JobOutcome {
    Query(SparqlResult),
    Raw(Vec<u8>),
    Governed(Box<GovernedOutcome>),
    Entailment(Box<GovernedEntailment>),
    Updated(Arc<RdfDataset>),
    UpdateGoverned {
        outcome: GovernedUpdateOutcome,
        frozen: Option<Arc<RdfDataset>>,
    },
    Negotiated(Box<NegotiatedValue>),
    /// A change validation's log beside the scope it describes.
    ShaclChange(ShaclChangeValidation),
    /// A prepared-product refusal: the job's error, carried as the class the synchronous
    /// twin rejects with rather than flattened into a message.
    Refused(ShaclProductRefusal),
    Failed(JobError),
}

impl fmt::Debug for JobOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Query(_) => "Query",
            Self::Raw(_) => "Raw",
            Self::Governed(_) => "Governed",
            Self::Entailment(_) => "Entailment",
            Self::Updated(_) => "Updated",
            Self::UpdateGoverned { .. } => "UpdateGoverned",
            Self::Negotiated(_) => "Negotiated",
            Self::ShaclChange(_) => "ShaclChange",
            Self::Refused(_) => "Refused",
            Self::Failed(_) => "Failed",
        })
    }
}

/// What an operation runs with: the job's stop signal and effect sources.
///
/// The sources are owned (`Arc`) rather than borrowed so an operation can hand them to
/// code that reads its sources off an ambient scope rather than off a request — SHACL
/// validation installs them with [`purrdf_shapes::sparql::enter_execution_scope`].
struct JobRun<'r> {
    stop: Arc<dyn StopSignal>,
    remote: Option<Arc<dyn ServiceResolver + Send + Sync>>,
    load: Option<Arc<dyn GraphResolver + Send + Sync>>,
    counters: &'r AsyncCounters,
}

impl JobRun<'_> {
    /// `ceilings` with the job's stop signal attached.
    fn governors(&self, ceilings: QueryGovernors) -> QueryGovernors {
        ceilings.with_stop_signal(Arc::clone(&self.stop))
    }

    /// Request options carrying the job's sources and `env`.
    fn options<'o>(&'o self, env: &'o purrdf_sparql_eval::ExtensionEnv) -> QueryOptions<'o> {
        QueryOptions::new()
            .with_env(env)
            .with_remote(
                self.remote
                    .as_deref()
                    .map(|remote| remote as &(dyn ServiceResolver + Sync)),
            )
            .with_load(
                self.load
                    .as_deref()
                    .map(|load| load as &(dyn GraphResolver + Sync)),
            )
    }

    /// The job's sources, for an ambient execution scope.
    fn sources(&self) -> purrdf_shapes::sparql::QuerySources {
        purrdf_shapes::sparql::QuerySources {
            remote: self.remote.clone(),
            load: self.load.clone(),
        }
    }

    fn timed<T>(&self, cell: &AtomicU64, work: impl FnOnce() -> T) -> T {
        let started = now_ms();
        let result = work();
        add_ms(cell, now_ms() - started);
        result
    }

    fn evaluate<T>(&self, work: impl FnOnce() -> T) -> T {
        self.timed(&self.counters.evaluate_ms, work)
    }

    fn serialize<T>(&self, work: impl FnOnce() -> T) -> T {
        self.timed(&self.counters.serialize_ms, work)
    }
}

/// A job's work: a closure over the run's signal and sources.
type Operation = Box<dyn FnOnce(&JobRun<'_>) -> JobOutcome>;

/// Everything an operation is built from.
struct OperationInput {
    kind: AsyncOperationKind,
    engine: Rc<NativeSparqlEngine>,
    frozen: Arc<RdfDataset>,
    sparql: String,
    base: Option<String>,
    aggregate_namespace: Option<String>,
    ceilings: GovernorArgs,
    format: Option<String>,
    provenance: Option<ProvenanceNamespace>,
    jsonld: Option<JsonLdSerializeOptions>,
    regime: Option<String>,
    program: Option<String>,
    accept: Option<String>,
}

/// Run an ungoverned query under the metered base and the job's signal.
fn ungoverned_query(
    run: &JobRun<'_>,
    engine: &NativeSparqlEngine,
    frozen: &Arc<RdfDataset>,
    sparql: &str,
    base: Option<&str>,
) -> Result<SparqlResult, JobError> {
    let governors = run.governors(QueryGovernors::METERED);
    let options = run.options(QueryOptions::EMPTY.env);
    let outcome = run.evaluate(|| {
        engine.query_governed(frozen, sparql_request(sparql, base), options, &governors)
    });
    match outcome {
        Err(diagnostic) => Err(JobError::error(diagnostic.to_string())),
        Ok(GovernedOutcome::Complete { result, .. }) => Ok(result),
        Ok(GovernedOutcome::BudgetExhausted(exhausted)) => {
            Err(JobError::stopped(exhausted.tripped))
        }
    }
}

/// The aggregate environment a governed operation's `aggregateNamespace` requests.
fn governed_env(
    aggregate_namespace: Option<String>,
) -> Result<purrdf_sparql_eval::ExtensionEnv, JobError> {
    aggregate_env_message(build_aggregates(aggregate_namespace).as_ref()).map_err(JobError::error)
}

impl OperationInput {
    fn into_operation(self) -> Operation {
        Box::new(move |run| self.execute(run).unwrap_or_else(JobOutcome::Failed))
    }

    fn execute(self, run: &JobRun<'_>) -> Result<JobOutcome, JobError> {
        let Self {
            kind,
            engine,
            frozen,
            sparql,
            base,
            aggregate_namespace,
            ceilings,
            format,
            provenance,
            jsonld,
            regime,
            program,
            accept,
        } = self;
        let request = sparql_request(&sparql, base.as_deref());
        match kind {
            AsyncOperationKind::Query => Ok(JobOutcome::Query(ungoverned_query(
                run,
                &engine,
                &frozen,
                &sparql,
                base.as_deref(),
            )?)),
            AsyncOperationKind::Raw | AsyncOperationKind::RawWithContext => {
                let result = ungoverned_query(run, &engine, &frozen, &sparql, base.as_deref())?;
                let text = run.serialize(|| match (&jsonld, format.as_deref()) {
                    (Some(options), Some(format)) => {
                        serialize_configured_graph(result, format, options)
                    }
                    _ => serialize_query_result(
                        &result,
                        format.as_deref(),
                        provenance.as_ref(),
                        &sparql,
                    ),
                });
                Ok(JobOutcome::Raw(text.map_err(JobError::error)?.into_bytes()))
            }
            AsyncOperationKind::Governed => {
                let env = governed_env(aggregate_namespace)?;
                let governors = run.governors(ceilings.ceilings());
                let outcome = run
                    .evaluate(|| {
                        engine.query_governed(&frozen, request, run.options(&env), &governors)
                    })
                    .map_err(|diagnostic| JobError::error(diagnostic.to_string()))?;
                Ok(JobOutcome::Governed(Box::new(outcome)))
            }
            AsyncOperationKind::Negotiated => {
                let env = governed_env(aggregate_namespace)?;
                let governors = run.governors(ceilings.ceilings());
                let outcome = run
                    .evaluate(|| {
                        engine.query_governed(&frozen, request, run.options(&env), &governors)
                    })
                    .map_err(|diagnostic| JobError::error(diagnostic.to_string()))?;
                let value = match outcome {
                    GovernedOutcome::Complete {
                        result, evidence, ..
                    } => {
                        // Negotiated against the result's actual shape: a graph carrying
                        // named graphs is offered only in the syntaxes that can hold it,
                        // so no format ever silently drops a graph.
                        let shape = negotiable_result_kind(&result);
                        let format = negotiate(accept.as_deref(), shape).ok_or_else(|| {
                            JobError::not_acceptable(not_acceptable_message(shape))
                        })?;
                        let text = run
                            .serialize(|| {
                                serialize_query_result(&result, Some(format), None, &sparql)
                            })
                            .map_err(JobError::error)?;
                        NegotiatedValue::Complete {
                            bytes: text.into_bytes(),
                            format,
                            evidence,
                        }
                    }
                    GovernedOutcome::BudgetExhausted(exhausted) => {
                        NegotiatedValue::Exhausted(exhausted)
                    }
                };
                Ok(JobOutcome::Negotiated(Box::new(value)))
            }
            AsyncOperationKind::EntailmentGoverned => {
                let regime = regime.unwrap_or_default();
                let plan = QueryEntailmentPlan::parse(&regime, program.as_deref().unwrap_or(""))
                    .map_err(JobError::error)?;
                let env = governed_env(aggregate_namespace)?;
                let governors = run.governors(ceilings.ceilings());
                let outcome = run
                    .evaluate(|| {
                        query_with_entailment_governed(
                            &engine,
                            &frozen,
                            request,
                            plan.entailment(),
                            run.options(&env),
                            // This surface registers no relation, exactly as the
                            // synchronous twin does.
                            &ClosureRelations::NONE,
                            &governors,
                        )
                    })
                    .map_err(|error| JobError::error(error.to_string()))?;
                Ok(JobOutcome::Entailment(Box::new(outcome)))
            }
            AsyncOperationKind::Explain => {
                // The synchronous twin's measuring run — metered, never bounded — with
                // the job's sources installed and its signal polled at every charge point.
                let explanation = run
                    .evaluate(|| {
                        engine.explain_query_with_stop_signal(
                            &frozen,
                            &sparql,
                            base.as_deref(),
                            run.options(QueryOptions::EMPTY.env),
                            Arc::clone(&run.stop),
                        )
                    })
                    .map_err(|diagnostic| JobError::error(diagnostic.to_string()))?;
                // A stop cut the measuring run short, so its ledger describes a truncated
                // run rather than the query: the stop is the job's error, as it is for
                // every ungoverned operation. Any other trip is part of the explanation,
                // exactly as the synchronous twin renders it.
                if let Some(tripped @ TrippedGovernor::Stopped { .. }) =
                    explanation.evidence().tripped
                {
                    return Err(JobError::stopped(tripped));
                }
                Ok(JobOutcome::Raw(explanation.render().into_bytes()))
            }
            AsyncOperationKind::Update => {
                let governors = run.governors(QueryGovernors::METERED);
                let mut target = Arc::clone(&frozen);
                let outcome = run
                    .evaluate(|| {
                        engine.update_governed(
                            &mut target,
                            request,
                            run.options(QueryOptions::EMPTY.env),
                            &governors,
                        )
                    })
                    .map_err(|diagnostic| JobError::error(diagnostic.to_string()))?;
                match outcome.tripped() {
                    None => Ok(JobOutcome::Updated(target)),
                    Some(tripped) => Err(JobError::stopped(tripped)),
                }
            }
            // `beginAsync` refuses the kind before an input is ever built; a SHACL job's
            // operation is `execute_shacl`, over no dataset and no SPARQL text.
            AsyncOperationKind::Shacl => Err(JobError::error(SHACL_STARTS_ELSEWHERE)),
            AsyncOperationKind::UpdateGoverned => {
                let env = governed_env(aggregate_namespace)?;
                let governors = run.governors(ceilings.ceilings());
                let mut target = Arc::clone(&frozen);
                let outcome = run
                    .evaluate(|| {
                        engine.update_governed(&mut target, request, run.options(&env), &governors)
                    })
                    .map_err(|diagnostic| JobError::error(diagnostic.to_string()))?;
                // The engine publishes into `target` only on the applied path, so a
                // tripped request offers nothing to commit.
                let frozen = outcome.is_applied().then_some(target);
                Ok(JobOutcome::UpdateGoverned { outcome, frozen })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SHACL operations
// ---------------------------------------------------------------------------

/// Why `beginAsync` refuses the SHACL kind.
const SHACL_STARTS_ELSEWHERE: &str =
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
            Ok(text) => JobOutcome::Raw(text.into_bytes()),
            Err(message) => JobOutcome::Failed(JobError::error(message)),
        };
        let refusable = |result: Result<String, ShaclProductRefusal>| match result {
            Ok(text) => JobOutcome::Raw(text.into_bytes()),
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
                Err(message) => JobOutcome::Failed(JobError::error(message)),
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
    service_handler: bool,
    load_handler: bool,
    catalog: Option<NativeServiceCatalog>,
    local_services: Vec<(String, Arc<RdfDataset>)>,
    dataset_id: u64,
    dataset_generation: u64,
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
    fn run(&self) -> u32 {
        if self.state.get() != JobState::Pending {
            return RUN_ALREADY_STARTED;
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
        // with no walk scope open, whatever the context that started it has open, until
        // it suspends or returns (see `JspiStopWatch::leave_region`).
        if cfg!(target_arch = "wasm32") {
            let outer = purrdf_stack::replace_context(purrdf_stack::Context::on_floor(
                self.region.bounds.base,
            ));
            *lock(&slots.outer_context) = outer;
            // Likewise the job starts with no SHACL scope installed, whatever the context
            // that started it has installed.
            let outer_ambient = purrdf_shapes::sparql::replace_ambient_context(
                purrdf_shapes::sparql::AmbientContext::default(),
            );
            *lock(&slots.outer_ambient) = outer_ambient;
        }
        let outcome = match (operation, self.watch.stack_check()) {
            (_, Err(fault)) => {
                slots.latch_fault(fault);
                JobOutcome::Failed(JobError::error("the job did not start"))
            }
            (Some(operation), Ok(())) => self.execute(operation),
            (None, Ok(())) => {
                slots.latch_fault("the job's operation was already taken");
                JobOutcome::Failed(JobError::error("the job had no operation"))
            }
        };
        slots.region_armed.store(false, Ordering::Relaxed);
        if cfg!(target_arch = "wasm32") {
            purrdf_stack::replace_context(*lock(&slots.outer_context));
            // Every guard the operation installed has dropped by now, so the job's own
            // context is idle and is discarded as the outer one goes back.
            let outer_ambient = std::mem::take(&mut *lock(&slots.outer_ambient));
            drop(purrdf_shapes::sparql::replace_ambient_context(
                outer_ambient,
            ));
        }
        match self.region.overrun() {
            Overrun::None => {}
            // Frames that never polled ran past the base into the zone: nothing outside
            // the job's allocation was touched, so this is the job's typed exhaustion.
            Overrun::Absorbed => slots.latch_fault(self.region.bounds.exhausted()),
            Overrun::Escaped => {
                slots.finished.store(true, Ordering::Relaxed);
                *self.error.borrow_mut() = Some(JobError {
                    kind: JobErrorKind::Fault,
                    message: format!(
                        "asynchronous job {} ran more than {} bytes past the base of its \
                         stack region ({} bytes), so memory outside it may be overwritten; \
                         raise stackBytes",
                        self.id,
                        STACK_OVERRUN_ZONE_BYTES - STACK_OVERRUN_FLOOR_BYTES,
                        self.region.bounds.top - self.region.bounds.base
                    ),
                });
                self.state.set(JobState::Done);
                return RUN_OVERRAN;
            }
        }
        slots.finished.store(true, Ordering::Relaxed);
        // A latched fault outranks whatever the operation reached: that outcome was
        // produced by a job whose effects cannot be trusted, and an update is never
        // committed from it.
        let outcome = match slots.fault() {
            Some(fault) => JobOutcome::Failed(JobError {
                kind: JobErrorKind::Fault,
                message: fault.to_owned(),
            }),
            None => outcome,
        };
        let status = match outcome {
            JobOutcome::Failed(error) => {
                *self.error.borrow_mut() = Some(error);
                RUN_ERROR
            }
            JobOutcome::Refused(refusal) => {
                *self.error.borrow_mut() = Some(JobError::error(refusal.to_js_string()));
                *self.refusal.borrow_mut() = Some(refusal);
                RUN_ERROR
            }
            JobOutcome::Updated(frozen) => {
                *self.pending_commit.borrow_mut() = Some(frozen);
                RUN_OUTCOME
            }
            JobOutcome::UpdateGoverned { outcome, frozen } => {
                *self.pending_commit.borrow_mut() = frozen;
                *self.outcome.borrow_mut() = Some(JobOutcome::UpdateGoverned {
                    outcome,
                    frozen: None,
                });
                RUN_OUTCOME
            }
            other => {
                *self.outcome.borrow_mut() = Some(other);
                RUN_OUTCOME
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
        let host = self.service_handler.then(|| {
            let transport = JspiTransport {
                watch: Arc::clone(&self.watch),
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
            }) as Arc<dyn GraphResolver + Send + Sync>
        });
        let stop: Arc<dyn StopSignal> = Arc::clone(&self.watch) as Arc<dyn StopSignal>;
        let run = JobRun {
            stop,
            remote,
            load,
            counters: &self.watch.slots.counters,
        };
        operation(&run)
    }

    fn require_done(&self, what: &str) -> Result<(), String> {
        match self.state.get() {
            JobState::Done => Ok(()),
            _ => Err(format!(
                "{what} is not available until the asynchronous job has finished"
            )),
        }
    }

    fn require_kind(&self, what: &str, kinds: &[AsyncOperationKind]) -> Result<(), String> {
        if kinds.contains(&self.kind) {
            Ok(())
        } else {
            Err(format!(
                "{what} is not available on a {} job",
                self.kind.name()
            ))
        }
    }

    /// The stored outcome, for a `take*` that expects this job's kind.
    fn take_outcome(&self, what: &str, kinds: &[AsyncOperationKind]) -> Result<JobOutcome, String> {
        self.require_kind(what, kinds)?;
        self.require_done(what)?;
        if let Some(error) = self.error.borrow().as_ref() {
            return Err(error.message.clone());
        }
        self.outcome
            .borrow_mut()
            .take()
            .ok_or_else(|| format!("{what}: the outcome was already taken"))
    }

    /// Commit an applied update into `dataset`, refusing any commit that could
    /// overwrite a mutation the update never saw.
    fn commit_into(&self, dataset: &mut Dataset) -> Result<(), String> {
        self.require_kind(
            "commitUpdate",
            &[
                AsyncOperationKind::Update,
                AsyncOperationKind::UpdateGoverned,
            ],
        )?;
        self.require_done("commitUpdate")?;
        if let Some(error) = self.error.borrow().as_ref() {
            return Err(error.message.clone());
        }
        if self.pending_commit.borrow().is_none() {
            return Err(
                "nothing to commit: the update was not applied, or was already committed"
                    .to_owned(),
            );
        }
        if dataset.identity() != self.dataset_id {
            return Err(format!(
                "commit targets a different dataset (the update read dataset {}, this is \
                 dataset {}); the update was not applied",
                self.dataset_id,
                dataset.identity()
            ));
        }
        if dataset.current_generation() != self.dataset_generation {
            return Err(format!(
                "dataset mutated while an asynchronous update was in flight (generation {} → \
                 {}); the update was not applied",
                self.dataset_generation,
                dataset.current_generation()
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

fn js_error(message: &str) -> JsError {
    JsError::new(message)
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

    /// How the job failed — `"error"`, `"cancelled"`, `"deadline"` or `"fault"` — or
    /// `undefined` when it did not (or has not finished).
    #[wasm_bindgen(getter, js_name = errorKind)]
    pub fn error_kind(&self) -> Option<String> {
        self.inner
            .error
            .borrow()
            .as_ref()
            .map(|error| error.kind.name().to_owned())
    }

    /// The effect the job is suspended on, once; `undefined` when there is none.
    #[wasm_bindgen(js_name = takeEffect)]
    pub fn take_effect(&self) -> Option<AsyncEffect> {
        self.inner.watch.slots.take_effect()
    }

    /// Answer `SERVICE` effect `seq` with SPARQL Results JSON bytes.
    #[wasm_bindgen(js_name = deliverBindings)]
    pub fn deliver_bindings(&self, seq: u32, bytes: Vec<u8>) -> u32 {
        self.inner
            .watch
            .slots
            .deliver(seq, Delivered::Bindings(bytes))
    }

    /// Fail effect `seq`: `kind` is `"transport"` (unreachable, or unreadable) or
    /// `"denied"` (the host's policy refused it). Any other kind is a fault.
    #[wasm_bindgen(js_name = deliverFailure)]
    pub fn deliver_failure(&self, seq: u32, kind: &str, message: String) -> u32 {
        let slots = &self.inner.watch.slots;
        match FailureKind::parse(kind) {
            Some(kind) => slots.deliver(seq, Delivered::Failure { kind, message }),
            None => {
                if slots.finished.load(Ordering::Relaxed) {
                    return DELIVERY_FINISHED;
                }
                slots.latch_fault(format!(
                    "unknown failure kind {kind:?} for effect {seq} (expected \"transport\" or \
                     \"denied\")"
                ));
                DELIVERY_FAULT
            }
        }
    }

    /// Answer `LOAD` effect `seq` with a document. It is parsed here, by media type (or
    /// any format name `Dataset.parse` accepts); a document that cannot be parsed is the
    /// `LOAD`'s failure, never a fault.
    #[wasm_bindgen(js_name = deliverGraph)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn deliver_graph(
        &self,
        seq: u32,
        bytes: &[u8],
        media_type: &str,
        base: Option<String>,
    ) -> u32 {
        let delivered = match parse_document(bytes, media_type, base.as_deref()) {
            Ok(dataset) => Delivered::Graph(dataset),
            Err(message) => Delivered::Failure {
                kind: FailureKind::Transport,
                message,
            },
        };
        self.inner.watch.slots.deliver(seq, delivered)
    }

    /// Answer `LOAD` effect `seq` with a snapshot of an existing dataset.
    #[wasm_bindgen(js_name = deliverGraphDataset)]
    pub fn deliver_graph_dataset(&self, seq: u32, dataset: &Dataset) -> u32 {
        let delivered = match dataset.view().freeze() {
            Ok(frozen) => Delivered::Graph(frozen),
            Err(diagnostic) => Delivered::Failure {
                kind: FailureKind::Transport,
                message: diagnostic.to_string(),
            },
        };
        self.inner.watch.slots.deliver(seq, delivered)
    }

    /// Record that the host abandoned effect `seq` on the job's stop signal. When that
    /// signal is not already latched (the host's own abort, rather than its deadline
    /// timer, which latches through [`Self::trip_deadline`] first) this cancels the job.
    #[wasm_bindgen(js_name = deliverGoverned)]
    pub fn deliver_governed(&self, seq: u32) -> u32 {
        let watch = &self.inner.watch;
        if watch.slots.finished.load(Ordering::Relaxed) {
            return DELIVERY_FINISHED;
        }
        if watch.peek().is_none() {
            watch.cancel.cancel();
        }
        watch.slots.deliver(seq, Delivered::Governed)
    }

    /// Latch `message` as the job's fault: a host bug the job cannot continue past. The
    /// job's signal fires, it winds down, and the fault becomes its error.
    pub fn fault(&self, message: String) -> u32 {
        let slots = &self.inner.watch.slots;
        if slots.finished.load(Ordering::Relaxed) {
            return DELIVERY_FINISHED;
        }
        slots.latch_fault(message);
        DELIVERY_ACCEPTED
    }

    /// Latch the job's deadline as expired — called by the host's deadline timer before
    /// it aborts the resolver's signal, so the trip is reported as a deadline.
    #[wasm_bindgen(js_name = tripDeadline)]
    pub fn trip_deadline(&self) -> u32 {
        let slots = &self.inner.watch.slots;
        if slots.finished.load(Ordering::Relaxed) {
            return DELIVERY_FINISHED;
        }
        slots.deadline_tripped.store(true, Ordering::Relaxed);
        DELIVERY_ACCEPTED
    }

    /// Cancel the job. It observes the cancellation at its next poll, yield or effect.
    pub fn cancel(&self) -> u32 {
        if self.inner.watch.slots.finished.load(Ordering::Relaxed) {
            return DELIVERY_FINISHED;
        }
        self.inner.watch.cancel.cancel();
        DELIVERY_ACCEPTED
    }

    /// A `query` job's typed result. `expect` — `"select"`, `"ask"`, `"construct"`,
    /// `"describe"` (or `"graph"`) — refuses any other result kind with the synchronous
    /// twin's message; `undefined` accepts any.
    #[wasm_bindgen(js_name = takeQueryResult)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn take_query_result(&self, expect: Option<String>) -> Result<QueryResult, JsError> {
        let expected = match expect.as_deref() {
            None => None,
            Some("select") => Some("SELECT solutions"),
            Some("ask") => Some("ASK boolean"),
            Some("construct" | "describe" | "graph") => Some("CONSTRUCT/DESCRIBE graph"),
            Some(other) => {
                return Err(js_error(&format!(
                    "unknown expected result kind {other:?} (expected select, ask, construct, \
                     describe or graph)"
                )));
            }
        };
        let outcome = self
            .inner
            .take_outcome("takeQueryResult", &[AsyncOperationKind::Query])
            .map_err(|message| js_error(&message))?;
        let JobOutcome::Query(result) = outcome else {
            return Err(js_error("takeQueryResult: the job holds no query result"));
        };
        if let Some(expected) = expected {
            let matches = matches!(
                (expected, &result),
                ("SELECT solutions", SparqlResult::Solutions { .. })
                    | ("ASK boolean", SparqlResult::Boolean(_))
                    | ("CONSTRUCT/DESCRIBE graph", SparqlResult::Graph(_))
            );
            if !matches {
                return Err(kind_mismatch(expected, &result));
            }
        }
        query_result_from_sparql(result)
    }

    /// A raw job's serialized bytes, an explain job's rendered ledger, or a SHACL job's
    /// SARIF log or entailed N-Triples (UTF-8 text).
    #[wasm_bindgen(js_name = takeRawBytes)]
    pub fn take_raw_bytes(&self) -> Result<Vec<u8>, JsError> {
        match self
            .inner
            .take_outcome(
                "takeRawBytes",
                &[
                    AsyncOperationKind::Raw,
                    AsyncOperationKind::RawWithContext,
                    AsyncOperationKind::Explain,
                    AsyncOperationKind::Shacl,
                ],
            )
            .map_err(|message| js_error(&message))?
        {
            JobOutcome::Raw(bytes) => Ok(bytes),
            _ => Err(js_error("takeRawBytes: the job holds no raw result")),
        }
    }

    /// A governed query job's outcome.
    #[wasm_bindgen(js_name = takeQueryOutcome)]
    pub fn take_query_outcome(&self) -> Result<QueryOutcome, JsError> {
        match self
            .inner
            .take_outcome("takeQueryOutcome", &[AsyncOperationKind::Governed])
            .map_err(|message| js_error(&message))?
        {
            JobOutcome::Governed(outcome) => query_outcome_from_governed(*outcome),
            _ => Err(js_error(
                "takeQueryOutcome: the job holds no governed outcome",
            )),
        }
    }

    /// A negotiated query job's outcome: the serialized complete answer, or the trip.
    #[wasm_bindgen(js_name = takeNegotiatedOutcome)]
    pub fn take_negotiated_outcome(&self) -> Result<NegotiatedOutcome, JsError> {
        match self
            .inner
            .take_outcome("takeNegotiatedOutcome", &[AsyncOperationKind::Negotiated])
            .map_err(|message| js_error(&message))?
        {
            JobOutcome::Negotiated(value) => negotiated_outcome_from_value(*value),
            _ => Err(js_error(
                "takeNegotiatedOutcome: the job holds no negotiated outcome",
            )),
        }
    }

    /// A governed entailment job's outcome.
    #[wasm_bindgen(js_name = takeEntailmentOutcome)]
    pub fn take_entailment_outcome(&self) -> Result<EntailmentQueryOutcome, JsError> {
        match self
            .inner
            .take_outcome(
                "takeEntailmentOutcome",
                &[AsyncOperationKind::EntailmentGoverned],
            )
            .map_err(|message| js_error(&message))?
        {
            JobOutcome::Entailment(outcome) => entailment_query_outcome_from_native(*outcome),
            _ => Err(js_error(
                "takeEntailmentOutcome: the job holds no entailment outcome",
            )),
        }
    }

    /// A governed update job's outcome. Whether it applied is on the outcome; applying it
    /// to the dataset is [`Self::commit_update`].
    #[wasm_bindgen(js_name = takeUpdateOutcome)]
    pub fn take_update_outcome(&self) -> Result<UpdateOutcome, JsError> {
        match self
            .inner
            .take_outcome("takeUpdateOutcome", &[AsyncOperationKind::UpdateGoverned])
            .map_err(|message| js_error(&message))?
        {
            JobOutcome::UpdateGoverned { outcome, .. } => {
                Ok(update_outcome_from_governed(&outcome))
            }
            _ => Err(js_error(
                "takeUpdateOutcome: the job holds no update outcome",
            )),
        }
    }

    /// A SHACL change-validation job's outcome: the SARIF log and the scope it
    /// describes, exactly as `shaclValidateChangesToSarif` returns them.
    #[wasm_bindgen(js_name = takeShaclChangeValidation)]
    pub fn take_shacl_change_validation(&self) -> Result<ShaclChangeValidation, JsError> {
        match self
            .inner
            .take_outcome("takeShaclChangeValidation", &[AsyncOperationKind::Shacl])
            .map_err(|message| js_error(&message))?
        {
            JobOutcome::ShaclChange(validation) => Ok(validation),
            _ => Err(js_error(
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

    /// The job's error message, once; `undefined` when it has none.
    #[wasm_bindgen(js_name = takeError)]
    pub fn take_error(&self) -> Option<String> {
        self.inner
            .error
            .borrow_mut()
            .take()
            .map(|error| error.message)
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
    pub fn commit_update(&self, dataset: &mut Dataset) -> Result<(), JsError> {
        self.inner
            .commit_into(dataset)
            .map_err(|message| js_error(&message))
    }

    /// Remove the job from the registry once it has finished: returns 0, or 1 (and does
    /// nothing) while it is still running, because the run is standing on the job's
    /// region.
    pub fn finish(&self) -> u32 {
        if self.inner.state.get() == JobState::Running {
            return 1;
        }
        unregister_job(self.inner.id);
        0
    }
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

/// The configuration of one asynchronous job, built by the package root through its
/// `set…` methods and validated when the job begins.
///
/// Every setter is an explicit method rather than a property, and none of them
/// validates: `beginAsync` does, all at once, so a refusal names the operation it was
/// refused for. The ceilings take `bigint` exactly as the synchronous governed entries
/// do; `yieldEveryPolls` and `stackBytes` take a `number` and are refused unless they are
/// integers in range, so a negative never wraps into a huge value.
#[wasm_bindgen]
#[derive(Debug, Default, Clone)]
pub struct AsyncJobOptions {
    base: Option<String>,
    format: Option<String>,
    options_json: Option<String>,
    provenance_prefix: Option<String>,
    provenance_iri: Option<String>,
    aggregate_namespace: Option<String>,
    regime: Option<String>,
    program: Option<String>,
    accept: Option<String>,
    fuel: Option<i64>,
    deadline_ms: Option<i64>,
    max_answers: Option<i64>,
    max_intermediate_cells: Option<i64>,
    max_scratch_bytes: Option<i64>,
    max_remote_requests: Option<i64>,
    yield_every_polls: Option<f64>,
    stack_bytes: Option<f64>,
    catalog: Option<NativeServiceCatalog>,
    local_services: Vec<(String, Arc<RdfDataset>)>,
    service_handler: bool,
    load_handler: bool,
}

/// An [`AsyncJobOptions`] that passed validation for one operation kind.
#[derive(Debug, Clone, Copy)]
struct ValidatedOptions {
    ceilings: GovernorArgs,
    quantum: u32,
    stack_bytes: u32,
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
impl AsyncJobOptions {
    /// Options with nothing set: every default, no handler, no ceiling.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// The query's base IRI.
    #[wasm_bindgen(js_name = setBase)]
    pub fn set_base(&mut self, base: Option<String>) {
        self.base = base;
    }

    /// The result format (raw operations only).
    #[wasm_bindgen(js_name = setFormat)]
    pub fn set_format(&mut self, format: Option<String>) {
        self.format = format;
    }

    /// A JSON-LD options document (`queryRawConfigured`; raw operations only).
    #[wasm_bindgen(js_name = setOptionsJson)]
    pub fn set_options_json(&mut self, options_json: Option<String>) {
        self.options_json = options_json;
    }

    /// The provenance namespace prefix (raw operations only; with the IRI).
    #[wasm_bindgen(js_name = setProvenancePrefix)]
    pub fn set_provenance_prefix(&mut self, prefix: Option<String>) {
        self.provenance_prefix = prefix;
    }

    /// The provenance namespace IRI (raw operations only; with the prefix).
    #[wasm_bindgen(js_name = setProvenanceIri)]
    pub fn set_provenance_iri(&mut self, iri: Option<String>) {
        self.provenance_iri = iri;
    }

    /// The statistical-aggregate namespace (governed operations only).
    #[wasm_bindgen(js_name = setAggregateNamespace)]
    pub fn set_aggregate_namespace(&mut self, namespace: Option<String>) {
        self.aggregate_namespace = namespace;
    }

    /// The entailment regime (the entailment operation only, where it is required).
    #[wasm_bindgen(js_name = setRegime)]
    pub fn set_regime(&mut self, regime: Option<String>) {
        self.regime = regime;
    }

    /// The RIF program text (the entailment operation only).
    #[wasm_bindgen(js_name = setProgram)]
    pub fn set_program(&mut self, program: Option<String>) {
        self.program = program;
    }

    /// The client's `Accept` header (the negotiated operation only; absent means the
    /// protocol's defaults).
    #[wasm_bindgen(js_name = setAccept)]
    pub fn set_accept(&mut self, accept: Option<String>) {
        self.accept = accept;
    }

    /// The fuel ceiling (governed operations only).
    #[wasm_bindgen(js_name = setFuel)]
    pub fn set_fuel(&mut self, value: Option<i64>) {
        self.fuel = value;
    }

    /// The wall deadline in milliseconds (governed operations only).
    #[wasm_bindgen(js_name = setDeadlineMs)]
    pub fn set_deadline_ms(&mut self, value: Option<i64>) {
        self.deadline_ms = value;
    }

    /// The answer ceiling (governed queries only).
    #[wasm_bindgen(js_name = setMaxAnswers)]
    pub fn set_max_answers(&mut self, value: Option<i64>) {
        self.max_answers = value;
    }

    /// The intermediate-cell ceiling (governed operations only).
    #[wasm_bindgen(js_name = setMaxIntermediateCells)]
    pub fn set_max_intermediate_cells(&mut self, value: Option<i64>) {
        self.max_intermediate_cells = value;
    }

    /// The scratch-byte ceiling (governed operations only).
    #[wasm_bindgen(js_name = setMaxScratchBytes)]
    pub fn set_max_scratch_bytes(&mut self, value: Option<i64>) {
        self.max_scratch_bytes = value;
    }

    /// The remote-request ceiling (governed operations only).
    #[wasm_bindgen(js_name = setMaxRemoteRequests)]
    pub fn set_max_remote_requests(&mut self, value: Option<i64>) {
        self.max_remote_requests = value;
    }

    /// Polls between yields: an integer from 0 (yield at every poll) to 4 294 967 295.
    /// Default 65 536.
    #[wasm_bindgen(js_name = setYieldEveryPolls)]
    pub fn set_yield_every_polls(&mut self, value: Option<f64>) {
        self.yield_every_polls = value;
    }

    /// The job's stack region in bytes: an integer of at least 524 288 (512 KiB).
    /// Default 2 MiB.
    #[wasm_bindgen(js_name = setStackBytes)]
    pub fn set_stack_bytes(&mut self, value: Option<f64>) {
        self.stack_bytes = value;
    }

    /// The per-service policy host-resolved `SERVICE` requests are authorized against
    /// (copied, so the catalog stays usable for later jobs). Requires the `SERVICE`
    /// handler.
    #[wasm_bindgen(js_name = setCatalog)]
    pub fn set_catalog(&mut self, catalog: &ServiceCatalog) {
        self.catalog = Some(catalog.inner.clone());
    }

    /// Serve `endpoint` in process from a snapshot of `dataset`, with no host call.
    #[wasm_bindgen(js_name = addLocalService)]
    pub fn add_local_service(
        &mut self,
        endpoint: String,
        dataset: &Dataset,
    ) -> Result<(), JsError> {
        let frozen = dataset.view().freeze().map_err(|e| diag_to_err(&e))?;
        self.add_local_frozen(endpoint, frozen)
            .map_err(|message| js_error(&message))
    }

    /// Which host handlers exist: the package root passes `typeof resolveService ===
    /// "function"` and `typeof resolveLoad === "function"`. An effect with no handler
    /// fails exactly as the synchronous lane's missing source does.
    #[wasm_bindgen(js_name = setHandlers)]
    pub fn set_handlers(&mut self, service: bool, load: bool) {
        self.service_handler = service;
        self.load_handler = load;
    }
}

impl AsyncJobOptions {
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

    /// The name of the first governor ceiling set, in the synchronous entries' order.
    fn first_ceiling_set(&self) -> Option<&'static str> {
        [
            ("fuel", self.fuel.is_some()),
            ("deadlineMs", self.deadline_ms.is_some()),
            ("maxAnswers", self.max_answers.is_some()),
            (
                "maxIntermediateCells",
                self.max_intermediate_cells.is_some(),
            ),
            ("maxScratchBytes", self.max_scratch_bytes.is_some()),
            ("maxRemoteRequests", self.max_remote_requests.is_some()),
        ]
        .into_iter()
        .find_map(|(name, set)| set.then_some(name))
    }

    /// Check these options for `kind`. Every option a kind would ignore is refused by
    /// name rather than dropped: an option a caller believes applies and that nothing
    /// enforces is the silent hole this surface exists to close.
    fn validate(&self, kind: AsyncOperationKind) -> Result<ValidatedOptions, String> {
        let op = kind.name();
        if !kind.is_governed() {
            if let Some(name) = self.first_ceiling_set() {
                return Err(format!(
                    "{name} is an execution governor, enforced only by the governed \
                     operations; a {op} operation would ignore it entirely"
                ));
            }
            if self.aggregate_namespace.is_some() {
                return Err(format!(
                    "aggregateNamespace is honored only by the governed operations; a {op} \
                     operation would ignore it entirely"
                ));
            }
        }
        if kind == AsyncOperationKind::UpdateGoverned && self.max_answers.is_some() {
            return Err(UPDATE_REFUSES_MAX_ANSWERS.to_owned());
        }
        if kind == AsyncOperationKind::Shacl && self.base.is_some() {
            return Err(
                "base is a SPARQL operation's base IRI; a shacl operation resolves its shapes \
                 graph against shapesBase and would ignore it"
                    .to_owned(),
            );
        }
        let ceilings = GovernorArgs::decode(
            self.fuel,
            self.deadline_ms,
            self.max_answers,
            self.max_intermediate_cells,
            self.max_scratch_bytes,
            self.max_remote_requests,
        )?;
        match kind {
            AsyncOperationKind::Raw => {
                if self.options_json.is_some() && self.format.is_none() {
                    return Err(
                        "optionsJson needs a format: a configured serialization names the \
                         JSON-LD or YAML-LD format it configures"
                            .to_owned(),
                    );
                }
                if self.options_json.is_some()
                    && (self.provenance_prefix.is_some() || self.provenance_iri.is_some())
                {
                    return Err(
                        "provenanceNamespace applies to SPARQL results documents, not to a \
                         configured JSON-LD graph serialization"
                            .to_owned(),
                    );
                }
            }
            AsyncOperationKind::RawWithContext => {
                if self.format.is_none() {
                    return Err(
                        "a rawWithContext operation needs a format to serialize under the context"
                            .to_owned(),
                    );
                }
                if self.options_json.is_some() {
                    return Err(
                        "optionsJson is not accepted beside a compiled context: the context \
                         is the configuration"
                            .to_owned(),
                    );
                }
            }
            _ => {
                if self.format.is_some() {
                    return Err(format!(
                        "format applies only to raw operations; a {op} operation returns a \
                         typed result and would ignore it"
                    ));
                }
                if self.options_json.is_some() {
                    return Err(format!(
                        "optionsJson applies only to raw operations; a {op} operation would \
                         ignore it"
                    ));
                }
            }
        }
        if kind != AsyncOperationKind::Raw
            && (self.provenance_prefix.is_some() || self.provenance_iri.is_some())
        {
            return Err(format!(
                "provenanceNamespace applies only to raw operations; a {op} operation would \
                 ignore it"
            ));
        }
        if kind != AsyncOperationKind::Negotiated && self.accept.is_some() {
            return Err(format!(
                "accept applies only to the negotiated operation; a {op} operation would \
                 ignore it"
            ));
        }
        if kind == AsyncOperationKind::EntailmentGoverned {
            if self.regime.is_none() {
                return Err("an entailmentGoverned operation needs a regime".to_owned());
            }
        } else if self.regime.is_some() || self.program.is_some() {
            return Err(format!(
                "regime and program apply only to the entailmentGoverned operation; a {op} \
                 operation would ignore them"
            ));
        }
        if self.catalog.is_some() && !self.service_handler {
            return Err(
                "a service catalog governs host-resolved SERVICE requests, and no \
                 resolveService handler was supplied; it would govern nothing"
                    .to_owned(),
            );
        }
        let quantum = count_option(
            "yieldEveryPolls",
            self.yield_every_polls,
            0,
            DEFAULT_YIELD_EVERY_POLLS,
        )?;
        let stack_bytes = count_option(
            "stackBytes",
            self.stack_bytes,
            MIN_STACK_BYTES,
            DEFAULT_STACK_BYTES,
        )?;
        Ok(ValidatedOptions {
            ceilings,
            quantum,
            stack_bytes,
        })
    }
}

// ---------------------------------------------------------------------------
// The service catalog
// ---------------------------------------------------------------------------

/// The per-service policy host-resolved `SERVICE` requests are authorized against,
/// before the host is ever called: deny by default, one profile per endpoint, an optional
/// fallback. Each profile is JSON:
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
/// `capabilities` is required; a host-resolved request needs `query` and `network`, and
/// a `credential` needs `credentials` too (the native rule: a credential that may not be
/// sent refuses the request rather than sending it without). `headers` is an array of
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
        out = out.with_timeout(Duration::from_millis(ms));
    }
    Ok(out)
}

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
    pub fn add_service(&mut self, endpoint: String, profile_json: &str) -> Result<(), JsError> {
        self.add_service_message(endpoint, profile_json)
            .map_err(|message| js_error(&message))
    }

    /// Apply the profile `profileJson` to every service with no entry of its own — the
    /// explicit opt-out of deny-by-default.
    #[wasm_bindgen(js_name = setFallback)]
    pub fn set_fallback(&mut self, profile_json: &str) -> Result<(), JsError> {
        self.set_fallback_message(profile_json)
            .map_err(|message| js_error(&message))
    }

    /// Whether the profile governing `endpoint` (its own, or the fallback) carries a
    /// credential — whether a request to it is one a shared cache must never answer.
    #[wasm_bindgen(js_name = carriesCredential)]
    #[must_use]
    pub fn carries_credential(&self, endpoint: &str) -> bool {
        self.inner
            .profile_for(endpoint)
            .is_some_and(|profile| profile.credential().is_some())
    }

    /// Authorize a `LOAD` of `iri` against this catalog, the same policy a `SERVICE`
    /// request meets: a document fetch needs the `network` capability, and a credential
    /// needs `credentials` too. The answer carries either the denial or everything the
    /// fetch must send — the profile's headers and credential, its user agent and its
    /// timeout, and an `Accept` header naming every RDF syntax a `LOAD` can parse.
    #[wasm_bindgen(js_name = authorizeLoad)]
    #[must_use]
    pub fn authorize_load(&self, iri: &str) -> LoadAuthorization {
        load_authorization(&self.inner, iri)
    }
}

/// What [`ServiceCatalog::authorize_load`] decided for one `LOAD` IRI.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct LoadAuthorization {
    denial: Option<String>,
    headers: Vec<(String, String)>,
    user_agent: Option<String>,
    timeout_ms: Option<f64>,
}

#[wasm_bindgen]
impl LoadAuthorization {
    /// Why the catalog refuses the `LOAD`, or `undefined` when it allows it.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn denial(&self) -> Option<String> {
        self.denial.clone()
    }

    /// The request headers as flattened `[name, value, name, value, …]` pairs — the
    /// profile's headers, then its credential header — in the order they must be sent.
    /// Empty on a denial.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn headers(&self) -> Vec<String> {
        self.headers
            .iter()
            .flat_map(|(name, value)| [name.clone(), value.clone()])
            .collect()
    }

    /// The `Accept` header a `LOAD` fetch sends: the media type of every RDF syntax the
    /// engine parses, in its registry order.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn accept(&self) -> String {
        load_accept()
    }

    /// The profile's `User-Agent`, when it names one.
    #[wasm_bindgen(getter, js_name = userAgent)]
    #[must_use]
    pub fn user_agent(&self) -> Option<String> {
        self.user_agent.clone()
    }

    /// The profile's per-request timeout in milliseconds, when it sets one.
    #[wasm_bindgen(getter, js_name = timeoutMs)]
    #[must_use]
    pub fn timeout_ms(&self) -> Option<f64> {
        self.timeout_ms
    }
}

/// The `Accept` header of a `LOAD` fetch: every parseable RDF syntax's media type.
fn load_accept() -> String {
    purrdf::NativeRdfFormat::all()
        .map(purrdf::NativeRdfFormat::media_type)
        .collect::<Vec<_>>()
        .join(", ")
}

/// [`ServiceCatalog::authorize_load`], on the native catalog.
fn load_authorization(catalog: &NativeServiceCatalog, iri: &str) -> LoadAuthorization {
    match catalog.authorize(
        iri,
        ServiceCapabilities::granting([ServiceCapability::Network]),
    ) {
        Err(denial) => LoadAuthorization {
            denial: Some(format!("LOAD <{iri}>: {denial}")),
            headers: Vec::new(),
            user_agent: None,
            timeout_ms: None,
        },
        Ok(profile) => LoadAuthorization {
            denial: None,
            headers: profile.request_headers(),
            user_agent: profile.user_agent().map(str::to_owned),
            timeout_ms: profile
                .timeout()
                .map(|timeout| timeout.as_secs_f64() * 1000.0),
        },
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

/// What `beginAsync` decoded at the boundary, beside the validated options.
struct Decoded {
    validated: ValidatedOptions,
    jsonld: Option<JsonLdSerializeOptions>,
    provenance: Option<ProvenanceNamespace>,
}

/// Freeze `dataset`, build the operation and register the job.
fn begin_job(
    engine: &Rc<NativeSparqlEngine>,
    dataset: &Dataset,
    kind: AsyncOperationKind,
    sparql: String,
    options: &AsyncJobOptions,
    decoded: Decoded,
) -> Result<AsyncJob, String> {
    let Decoded {
        validated,
        jsonld,
        provenance,
    } = decoded;
    let freeze_started = now_ms();
    let frozen = dataset
        .view()
        .freeze()
        .map_err(|diagnostic| diagnostic.to_string())?;
    let freeze_ms = now_ms() - freeze_started;
    let input = OperationInput {
        kind,
        engine: Rc::clone(engine),
        frozen,
        sparql,
        base: options.base.clone(),
        aggregate_namespace: options.aggregate_namespace.clone(),
        ceilings: validated.ceilings,
        format: options.format.clone(),
        provenance,
        jsonld,
        regime: options.regime.clone(),
        program: options.program.clone(),
        accept: options.accept.clone(),
    };
    Ok(register_operation(
        kind,
        input.into_operation(),
        validated,
        options,
        freeze_ms,
        (dataset.identity(), dataset.current_generation()),
    ))
}

/// Register a job of `kind` that runs `operation` under `validated` and `options`.
/// `dataset` is the identity and generation an update's commit is checked against
/// (`(0, 0)` for an operation that reads no dataset).
fn register_operation(
    kind: AsyncOperationKind,
    operation: Operation,
    validated: ValidatedOptions,
    options: &AsyncJobOptions,
    freeze_ms: f64,
    dataset: (u64, u64),
) -> AsyncJob {
    let region = StackRegion::new(validated.stack_bytes as usize);
    let inner = register_job(|id| {
        let slots = Arc::new(JobSlots::new(id, region.bounds));
        add_ms(&slots.counters.freeze_ms, freeze_ms);
        let watch = Arc::new(JspiStopWatch::new(
            slots,
            validated.ceilings.deadline_ms(),
            validated.quantum,
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

/// Validate `options` for a SHACL job, match the arguments to `operation`, and register
/// the job. Native-testable core of [`AsyncJob::begin_shacl`].
fn begin_shacl_job(
    operation: ShaclAsyncOperation,
    arguments: ShaclArguments,
    options: &AsyncJobOptions,
) -> Result<AsyncJob, String> {
    let validated = options.validate(AsyncOperationKind::Shacl)?;
    let request = ShaclRequest::build(operation, arguments)?;
    Ok(register_operation(
        AsyncOperationKind::Shacl,
        Box::new(move |run| execute_shacl(request, run)),
        validated,
        options,
        0.0,
        (0, 0),
    ))
}

#[wasm_bindgen]
impl AsyncJob {
    /// Start an asynchronous SHACL operation: the twin of the synchronous entry
    /// `operation` names, over the same arguments.
    ///
    /// `shapesTtl`, `shapesBase`, `addedNt` and `removedNt` are the text-shapes entries'
    /// arguments and `product` and `expectIdentity` the product entries'; an argument the
    /// operation does not take is refused by name, as is an option it would ignore. The
    /// documents are parsed when the job runs, so a parse error — like a product refusal
    /// — is the job's error, with the synchronous twin's words.
    ///
    /// A static constructor rather than a free function: it is the package root's
    /// plumbing, reached through its `shacl…Async` twins, never a consumer entry point.
    ///
    /// # Errors
    ///
    /// An option the operation would ignore or cannot honor, or an argument it does not
    /// take or is missing.
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
    ) -> Result<Self, JsError> {
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
        .map_err(|message| js_error(&message))
    }
}

#[wasm_bindgen]
impl QueryEngine {
    /// Start an asynchronous operation of `kind` over a snapshot of `dataset`.
    ///
    /// The dataset is frozen now: a query sees this snapshot whatever happens to the
    /// dataset afterwards, and an update captures its identity and generation so
    /// `commitUpdate` can refuse to overwrite a mutation made while it ran. The SPARQL
    /// text is parsed when the job runs, so a parse error is the job's error, with the
    /// synchronous twin's words. A `rawWithContext` operation starts through
    /// `beginAsyncWithContext` instead.
    ///
    /// # Errors
    ///
    /// An option the operation would ignore or cannot honor, a malformed ceiling, a
    /// malformed JSON-LD options document or provenance namespace, or a dataset that
    /// cannot be frozen.
    #[wasm_bindgen(js_name = beginAsync)]
    pub fn begin_async(
        &self,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: String,
        options: &AsyncJobOptions,
    ) -> Result<AsyncJob, JsError> {
        if kind == AsyncOperationKind::RawWithContext {
            return Err(js_error(
                "a rawWithContext operation starts through beginAsyncWithContext",
            ));
        }
        if kind == AsyncOperationKind::Shacl {
            return Err(js_error(SHACL_STARTS_ELSEWHERE));
        }
        let validated = options
            .validate(kind)
            .map_err(|message| js_error(&message))?;
        let jsonld = match (kind, options.options_json.as_deref()) {
            (AsyncOperationKind::Raw, Some(json)) => Some(decode_options(json)?),
            _ => None,
        };
        let provenance = build_provenance_namespace(
            options.provenance_prefix.clone(),
            options.provenance_iri.clone(),
        )?;
        begin_job(
            self.engine(),
            dataset,
            kind,
            sparql,
            options,
            Decoded {
                validated,
                jsonld,
                provenance,
            },
        )
        .map_err(|message| js_error(&message))
    }

    /// Start a `rawWithContext` operation: a CONSTRUCT/DESCRIBE serialized under a
    /// compiled JSON-LD context (and, for YAML-LD, `yamlSchemaUrl`).
    ///
    /// # Errors
    ///
    /// As [`Self::begin_async`], plus a kind other than `rawWithContext` and an invalid
    /// `yamlSchemaUrl`.
    #[wasm_bindgen(js_name = beginAsyncWithContext)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn begin_async_with_context(
        &self,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: String,
        options: &AsyncJobOptions,
        context: &CompiledJsonLdContext,
        yaml_schema_url: Option<String>,
    ) -> Result<AsyncJob, JsError> {
        if kind != AsyncOperationKind::RawWithContext {
            return Err(js_error(&format!(
                "beginAsyncWithContext starts only a rawWithContext operation, not {}",
                kind.name()
            )));
        }
        let validated = options
            .validate(kind)
            .map_err(|message| js_error(&message))?;
        let mut jsonld = context_options(context);
        if let Some(url) = yaml_schema_url {
            jsonld = jsonld
                .with_yaml_schema_url(&url)
                .map_err(|error| js_error(&error.to_string()))?;
        }
        begin_job(
            self.engine(),
            dataset,
            kind,
            sparql,
            options,
            Decoded {
                validated,
                jsonld: Some(jsonld),
                provenance: None,
            },
        )
        .map_err(|message| js_error(&message))
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::SparqlEngine as _;

    use super::*;

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

    fn options() -> AsyncJobOptions {
        AsyncJobOptions::new()
    }

    /// Validate and begin, as `beginAsync` does, without the wasm-only decoding.
    fn begin(
        engine: &QueryEngine,
        dataset: &Dataset,
        kind: AsyncOperationKind,
        sparql: &str,
        options: &AsyncJobOptions,
    ) -> AsyncJob {
        let validated = options.validate(kind).expect("options are valid");
        begin_job(
            engine.engine(),
            dataset,
            kind,
            sparql.to_owned(),
            options,
            Decoded {
                validated,
                jsonld: None,
                provenance: None,
            },
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

    // ── Options: every refusal beside the valid neighbour it must not catch ─────────

    #[test]
    fn yield_every_polls_refuses_a_negative_and_accepts_zero() {
        let mut refused = options();
        refused.set_yield_every_polls(Some(-1.0));
        let error = refused
            .validate(AsyncOperationKind::Query)
            .expect_err("a negative quantum is refused");
        assert!(error.contains("yieldEveryPolls"), "{error}");
        let mut fractional = options();
        fractional.set_yield_every_polls(Some(1.5));
        assert!(fractional.validate(AsyncOperationKind::Query).is_err());

        let mut zero = options();
        zero.set_yield_every_polls(Some(0.0));
        let validated = zero
            .validate(AsyncOperationKind::Query)
            .expect("zero yields at every poll");
        assert_eq!(validated.quantum, 0);
        let default = options()
            .validate(AsyncOperationKind::Query)
            .expect("defaults are valid");
        assert_eq!(default.quantum, DEFAULT_YIELD_EVERY_POLLS);
    }

    #[test]
    fn stack_bytes_refuses_a_region_below_the_minimum_and_accepts_the_minimum() {
        let mut refused = options();
        refused.set_stack_bytes(Some(4096.0));
        let error = refused
            .validate(AsyncOperationKind::Query)
            .expect_err("a 4 KiB region is refused");
        assert!(
            error.contains("stackBytes") && error.contains("524288"),
            "{error}"
        );

        let mut minimum = options();
        minimum.set_stack_bytes(Some(524_288.0));
        let validated = minimum
            .validate(AsyncOperationKind::Query)
            .expect("the minimum is accepted");
        assert_eq!(validated.stack_bytes, 524_288);
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
        let mut with_fuel = options();
        with_fuel.set_fuel(Some(10));
        let error = with_fuel
            .validate(AsyncOperationKind::Query)
            .expect_err("an ungoverned query would ignore fuel");
        assert!(error.contains("fuel") && error.contains("query"), "{error}");
        let governed = with_fuel
            .validate(AsyncOperationKind::Governed)
            .expect("a governed query enforces fuel");
        assert!(format!("{:?}", governed.ceilings).contains("fuel: Some(10)"));

        let mut with_namespace = options();
        with_namespace.set_aggregate_namespace(Some("http://example.org/agg#".to_owned()));
        assert!(
            with_namespace
                .validate(AsyncOperationKind::Raw)
                .expect_err("an ungoverned raw query would ignore it")
                .contains("aggregateNamespace")
        );
        assert!(
            with_namespace
                .validate(AsyncOperationKind::Governed)
                .is_ok()
        );
    }

    #[test]
    fn a_negative_ceiling_is_refused_and_zero_is_a_ceiling() {
        let mut negative = options();
        negative.set_max_answers(Some(-1));
        let error = negative
            .validate(AsyncOperationKind::Governed)
            .expect_err("negative");
        assert!(error.contains("maxAnswers"), "{error}");
        let mut zero = options();
        zero.set_max_answers(Some(0));
        assert!(zero.validate(AsyncOperationKind::Governed).is_ok());
    }

    #[test]
    fn a_governed_update_refuses_max_answers_and_a_governed_query_takes_it() {
        let mut with_answers = options();
        with_answers.set_max_answers(Some(5));
        assert_eq!(
            with_answers
                .validate(AsyncOperationKind::UpdateGoverned)
                .expect_err("an UPDATE has no answer sequence"),
            UPDATE_REFUSES_MAX_ANSWERS
        );
        assert!(with_answers.validate(AsyncOperationKind::Governed).is_ok());
        let mut with_fuel = options();
        with_fuel.set_fuel(Some(5));
        assert!(
            with_fuel
                .validate(AsyncOperationKind::UpdateGoverned)
                .is_ok()
        );
    }

    #[test]
    fn format_is_refused_where_nothing_is_serialized() {
        let mut with_format = options();
        with_format.set_format(Some("json".to_owned()));
        assert!(
            with_format
                .validate(AsyncOperationKind::Query)
                .expect_err("a typed result has no format")
                .contains("format")
        );
        assert!(with_format.validate(AsyncOperationKind::Raw).is_ok());
        assert!(
            with_format
                .validate(AsyncOperationKind::RawWithContext)
                .is_ok()
        );
        assert!(
            options()
                .validate(AsyncOperationKind::RawWithContext)
                .is_err(),
            "a context serialization needs its format"
        );
        assert!(options().validate(AsyncOperationKind::Raw).is_ok());
    }

    #[test]
    fn options_json_needs_a_format_and_a_raw_operation() {
        let mut configured = options();
        configured.set_options_json(Some("{}".to_owned()));
        assert!(configured.validate(AsyncOperationKind::Raw).is_err());
        assert!(configured.validate(AsyncOperationKind::Governed).is_err());
        configured.set_format(Some("jsonld".to_owned()));
        assert!(configured.validate(AsyncOperationKind::Raw).is_ok());
    }

    #[test]
    fn provenance_is_refused_outside_a_raw_operation() {
        let mut provenance = options();
        provenance.set_provenance_prefix(Some("prov".to_owned()));
        provenance.set_provenance_iri(Some("http://example.org/prov#".to_owned()));
        assert!(
            provenance
                .validate(AsyncOperationKind::Governed)
                .expect_err("a governed outcome has no results document")
                .contains("provenanceNamespace")
        );
        assert!(provenance.validate(AsyncOperationKind::Raw).is_ok());
    }

    #[test]
    fn an_entailment_operation_needs_a_regime_that_no_other_operation_takes() {
        assert!(
            options()
                .validate(AsyncOperationKind::EntailmentGoverned)
                .is_err()
        );
        let mut with_regime = options();
        with_regime.set_regime(Some("rdfs".to_owned()));
        assert!(
            with_regime
                .validate(AsyncOperationKind::EntailmentGoverned)
                .is_ok()
        );
        assert!(with_regime.validate(AsyncOperationKind::Governed).is_err());
    }

    #[test]
    fn a_catalog_without_a_service_handler_is_refused() {
        let mut catalog = ServiceCatalog::new();
        catalog
            .add_service_message(
                ENDPOINT.to_owned(),
                r#"{"capabilities":["query","network"]}"#,
            )
            .expect("profile parses");
        let mut orphan = options();
        orphan.set_catalog(&catalog);
        assert!(
            orphan
                .validate(AsyncOperationKind::Query)
                .expect_err("a catalog with nothing to govern")
                .contains("resolveService")
        );
        orphan.set_handlers(true, false);
        assert!(orphan.validate(AsyncOperationKind::Query).is_ok());
    }

    #[test]
    fn a_local_service_declared_twice_is_refused() {
        let frozen = seed().view().freeze().expect("freeze");
        let mut local = options();
        local
            .add_local_frozen(ENDPOINT.to_owned(), Arc::clone(&frozen))
            .expect("first declaration");
        assert!(
            local
                .add_local_frozen("http://example.org/other".to_owned(), Arc::clone(&frozen))
                .is_ok()
        );
        assert!(
            local
                .add_local_frozen(ENDPOINT.to_owned(), frozen)
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

    #[test]
    fn a_fired_signal_is_reported_before_whatever_was_delivered() {
        let slots = slots();
        let abandoned = service_answer(ENDPOINT, Some(StopCause::Deadline), None, &slots);
        assert_eq!(
            abandoned,
            Err(RemoteError::Governed(TrippedGovernor::Stopped {
                cause: StopCause::Deadline
            })),
            "an abandoned exchange keeps the positional prefix"
        );
        let completed = service_answer(
            ENDPOINT,
            Some(StopCause::Cancelled),
            Some(Delivered::Bindings(SRJ.to_vec())),
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
        let failed = service_answer(
            ENDPOINT,
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
            service_answer(
                ENDPOINT,
                None,
                Some(Delivered::Bindings(SRJ.to_vec())),
                &slots
            ),
            Ok(SRJ.to_vec())
        );
        assert_eq!(
            service_answer(
                ENDPOINT,
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
        let denied = service_answer(
            ENDPOINT,
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
        let result = service_answer(ENDPOINT, None, None, &slots);
        assert!(
            matches!(result, Err(RemoteError::Governed(_))),
            "{result:?}"
        );
        let fault = slots.fault().expect("a fault is latched");
        assert!(
            fault.contains("resolver returned without a delivery"),
            "{fault}"
        );
    }

    #[test]
    fn load_deliveries_map_onto_the_load_seam() {
        let slots = slots();
        let graph = parse_document(REMOTE_NT.as_bytes(), "text/turtle", None).expect("parses");
        assert!(
            load_answer(
                "http://example.org/doc",
                None,
                Some(Delivered::Graph(graph)),
                &slots
            )
            .is_ok()
        );
        let failed = load_answer(
            "http://example.org/doc",
            None,
            Some(Delivered::Failure {
                kind: FailureKind::Transport,
                message: "HTTP 404".to_owned(),
            }),
            &slots,
        )
        .expect_err("a failure");
        assert_eq!(failed.code, "native-sparql-load-failed");
        assert!(failed.message.contains("HTTP 404"), "{}", failed.message);
        let denied = load_answer(
            "http://example.org/doc",
            None,
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                message: "policy".to_owned(),
            }),
            &slots,
        )
        .expect_err("a denial");
        assert_eq!(denied.code, purrdf_sparql_eval::LOAD_DENIED);
        assert_eq!(
            denied.message,
            "LOAD <http://example.org/doc>: the host denied the request: policy"
        );
        assert!(
            !denied.message.contains("withholds the"),
            "a LOAD denial reports the host's own decision, never an invented catalog cause: \
             {}",
            denied.message
        );
        assert!(slots.fault().is_none(), "failures are answers, not faults");
        let missing =
            load_answer("http://example.org/doc", None, None, &slots).expect_err("no delivery");
        assert_eq!(missing.code, "native-sparql-load-fault");
        assert!(slots.fault().is_some(), "a missing delivery is a fault");
    }

    // ── The ticket exchange ─────────────────────────────────────────────────────────

    #[test]
    fn a_delivery_must_name_the_outstanding_effect() {
        let slots = slots();
        let seq = slots
            .issue(EffectPayload::Load {
                iri: "http://example.org/doc".to_owned(),
            })
            .expect("issued");
        let effect = slots.take_effect().expect("posted");
        assert_eq!(effect.seq(), seq);
        assert_eq!(effect.kind(), AsyncEffectKind::Load);
        assert_eq!(effect.iri().as_deref(), Some("http://example.org/doc"));
        assert!(slots.take_effect().is_none(), "an effect is taken once");

        // Bindings cannot answer a LOAD: a fault, and the effect stays outstanding.
        assert_eq!(
            slots.deliver(seq, Delivered::Bindings(SRJ.to_vec())),
            DELIVERY_FAULT
        );
        assert!(slots.fault().expect("latched").contains("Load"));
    }

    #[test]
    fn stale_future_and_late_deliveries_are_told_apart() {
        let slots = slots();
        let first = slots
            .issue(EffectPayload::Service(Box::new(service_effect())))
            .expect("issued");
        assert_eq!(
            slots.deliver(first, Delivered::Bindings(SRJ.to_vec())),
            DELIVERY_ACCEPTED
        );
        assert!(matches!(slots.resume(first), Some(Delivered::Bindings(_))));
        let second = slots
            .issue(EffectPayload::Service(Box::new(service_effect())))
            .expect("issued");
        // A promise settling for the first effect after it was answered: stale, no fault.
        assert_eq!(
            slots.deliver(first, Delivered::Bindings(SRJ.to_vec())),
            DELIVERY_STALE
        );
        assert!(slots.fault().is_none());
        // A sequence number never issued is a bridge bug.
        assert_eq!(
            slots.deliver(second + 5, Delivered::Bindings(SRJ.to_vec())),
            DELIVERY_FAULT
        );
        let fault = slots.fault().expect("latched");
        assert!(
            fault.contains(&(second + 5).to_string()) && fault.contains(&second.to_string()),
            "the fault names both sequence numbers: {fault}"
        );
        slots.finished.store(true, Ordering::Relaxed);
        assert_eq!(
            slots.deliver(second, Delivered::Bindings(SRJ.to_vec())),
            DELIVERY_FINISHED
        );
    }

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
        }
    }

    #[test]
    fn a_service_effect_exposes_its_request_and_hides_its_secret_from_logs() {
        let effect = AsyncEffect {
            seq: 7,
            payload: EffectPayload::Service(Box::new(service_effect())),
        };
        assert_eq!(effect.kind(), AsyncEffectKind::Service);
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
        assert_eq!(effect.max_intermediate_cells(), Some(10));
        assert_eq!(effect.headers(), vec!["Authorization", "Bearer secret"]);
        assert!(effect.iri().is_none());
        let logged = format!("{effect:?}");
        assert!(!logged.contains("secret"), "{logged}");
    }

    #[test]
    fn an_unknown_failure_kind_is_a_fault_and_denied_is_an_answer() {
        let engine = QueryEngine::new();
        let dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            &options(),
        );
        let slots = &job.inner.watch.slots;
        let seq = slots
            .issue(EffectPayload::Service(Box::new(service_effect())))
            .expect("issued");
        assert_eq!(
            job.deliver_failure(seq, "nope", "x".to_owned()),
            DELIVERY_FAULT
        );
        assert!(slots.fault().expect("latched").contains("nope"));

        let neighbour = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            &options(),
        );
        let slots = &neighbour.inner.watch.slots;
        let seq = slots
            .issue(EffectPayload::Service(Box::new(service_effect())))
            .expect("issued");
        assert_eq!(
            neighbour.deliver_failure(seq, "denied", "policy".to_owned()),
            DELIVERY_ACCEPTED
        );
        assert!(slots.fault().is_none());
        assert!(matches!(
            slots.resume(seq),
            Some(Delivered::Failure {
                kind: FailureKind::Denied,
                ..
            })
        ));
    }

    #[test]
    fn an_unparseable_load_document_is_a_load_failure_not_a_fault() {
        let engine = QueryEngine::new();
        let dataset = seed();
        for (media_type, parses) in [("text/x-unknown", false), ("text/turtle", true)] {
            let job = begin(
                &engine,
                &dataset,
                AsyncOperationKind::Update,
                "CLEAR ALL",
                &options(),
            );
            let slots = &job.inner.watch.slots;
            let seq = slots
                .issue(EffectPayload::Load {
                    iri: "http://example.org/doc".to_owned(),
                })
                .expect("issued");
            assert_eq!(
                job.deliver_graph(seq, REMOTE_NT.as_bytes(), media_type, None),
                DELIVERY_ACCEPTED
            );
            assert!(slots.fault().is_none(), "{media_type}");
            match (slots.resume(seq), parses) {
                (Some(Delivered::Graph(graph)), true) => assert_eq!(graph.quads().count(), 1),
                (Some(Delivered::Failure { message, .. }), false) => {
                    assert!(message.contains("text/x-unknown"), "{message}");
                }
                (other, _) => panic!("{media_type}: unexpected {other:?}"),
            }
        }
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
    fn cancellation_faults_and_the_host_deadline_stop_the_watch_and_stay_stopped() {
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
            &options(),
        );
        let seq = job
            .inner
            .watch
            .slots
            .issue(EffectPayload::Service(Box::new(service_effect())))
            .expect("issued");
        assert_eq!(job.deliver_governed(seq), DELIVERY_ACCEPTED);
        assert_eq!(job.inner.watch.observe_now(), Some(StopCause::Cancelled));

        let timed_out = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Query,
            "ASK {}",
            &options(),
        );
        let seq = timed_out
            .inner
            .watch
            .slots
            .issue(EffectPayload::Service(Box::new(service_effect())))
            .expect("issued");
        assert_eq!(timed_out.trip_deadline(), DELIVERY_ACCEPTED);
        assert_eq!(timed_out.deliver_governed(seq), DELIVERY_ACCEPTED);
        assert_eq!(
            timed_out.inner.watch.observe_now(),
            Some(StopCause::Deadline),
            "the host's deadline timer latched first, so the trip is a deadline"
        );
    }

    #[test]
    fn the_stack_guard_stops_a_frame_in_the_guard_band() {
        let bounds = StackBounds {
            base: 0x10_0000,
            top: 0x18_0000,
        };
        // What `purrdf_stack::remaining` reports on the region, whose base
        // `JobInner::run` installs as the floor.
        let check = |sp: usize| bounds.check(sp, sp.saturating_sub(bounds.base));
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
                .expect_err("an outcome is taken once"),
            "takeRawBytes: the outcome was already taken"
        );
        assert_eq!(run_job(job.id()), RUN_ALREADY_STARTED);
        assert_eq!(job.finish(), 0);
        assert_eq!(run_job(job.id()), RUN_UNKNOWN_JOB);
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_ERROR);
        assert_eq!(job.error_kind().as_deref(), Some("error"));
        let frozen = dataset.view().freeze().expect("freeze");
        let expected = NativeSparqlEngine::new()
            .query(&frozen, sparql_request("SELEC", None))
            .expect_err("the sync twin refuses it too")
            .to_string();
        assert_eq!(job.take_error().as_deref(), Some(expected.as_str()));
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_ERROR);
        let error = job.take_error().expect("an error");
        assert!(
            error.contains(&format!(
                "no remote query source configured for SERVICE <{ENDPOINT}>"
            )),
            "{error}"
        );
        job.finish();

        let silent = query.replace("SERVICE <", "SERVICE SILENT <");
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Raw,
            &silent,
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        let expected = engine
            .query_raw(&dataset, &silent, None, None, None, None)
            .expect("sync twin");
        assert_eq!(
            raw_text(&job),
            expected,
            "SILENT is the join identity on both lanes"
        );
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
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
        assert_eq!(run_job(job.id()), RUN_ERROR);
        assert!(
            job.take_error()
                .expect("an error")
                .contains("no remote query source configured")
        );
        job.finish();
        let silent = unlisted.replace("SERVICE <", "SERVICE SILENT <");
        let job = begin(&engine, &dataset, AsyncOperationKind::Raw, &silent, &local);
        assert_eq!(run_job(job.id()), RUN_OUTCOME, "silenceable, as offline");
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
            &options(),
        );
        assert_eq!(job.cancel(), DELIVERY_ACCEPTED);
        assert_eq!(run_job(job.id()), RUN_ERROR);
        assert_eq!(job.error_kind().as_deref(), Some("cancelled"));
        assert_eq!(job.cancel(), DELIVERY_FINISHED);
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
                .expect_err("a take of the wrong kind is refused"),
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
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
            &options(),
        );
        assert_eq!(job.cancel(), DELIVERY_ACCEPTED);
        assert_eq!(run_job(job.id()), RUN_ERROR);
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
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        let mut outcome = job.take_query_outcome().expect("an outcome");
        assert!(!outcome.is_complete());
        assert!(outcome.take_tripped().is_some());
        assert_eq!(
            job.inner
                .take_outcome("takeRawBytes", &[AsyncOperationKind::Raw])
                .expect_err("a take of the wrong kind is refused"),
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
            &options(),
        );
        assert_eq!(job.fault("host bug".to_owned()), DELIVERY_ACCEPTED);
        assert_eq!(run_job(job.id()), RUN_ERROR);
        assert_eq!(job.error_kind().as_deref(), Some("fault"));
        let before = dataset.current_generation();
        assert_eq!(
            job.inner
                .commit_into(&mut dataset)
                .expect_err("nothing to commit"),
            "host bug"
        );
        assert_eq!(dataset.current_generation(), before);
        assert_eq!(dataset.size(), 2);
        job.finish();
    }

    const INSERT: &str =
        "INSERT DATA { <http://example.org/n> <http://example.org/p> <http://example.org/o> }";

    #[test]
    fn an_update_commits_into_the_dataset_it_read() {
        let engine = QueryEngine::new();
        let mut dataset = seed();
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Update,
            INSERT,
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        assert_eq!(dataset.size(), 2, "nothing lands before the commit");
        job.inner.commit_into(&mut dataset).expect("commits");
        assert_eq!(dataset.size(), 3);
        assert_eq!(dataset.current_generation(), 1);
        assert_eq!(
            job.inner
                .commit_into(&mut dataset)
                .expect_err("a second commit"),
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        engine
            .update(
                &mut dataset,
                "INSERT DATA { <http://example.org/m> <http://example.org/p> <http://example.org/o> }",
                None,
            )
            .expect("a synchronous mutation meanwhile");
        let error = job.inner.commit_into(&mut dataset).expect_err("stale");
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
            &options(),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        let error = job
            .inner
            .commit_into(&mut other)
            .expect_err("wrong dataset");
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
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        let outcome = job.take_update_outcome().expect("an outcome");
        assert!(!outcome.is_applied());
        assert!(
            job.inner
                .commit_into(&mut dataset)
                .expect_err("not applied")
                .starts_with("nothing to commit")
        );
        assert_eq!(dataset.size(), 2);
        job.finish();

        let applied = begin(
            &engine,
            &dataset,
            AsyncOperationKind::UpdateGoverned,
            INSERT,
            &options(),
        );
        assert_eq!(run_job(applied.id()), RUN_OUTCOME);
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

    fn negotiated(accept: Option<&str>) -> AsyncJobOptions {
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
            &negotiated(None),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
            &negotiated(None),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
            &negotiated(Some("text/turtle")),
        );
        assert_eq!(run_job(job.id()), RUN_ERROR);
        assert_eq!(job.error_kind().as_deref(), Some("not-acceptable"));
        let error = job.take_error().expect("an error");
        assert!(error.contains("application/trig, application/n-quads, application/ld+json"));
        job.finish();

        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            "CONSTRUCT WHERE { ?s ?p ?o }",
            &negotiated(Some("text/turtle")),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
        let outcome = job.take_negotiated_outcome().expect("an outcome");
        assert_eq!(outcome.format().as_deref(), Some("turtle"));
        job.finish();

        // A client that prefers Turtle but accepts N-Quads is sent N-Quads for the dataset.
        let job = begin(
            &engine,
            &dataset,
            AsyncOperationKind::Negotiated,
            GRAPH_CONSTRUCT,
            &negotiated(Some("text/turtle, application/n-quads;q=0.5")),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
            &negotiated(Some("text/csv")),
        );
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
        assert_eq!(run_job(job.id()), RUN_OUTCOME);
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
        let with_accept = negotiated(Some("text/csv"));
        let error = with_accept
            .validate(AsyncOperationKind::Governed)
            .expect_err("a governed query would ignore accept");
        assert!(
            error.contains("accept applies only to the negotiated operation"),
            "{error}"
        );
        assert!(with_accept.validate(AsyncOperationKind::Negotiated).is_ok());
        let mut with_format = options();
        with_format.set_format(Some("json".to_owned()));
        assert!(
            with_format
                .validate(AsyncOperationKind::Negotiated)
                .is_err()
        );
        assert!(with_format.validate(AsyncOperationKind::Raw).is_ok());
    }

    #[test]
    fn load_authorization_is_the_catalogs_policy() {
        let mut catalog = ServiceCatalog::new();
        catalog
            .add_service_message(
                "http://example.org/doc".to_owned(),
                r#"{"capabilities":["network"],"headers":[["X-A","1"]],"userAgent":"w/1","timeoutMs":250}"#,
            )
            .expect("profile");
        catalog
            .add_service_message(
                "http://example.org/query-only".to_owned(),
                r#"{"capabilities":["query"]}"#,
            )
            .expect("profile");
        let allowed = catalog.authorize_load("http://example.org/doc");
        assert_eq!(allowed.denial(), None);
        assert_eq!(allowed.headers(), ["X-A", "1"]);
        assert_eq!(allowed.user_agent().as_deref(), Some("w/1"));
        assert_eq!(allowed.timeout_ms(), Some(250.0));
        assert!(allowed.accept().contains("text/turtle"));
        let withheld = catalog.authorize_load("http://example.org/query-only");
        assert!(withheld.denial().expect("denied").contains("network"));
        assert!(
            catalog
                .authorize_load("http://example.org/unlisted")
                .denial()
                .is_some()
        );
        assert!(!catalog.carries_credential("http://example.org/doc"));
        catalog
            .add_service_message(
                "http://example.org/secret".to_owned(),
                r#"{"capabilities":["network","credentials"],"credential":{"header":"X-Key","value":"k"}}"#,
            )
            .expect("profile");
        assert!(catalog.carries_credential("http://example.org/secret"));
        assert_eq!(
            catalog
                .authorize_load("http://example.org/secret")
                .headers(),
            ["X-Key", "k"]
        );
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
        options: &AsyncJobOptions,
    ) -> (AsyncJob, u32) {
        let job = begin_shacl_job(operation, arguments, options).expect("the job begins");
        let status = run_job(job.id());
        (job, status)
    }

    #[test]
    fn a_shacl_job_answers_what_each_synchronous_entry_answers() {
        let shapes = core_shapes();
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ValidateToSarif,
            shacl_arguments(Some(shapes.clone()), PEOPLE_NT),
            &options(),
        );
        assert_eq!(status, RUN_OUTCOME);
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
            &options(),
        );
        assert_eq!(status, RUN_OUTCOME);
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
            &options(),
        );
        assert_eq!(status, RUN_OUTCOME);
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
                &options(),
            );
            assert_eq!(status, RUN_OUTCOME, "{operation:?}");
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
            &options(),
        );
        assert_eq!(
            status, RUN_ERROR,
            "a job with no source refuses it the same way"
        );
        assert!(
            job.take_error()
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
            assert_eq!(status, RUN_OUTCOME);
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
                &options(),
            )
            .expect("begins");
            assert_eq!(job.cancel(), DELIVERY_ACCEPTED);
            let status = run_job(job.id());
            (job, status)
        };
        assert_eq!(status, RUN_ERROR);
        assert_eq!(job.error_kind().as_deref(), Some("cancelled"));
        job.finish();
        let (job, status) = run_shacl(
            ShaclAsyncOperation::ValidateToSarif,
            shacl_arguments(Some(core_shapes()), PEOPLE_NT),
            &options(),
        );
        assert_eq!(status, RUN_OUTCOME);
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
            &options(),
        );
        assert_eq!(status, RUN_ERROR);
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
            &options(),
        );
        assert_eq!(status, RUN_ERROR);
        let refusal = job.take_shacl_refusal().expect("the refusal is kept");
        assert_eq!(refusal.dimension(), None, "no product was inspected");
        job.finish();

        let (job, status) = run_shacl(
            ShaclAsyncOperation::ProductValidateToSarif,
            ShaclArguments {
                product: Some(product),
                ..shacl_arguments(None, PEOPLE_NT)
            },
            &options(),
        );
        assert_eq!(status, RUN_OUTCOME);
        assert!(job.take_shacl_refusal().is_none());
        job.finish();
    }

    #[test]
    fn a_shacl_operation_refuses_what_it_would_ignore_and_takes_what_its_twin_takes() {
        let mut ceiling = options();
        ceiling.set_fuel(Some(10));
        assert!(
            ceiling
                .validate(AsyncOperationKind::Shacl)
                .expect_err("no SHACL entry takes a ceiling")
                .contains("fuel is an execution governor")
        );
        let mut based = options();
        based.set_base(Some("http://example.org/".to_owned()));
        assert!(
            based
                .validate(AsyncOperationKind::Shacl)
                .expect_err("a SPARQL base would be ignored")
                .contains("shapesBase")
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
