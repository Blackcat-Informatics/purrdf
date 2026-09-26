// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// This bench is a plain `main`, but the workspace `missing_docs` lint applies to
// its items just the same; the reporting helpers below are internal probes, not
// API.
#![allow(missing_docs)]

//! What one property-function invocation allocates, and where those allocations
//! come from.
//!
//! The seam this measures is entered **once per driving row**, so its per-invocation
//! cost is multiplied by the input size. Three of its parts are visible here:
//!
//! * the relation's own attestation of its index generation, which an index-backed
//!   producer over a frozen index knows before the query starts;
//! * the engine's reading of that attestation, which happens only on a lane whose
//!   return type can carry it;
//! * the per-relation ledger entry the reading is folded into.
//!
//! **This bench asserts nothing.** It reports allocation COUNTS and peak bytes,
//! because those are properties of the code rather than of the machine — the box
//! this runs on is not quiet, and a timing threshold here would be a flaky gate
//! rather than a measurement. Counts do not need a quiet machine: the same code
//! over the same fixture requests the same allocations on any box.
//!
//! Each phase reports its figures **per invocation**, over a fixture whose driving
//! pattern is `ROWS` rows wide, so the numbers can be read directly as "what one
//! more driving row costs" and compared across phases without dividing anything.
//!
//! The phases are chosen so the reader can attribute each figure:
//!
//! * `shared-generation, governed` — the shape every shipped index-backed producer
//!   has (`purrdf-text`'s ranked and positional relations, this crate's kNN
//!   cursor): the generation is interned once at construction as an `Arc<str>` and
//!   attesting it clones the pointer.
//! * `shared-generation, ungoverned` — the identical producer on a lane with no
//!   witness slot, where the engine asks for no generation and folds no ledger
//!   entry. The difference against the phase above is the cost of witnessing a
//!   producer that had nothing to allocate in the first place, so it is small by
//!   construction: `RelationWitness::record` allocates a key only for a relation's
//!   FIRST invocation and shares the rest.
//! * `owned-generation, ungoverned` — the same skip for the producer shape that
//!   genuinely cannot share its spelling, where every reading costs an allocation
//!   and this lane takes none. Read against `owned-generation, governed`, this is
//!   what asking a question whose answer nobody can read used to cost.
//! * `owned-generation, governed` — a producer that renders the same 64-character
//!   generation into a FRESH allocation per invocation, which is what an attestation
//!   payload that could not hold a shared pointer forced on every producer. Its
//!   difference against the first phase is the cost of that copy, reported rather
//!   than argued.
//! * `silent, governed` / `silent, ungoverned` — a relation that declares nothing,
//!   so the remaining figures are the seam's own per-invocation floor (argument
//!   buffers, row interning, the solution row) and no phase above can be mistaken
//!   for measuring that floor.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, TermValue};
use purrdf_sparql_eval::{
    EvalError, ExtensionEnv, IndexGeneration, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, QueryGovernors, QueryOptions, Volatility,
};

// ---------------------------------------------------------------------------
// The tracking allocator
// ---------------------------------------------------------------------------

static LIVE_BYTES: AtomicI64 = AtomicI64::new(0);
static PEAK_BYTES: AtomicI64 = AtomicI64::new(0);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

fn to_i64(size: usize) -> i64 {
    i64::try_from(size).unwrap_or(i64::MAX)
}

fn record_allocation(size: usize) {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    let size = to_i64(size);
    let live = LIVE_BYTES
        .fetch_add(size, Ordering::Relaxed)
        .saturating_add(size);
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, live, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => peak = observed,
        }
    }
}

fn record_deallocation(size: usize) {
    LIVE_BYTES.fetch_sub(to_i64(size), Ordering::Relaxed);
}

struct CountingAllocator;

// SAFETY: every operation delegates to `System` with the exact incoming pointer
// and layout; the atomic accounting does not affect allocator ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: delegated with the caller's exact layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: delegated with the caller's exact pointer/layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: delegated with the caller's exact pointer/layout and size.
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            record_deallocation(layout.size());
            record_allocation(new_size);
        }
        resized
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// The allocation count and peak high-water baseline to measure a phase against.
fn reset() -> (u64, i64) {
    let live = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(live, Ordering::Relaxed);
    (ALLOCATIONS.load(Ordering::Relaxed), live)
}

fn report(label: &str, baseline: (u64, i64), invocations: u64) {
    let allocations = ALLOCATIONS
        .load(Ordering::Relaxed)
        .saturating_sub(baseline.0);
    let peak = PEAK_BYTES
        .load(Ordering::Relaxed)
        .saturating_sub(baseline.1);
    let per_invocation = f64::from(u32::try_from(allocations).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(invocations).unwrap_or(1));
    println!(
        "[relation_attestation_alloc] {label}: invocations={invocations} \
         allocations={allocations} allocations_per_invocation={per_invocation:.2} \
         peak_allocated_bytes={peak}"
    );
}

// ---------------------------------------------------------------------------
// The fixture relation
// ---------------------------------------------------------------------------

/// The data namespace of the fixture terms. PurRDF mints no vocabulary; every IRI
/// here is fixture configuration.
const EX: &str = "https://example.org/d/";

/// The relation IRI the query calls.
const REL_IRI: &str = "https://example.org/rel/indexed";

/// Driving rows per phase, and therefore invocations per phase.
const ROWS: u64 = 10_000;

/// A 64-character lowercase-hex generation, the width the shipped text producer's
/// index fingerprint renders to.
const GENERATION: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// How a fixture cursor attests the generation it already knows.
#[derive(Clone, Copy)]
enum Attests {
    /// Clones the relation's shared pointer — what every shipped producer does.
    SharedPointer,
    /// Copies the spelling into a fresh allocation, the way an owned attestation
    /// payload would force every producer to.
    FreshCopy,
    /// Declares nothing, so this phase reports the seam's floor.
    Nothing,
}

struct IndexedRelation {
    modes: Vec<BindingPattern>,
    generation: Arc<str>,
    attests: Attests,
}

impl PropertyFunction for IndexedRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let subject = args
            .subject()
            .first()
            .copied()
            .flatten()
            .cloned()
            .unwrap_or_else(|| TermValue::iri(format!("{EX}row-0")));
        Ok(Box::new(IndexedCursor {
            row: Some(vec![subject, TermValue::iri(format!("{EX}alpha"))]),
            generation: Arc::clone(&self.generation),
            attests: self.attests,
        }))
    }
}

struct IndexedCursor {
    row: Option<PfRow>,
    generation: Arc<str>,
    attests: Attests,
}

impl PfCursor for IndexedCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.row.take())
    }

    fn generation(&self) -> IndexGeneration {
        match self.attests {
            Attests::SharedPointer => IndexGeneration::Declared(Arc::clone(&self.generation)),
            // `declared` over a `&str` allocates, which is the whole point of this
            // phase: it is the cost an attestation that cannot share pays.
            Attests::FreshCopy => IndexGeneration::declared(&*self.generation),
            Attests::Nothing => IndexGeneration::Undeclared,
        }
    }
}

// ---------------------------------------------------------------------------
// The fixture query
// ---------------------------------------------------------------------------

/// `ROWS` ordinary triples, so the driving pattern has `ROWS` rows to lateral over
/// and the relation is invoked once per row.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let o = builder.intern_iri(&format!("{EX}team"));
    for row in 0..ROWS {
        let s = builder.intern_iri(&format!("{EX}row-{row}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture dataset freezes")
}

/// The driving pattern plus the call, so the subject reaching the relation is bound
/// per row — the correlated shape a real ranked producer is driven in.
const PER_ROW_CALL: &str = "PREFIX ex: <https://example.org/d/>\n\
                            SELECT ?s ?t WHERE {\n\
                              ?s ex:p ?o .\n\
                              ?s <https://example.org/rel/indexed> ?t\n\
                            }";

fn registry(attests: Attests) -> ExtensionEnv {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL_IRI.to_owned(),
        Arc::new(IndexedRelation {
            modes: vec![BindingPattern::from_code("bf")],
            generation: Arc::from(GENERATION),
            attests,
        }),
    );
    ExtensionEnv::over_relations(registry).expect("the fixture declarations read cleanly")
}

fn options(env: &ExtensionEnv) -> QueryOptions<'_> {
    QueryOptions::new().with_env(env)
}

/// Whether a phase runs the lane that can carry a witness.
#[derive(Clone, Copy)]
enum Lane {
    /// `query_prepared_governed_view`: its outcome carries the relation witness, so
    /// the engine reads and records what each invocation attested.
    Governed,
    /// `query_with_options_view`: a bare result with no witness slot, so there is
    /// nothing for a generation to be recorded on.
    Ungoverned,
}

/// Run one phase and report what it allocated per invocation.
fn phase(label: &str, attests: Attests, lane: Lane, dataset: &Arc<RdfDataset>) {
    let engine = NativeSparqlEngine::new();
    let relations = registry(attests);
    let request = SparqlRequest {
        query: PER_ROW_CALL,
        base_iri: None,
        substitutions: &[],
    };
    // Prepared outside the measured window on both lanes, so neither phase pays for
    // parsing and planning and the figures are the evaluation's own.
    let prepared = engine
        .prepare_query_with_options(PER_ROW_CALL, None, options(&relations))
        .expect("the fixture query prepares against the registry");

    let baseline = reset();
    match lane {
        Lane::Governed => {
            let outcome = engine
                .query_prepared_governed_view(
                    &**dataset,
                    &prepared,
                    &[],
                    options(&relations),
                    &QueryGovernors::UNBOUNDED,
                )
                .expect("a governed run of the fixture query is an outcome");
            black_box(outcome.relations().witness.len());
        }
        Lane::Ungoverned => {
            let result = engine
                .query_with_options_view(&**dataset, request, options(&relations))
                .expect("the fixture relation declares nothing short, so this answers");
            black_box(&result);
        }
    }
    report(label, baseline, ROWS);
}

fn main() {
    let dataset = dataset();

    // Warm every lazy one-time allocation outside the reported phases.
    phase("warmup", Attests::SharedPointer, Lane::Governed, &dataset);

    println!("[relation_attestation_alloc] --- what attesting an index generation costs ---");
    phase(
        "shared-generation, governed",
        Attests::SharedPointer,
        Lane::Governed,
        &dataset,
    );
    phase(
        "owned-generation, governed",
        Attests::FreshCopy,
        Lane::Governed,
        &dataset,
    );

    println!("[relation_attestation_alloc] --- what witnessing costs, per lane ---");
    phase(
        "shared-generation, ungoverned",
        Attests::SharedPointer,
        Lane::Ungoverned,
        &dataset,
    );
    // The pair that shows what NOT reading an unreadable generation is worth: this
    // producer cannot share its spelling, so every reading of it costs an
    // allocation — and a lane that cannot carry the answer takes no readings.
    phase(
        "owned-generation, ungoverned",
        Attests::FreshCopy,
        Lane::Ungoverned,
        &dataset,
    );

    println!("[relation_attestation_alloc] --- the seam's floor, with nothing attested ---");
    phase(
        "silent, governed",
        Attests::Nothing,
        Lane::Governed,
        &dataset,
    );
    phase(
        "silent, ungoverned",
        Attests::Nothing,
        Lane::Ungoverned,
        &dataset,
    );
}
